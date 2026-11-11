import json, os, pathlib, signal, subprocess, sys, time, shutil, hashlib

root = pathlib.Path(os.environ["S60_SOURCE"]).resolve()
task = pathlib.Path(os.environ["S60_TASK"]).resolve()
phase = sys.argv[1]
cmd = sys.argv[2:]


def git(*args):
    return subprocess.check_output(["git", *args], cwd=root).decode().strip()


def usage():
    p = subprocess.run(
        ["du", "-sb", str(task), str(root)], capture_output=True, text=True
    )
    values = [
        int(l.split()[0])
        for l in p.stdout.splitlines()
        if l.split() and l.split()[0].isdigit()
    ]
    if len(values) != 2:
        p = subprocess.run(
            ["du", "-sb", str(task), str(root)], capture_output=True, text=True
        )
        values = [
            int(l.split()[0])
            for l in p.stdout.splitlines()
            if l.split() and l.split()[0].isdigit()
        ]
    if len(values) != 2:
        raise RuntimeError("owned size measurement unavailable: " + p.stderr)
    return sum(values)


def sample():
    mem = {
        a: int(b.split()[0]) * 1024
        for a, b in (
            l.split(":", 1)
            for l in pathlib.Path("/proc/meminfo").read_text().splitlines()
        )
    }
    vm = dict(l.split() for l in pathlib.Path("/proc/vmstat").read_text().splitlines())
    return dict(
        time=time.time(),
        root_free=shutil.disk_usage("/").free,
        task_free=shutil.disk_usage(task).free,
        memory_available=mem["MemAvailable"],
        owned_bytes=usage(),
        swap_pages=int(vm["pswpin"]) + int(vm["pswpout"]),
    )


start = sample()
GiB = 1024**3
if (
    start["root_free"] < 32 * GiB
    or start["task_free"] < 250 * GiB
    or start["memory_available"] < 16 * GiB
    or start["owned_bytes"] >= 200 * GiB
):
    raise SystemExit("resource start floor fails: " + json.dumps(start))
record = dict(
    phase=phase,
    command=cmd,
    cwd=str(root),
    sha_start=git("rev-parse", "HEAD"),
    delta_start=git("diff", "HEAD", "--stat"),
    started=time.time(),
    samples=[start],
    resource_stop=None,
)
with (task / "logs" / f"{phase}.log").open("wb") as log:
    child = subprocess.Popen(
        cmd, cwd=root, stdout=log, stderr=subprocess.STDOUT, start_new_session=True
    )
    record["pid"] = child.pid
    last = start
    swap_streak = 0
    while True:
        try:
            code = child.wait(timeout=30)
            break
        except subprocess.TimeoutExpired:
            try:
                s = sample()
                record["samples"].append(s)
                rate = (
                    (s["swap_pages"] - last["swap_pages"])
                    * os.sysconf("SC_PAGE_SIZE")
                    / max(s["time"] - last["time"], 1)
                )
                swap_streak = swap_streak + 1 if rate > 64 * 1024**2 else 0
                last = s
                stop = (
                    s["root_free"] < 20 * GiB
                    or s["task_free"] < 50 * GiB
                    or s["memory_available"] < 8 * GiB
                    or s["owned_bytes"] >= 200 * GiB
                    or swap_streak >= 3
                )
            except Exception as error:
                record["resource_stop"] = str(error)
                stop = True
            if stop:
                record["resource_stop"] = record["resource_stop"] or s
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    code = child.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    code = child.wait()
                break
record.update(
    exit=code,
    ended=time.time(),
    sha_end=git("rev-parse", "HEAD"),
    delta_end=git("diff", "HEAD", "--stat"),
)
try:
    record["samples"].append(sample())
except Exception as error:
    record["final_sample_error"] = str(error)
(task / "receipts" / f"{phase}.json").write_text(json.dumps(record, indent=2) + "\n")
print(
    json.dumps({k: record[k] for k in ["phase", "exit", "pid", "resource_stop"]}),
    flush=True,
)
if code:
    print((task / "logs" / f"{phase}.log").read_text(errors="replace")[-18000:])
raise SystemExit(code if code >= 0 else 128 - code)
