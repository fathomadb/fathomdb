"""Exercise the actual hosted diagnostic command before cancelling its later child."""

import os
from pathlib import Path
import signal
import subprocess
import tempfile
import textwrap
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]


class HostedCancellationProbe(unittest.TestCase):
    """Keep the hosted readiness signal and complete redacted failure observable."""

    def test_readiness_and_first_full_failure_survive_cancellation(self) -> None:
        """Run the workflow's real shell body, then stop its process group."""
        workflow = (ROOT / ".github/workflows/ci-diagnostic-cancellation.yml").read_text()
        command = textwrap.dedent(workflow.rsplit("        run: |\n", 1)[1])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / "job.log"
            summary = root / "summary"
            environment = os.environ | {
                "RUNNER_TEMP": directory,
                "AGENT_CI_LIVE": "1",
                "GITHUB_ACTIONS": "true",
                "GITHUB_STEP_SUMMARY": str(summary),
            }
            with log.open("wb") as output:
                child = subprocess.Popen(
                    ["bash", "-c", command], cwd=ROOT, env=environment,
                    stdout=output, stderr=subprocess.STDOUT, start_new_session=True,
                )
                try:
                    deadline = time.monotonic() + 5
                    while child.poll() is None and time.monotonic() < deadline:
                        text = log.read_text()
                        if "CANCELLATION_PROBE_READY" in text:
                            break
                        time.sleep(0.02)
                    text = log.read_text()
                    self.assertIn("CANCELLATION_PROBE_READY", text)
                    for ordinal in range(1, 251):
                        self.assertIn(f"compiler diagnostic {ordinal:03d}\n", text)
                    self.assertIn("error: final complete compiler diagnostic 251", text)
                    self.assertIn("SUITE RESULT cancel-pass PASS rc=0", text)
                    self.assertIn("SUITE RESULT cancel-fail FAIL rc=101", text)
                    self.assertIn("SUITE PLAN cancel-unrun", text)
                    self.assertIn("SUITE START cancel-block", text)
                    self.assertNotIn("SUITE RESULT cancel-block", text)
                    self.assertNotIn("SUITE START cancel-unrun", text)
                    self.assertNotIn("CANCELLATION_PROBE_UNRUN_EXECUTED", text)
                    self.assertNotIn("synthetic-cancel-", text)
                    self.assertNotIn("live diagnostic truncated", text)
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid, signal.SIGTERM)
                    child.wait(timeout=5)
            self.assertNotEqual(child.returncode, 0)
            self.assertFalse(summary.exists())
            self.assertEqual(log.read_text(), text)


if __name__ == "__main__":
    unittest.main()
