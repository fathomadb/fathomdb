"""The D27 controller must preserve the frozen corpus and run contract."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "d27-runtime-runner.py"
PROTOCOL = ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol.json"
spec = importlib.util.spec_from_file_location("d27_runtime_runner", SCRIPT)
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class D27RunnerTests(unittest.TestCase):
    def setUp(self):
        self.protocol = json.loads(PROTOCOL.read_text())

    def test_corpus_matches_frozen_shape_and_is_byte_identical(self):
        with tempfile.TemporaryDirectory() as directory:
            one = Path(directory) / "one.jsonl"
            two = Path(directory) / "two.jsonl"
            runner.generate_corpus(self.protocol, one)
            runner.generate_corpus(self.protocol, two)
            self.assertEqual(one.read_bytes(), two.read_bytes())
            rows = [json.loads(line) for line in one.read_text().splitlines()]
            self.assertEqual(len(rows), 10_000)
            self.assertEqual(len({row["logical_id"] for row in rows}), 10_000)
            self.assertTrue(all(len(row["body"].encode()) == 512 for row in rows))
            self.assertEqual(len({word for row in rows for word in row["body"].split()}), 1024)

    def test_order_and_ratios_are_not_silently_changed(self):
        expected = self.protocol["execution"]["repetition_order"]
        self.assertEqual(runner.repetition_order(self.protocol, "entry"), expected)
        self.assertEqual(len(runner.repetition_order(self.protocol, "candidate")), 6)
        raw = {"direction": "projection_heavy", "repetition": 1, "warmup_seconds": 10, "measurement_seconds": 60, "epoch_size": 10, "operation_counts": {"canonical_writes": 40, "foreground_hybrid_queries": 40, "direct_embeds": 20}}
        runner.validate_raw_contract(raw, self.protocol, "projection_heavy", 1)
        raw["operation_counts"]["canonical_writes"] = 39
        with self.assertRaisesRegex(ValueError, "ratios"):
            runner.validate_raw_contract(raw, self.protocol, "projection_heavy", 1)
        raw["operation_counts"]["canonical_writes"] = 40
        raw["warmup_seconds"] = 1
        with self.assertRaisesRegex(ValueError, "warmup"):
            runner.validate_raw_contract(raw, self.protocol, "projection_heavy", 1)

    def test_candidate_requires_default_resolved_worker_pair(self):
        raw = {"direction": "projection_heavy", "repetition": 1, "warmup_seconds": 10,
               "measurement_seconds": 60, "epoch_size": 10,
               "operation_counts": {"canonical_writes": 4, "foreground_hybrid_queries": 4, "direct_embeds": 2},
               "configuration_observation": {"source": "engine", "scheduler_runtime_threads": 4, "embedder_pool_size": 4}}
        with self.assertRaisesRegex(ValueError, "default.*2/1"):
            runner.validate_raw_contract(raw, self.protocol, "projection_heavy", 1, "candidate")

    def test_candidate_trace_rejects_missing_duplicate_and_unowned_requests(self):
        raw = {"operations": [
            {"sequence": 1, "class": "direct_embed", "cursor": None, "admitted_ns": 0, "completed_ns": 10},
            {"sequence": 2, "class": "canonical_write", "cursor": 7, "outcome": "completed", "admitted_ns": 0, "completed_ns": 10}],
            "provider_peak_concurrency": 1}
        direct = {"source": "engine", "request_id": 1, "owner": {"operation_sequence": 1},
                  "admitted_ns": 1, "started_ns": 2, "terminal_ns": 3}
        projection = {"source": "engine", "request_id": 2, "owner": {"projection_cursors": [7]},
                      "admitted_ns": 4, "started_ns": 5, "terminal_ns": 6}
        for events, error in (([], "missing"), ([direct, direct, projection], "identity"),
                              ([direct, {**projection, "owner": {}}], "owner"),
                              ([direct], "cover")):
            with self.subTest(error=error), self.assertRaisesRegex(ValueError, error):
                runner.dispatch_from_raw({**raw, "embed_dispatch_events": events}, 1)

    def test_candidate_trace_rejects_foreground_dispatch_after_operation_completion(self):
        raw = {"operations": [{"sequence": 1, "class": "direct_embed", "cursor": None,
                               "admitted_ns": 1, "completed_ns": 5}],
               "provider_peak_concurrency": 1,
               "embed_dispatch_events": [{"source": "engine", "request_id": 1,
                                           "owner": {"operation_sequence": 1},
                                           "admitted_ns": 6, "started_ns": 7, "terminal_ns": 8}]}
        with self.assertRaisesRegex(ValueError, "foreground.*interval"):
            runner.dispatch_from_raw(raw, 1)

    def test_candidate_trace_rejects_foreground_terminal_after_operation_completion(self):
        raw = {"operations": [{"sequence": 1, "class": "direct_embed", "cursor": None,
                               "admitted_ns": 1, "completed_ns": 5}],
               "provider_peak_concurrency": 1,
               "embed_dispatch_events": [{"source": "engine", "request_id": 1,
                                           "owner": {"operation_sequence": 1},
                                           "admitted_ns": 2, "started_ns": 3, "terminal_ns": 6}]}
        with self.assertRaisesRegex(ValueError, "foreground.*interval"):
            runner.dispatch_from_raw(raw, 1)

    def test_candidate_sqlite_inventory_requires_exact_live_roles(self):
        accepted = "live=writer:1,readers:8,dispatcher:1,workers:2,probes:0"
        self.assertEqual(runner.candidate_sqlite_inventory(accepted, 2), 12)
        for invalid in ("creation=writer:1,readers:8,dispatcher:1,workers:2,probes:0",
                        "live=writer:2,readers:7,dispatcher:1,workers:2,probes:0",
                        "live=writer:1,readers:7,dispatcher:2,workers:2,probes:0",
                        "live=writer:1,readers:8,dispatcher:1,workers:1,probes:1"):
            with self.subTest(invalid=invalid), self.assertRaisesRegex(ValueError, "SQLite.*inventory"):
                runner.candidate_sqlite_inventory(invalid, 2)

    def test_candidate_workload_uses_engine_observation_hooks(self):
        source = (ROOT / "scripts/d27_runtime_workload.rs").read_text()
        self.assertIn("Engine::open_with_choice_and_config", source)
        self.assertIn("EngineConfig::default()", source)
        self.assertNotIn("scheduler_runtime_threads: Some(2)", source)
        self.assertNotIn("embedder_pool_size: Some(1)", source)
        self.assertIn("begin_d27_observation_for_test", source)
        self.assertIn("d27_observation_for_test", source)


if __name__ == "__main__":
    unittest.main()
