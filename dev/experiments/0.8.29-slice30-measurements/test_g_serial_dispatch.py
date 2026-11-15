"""The last dispatch check sees actual output growth and keeps the live envelope."""

from pathlib import Path
import tempfile
import unittest

from g_campaign import run_g_plan
from test_g_serial_controller import child, prepared, GiB


class GSerialDispatchTests(unittest.TestCase):
    def test_shared_growth_during_command_preparation_refuses_before_popen(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = prepared()
            def command(_cell, _deadline):
                with (root/"late-shared-output").open("wb") as f:
                    f.truncate(40*GiB)
                return child(root)
            run_g_plan(spec, root/"results", command, allocation_root=root, reused=reused)
            self.assertFalse((root/"calls").exists(),
                             "last command/context serialization was not charged before payload dispatch")

    def test_live_case_output_excess_is_not_given_closed_case_credit(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            spec, reused = prepared()
            command = child(root)
            command[-1] = command[-1].replace("sys.exit(int(failed))", "\n".join([
                "if first:\n with (out/'unexpected-live-output').open('wb') as f:f.truncate(9*1024**3)",
                "sys.exit(int(failed))",
            ]))
            rows = run_g_plan(spec, root/"results", command, allocation_root=root, reused=reused)
            first = next(row for row in rows if row["id"] == "g-w-b1-s1-engine-text-r0")
            self.assertEqual(first["status"], "failed",
                             "live bytes above L_i were silently charged using only eventual closed bytes")
            self.assertEqual(first["error"], "serial_live_storage_exhausted")
