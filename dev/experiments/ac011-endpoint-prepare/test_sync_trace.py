import unittest
from decimal import Decimal

from sync_trace import analyze


class SyncTraceMathTests(unittest.TestCase):
    def test_foreground_identity_groups_and_hand_computed_totals(self):
        trace = '''10 99.000000 prctl(PR_GET_DUMPABLE) = 1 <0.000010>
11 100.000000 prctl(PR_SET_NAME, "ac_011a_write_t") = 0 <0.000010>
11 101.000000 fdatasync(7</tmp/fixture/AC-011a_write_throughput.sqlite-wal>) = 0 <1.250000>
11 103.000000 fsync(7</tmp/fixture/AC-011a_write_throughput.sqlite-wal>) = 0 <0.750000>
11 104.000000 fdatasync(8</tmp/fixture/AC-011a_write_throughput.sqlite>) = 0 <3.500000>
11 108.000000 fsync(9</tmp/fixture>) = 0 <0.125000>
11 109.000000 fsync(10</tmp/unrelated>) = 0 <0.250000>
12 110.000000 fsync(7</tmp/fixture/AC-011a_write_throughput.sqlite-wal>) = 0 <10.000000>
'''
        report = analyze(trace.splitlines())
        self.assertEqual(report['foreground_tid'], 11)
        self.assertEqual(report['decision'], 'material_observed_sync_spans')
        self.assertEqual(Decimal(report['foreground_fixture_sync_seconds']), Decimal('5.5'))
        groups = {(g['tid'], g['path_kind']): g for g in report['groups']}
        wal = groups[11, 'wal']
        self.assertEqual((wal['calls'], wal['successes'], wal['errors']), (2, 2, 0))
        self.assertEqual(Decimal(wal['sum_seconds']), Decimal('2.0'))
        self.assertEqual(Decimal(wal['max_seconds']), Decimal('1.25'))
        self.assertEqual(groups[12, 'wal']['role'], 'other')
        self.assertEqual(Decimal(groups[12, 'wal']['sum_seconds']), Decimal('10'))
        self.assertEqual(Decimal(groups[11, 'directory']['sum_seconds']), Decimal('.125'))
        self.assertEqual(Decimal(groups[11, 'other']['sum_seconds']), Decimal('.25'))

    def test_unfinished_resumed_is_one_complete_observed_span(self):
        trace = '''[pid 21] 100.000000 prctl(PR_SET_NAME, "ac_011a_write_t") = 0 <0.000010>
[pid 21] 101.000000 fdatasync(7</tmp/fixture/AC-011a_write_throughput.sqlite-wal> <unfinished ...>
[pid 22] 101.500000 prctl(PR_SET_NAME, "worker") = 0 <0.000010>
[pid 21] 102.500000 <... fdatasync resumed>) = 0 <1.500000>
'''
        report = analyze(trace.splitlines())
        self.assertEqual(report['issues'], [])
        self.assertEqual(report['decision'], 'below_material_span_criterion')
        self.assertEqual(report['groups'][0]['calls'], 1)
        self.assertEqual(Decimal(report['groups'][0]['sum_seconds']), Decimal('1.5'))

    def test_error_duration_is_reported_and_unfinished_call_is_incomplete(self):
        trace = '''31 100.000000 prctl(PR_SET_NAME, "ac_011a_write_t") = 0 <0.000010>
31 101.000000 fsync(7</tmp/fixture/AC-011a_write_throughput.sqlite>) = -1 EIO (Input/output error) <0.250000>
31 102.000000 fdatasync(8</tmp/fixture/AC-011a_write_throughput.sqlite-wal> <unfinished ...>
'''
        report = analyze(trace.splitlines())
        self.assertEqual(report['decision'], 'inconclusive')
        self.assertEqual(report['groups'][0]['errors'], 1)
        self.assertEqual(Decimal(report['groups'][0]['sum_seconds']), Decimal('.25'))
        self.assertTrue(any('unfinished' in issue for issue in report['issues']))


if __name__ == '__main__':
    unittest.main()
