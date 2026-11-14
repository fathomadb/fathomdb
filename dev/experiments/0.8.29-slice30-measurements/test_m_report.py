"""Hand-computed M populations, including a cancellation with no reply."""

import unittest

from m_report import summarize_invocations
from metrics import distribution


class AccountingTests(unittest.TestCase):
    """Keep submitted, returned, success and conditional logged waits distinct."""

    def test_four_submitted_includes_unresolved_without_a_fabricated_latency(self):
        rows = [
            {"rpc_id": 1, "phase": "first", "status": "completed", "ok": True, "elapsed_ns": 1},
            {"rpc_id": 2, "phase": "warmup", "status": "completed", "ok": True, "elapsed_ns": 3},
            {"rpc_id": 3, "phase": "timed", "status": "completed", "ok": False,
             "elapsed_ns": 2, "error": "busy/admission/deadline"},
            {"rpc_id": 4, "phase": "control", "status": "unresolved",
             "cancellation_requested": True},
        ]
        result = summarize_invocations(rows, 2000000000, log_level="debug", stderr=[
            'DEBUG admitted after a wait store="a" waited_ms=7 queued=0',
            'WARN running call cancelled by the client; its engine work keeps its permit',
        ])
        self.assertEqual(result.get("submitted"), 4, "unresolved submission disappeared")
        self.assertEqual(result["completed"], 3)
        self.assertEqual(result["success"], 2)
        self.assertEqual(result["unresolved"], 1)
        self.assertEqual(result["cancellation_requested"], 1)
        self.assertEqual(result["errors"], {"busy/admission/deadline": 1})
        self.assertEqual(result["throughput_submitted_per_s"], 2)
        self.assertEqual(result["throughput_completed_per_s"], 1.5)
        self.assertEqual(result["throughput_success_per_s"], 1)
        self.assertEqual(result["successful_latency"]["mean_ms"], 0.000002)
        self.assertEqual(result["error_latency"]["count"], 1)
        self.assertEqual(result["queue"]["events"], 1)
        self.assertEqual(result["queue"]["whole_process_submitted"], 4)
        self.assertEqual(result["queue"]["phase_submitted"],
                         {"first": 1, "warmup": 1, "timed": 1, "control": 1})
        self.assertEqual(result["queue"]["conditional_wait_latency"]["p50_ms"], 7)
        warn = summarize_invocations(rows, 2000000000)
        self.assertEqual(warn["queue"]["availability"], "unavailable")
        self.assertIsNone(warn["queue"]["events"])
        self.assertIsNone(warn["queue"]["conditional_wait_latency"])

    def test_existing_nearest_rank_ns_math_is_preserved(self):
        result = distribution([1, 2, 3, 100])
        self.assertEqual(result["p50_ms"], 0.000002)
        self.assertEqual(result["p99_ms"], 0.0001)
        self.assertEqual(result["mean_ms"], 0.0000265)


if __name__ == "__main__":
    unittest.main()
