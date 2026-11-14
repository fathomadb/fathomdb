"""Real partial child output and hand-specified invocation checkpoint counts."""

from pathlib import Path
import sys
import tempfile
import unittest

from control import read_jsonl_records
from m_client import RpcChild


PARTIAL_CHILD = """
import json, sys
for index in range(16):
    request = json.loads(sys.stdin.readline())
    print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': request['params']}), flush=True)
sys.stdout.buffer.write(b'{"jsonrpc":"2.0","id":')
sys.stdout.buffer.flush()
sys.exit(7)
"""


class PersistenceTests(unittest.TestCase):
    """An actual child failure preserves completed checkpoints, not qualification."""

    def test_sixteen_completed_invocations_are_checkpointed_before_close(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with RpcChild([sys.executable, "-u", "-c", PARTIAL_CHILD], root) as client:
                ids = [client.submit("echo", {"ordinal": index}, phase="timed",
                                     submission_index=index, query_index=index % 2)
                       for index in range(16)]
                for request_id in ids:
                    client.wait(request_id)
                path = root / "invocations.jsonl"
                issues = []
                records = read_jsonl_records(path, issues) if path.exists() else []
                self.assertEqual(len(records), 16,
                                 "completed invocation checkpoint was not persisted")
                self.assertEqual([row["rpc_id"] for row in records], list(range(1, 17)))
                self.assertEqual([row["response"]["result"]["ordinal"] for row in records],
                                 list(range(16)))
                self.assertEqual(issues, [])
            self.assertEqual(client.exit["code"], 7)
            self.assertTrue(client.exit["readers_joined"])
            self.assertIn("stdout: bounded stream incomplete", client.failures)
            issues = []
            stdout = read_jsonl_records(root / "stdout.raw", issues)
            self.assertEqual(len(stdout), 16)
            self.assertTrue(issues)
            self.assertTrue((root / "stdout.raw").read_bytes().endswith(b'"id":'))

    def test_raw_stream_write_and_flush_costs_remain_separate_from_invocations(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with RpcChild([sys.executable, "-u", "-c", PARTIAL_CHILD], root) as client:
                for index in range(16):
                    client.wait(client.submit("echo", {"ordinal": index}))
            stdout = client.exit["persistence"].get("raw", {}).get("stdout", {})
            self.assertEqual(stdout.get("encoded_bytes", 0), len((root / "stdout.raw").read_bytes()),
                             "raw saved bytes/costs disappeared from persistence accounting")
            self.assertEqual(stdout["records"], 17)
            self.assertEqual(stdout["checkpoints"], 17)
            self.assertIsInstance(stdout["persistence_ns"], int)
            self.assertGreaterEqual(stdout["persistence_ns"], stdout["checkpoint_ns"])


if __name__ == "__main__":
    unittest.main()
