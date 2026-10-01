"""Reject a PASS receipt whose claimed metrics diverge from its raw observations."""

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
PROTOCOL = json.loads((ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json").read_text())


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


runner = module("d27_runner_linkage", ROOT / "scripts/d27-runtime-runner.py")
verifier = module("d27_verifier_linkage", ROOT / "scripts/d27-runtime-qualification.py")


def synthetic_raw(direction, repetition):
    ratio = PROTOCOL["execution"][f"{direction}_epoch"]
    operations = []
    projection = []
    cursor = 10_001
    for name, count in (("canonical_write", ratio["canonical_writes"]), ("foreground_hybrid_query", ratio["foreground_hybrid_queries"]), ("direct_embed", ratio["direct_embeds"])):
        for _ in range(count):
            record = {"class": name, "admitted_ns": 1_000_000, "completed_ns": 2_000_000, "outcome": "completed", "cursor": cursor if name == "canonical_write" else None}
            operations.append(record)
            if name == "canonical_write":
                projection.append({"cursor": cursor, "committed_ns": 2_000_000, "observed_ns": 3_000_000})
                cursor += 1
    return {
        "direction": direction, "repetition": repetition, "warmup_seconds": 10,
        "measurement_seconds": 60, "measurement_elapsed_ns": 60_000_000_000,
        "epoch_size": 10, "operation_counts": ratio, "operations": operations,
        "projection_completions": projection, "projection_backlog_high_water": count,
        "provider_peak_concurrency": 1, "engine_thread_inventory": 11,
        "connection_inventory": "Err(Storage)", "residual_workers_after_close": 0,
        "close_start_ns": 60_010_000_000, "close_end_ns": 60_020_000_000,
        "close_result": "Ok(())", "smoke": False,
        "environment_valid": True,
    }


class RawLinkageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.raw_path = Path(self.temp.name) / "raw.jsonl"
        self.raw = [synthetic_raw(direction, repetition) for repetition in (1, 2, 3) for direction in ("projection_heavy", "foreground_heavy")]
        self.raw_path.write_text("".join(json.dumps(item) + "\n" for item in self.raw))
        self.receipt = {"phase": "entry", "per_repetition_metrics": {direction: [runner.summarize_raw(item, "entry") for item in self.raw if item["direction"] == direction] for direction in ("projection_heavy", "foreground_heavy")}}

    def test_valid_raw_matches_receipt_without_rounding(self):
        verifier.verify_raw_linkage(self.receipt, PROTOCOL, self.raw_path)

    def test_fabricated_throughput_is_rejected(self):
        changed = copy.deepcopy(self.receipt)
        changed["per_repetition_metrics"]["projection_heavy"][0]["throughput"]["canonical_commits"] += 1
        with self.assertRaisesRegex(ValueError, "raw metric mismatch"):
            verifier.verify_raw_linkage(changed, PROTOCOL, self.raw_path)

    def test_smoke_and_missing_projection_are_rejected(self):
        changed = copy.deepcopy(self.raw)
        changed[0]["smoke"] = True
        self.raw_path.write_text("".join(json.dumps(item) + "\n" for item in changed))
        with self.assertRaisesRegex(ValueError, "smoke"):
            verifier.verify_raw_linkage(self.receipt, PROTOCOL, self.raw_path)
        changed[0]["smoke"] = False
        changed[0]["projection_completions"].pop()
        self.raw_path.write_text("".join(json.dumps(item) + "\n" for item in changed))
        with self.assertRaisesRegex(ValueError, "projection"):
            verifier.verify_raw_linkage(self.receipt, PROTOCOL, self.raw_path)


if __name__ == "__main__":
    unittest.main()
