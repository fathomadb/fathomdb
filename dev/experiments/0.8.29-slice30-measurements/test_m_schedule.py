"""Actual echo child requires the initial batch before sending any response."""

from pathlib import Path
import sys
import tempfile
import unittest

from m_client import RpcChild
from m_schedule import closed_loop


REFILL_CHILD = """
import json, sys
c = int(sys.argv[1])
pending = [json.loads(sys.stdin.readline()) for _ in range(c)]
initial = [row['params']['arguments']['query'] for row in pending]
sent = c
for completed in range(33):
    request = pending.pop()
    print(json.dumps({'jsonrpc': '2.0', 'id': request['id'],
                      'result': {'echo': request['params'], 'initial': initial}}), flush=True)
    if sent < 33:
        pending.append(json.loads(sys.stdin.readline()))
        sent += 1
for line in sys.stdin:
    pass
"""


class ClosedLoopTests(unittest.TestCase):
    """Exercise the five accepted client outstanding levels without a database."""

    def test_initial_batch_and_each_terminal_refill_preserve_33_submissions(self):
        queries = ["alpha", "beta", "alpha"]
        with tempfile.TemporaryDirectory() as temporary:
            for concurrency in (1, 4, 8, 16, 32):
                root = Path(temporary) / f"c{concurrency}"
                with RpcChild([sys.executable, "-u", "-c", REFILL_CHILD, str(concurrency)],
                              root, timeout=0.3) as client:
                    result = closed_loop(client, queries, concurrency, total=33, method="echo")
                    self.assertEqual(len(result["submitted"]), 33,
                                     f"c{concurrency} did not submit its required initial batch")
                    self.assertEqual(len(result["completed"]), 33)
                    self.assertEqual(result["outstanding_high_water"], concurrency)
                    self.assertEqual([row["query_index"] for row in client.records.values()],
                                     [index % 3 for index in range(33)])
                    self.assertEqual([row["response"]["result"]["echo"]["arguments"]["query"]
                                      for row in client.records.values()],
                                     [queries[index % 3] for index in range(33)])
                    self.assertEqual(client.records[concurrency]["response"]["result"]["initial"],
                                     [queries[index % 3] for index in range(concurrency)])
                    self.assertEqual(result["completed"],
                                     list(range(concurrency, 34)) + list(range(concurrency - 1, 0, -1)))
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])
                self.assertEqual(client.failures, [])


if __name__ == "__main__":
    unittest.main()
