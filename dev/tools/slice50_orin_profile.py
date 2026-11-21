#!/usr/bin/env python3
"""Run the selected default Node pair under the existing Orin observed-peer guard."""

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pathlib
import re
import shutil
import subprocess
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import slice130_soak as soak

# Observed-peer classification and identity checks reuse the completed Slice 60
# D45 guard; accessible /proc entries never establish host-wide peer visibility.
CAMPAIGN = "healthy"
owned_root = Path("/tmp")
output_root = Path("/tmp")
last_fuser = None
last_details = {}
last_fuser_rss = None


def classify_fuser_stderr(stderr):
    warnings = []
    for raw in stderr.splitlines():
        line = raw.strip()
        if not line:
            continue
        m = re.fullmatch(
            r"Cannot stat file /proc/([0-9]+)/fd/([0-9]+): Permission denied", line
        )
        if m:
            warnings.append({"pid": int(m.group(1)), "fd": int(m.group(2))})
            continue
        if re.fullmatch(
            r"(?:/dev/nvhost-gpu:|/dev/nvmap:)(?:\s*(?:/dev/nvhost-gpu:|/dev/nvmap:))*",
            line,
        ):
            continue
        raise RuntimeError("unexpected fuser stderr: " + line)
    return warnings


last_fuser = None
last_details = {}
last_fuser_rss = None


def proc(pid):
    status = pathlib.Path(f"/proc/{pid}/status").read_text()
    fields = {
        x.split(":", 1)[0]: x.split(":", 1)[1].strip()
        for x in status.splitlines()
        if ":" in x
    }
    stat = pathlib.Path(f"/proc/{pid}/stat").read_text()
    tail = stat[stat.rfind(")") + 2 :].split()
    return {
        "Name": fields.get("Name"),
        "State": fields.get("State"),
        "Tgid": int(fields["Tgid"]),
        "PPid": int(fields["PPid"]),
        "PGID": os.getpgid(pid),
        "SID": os.getsid(pid),
        "start_ticks": tail[19],
        "rss": int(fields.get("VmRSS", "0 kB").split()[0]) * 1024,
    }


def actors():
    global last_fuser, last_details, last_fuser_rss
    f = None
    last_fuser = None
    last_fuser_rss = None
    last_details = {}
    try:
        env = os.environ.copy()
        env["LC_ALL"] = "C"
        f = subprocess.Popen(
            ["/usr/bin/fuser", "/dev/nvhost-gpu", "/dev/nvmap"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env,
        )
        last_fuser = f.pid
        try:
            last_fuser_rss = proc(f.pid)["rss"]
        except (OSError, KeyError, IndexError):
            pass
        stdout, stderr = f.communicate(timeout=2)
    finally:
        if f is not None:
            if f.poll() is None:
                try:
                    f.terminate()
                except ProcessLookupError:
                    pass
                try:
                    f.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    try:
                        f.kill()
                    except ProcessLookupError:
                        pass
                    f.wait(timeout=5)
            else:
                f.wait(timeout=5)
    record = {
        "helper_pid": last_fuser,
        "returncode": f.returncode,
        "stderr": stderr,
        "stdout": stdout,
        "locale": "C",
    }
    try:
        if f.returncode not in (0, 1):
            raise RuntimeError(f"unexpected fuser exit {f.returncode}")
        record["visibility_warnings"] = classify_fuser_stderr(stderr)
    except BaseException as exc:
        record["error"] = repr(exc)
        with (output_root / "fuser-observations.jsonl").open("a") as evidence:
            evidence.write(json.dumps(record) + "\n")
        raise
    with (output_root / "fuser-observations.jsonl").open("a") as evidence:
        evidence.write(json.dumps(record) + "\n")
    ids = sorted(set(int(x) for x in stdout.split()))
    for pid in ids:
        try:
            last_details[str(pid)] = proc(pid)
        except (OSError, KeyError, IndexError) as e:
            last_details[str(pid)] = {"error": repr(e)}
    return ids


def owned_rss(leader, birth, ids):
    tgids = {leader}
    for pid in ids:
        if pid == last_fuser:
            continue
        d = last_details.get(str(pid), {})
        if pid == leader or d.get("Tgid") == leader or d.get("PGID") == leader:
            if "Tgid" not in d:
                raise RuntimeError(f"cannot identify owned GPU PID {pid}: {d}")
            tgids.add(d["Tgid"])
        else:
            raise RuntimeError(f"unknown GPU peer {pid}: {d}")
    result = {}
    for tgid in sorted(tgids):
        first = proc(tgid)
        second = proc(tgid)
        if (
            first["Tgid"] != tgid
            or first["PGID"] != leader
            or first["SID"] != leader
            or first["start_ticks"] != second["start_ticks"]
            or second["PGID"] != leader
            or second["SID"] != leader
        ):
            raise RuntimeError(f"changed owned process identity {tgid}")
        if tgid == leader and first["start_ticks"] != birth:
            raise RuntimeError("child PID reused")
        result[str(tgid)] = first
    return sum(x["rss"] for x in result.values()) + (last_fuser_rss or 0), result


def ordinary_owned_rss(leader, birth, ids):
    if CAMPAIGN != "healthy":
        return owned_rss(leader, birth, ids)
    identity = (leader, birth)
    if getattr(ordinary_owned_rss, "identity", None) != identity:
        ordinary_owned_rss.identity = identity
        ordinary_owned_rss.history = {}
    history = ordinary_owned_rss.history
    fields = ("Name", "Tgid", "PPid", "PGID", "SID", "start_ticks")

    def gone(pid):
        previous = history.get(str(pid))
        if previous is None:
            raise RuntimeError(f"unseen GPU PID disappeared: {pid}")
        try:
            proc(pid)
        except FileNotFoundError:
            last_details[str(pid)] = {
                "exited_owned_history": dict(previous),
                "current_ownership": "unproven; process now absent",
            }
            return
        raise RuntimeError(f"GPU PID became visible again: {pid}")

    live = []
    for pid in ids:
        d = last_details.get(str(pid), {})
        previous = history.get(str(pid))
        if previous is not None:
            if str(d.get("error", "")).startswith("FileNotFoundError("):
                gone(pid)
                continue
            if any(d.get(k) != previous[k] for k in fields):
                raise RuntimeError(f"previously owned GPU PID identity changed: {pid}")
        live.append(pid)
    # A verified helper can also exit between the actor lookup and RSS read.
    # Each removed PID is checked against this child's full identity history.
    for _ in range(len(live) + 1):
        try:
            total, current = owned_rss(leader, birth, live)
            break
        except FileNotFoundError as error:
            match = re.fullmatch(r"/proc/([0-9]+)/(?:status|stat)", str(error.filename))
            if not match:
                raise
            pid = int(match.group(1))
            if pid == leader or pid not in live:
                raise
            gone(pid)
            live.remove(pid)
    else:
        raise RuntimeError("owned RSS observation did not finish")
    for key, d in current.items():
        if int(key) == leader:
            continue
        if (
            d["Name"] != "nvidia-smi"
            or d["PPid"] != leader
            or d["PGID"] != leader
            or d["SID"] != leader
        ):
            raise RuntimeError(f"unexpected owned GPU helper: {key}: {d}")
        previous = history.get(key)
        if previous is not None and any(d[k] != previous[k] for k in fields):
            raise RuntimeError(f"owned helper PID reused: {key}")
        history[key] = dict(d)
    return total, current


def mem():
    return {
        k: int(v.split()[0]) * 1024
        for k, v in (
            line.split(":", 1)
            for line in pathlib.Path("/proc/meminfo").read_text().splitlines()
        )
    }


def resource(start=False):
    m = mem()
    d = shutil.disk_usage(owned_root).free
    return {
        "available": m["MemAvailable"],
        "swap": m["SwapFree"],
        "gpu_free": None,
        "gpu_used": None,
        "disk": d,
        "ok": m["MemAvailable"] >= (12 if start else 8) * 2**30
        and m["SwapFree"] >= (2 if start else 1) * 2**30
        and d >= (32 if start else 16) * 2**30,
    }


def rss(pid):
    try:
        for line in pathlib.Path(f"/proc/{pid}/status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except FileNotFoundError:
        pass
    return 0


def confirmed_owned_exit(process, recorded_birth, observed_peers, *, deadline=None):
    if any(pid != process.pid for pid in observed_peers):
        return False
    if (
        recorded_birth is None
        or getattr(process, "_slice50_birth", None) != recorded_birth
    ):
        return False
    started = time.monotonic()
    deadline = min(started + 0.1, deadline) if deadline is not None else started + 0.1
    if process.poll() is None:
        # procfs can lose the main thread's exe before the kernel makes the
        # registered child waitable. Completion still requires an actual reap.
        remaining = min(0.05, deadline - time.monotonic())
        if remaining <= 0:
            return False
        try:
            process.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            return False
    if process.returncode != 0:
        return False
    try:
        proc(process.pid)
    except FileNotFoundError:
        return not actors() and time.monotonic() <= deadline
    return False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("spec", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    global owned_root, output_root
    output_root = args.output.resolve()
    owned_root = output_root.parent
    if (
        not str(owned_root).startswith("/tmp/fathomdb-0830-slice50-")
        or output_root.exists()
    ):
        raise ValueError("fresh Slice 50-owned /tmp output required")
    spec = json.loads(args.spec.read_text())
    if spec.get("profile") != "node-default-10m" or set(spec.get("workers", {})) != {
        "node"
    }:
        raise ValueError("only the reviewed Node default profile is accepted")
    node = Path(spec["workers"]["node"]["command"][0]).resolve()
    assert (
        subprocess.check_output([str(node), "--version"], text=True, timeout=5).strip()
        == "v25.9.0"
    )
    assert not os.environ.get("NODE_OPTIONS")
    uuid = "GPU-bbbe9f37-7028-556a-930b-54e5f3b67a82"
    env = spec["workers"]["node"].get("environment", {})
    for key in ("CUDA_VISIBLE_DEVICES", "FATHOMDB_SELECTED_UUID"):
        assert env.get(key) == uuid
    assert env.get("FATHOMDB_POOL_MAXSIZE") == "3G"
    for policy in ("FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE"):
        assert env.get(policy) == "cuda:0"
    command = spec["workers"]["node"]["command"]
    consumer = Path(command[command.index("--consumer-root") + 1]).resolve()
    package = consumer / "node_modules/fathomdb"
    counterpart = consumer / "node_modules/fathomdb-linux-arm64-gnu-tegra"
    cache = Path(env["XDG_CACHE_HOME"]) / "fathomdb"
    models = {
        path.resolve()
        for path in cache.rglob("*")
        if path.is_file() and path.suffix != ".lock"
    }
    if len(models) != 6:
        raise ValueError("the selected pinned fixture requires six model files")
    required = models | {
        node,
        Path(command[1]).resolve(),
        Path(__file__).resolve(),
        Path(soak.__file__).resolve(),
        (package / "package.json").resolve(),
        (counterpart / "package.json").resolve(),
        (counterpart / "fathomdb.linux-arm64-gnu.node").resolve(),
    }
    required.update(path.resolve() for path in (package / "dist").rglob("*.js"))
    if not required.issubset({Path(path).resolve() for path in spec["subjects"]}):
        raise ValueError(
            "incomplete frozen executable, worker, package, addon, model or sampler subjects"
        )
    identity = {}
    for filename, expected in spec["subjects"].items():
        actual = hashlib.sha256(Path(filename).read_bytes()).hexdigest()
        assert actual == expected, filename
        identity[filename] = actual
    output_root.mkdir(parents=True)
    (output_root / "subjects.json").write_text(json.dumps(identity, indent=2) + "\n")
    original_memory = soak.read_linux_memory
    started = time.monotonic()
    history = {}
    last_guard = [None]
    owned_workers = []
    original_worker = soak.Worker

    class ObservedWorker(original_worker):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            owned_workers.append(self.process)

    soak.Worker = ObservedWorker

    def checked_memory():
        checked_at = time.monotonic()
        memory = original_memory()
        resource_now = resource()
        if not resource_now["ok"] or time.monotonic() - started > 1800:
            raise soak.ResourceStop("Orin resource floor or shared 30-minute deadline")
        # The collector supplies its actual owned Popen objects; this does
        # not depend on optional procfs children entries on the Tegra kernel.
        children = [process.pid for process in owned_workers if process.poll() is None]
        peers = actors()
        owned_total = memory["rss_bytes"]
        owned = {}
        known_exit = None
        if children:
            if len(children) != 1:
                raise soak.ResourceStop("unexpected collector child")
            leader = children[0]
            process = next(
                item for item in reversed(owned_workers) if item.pid == leader
            )
            try:
                actual = proc(leader)
                if actual["PPid"] != os.getpid():
                    raise soak.ResourceStop("unexpected Node worker parent")
                prior = history.get(leader)
                if str(actual["State"]).startswith("Z"):
                    last_guard[0] = None
                    if prior != actual["start_ticks"] or not confirmed_owned_exit(
                        process, prior, peers, deadline=started + 1800
                    ):
                        raise soak.ResourceStop(
                            "unproven exited worker or remaining peer"
                        )
                    known_exit = {
                        "pid": leader,
                        "start_ticks": prior,
                        "exit_code": process.returncode,
                        "stale_peers": peers,
                        "fresh_peers": [],
                    }
                else:
                    if Path(f"/proc/{leader}/exe").resolve(strict=True) != node:
                        raise soak.ResourceStop("unexpected Node worker executable")
                    identity = (leader, actual["start_ticks"])
                    if (
                        last_guard[0] is not None
                        and last_guard[0][:2] == identity
                        and checked_at - last_guard[0][2] > 0.1
                    ):
                        raise soak.ResourceStop(
                            "observed-peer guard exceeded 100ms live-worker interval"
                        )
                    last_guard[0] = (*identity, checked_at)
                    prior = history.setdefault(leader, actual["start_ticks"])
                    process._slice50_birth = prior
                    total, owned = ordinary_owned_rss(leader, prior, peers)
                    owned_total += total
            except FileNotFoundError:
                last_guard[0] = None
                prior = history.get(leader)
                if not confirmed_owned_exit(
                    process, prior, peers, deadline=started + 1800
                ):
                    raise soak.ResourceStop(
                        "worker identity inaccessible while live or with peers"
                    )
                known_exit = {
                    "pid": leader,
                    "start_ticks": prior,
                    "exit_code": process.returncode,
                    "stale_peers": peers,
                    "fresh_peers": [],
                }
        elif peers:
            raise soak.ResourceStop("unknown observed GPU peer")
        else:
            last_guard[0] = None
        if owned_total > 16 * 2**30:
            raise soak.ResourceStop("owned RSS exceeded 16GiB")
        with (output_root / "guard.jsonl").open("a") as log:
            log.write(
                json.dumps(
                    {
                        "elapsed_s": time.monotonic() - started,
                        "resources": resource_now,
                        "owned_rss": owned_total,
                        "owned": owned,
                        "actors": peers,
                        "confirmed_owned_exit": known_exit,
                    }
                )
                + "\n"
            )
        return memory

    lock_path = Path("/tmp/fathomdb-gpu.lock")
    with lock_path.open("rb") as lock:
        assert os.fstat(lock.fileno()).st_ino == 32899085
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        before = resource(True)
        assert before["ok"] and not actors()
        (output_root / "preflight.json").write_text(
            json.dumps(
                {
                    "resources": before,
                    "uuid": uuid,
                    "gpu_total_used": "unavailable",
                    "peer_visibility": "unprivileged fuser observed-only",
                }
            )
        )
        soak.read_linux_memory = checked_memory
        result = soak.collect_pair(spec, output_root / "measurement")
        (output_root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        assert not actors(), "GPU actor remained after pair"
        print(json.dumps(result), flush=True)
        return (
            0
            if set(result.get("pairs", {})) == {"node"}
            and result["pairs"]["node"]["status"] == "PASS"
            else 1
        )


if __name__ == "__main__":
    raise SystemExit(main())
