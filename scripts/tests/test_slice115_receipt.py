"""Contract checks for the Slice 115 raw measurement receipt."""

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "slice115_receipt.py"
SPEC = importlib.util.spec_from_file_location("slice115_receipt", SCRIPT)
assert SPEC and SPEC.loader
receipt = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(receipt)


def valid_data():
    cells = {}
    for path in receipt.PATHS:
        samples = []
        for number in (10, 20, 30, 40, 50, 60, 70):
            sample = {"latency_ns": number, "stage_ns": number // 2,
                      "stage": "provider" if path == "hybrid" else "sqlite",
                      "correctness_count": 1, "valid": True}
            if path == "projection":
                sample.update(write_ns=2, drain_ns=4)
            if path == "model_cpu":
                sample.update(model_projection_rows=1, projection_ns=5)
            if path in ("canonical_write", "projection", "erasure"):
                rows = 1 if path == "erasure" else 0
                sample["prestate"] = {"canonical_rows": rows,
                                      "projection_rows": rows,
                                      "source_ids": ["slice115:erase"] if rows else [],
                                      "digest": "f" * 64}
                sample["poststate"] = {"canonical_rows": 0 if rows else 1}
            samples.append(sample)
        cells[path] = samples
    return {
        "source_sha": "a" * 40, "protocol_sha256": "b" * 64,
        "runner_sha256": "c" * 64, "binary_sha256": "d" * 64,
        "corpus_sha256": "e" * 64, "environment_valid": True,
        "cells": cells,
    }


class ReceiptTests(unittest.TestCase):
    def test_nearest_rank_and_no_interpolated_tail(self):
        result = receipt.validate(valid_data(), "a" * 40, "b" * 64, "c" * 64)
        self.assertEqual(result["cells"]["hybrid"]["p50_ns"], 40)
        self.assertEqual(result["cells"]["hybrid"]["p90_ns"], 70)
        self.assertEqual(result["cells"]["hybrid"]["p99_ns"], 70)
        self.assertEqual(result["cells"]["hybrid"]["maximum_ns"], 70)

    def test_rejects_missing_samples_and_invalid_semantics(self):
        data = valid_data()
        data["cells"]["erasure"].pop()
        with self.assertRaisesRegex(ValueError, "seven"):
            receipt.validate(data, "a" * 40, "b" * 64, "c" * 64)
        data = valid_data()
        data["cells"]["erasure"][0]["correctness_count"] = 0
        with self.assertRaisesRegex(ValueError, "semantic"):
            receipt.validate(data, "a" * 40, "b" * 64, "c" * 64)

    def test_rejects_binding_or_environment_mismatch(self):
        for field, expected in (("source_sha", "a" * 40),
                                ("protocol_sha256", "b" * 64),
                                ("runner_sha256", "c" * 64)):
            data = valid_data()
            data[field] = "f" * len(expected)
            with self.assertRaisesRegex(ValueError, field):
                receipt.validate(data, "a" * 40, "b" * 64, "c" * 64)
        data = valid_data()
        data["environment_valid"] = False
        with self.assertRaisesRegex(ValueError, "environment"):
            receipt.validate(data, "a" * 40, "b" * 64, "c" * 64)

    def test_rejects_missing_attribution_and_preserves_invalid_attempts(self):
        data = valid_data()
        data["cells"]["open_fresh"][0]["stage_ns"] = 0
        with self.assertRaisesRegex(ValueError, "stage"):
            receipt.validate(data, "a" * 40, "b" * 64, "c" * 64)
        data = valid_data()
        data["cells"]["open_fresh"].append({
            "latency_ns": 1, "stage_ns": 0, "stage": "sqlite",
            "correctness_count": 0, "valid": False, "reason": "prestate mismatch"})
        result = receipt.validate(data, "a" * 40, "b" * 64, "c" * 64)
        self.assertEqual(result["cells"]["open_fresh"]["invalid_attempts"], 1)
        self.assertEqual(result["cells"]["open_fresh"]["p50_ns"], 40)

    def test_projection_latency_spans_write_and_drain(self):
        data = valid_data()
        data["cells"]["projection"][0]["latency_ns"] = 5
        with self.assertRaisesRegex(ValueError, "write-to-ready"):
            receipt.validate(data, "a" * 40, "b" * 64, "c" * 64)
        data = valid_data()
        del data["cells"]["projection"][0]["write_ns"]
        with self.assertRaisesRegex(ValueError, "write-to-ready"):
            receipt.validate(data, "a" * 40, "b" * 64, "c" * 64)


if __name__ == "__main__":
    unittest.main()
