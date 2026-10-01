"""Reject a PASS receipt whose claimed metrics diverge from its raw observations."""

import copy
import hashlib
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
            record = {"class": name, "sequence": len(operations), "admitted_ns": 1_000_000, "completed_ns": 2_000_000, "outcome": "completed", "cursor": cursor if name == "canonical_write" else None}
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
        "environment_start": {"cpu_governor": "performance", "competing_processes": [], "swap_pages_in": 0, "swap_pages_out": 0, "database_device": "/dev/nvme1n1p1"},
        "environment_end": {"cpu_governor": "performance", "competing_processes": [], "swap_pages_in": 0, "swap_pages_out": 0, "database_device": "/dev/nvme1n1p1"},
        "environment_samples": [{"cpu_governor": "performance", "competing_processes": [], "swap_pages_in": 0, "swap_pages_out": 0, "database_device": "/dev/nvme1n1p1"}],
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

    def test_forged_environment_valid_flag_is_rejected(self):
        changed = copy.deepcopy(self.raw)
        changed[0]["environment_end"]["cpu_governor"] = "powersave"
        self.raw_path.write_text("".join(json.dumps(item) + "\n" for item in changed))
        with self.assertRaisesRegex(ValueError, "environment"):
            verifier.verify_raw_linkage(self.receipt, PROTOCOL, self.raw_path)

    def test_middle_of_repetition_competing_workload_is_rejected(self):
        changed = copy.deepcopy(self.raw)
        sample = copy.deepcopy(changed[0]["environment_start"])
        sample["competing_processes"] = [{"pid": 123, "name": "cargo"}]
        changed[0]["environment_samples"] = [sample]
        self.raw_path.write_text("".join(json.dumps(item) + "\n" for item in changed))
        with self.assertRaisesRegex(ValueError, "environment"):
            verifier.verify_raw_linkage(self.receipt, PROTOCOL, self.raw_path)

    def test_missing_midrun_samples_are_rejected(self):
        changed = copy.deepcopy(self.raw)
        changed[0]["environment_samples"] = []
        self.raw_path.write_text("".join(json.dumps(item) + "\n" for item in changed))
        with self.assertRaisesRegex(ValueError, "environment samples"):
            verifier.verify_raw_linkage(self.receipt, PROTOCOL, self.raw_path)

    def test_each_epoch_is_checked_from_sequence_numbers(self):
        changed = copy.deepcopy(self.raw)
        changed[0]["operations"][1]["sequence"] = 10
        self.raw_path.write_text("".join(json.dumps(item) + "\n" for item in changed))
        with self.assertRaisesRegex(ValueError, "epoch"):
            verifier.verify_raw_linkage(self.receipt, PROTOCOL, self.raw_path)

    def test_projection_throughput_stops_at_admission_boundary(self):
        raw = copy.deepcopy(self.raw[0])
        raw["projection_completions"][0]["observed_ns"] = 61_000_000_000
        summary = runner.summarize_raw(raw, "entry")
        self.assertEqual(summary["throughput"]["projection_completions"], 3 / 60)
        self.assertEqual(len(raw["projection_completions"]), 4)

    def test_any_pending_work_without_progress_fails_starvation_window(self):
        raw = {"measurement_elapsed_ns": 10_000_000_000, "operations": [{"class": "canonical_write", "admitted_ns": 1_000_000_000, "completed_ns": 6_000_000_000}], "projection_completions": []}
        self.assertFalse(runner.starvation_from_raw(raw))

    def test_candidate_requires_observed_configuration_and_entry_artifacts(self):
        raw = copy.deepcopy(self.raw[0])
        with self.assertRaisesRegex(ValueError, "configuration observation"):
            runner.summarize_raw(raw, "candidate")
        with self.assertRaisesRegex(ValueError, "entry artifacts"):
            verifier.validate_entry_for_candidate({"phase": "entry", "status": "PASS"}, PROTOCOL, ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json", None)

    def test_candidate_cannot_use_unlinked_queue_numbers(self):
        raw = copy.deepcopy(self.raw[0])
        raw["configuration_observation"] = {"source": "engine", "scheduler_runtime_threads": 2, "embedder_pool_size": 1}
        raw["projection_admission_high_water"] = 4
        raw["embed_requests_waiting_high_water"] = 0
        raw["embed_queue_wait_ns"] = [0]
        raw["engine_thread_inventory"] = 12
        raw["connection_inventory"] = "Ok(\"creation=writer:1,readers:8,dispatcher:1,workers:2,probes:0\")"
        with self.assertRaisesRegex(ValueError, "dispatch trace"):
            runner.summarize_raw(raw, "candidate")

    def test_protocol_declares_historical_sqlite_inventory_unavailable(self):
        self.assertIn("sqlite_connections", PROTOCOL["metrics"]["historical_unavailable"])

    def test_tampered_entry_aggregate_cannot_be_used_by_candidate(self):
        artifacts = {}
        for name in ("runner", "binary", "corpus"):
            artifact = Path(self.temp.name) / name
            artifact.write_bytes(name.encode())
            artifacts[name] = artifact
        artifacts["raw"] = self.raw_path
        receipt = copy.deepcopy(self.receipt)
        receipt.update({
            "source_sha": PROTOCOL["entry_engine_candidate_sha"],
            "runner_sha256": hashlib.sha256(artifacts["runner"].read_bytes()).hexdigest(),
            "binary_sha256": hashlib.sha256(artifacts["binary"].read_bytes()).hexdigest(),
            "corpus_sha256": hashlib.sha256(artifacts["corpus"].read_bytes()).hexdigest(),
            "raw_output_sha256": hashlib.sha256(self.raw_path.read_bytes()).hexdigest(),
            "protocol_sha256": hashlib.sha256((ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json").read_bytes()).hexdigest(),
            "runner_inventory": {"host": "windchill3", "operating_system": "Linux x86_64", "database_storage": "local NVMe", "build": "cargo test --release", "features": ["test-hooks"], "online_cpus": 8, "memory_gib": 16},
            "environment_start": self.raw[0]["environment_start"],
            "environment_end": self.raw[-1]["environment_end"],
            "historical_unavailable": PROTOCOL["metrics"]["historical_unavailable"],
            "status": "PASS",
        })
        protocol_path = ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json"
        validated = verifier.validate_receipt(receipt, PROTOCOL, protocol_path, artifacts)
        receipt["aggregate_metrics"] = validated["aggregate_metrics"]
        verifier.validate_entry_for_candidate(receipt, PROTOCOL, protocol_path, artifacts)
        receipt["aggregate_metrics"]["projection_heavy"]["throughput"]["canonical_commits"]["median"] += 1
        with self.assertRaisesRegex(ValueError, "entry aggregate"):
            verifier.validate_entry_for_candidate(receipt, PROTOCOL, protocol_path, artifacts)

    def test_candidate_dispatch_trace_is_derived_and_linked(self):
        raw = copy.deepcopy(self.raw[0])
        raw["configuration_observation"] = {"source": "engine", "scheduler_runtime_threads": 2, "embedder_pool_size": 1}
        raw["projection_admission_observation"] = {"source": "engine", "active_plus_queued_high_water": 4}
        raw["engine_thread_inventory"] = 12
        raw["connection_inventory"] = "Ok(\"creation=writer:1,readers:8,dispatcher:1,workers:2,probes:0\")"
        events = []
        for index, operation in enumerate(raw["operations"]):
            owner = {"projection_cursors": [operation["cursor"]]} if operation["class"] == "canonical_write" else {"operation_sequence": operation["sequence"]}
            start = 2_100_000 + index * 100_000
            events.append({"source": "engine", "request_id": index, "owner": owner, "admitted_ns": start, "started_ns": start + 10_000, "terminal_ns": start + 80_000, "outcome": "completed"})
        raw["embed_dispatch_events"] = events
        summary = runner.summarize_raw(raw, "candidate")
        self.assertEqual(summary["high_water"]["embed_requests_waiting"], 1)
        self.assertEqual(summary["latency_ms"]["embed_queue_wait"]["50"], 0.01)
        raw["embed_dispatch_events"][0]["owner"] = {"operation_sequence": 999}
        with self.assertRaisesRegex(ValueError, "dispatch trace"):
            runner.summarize_raw(raw, "candidate")


if __name__ == "__main__":
    unittest.main()
