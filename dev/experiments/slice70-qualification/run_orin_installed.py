"""Run maintained installed Orin consumers under the existing observed-peer guard."""

import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import tarfile
import time

ROOT = Path(__file__).resolve().parents[3]
BASE = Path("/tmp/fathomdb-0830-slice75")
CANDIDATE = "53400f69d29cbc5029f0c759d58a5edb1f76e874"
sys.path.insert(0, str(ROOT / "dev/tools"))
import slice50_orin_profile as guard  # noqa: E402

spec = importlib.util.spec_from_file_location(
    "tc5_guard", ROOT / "dev/experiments/slice70-qualification/run_orin_tc5.py"
)
controller = importlib.util.module_from_spec(spec)
spec.loader.exec_module(controller)
GROUP = controller.GROUP
verdict_spec = importlib.util.spec_from_file_location(
    "tegra_verdict", ROOT / "scripts/release/smoke/tegra_node_package_verdict.py"
)
verdict = importlib.util.module_from_spec(verdict_spec)
verdict_spec.loader.exec_module(verdict)
stopped = False


def interrupted(_signum, _frame):
    global stopped
    stopped = True


def run(name, command, environment, deadline):
    if (
        stopped
        or time.monotonic() >= deadline
        or not guard.resource(True)["ok"]
        or guard.actors()
    ):
        raise RuntimeError("Orin admission refused: " + name)
    child = None
    output = BASE / "runtime" / name
    output.mkdir()
    with (
        (output / "run.log").open("w") as log,
        (output / "samples.jsonl").open("w") as samples,
    ):
        try:
            if stopped or time.monotonic() >= deadline:
                raise RuntimeError("Orin stop/deadline before dispatch")
            child = subprocess.Popen(
                command,
                cwd=environment.get("CONSUMER_CWD", str(ROOT)),
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            birth = guard.proc(child.pid)["start_ticks"]
            until = min(deadline, time.monotonic() + 120)
            while True:
                if stopped or time.monotonic() >= until:
                    raise RuntimeError("Orin stop/deadline: " + name)
                code = GROUP.unreaped_exit(child)
                if code is not None:
                    if code or GROUP.live_owned_pids(child) or guard.actors():
                        raise RuntimeError("Orin failed or owners remained: " + name)
                    break
                peers = guard.actors()
                total, identities = controller.owned_sample_rss(child.pid, birth, peers)
                for pid in GROUP.live_owned_pids(child):
                    try:
                        observed = guard.proc(pid)
                    except (FileNotFoundError, ProcessLookupError):
                        continue
                    if (observed["PGID"], observed["SID"]) != (child.pid, child.pid):
                        raise RuntimeError("owned process group changed")
                    key = str(observed["Tgid"])
                    previous = identities.get(key)
                    if previous is None or observed["rss"] > previous["rss"]:
                        total += observed["rss"] - (previous["rss"] if previous else 0)
                        identities[key] = observed
                resources = guard.resource()
                samples.write(
                    json.dumps(dict(resources, rss=total, identities=identities)) + "\n"
                )
                samples.flush()
                if not resources["ok"] or total > 16 * 2**30:
                    raise RuntimeError("Orin resource guard: " + name)
                time.sleep(0.5)
        finally:
            if child is not None:
                try:
                    if GROUP.live_owned_pids(child):
                        GROUP.stop_owned_group(child)
                    else:
                        child.wait(timeout=5)
                except BaseException:
                    print(
                        "retaining Orin lease until owned cleanup is certain",
                        file=sys.stderr,
                        flush=True,
                    )
                    while True:
                        if not GROUP.live_owned_pids(child):
                            child.wait()
                            break
                        time.sleep(0.2)
                    raise
                if guard.actors():
                    raise RuntimeError("observed owners after Orin cleanup")
    print(name, "PASS", flush=True)


def main():
    """Exercise final local packages without registry or publication claims."""
    wheel = next((BASE / "artifacts/final-python").glob("*.whl"))
    addon = next(
        (BASE / "artifacts/final-npm").glob("fathomdb-linux-arm64-gnu-tegra-*.tgz")
    )
    main = BASE / "artifacts/fathomdb-0.8.30.tgz"
    generic = BASE / "artifacts/fathomdb-linux-arm64-gnu-0.8.30.tgz"
    for path in (wheel, addon, main, generic):
        if not path.is_file() or not path.resolve().is_relative_to(BASE):
            raise ValueError("owned artifact missing: " + str(path))
    env = dict(
        os.environ,
        FATHOMDB_POOL_MODE="on",
        FATHOMDB_POOL_MAXSIZE="3G",
        FATHOMDB_POOL_RELEASE_THRESHOLD="0",
        CUDA_VISIBLE_DEVICES="GPU-bbbe9f37-7028-556a-930b-54e5f3b67a82",
        FATHOMDB_SELECTED_UUID="GPU-bbbe9f37-7028-556a-930b-54e5f3b67a82",
        HF_HUB_OFFLINE="1",
        TRANSFORMERS_OFFLINE="1",
        npm_config_cache=str(BASE / "npm-cache"),
    )
    (BASE / "tmp").mkdir(exist_ok=True)
    env["TMPDIR"] = str(BASE / "tmp")
    env["PATH"] = "/home/coreyt/.nvm/versions/node/v25.9.0/bin:" + env["PATH"]
    tools = BASE / "tools"
    tools.mkdir(exist_ok=True)
    timeout = tools / "timeout"
    timeout.write_text('#!/bin/sh\nexec /usr/bin/timeout --foreground "$@"\n')
    timeout.chmod(0o755)
    env["PATH"] = str(tools) + ":" + env["PATH"]
    consumer = BASE / "node26-consumer"
    prepared = consumer.exists()
    consumer.mkdir(exist_ok=True)
    packages = {
        "fathomdb": main,
        "fathomdb-linux-arm64-gnu-tegra": addon,
        "fathomdb-linux-arm64-gnu": generic,
    }
    manifest = {
        "private": True,
        "type": "module",
        "dependencies": {name: "file:" + str(path) for name, path in packages.items()},
    }
    if prepared:
        if json.loads((consumer / "package.json").read_text()) != manifest:
            raise RuntimeError("prepared consumer input changed")
    else:
        (consumer / "package.json").write_text(json.dumps(manifest))
        subprocess.run(
            [
                "npm",
                "install",
                "--offline",
                "--ignore-scripts",
                "--no-audit",
                "--no-fund",
            ],
            cwd=consumer,
            env=env,
            check=True,
            timeout=120,
        )
    shutil.copyfile(
        ROOT / "scripts/release/smoke/tegra_node_package_row.mjs", consumer / "row.mjs"
    )
    (consumer / "databases").mkdir(exist_ok=True)
    with tarfile.open(addon) as package:
        native_digest = hashlib.sha256(
            package.extractfile("package/fathomdb.linux-arm64-gnu.node").read()
        ).hexdigest()
    context = {
        "consumers": {"tegra": {"root": str(consumer.resolve())}},
        "tegra_node_sha256": native_digest,
    }
    guard.owned_root = BASE
    (BASE / "runtime").mkdir(exist_ok=True)
    guard.output_root = BASE / "runtime"
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    with Path("/tmp/fathomdb-gpu.lock").open("rb") as lease:
        if os.fstat(lease.fileno()).st_ino != 32899085:
            raise RuntimeError("Orin lease inode changed")
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        smi = next(
            path
            for path in (
                "/usr/bin/nvidia-smi",
                "/usr/sbin/nvidia-smi",
                "/usr/local/bin/nvidia-smi",
            )
            if os.access(path, os.X_OK)
        )
        actual = (
            subprocess.check_output(
                [smi, "--query-gpu=uuid", "--format=csv,noheader"],
                text=True,
                timeout=5,
            )
            .strip()
            .removeprefix("GPU-")
        )
        if actual != "bbbe9f37-7028-556a-930b-54e5f3b67a82" or Path(
            "/usr/local/cuda"
        ).resolve() != Path("/usr/local/cuda-12.6"):
            raise RuntimeError("Orin device/toolkit identity changed")
        deadline = time.monotonic() + 1800
        run(
            "python-policies",
            [
                "bash",
                str(ROOT / "scripts/release/smoke/smoke-tegra-pages-wheel.sh"),
                "--local-wheel",
                str(wheel),
                "0.8.30+tegra",
                hashlib.sha256(wheel.read_bytes()).hexdigest(),
                CANDIDATE,
                str(BASE / "python-installed"),
            ],
            env,
            deadline,
        )
        run(
            "node25-policies",
            [
                "bash",
                str(ROOT / "scripts/release/smoke/smoke-tegra-node-package.sh"),
                "--source",
                "local",
                "--main-tarball",
                str(main),
                "--tegra-tarball",
                str(addon),
                "--generic-tarball",
                str(generic),
                "--candidate-sha",
                CANDIDATE,
                "--evidence-dir",
                str(BASE / "node25-installed"),
            ],
            env,
            deadline,
        )
        node26 = "/home/coreyt/.nvm/versions/node/v26.10.0/bin/node"
        for policy in ("cpu", "cuda:0"):
            for component in ("embed", "rerank"):
                name = "node26-" + component + "-" + policy.replace(":", "-")
                record = BASE / "runtime" / (name + ".json")
                selected = dict(
                    env,
                    CONSUMER_CWD=str(consumer),
                    FATHOMDB_EMBED_DEVICE=policy,
                    FATHOMDB_RERANK_DEVICE=policy,
                    FATHOMDB_GPU_ALLOCATION_WITNESS="1" if policy == "cuda:0" else "0",
                )
                run(
                    name,
                    [
                        node26,
                        str(consumer / "row.mjs"),
                        "tegra",
                        component,
                        policy,
                        str(record),
                        str(consumer / "databases"),
                    ],
                    selected,
                    deadline,
                )
                observed = json.loads(record.read_text())
                expected = "cpu" if policy == "cpu" else "cuda"
                problems = verdict.check_selection(
                    observed, context, "tegra"
                ) + verdict.check_policy(observed, component, policy, expected)
                if (
                    observed.get("node") != "v26.10.0"
                    or observed.get("outcome") != "completed"
                    or problems
                ):
                    raise RuntimeError(
                        "Node26 installed outcome mismatch: "
                        + name
                        + ": "
                        + repr(problems)
                    )
    print(
        "Orin installed candidate policies complete; observed-only GPU inventory",
        flush=True,
    )


if __name__ == "__main__":
    main()
