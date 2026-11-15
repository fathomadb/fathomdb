"""Actual D CLI admission for inherited treatment settings; harmless payloads only."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


KEYS = ("FATHOMDB_PROJECTION_BATCH", "FATHOMDB_PERF_EXPERIMENTS",
        "FATHOMDB_PERF_READER_PRAGMAS", "FATHOMDB_PERF_WRITER_PRAGMAS", "FATHOMDB_PERF_SEARCH_LIMIT")


class DInheritedSettingsTests(unittest.TestCase):
    def invoke(self, root, settings):
        """Run an isolated actual CLI environment without changing this test process."""
        environment = {key: value for key, value in os.environ.items() if key not in KEYS}
        environment.update(settings)
        result = subprocess.run([sys.executable, str(Path(__file__).with_name("d_runner.py")),
            "source-control", "--root", str(root)], env=environment, text=True,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30)
        self.assertTrue((root/"d-report.json").exists(), result.stdout)
        return result, json.loads((root/"d-report.json").read_text())

    def test_actual_cli_records_treatments_and_refuses_before_any_payload(self):
        task = Path(os.environ["S30_TASK"])
        cases = [
            {"FATHOMDB_PROJECTION_BATCH": value} for value in ("1", "true", "on")
        ] + [
            {"FATHOMDB_PERF_EXPERIMENTS": "1", "FATHOMDB_PERF_READER_PRAGMAS": "cache_size=-64"},
            {"FATHOMDB_PERF_EXPERIMENTS": "1", "FATHOMDB_PERF_WRITER_PRAGMAS": "synchronous=OFF"},
            {"FATHOMDB_PERF_EXPERIMENTS": "1", "FATHOMDB_PERF_SEARCH_LIMIT": "1"},
            {"FATHOMDB_PERF_EXPERIMENTS": "0", "FATHOMDB_PERF_SEARCH_LIMIT": "0"},
        ]
        for settings in cases:
            with self.subTest(settings=settings), tempfile.TemporaryDirectory(prefix="d-env-", dir=task/"tmp") as temporary:
                root = Path(temporary)/"source"
                result, report = self.invoke(root, settings)
                self.assertFalse((root/"dispatches.jsonl").exists(), "inherited treatment reached D payload")
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(report["source_control_complete"])
                self.assertEqual(report["inherited_settings"], settings)
                self.assertEqual(len(report["rows"]), 24)
                self.assertTrue(all(row["status"] == "not_run_inherited_settings" for row in report["rows"][:4]))
                self.assertTrue(all(row["inherited_settings"] == settings for row in report["rows"][:4]))
                self.assertTrue(all(row["status"] == "not_run_stage_review" for row in report["rows"][4:]))
                self.assertFalse(list(root.rglob("store.sqlite")))

    def test_disabled_batch_and_inactive_perf_values_keep_default_admission(self):
        task = Path(os.environ["S30_TASK"])
        for settings in ({"FATHOMDB_PROJECTION_BATCH": "0"},
                         {"FATHOMDB_PERF_READER_PRAGMAS": "cache_size=-64", "FATHOMDB_PERF_SEARCH_LIMIT": "1"},
                         {"FATHOMDB_PERF_EXPERIMENTS": "1", "FATHOMDB_PERF_SEARCH_LIMIT": "invalid"}):
            with self.subTest(settings=settings), tempfile.TemporaryDirectory(prefix="d-env-default-", dir=task/"tmp") as temporary:
                root = Path(temporary)/"source"
                result, report = self.invoke(root, settings)
                self.assertEqual(result.returncode, 0, result.stdout)
                self.assertTrue(report["source_control_complete"])
                self.assertEqual(len((root/"dispatches.jsonl").read_text().splitlines()), 4)


if __name__ == "__main__":
    unittest.main()
