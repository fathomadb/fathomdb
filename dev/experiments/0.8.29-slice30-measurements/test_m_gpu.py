"""Hand observations express the accepted same-PID positive-compute distinction."""

import unittest

from m_gpu import qualification


class GpuQualificationTests(unittest.TestCase):
    """No old/foreign PID or memory-only observation supplies a hybrid witness."""

    def test_actual_compute_requires_same_pid_memory_increase_and_positive_utilization(self):
        hybrid = {"family": "M-R", "search_mode": "hybrid"}
        rows = [{"pid": 8, "pid_memory_mib": 10, "gpu_memory_mib": 20,
                 "baseline_gpu_memory_mib": 10, "utilization_percent": 5}]
        self.assertFalse(qualification(hybrid, rows, 7)["complete"])
        rows.append({"pid": 7, "pid_memory_mib": 10, "gpu_memory_mib": 20,
                     "baseline_gpu_memory_mib": 10, "utilization_percent": 0})
        self.assertFalse(qualification(hybrid, rows, 7)["complete"])
        rows.append(dict(rows[-1], utilization_percent=1))
        self.assertTrue(qualification(hybrid, rows, 7)["complete"])
        self.assertEqual(qualification(hybrid, rows, 7)["witnesses"], 1)
        for cell in ({"family": "M-C", "search_mode": "text"},
                     {"family": "M0", "search_mode": "text"},
                     {"family": "M-E", "search_mode": "text"}):
            self.assertEqual(qualification(cell, [], 7)["requirement"], "N/A")
            self.assertTrue(qualification(cell, [], 7)["complete"])


if __name__ == "__main__":
    unittest.main()
