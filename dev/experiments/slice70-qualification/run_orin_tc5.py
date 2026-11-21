#!/usr/bin/env python3
"""Run only the frozen TC5 bridge under the established Orin resource guard."""

import argparse
import fcntl
import json
import importlib.util
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "dev/tools"))
import slice50_orin_profile as guard  # noqa: E402

GROUP_SPEC = importlib.util.spec_from_file_location(
    "tc5_owned_group", ROOT / "dev/experiments/0.8.29-slice30-measurements/g_attempt.py"
)
GROUP = importlib.util.module_from_spec(GROUP_SPEC)
GROUP_SPEC.loader.exec_module(GROUP)
STOP_REQUESTED = False


def _interrupted(_signum, _frame):
    global STOP_REQUESTED
    STOP_REQUESTED = True


def owned_sample_rss(leader, birth, peers):
    """Count checked owner snapshots conservatively without racing a second child lookup."""
    root = guard.proc(leader)
    if (root["Tgid"], root["PGID"], root["SID"], root["start_ticks"]) != (leader, leader, leader, birth):
        raise RuntimeError("owned leader identity changed")
    owners = {str(leader): root}
    required = {"Name", "Tgid", "PGID", "SID", "start_ticks", "rss"}
    for pid in peers:
        if pid == guard.last_fuser:
            continue
        checked = guard.last_details.get(str(pid), {})
        if "error" in checked or not required <= checked.keys() or (checked["PGID"], checked["SID"]) != (leader, leader):
            raise RuntimeError(f"unknown observed GPU owner: {pid}: {checked}")
        tgid = str(checked["Tgid"])
        if tgid != str(leader) and (tgid not in owners or checked["rss"] > owners[tgid]["rss"]):
            owners[tgid] = checked
    # A child may already have exited; including its last checked RSS overcounts safely.
    return sum(row["rss"] for row in owners.values()) + (guard.last_fuser_rss or 0), owners


def main():
    """Run the selected bridge; stop owned work before releasing its GPU lease."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--resume-from", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    if not str(output).startswith("/tmp/fathomdb-0830-slice70/") or output.exists():
        raise ValueError("fresh Slice70-owned output required")
    config = json.loads(args.config.read_text())
    uuid = "GPU-bbbe9f37-7028-556a-930b-54e5f3b67a82"
    if config["release"] != "0.8.30" or config["runtime"]["cuda_uuid"] != uuid:
        raise ValueError("only the selected .30 Orin bridge is admitted")
    if Path(sys.executable).absolute() != Path(config["runtime"]["python"]).absolute():
        raise ValueError("use the configured owned interpreter")
    output.mkdir(parents=True)
    guard.owned_root = output.parent
    guard.output_root = output
    env = {key: value for key, value in os.environ.items() if not key.startswith("FATHOMDB_")} | {
        "CUDA_VISIBLE_DEVICES": uuid,
        "FATHOMDB_SELECTED_UUID": uuid,
        "FATHOMDB_EMBED_DEVICE": "cuda:0",
        "FATHOMDB_RERANK_DEVICE": "cpu",
        "FATHOMDB_POOL_MODE": "on",
        "FATHOMDB_POOL_MAXSIZE": "3G",
        "FATHOMDB_POOL_RELEASE_THRESHOLD": "0",
        "FATHOMDB_REQUIRE_LIVE": "1",
        "HF_HUB_OFFLINE": "1",
        "TRANSFORMERS_OFFLINE": "1",
    }
    command = [
        sys.executable, "-m", "experiments.tc5_gpu_v2", "run", "--config",
        str(args.config.resolve()), "--arm", "bridge", "--output-root",
        str(output / "bridge"), "--binary", config["candidate"]["benchmark_binary"],
    ]
    if args.resume_from is not None:
        prior = args.resume_from.resolve()
        if not str(prior).startswith("/tmp/fathomdb-0830-slice70/"):
            raise ValueError("only the owned prior bridge may be reused")
        command.extend(["--resume-from", str(prior)])
    lock = Path("/tmp/fathomdb-gpu.lock")
    with lock.open("rb") as lease:
        if os.fstat(lease.fileno()).st_ino != 32899085:
            raise RuntimeError("GPU lease inode changed")
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if not guard.resource(True)["ok"] or guard.actors():
            raise RuntimeError("resource or observed-peer preflight refused")
        process = None
        start = time.monotonic()
        deadline = start + 1800
        old_handlers = {sig: signal.signal(sig, _interrupted) for sig in (signal.SIGTERM, signal.SIGINT)}
        with (output / "run.log").open("w") as log, (output / "resources.jsonl").open("w") as rows:
            try:
                if STOP_REQUESTED:
                    raise KeyboardInterrupt("stop requested before dispatch")
                process = subprocess.Popen(command, env=env, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                birth = guard.proc(process.pid)["start_ticks"]
                while True:
                    if STOP_REQUESTED or time.monotonic() >= deadline:
                        raise RuntimeError("stop or30-minute deadline")
                    code = GROUP.unreaped_exit(process)
                    if code is not None:
                        if GROUP.live_owned_pids(process):
                            raise RuntimeError("owned work survived its leader")
                        if code != 0 or guard.actors() or time.monotonic() >= deadline:
                            raise RuntimeError("TC5 failed, deadline expired or observed GPU owners remained")
                        break
                    peers = guard.actors()
                    resource = guard.resource()
                    total, identities = owned_sample_rss(process.pid, birth, peers)
                    resource.update(elapsed=time.monotonic() - start, rss=total, gpu_owners=identities)
                    rows.write(json.dumps(resource) + "\n")
                    rows.flush()
                    if not resource["ok"] or total > 16 * 2**30:
                        raise RuntimeError("resource guard refused")
                    time.sleep(1)
            finally:
                try:
                    if process is not None:
                        try:
                            if GROUP.live_owned_pids(process):
                                GROUP.stop_owned_group(process)
                            else:
                                process.wait(timeout=5)
                        except BaseException as error:
                            try:
                                (output / "cleanup-error.txt").write_text(repr(error) + "\n")
                                print("owned cleanup uncertain; retaining GPU lease until quiescence", file=sys.stderr, flush=True)
                            finally:
                                while True:
                                    try:
                                        if not GROUP.live_owned_pids(process):
                                            process.wait()
                                            break
                                    except BaseException:
                                        pass
                                    time.sleep(0.2)
                            raise
                        if guard.actors():
                            raise RuntimeError("observed GPU owners remain after owned group cleanup")
                finally:
                    for sig, handler in old_handlers.items():
                        signal.signal(sig, handler)
        print(json.dumps({"exit": process.returncode, "seconds": time.monotonic() - start}))


if __name__ == "__main__":
    main()
