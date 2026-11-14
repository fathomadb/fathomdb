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

    def test_normal_close_drains_arrived_responses_before_unresolved_disposition(self):
        with tempfile.TemporaryDirectory() as root:
            client = RpcChild([sys.executable, "-u", "-c", REVERSE_PAIR], root)
            ids = [client.submit("echo", {"ordinal": index}) for index in range(2)]
            client.close()
            self.assertEqual(client.exit["code"], 0)
            self.assertEqual([client.records[key]["status"] for key in ids],
                             ["completed", "completed"])
            self.assertEqual([client.records[key]["response"]["result"]["ordinal"]
                              for key in ids], [0, 1])

    def test_malformed_and_unassociated_frames_do_not_erase_a_later_actual_reply(self):
        child = """
import json,sys
r=json.loads(sys.stdin.readline())
print('null',flush=True)
print('{broken',flush=True)
print(json.dumps({'jsonrpc':'2.0','id':999,'result':{}}),flush=True)
print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':{'literal':'retained'}}),flush=True)
for line in sys.stdin: pass
"""
        with tempfile.TemporaryDirectory() as root:
            with RpcChild([sys.executable, "-u", "-c", child], root) as client:
                request = client.submit("echo", {})
                self.assertEqual(client.wait(request)["result"], {"literal": "retained"})
                self.assertEqual(len(client.protocol_failures), 3)
                self.assertEqual(client.records[request]["status"], "completed")
            self.assertEqual(client.exit["code"], 0)
            self.assertTrue(client.exit["readers_joined"])


if __name__ == "__main__":
    unittest.main()
