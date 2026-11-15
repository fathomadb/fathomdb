"""Literal exact-execution overlap oracles; no runtime or size qualification."""

import copy
import unittest

from d_overlap import classify_overlap


def literal_case(n=10000, repeat=0, *, intersect=True):
    """Build hand intervals; only[15,20] inside query[10,30]/worker[0,40] qualifies."""
    cell = {"cell_id": f"d-c-overlap-n{n}-r{repeat}", "attempt_id": "one",
            "n": n, "repeat": repeat, "layer": "C"}
    query = (10, 30) if intersect else (50, 60)
    pending = (15, 20) if intersect else (51, 52)
    records = [{"cell": copy.deepcopy(cell), "record": {
        "engine_id": 42, "session_id": 7, "operation_id": operation,
        "span_id": span, "parent_span_id": None, "phase": phase, "status": "ok",
        "started_ns": start, "ended_ns": end,
    }} for phase, operation, span, start, end in (
        ("api_hybrid", 17, 1, *query),
        ("projection_processing", 1 << 63, 2, 0, 40),
    )]
    return {"cell": cell, "engine_id": 42, "session_id": 7, "operation_id": 17,
            "records": records, "dropped_records": 0, "issues": [],
            "pending": [{"cell": copy.deepcopy(cell), "engine_id": 42, "session_id": 7,
                         "started_ns": pending[0], "ended_ns": pending[1], "pending_count": 2}]}


class DOverlapPredicateTests(unittest.TestCase):
    def test_no_intersection_and_straddling_pending_are_not_observed_overlap(self):
        cases = [literal_case(intersect=False), literal_case(), literal_case()]
        cases[1]["pending"][0]["started_ns"] = 9
        cases[2]["pending"][0]["ended_ns"] = 31
        for case in cases:
            with self.subTest(pending=case["pending"]):
                self.assertEqual(classify_overlap(case)["verdict"], "NO_OBSERVED_OVERLAP")

    def test_t_layer_remains_unobserved_even_when_a_c_execution_has_a_witness(self):
        case = literal_case()
        case["cell"] = {**case["cell"], "layer": "T", "cell_id": "d-t-overlap-n10000-r0"}
        self.assertEqual(classify_overlap(case)["verdict"], "ACTUAL_OVERLAP_UNOBSERVED")

    def test_two_sizes_two_repeats_never_borrow_the_first_cells_overlap(self):
        cases = [literal_case(n, repeat, intersect=(n == 10000 and repeat == 0))
                 for n in (10000, 100000) for repeat in (0, 1)]
        records = [row for case in cases for row in case["records"]]
        pending = [row for case in cases for row in case["pending"]]
        for case in cases:
            with self.subTest(n=case["cell"]["n"], repeat=case["cell"]["repeat"]):
                case["records"], case["pending"] = records, pending
                expected = "OBSERVED_OVERLAP" if (case["cell"]["n"], case["cell"]["repeat"]) == (10000, 0) else "NO_OBSERVED_OVERLAP"
                self.assertEqual(classify_overlap(case)["verdict"], expected)

    def test_other_call_and_previous_attempt_cannot_supply_the_target_witness(self):
        positive = literal_case()
        target = literal_case(intersect=False)
        target["operation_id"] = 18
        target["records"][0]["record"]["operation_id"] = 18
        target["records"] += positive["records"]
        target["pending"] += positive["pending"]
        self.assertEqual(classify_overlap(target)["verdict"], "NO_OBSERVED_OVERLAP")
        target = literal_case(intersect=False)
        target["cell"]["attempt_id"] = "two"
        for row in target["records"] + target["pending"]:
            row["cell"]["attempt_id"] = "two"
        target["records"] += positive["records"]
        target["pending"] += positive["pending"]
        self.assertEqual(classify_overlap(target)["verdict"], "NO_OBSERVED_OVERLAP")

    def test_missing_or_lost_observations_remain_incomplete(self):
        for key, value in (("records", []), ("pending", []), ("dropped_records", 1), ("issues", ["unavailable"])):
            case = literal_case()
            case[key] = value
            self.assertEqual(classify_overlap(case)["verdict"], "INCOMPLETE")


if __name__ == "__main__":
    unittest.main()
