"""Conservative owned G/D resource accounting and bounded phase execution."""

from contextlib import contextmanager


def aggregate_roots(roots):
    """Return the sum of each disjoint root's larger allocated/apparent size."""
    return sum(max(root["allocated"], root["apparent"]) for root in roots)


def setup_growth(roots, task_path, raw_roots, baseline):
    """Account setup growth after removing raw subtrees in both byte metrics."""
    remaining = []
    for root in roots:
        row = dict(root)
        if row["path"] == task_path:
            for metric in ("allocated", "apparent"):
                row[metric] = max(0, row[metric] - sum(raw[metric] for raw in raw_roots))
        remaining.append(row)
    return max(0, aggregate_roots(remaining) - baseline)


GiB = 1024**3


def reservation_ok(owned, setup_used, g_raw, d_raw):
    """Preserve accepted setup and both raw-family reservations under200GiB."""
    if owned >= 200*GiB or setup_used > 96*GiB or g_raw > 16*GiB or d_raw > 16*GiB:
        return False
    forecast = owned + (96*GiB - setup_used) + (16*GiB - g_raw) + (16*GiB - d_raw)
    return forecast <= 200*GiB


def measure_root(path, deadline=None):
    """Accept successful du totals only, retaining both bounded attempts on failure."""
    import subprocess
    import time
    row = {"path": str(path), "warnings": []}
    for key, flag in (("allocated", "-sB1"), ("apparent", "-sb")):
        for attempt in range(2):
            if deadline is not None and time.monotonic() >= deadline:
                raise RuntimeError(f"owned size unverifiable: probe deadline exhausted before du {flag} {path}")
            command = ["du", flag, str(path)]
            try:
                process = (_deadline_probe(command, deadline) if deadline is not None else
                           subprocess.run(command, capture_output=True, text=True))
            except RuntimeError as error:
                row["warnings"].append(f"attempt {attempt+1}: {error}")
                raise RuntimeError("owned size unverifiable: " + "\n".join(row["warnings"])) from error
            lines = process.stdout.splitlines()
            fields = lines[-1].split() if lines else []
            if process.returncode == 0 and fields and fields[0].isdigit():
                row[key] = int(fields[0])
                if process.stderr:
                    row["warnings"].append(process.stderr)
                break
            row["warnings"].append(
                f"attempt {attempt+1} du {flag} {path!s}: exit={process.returncode}, "
                f"stdout={process.stdout!r}, stderr={process.stderr!r}"
            )
            if attempt == 1:
                raise RuntimeError("owned size unverifiable: " + "\n".join(row["warnings"]))
    return row


def _deadline_probe(command, deadline):
    import subprocess
    import tempfile
    import time
    if time.monotonic() + 0.02 >= deadline:
        raise RuntimeError("probe deadline leaves no owned-child cleanup allowance")
    # Files avoid an inherited pipe writer turning output collection into a wait.
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        child = subprocess.Popen(command, stdout=stdout, stderr=stderr)
        status = None
        timely = False
        try:
            while True:
                status = child.poll()
                now = time.monotonic()
                if status is not None:
                    timely = now < deadline
                    break
                if now >= deadline-0.02:
                    break
                time.sleep(min(0.002, deadline-0.02-now))
        finally:
            if child.poll() is None:
                try:
                    child.kill()
                except ProcessLookupError:
                    pass
                while time.monotonic() < deadline and child.poll() is None:
                    time.sleep(min(0.002, max(0, deadline-time.monotonic())))
        stdout.seek(0)
        stderr.seek(0)
        output, errors = stdout.read().decode(), stderr.read().decode()
        if not timely or time.monotonic() >= deadline:
            raise RuntimeError(f"probe deadline pid={child.pid} reaped={child.poll() is not None} "
                               f"stdout={output!r} stderr={errors!r}")
        assert status is not None
        return subprocess.CompletedProcess(command, status, output, errors)


def resource_sample(roots, task, baseline):
    """Observe disjoint aggregate, independent raw budgets, memory/disk and swap."""
    import os
    from pathlib import Path
    import shutil
    import time
    measured = [measure_root(path) for path in roots]
    raw = [measure_root(task / name) if (task / name).exists()
           else {"allocated":0, "apparent":0} for name in ("g", "d")]
    mem = dict(line.split(":",1) for line in Path("/proc/meminfo").read_text().splitlines())
    vm = dict(line.split() for line in Path("/proc/vmstat").read_text().splitlines())
    return {"time":time.time(), "roots":measured, "owned":aggregate_roots(measured),
            "setup_used":setup_growth(measured,str(task),raw,baseline),
            "g_raw":aggregate_roots(raw[:1]), "d_raw":aggregate_roots(raw[1:]),
            "root_free":shutil.disk_usage("/").free,"task_free":shutil.disk_usage(task).free,
            "memory_available":int(mem["MemAvailable"].split()[0])*1024,
            "swap_bytes":(int(vm["pswpin"])+int(vm["pswpout"]))*os.sysconf("SC_PAGE_SIZE")}


def resource_issue(sample, max_owned, start=False):
    """Return a resource refusal reason without hiding missing observations."""
    if sample["owned"] >= max_owned:
        return "owned_capacity"
    if not reservation_ok(sample["owned"],sample["setup_used"],sample["g_raw"],sample["d_raw"]):
        return "accepted_reservation"
    for key, floor in (("root_free",32 if start else 20),
                       ("task_free",250 if start else 50),
                       ("memory_available",16 if start else 8)):
        if sample[key] < floor*GiB:
            return key
    return None


@contextmanager
def _owned_subreaper():
    import ctypes
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    if libc.prctl(37, ctypes.byref(previous), 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), "get process child-subreaper state")
    if libc.prctl(36, 1, 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), "set process child-subreaper state")
    try:
        yield
    finally:
        if libc.prctl(36, previous.value, 0, 0, 0) != 0:
            raise OSError(ctypes.get_errno(), "restore process child-subreaper state")


def _cleanup_owned_group(child):
    import os
    import signal
    import time
    result = {"pgid": child.pid, "term_sent": False, "kill_sent": False, "reaped": []}

    def reap():
        child.poll()
        while True:
            try:
                pid, status = os.waitpid(-child.pid, os.WNOHANG)
            except ChildProcessError:
                break
            if not pid:
                break
            if pid == child.pid:
                child.returncode = os.waitstatus_to_exitcode(status)
            result["reaped"].append(pid)

    def alive():
        reap()
        try:
            os.killpg(child.pid, 0)
            return True
        except ProcessLookupError:
            return False

    def send(sig):
        try:
            os.killpg(child.pid, sig)
            return True
        except ProcessLookupError:
            return False

    if alive():
        result["term_sent"] = send(signal.SIGTERM)
        deadline = time.monotonic() + 15
        while alive() and time.monotonic() < deadline:
            time.sleep(0.02)
        if alive():
            result["kill_sent"] = send(signal.SIGKILL)
            deadline = time.monotonic() + 5
            while alive() and time.monotonic() < deadline:
                time.sleep(0.02)
    result["complete"] = not alive()
    return result


def run_guarded(command, phase_dir, roots=None, baseline=None, max_owned=200*GiB,
                interval=30, phase_name="phase"):
    """Run one owned child, preserving output/exit and enforcing accepted resources."""
    import json
    from pathlib import Path
    import subprocess
    import time
    task = Path(phase_dir).resolve()
    for name in ("logs","receipts"):
        (task/name).mkdir(parents=True,exist_ok=True)
    if roots is None or baseline is None:
        admission = json.loads((task/"receipts/gd-source-entry-aggregate.json").read_text())
        if roots is None:
            roots = [row["path"] for row in admission["roots"]]
        if baseline is None:
            baseline = admission["aggregate_bytes"]
    def git(*args):
        return subprocess.check_output(["git",*args],text=True).strip()
    start = resource_sample(roots,task,baseline)
    issue = resource_issue(start,max_owned,start=True)
    record = {"phase":phase_name,"command":command,"cwd":str(Path.cwd()),
              "sha_start":git("rev-parse","HEAD"),"delta_start":git("status","--porcelain"),
              "started":time.time(),"samples":[start],"resource_stop":issue,
              "baseline_owned":baseline,"setup_cap":96*GiB,"raw_cap_each":16*GiB,
              "owned_cap":max_owned,"pid":None}
    code = 125
    if issue is None:
        with _owned_subreaper(), (task/"logs"/(phase_name+".log")).open("wb") as log:
            child = subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            record["pid"] = child.pid
            last, streak = start, 0
            try:
                while True:
                    try:
                        code = child.wait(timeout=interval)
                        break
                    except subprocess.TimeoutExpired:
                        try:
                            sample = resource_sample(roots,task,baseline)
                            record["samples"].append(sample)
                            rate = (sample["swap_bytes"]-last["swap_bytes"])/max(sample["time"]-last["time"],0.001)
                            streak = streak+1 if rate>=64*1024**2 else 0
                            last = sample
                            issue = resource_issue(sample,max_owned) or ("swap_rate" if streak>=3 else None)
                        except Exception as error:
                            issue = str(error)
                        if issue:
                            record["resource_stop"] = issue
                            break
            finally:
                record["cleanup"] = _cleanup_owned_group(child)
                if not record["cleanup"]["complete"]:
                    record["resource_stop"] = record["resource_stop"] or "owned_group_cleanup_incomplete"
                record["child_exit"] = child.returncode
        if record["resource_stop"]:
            code = 124
    try:
        final = resource_sample(roots,task,baseline)
        record["samples"].append(final)
        final_issue = resource_issue(final,max_owned)
        if record["pid"] is not None:
            rate = (final["swap_bytes"]-last["swap_bytes"])/max(final["time"]-last["time"],0.001)
            streak = streak+1 if rate>=64*1024**2 else 0
            final_issue = final_issue or ("swap_rate" if streak>=3 else None)
        record["resource_stop"] = record["resource_stop"] or final_issue
    except Exception as error:
        record["final_sample_error"] = str(error)
        record["resource_stop"] = record["resource_stop"] or "final_observation_unavailable"
    if record["pid"] is not None and record["resource_stop"]:
        code = 124
    record.update(exit=code if code>=0 else 128-code,ended=time.time(),
                  sha_end=git("rev-parse","HEAD"),delta_end=git("status","--porcelain"))
    (task/"receipts"/(phase_name+".json")).write_text(json.dumps(record,indent=2)+"\n")
    return record


def main():
    """Run a named owned qualification/setup phase under fixed reviewed limits."""
    import json
    import os
    import sys
    result = run_guarded(sys.argv[2:],os.environ["S30_GD_TASK"],phase_name=sys.argv[1])
    print(json.dumps({key:result[key] for key in ("phase","exit","pid","resource_stop")}),flush=True)
    if result["exit"]:
        from pathlib import Path
        log = Path(os.environ["S30_GD_TASK"])/"logs"/(sys.argv[1]+".log")
        if log.exists():
            print(log.read_text(errors="replace")[-18000:])
    raise SystemExit(result["exit"])


if __name__ == "__main__":
    main()
