"""Canonical M controller/preparation/payload bindings under the owned outer guardian."""

import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import threading
import time
import uuid

from g_boundary import require_known_containment
from m_cell import execute
from m_controller import load_plan, run_budgeted_table
from m_freeze import validate
from m_gpu import qualification
from m_guard import process_start, write_budget
from m_record import CheckpointLog


def controller_admission(fd, root):
    """Accept only the active owning guardian's inherited bounded anonymous admission."""
    ready = select.select([fd], [], [], 2)[0]
    if not ready:
        raise RuntimeError("owning M guardian admission is unavailable")
    try:
        raw = os.read(fd, 4097)
    finally:
        os.close(fd)
    if len(raw) > 4096:
        raise ValueError("unbounded M guardian admission")
    owner = json.loads(raw)
    if (owner["pid"] != os.getppid() or owner["start"] != process_start(os.getppid())
            or owner["controller_pid"] != os.getpid()
            or owner["controller_start"] != process_start(os.getpid())
            or Path(owner["root"]).resolve() != Path(root).resolve()):
        raise RuntimeError("controller is outside its actual owning M guardian")
    require_known_containment(os.environ["S30_TASK"])
    return owner


def worker_admission(manifest, cell, attempt):
    """Require the live canonical controller and its actual active registered attempt."""
    require_known_containment(manifest["task"])
    value = json.loads((Path(manifest["allocation"]) / "g-budget.json").read_text())
    owner = value.get("m_guardian") or {}
    active = value.get("inflight") or {}
    if (owner.get("controller_pid") != os.getppid()
            or owner.get("controller_start") != process_start(os.getppid())
            or owner.get("root") != manifest["root"]
            or active.get("cell") != cell["id"] or active.get("status") is not None
            or active.get("attempt") != str(Path(attempt).resolve())
            or active.get("freeze_uuid") != manifest["uuid"]
            or cell not in manifest["plan"]):
        raise RuntimeError("private M cell lacks canonical active controller admission")
    return owner


def prepare(manifest, cell, attempt, deadline_ns):
    """Create fresh retained closed packages/config; no checkpoint, reset or writable source."""
    worker_admission(manifest, cell, attempt)
    attempt = Path(attempt)
    began = time.monotonic_ns()
    stores = attempt / "stores"
    stores.mkdir()
    files, copies = {}, []
    if cell["family"] in ("M0", "M-E"):
        helper = manifest["builds"]["helper"]["binary"]["path"]
        # The fixture helper owns creation and refuses an existing root.
        fixture = stores / "tiny"
        seeded = subprocess.run([helper, "--seed-fixture", str(fixture)], capture_output=True,
                                timeout=min(15, max(.001, (deadline_ns-time.monotonic_ns())/10**9)))
        (attempt / "seed.stdout").write_bytes(seeded.stdout)
        (attempt / "seed.stderr").write_bytes(seeded.stderr)
        if seeded.returncode:
            raise RuntimeError("tiny SDK fixture failed")
        files = {name: str(fixture / f"{name}.sqlite") for name in ("a", "b")}
    else:
        main = manifest["closed_subject"]["main"]
        for name in ("a", "b")[:cell["copies"]]:
            destination = stores / f"{name}.sqlite"
            files[name] = str(destination)
            for identity in manifest["package"]:
                suffix = identity["path"][len(main):]
                selected = Path(str(destination) + suffix)
                total = 0
                digest = hashlib.sha256()
                with Path(identity["path"]).open("rb") as source, selected.open("xb") as target:
                    while block := source.read(1024 * 1024):
                        if time.monotonic_ns() >= deadline_ns:
                            raise TimeoutError("closed package copy reached shared work deadline")
                        target.write(block)
                        digest.update(block)
                        total += len(block)
                if total != identity["bytes"] or digest.hexdigest() != identity["sha256"]:
                    raise ValueError("closed package copy incomplete")
                copies.append({"source": identity, "destination": str(selected), "bytes": total})
    config = dict(cell["config"], stores={name: {"path": path,
                  "eager": cell.get("case") == "eager" and name == "a"}
                  for name, path in files.items()}, default_store="a")
    (attempt / "mcp.json").write_text(json.dumps(config, indent=2) + "\n")
    (attempt / "prepared.json").write_text(json.dumps({"files": files, "copies": copies,
        "preparation_ns": time.monotonic_ns() - began}, indent=2) + "\n")


def payload(manifest, cell, attempt, deadline_ns):
    """Execute one admitted actual MCP cell; GPU observers run only for hybrid natural cells."""
    worker_admission(manifest, cell, attempt)
    attempt = Path(attempt)
    prepared = json.loads((attempt / "prepared.json").read_text())
    natural = cell["family"] in ("M-R", "M-C", "M-L")
    variant = "ordinary" if natural else "helper" if cell["family"] == "M-E" else "providerfree"
    command = [manifest["builds"][variant]["binary"]["path"], "--config", str(attempt / "mcp.json")]
    socket = None
    if variant == "helper":
        socket = Path(manifest["task"]) / "tmp" / ("m-" + uuid.uuid4().hex[:12] + ".sock")
        command += ["--control-socket", str(socket)]
    os.environ["FATHOMDB_MCP_LOG"] = cell["log"]
    if natural:
        os.environ.update(HF_HOME=manifest["hf_home"], HF_HUB_OFFLINE="1",
                          FATHOMDB_EMBED_DEVICE=f"cuda:{manifest['gpu']['ordinal']}")
    queries = Path(manifest["inputs"]["queries"]["path"]).read_text().splitlines() if natural else ["alpha"]
    samples, failures, stop, thread = [], [], threading.Event(), None
    stream = None
    observer = None
    if natural and cell["search_mode"] == "hybrid":
        from g_gpu import GpuObserver
        observer = GpuObserver(manifest["gpu"])
        stream = CheckpointLog(attempt / "gpu.jsonl")

    def started(pid):
        nonlocal thread
        if observer:
            def sample():
                while not stop.is_set():
                    try:
                        row = observer.sample(pid)
                        samples.append(row)
                        stream.write(row)
                    except Exception as error:
                        failures.append(f"{type(error).__name__}: {error}")
                        return
                    stop.wait(1)
            thread = threading.Thread(target=sample, daemon=True)
            thread.start()

    try:
        result = execute(cell, command, attempt / "wire", queries=queries,
                         deadline_ns=deadline_ns, files=prepared["files"], control_path=socket,
                         on_spawn=started)
    finally:
        stop.set()
        if thread:
            thread.join(timeout=6)
            if thread.is_alive():
                failures.append("GPU polling did not stop within its bound")
        if stream and (not thread or not thread.is_alive()):
            stream.close()
    result["gpu"] = qualification(cell, samples, result["pid"])
    result["gpu"]["failures"] = failures
    result["gpu"]["samples"] = len(samples)
    result["complete"] &= result["gpu"]["complete"] and not failures
    result.update(scope="registered-M-cell", cell=cell, freeze_uuid=manifest["uuid"],
                  selected_build=manifest["builds"][variant], prepared=prepared)
    (attempt / "result.json").write_text(json.dumps(result, indent=2, allow_nan=False) + "\n")
    return result


def control(root, owner, *, source_manifest=None):
    """Validate the exact freeze, then dispatch all46 cells with whole shared accounting."""
    manifest = validate(root) if source_manifest is None else source_manifest
    if source_manifest is not None and (manifest.get("scope") != "tiny-provider-free-source-control"
            or manifest["plan"] != load_plan()[:2]):
        raise ValueError("source controller permits only the exact two provider-free M0 cells")
    budget_path = Path(manifest["allocation"]) / "g-budget.json"
    if json.loads(budget_path.read_text()).get("m_guardian") != owner:
        raise RuntimeError("current shared guardian admission differs")
    runner = str(Path(__file__).with_name("m_runner.py"))

    def register(cell, attempt):
        value = json.loads(budget_path.read_text())
        value["inflight"].update(attempt=str(Path(attempt).resolve()), freeze_uuid=manifest["uuid"])
        write_budget(budget_path, value)

    def command(cell, attempt, deadline):
        return [sys.executable, runner, "_cell", "--root", str(root), "--deadline-ns", str(deadline)]

    def admit_preparation(cell, attempt, deadline):
        prepare(manifest, cell, attempt, deadline)

    rows = run_budgeted_table(Path(root) / "results", command, profile=manifest["profile"],
        allocation=manifest["allocation"], forecasts=manifest["forecasts"]["cells"], plan=manifest["plan"],
        prepare=admit_preparation, register=register,
        common_reserved_ns=2 * sum(manifest["forecasts"]["common_components_ns"].values()))
    (Path(root) / "results").mkdir(exist_ok=True)
    (Path(root) / "results/campaign-state.json").write_text(json.dumps(rows, indent=2) + "\n")
    return rows
