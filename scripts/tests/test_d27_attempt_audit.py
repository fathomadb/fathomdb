"""An invalid D27 repetition remains auditable and names its invalidator."""

import contextlib
import copy
import errno
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
            ("pid_namespace", "pid:[4026534361]", "pid namespace"),
            ("pid_one_namespace", "pid:[4026534361]", "process view"),
            ("pid_one_comm", "codex", "process view"),
            ("ps_pid_one_comm", None, "process view"),
            ("procfs_hidepid", "2", "process view"),
            ("ps_self_pid", None, "process view"),
            ("proc_self_pid", None, "process view"),
        )
        for key, value, expected in cases:
            raw = copy.deepcopy(self.raw)
            raw["environment_samples"][0][key] = value
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, expected):
                runner.check_raw_observations(raw, "entry")

    def test_combined_swap_cap_includes_both_counters_and_every_sample(self):
        for phase in ("entry", "candidate"):
            for incoming, outgoing in ((128, 0), (64, 64), (0, 128)):
                raw = copy.deepcopy(self.raw)
                raw["environment_samples"][0]["swap_pages_in"] = incoming // 2
                raw["environment_samples"][0]["swap_pages_out"] = outgoing // 2
                raw["environment_end"]["swap_pages_in"] = incoming
                raw["environment_end"]["swap_pages_out"] = outgoing
                with self.subTest(phase=phase, incoming=incoming, outgoing=outgoing):
                    self.assertEqual(runner.environment_invalidators(raw), [])
            raw = copy.deepcopy(self.raw)
            raw["environment_samples"][0]["swap_pages_in"] = 65
            raw["environment_samples"][0]["swap_pages_out"] = 64
            raw["environment_end"]["swap_pages_in"] = 65
            raw["environment_end"]["swap_pages_out"] = 64
            with self.subTest(phase=phase, case="over cap"):
                self.assertRegex("; ".join(runner.environment_invalidators(raw)), "swap.*129")

    def test_v2_reports_129_host_swap_pages_without_invalidating(self):
        protocol = json.loads((ROOT / "dev/plans/0.8.27/features/slice-90/d27-runtime-qualification-protocol-v2.json").read_text())
        raw = copy.deepcopy(self.raw)
        raw["environment_samples"][0]["swap_pages_in"] = 65
        raw["environment_samples"][0]["swap_pages_out"] = 64
        raw["environment_end"]["swap_pages_in"] = 65
        raw["environment_end"]["swap_pages_out"] = 64
        self.assertEqual(runner.environment_invalidators(raw, protocol), [])
        self.assertEqual(runner.summarize_raw(raw, "entry", protocol)["swap_pages_in_delta"], 65)
        self.assertEqual(runner.summarize_raw(raw, "entry", protocol)["swap_pages_out_delta"], 64)
        raw["environment_samples"][0]["swap_pages_in"] = None
        self.assertRegex("; ".join(runner.environment_invalidators(raw, protocol)), "swap_pages_in")
        raw["environment_samples"] = [copy.deepcopy(raw["environment_start"]) for _ in range(2)]
        raw["environment_samples"][0]["swap_pages_in"] = 66
        raw["environment_samples"][1]["swap_pages_in"] = 65
        self.assertRegex("; ".join(runner.environment_invalidators(raw, protocol)), "decreased")

    def test_missing_negative_or_reset_swap_counter_invalidates_any_sample(self):
        for boundary in ("environment_start", "environment_samples", "environment_end"):
            for key, value in (("swap_pages_in", None), ("swap_pages_out", -1)):
                raw = copy.deepcopy(self.raw)
                observation = raw[boundary][0] if boundary == "environment_samples" else raw[boundary]
                observation[key] = value
                with self.subTest(boundary=boundary, key=key):
                    self.assertRegex("; ".join(runner.environment_invalidators(raw)), key)
        raw = copy.deepcopy(self.raw)
        raw["environment_samples"] = [copy.deepcopy(raw["environment_start"]) for _ in range(2)]
        raw["environment_samples"][0]["swap_pages_in"] = 5
        raw["environment_samples"][1]["swap_pages_in"] = 4
        raw["environment_end"]["swap_pages_in"] = 5
        self.assertRegex("; ".join(runner.environment_invalidators(raw)), "swap_pages_in.*decreased")

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

    def test_effective_procfs_overmount_restriction_is_observed(self):
        mountinfo = (
            "29 33 0:26 / /proc rw - proc proc rw\n"
            "7061 6789 0:361 / /proc rw - proc proc rw,hidepid=2\n"
        )
        with mock.patch.object(runner.Path, "read_text", return_value=mountinfo):
            self.assertEqual(runner.procfs_hidepid(), "2")

    def test_host_process_view_accepts_unavailable_pid_one_namespace_link(self):
        snapshot = copy.deepcopy(self.raw["environment_start"])
        snapshot["pid_one_namespace"] = ""
        self.assertEqual(runner.process_view_invalidators(snapshot, "host"), [])

    def observed_host_environment(self, link_error=None, comm_error=None):
        pid = 4242
        reads = {
            "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor": "performance\n",
            "/proc/vmstat": "pswpin 0\npswpout 0\n",
            "/proc/1/comm": "systemd\n",
            "/proc/self/mountinfo": "29 33 0:26 / /proc rw - proc proc rw\n",
            "/proc/cpuinfo": "model name : test CPU\n",
            "/proc/meminfo": "MemAvailable: 1048576 kB\n",
        }
        read_paths = []

        def read_text(path, *args, **kwargs):
            name = str(path)
            read_paths.append(name)
            if name == "/proc/1/comm" and comm_error is not None:
                raise OSError(comm_error, "PID 1 name unavailable", name)
            return reads[name]

        def readlink(path):
            if path == "/proc/1/ns/pid" and link_error is not None:
                raise OSError(link_error, "PID 1 link unavailable", path)
            return {"/proc/1/ns/pid": runner.HOST_PID_NAMESPACE, "/proc/self/ns/pid": runner.HOST_PID_NAMESPACE, "/proc/self": str(pid)}[path]

        def command(args, **kwargs):
            output = "1 systemd\n4242 python3\n" if args[0] == "ps" else "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/nvme0n1 1 0 1 0% /tmp\n"
            return subprocess.CompletedProcess(args, 0, stdout=output)

        with mock.patch.object(runner.Path, "read_text", read_text), mock.patch.object(runner.os, "readlink", side_effect=readlink), mock.patch.object(runner.os, "getpid", return_value=pid), mock.patch.object(runner, "command", side_effect=command):
            observation = runner.environment()
        return observation, read_paths

    def test_permission_denied_pid_one_link_retains_independent_host_proof(self):
        for error in (errno.EACCES, errno.EPERM):
            with self.subTest(error=error):
                observation, read_paths = self.observed_host_environment(link_error=error)
                self.assertIn("/proc/1/comm", read_paths)
                self.assertEqual(observation["pid_one_namespace"], "")
                self.assertEqual(observation["pid_one_comm"], "systemd")
                self.assertEqual(observation["ps_pid_one_comm"], "systemd")
                self.assertEqual(observation["procfs_hidepid"], "0")
                self.assertEqual(observation["runner_pid"], observation["proc_self_pid"])
                self.assertEqual(observation["runner_pid"], observation["ps_self_pid"])
                self.assertEqual(runner.process_view_invalidators(observation, "host"), [])
                raw = copy.deepcopy(self.raw)
                for boundary in ("environment_start", "environment_end"):
                    raw[boundary] = copy.deepcopy(observation)
                raw["environment_samples"] = [copy.deepcopy(observation)]
                self.assertEqual(runner.environment_invalidators(raw), [])

    def test_other_pid_one_link_errors_and_unreadable_name_fail_closed(self):
        for error in (errno.ENOENT, errno.EIO):
            with self.subTest(link_error=error):
                observation, read_paths = self.observed_host_environment(link_error=error)
                self.assertIn("/proc/1/comm", read_paths)
                self.assertEqual(observation["pid_one_comm"], "systemd")
                self.assertRegex("; ".join(runner.process_view_invalidators(observation, "host")), "process view")
        observation, _ = self.observed_host_environment(link_error=errno.EACCES, comm_error=errno.EACCES)
        self.assertIsNone(observation["pid_one_comm"])
        self.assertRegex("; ".join(runner.process_view_invalidators(observation, "host")), "process view")

    def test_nested_procfs_without_runner_host_pid_cannot_start(self):
        protocol = json.loads(runner.PROTOCOL.read_text())
        with tempfile.TemporaryDirectory() as directory:
            snapshot = copy.deepcopy(self.raw["environment_start"])
            snapshot["pid_one_namespace"] = ""
            snapshot["ps_self_pid"] = None
            argv = ["d27-runtime-runner.py", "--source", directory, "--phase", "entry", "--output-dir", directory]
            with mock.patch.object(sys, "argv", argv), mock.patch.object(runner, "git_sha", return_value=protocol["entry_engine_candidate_sha"]), mock.patch.object(runner, "environment", return_value=snapshot), mock.patch.object(runner, "build_binary") as build, mock.patch.object(runner, "run_repetition") as run:
                with self.assertRaisesRegex(ValueError, "process view"):
                    runner.main()
                build.assert_not_called()
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
