"""Reviewed cleanup and incomplete-audit counterexamples; disposable children only."""
import contextlib
import io
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run as runner
import score
from test_current_controls import transcript


def args():
    return SimpleNamespace(binary="/unused", result_format="text", model="unused", max_turns=8,
                           episode_cap_usd="0.02", timeout=1, item="5")


def running(pid):
    try:
        return Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0] != "Z"
    except FileNotFoundError:
        return False


class ReviewFixes(unittest.TestCase):
    def test_descendant_ignoring_term_is_killed_after_parent_exits(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            worker = root / "worker"
            (worker / "cwd").mkdir(parents=True)
            identity = root / "disposable-child.json"
            script = '''import json, os, signal, sys, time
from pathlib import Path
child = os.fork()
if child == 0:
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    fd = os.open(os.devnull, os.O_RDWR)
    for stream in (0, 1, 2): os.dup2(fd, stream)
    os.close(fd)
    Path(sys.argv[1]).write_text(json.dumps([os.getpid(), os.getpgrp()]))
    time.sleep(30)
else:
    time.sleep(30)
'''
            try:
                with patch.object(runner, "claude_command", return_value=[sys.executable, "-c", script, str(identity)]):
                    row = runner.run_question(SimpleNamespace(qid="conv1_q0", question="unused"), worker=worker,
                                              run_dir=root, args=args(), default_store="unused")
                self.assertEqual(row["exit"], "timeout")
                self.assertTrue(identity.exists(), "disposable descendant must be ready before timeout")
                pid, _ = json.loads(identity.read_text())
                deadline = time.monotonic() + 2
                while running(pid) and time.monotonic() < deadline:
                    time.sleep(0.01)
                self.assertFalse(running(pid), "TERM-resistant descendant survived its parent's cleanup")
            finally:
                if identity.exists():
                    _, group = json.loads(identity.read_text())
                    with contextlib.suppress(ProcessLookupError):
                        os.killpg(group, signal.SIGKILL)

    def test_timeout_tolerates_an_already_gone_owned_group(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            worker = root / "worker"
            (worker / "cwd").mkdir(parents=True)
            proc = SimpleNamespace(pid=123, communicate=Mock(side_effect=[subprocess.TimeoutExpired("fake", 1),
                                                                         (None, ""), (None, "")]))
            with patch.object(runner.subprocess, "Popen", return_value=proc), patch.object(runner.os, "killpg", side_effect=ProcessLookupError):
                row = runner.run_question(SimpleNamespace(qid="conv1_q0", question="unused"), worker=worker,
                                          run_dir=root, args=args(), default_store="unused")
            self.assertEqual(row["exit"], "timeout")
            self.assertFalse(row["valid"])

    def audit(self, root, complete):
        paths = []
        for arm in ("text", "structured"):
            run = root / arm
            (run / "q").mkdir(parents=True)
            (run / "run.json").write_text(json.dumps({"item":"5", "conv":1, "arm":arm, "model":"unused",
                                                       "questions":["conv1_q0", "conv1_q1"]}))
            rows = [{"qid":qid, "status":"complete", "valid":True, "answer":"a", "answer_json":{"answer":"a"},
                     "exit":0, "wall_s":1} for qid in ("q0", "q1")]
            (run / "checkpoint.jsonl").write_text("\n".join(map(json.dumps, rows)))
            for row in rows:
                (run / "q" / f"{row['qid']}.stream.jsonl").write_text("\n".join(map(json.dumps, transcript(row["qid"]))))
            graded = ("q0", "q1") if complete or arm == "structured" else ("q0",)
            (run / "judged.jsonl").write_text("\n".join(json.dumps({"key":score.answer_key(qid, "a"), "status":"complete",
                                                                    "judge_model":"gpt-4o-mini", "lenient":1.0, "strict":1.0}) for qid in graded))
            paths.append(str(run))
        output = root / "audit"
        with patch.object(sys, "argv", ["score.py", "--out", str(output), "--no-judge", *paths]), patch.object(score, "Judges", side_effect=AssertionError("audit must never create a client")), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(score.main(), 0)
        return json.loads((output / "summary.json").read_text())

    def test_mixed_cached_missing_audit_suppresses_all_accuracy_and_pairs(self):
        with tempfile.TemporaryDirectory() as tmp:
            summary = self.audit(Path(tmp), complete=False)
            for run in summary["runs"].values():
                self.assertEqual(run["mean_searches"], 1)
                self.assertEqual(run["mean_hits_per_page"], 1)
                self.assertIsNone(run["lenient"]["mean"])
                self.assertIsNone(run["strict"]["mean"])
                self.assertEqual(run["lenient"]["n"], 0)
                self.assertIsNone(run["gold_ref_used_rate"])
            self.assertEqual(summary["paired"], [])

    def test_fully_cached_audit_keeps_full_accuracy_and_pairs(self):
        with tempfile.TemporaryDirectory() as tmp:
            summary = self.audit(Path(tmp), complete=True)
            self.assertEqual(summary["runs"]["text"]["lenient"]["n"], 2)
            self.assertEqual(summary["runs"]["structured"]["strict"]["mean"], 1)
            self.assertEqual(summary["paired"][0]["n"], 2)


if __name__ == "__main__":
    unittest.main()
