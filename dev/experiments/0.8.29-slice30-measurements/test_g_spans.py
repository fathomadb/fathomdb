"""Literal interval oracles from the reviewed G phase-math contract."""

import unittest

from g_spans import summarize_spans


def record(span, parent, start, end, *, status="ok", operation=1, phase=None):
    """Construct explicit hand intervals with one session clock origin."""
    return {"engine_id": 7, "session_id": 3, "operation_id": operation,
            "span_id": span, "parent_span_id": parent, "phase": phase or str(span),
            "started_ns": start, "ended_ns": end, "status": status, "facts": {}}


class SpanMathTests(unittest.TestCase):
    def test_literal_nested_tree_counts_only_disjoint_direct_children(self):
        result = summarize_spans([record(1, None, 0, 10), record(2, 1, 1, 4),
                                  record(3, 1, 5, 7), record(4, 2, 2, 3)])
        self.assertTrue(result["complete"])
        spans = {row["span_id"]: row for row in result["spans"]}
        self.assertEqual(spans[1]["inclusive_ns"], 10)
        self.assertEqual(spans[1]["exclusive_remainder_ns"], 5)
        self.assertEqual(spans[1]["additivity"], "nonoverlapping_children")
        self.assertEqual(spans[2]["inclusive_ns"], 3)
        self.assertEqual(spans[2]["exclusive_remainder_ns"], 2)
        self.assertEqual(spans[4]["parent_span_id"], 2)

    def test_overlapping_or_nested_siblings_remain_a_nonadditive_time_tree(self):
        for intervals in [((1, 6), (5, 7)), ((1, 8), (2, 4))]:
            with self.subTest(intervals=intervals):
                result = summarize_spans([record(1, None, 0, 10),
                                          record(2, 1, *intervals[0]),
                                          record(3, 1, *intervals[1])])
                parent = result["spans"][0]
                self.assertEqual(parent["inclusive_ns"], 10)
                self.assertIsNone(parent["exclusive_remainder_ns"])
                self.assertEqual(parent["additivity"], "non_additive")
                self.assertEqual(len(result["spans"]), 3)

    def test_error_not_run_unavailable_and_missing_are_never_zero_cost(self):
        result = summarize_spans([record(1, None, 0, 4, status="error", phase="error"),
                                  record(2, None, None, None, status="not_run"),
                                  record(3, None, None, None, status="unavailable")],
                                 required_phases=["error", "never"])
        self.assertFalse(result["complete"])
        self.assertIn("missing_required_phase:never", result["issues"])
        self.assertEqual(result["spans"][0]["inclusive_ns"], 4)
        for row in result["spans"]:
            self.assertIsNone(row["exclusive_remainder_ns"])
        self.assertIsNone(result["spans"][1]["inclusive_ns"])
        self.assertIsNone(result["spans"][2]["inclusive_ns"])

    def test_collection_loss_prevents_any_exclusive_remainder(self):
        result = summarize_spans([record(1, None, 0, 10), record(2, 1, 1, 4)],
                                 dropped_records=2, issues=["collection_issue"])
        self.assertFalse(result["complete"])
        self.assertIn("dropped_records:2", result["issues"])
        self.assertIn("collection_issue", result["issues"])
        self.assertEqual(result["spans"][0]["inclusive_ns"], 10)
        self.assertTrue(all(row["exclusive_remainder_ns"] is None for row in result["spans"]))

    def test_separate_operations_never_share_children_or_clock_arithmetic(self):
        result = summarize_spans([record(1, None, 0, 10), record(2, 1, 1, 4),
                                  record(1, None, 20, 30, operation=2),
                                  record(2, 1, 21, 28, operation=2)])
        self.assertTrue(result["complete"])
        parents = [row for row in result["spans"] if row["span_id"] == 1]
        self.assertEqual([row["exclusive_remainder_ns"] for row in parents], [7, 3])


    def test_child_outside_actual_parent_invalidates_complete_attribution(self):
        result = summarize_spans([record(1, None, 0, 10), record(2, 1, 9, 11)])
        self.assertFalse(result["complete"], "child escapes its actual parent interval")
        self.assertIn("child_outside_parent", result["issues"])
        self.assertEqual([row["inclusive_ns"] for row in result["spans"]], [10, 2])
        self.assertTrue(all(row["exclusive_remainder_ns"] is None for row in result["spans"]))


if __name__ == "__main__":
    unittest.main()
