"""Literal JSONL query records retain the same strings as the registered G reader."""

from pathlib import Path
import sys
import tempfile
import unittest

from m_client import RpcChild
from m_runtime import read_queries
from m_schedule import closed_loop


ECHO = """
import json,sys
for line in sys.stdin:
    request=json.loads(line)
    print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':request['params']},
                     ensure_ascii=False),flush=True)
"""


class RegisteredQueryTests(unittest.TestCase):
    """Actual harmless RPC children observe decoded strings, not JSON source spellings."""

    def test_literal_jsonl_strings_reach_actual_submissions_in_registered_order(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "queries.jsonl"
            path.write_bytes(b'"river crossing"\n"say \\"alpha\\""\n"line one\\nline two"\n'
                             b'"c:\\\\notes\\\\alpha"\n"caf\xc3\xa9 \\ud83d\\ude00"\n')
            expected = ["river crossing", 'say "alpha"', "line one\nline two",
                        "c:\\notes\\alpha", "café U0001f600"]
            with RpcChild([sys.executable, "-u", "-c", ECHO], root / "rpc") as client:
                result = closed_loop(client, read_queries(path), 4, total=10, mode="text")
                actual = [client.records[key]["response"]["result"]["arguments"]["query"]
                          for key in result["submitted"]]
                self.assertEqual(actual, expected * 2,
                                 "registered JSONL query syntax leaked into actual RPC arguments")
                self.assertEqual([client.records[key]["query_index"] for key in result["submitted"]],
                                 [0, 1, 2, 3, 4] * 2)
            self.assertEqual(client.exit["code"], 0)
