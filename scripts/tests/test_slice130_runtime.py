"""Native-free observer, lifecycle-oracle and owned-process controls."""

import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import threading
import unittest

from hypothesis import given, strategies as st

SPEC = importlib.util.spec_from_file_location("runtime_soak", Path(__file__).resolve().parents[1] / "slice130_soak.py")
soak = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(soak)


def observation(**updates):
    return dict({"schema": "fathomdb.cuda-pool-observation/v1", "status": "private",
                 "pid": os.getpid(), "uuid": "actual-device", "device_ordinal": 0,
                 "context_id": "18446744073709551615", "pool_id": "actual-pool",
                 "live_models_before": "2", "live_models_after": "2",
                 "used_current_bytes": "1048576", "reserved_current_bytes": "67108864"}, **updates)


class ObservationControls(unittest.TestCase):
    @given(st.integers(min_value=0, max_value=2**64 - 1))
    def test_exact_u64_wire_round_trip(self, value):
        raw = json.loads(json.dumps(observation(context_id=str(value), used_current_bytes=str(value))))
        result = soak.adapt_observation(raw, "on", pid=os.getpid())
        self.assertEqual(result["identity"]["context_id"], str(value))
        self.assertEqual(result["private_used_bytes"], value)
        self.assertEqual(result["raw_observation"], raw)

    def test_first_close_rejects_smaller_but_positive_reservation(self):
        before = soak.adapt_observation(observation(), "on")
        after = soak.adapt_observation(observation(live_models_before="1", live_models_after="1"), "on")
        soak.require_first_close(before, after)
        after["private_reserved_bytes"] -= 1
        with self.assertRaisesRegex(ValueError, "first close"):
            soak.require_first_close(before, after)

    def test_identity_and_partial_numeric_changes_are_refused(self):
        expected = soak.adapt_observation(observation(), "on")["identity"]
        for updates in ({"context_id": "1"}, {"pool_id": "other"}, {"pid": os.getpid() + 1},
                        {"used_current_bytes": None}, {"used_current_bytes": 9007199254740993},
                        {"used_current_bytes": str(2**64)}, {"status": "lost"},
                        {"live_models_after": "1"}):
            with self.subTest(updates=updates), self.assertRaises(ValueError):
                soak.adapt_observation(observation(**updates), "on", pid=os.getpid(), expected=expected)

    def test_last_zero_wait_never_performs_extra_work_and_stops_at_five_seconds(self):
        now = [0.0]
        raw = observation(live_models_before="0", live_models_after="0")
        expected = soak.adapt_observation(raw, "on")["identity"]
        events = []
        with self.assertRaises(TimeoutError):
            soak.wait_for_zero(lambda: raw, events.append, expected=expected,
                               clock=lambda: now[0], sleep=lambda seconds: now.__setitem__(0, now[0] + seconds))
        self.assertEqual(now[0], 5)
        self.assertEqual(events[-1]["elapsed_s"], 5)
        self.assertTrue(all(event["raw_observation"] == raw for event in events))


PEER = r'''
import json, os, sys, time
for line in sys.stdin:
    operation = json.loads(line)["operation"]
    if operation == "hang": time.sleep(20)
    if operation == "early": sys.exit(7)
    print(json.dumps({"ok": True, "operation": operation, "result": {"pid": os.getpid()}}), flush=True)
    if operation == "close": sys.exit(int(sys.argv[1]))
'''


class ProcessControls(unittest.TestCase):
    def worker(self, code=0, timeout=0.1):
        err = tempfile.TemporaryFile()
        worker = soak.Worker([sys.executable, "-u", "-c", PEER, str(code)], "/tmp", os.environ,
                             err, threading.Event(), timeout)
        self.addCleanup(err.close)
        self.addCleanup(worker.reap)
        return worker

    def test_nonzero_exit_after_successful_close_reply_is_not_healthy(self):
        worker = self.worker(9)
        with self.assertRaisesRegex(RuntimeError, "exit after close: 9"):
            worker.close()
        self.assertEqual(worker.reap(), 9)

    def test_timeout_and_early_exit_reap_owned_processes(self):
        for operation, error in (("hang", TimeoutError), ("early", RuntimeError)):
            worker = self.worker()
            with self.subTest(operation=operation), self.assertRaises(error):
                worker.request(operation)
            self.assertIsNotNone(worker.reap())
            self.assertIsNotNone(worker.process.poll())

    def test_healthy_close_reaps_actual_zero_exit(self):
        worker = self.worker()
        self.assertEqual(worker.request("ready")["pid"], worker.process.pid)
        self.assertEqual(worker.close(), 0)

    def test_resource_stop_remains_unmeasured_with_real_cleanup_signal(self):
        result = {"sdk": "node", "pool_mode": "on", "exit_code": -15,
                  "aborted": "MemAvailable floor", "abort_kind": "resource", "samples": []}
        self.assertEqual(soak.evaluate_arm(result)["status"], "UNMEASURED")


CONCURRENT_PEER = r'''
import json, os, pathlib, sys, time
root, sdk = pathlib.Path(sys.argv[1]), sys.argv[2]
for line in sys.stdin:
    operation = json.loads(line)["operation"]
    result = {}
    if operation == "ready":
        (root / sdk).touch()
        deadline = time.monotonic() + 2
        while not all((root / name).exists() for name in ("python", "node")):
            if time.monotonic() > deadline: raise RuntimeError("SDK workers were serialized")
            time.sleep(.01)
        result = {"pid": os.getpid()}
    if operation == "iteration": result = {"mismatches": 0, "latency_ms": {}, "outputs": {}}
    if operation == "snapshot":
        result = {"raw_observation": {"schema": "fathomdb.cuda-pool-observation/v1", "status": "private",
            "pid": os.getpid(), "uuid": "device", "device_ordinal": 0, "context_id": "1", "pool_id": "pool",
            "live_models_before": "1", "live_models_after": "1", "used_current_bytes": "1", "reserved_current_bytes": "1"}}
    print(json.dumps({"operation": operation, "ok": True, "result": result}), flush=True)
    if operation == "close": sys.exit(0)
'''


class ConcurrentControls(unittest.TestCase):
    def test_both_workers_start_before_collection_and_stream_real_exits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            commands = {sdk: {"command": [sys.executable, "-u", "-c", CONCURRENT_PEER, directory, sdk],
                              "cwd": directory} for sdk in ("python", "node")}
            limits = {"mem_floor_bytes": 1, "free_floor_bytes": 1, "storage_limit_bytes": 1048576,
                      "arm_timeout_s": 4000, "request_timeout_s": 3}

            def short_control(sdk, mode, iteration, snapshot, emit):
                iteration()
                point = snapshot()
                emit({"event": "control_snapshot", "point": point})
                return {"sdk": sdk, "pool_mode": mode, "aborted": None, "samples": [], "baseline": point}

            result = soak.run_concurrent_arm(commands, "on", root / "out", limits, collect=short_control)
            self.assertEqual(set(result), {"python", "node"})
            for sdk in result:
                self.assertEqual(result[sdk]["exit_code"], 0)
                events = [json.loads(line) for line in (root / "out" / f"{sdk}.jsonl").read_text().splitlines()]
                self.assertEqual(events[-1]["event"], "exit")
                self.assertEqual(events[-1]["exit_code"], 0)
                # This short process control is deliberately not a qualifying arm.
                self.assertEqual(soak.evaluate_arm(result[sdk])["status"], "UNMEASURED")


class DescendantControls(unittest.TestCase):
    def test_reap_stops_descendant_after_group_leader_exits(self):
        code = r'''
import json, os, sys, time
json.loads(sys.stdin.readline())
pid = os.fork()
if pid == 0:
    time.sleep(20)
    os._exit(0)
print(json.dumps({"operation": "ready", "ok": True, "result": {"child": pid}}), flush=True)
sys.exit(0)
'''
        with tempfile.TemporaryFile() as stderr:
            worker = soak.Worker([sys.executable, "-u", "-c", code], "/tmp", os.environ,
                                 stderr, threading.Event(), 2)
            descendant = worker.request("ready")["child"]
            self.addCleanup(lambda: os.kill(descendant, 9) if Path(f"/proc/{descendant}").exists() else None)
            worker.process.wait(timeout=2)
            worker.reap()
            until = __import__("time").monotonic() + 1
            while Path(f"/proc/{descendant}/stat").exists() and __import__("time").monotonic() < until:
                if Path(f"/proc/{descendant}/stat").read_text().split()[2] == "Z":
                    break
                __import__("time").sleep(.01)
            if Path(f"/proc/{descendant}/stat").exists():
                self.assertEqual(Path(f"/proc/{descendant}/stat").read_text().split()[2], "Z",
                                 "reaping must stop descendants even after their leader exits")


ENVIRONMENT_PEER = r'''
import json, os, pathlib, sys
keys = ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE", "FATHOMDB_POOL_MODE",
        "FATHOMDB_POOL_RELEASE_THRESHOLD", "SLICE130_ENV_CONTROL")
for line in sys.stdin:
    operation = json.loads(line)["operation"]
    if operation == "ready":
        pathlib.Path(sys.argv[1]).write_text(json.dumps({key: os.environ.get(key) for key in keys}))
    if operation not in ("ready", "close"):
        print(json.dumps({"operation": operation, "ok": False, "error": "environment-control-stop"}), flush=True)
        sys.exit(1)
    print(json.dumps({"operation": operation, "ok": True, "result": {"pid": os.getpid()}}), flush=True)
    if operation == "close": sys.exit(0)
'''


class EnvironmentControls(unittest.TestCase):
    def config(self, root, sdk):
        return {"command": [sys.executable, "-u", "-c", ENVIRONMENT_PEER, str(root / sdk)],
                "cwd": str(root), "environment": {
                    "HF_HUB_OFFLINE": "0", "TRANSFORMERS_OFFLINE": "0",
                    "FATHOMDB_POOL_MODE": "off", "FATHOMDB_POOL_RELEASE_THRESHOLD": "9",
                    "SLICE130_ENV_CONTROL": "preserve-worker-setting"}}

    def assert_environment(self, path, threshold):
        self.assertEqual(json.loads(path.read_text()), {
            "HF_HUB_OFFLINE": "1", "TRANSFORMERS_OFFLINE": "1", "FATHOMDB_POOL_MODE": "on",
            "FATHOMDB_POOL_RELEASE_THRESHOLD": threshold,
            "SLICE130_ENV_CONTROL": "preserve-worker-setting"})

    def test_lifecycle_overlapping_worker_environment_reaches_child(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = self.config(root, "python")
            # The peer stops before observations: this checks process environment,
            # never a fabricated lifecycle or native qualification result.
            with self.assertRaisesRegex(RuntimeError, "environment-control-stop"):
                soak.run_lifecycle(config["command"], config["cwd"], config["environment"],
                                   "max", root / "out")
            self.assert_environment(root / "python", "max")

    def test_soak_overlapping_worker_environment_reaches_both_children(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            commands = {sdk: self.config(root, sdk) for sdk in ("python", "node")}
            limits = {"mem_floor_bytes": 1, "free_floor_bytes": 1, "storage_limit_bytes": 1048576,
                      "arm_timeout_s": 3900, "request_timeout_s": 3}

            def environment_control(sdk, mode, iteration, snapshot, emit):
                return {"sdk": sdk, "pool_mode": mode, "aborted": None, "samples": []}

            result = soak.run_concurrent_arm(commands, "on", root / "out", limits,
                                             collect=environment_control)
            for sdk in ("python", "node"):
                self.assertEqual(result[sdk]["exit_code"], 0)
                self.assert_environment(root / sdk, "0")
                self.assertEqual(soak.evaluate_arm(result[sdk])["status"], "UNMEASURED")


class NodeWorkerControls(unittest.TestCase):
    def test_loaded_addon_paths_resolve_without_array_callback_options(self):
        source = (Path(__file__).resolve().parents[2]
                  / "dev/experiments/slice130-soak/node_consumer.mjs").read_text()
        expression = source.split("const loaded = ", 1)[1].split(";\n", 1)[0]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            addon = root / "installed.node"
            addon.touch()
            alias = root / "alias.node"
            alias.symlink_to(addon)
            # Evaluate the worker's actual cache-path expression against real
            # filesystem paths, without loading an addon or claiming native work.
            code = '''
import { realpathSync } from "node:fs";
const require = { cache: { [process.argv[1]]: {}, "ignored.js": {} } };
console.log(JSON.stringify(''' + expression + '''));
'''
            completed = __import__("subprocess").run(
                ["node", "--input-type=module", "-e", code, str(alias)],
                capture_output=True, text=True, timeout=10,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertEqual(json.loads(completed.stdout), [str(addon.resolve())])


if __name__ == "__main__":
    unittest.main()
