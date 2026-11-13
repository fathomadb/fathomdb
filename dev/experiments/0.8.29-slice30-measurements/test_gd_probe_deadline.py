"""Actual du launch controls for the optional shared campaign probe deadline."""

import os
from pathlib import Path
import shutil
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from hypothesis import Phase, given, settings, strategies as st

from gd_resources import measure_root


def expired_probe(size):
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        tree = root / "tree"
        tree.mkdir()
        (tree / "payload").write_bytes(b"x" * size)
        tools = root / "tools"
        tools.mkdir()
        marker = root / "calls"
        real_du = shutil.which("du")
        assert real_du
        wrapper = tools / "du"
        wrapper.write_text(f"#!{sys.executable}\nimport os,sys\nfrom pathlib import Path\np=Path({str(marker)!r})\nwith p.open('a') as out: out.write('called\\n')\nos.execv({real_du!r},[{real_du!r},*sys.argv[1:]])\n")
        wrapper.chmod(0o755)
        with patch.dict(os.environ, PATH=str(tools)+os.pathsep+os.environ["PATH"]):
            healthy = measure_root(tree)
            assert healthy["allocated"] >= 0 and healthy["apparent"] >= size
            assert marker.read_text().splitlines() == ["called", "called"]
            marker.write_text("")
            refused = False
            try:
                measure_root(tree, deadline=time.monotonic()-1)
            except RuntimeError:
                refused = True
            actual = (refused, len(marker.read_text().splitlines()))
            print(f"real du expired probe refused/calls={actual}", flush=True)
            return actual


class ProbePrelaunchTests(unittest.TestCase):
    def test_expired_shared_deadline_launches_no_real_du(self):
        self.assertEqual(expired_probe(17), (True, 0))

    @settings(max_examples=4, deadline=None, database=None, phases=[Phase.generate])
    @given(st.integers(min_value=0, max_value=4096))
    def test_expired_probe_refuses_before_launch_for_bounded_real_trees(self, size):
        self.assertEqual(expired_probe(size), (True, 0))


if __name__ == "__main__":
    unittest.main()
