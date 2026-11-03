import unittest
from unittest import mock
from pathlib import Path
import tempfile
import subprocess
import sys
import time
import campaign
from campaign import cells, summarize


class Measurement(unittest.TestCase):
    def test_frozen_grid_has_24_unique_cells(self):
        grid = cells()
        self.assertEqual(len(grid), 24)
        self.assertEqual(len({tuple(row.values()) for row in grid}), 24)
        self.assertEqual({row['arm'] for row in grid}, {'A', 'B'})

    def test_quantiles_and_refused_unfinished_accounting(self):
        events = []
        for i in range(1000):
            events.extend([{'kind': 'offer', 'stream': 'read', 'id': i, 'state': 'started'},
                           {'kind': 'finish', 'stream': 'read', 'id': i, 'outcome': 'success', 'latency_ns': i + 1}])
        result = summarize(events, 'read', True, 1000)
        self.assertEqual((result['p50_ns'], result['p99_ns']), (500, 990))
        self.assertEqual(result['success'], 1000)
        self.assertIsNone(summarize(events, 'read', False, 1000)['p99_ns'])
        events.extend([{'kind': 'offer', 'stream': 'read', 'id': 1000, 'state': 'refused'},
                       {'kind': 'offer', 'stream': 'read', 'id': 1001, 'state': 'started'}])
        result = summarize(events, 'read', True, 1003)
        self.assertEqual((result['refused'], result['unfinished'], result['not_offered']), (1, 1, 1))
        self.assertIsNone(result['p99_ns'])

    def test_duplicates_and_orphan_finishes_cannot_qualify(self):
        events = []
        for i in range(1000):
            events.extend([{'kind':'offer','stream':'read','id':i,'state':'started'},
                           {'kind':'finish','stream':'read','id':i,'outcome':'success','latency_ns':i+1}])
        duplicate = summarize(events + [events[0]], 'read', True, 1000)
        self.assertIsNone(duplicate['p99_ns'])
        self.assertGreater(duplicate['accounting_errors'], 0)
        orphan = summarize(events + [{'kind':'finish','stream':'read','id':9999,'outcome':'success','latency_ns':1}], 'read', True, 1000)
        self.assertIsNone(orphan['p99_ns'])
        self.assertGreater(orphan['accounting_errors'], 0)

    def test_monitor_exception_terminates_and_reaps_child(self):
        processes = []
        original = subprocess.Popen
        def launch(*args, **kwargs):
            process = original(*args, **kwargs)
            processes.append(process)
            return process
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            try:
                with mock.patch.object(campaign.subprocess, 'Popen', side_effect=launch), mock.patch.object(campaign, 'resource_sample', side_effect=RuntimeError('monitor fault')):
                    with self.assertRaisesRegex(RuntimeError, 'monitor fault'):
                        campaign.child([sys.executable, '-c', 'import time; time.sleep(30)'], root/'log', root, time.monotonic()+5)
                self.assertTrue(all(p.poll() is not None for p in processes), 'monitor error leaked child')
            finally:
                for process in processes:
                    if process.poll() is None:
                        process.kill()
                    process.wait()

    def test_deadline_stop_is_reaped_and_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            record = campaign.child([sys.executable, '-c', 'import time; time.sleep(30)'], root/'log', root, time.monotonic()+.01)
            self.assertEqual(record['stop'], 'time_limit')
            self.assertNotEqual(record['exit_code'], 0)
            self.assertGreaterEqual(record['deadline_overshoot_s'], 0)

    def test_late_exit_is_not_mistaken_for_ontime_completion(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            record = campaign.child([sys.executable, '-c', """import time; time.sleep(.05); print('{"status":"complete"}')"""], root/'log', root, time.monotonic()+.03)
            self.assertEqual(record['stop'], 'time_limit')
            self.assertGreater(record['deadline_overshoot_s'], 0)

    def test_resource_stops_and_remaining_dispositions(self):
        for key, limit, reason in [('rss_bytes', 8*campaign.GIB, 'rss_limit'), ('scratch_bytes', 12*campaign.GIB, 'disk_limit')]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                sample = dict(rss_bytes=0, scratch_bytes=0)
                sample[key] = limit+1
                with mock.patch.object(campaign, 'resource_sample', return_value=sample):
                    record = campaign.child([sys.executable, '-c', 'import time; time.sleep(30)'], root/'log', root, time.monotonic()+5)
                self.assertEqual(record['stop'], reason)
                self.assertNotEqual(record['exit_code'], 0)
                results = dict(attempts=[dict(disposition='complete'), dict(disposition='not_run')], controls=[dict(disposition='not_run')])
                campaign.stop_campaign(results, root, time.monotonic(), reason)
                self.assertEqual(results['attempts'][0]['disposition'], 'complete')
                self.assertEqual(results['attempts'][1]['reason'], reason)
                self.assertEqual(results['controls'][0]['reason'], reason)

    def test_malformed_or_late_completion_is_unqualified(self):
        for value in (None, [], 'complete'):
            self.assertFalse(campaign.completed(dict(exit_code=0, result=value)))
        self.assertFalse(campaign.completed(dict(exit_code=0, result={'status':'complete'}, deadline_overshoot_s=.1)))
        self.assertTrue(campaign.completed(dict(exit_code=0, result={'status':'complete'})))

    def test_error_outcome_keeps_quantiles_null(self):
        events = [{'kind':'offer','stream':'read','id':0,'state':'started'}, {'kind':'finish','stream':'read','id':0,'outcome':'error','latency_ns':1}]
        result = summarize(events, 'read', True, 1)
        self.assertEqual(result['error'], 1)
        self.assertIsNone(result['p50_ns'])


if __name__ == '__main__':
    unittest.main()
