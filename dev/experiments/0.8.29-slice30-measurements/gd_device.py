"""Bounded approved-device identity and memory accounting shared by G tools."""

import subprocess
import time

GPU = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b"


def _remaining(deadline):
    budget = deadline-time.monotonic()
    if budget<=0:
        raise RuntimeError("bounded text fixture deadline exceeded")
    return budget


def _nvml(arguments, deadline):
    child = subprocess.run(["nvidia-smi",*arguments,"--format=csv,noheader,nounits"],
        capture_output=True,text=True,timeout=min(5,_remaining(deadline)))
    if child.returncode:
        raise RuntimeError("required GPU accounting failed: "+child.stderr)
    return child.stdout


def _accounting(pid, deadline, require_pid=False):
    line = _nvml(["-i",GPU,"--query-gpu=uuid,name,driver_version,memory.total,memory.used,utilization.gpu"],deadline)
    values = [value.strip() for value in line.strip().split(",")]
    if len(values)!=6 or values[0]!=GPU or "3090" not in values[1] or not values[2].startswith("580."):
        raise RuntimeError("required approved GPU identity invalid")
    total,used,util = map(int,values[3:])
    if total<=0 or used<0 or used>total:
        raise RuntimeError("required GPU memory accounting invalid")
    own=0
    present=False
    others=[]
    for row in _nvml(["-i",GPU,"--query-compute-apps=pid,gpu_uuid,used_gpu_memory"],deadline).splitlines():
        if not row.strip():
            continue
        fields=[value.strip() for value in row.split(",")]
        if len(fields)!=3:
            raise RuntimeError("required process memory accounting malformed")
        owner=int(fields[0])
        memory=int(fields[2])
        if memory<0:
            raise RuntimeError("required process memory accounting invalid")
        if fields[1]==GPU:
            if owner==pid:
                if present:
                    raise RuntimeError("required own PID accounting duplicated")
                present=True
                own=memory
            else:
                others.append(owner)
    if others or require_pid and (not present or own<=0):
        raise RuntimeError("reserved own PID/UUID accounting unavailable or interfered")
    if own>8192:
        raise RuntimeError("owned GPU memory exceeds8GiB cap")
    return {"at":time.monotonic(),"uuid":GPU,"pid":pid,"own_MiB":own,
            "pid_present":present,"total_MiB":total,"used_MiB":used,"util":util}


