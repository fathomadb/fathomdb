"""One operation refusal must not contaminate other calls in its case."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "slice135_python_capabilities.py"
SPEC = importlib.util.spec_from_file_location("capabilities", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class OperationIsolation(unittest.TestCase):
    def test_shared_case_preserves_one_contract_failure(self) -> None:
        cases = {
            "positive": {"status": "passed", "reopen": [{"body": "bytes"}]},
            "negative": {"status": "partial_failed", "assertions": {"good": {"typed": True}},
                         "operation_errors": {"bad": "missing stable reason field"}},
        }
        good = runner.operation_result("good", "positive", "negative", cases)
        bad = runner.operation_result("bad", "positive", "negative", cases)
        self.assertEqual(good["status"], "executed")
        self.assertEqual(bad["status"], "failed")
        self.assertIn("missing stable reason", bad["error"])

    def test_skipped_positive_cannot_count_as_execution(self) -> None:
        row = runner.operation_result("op", "case", "case",
                                      {"case": {"status": "failed", "error": "pytest skip"}})
        self.assertEqual(row["status"], "failed")


if __name__ == "__main__":
    unittest.main()
