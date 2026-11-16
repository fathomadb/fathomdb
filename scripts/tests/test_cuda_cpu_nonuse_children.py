#!/usr/bin/env python3
"""Require every child identified by a syscall trace to have a completed capture."""

import os
import sys
import tempfile
import unittest
from pathlib import Path

import test_cuda_cpu_nonuse as oracle


class DescendantCaptureTests(unittest.TestCase):
    def test_missing_spawned_child_trace_refuses(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle.MeasurementTests().fixture(root)
            path = root / "trace.4242"
            path.write_text(
                "clone(child_stack=NULL, flags=SIGCHLD) = 4243\n" + path.read_text()
            )
            with self.assertRaisesRegex(ValueError, "child.*capture"):
                oracle.TOOL.validate_trace(root, cuda_expected=False)

    def test_real_harmless_child_is_traced_through_exit(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            process = oracle.TOOL.start_trace(
                [
                    sys.executable,
                    "-c",
                    "import subprocess,sys; subprocess.run([sys.executable,'-c','print(42)'],check=True)",
                ],
                root,
                os.environ.copy(),
            )
            self.assertEqual(process.wait(timeout=15), 0)
            self.assertGreaterEqual(
                len(oracle.TOOL.validate_trace(root, cuda_expected=False)), 2
            )


if __name__ == "__main__":
    unittest.main()
