"""Contract tests for exhaustive installed-wheel capability accounting."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "slice135_python_capabilities.py"
SPEC = importlib.util.spec_from_file_location("capabilities", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class CapabilityContract(unittest.TestCase):
    def test_every_supported_operation_requires_an_outcome(self) -> None:
        with self.assertRaisesRegex(ValueError, "operation set"):
            runner.summarize(["a", "b"], {"a": {"status": "executed"}})

    def test_failed_and_unavailable_rows_do_not_count_as_executed(self) -> None:
        rows = {
            "a": {"status": "executed", "positive": {"body": "exact bytes"},
                  "negative": {"type": "ValueError"}, "reopen": {"body": "exact bytes"}},
            "b": {"status": "failed", "error": "assertion failed"},
            "c": {"status": "unavailable", "reason": "provider absent", "owner": "Slice 135"},
            "d": {"status": "gap", "reason": "not implemented", "owner": "Slice 135"},
        }
        self.assertEqual(runner.summarize(list(rows), rows),
                         {"supported": 4, "executed": 1, "failed": 1,
                          "unavailable": 1, "gap": 1, "unexecuted": 2})

    def test_empty_assertions_and_unexplained_gaps_fail_closed(self) -> None:
        for row in ({"status": "executed"}, {"status": "gap"},
                    {"status": "unavailable", "reason": "absent"}, {"status": "unknown"}):
            with self.subTest(row=row), self.assertRaises(ValueError):
                runner.summarize(["a"], {"a": row})

    def test_refusal_check_rejects_a_successful_call_and_wrong_reason(self) -> None:
        with self.assertRaisesRegex(AssertionError, "accepted"):
            runner.refusal(lambda: None, ValueError)
        def wrong() -> None:
            error = ValueError("error")
            error.reason = "wrong"
            raise error
        with self.assertRaisesRegex(AssertionError, "reason"):
            runner.refusal(wrong, ValueError, reason="expected")

    def test_canonical_map_is_exactly_partitioned(self) -> None:
        import json
        operations = json.loads((SCRIPT.parents[1] / "src/conformance/governed-operation-parity.json").read_text())["operations"]
        expected = {item["id"] for item in operations if item["state"] == "live"}
        self.assertEqual(expected, set(runner.PLANNED) | set(runner.UNAVAILABLE))
        self.assertFalse(set(runner.PLANNED) & set(runner.UNAVAILABLE))


if __name__ == "__main__":
    unittest.main()
