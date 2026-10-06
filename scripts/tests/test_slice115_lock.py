"""The measured Cargo dependency lock must be frozen with the protocol."""

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "slice115_runner.py"
SPEC = importlib.util.spec_from_file_location("slice115_runner", SCRIPT)
assert SPEC and SPEC.loader
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class LockBindingTests(unittest.TestCase):
    def test_protocol_binds_exact_cargo_lock(self):
        protocol = json.loads(runner.PROTOCOL.read_text())
        expected = protocol["cargo_lock_sha256"]
        source_sha = protocol["engine_source_sha"]
        measured_lock = subprocess.check_output(
            ["git", "show", f"{source_sha}:Cargo.lock"], cwd=runner.ROOT
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.lock").write_bytes(measured_lock)
            self.assertEqual(expected, runner.sha256(root / "Cargo.lock"))
            with patch.object(runner, "ROOT", root):
                runner.require_lock(expected)
                with self.assertRaisesRegex(ValueError, "Cargo.lock"):
                    runner.require_lock("0" * 64)


if __name__ == "__main__":
    unittest.main()
