"""Fail-closed checks for the provisional Slice 135 Phase 1 raw receipt."""

import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_receipt.py"
SPEC = importlib.util.spec_from_file_location("slice135_receipt", SCRIPT)
assert SPEC and SPEC.loader
receipt = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(receipt)


def digest(data):
    return hashlib.sha256(data).hexdigest()


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "candidate.bin").write_bytes(b"candidate")
        (self.root / "corpus.txt").write_bytes(b"fixed corpus")
        self.source = "a" * 40
        self.runner_hash = "b" * 64
        self.protocol = {
            "schema_version": 1,
            "source_sha": self.source,
            "runner_sha256": self.runner_hash,
            "artifact_sha256": {
                "candidate.bin": digest(b"candidate"),
                "corpus.txt": digest(b"fixed corpus"),
            },
            "features": {"graph": True, "rerank": False},
            "settings": {"top_k": 10, "concurrency": 1},
            "min_disk_free_bytes": 100,
            "cells": {
                "query": {
                    "kind": "query",
                    "boundary": "client-call-to-materialized-output",
                    "expected_checks": {"record_count": 10},
                },
                "close": {
                    "kind": "lifecycle",
                    "boundary": "close-call-to-return",
                    "expected_checks": {"workers_stopped": True},
                },
            },
        }
        self.protocol_hash = digest(json.dumps(self.protocol, sort_keys=True).encode())
        state = {
            "host": "host", "kernel": "kernel", "cpu": "cpu", "storage": "ssd",
            "governor": "performance", "toolchain": "rustc 1", "profiler": "none",
            "swap_pages": 0, "competing_jobs": [], "disk_free_bytes": 1000,
        }
        self.raw = {
            "schema_version": 1,
            "source_sha": self.source,
            "runner_sha256": self.runner_hash,
            "protocol_sha256": self.protocol_hash,
            "artifact_sha256": copy.deepcopy(self.protocol["artifact_sha256"]),
            "features": copy.deepcopy(self.protocol["features"]),
            "settings": copy.deepcopy(self.protocol["settings"]),
            "environment": {"start": state, "end": copy.deepcopy(state),
                            "invalidators": []},
            "cells": {
                name: {
                    "boundary": cell["boundary"],
                    "attempts": [
                        {"valid": True, "latency_ns": i,
                         "semantic_ok": True,
                         "observed_checks": copy.deepcopy(cell["expected_checks"])}
                        for i in range(1, 101)
                    ],
                }
                for name, cell in self.protocol["cells"].items()
            },
        }

    def validate(self):
        return receipt.validate(
            self.raw, self.protocol, self.root,
            expected_source_sha=self.source,
            protocol_sha256=self.protocol_hash,
            runner_sha256=self.runner_hash,
        )

    def test_recomputes_nearest_rank_and_omits_unsupported_tail(self):
        self.raw["cells"]["query"]["attempts"].append({
            "valid": False, "reason": "timeout", "latency_ns": 5000,
        })
        summary = self.validate()
        self.assertEqual(summary["cells"]["query"]["p50_ns"], 50)
        self.assertEqual(summary["cells"]["query"]["p95_ns"], 95)
        self.assertNotIn("p99_ns", summary["cells"]["query"])
        self.assertEqual(summary["cells"]["query"]["unsupported_statistics"], ["p99"])
        self.assertEqual(summary["cells"]["query"]["invalid_attempts"], 1)
        self.assertEqual(len(self.raw["cells"]["query"]["attempts"]), 101)
        self.assertEqual(summary["cells"]["close"]["p95_ns"], 95)
        self.assertNotIn("p99_ns", summary["cells"]["close"])
        self.raw["cells"]["query"]["attempts"] = [
            {"valid": True, "latency_ns": i, "semantic_ok": True,
             "observed_checks": {"record_count": 10}}
            for i in range(1, 1001)
        ]
        self.assertEqual(self.validate()["cells"]["query"]["p99_ns"], 990)

    def test_rejects_source_runner_protocol_and_artifact_mismatch(self):
        for field in ("source_sha", "runner_sha256", "protocol_sha256"):
            with self.subTest(field=field):
                raw = copy.deepcopy(self.raw)
                self.raw[field] = "f" * len(self.raw[field])
                with self.assertRaisesRegex(ValueError, field):
                    self.validate()
                self.raw = raw
        self.raw["artifact_sha256"]["corpus.txt"] = "f" * 64
        with self.assertRaisesRegex(ValueError, "artifact"):
            self.validate()
        self.raw["artifact_sha256"] = copy.deepcopy(self.protocol["artifact_sha256"])
        (self.root / "candidate.bin").write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "artifact"):
            self.validate()

    def test_rejects_changed_features_settings_and_boundaries(self):
        self.raw["features"]["graph"] = False
        with self.assertRaisesRegex(ValueError, "features"):
            self.validate()
        self.raw["features"] = copy.deepcopy(self.protocol["features"])
        self.raw["settings"]["top_k"] = 20
        with self.assertRaisesRegex(ValueError, "settings"):
            self.validate()
        self.raw["settings"] = copy.deepcopy(self.protocol["settings"])
        self.raw["cells"]["query"]["boundary"] = "internal-stage"
        with self.assertRaisesRegex(ValueError, "boundary"):
            self.validate()

    def test_rejects_insufficient_samples_and_unexplained_invalid_attempt(self):
        self.raw["cells"]["close"]["attempts"].pop()
        with self.assertRaisesRegex(ValueError, "100 valid"):
            self.validate()
        self.raw["cells"]["close"]["attempts"].append({
            "valid": False, "latency_ns": 100,
        })
        with self.assertRaisesRegex(ValueError, "reason"):
            self.validate()

    def test_rejects_semantic_failure_and_wrong_observed_state(self):
        self.raw["cells"]["query"]["attempts"][0]["semantic_ok"] = False
        with self.assertRaisesRegex(ValueError, "semantic"):
            self.validate()
        self.raw["cells"]["query"]["attempts"][0]["semantic_ok"] = True
        self.raw["cells"]["query"]["attempts"][0]["observed_checks"] = {
            "record_count": 9,
        }
        with self.assertRaisesRegex(ValueError, "observed"):
            self.validate()

    def test_rejects_invalid_or_drifting_environment(self):
        self.raw["environment"]["invalidators"] = ["competing heavy job"]
        with self.assertRaisesRegex(ValueError, "environment"):
            self.validate()
        self.raw["environment"]["invalidators"] = []
        self.raw["environment"]["end"]["governor"] = "powersave"
        with self.assertRaisesRegex(ValueError, "environment"):
            self.validate()
        self.raw["environment"]["end"]["governor"] = "performance"
        self.raw["environment"]["start"]["disk_free_bytes"] = 1
        with self.assertRaisesRegex(ValueError, "environment"):
            self.validate()

    def test_cli_recomputes_without_overwriting_raw_attempts(self):
        self.protocol["runner_sha256"] = digest(SCRIPT.read_bytes())
        protocol_bytes = json.dumps(self.protocol, sort_keys=True).encode()
        self.raw["runner_sha256"] = self.protocol["runner_sha256"]
        self.raw["protocol_sha256"] = digest(protocol_bytes)
        self.raw["cells"]["query"]["attempts"].append({
            "valid": False, "reason": "timeout", "latency_ns": 9999,
        })
        raw_path = self.root / "raw.json"
        protocol_path = self.root / "protocol.json"
        output_path = self.root / "summary.json"
        raw_bytes = json.dumps(self.raw, sort_keys=True).encode()
        raw_path.write_bytes(raw_bytes)
        protocol_path.write_bytes(protocol_bytes)
        command = [
            sys.executable, str(SCRIPT), "--raw", str(raw_path),
            "--protocol", str(protocol_path), "--artifacts-root", str(self.root),
            "--source-sha", self.source, "--output", str(output_path),
        ]
        subprocess.run(command, check=True, capture_output=True, text=True)
        summary = json.loads(output_path.read_text())
        self.assertEqual(summary["raw_sha256"], digest(raw_bytes))
        self.assertEqual(summary["cells"]["query"]["invalid_attempts"], 1)
        self.assertEqual(raw_path.read_bytes(), raw_bytes)
        forbidden = subprocess.run(
            [*command[:-1], str(raw_path)], capture_output=True, text=True,
        )
        self.assertNotEqual(forbidden.returncode, 0)
        self.assertEqual(raw_path.read_bytes(), raw_bytes)


if __name__ == "__main__":
    unittest.main()
