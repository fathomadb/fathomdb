import unittest
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


if __name__ == '__main__':
    unittest.main()
