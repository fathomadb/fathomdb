"""Run only the two remaining current-pin CUDA source selections under the owned lease."""

import ctypes, fcntl, hashlib, json, os, pathlib, shlex, shutil, signal, subprocess, sys, time

root = pathlib.Path(__file__).resolve().parents[3]
node26 = sys.argv[1:] == ["--node26"]
out = pathlib.Path("/tmp/fathomdb-0830-slice75-evidence/" + ("node26-shipping/gpu" if node26 else "terra-cuda"))
if node26:
    out.mkdir()
cache = pathlib.Path("/tmp/fathomdb-0830-slice70-evidence/cache")
uuid = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b"


stop_requested = False


def interrupted(signum, frame):
    global stop_requested
    stop_requested = True


signal.signal(signal.SIGTERM, interrupted)
signal.signal(signal.SIGINT, interrupted)
if ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) != 0:
    raise OSError(ctypes.get_errno(), "PR_SET_CHILD_SUBREAPER")
lock = pathlib.Path("/mnt/data/tmp2/fathomdb-0829-continuation/slice30-gd/locks") / (
    uuid + ".lock"
)
env = os.environ.copy()
for k in [
    "FATHOMDB_SKIP_NETWORK_TESTS",
    "FATHOMDB_WRITE_CALIBRATION_RECORD",
    "FATHOMDB_SLICE72_STRESS",
    "CUDA_FORCE_PTX_JIT",
    "CUDA_DISABLE_PTX_JIT",
    "FATHOMDB_CAL_LEG",
    "FATHOMDB_CAL_OUT",
]:
    env.pop(k, None)
env.update(
    CARGO_HOME="/mnt/data/tmp2/fathomdb-0830-slice60/cuda-build-cargo-home",
    CARGO_TARGET_DIR=str(out / "target"),
    CARGO_NET_OFFLINE="true",
    XDG_CACHE_HOME=str(cache),
    FATHOMDB_REQUIRE_LIVE="1",
    HF_HUB_OFFLINE="1",
    TRANSFORMERS_OFFLINE="1",
    CUDA_DEVICE_ORDER="PCI_BUS_ID",
    CUDA_VISIBLE_DEVICES=uuid,
    FATHOMDB_EMBED_DEVICE="auto",
    ORT_DYLIB_PATH=str(cache / "fathomdb/onnxruntime/1.26.0/libonnxruntime.so.1.26.0"),
    FATHOMDB_ONNX_MODEL_PATH=str(
        cache / "fathomdb/embedders/onnx/bge-small-en-v1.5/model.onnx"
    ),
    FATHOMDB_ONNX_TOKENIZER_PATH=str(
        cache / "fathomdb/embedders/0b2926f8a9b1/tokenizer.json"
    ),
    TMPDIR=str(out / "tmp"),
    CARGO_TARGET_TMPDIR=str(out / "tmp"),
)
env["CUDA_COMPUTE_CAP"] = "75"
env["CANDLE_CUDA_NATIVE"] = "0"
if sys.argv[1:] == ["--build"]:
    env["CUDA_VISIBLE_DEVICES"] = ""
env["CUDA_PATH"] = "/usr/local/cuda-12.6"
env["CUDA_HOME"] = "/usr/local/cuda-12.6"
env["NVCC"] = "/usr/local/cuda-12.6/bin/nvcc"
env["CUDARC_CUDA_VERSION"] = "12060"
env["PATH"] = "/usr/local/cuda-12.6/bin:" + env.get("PATH", "")
env["LIBRARY_PATH"] = "/usr/local/cuda-12.6/lib64:" + env.get("LIBRARY_PATH", "")
(out / "tmp").mkdir(exist_ok=True)
cmds = [
    [
        "cargo",
        "test",
        "--locked",
        "--release",
        "-p",
        "fathomdb-embedder",
        "--no-default-features",
        "--features",
        "default-embedder,onnx-embedder,embed-cuda,rerank-cuda",
        "--test",
        "cross_backend_calibration",
        "--no-fail-fast",
    ],
    [
        "cargo",
        "test",
        "--locked",
        "--release",
        "-p",
        "fathomdb-engine",
        "--no-default-features",
        "--features",
        "slice72-gpu-tests,test-hooks",
        "--test",
        "slice72_concurrent_gpu",
        "--no-fail-fast",
    ],
]
selectors = [
    [
        "calibration_candle_cuda_leg_measures_on_gpu",
        "calibration_reports_p1_flips_and_p2_l2",
    ],
    [
        "basic_shared_cuda_device_runs_real_bge_and_ce",
        "bounded_overlap_characterizes_shared_cuda_residency",
    ],
]


def mem():
    d = {}
    for line in pathlib.Path("/proc/meminfo").read_text().splitlines():
        k, v = line.split(":", 1)
        d[k] = int(v.split()[0]) * 1024
    return d


def members(pgid):
    pids = {}
    total = 0
    for f in pathlib.Path("/proc").iterdir():
        if not f.name.isdigit():
            continue
        try:
            s = (f / "stat").read_text()
            parts = s[s.rfind(")") + 2 :].split()
            if int(parts[2]) != pgid:
                continue
            rss = 0
            for line in (f / "status").read_text().splitlines():
                if line.startswith("VmRSS:"):
                    rss = int(line.split()[1]) * 1024
                    break
            pids[int(f.name)] = {"rss": rss, "start": parts[19], "state": parts[0]}
            total += rss
        except (FileNotFoundError, ProcessLookupError):
            pass
    return pids, total


def resource(start=False):
    m = mem()
    disk = shutil.disk_usage(out).free
    return {
        "mem_available": m["MemAvailable"],
        "swap_free": m["SwapFree"],
        "swap_total": m["SwapTotal"],
        "disk_free": disk,
        "ok": m["MemAvailable"] >= (40 if start else 24) * 2**30
        and disk >= (32 if start else 16) * 2**30,
    }


def run(name, cmd, gpu=None, deadline=None):
    pre = resource(True)
    if stop_requested or not pre["ok"] or (deadline and time.monotonic() >= deadline):
        raise RuntimeError(f"{name} stop/deadline/start floor {pre}")
    if gpu:
        free, pre_gpu_used, actors = gpu()
        if free < 8192 or actors:
            raise RuntimeError(f"{name} GPU admission free={free} actors={actors}")
    with (
        (out / (name + ".log")).open("w") as log,
        (out / (name + ".samples.jsonl")).open("w") as samples,
    ):
        log.write("COMMAND " + repr(cmd) + "\n")
        log.flush()
        p = None
        start = time.monotonic()
        prior = None
        gap = 0
        reason = None
        try:
            if stop_requested or (deadline and time.monotonic() >= deadline):
                raise RuntimeError("stop/deadline before dispatch")
            p = subprocess.Popen(
                cmd,
                cwd=root,
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            while True:
                now = time.monotonic()
                gap = max(gap, now - prior if prior else 0)
                prior = now
                group, rss = members(p.pid)
                sample = resource()
                sample.update(elapsed=now - start, group=group, rss=rss)
                if gpu:
                    free, used, actors = gpu()
                    sample.update(
                        gpu_free_mib=free, gpu_used_mib=used, gpu_actors=actors
                    )
                    unknown = [a for a in actors if a not in group]
                    if unknown:
                        reason = "unknown GPU peer " + repr(unknown)
                    if used - pre_gpu_used > 8192:
                        reason = "device 8 GiB ceiling"
                    if now - start > 120:
                        reason = "body 120 seconds"
                    if now >= deadline:
                        reason = "shared venue 30 minutes"
                if stop_requested:
                    reason = "requested termination"
                if not sample["ok"] or rss > 16 * 2**30:
                    reason = "host resource floor or owned RSS ceiling"
                samples.write(json.dumps(sample) + "\n")
                samples.flush()
                # Keep the leader unreaped until all group cleanup is complete.
                finished = os.waitid(
                    os.P_PID, p.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT
                )
                if finished is not None or reason:
                    if finished is not None and any(
                        x["state"] not in {"Z", "X"}
                        for pid, x in group.items()
                        if pid != p.pid
                    ):
                        reason = reason or "live owned descendants after leader exit"
                    break
                time.sleep(0.2 if gpu else 1)
        finally:
            if p is not None:
                group, _ = members(p.pid)
                if any(x["state"] not in {"Z", "X"} for x in group.values()):
                    finished = os.waitid(
                        os.P_PID, p.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT
                    )
                    if finished is not None:
                        reason = reason or "live owned descendants after leader exit"
                    try:
                        os.killpg(p.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    cleanup_deadline = time.monotonic() + 5
                    while time.monotonic() < cleanup_deadline:
                        group, _ = members(p.pid)
                        if not any(
                            x["state"] not in {"Z", "X"} for x in group.values()
                        ):
                            break
                        time.sleep(0.05)
                    else:
                        try:
                            os.killpg(p.pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                p.wait(timeout=5)
                reap_deadline = time.monotonic() + 5
                while time.monotonic() < reap_deadline:
                    try:
                        pid, _ = os.waitpid(-1, os.WNOHANG)
                        if pid == 0:
                            time.sleep(0.05)
                    except ChildProcessError:
                        break
                else:
                    raise RuntimeError(
                        "owned children did not reap within cleanup bound"
                    )
        rc = p.returncode
        log.write(f"\nEXIT {rc} GUARD {reason} MAX_SAMPLE_GAP_SECONDS {gap:.6f}\n")
        log.flush()
    print(name, "exit", rc, "guard", reason, "max_gap", gap, flush=True)
    if rc or reason:
        raise RuntimeError(name + " failed; see log")


if sys.argv[1:] == ["--build"]:
    for i in range(2):
        run("build-sm75-" + str(i + 1), cmds[i] + ["--no-run"])
    print("builds finished", flush=True)
    sys.exit(0)
if sys.argv[1:] not in (["--gpu"], ["--node26"]):
    raise SystemExit("usage: runner.py --build|--gpu|--node26")
sys.path.insert(0, str(root / "scripts/lib"))
import feature_complete  # noqa: E402

if not node26:
    assets = feature_complete.stage_slice72_assets(
        feature_complete.SLICE72_SUPPORT, cache, out / "assets"
    )
    (out / "receipts").mkdir(exist_ok=False)
    env.update(
        FATHOMDB_SLICE72_RUNNER="approved-nvidia",
        FATHOMDB_SLICE72_ASSET_ROOT=str(assets),
        FATHOMDB_SLICE72_RECEIPT_DIR=str(out / "receipts"),
    )
else:
    consumer = out.parent / "consumer"
    source = (root / "scripts/release/cuda-preflight.sh").read_text()
    marker = "cat > \"$WORK_DIR/gpu-napi-smoke.mjs\" <<'JS'\n"
    body = source.split(marker, 1)[1].split("\nJS\n", 1)[0]
    body = body.replace("/fathomdb-tmp", str(out)).replace("/evidence", str(out))
    probe = consumer / "gpu-napi-smoke.mjs"
    probe.write_text(body + "\n")
    env.update(HF_HOME=str(pathlib.Path.home() / ".cache/huggingface"),
               FATHOMDB_EMBED_DEVICE="cuda:0", FATHOMDB_GPU_ALLOCATION_WITNESS="1",
               LD_LIBRARY_PATH="/usr/local/cuda-12.6/lib64")
    cli_root = out.parents[1]
    base_cli = cli_root / "cuda-base-rehearsal/independent/fathomdb"
    rerank_cli = cli_root / "cuda-rerank-rehearsal/independent/fathomdb"
    cli_commands = [shlex.join([str(base_cli), "doctor", "gpu", "--json"]),
                    shlex.join([str(rerank_cli), "doctor", "reranker-gpu", "--json"])]
    node_command = shlex.join(["/tmp/fathomdb-0830-slice50-tools/node_modules/node-linux-x64/bin/node", str(probe)])
    cmds = [["bash", "-ceu", "; ".join(cli_commands + ["exec " + node_command])]]
nv = ctypes.CDLL("libnvidia-ml.so.1")


def check(rc):
    if rc:
        raise RuntimeError(f"NVML error {rc}")


check(nv.nvmlInit_v2())
handle = ctypes.c_void_p()
check(nv.nvmlDeviceGetHandleByUUID(uuid.encode(), ctypes.byref(handle)))


class Memory(ctypes.Structure):
    _fields_ = [
        ("total", ctypes.c_ulonglong),
        ("free", ctypes.c_ulonglong),
        ("used", ctypes.c_ulonglong),
    ]


class Process(ctypes.Structure):
    _fields_ = [
        ("pid", ctypes.c_uint),
        ("used", ctypes.c_ulonglong),
        ("gpu_instance", ctypes.c_uint),
        ("compute_instance", ctypes.c_uint),
    ]


def gpu():
    m = Memory()
    check(nv.nvmlDeviceGetMemoryInfo(handle, ctypes.byref(m)))
    actors = set()
    for fn in (
        nv.nvmlDeviceGetComputeRunningProcesses_v3,
        nv.nvmlDeviceGetGraphicsRunningProcesses_v3,
    ):
        arr = (Process * 256)()
        n = ctypes.c_uint(256)
        check(fn(handle, ctypes.byref(n), arr))
        actors.update(arr[i].pid for i in range(n.value))
    return m.free // 2**20, m.used // 2**20, sorted(actors)


def versions():
    return {
        "driver": subprocess.check_output(
            [
                "nvidia-smi",
                "-i",
                uuid,
                "--query-gpu=driver_version",
                "--format=csv,noheader",
            ],
            text=True,
            timeout=5,
        ).strip(),
        "cuda_primary": os.path.realpath("/usr/local/cuda"),
    }


with lock.open("rb") as lf:
    if os.fstat(lf.fileno()).st_ino != 8532374:
        raise RuntimeError("unexpected lock inode")
    fcntl.flock(lf, fcntl.LOCK_EX | fcntl.LOCK_NB)
    start = time.monotonic()
    res = resource(True)
    free, pre_gpu_used, actors = gpu()
    v = versions()
    pre = {
        "resource": res,
        "gpu_free_mib": free,
        "gpu_used_mib": pre_gpu_used,
        "gpu_actors": actors,
        "versions": v,
        "lock_inode": 8532374,
    }
    (out / "preflight.json").write_text(json.dumps(pre, indent=2))
    if (
        not res["ok"]
        or free < 8192
        or actors
        or v["driver"] != "580.178.04"
        or v["cuda_primary"] != "/usr/local/cuda-12.6"
    ):
        raise RuntimeError("preflight failed " + repr(pre))
    for i in range(len(cmds)):
        run(
            "row-" + str(i + 1),
            cmds[i] if node26 else cmds[i]
            + ["--", "--exact"]
            + selectors[i]
            + ["--nocapture", "--test-threads=1"],
            gpu,
            start + 1800,
        )
        log = (out / ("row-" + str(i + 1) + ".log")).read_text()
        if (
            ("installed N-API CUDA artifact GPU smoke: ok" if node26 else "2 passed; 0 failed") not in log
            or "SKIP " in log
            or "PENDING_EXTERNAL" in log
        ):
            raise RuntimeError("nonpassing or skipped row " + str(i + 1))
    if node26:
        report = json.loads((out / "gpu-napi-open-report.json").read_text())
        witness = report["allocation_witness"]
        def normalize(value):
            return value.removeprefix("GPU-").lower()
        if (report["effective_device"] != "cuda:0"
            or normalize(report["selected_uuid"]) != normalize(uuid)
            or normalize(witness["device_uuid"]) != normalize(uuid)):
            raise RuntimeError("Node26 selected device differs from leased GPU")
        pid = witness["process_id"]
        samples = [json.loads(line) for line in (out / "row-1.samples.jsonl").read_text().splitlines()]
        if not any(pid in row["gpu_actors"] and str(pid) in row["group"] for row in samples):
            raise RuntimeError("Node26 witness PID was not independently observed on leased GPU")
    post = {
        "resource": resource(),
        "gpu": gpu(),
        "versions": versions(),
        "elapsed": time.monotonic() - start,
    }
    (out / "postflight.json").write_text(json.dumps(post, indent=2))
    if post["versions"] != v or post["gpu"][2]:
        raise RuntimeError("postflight change or peer")
print("complete", flush=True)
