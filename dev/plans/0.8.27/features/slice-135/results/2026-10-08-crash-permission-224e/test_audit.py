"""The crash and refusal receipt auditor must reject false state reports."""

import unittest

from audit import audit_receipt, check_records, read_records, ROOT


class CrashPermissionAuditTest(unittest.TestCase):
    def test_retained_receipt_replays_with_manifest_snapshot(self):
        self.assertEqual(audit_receipt(ROOT)["status"], "PASS")

    def test_retained_records_pass(self):
        check_records(*read_records(ROOT))

    def test_false_recovery_and_integrity_are_rejected(self):
        stdout, stderr = read_records(ROOT)
        with self.assertRaises(ValueError):
            check_records(stdout, stderr.replace('"vector":true', '"vector":false'))
        with self.assertRaises(ValueError):
            check_records(
                stdout, stderr.replace('"integrity":["ok"]', '"integrity":["corrupt"]')
            )

    def test_missing_pass_and_typed_error_are_rejected(self):
        stdout, stderr = read_records(ROOT)
        with self.assertRaises(ValueError):
            check_records(stdout.replace("3 passed", "2 passed"), stderr)
        with self.assertRaises(ValueError):
            check_records(stdout, stderr.replace('"Storage"', '"Busy"', 1))


if __name__ == "__main__":
    unittest.main()
