"""Actual G CLI opt-in refusal using harmless command-only inputs."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from test_g_pipeline import controlled_manifest


class GEntryTests(unittest.TestCase):
    def test_run_cli_requires_long_opt_in_before_any_command_starts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest, calls = controlled_manifest(root)
            (root / "manifest.json").write_text(json.dumps(manifest))
            environment = dict(os.environ)
            environment.pop("AGENT_LONG", None)
            child = subprocess.run([sys.executable, str(Path(__file__).with_name("g_runner.py")), "run", str(root)],
                                   env=environment, capture_output=True, text=True, check=False, timeout=30)
            self.assertFalse(calls.exists(), "G CLI dispatched commands without the required long opt-in")
            self.assertEqual(child.returncode, 2)
            self.assertIn("AGENT_LONG=1", child.stderr)


if __name__ == "__main__":
    unittest.main()
