"""Selected3090 read-only observations; never driver changes or substitute GPU work."""

import csv
import io
from pathlib import Path
import subprocess
import time


def inspect_gpu(uuid, pid=None, *, deadline_ns=None):
    """Read selected metadata/process memory within an optional absolute deadline.

    Without a deadline each existing command keeps its 5s timeout. A supplied
    deadline covers both commands and rejects results completed at/after it.
    """
    base = ["nvidia-smi", "--id", uuid]
    def remaining():
        if deadline_ns is None:
            return 5
        value = (deadline_ns-time.monotonic_ns())/10**9
        if value <= 0:
            raise TimeoutError("selected GPU observation deadline")
        return min(5,value)
    def query(fields, kind):
        completed = subprocess.run([*base, f"--query-{kind}={fields}", "--format=csv,noheader,nounits"],
                                   text=True, capture_output=True, timeout=remaining(), check=True)
        remaining()
        return [[value.strip() for value in row] for row in csv.reader(io.StringIO(completed.stdout)) if row]
    rows = query("uuid,name,driver_version,memory.used,utilization.gpu", "gpu")
    if len(rows) != 1 or len(rows[0]) != 5:
        raise RuntimeError("selected GPU metadata unavailable")
    found, name, driver, memory, utilization = rows[0]
    primary = str(Path("/usr/local/cuda").resolve())
    if found != uuid or "3090" not in name or "K620" in name or driver != "580.178.04" or primary != "/usr/local/cuda-12.6":
        raise RuntimeError("selected3090/R580/primaryCUDA12.6 identity mismatch")
    applications = []
    for row in query("pid,gpu_uuid,used_gpu_memory", "compute-apps"):
        if len(row) != 3:
            raise RuntimeError("selected GPU compute-process observation unavailable")
        if row[1] != uuid:
            continue
        applications.append({"pid": int(row[0]), "uuid": row[1], "memory_mib": int(row[2])})
    remaining()
    return {"uuid": found, "name": name, "driver_version": driver, "primary_cuda": primary,
            "pid": pid, "gpu_memory_mib": int(memory), "utilization_percent": int(utilization),
            "pid_memory_mib": sum(row["memory_mib"] for row in applications if row["pid"] == pid),
            "applications": applications, "observed_at_ns": time.time_ns(), "monotonic_ns": time.monotonic_ns()}


class GpuObserver:
    """Observe actual child engagement; the caller owns cadence and deadlines."""

    def __init__(self, gpu, *, deadline_ns=None):
        self.gpu = gpu
        self.baseline = (inspect_gpu(gpu["uuid"]) if deadline_ns is None
                         else inspect_gpu(gpu["uuid"],deadline_ns=deadline_ns))
        if self.baseline["applications"]:
            failure = RuntimeError("selected GPU already has compute work; reservation unavailable")
            failure.observation = self.baseline
            raise failure

    def sample(self, pid, *, deadline_ns=None):
        """Retain observed app facts; foreign work or a missed deadline refuses."""
        record = (inspect_gpu(self.gpu["uuid"],pid) if deadline_ns is None
                  else inspect_gpu(self.gpu["uuid"],pid,deadline_ns=deadline_ns))
        record["baseline_gpu_memory_mib"] = self.baseline["gpu_memory_mib"]
        if any(row["pid"] != pid for row in record["applications"]):
            failure = RuntimeError("selected GPU has foreign compute work")
            failure.observation = record
            raise failure
        return record
