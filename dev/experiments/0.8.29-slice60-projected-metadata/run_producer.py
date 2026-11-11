import sys, csv, hashlib, json, os, pathlib, subprocess, time

base = pathlib.Path(os.environ["S60_TASK"]).resolve() / "gpu-metadata"
run = base / sys.argv[1]
assert run.parent == base
run.mkdir(exist_ok=False)
review = pathlib.Path(__file__).resolve().parent
manifest = json.loads((review / "operator-sources.json").read_text())
for name in ["netns_exec.sh", "netns_final_exec.py"]:
    assert hashlib.sha256((review / name).read_bytes()).hexdigest() == manifest[name], (
        name
    )
binary = base / "target/debug/s60-manual-metadata-acceptance"
assert binary.is_file()
uuid = os.environ["S60_GPU_UUID"]


def sample():
    query = subprocess.run(
        [
            "nvidia-smi",
            "--query-gpu=uuid,name,utilization.gpu,memory.free,driver_version",
            "--format=csv,noheader,nounits",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    apps = subprocess.run(
        [
            "nvidia-smi",
            "--query-compute-apps=gpu_uuid,pid,used_gpu_memory",
            "--format=csv,noheader,nounits",
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    row = next(
        [v.strip() for v in row]
        for row in csv.reader(query.stdout.splitlines())
        if row[0].strip() == uuid
    )
    return {
        "time": time.time(),
        "uuid": uuid,
        "name": row[1],
        "utilization": int(row[2]),
        "free_mib": int(row[3]),
        "driver": row[4],
        "apps": [
            [v.strip() for v in row] for row in csv.reader(apps.stdout.splitlines())
        ],
        "raw_gpu": query.stdout,
        "raw_apps": apps.stdout,
    }


first = sample()
assert (
    first["utilization"] <= 5
    and first["free_mib"] >= 20480
    and first["driver"] == "580.178.04"
    and "3090" in first["name"]
)
assert not any(row[0] == uuid for row in first["apps"])
assert pathlib.Path("/usr/local/cuda").resolve() == pathlib.Path("/usr/local/cuda-12.6")
command = [
    "unshare",
    "-rUn",
    "--",
    "bash",
    str(review / "netns_exec.sh"),
    "run",
    os.readlink("/proc/self/ns/net"),
    str(binary),
    str(run),
]
record = {
    "command": command,
    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    "started": time.time(),
    "samples": [first],
    "source_sha": subprocess.check_output(
        ["git", "rev-parse", "HEAD"], text=True
    ).strip(),
    "source_delta": subprocess.check_output(
        ["git", "status", "--porcelain"], text=True
    ),
    "stop": None,
}
with (
    (run / "producer.stdout.log").open("wb") as out,
    (run / "producer.stderr.log").open("wb") as err,
    open(os.devnull, "rb") as null,
):
    child = subprocess.Popen(
        command, stdin=null, stdout=out, stderr=err, close_fds=True
    )
    record["launcher_pid"] = child.pid
    try:
        while child.poll() is None:
            point = sample()
            record["samples"].append(point)
            if time.time() - record["started"] > 240:
                raise RuntimeError("owned producer watchdog exceeded240s")
            if any(
                row[0] == uuid and int(row[1]) != child.pid for row in point["apps"]
            ):
                raise RuntimeError("foreign selected-GPU allocation appeared")
            time.sleep(0.05)
    except BaseException as error:
        record["stop"] = repr(error)
        child.terminate()
        try:
            child.wait(timeout=15)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait()
    finally:
        record["exit"] = child.wait()
        record["ended"] = time.time()
        record["samples"].append(sample())
        record["driver_primary_cuda_after"] = str(
            pathlib.Path("/usr/local/cuda").resolve()
        )
        record["engaged"] = any(
            point["utilization"] > 0
            and any(
                row[0] == uuid and int(row[1]) == child.pid and int(row[2]) > 0
                for row in point["apps"]
            )
            for point in record["samples"]
        )
        (run / "producer-monitor.json").write_text(json.dumps(record, indent=2) + "\n")
print(
    json.dumps(
        {key: record[key] for key in ["exit", "launcher_pid", "engaged", "stop"]},
        indent=2,
    )
)
assert record["stop"] is None and record["exit"] == 0, record
assert record["engaged"], (
    "actual selected-GPU utilization and attributable process memory not observed; no automatic retry"
)
assert record["driver_primary_cuda_after"] == "/usr/local/cuda-12.6"
producer = json.loads((run / "producer.json").read_text())
assert producer["producer_pid"] == record["launcher_pid"]
assert all(point["driver"] == "580.178.04" for point in record["samples"])
