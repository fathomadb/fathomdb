"""Literal encoded-byte limits retain prefixes and cannot publish shortened success."""

import json
from pathlib import Path
import sys
import tempfile
import time
import subprocess
import unittest
from unittest.mock import patch

from m_client import RpcChild
import m_record
import m_runner
from m_record import CheckpointLog
from m_runner import report
import test_m_selected_report


class OutputLimitTests(unittest.TestCase):
    def test_checkpoint_limit_counts_escaped_utf8_before_writing_the_next_record(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "capture.jsonl"
            expected = b'{"text":"\\u0000\xc3\xa9\xf0\x9f\x98\x80"}\n'
            with patch("m_record.JSONL_MAX_BYTES", len(expected), create=True):
                log = CheckpointLog(path)
                try:
                    log.write({"text": "\x00é😀"})
                    with self.assertRaisesRegex(RuntimeError, "capture limit"):
                        log.write({"text": "another"})
                finally:
                    log.close()
            self.assertEqual(path.read_bytes(), expected)

    def test_raw_overflow_does_not_write_the_line_past_its_stream_limit(self):
        with tempfile.TemporaryDirectory() as temporary:
            child = "import sys;sys.stdout.buffer.write(b'abcd\\n');sys.stdout.flush()"
            with RpcChild([sys.executable, "-c", child], temporary, max_stream_bytes=4) as client:
                pass
            self.assertTrue(client.transport_failures)
            self.assertLessEqual((Path(temporary) / "stdout.raw").stat().st_size, 4)
            self.assertTrue(client.exit["readers_joined"])

    def test_checkpoint_overflow_still_closes_the_actual_child_and_readers(self):
        with tempfile.TemporaryDirectory() as temporary:
            child = "import sys;[None for line in sys.stdin]"
            with patch("m_record.JSONL_MAX_BYTES", 16, create=True):
                with self.assertRaisesRegex(RuntimeError, "capture limit"):
                    with RpcChild([sys.executable, "-c", child], temporary) as client:
                        client.submit("echo", {"value": "too large"})
            self.assertEqual(client.exit["code"], 0)
            self.assertTrue(client.exit["readers_joined"])
            self.assertTrue(any("capture limit" in failure for failure in client.failures))

    def test_control_log_overflow_keeps_a_prefix_and_exposes_failure(self):
        log_type = getattr(m_record, "BoundedLog", None)
        self.assertTrue(callable(log_type), "existing control logs are not bounded before writes")
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "child.log"
            with patch("m_record.LOG_MAX_BYTES", 16, create=True):
                with log_type(path) as log:
                    child = subprocess.Popen([sys.executable, "-c", "print('x'*100)"], stdout=log)
                    log.start()
                    self.assertEqual(child.wait(timeout=5), 0)
            self.assertLessEqual(path.stat().st_size, 16)
            self.assertIn("capture limit", log.error)

    def test_guardian_log_overflow_prevents_success_after_owned_child_exit_zero(self):
        from test_m_calibration_guard import CalibrationGuardianTests
        fixture = CalibrationGuardianTests()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        with patch("m_record.LOG_MAX_BYTES", 16, create=True):
            result = fixture.run_child("print('x'*100)", "overflow")
        self.assertNotEqual(result["exit"], 0)
        self.assertFalse(result["cleanup_uncertain"])
        value = json.loads(fixture.ledger.read_text())
        self.assertIsNone(value["m_guardian"])
        self.assertIn("capture limit", result.get("error", ""))

    def test_outer_capture_failure_revokes_the_existing_publication_after_cleanup(self):
        from test_m_publication_write_cap import PublicationWriteCapTests
        fixture = PublicationWriteCapTests()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        first = report(fixture.root)
        self.assertTrue(first["phase_qualified"])
        invalidate = getattr(m_runner, "invalidate_selected", None)
        self.assertTrue(callable(invalidate), "outer capture failure cannot revoke published credit")
        value = invalidate(fixture.root, "service capture limit")
        self.assertFalse(value["phase_qualified"])
        self.assertFalse(next(row for row in value["rows"] if row["id"] == fixture.cell["id"])["complete"])
        self.assertNotEqual(json.loads(fixture.guardian_path.read_text())["exit"], 0)
        self.assertGreater(json.loads(fixture.ledger_path.read_text())["used_ns"], 1000)

    def test_shared_budget_writer_keeps_a_valid_24MiB_non_m_history(self):
        from m_guard import write_budget
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "d-state.json"
            write_budget(path, {"history": "x" * (24 * 1024**2)})
            self.assertEqual(len(json.loads(path.read_text())["history"]), 24 * 1024**2)

    def test_non_m_attempt_retains_its_original_uncapped_control_log(self):
        from m_attempt import run_attempt
        with tempfile.TemporaryDirectory() as temporary:
            command = [sys.executable, "-c",
                "import pathlib,sys;print('x'*100);"
                "(pathlib.Path(sys.argv[2])/'result.json').write_text('{\"complete\":true}')"]
            with patch("m_record.LOG_MAX_BYTES", 16):
                outcome = run_attempt({"id": "D-independent", "family": "D-R"}, temporary,
                                      command, deadline_ns=time.monotonic_ns()+15*10**9)
            self.assertTrue(outcome["complete"])
            self.assertGreater((Path(outcome["attempt"]) / "child.log").stat().st_size, 16)

    def test_invocation_finalization_charges_outer_work_without_renewing_the_phase(self):
        from test_m_publication_write_cap import PublicationWriteCapTests
        fixture = PublicationWriteCapTests()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        report(fixture.root)
        finalize = getattr(m_runner, "finalize_invocation", None)
        self.assertTrue(callable(finalize), "outer capture work is outside cumulative phase accounting")
        with patch("m_runner.time.monotonic_ns", return_value=fixture.clock):
            value = finalize(fixture.root, fixture.clock-3*10**9, 897*10**9)
        self.assertFalse(value["phase_qualified"])
        usage = json.loads(fixture.ledger_path.read_text())["selected_cell_usage"][fixture.cell["id"]]
        self.assertEqual(usage["wall_ns"], 900*10**9)

    def test_publication_overflow_retains_all46_rows_but_cannot_qualify_the_cell(self):
        fixture = test_m_selected_report.SelectedReportTests()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        guard = fixture.root / "guardians" / "original"
        guard.mkdir(parents=True)
        (guard / "guardian.json").write_text(json.dumps(fixture.fixture.guardian))
        with patch("m_record.JSON_MAX_BYTES", 1024, create=True):
            value = report(fixture.root)
        self.assertFalse(value["phase_qualified"])
        self.assertFalse(value["complete"])
        self.assertEqual(len(value["rows"]), 46)
        self.assertIn("output_overflow", value)
        saved = json.loads(sorted((fixture.root / "reports").glob("report-*.json"))[-1].read_text())
        self.assertFalse(saved["phase_qualified"])
        self.assertEqual(len(saved["rows"]), 46)


if __name__ == "__main__":
    unittest.main()
