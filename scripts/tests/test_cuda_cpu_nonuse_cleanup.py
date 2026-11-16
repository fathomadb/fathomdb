#!/usr/bin/env python3
"""Exercise tracer death with a real harmless consumer that survives its leader."""

import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import patch

import test_cuda_cpu_nonuse as oracle


class TracerCleanupTests(unittest.TestCase):
    def test_dead_tracer_cleans_live_consumer_and_preserves_failure(self) -> None:
        sys.path.insert(
            0, str(oracle.ROOT / "dev/experiments/0.8.29-slice30-measurements")
        )
        from g_boundary_tests import PinnedRescue, actor_facts

        rescue = PinnedRescue()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "capture"
            owned = {}
            real_start = oracle.TOOL.start_trace
            sentinel = subprocess.Popen(
                [sys.executable, "-c", "import time; time.sleep(30)"],
                start_new_session=True,
            )

            def start(*args):
                process = real_start(*args)
                owned["tracer"] = process
                return process

            def observe(_uuid):
                ready = root / "ready.json"
                if "tracer" in owned and "pidfd" not in owned and ready.exists():
                    pid = json.loads(ready.read_text())["pid"]
                    owned["consumer"] = pid
                    owned["pidfd"] = rescue.pin(actor_facts(pid))
                    os.kill(owned["tracer"].pid, signal.SIGKILL)
                    time.sleep(0.05)
                return []

            program = (
                "import json,os,pathlib,signal,time; "
                "signal.signal(signal.SIGTERM, signal.SIG_IGN); "
                "root=pathlib.Path(os.environ['FATHOMDB_CAPTURE_DIR']); "
                "(root/'ready.json').write_text(json.dumps({'pid':os.getpid()})); "
                "time.sleep(30)"
            )
            try:
                with (
                    patch.object(oracle.TOOL, "start_trace", side_effect=start),
                    patch.object(oracle.TOOL, "query_compute", side_effect=observe),
                ):
                    with self.assertRaisesRegex(ValueError, "live|surviv|tracer"):
                        oracle.TOOL.capture(
                            "python",
                            "cuda:0",
                            [sys.executable, "-c", program],
                            root,
                            os.environ.copy(),
                            oracle.UUID,
                            8,
                        )
                self.assertIn("consumer", owned)
                self.assertIsNone(
                    oracle.TOOL.process_start(owned["consumer"]),
                    "owned consumer survived tracer-death cleanup",
                )
                self.assertIsNone(
                    sentinel.poll(), "cleanup affected an unrelated process group"
                )
                record = json.loads((root / "observations.json").read_text())
                self.assertNotEqual(record["returncode"], 0)
                self.assertTrue(
                    list(root.glob("trace.*")), "failure discarded raw traces"
                )
            finally:
                rescue.close()
                if "tracer" in owned:
                    owned["tracer"].wait(timeout=5)
                if sentinel.poll() is None:
                    sentinel.terminate()
                sentinel.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
