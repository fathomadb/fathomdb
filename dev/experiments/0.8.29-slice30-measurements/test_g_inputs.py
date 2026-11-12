"""Initial normal-G projection/framing oracles for selected real JSONL bytes."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from hypothesis import Phase, given, settings, strategies as st


class RecordEncoderTests(unittest.TestCase):
    def require_projection(self, bodies):
        root = Path(tempfile.mkdtemp(prefix="s30-g-input-",dir=os.environ["TMPDIR"]))
        source = root/"corpus.jsonl"
        payload = b"".join((json.dumps(body)+"\n").encode("utf-8") for body in bodies)
        source.write_bytes(payload)
        output = root/"records.jsonl"
        tool = Path(__file__).with_name("g_inputs.py")
        command = [sys.executable,str(tool),"--corpus",str(source),"--out",str(output)]
        identity = {"command":command,"cwd":str(Path.cwd()),
            "source":subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),
            "python_sha256":hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(),
            "oracle_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "tool_sha256":hashlib.sha256(tool.read_bytes()).hexdigest(),"input_sha256":hashlib.sha256(payload).hexdigest()}
        (root/"before-runtime.json").write_text(json.dumps(identity,indent=2)+"\n")
        child = subprocess.run(command,capture_output=True,text=True,timeout=10)
        (root/"stdout.log").write_text(child.stdout)
        (root/"stderr.log").write_text(child.stderr)
        if child.returncode:
            raise RuntimeError("encoder CLI/setup failure NOT_RED: "+child.stderr)
        result = json.loads(child.stdout)
        actual = [json.loads(line) for line in output.read_bytes().splitlines()]
        expected = [{"id":f"s30doc-{index}","kind":"doc","body":body} for index,body in enumerate(bodies)]
        if result != {"records":len(bodies)} or source.read_bytes()!=payload:
            raise RuntimeError("existing count/input preservation control failed")
        print(json.dumps({"root":str(root),"cli_exit":child.returncode,"records":result["records"],"framed_rows":len(actual)}),flush=True)
        self.assertEqual(actual,expected,"missing selected-body to ordered G record projection")
        self.assertEqual([row["body"].encode("utf-8") for row in actual],[body.encode("utf-8") for body in bodies])
        self.assertEqual(len(output.read_bytes().splitlines()),len(bodies))

    def test_literal_ids_kinds_unicode_and_line_framing(self):
        self.require_projection(["alpha","βeta\nsecond line","quoted \"évidence\" \\ tail"])

    @settings(max_examples=8,deadline=None,derandomize=True,phases=[Phase.generate])
    @given(bodies=st.lists(st.text(st.characters(blacklist_categories=("Cs",)),min_size=1,max_size=40),min_size=1,max_size=8))
    def test_selected_utf8_body_order_count_and_roundtrip(self, bodies):
        self.require_projection(bodies)


if __name__ == "__main__":
    unittest.main()
