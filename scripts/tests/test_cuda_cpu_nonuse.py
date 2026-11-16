#!/usr/bin/env python3
"""Check explicit-CPU measurement completeness without querying GPU hardware."""

from __future__ import annotations

import importlib.util
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "cpu_nonuse", ROOT / "scripts/release/cuda-cpu-nonuse.py"
)
assert SPEC is not None and SPEC.loader is not None
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)
UUID = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b"


class MeasurementTests(unittest.TestCase):
    def fixture(self, root: Path, policy: str = "cpu") -> None:
        positive = policy == "cuda:0"
        ready = {
            "pid": 4242,
            "policy": policy,
            "cuda_compiled": True,
            "effective_device": "cuda" if positive else "cpu",
            "selected_uuid": UUID if positive else None,
            "vector_dim": 384,
            "result_count": 2,
            "reranked": True,
        }
        (root / "ready.json").write_text(json.dumps(ready))
        (root / "done.json").write_text(json.dumps({"pid": 4242, "closed": True}))
        (root / "observations.json").write_text(
            json.dumps(
                {
                    "binding": "python",
                    "policy": policy,
                    "returncode": 0,
                    "selected_uuid": UUID,
                    "samples": [
                        {"compute_pids": [], "pid_alive": None},
                        {"compute_pids": [4242] if positive else [], "pid_alive": True},
                        {"compute_pids": [], "pid_alive": False},
                    ],
                }
            )
        )
        access = (
            'openat(AT_FDCWD, "/usr/lib/libcuda.so.1", O_RDONLY) = 3\n'
            if positive
            else ""
        )
        (root / "trace.4242").write_text(
            'execve("/usr/bin/python", ["python"], []) = 0\n'
            + access
            + "exit_group(0) = ?\n+++ exited with 0 +++\n"
        )
        (root / "loader.4242").write_text(
            "4242: initialize program: /usr/bin/python\n"
            "4242: transferring control: /usr/bin/python\n"
            + ("4242: file=libcuda.so.1 [0]; dynamically loaded\n" if positive else "")
            + "4242: calling fini: /lib/libc.so.6 [0]\n"
        )
        (root / "stderr.txt").write_text("")

    def test_complete_cpu_and_separate_positive_qualify(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            cpu, gpu = root / "cpu", root / "gpu"
            cpu.mkdir()
            gpu.mkdir()
            self.fixture(cpu)
            self.fixture(gpu, "cuda:0")
            TOOL.validate_pair(cpu, gpu, UUID)

    def test_cuda_access_in_any_child_refuses_cpu(self) -> None:
        for access in ("/dev/nvidiactl", "/usr/lib/libcuda.so.1"):
            with self.subTest(access=access), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                self.fixture(root)
                (root / "trace.4243").write_text(
                    f'openat(AT_FDCWD, "{access}", O_RDONLY) = -1 ENOENT\n'
                    "+++ exited with 0 +++\n"
                )
                with self.assertRaisesRegex(ValueError, "CUDA access"):
                    TOOL.validate_capture(root, "cpu", UUID)

    def test_missing_or_incomplete_capture_refuses(self) -> None:
        for missing in ("trace.4242", "loader.4242", "done.json"):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                self.fixture(root)
                (root / missing).unlink()
                with self.assertRaises(ValueError):
                    TOOL.validate_capture(root, "cpu", UUID)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.fixture(root)
            (root / "trace.4242").write_text('execve("/usr/bin/python", [], []) = 0\n')
            with self.assertRaisesRegex(ValueError, "incomplete"):
                TOOL.validate_capture(root, "cpu", UUID)

    def test_live_cpu_gpu_pid_or_failed_capture_refuses(self) -> None:
        for field, value in (
            ("returncode", 1),
            (
                "samples",
                [
                    {"compute_pids": [4242], "pid_alive": True},
                    {"compute_pids": [], "pid_alive": False},
                ],
            ),
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                self.fixture(root)
                path = root / "observations.json"
                record = json.loads(path.read_text())
                record[field] = value
                path.write_text(json.dumps(record))
                with self.assertRaises(ValueError):
                    TOOL.validate_capture(root, "cpu", UUID)

    def test_positive_requires_gpu_pid_and_capture_power(self) -> None:
        for change in ("pid", "trace", "loader"):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                self.fixture(root, "cuda:0")
                if change == "pid":
                    path = root / "observations.json"
                    record = json.loads(path.read_text())
                    for sample in record["samples"]:
                        sample["compute_pids"] = []
                    path.write_text(json.dumps(record))
                else:
                    path = root / ("trace.4242" if change == "trace" else "loader.4242")
                    path.write_text(
                        path.read_text().replace("libcuda.so.1", "libordinary.so.1")
                    )
                with self.assertRaises(ValueError):
                    TOOL.validate_capture(root, "cuda:0", UUID)

    def test_real_harmless_cpu_trace_and_loader_are_complete(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            process = TOOL.start_trace(
                [sys.executable, "-c", "print('CPU trace control')"],
                root,
                os.environ.copy(),
            )
            self.assertEqual(process.wait(timeout=15), 0)
            TOOL.validate_trace(root, cuda_expected=False)
            self.assertIn("CPU trace control", (root / "stdout.txt").read_text())


if __name__ == "__main__":
    unittest.main()
