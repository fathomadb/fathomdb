"""Owned G build/test executor; exact reviewed sampler, unchanged G resource envelope."""

import json, os, pathlib, subprocess, sys, time, traceback
from g_resource import resource_sample
from g_boundary import TaskBoundary
from g_attempt import live_owned_pids, stop_owned_group, wait_unreaped
from g_closed_guard import ClosedGuard, discover

phase_started_ns = time.monotonic_ns()
root = pathlib.Path(os.environ["S30_SOURCE"]).resolve()
task = pathlib.Path(os.environ["S30_TASK"]).resolve()
phase = sys.argv[1]
cmd = sys.argv[2:]
closed_profile = discover(cmd)
closed_guard = ClosedGuard(closed_profile, phase_started_ns, task) if closed_profile else None
log_root = closed_guard.directory / "logs" if closed_guard else task / "logs"
receipt_root = closed_guard.directory / "receipts" if closed_guard else task / "receipts"
if closed_guard:
    log_root.mkdir(parents=True, exist_ok=True)
    receipt_root.mkdir(parents=True, exist_ok=True)


def git(*args):
    return subprocess.check_output(["git", *args], cwd=root).decode().strip()


def sample():
    observed = resource_sample(root, task)
    vm = dict(line.split() for line in pathlib.Path("/proc/vmstat").read_text().splitlines())
    return dict(time=time.time(), root_free=observed["root_free"], task_free=observed["task_free"],
                memory_available=observed["memory_available"], owned_bytes=observed["owned"],
                measurement_retries=observed["measurement_retries"],
                swap_pages=int(vm["pswpin"]) + int(vm["pswpout"]))

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
with (log_root / f"{phase}.log").open("w", encoding="utf-8") as log:
    child = None
    boundary = TaskBoundary(task)
    last = start
    swap_streak = 0
    code = 125
    record["cleanup_uncertain"] = False
    record["live_owned_pids"] = []
    try:
        if closed_guard:
            closed_guard.admit()
        boundary.create()
        child = subprocess.Popen(
            cmd, cwd=root, stdout=log, stderr=subprocess.STDOUT, start_new_session=True,
            pass_fds=(boundary.procs_fd,), preexec_fn=boundary.attach,
        )
        record["pid"] = child.pid
        while True:
            if closed_guard:
                common_stop = closed_guard.check()
                if common_stop:
                    record["resource_stop"] = common_stop
                    code = 124
                    break
            code = wait_unreaped(child, 30)
            if code is not None:
                survivors = live_owned_pids(child)
                record["survivors_at_leader_exit"] = survivors
                if survivors:
                    record["executor_error"] = "owned_work_survived_leader"
                    code = 125
                else:
                    child.wait()
                    if boundary.populated():
                        record["executor_error"] = "owned_boundary_work_survived_leader"
                        code = 125
                break
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
                code = 124
                break
    except BaseException as error:
        record["executor_error"] = f"{type(error).__name__}: {error}"
        traceback.print_exc(file=log)
        code = 125
    finally:
        if child is not None and child.returncode is None:
            try:
                record["live_owned_pids"] = stop_owned_group(child)
            except BaseException as error:
                record["group_cleanup_error"] = f"{type(error).__name__}: {error}"
                traceback.print_exc(file=log)
                code = 125
        try:
            boundary.cleanup()
        except BaseException as error:
            record["cleanup_uncertain"] = True
            record["live_owned_pids"] = None
            record["cleanup_error"] = f"{type(error).__name__}: {error}"
            traceback.print_exc(file=log)
            code = 125
        record["boundary"] = boundary.evidence
    record["child_exit"] = child.returncode if child else None
    record.setdefault("pid", None)

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
    record["resource_stop"] = str(error)
    if code == 0:
        code = 124
    record["exit"] = code
if closed_guard:
    record["closed_common"] = closed_guard.finish(record["cleanup_uncertain"])
    if record["closed_common"].get("reserve_exceeded"):
        record["resource_stop"] = "closed_common_reserve_exceeded"
        code = 124
        record["exit"] = code
(receipt_root / f"{phase}.json").write_text(json.dumps(record, indent=2) + "\n")
print(
    json.dumps({k: record[k] for k in ["phase", "exit", "pid", "resource_stop"]}),
    flush=True,
)
if code:
    print((log_root / f"{phase}.log").read_text(errors="replace")[-18000:])
raise SystemExit(code if code >= 0 else 128 - code)
