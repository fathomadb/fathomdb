"""Fail-closed profile and mutation-prestate receipt checks."""

import copy
import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "slice115_receipt.py"
SPEC = importlib.util.spec_from_file_location("slice115_receipt", SCRIPT)
assert SPEC and SPEC.loader
receipt = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(receipt)


def data():
    cells = {}
    for path in receipt.PATHS:
        cell = {"latency_ns": 100, "correctness_count": 1, "valid": True,
                "stage": "provider", "stage_ns": 50}
        if path == "projection":
            cell["write_ns"] = 10
            cell["drain_ns"] = 60
        if path == "model_cpu":
            cell["model_projection_rows"] = 1
            cell["projection_ns"] = 25
        if path in ("canonical_write", "projection", "erasure"):
            cell["prestate"] = {"canonical_rows": 1 if path == "erasure" else 0,
                                "projection_rows": 1 if path == "erasure" else 0,
                                "source_ids": ["slice115:erase"] if path == "erasure" else [],
                                "digest": "a" * 64}
            cell["poststate"] = {"canonical_rows": 0 if path == "erasure" else 1}
        cells[path] = [copy.deepcopy(cell) for _ in range(7)]
    return {"source_sha": "a" * 40, "protocol_sha256": "b" * 64,
            "runner_sha256": "c" * 64, "binary_sha256": "d" * 64,
            "corpus_sha256": "e" * 64, "environment_valid": True,
            "cells": cells,
            "profiles": {"text": {"sha256": "f" * 64, "operation_samples": 4,
                                  "method": "gdb-interrupt-stack"}}}


class ProfileReceiptTests(unittest.TestCase):
    def test_profile_can_substitute_for_stage_only_when_usable(self):
        raw = data()
        for sample in raw["cells"]["text"]:
            del sample["stage"]
            del sample["stage_ns"]
            sample["profile_ref"] = "profiles/text.txt"
        receipt.validate(raw, "a" * 40, "b" * 64, "c" * 64)
        raw["profiles"]["text"]["operation_samples"] = 0
        with self.assertRaisesRegex(ValueError, "profile"):
            receipt.validate(raw, "a" * 40, "b" * 64, "c" * 64)

    def test_erasure_prestate_cannot_be_already_empty(self):
        raw = data()
        raw["cells"]["erasure"][0]["prestate"] = {
            "canonical_rows": 0, "projection_rows": 0, "source_ids": [],
            "digest": "a" * 64}
        with self.assertRaisesRegex(ValueError, "prestate"):
            receipt.validate(raw, "a" * 40, "b" * 64, "c" * 64)

    def test_all_mutation_prestates_match(self):
        raw = data()
        raw["cells"]["projection"][1]["prestate"]["digest"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "prestate"):
            receipt.validate(raw, "a" * 40, "b" * 64, "c" * 64)


if __name__ == "__main__":
    unittest.main()
