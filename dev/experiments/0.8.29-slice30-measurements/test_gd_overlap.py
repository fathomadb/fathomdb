"""Literal interval oracles and bounded translation invariants."""

import json
from pathlib import Path
import subprocess
import sys
import unittest

from hypothesis import given, settings, strategies as st

TOOL = Path(__file__).with_name("gd_overlap.py")


def span(start, end, **fields):
    return {"start_ns": start, "end_ns": end, "ok": True, **fields}


def run(query=(0, 10), write=(1, 9), worker=(2, 8), pending=(3, 4), count=1,
        enabled=True, dropped=0):
    payload = {"queries": [span(*query)], "writes": [span(*write)],
               "workers": [span(*worker, worker_idx=0, batch_id=7)],
               "readiness": [span(*pending, pending_count=count)],
               "enabled": enabled, "dropped": dropped}
    child = subprocess.run([sys.executable, str(TOOL)], input=json.dumps(payload),
                           capture_output=True, text=True, timeout=5)
    if child.returncode:
        raise AssertionError(child.stderr)
    return json.loads(child.stdout)


class OverlapTests(unittest.TestCase):
    def test_strict_common_interval_and_positive_full_readiness_call(self):
        result = run()
        self.assertEqual(result.get("status"), "OBSERVED_OVERLAP")
        self.assertEqual(result["witnesses"], [{"query": 0, "write": 0, "worker": 0,
                                             "readiness": 0, "start_ns": 2, "end_ns": 8}])
        self.assertEqual(result["outside_readiness"], [])

    def test_disjoint_pairwise_intersections_are_not_triple_overlap(self):
        result = run(write=(1, 2), worker=(8, 9), pending=(1, 1))
        self.assertEqual(result.get("status"), "INCOMPLETE_OVERLAP")
        self.assertEqual(result["witnesses"], [])
        self.assertEqual(result["partial_intersections"], 1)

    def test_boundary_only_contact_is_incomplete(self):
        result = run(write=(1, 2), worker=(2, 8), pending=(2, 2))
        self.assertEqual(result.get("status"), "INCOMPLETE_OVERLAP")
        self.assertEqual(result["witnesses"], [])

    def test_pending_invocation_must_fit_wholly_inside_common_interval(self):
        for pending in [(1, 3), (3, 8), (2, 4), (8, 9)]:
            with self.subTest(pending=pending):
                result = run(pending=pending)
                self.assertEqual(result.get("status"), "INCOMPLETE_OVERLAP")
                self.assertEqual(result["outside_readiness"], [0])

    def test_zero_pending_does_not_witness_overlap(self):
        result = run(count=0)
        self.assertEqual(result.get("status"), "INCOMPLETE_OVERLAP")
        self.assertEqual(result["witnesses"], [])

    def test_disabled_control_keeps_worker_attribution_unavailable(self):
        result = run(enabled=False)
        self.assertEqual(result.get("status"), "WORKER_ATTRIBUTION_UNAVAILABLE")
        self.assertIsNone(result["witnesses"])
        self.assertIsNone(result["partial_intersections"])

    def test_loss_retains_witness_but_prevents_complete_attribution(self):
        result = run(dropped=1)
        self.assertEqual(result.get("status"), "INCOMPLETE_ATTRIBUTION")
        self.assertEqual(len(result["witnesses"]), 1)

    @settings(max_examples=8, deadline=None)
    @given(offset=st.integers(min_value=0, max_value=10**9),
           width=st.integers(min_value=5, max_value=10**6))
    def test_translation_preserves_strict_witness_geometry(self, offset, width):
        result = run(query=(offset, offset+width+6), write=(offset+1, offset+width+5),
                     worker=(offset+2, offset+width+4), pending=(offset+3, offset+width+3))
        self.assertEqual(result.get("status"), "OBSERVED_OVERLAP")
        self.assertEqual((result["witnesses"][0]["start_ns"], result["witnesses"][0]["end_ns"]),
                         (offset+2, offset+width+4))


def worker_analysis(workers, complete=True):
    payload = {"queries": [span(0, 10)], "writes": [span(1, 9)],
               "workers": workers, "readiness": [span(3, 4, pending_count=1)],
               "attribution_complete": complete}
    child = subprocess.run([sys.executable, str(TOOL)], input=json.dumps(payload),
                           capture_output=True, text=True, timeout=5)
    if child.returncode:
        raise AssertionError(child.stderr)
    return json.loads(child.stdout)


class AttributionCompletenessTests(unittest.TestCase):
    def test_invalid_worker_cannot_be_skipped_to_claim_complete_attribution(self):
        for invalid in [span(None, None, clock_issue="unavailable"),
                        span(2, 8, clock_issue="unavailable"), {}]:
            with self.subTest(invalid=invalid):
                result = worker_analysis([span(2, 8), invalid])
                self.assertEqual(result.get("status"), "INCOMPLETE_ATTRIBUTION")
                self.assertEqual(len(result["witnesses"]), 1)
                self.assertEqual(result["unavailable_workers"], [1])

    def test_native_adapter_missing_record_flag_prevents_complete_attribution(self):
        result = worker_analysis([span(2, 8)], complete=False)
        self.assertEqual(result.get("status"), "INCOMPLETE_ATTRIBUTION")
        self.assertEqual(len(result["witnesses"]), 1)

    def test_missing_worker_stream_is_not_complete_zero_concurrency(self):
        result = worker_analysis([])
        self.assertEqual(result.get("status"), "INCOMPLETE_ATTRIBUTION")
        self.assertEqual(result["witnesses"], [])


class ReadinessSuccessTests(unittest.TestCase):
    def test_failed_readiness_with_stale_positive_count_cannot_witness_overlap(self):
        payload = {"queries": [span(0, 10)], "writes": [span(1, 9)],
                   "workers": [span(2, 8)],
                   "readiness": [span(3, 4, pending_count=1, ok=False)]}
        child = subprocess.run([sys.executable, str(TOOL)], input=json.dumps(payload),
                               capture_output=True, text=True, timeout=5)
        self.assertEqual(child.returncode, 0, child.stderr)
        result = json.loads(child.stdout)
        self.assertEqual(result["status"], "INCOMPLETE_OVERLAP")
        self.assertEqual(result["witnesses"], [])
        self.assertEqual(result["outside_readiness"], [0])


if __name__ == "__main__":
    unittest.main()
