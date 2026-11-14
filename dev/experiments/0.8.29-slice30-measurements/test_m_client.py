"""Actual harmless echo children; these controls do not emulate a database."""

import sys
import tempfile
import unittest

from m_client import RpcChild


REVERSE_PAIR = """
import json, sys
requests = [json.loads(sys.stdin.readline()) for _ in range(2)]
for request in reversed(requests):
    print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': request['params']}), flush=True)
for line in sys.stdin:
    pass
"""


class InvocationTests(unittest.TestCase):
    """Associate actual out-of-order child replies with distinct invocations."""

    def test_reverse_replies_keep_both_request_identities(self):
        with tempfile.TemporaryDirectory() as root:
            with RpcChild([sys.executable, "-u", "-c", REVERSE_PAIR], root) as client:
                first = client.submit("echo", {"query": "same", "ordinal": 0},
                                      phase="timed", query_index=0, submission_index=0)
                second = client.submit("echo", {"query": "same", "ordinal": 1},
                                       phase="timed", query_index=0, submission_index=1)
                self.assertEqual(client.wait(first)["result"], {"query": "same", "ordinal": 0})
                answer = client.wait(second, timeout=0.2)
                self.assertEqual(answer, {"jsonrpc": "2.0", "id": second,
                                          "result": {"query": "same", "ordinal": 1}},
                                 "an already-arrived second reply was lost")
                self.assertNotEqual(first, second)
                self.assertEqual(len(client.records), 2)
                for row in client.records.values():
                    self.assertEqual(row["status"], "completed")
                    self.assertLessEqual(row["send_start_ns"], row["flush_end_ns"])
                    self.assertLessEqual(row["send_start_ns"], row["arrival_ns"])
                    self.assertEqual(row["elapsed_ns"], row["arrival_ns"] - row["send_start_ns"])
            self.assertEqual(client.exit["code"], 0)
            self.assertTrue(client.exit["readers_joined"])
            self.assertEqual(client.failures, [])


if __name__ == "__main__":
    unittest.main()
