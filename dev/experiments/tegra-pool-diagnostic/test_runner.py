"""Owned-process checks only; the fake Node never imports a GPU package."""
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

RUNNER = Path(__file__).with_name("run.sh")


class RunnerTests(unittest.TestCase):
    def run_child(self, interruption):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            consumer = root / "consumer"
            consumer.mkdir()
            tools = root / "bin"
            tools.mkdir()
            fake_node = tools / "node"
            fake_node.write_text(
                f"#!{sys.executable}\n"
                "import os, signal, sys, time\n"
                "for sig in (signal.SIGINT, signal.SIGTERM):\n"
                "    signal.signal(sig, lambda sig, frame: sys.exit(128 + sig))\n"
                "print('partial output', flush=True)\n"
                "open(os.environ['READY'], 'w').close()\n"
                "if os.environ['MODE'] == 'normal': sys.exit(7)\n"
                "while True: time.sleep(0.05)\n"
            )
            fake_node.chmod(0o755)
            evidence = root / "evidence"
            ready = root / "ready"
            environment = dict(os.environ, PATH=f"{tools}:{os.environ['PATH']}",
                               READY=str(ready), MODE="wait" if interruption else "normal")
            runner = subprocess.Popen(["bash", str(RUNNER), str(consumer), str(evidence)],
                                      env=environment, start_new_session=True)
            child = None
            try:
                deadline = time.monotonic() + 5
                while not ready.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertTrue(ready.exists(), "fake consumer must actually start")
                child = int((evidence / "pid.txt").read_text())
                if interruption:
                    runner.send_signal(interruption)
                status = runner.wait(timeout=5)
                if interruption:
                    with self.assertRaises(ProcessLookupError,
                                           msg="runner must stop and reap its owned consumer"):
                        os.kill(child, 0)
                    self.assertEqual(status, 128 + interruption)
                else:
                    self.assertEqual(status, 7)
                self.assertEqual(int((evidence / "exit.txt").read_text()), status)
                self.assertEqual((evidence / "stdout.log").read_text(), "partial output\n")
            finally:
                if child:
                    try:
                        os.kill(child, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                if runner.poll() is None:
                    runner.kill()
                runner.wait()

    def test_original_nonzero_exit_and_partial_logs(self):
        self.run_child(None)

    def test_int_stops_and_waits_for_owned_consumer(self):
        self.run_child(signal.SIGINT)

    def test_term_stops_and_waits_for_owned_consumer(self):
        self.run_child(signal.SIGTERM)


if __name__ == "__main__":
    unittest.main()
