"""Question identity/order/UTF8 projection controls for the generic G query feed."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from hypothesis import Phase, given, settings, strategies as st


class QueryEncoderTests(unittest.TestCase):
    def require_projection(self, queries):
        root = Path(tempfile.mkdtemp(prefix="s30-g-query-input-", dir=os.environ["TMPDIR"]))
        source = root / "queries.jsonl"
        payload = b"".join((json.dumps(query) + "\n").encode("utf-8") for query in queries)
        source.write_bytes(payload)
        output = root / "query-records.jsonl"
        tool = Path(__file__).with_name("g_inputs.py")
        command = [sys.executable, str(tool), "--queries", str(source), "--out", str(output)]
        identity = {"command": command, "cwd": str(Path.cwd()),
                    "source": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                    "delta": subprocess.check_output(["git", "status", "--porcelain"], text=True),
                    "python_sha256": hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(),
                    "oracle_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "tool_sha256": hashlib.sha256(tool.read_bytes()).hexdigest(),
                    "input_sha256": hashlib.sha256(payload).hexdigest()}
        (root / "before-runtime.json").write_text(json.dumps(identity, indent=2) + "\n")
        child = subprocess.run(command, capture_output=True, text=True, timeout=10)
        (root / "stdout.log").write_text(child.stdout)
        (root / "stderr.log").write_text(child.stderr)
        if child.returncode:
            raise RuntimeError("question encoder setup/CLI failure, NOT_RED: " + child.stderr)
        result = json.loads(child.stdout)
        actual = [json.loads(line) for line in output.read_bytes().splitlines()]
        if result != {"records": len(queries)} or source.read_bytes() != payload:
            raise RuntimeError("existing CLI/count/source preservation controls failed")
        print(json.dumps({"root": str(root), "cli_exit": child.returncode,
                          "records": result["records"], "framed_rows": len(actual)}), flush=True)
        expected = [{"id": index, "query": query} for index, query in enumerate(queries)]
        self.assertEqual(actual, expected, "missing source-order question ID/text projection")
        self.assertEqual([row["query"].encode("utf-8") for row in actual],
                         [query.encode("utf-8") for query in queries])
        self.assertEqual(len(output.read_bytes().splitlines()), len(queries))

    def test_literal_duplicate_questions_keep_distinct_ids_and_exact_text(self):
        self.require_projection(["alphaonly", "βeta\nsecond line", "alphaonly", 'quoted "épreuve" \\ tail'])

    @settings(max_examples=8, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(queries=st.lists(st.text(st.characters(blacklist_categories=("Cs",)), min_size=1, max_size=40),
                           min_size=1, max_size=8))
    def test_selected_question_utf8_order_count_and_roundtrip(self, queries):
        self.require_projection(queries)


if __name__ == "__main__":
    unittest.main()
