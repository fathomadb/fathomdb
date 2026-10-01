"""An invalid D27 repetition remains auditable and names its invalidator."""

import contextlib
import copy
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("d27_attempt_audit", ROOT / "scripts/d27-runtime-runner.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)

raw_spec = importlib.util.spec_from_file_location("d27_raw_fixture", ROOT / "scripts/tests/test_d27_raw_linkage.py")
raw_fixture = importlib.util.module_from_spec(raw_spec)
raw_spec.loader.exec_module(raw_fixture)


class AttemptAuditTests(unittest.TestCase):
    def setUp(self):
        self.raw = raw_fixture.synthetic_raw("projection_heavy", 1)

    def test_each_environment_invalidator_is_named(self):
        cases = (
            ("cpu_governor", "powersave", "governor"),
            ("competing_processes", [{"pid": 123, "name": "cargo"}], "cargo.*123"),
            ("swap_pages_in", 1, "swap_pages_in"),
            ("swap_pages_out", 1, "swap_pages_out"),
            ("pid_namespace", "pid:[4026534361]", "pid namespace"),
            ("pid_one_namespace", "pid:[4026534361]", "process view"),
            ("pid_one_comm", "codex", "process view"),
            ("ps_pid_one_comm", None, "process view"),
            ("procfs_hidepid", "2", "process view"),
        )
        for key, value, expected in cases:
            raw = copy.deepcopy(self.raw)
            raw["environment_samples"][0][key] = value
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, expected):
                runner.check_raw_observations(raw, "entry")

    def test_namespaced_process_view_cannot_start_a_repetition(self):
        protocol = json.loads(runner.PROTOCOL.read_text())
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            binary = output / "binary"
            binary.write_bytes(b"test binary")
            snapshot = copy.deepcopy(self.raw["environment_start"])
            snapshot["pid_namespace"] = "pid:[4026534361]"
            argv = ["d27-runtime-runner.py", "--source", directory, "--phase", "entry", "--output-dir", directory]
            with mock.patch.object(sys, "argv", argv), mock.patch.object(runner, "git_sha", return_value=protocol["entry_engine_candidate_sha"]), mock.patch.object(runner, "build_binary", return_value=binary), mock.patch.object(runner, "environment", return_value=snapshot), mock.patch.object(runner, "run_repetition") as run:
                with self.assertRaisesRegex(ValueError, "pid namespace"):
                    runner.main()
                run.assert_not_called()

    def test_host_namespace_with_restricted_procfs_cannot_start(self):
        protocol = json.loads(runner.PROTOCOL.read_text())
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            binary = output / "binary"
            binary.write_bytes(b"test binary")
            snapshot = copy.deepcopy(self.raw["environment_start"])
            snapshot["ps_pid_one_comm"] = None
            argv = ["d27-runtime-runner.py", "--source", directory, "--phase", "entry", "--output-dir", directory]
            with mock.patch.object(sys, "argv", argv), mock.patch.object(runner, "git_sha", return_value=protocol["entry_engine_candidate_sha"]), mock.patch.object(runner, "build_binary", return_value=binary), mock.patch.object(runner, "environment", return_value=snapshot), mock.patch.object(runner, "run_repetition") as run:
                with self.assertRaisesRegex(ValueError, "process view"):
                    runner.main()
                run.assert_not_called()

    def test_first_failed_repetition_is_persisted_before_validation_aborts(self):
        protocol = json.loads(runner.PROTOCOL.read_text())
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            binary = output / "binary"
            binary.write_bytes(b"test binary")
            environment = copy.deepcopy(self.raw["environment_start"])

            def fake_run(_binary, env):
                Path(env["D27_RAW_OUTPUT"]).write_text(json.dumps(self.raw))
                invalid = copy.deepcopy(environment)
                invalid["competing_processes"] = [{"pid": 123, "name": "cargo"}]
                return [invalid]

            argv = ["d27-runtime-runner.py", "--source", directory, "--phase", "entry", "--output-dir", directory]
            with mock.patch.object(sys, "argv", argv), mock.patch.object(runner, "git_sha", return_value=protocol["entry_engine_candidate_sha"]), mock.patch.object(runner, "build_binary", return_value=binary), mock.patch.object(runner, "environment", return_value=environment), mock.patch.object(runner, "run_repetition", side_effect=fake_run):
                with self.assertRaisesRegex(ValueError, "cargo.*123"):
                    runner.main()
            audit = json.loads((output / "entry_projection_heavy_1.attempt.json").read_text())
            self.assertEqual(audit["environment_samples"][0]["competing_processes"], [{"pid": 123, "name": "cargo"}])
            self.assertIn("competing", (output / "entry_projection_heavy_1.invalidation.json").read_text())
            self.assertEqual(len((output / "raw-output.jsonl").read_text().splitlines()), 1)

    def test_cargo_stderr_is_emitted_without_rewording(self):
        diagnostic = "error: failed to download package\ncaused by: network unavailable\n"
        with tempfile.TemporaryDirectory() as directory:
            captured = io.StringIO()
            failure = subprocess.CalledProcessError(101, ["cargo", "test"], stderr=diagnostic)
            with mock.patch.object(runner, "command", side_effect=failure), contextlib.redirect_stderr(captured):
                with self.assertRaises(subprocess.CalledProcessError):
                    runner.build_binary(Path(directory) / "Cargo.toml", Path(directory))
            self.assertEqual(captured.getvalue(), diagnostic)


if __name__ == "__main__":
    unittest.main()
