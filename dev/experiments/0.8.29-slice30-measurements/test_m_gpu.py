"""Hand observations express the accepted same-PID positive-compute distinction."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from g_gpu import GpuObserver
from m_runtime import payload

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


class GpuRefusalRetentionTests(unittest.TestCase):
    def setUp(self):
        self.baseline = {"gpu_memory_mib": 2, "applications": []}
        self.offending = {"uuid": "selected-gpu", "pid": 7, "gpu_memory_mib": 100,
            "pid_memory_mib": 40, "utilization_percent": 20, "observed_at_ns": 1000,
            "monotonic_ns": 900, "applications": [{"pid": 7, "memory_mib": 40},
                                                    {"pid": 99, "memory_mib": 58}]}

    def test_foreign_refusal_carries_the_actual_observation_without_an_extra_query(self):
        with patch("g_gpu.inspect_gpu", side_effect=[self.baseline, copy.deepcopy(self.offending)]) as query:
            observer = GpuObserver({"uuid": "selected-gpu"})
            with self.assertRaisesRegex(RuntimeError, "selected GPU has foreign compute work") as refusal:
                observer.sample(7)
        self.assertEqual(query.call_count, 2)
        expected = dict(self.offending, baseline_gpu_memory_mib=2)
        self.assertEqual(getattr(refusal.exception, "observation", None), expected)

    def test_payload_retains_foreign_record_and_failure_without_qualifying_it(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            queries = root / "queries.jsonl"
            queries.write_text('"alpha"\n' * 500)
            (root / "prepared.json").write_text('{"files":{}}')
            cell = {"family": "M-R", "search_mode": "hybrid", "log": "warn"}
            manifest = {"task": str(root), "builds": {"ordinary": {"binary": {"path": "unused"}}},
                "gpu": {"uuid": "selected-gpu", "ordinal": 0}, "hf_home": str(root),
                "inputs": {"queries": {"path": str(queries)}}, "uuid": "literal-freeze"}

            def completed(*args, on_spawn, **kwargs):
                on_spawn(7)
                return {"complete": True, "pid": 7}

            with patch("m_runtime.worker_admission"), patch("m_runtime.execute", completed), \
                    patch.dict("os.environ"), \
                    patch("g_gpu.inspect_gpu", side_effect=[self.baseline, copy.deepcopy(self.offending)]) as query:
                result = payload(manifest, cell, root, 1000)
            self.assertFalse(result["complete"])
            self.assertEqual(result["gpu"]["failures"],
                             ["RuntimeError: selected GPU has foreign compute work"])
            expected = dict(self.offending, baseline_gpu_memory_mib=2)
            self.assertEqual(result["gpu"].get("failure_observations"), [expected])
            self.assertEqual(result["gpu"]["witnesses"], 0)
            self.assertEqual(query.call_count, 2)
            self.assertEqual([json.loads(line) for line in (root / "gpu.jsonl").read_text().splitlines()],
                             [expected])
            self.assertEqual(json.loads((root / "result.json").read_text())["gpu"]["failure_observations"],
                             [expected])


if __name__ == "__main__":
    unittest.main()
