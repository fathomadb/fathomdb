#!/usr/bin/env python3
"""Capture AC-082d installed explicit-CPU nonuse on an exclusively owned CUDA host."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import math
import os
import re
import shutil
import signal
import subprocess
import time
from pathlib import Path
from typing import Any

CUDA_ACCESS = re.compile(
    r"lib(?:cuda|cublas|curand|cufft|cusolver|cusparse|nvrtc|nvjitlink|nvidia)[\w.-]*"
    r"|/dev/(?:nvidia|nvhost|nvmap)[\w/-]*|/proc/driver/nvidia|/nvidia-smi",
    re.IGNORECASE,
)
EXIT_ZERO = re.compile(r"\+\+\+ exited with 0 \+\+\+\s*$")
HERE = Path(__file__).resolve().parent


def load_json(path: Path) -> dict[str, Any]:
    """Read a required object, refusing missing or incomplete capture output."""
    try:
        value = json.loads(path.read_text())
    except (OSError, ValueError) as error:
        raise ValueError(f"missing or incomplete capture: {path}: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"capture is not an object: {path}")
    return value


def validate_trace(root: Path, *, cuda_expected: bool) -> set[int]:
    """Require complete all-child syscall/loader traces with the expected CUDA access."""
    traces = list(root.glob("trace.*"))
    loaders = list(root.glob("loader.*"))
    if not traces or not loaders:
        raise ValueError("missing syscall or loader capture")
    pids = set()
    trace_access = False
    children = set()
    for path in traces:
        try:
            pid = int(path.suffix[1:])
            text = path.read_text()
        except (OSError, ValueError) as error:
            raise ValueError(f"invalid trace capture: {path}") from error
        if pid < 1 or not EXIT_ZERO.search(text):
            raise ValueError(f"incomplete or unsuccessful syscall trace: {path}")
        pids.add(pid)
        children.update(
            int(child)
            for child in re.findall(
                r"(?:clone3?|fork|vfork)(?:\(| resumed>)[^\n]* = ([1-9][0-9]*)", text
            )
        )
        if (
            re.search(r"execve(?:at)?\(", text)
            and not (root / f"loader.{pid}").is_file()
        ):
            raise ValueError(f"missing executed child's loader capture: {pid}")
        trace_access |= CUDA_ACCESS.search(text) is not None
    if children - pids:
        raise ValueError(f"missing child syscall capture: {sorted(children - pids)}")
    loader_access = False
    for path in loaders:
        text = path.read_text()
        if not all(
            marker in text
            for marker in (
                "initialize program:",
                "transferring control:",
                "calling fini:",
            )
        ):
            raise ValueError(f"incomplete loader capture: {path}")
        loader_access |= CUDA_ACCESS.search(text) is not None
    stderr = (root / "stderr.txt").read_text()
    for line in stderr.splitlines():
        if line.startswith("strace:") and not re.fullmatch(
            r"strace: Process [0-9]+ attached", line
        ):
            raise ValueError(f"failed syscall capture: {line}")
    if cuda_expected and not (trace_access and loader_access):
        raise ValueError(
            "positive control lacks CUDA access in syscall and loader captures"
        )
    if not cuda_expected and (trace_access or loader_access):
        raise ValueError("CUDA access in explicit-CPU process or child")
    return pids


def validate_capture(root: Path, policy: str, uuid: str) -> None:
    """Validate the lifecycle, live selected-device observations and complete captures."""
    positive = policy == "cuda:0"
    ready, done, record = (
        load_json(root / name)
        for name in ("ready.json", "done.json", "observations.json")
    )
    pids = validate_trace(root, cuda_expected=positive)
    pid = ready.get("pid")
    if not isinstance(pid, int) or isinstance(pid, bool) or pid not in pids:
        raise ValueError("consumer PID is not in the syscall capture")
    if (
        record.get("policy") != policy
        or ready.get("policy") != policy
        or ready.get("cuda_compiled") is not True
        or record.get("returncode") != 0
        or ready.get("vector_dim") != 384
        or ready.get("result_count") != 2
        or ready.get("reranked") is not True
        or done.get("pid") != pid
        or done.get("closed") is not True
    ):
        raise ValueError("incomplete or failed installed CUDA-compiled lifecycle")
    if record.get("selected_uuid") != uuid:
        raise ValueError("observed CUDA host UUID differs")
    if ready.get("effective_device") != ("cuda" if positive else "cpu"):
        raise ValueError("effective device differs from explicit policy")
    if ready.get("selected_uuid") != (uuid if positive else None):
        raise ValueError("consumer selected UUID differs from explicit policy")
    samples = record.get("samples", [])
    if (
        not samples
        or not any(sample.get("pid_alive") is True for sample in samples)
        or samples[-1].get("pid_alive") is not False
    ):
        raise ValueError("missing live consumer or terminal observation")
    observed = any(
        pid in sample.get("compute_pids", []) and sample.get("pid_alive") is True
        for sample in samples
    )
    if positive and not observed:
        raise ValueError("CUDA positive control lacks live PID on the selected UUID")
    if not positive and any(
        pids.intersection(sample.get("compute_pids", [])) for sample in samples
    ):
        raise ValueError(
            "explicit-CPU process or child appeared in GPU compute processes"
        )


def validate_pair(cpu: Path, positive: Path, uuid: str) -> None:
    """Qualify CPU nonuse only together with its binding's real CUDA positive control."""
    validate_capture(positive, "cuda:0", uuid)
    validate_capture(cpu, "cpu", uuid)
    if load_json(cpu / "observations.json").get("binding") != load_json(
        positive / "observations.json"
    ).get("binding"):
        raise ValueError("CPU and positive controls belong to different bindings")


def start_trace(
    command: list[str], root: Path, env: dict[str, str]
) -> subprocess.Popen[bytes]:
    """Start the command with all-child file/process/ioctl and ELF-loader capture."""
    traced_env = env.copy()
    # Apply loader diagnostics to the consumer, not to strace's own startup.
    command = [
        "strace",
        "-ff",
        "-tt",
        "-s",
        "4096",
        "-yy",
        "-e",
        "trace=%file,%process,ioctl",
        "-o",
        str(root / "trace"),
        "-E",
        "LD_DEBUG=libs,files",
        "-E",
        f"LD_DEBUG_OUTPUT={root / 'loader'}",
        "--",
        *command,
    ]
    traced_env.pop("LD_DEBUG", None)
    traced_env.pop("LD_DEBUG_OUTPUT", None)
    with (
        (root / "stdout.txt").open("wb") as stdout,
        (root / "stderr.txt").open("wb") as stderr,
    ):
        return subprocess.Popen(
            command,
            cwd=root,
            env=traced_env,
            stdout=stdout,
            stderr=stderr,
            start_new_session=True,
        )


def query_compute(uuid: str) -> list[int]:
    """Observe the selected GPU externally; this function never runs inside a consumer."""
    result = subprocess.run(
        [
            "nvidia-smi",
            f"--id={uuid}",
            "--query-compute-apps=pid",
            "--format=csv,noheader",
        ],
        check=True,
        text=True,
        capture_output=True,
        timeout=10,
    )
    return [int(line.strip()) for line in result.stdout.splitlines() if line.strip()]


def process_start(pid: int) -> str | None:
    """Return the live process start tick to detect exit or PID reuse."""
    try:
        fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        return None if fields[0] == "Z" else fields[19]
    except (OSError, IndexError):
        return None


def capture(
    binding: str,
    policy: str,
    command: list[str],
    root: Path,
    env: dict[str, str],
    uuid: str,
    timeout: float,
) -> None:
    """Capture a bounded installed lifecycle, observing GPU PIDs until all traced tasks exit."""
    root.mkdir()
    if query_compute(uuid):
        raise ValueError("selected GPU must have no pre-existing compute consumers")
    process = start_trace(command, root, env | {"FATHOMDB_CAPTURE_DIR": str(root)})
    samples = []
    deadline = time.monotonic() + timeout
    ready = None
    identity = None
    try:
        while True:
            now = time.monotonic()
            if now > deadline:
                raise TimeoutError(
                    f"{binding}/{policy} capture exceeded {timeout} seconds"
                )
            pids = query_compute(uuid)
            if ready is None and (root / "ready.json").exists():
                ready = load_json(root / "ready.json")
                identity = process_start(ready["pid"])
                if identity is None:
                    raise ValueError("consumer exited before live observation")
            alive = (
                None
                if ready is None
                else identity is not None and process_start(ready["pid"]) == identity
            )
            samples.append(
                {"monotonic_seconds": now, "compute_pids": pids, "pid_alive": alive}
            )
            # Positive controls keep their CUDA context until a live PID observation.
            if (
                ready is not None
                and alive
                and (policy == "cpu" or ready["pid"] in pids)
            ):
                (root / "release").touch()
            if process.poll() is not None:
                break
            time.sleep(0.2)
        if ready is None:
            raise ValueError("consumer never completed its lifecycle handshake")
        if process_start(ready["pid"]) == identity:
            raise ValueError("consumer still live after tracer exit")
        samples.append(
            {
                "monotonic_seconds": time.monotonic(),
                "compute_pids": query_compute(uuid),
                "pid_alive": False,
            }
        )
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
        (root / "observations.json").write_text(
            json.dumps(
                {
                    "binding": binding,
                    "policy": policy,
                    "returncode": process.returncode,
                    "selected_uuid": uuid,
                    "samples": samples,
                },
                indent=2,
            )
            + "\n"
        )
    validate_capture(root, policy, uuid)


def check_models(hf_home: Path, reranker_root: Path) -> None:
    """Require the existing release contract's pinned offline model files."""
    names = ["CUDA_DEFAULT_EMBEDDER_HF_REPO", "CUDA_DEFAULT_EMBEDDER_HF_REVISION"]
    names += [
        f"CUDA_{family}_{member}_SHA256"
        for family in ("DEFAULT_EMBEDDER", "RERANKER")
        for member in ("CONFIG", "TOKENIZER", "MODEL")
    ]
    shell = '. "$1"; printf "%s\\n" ' + " ".join(f'"${name}"' for name in names)
    values = subprocess.run(
        ["bash", "-ceu", shell, "model-pins", str(HERE / "cuda-artifact-contract.sh")],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    repo, revision, *digests = values
    snapshots = [
        hf_home / "hub" / f"models--{repo.replace('/', '--')}" / "snapshots" / revision,
        reranker_root / "fathomdb/reranker/0290849b0459",
    ]
    for snapshot in snapshots:
        for member in ("config.json", "tokenizer.json", "model.safetensors"):
            with (snapshot / member).open("rb") as handle:
                actual = hashlib.file_digest(handle, "sha256").hexdigest()
            if actual != digests.pop(0):
                raise ValueError(f"pinned offline model differs: {snapshot / member}")


def main() -> None:
    """Install exact supplied archives and capture both bindings under exclusive FD9 custody."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--gpu-uuid", required=True)
    parser.add_argument("--lock-inode", type=int, required=True)
    parser.add_argument("--python-wheel", type=Path, required=True)
    parser.add_argument("--npm-main", type=Path, required=True)
    parser.add_argument("--napi-platform", type=Path, required=True)
    parser.add_argument("--python-base", type=Path, required=True)
    parser.add_argument("--node-bin-dir", type=Path, required=True)
    parser.add_argument("--hf-home", type=Path, required=True)
    parser.add_argument("--reranker-cache-root", type=Path, required=True)
    parser.add_argument("--cuda-lib-dir", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=float, default=120)
    args = parser.parse_args()
    if not math.isfinite(args.timeout_seconds) or args.timeout_seconds <= 0:
        raise ValueError("capture timeout must be finite and positive")
    if os.fstat(9).st_ino != args.lock_inode:
        raise ValueError("wrong inherited GPU lock inode")
    fcntl.flock(9, fcntl.LOCK_EX | fcntl.LOCK_NB)
    if args.output_dir.exists():
        raise ValueError("output directory must be new")
    args.output_dir.mkdir(parents=True)
    root = args.output_dir.resolve()
    uuid = subprocess.run(
        [
            "nvidia-smi",
            f"--id={args.gpu_uuid}",
            "--query-gpu=uuid",
            "--format=csv,noheader",
        ],
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    ).stdout.strip()
    if uuid != args.gpu_uuid:
        raise ValueError("actual selected CUDA host UUID differs")
    archives = [
        args.python_wheel.resolve(),
        args.npm_main.resolve(),
        args.napi_platform.resolve(),
    ]
    identities = {}
    for path in archives:
        with path.open("rb") as handle:
            identities[str(path)] = hashlib.file_digest(handle, "sha256").hexdigest()
    (root / "artifacts.json").write_text(json.dumps(identities, indent=2) + "\n")
    check_models(args.hf_home.resolve(), args.reranker_cache_root.resolve())
    base_env = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith("FATHOMDB_")
        and key
        not in ("PYTHONPATH", "PYTHONHOME", "NODE_PATH", "LD_DEBUG", "LD_DEBUG_OUTPUT")
    }
    base_env["PATH"] = f"{args.node_bin_dir}:{base_env.get('PATH', '')}"
    python_root, node_root = root / "installed-python", root / "installed-napi"
    subprocess.run(
        [str(args.python_base), "-m", "venv", str(python_root)],
        check=True,
        env=base_env,
    )
    python = str(python_root / "bin/python")
    subprocess.run(
        [python, "-m", "pip", "install", "--no-index", "--no-deps", str(archives[0])],
        check=True,
        env=base_env,
    )
    node_root.mkdir()
    (node_root / "package.json").write_text(
        json.dumps(
            {
                "private": True,
                "type": "module",
                "dependencies": {
                    "fathomdb": f"file:{archives[1]}",
                    "fathomdb-linux-x64-gnu": f"file:{archives[2]}",
                },
            }
        )
        + "\n"
    )
    subprocess.run(
        [
            str(args.node_bin_dir / "npm"),
            "install",
            "--offline",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
        ],
        cwd=node_root,
        check=True,
        env=base_env,
    )
    node_probe = node_root / "cpu-nonuse-napi.mjs"
    shutil.copyfile(HERE / "smoke/cpu-nonuse-napi.mjs", node_probe)
    commands = {
        "python": [python, str(HERE / "smoke/cpu-nonuse-python.py")],
        "napi": [str(args.node_bin_dir / "node"), str(node_probe)],
    }
    for binding, command in commands.items():
        for policy, suffix in (("cuda:0", "positive"), ("cpu", "cpu")):
            cell = root / f"{binding}-{suffix}"
            cache = root / f"{binding}-{suffix}-cache"
            cache.mkdir()
            env = base_env | {
                "CUDA_VISIBLE_DEVICES": uuid,
                "NVIDIA_VISIBLE_DEVICES": uuid,
                "FATHOMDB_EMBED_DEVICE": policy,
                "FATHOMDB_RERANK_DEVICE": policy,
                "FATHOMDB_RERANKER_CACHE": str(args.reranker_cache_root.resolve()),
                "HF_HOME": str(args.hf_home.resolve()),
                "HF_HUB_OFFLINE": "1",
                "XDG_CACHE_HOME": str(cache),
                "HOME": str(cache),
                "LD_LIBRARY_PATH": f"{args.cuda_lib_dir.resolve()}:{base_env.get('LD_LIBRARY_PATH', '')}",
            }
            capture(binding, policy, command, cell, env, uuid, args.timeout_seconds)
        validate_pair(root / f"{binding}-cpu", root / f"{binding}-positive", uuid)
    print(
        "AC-082d: Python and N-API installed explicit-CPU nonuse and separate CUDA capture controls passed"
    )


if __name__ == "__main__":
    main()
