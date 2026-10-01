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


if __name__ == "__main__":
    unittest.main()
