"""Bind internal production cell execution to its live owned controller admission."""

import json
import os
from pathlib import Path
import time
import uuid


def controller_admission(allocation, root, cell, deadline, manifest, mode):
    """Record this controller identity and one fixed cell before the worker can exec."""
    fields = Path("/proc/self/stat").read_text().rsplit(") ", 1)[1].split()
    return dict(nonce=uuid.uuid4().hex, controller_pid=os.getpid(), controller_start=int(fields[19]),
                allocation=str(Path(allocation).resolve()), results=str(Path(root).resolve()),
                cell=cell, deadline_ns=deadline, manifest=str(Path(manifest).resolve()), mode=mode)


def require_cell_admission(manifest_path, deadline, config_path, output):
    """Refuse bare/stale cells before copying or invoking a measured program."""
    from g_campaign import calibration_plan, make_g_plan

    if os.environ.get("AGENT_LONG") != "1":
        raise RuntimeError("production cell requires AGENT_LONG=1 and active controller admission")
    admission, inflight = _active_admission("S30_G_CELL_ADMISSION")
    allocation = Path(admission["allocation"])
    manifest_path, config_path, output = map(lambda value: Path(value).resolve(),
                                             (manifest_path, config_path, output))
    cell = json.loads(config_path.read_text())
    manifest = json.loads(manifest_path.read_text())
    mode = manifest.get("stage")
    plan = calibration_plan(manifest["spec"]) if mode == "closed-copy-calibration" else make_g_plan(manifest["spec"])
    canonical = next((row for row in plan if row["id"] == inflight.get("cell")), None)
    if (manifest.get("schema") != "slice30-g-campaign/v1" or cell != canonical
            or cell != admission["cell"] or str(manifest_path) != admission["manifest"]
            or str(Path(manifest["allocation"]).resolve()) != str(allocation.resolve())
            or deadline != admission["deadline_ns"] or time.monotonic_ns() >= deadline
            or mode != admission["mode"] or config_path != output / "cell.json"
            or output.parent != Path(admission["results"]) / cell["id"]
            or not output.name.startswith("attempt-")):
        raise RuntimeError("cell configuration/path/deadline is outside its fixed controller admission")
    if ((mode == "closed-copy-read-r1" and cell["id"] != "g-r-n100000-r1")
            or (mode == "closed-copy-write-matrix" and cell["stage"] != "G-W")
            or (mode == "branch-controls" and cell["id"] != "g-o-branches")
            or (mode == "first-100k-seed-exploratory" and cell["id"] != "g1-n100000")
            or cell["id"] in manifest.get("reused", {})):
        raise RuntimeError("cell is outside the admitted closed stage or is already qualified")


def _active_admission(variable):
    from g_boundary import require_known_containment
    require_known_containment(os.environ["S30_TASK"])
    credential = json.loads(os.environ.get(variable, "null"))
    if not isinstance(credential, dict):
        raise RuntimeError("production cell requires active controller admission")
    allocation = Path(credential["allocation"])
    budget = json.loads((allocation / "g-budget.json").read_text())
    inflight = budget.get("inflight") or {}
    admission = inflight.get("admission")
    if admission != credential or inflight.get("status", "running") != "running":
        raise RuntimeError("cell admission is absent, stale or cleanup is unknown")
    parent = os.getppid()
    fields = Path(f"/proc/{parent}/stat").read_text().rsplit(") ", 1)[1].split()
    if (parent != admission["controller_pid"] or int(fields[19]) != admission["controller_start"]
            or fields[0] in ("Z", "X")
            or Path(f"/proc/{parent}/cgroup").read_text() != Path("/proc/self/cgroup").read_text()):
        raise RuntimeError("cell controller identity/boundary does not match admission")
    return admission, inflight


def require_common_admission(job, config, output):
    """Refuse bare metadata workers before any freeze/preflight/report output or read."""
    admission, inflight = _active_admission("S30_G_COMMON_ADMISSION")
    job, config, output = (Path(value).resolve() for value in (job, config, output))
    cell = json.loads(config.read_text())
    mode = admission["mode"]
    expected = {"id": "g-common-"+mode}
    if (mode not in ("closed-copy-calibration", "closed-copy-read-r1", "closed-copy-write-matrix")
            or cell != expected or admission["cell"] != expected or inflight.get("cell") != expected["id"]
            or str(job) != admission["manifest"] or time.monotonic_ns() >= admission["deadline_ns"]
            or not job.is_relative_to(Path(admission["allocation"]) / "common/jobs")
            or config != output / "cell.json" or output.parent != Path(admission["results"]) / expected["id"]
            or not output.name.startswith("attempt-")):
        raise RuntimeError("common worker is outside its live fixed admission")
