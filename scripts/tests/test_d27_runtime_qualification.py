"""Contract tests for the frozen D27 qualification receipt verifier."""

import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "d27-runtime-qualification.py"
PROTOCOL = ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json"
spec = importlib.util.spec_from_file_location("d27_runtime_qualification", SCRIPT)
d27 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(d27)


def digest(data):
    return hashlib.sha256(data).hexdigest()


class D27ReceiptTests(unittest.TestCase):
    def setUp(self):
        self.protocol = json.loads(PROTOCOL.read_text())
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.artifacts = {}
        for name in ("runner", "binary", "corpus", "raw"):
            path = self.directory / name
            path.write_bytes(name.encode())
            self.artifacts[name] = path
        self.receipt = self.complete_receipt()

    def complete_receipt(self):
        metrics = {}
        for direction in ("projection_heavy", "foreground_heavy"):
            metrics[direction] = []
            for _ in range(3):
                metrics[direction].append({
                    "throughput": {name: 100.0 for name in self.protocol["metrics"]["throughput_rates_per_second"]},
                    "latency_ms": {
                        name: {str(p): 10.0 for p in (50, 95, 99)}
                        for name in self.protocol["metrics"]["latency_classes"]
                    },
                    "counts": {name: 1 for name in self.protocol["metrics"]["counts"]},
                    "high_water": {"projection_rows_active_plus_queued": 2, "durable_projection_backlog": 2},
                    "inventory": {"provider_concurrency": 1, "engine_threads": 3, "sqlite_connections": 6, "residual_workers_after_close": 0},
                    "starvation_pass": True,
                    "environment_valid": True,
                })
        return {
            "source_sha": self.protocol["entry_engine_candidate_sha"],
            "phase": "entry",
            "binary_sha256": digest(b"binary"),
            "runner_sha256": digest(b"runner"),
            "protocol_sha256": digest(PROTOCOL.read_bytes()),
            "corpus_sha256": digest(b"corpus"),
            "raw_output_sha256": digest(b"raw"),
            "runner_inventory": {"host": "windchill3", "operating_system": "Linux x86_64", "online_cpus": 8, "memory_gib": 16, "database_storage": "local NVMe", "build": "cargo test --release", "features": ["test-hooks"]},
            "environment_start": {"cpu_governor": "performance", "competing_processes": [], "swap_pages_in": 0, "swap_pages_out": 0},
            "environment_end": {"cpu_governor": "performance", "competing_processes": [], "swap_pages_in": 0, "swap_pages_out": 0},
            "per_repetition_metrics": metrics,
            "aggregate_metrics": {},
            "decision_rule_evaluation": {},
            "status": "PASS",
            "historical_unavailable": self.protocol["metrics"]["historical_unavailable"],
        }

    def validate(self, receipt=None, protocol=None):
        return d27.validate_receipt(receipt or self.receipt, protocol or self.protocol, PROTOCOL, self.artifacts)

    def test_complete_entry_is_accepted_and_centers_are_computed(self):
        result = self.validate()
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(result["aggregate_metrics"]["projection_heavy"]["throughput"]["canonical_commits"], {"median": 100.0, "mad": 0.0})

    def test_wrong_candidate_and_modified_protocol_are_rejected(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["source_sha"] = "0" * 40
        with self.assertRaisesRegex(ValueError, "entry candidate"):
            self.validate(receipt)
        protocol = copy.deepcopy(self.protocol)
        protocol["fixture"]["seed"] = "changed"
        with self.assertRaisesRegex(ValueError, "frozen protocol"):
            self.validate(protocol=protocol)

    def test_modified_corpus_and_runner_binary_raw_hashes_are_rejected(self):
        for name in ("corpus", "runner", "binary", "raw"):
            self.artifacts[name].write_bytes(b"changed")
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "sha256"):
                self.validate()
            self.artifacts[name].write_bytes(name.encode())

    def test_missing_shared_metric_and_wrong_repetition_count_are_rejected(self):
        receipt = copy.deepcopy(self.receipt)
        del receipt["per_repetition_metrics"]["foreground_heavy"][0]["throughput"]["direct_embeds"]
        with self.assertRaisesRegex(ValueError, "direct_embeds"):
            self.validate(receipt)
        receipt = copy.deepcopy(self.receipt)
        receipt["per_repetition_metrics"]["projection_heavy"].pop()
        with self.assertRaisesRegex(ValueError, "repetitions"):
            self.validate(receipt)

    def test_invalid_environment_and_imputed_historical_metric_are_rejected(self):
        receipt = copy.deepcopy(self.receipt)
        receipt["environment_end"]["cpu_governor"] = "powersave"
        with self.assertRaisesRegex(ValueError, "governor"):
            self.validate(receipt)
        receipt = copy.deepcopy(self.receipt)
        receipt["per_repetition_metrics"]["projection_heavy"][0]["high_water"]["embed_requests_waiting"] = 0
        with self.assertRaisesRegex(ValueError, "historical unavailable"):
            self.validate(receipt)

    def test_candidate_comparison_uses_independent_median_mad(self):
        entry = self.validate()
        candidate = copy.deepcopy(self.receipt)
        candidate["phase"] = "candidate"
        candidate["source_sha"] = "1" * 40
        candidate.pop("historical_unavailable")
        for repetitions in candidate["per_repetition_metrics"].values():
            for repetition in repetitions:
                repetition["high_water"]["embed_requests_waiting"] = 0
                repetition["latency_ms"]["embed_queue_wait"] = {"50": 1.0, "95": 1.0, "99": 1.0}
        candidate["per_repetition_metrics"]["foreground_heavy"][0]["throughput"]["canonical_commits"] = 80.0
        candidate["per_repetition_metrics"]["foreground_heavy"][1]["throughput"]["canonical_commits"] = 80.0
        candidate["per_repetition_metrics"]["foreground_heavy"][2]["throughput"]["canonical_commits"] = 80.0
        with self.assertRaisesRegex(ValueError, "foreground_heavy.*canonical_commits"):
            d27.validate_receipt(candidate, self.protocol, PROTOCOL, self.artifacts, entry)


if __name__ == "__main__":
    unittest.main()
