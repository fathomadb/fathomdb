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
from m_record import CheckpointLog, CONTROL_MAX_BYTES, OutputLimitError, write_bytes, write_json


def read_queries(path):
    """Decode UTF8 JSON string lines in the same original order as the G reader."""
    with Path(path).open("rb") as stream:
        queries = [json.loads(line.decode("utf-8")) for line in stream]
    if any(not isinstance(value, str) for value in queries):
        raise ValueError("registered query records must be JSON strings")
    return queries


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
        write_bytes(attempt / "seed.stdout", seeded.stdout)
        write_bytes(attempt / "seed.stderr", seeded.stderr)
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
    write_json(attempt / "mcp.json", config, limit=CONTROL_MAX_BYTES)
    write_json(attempt / "prepared.json", {"files": files, "copies": copies,
        "preparation_ns": time.monotonic_ns() - began}, limit=CONTROL_MAX_BYTES)


def _retain_result(attempt, result):
    try:
        write_json(attempt / "result.json", result)
    except OutputLimitError as error:
        result = {key: result.get(key) for key in ("pid", "cell", "scope", "freeze_uuid", "selected_build")}
        result.update(complete=False, output_overflow=str(error))
        write_json(attempt / "result.json", result, limit=CONTROL_MAX_BYTES)
    return result


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
    queries = read_queries(manifest["inputs"]["queries"]["path"]) if natural else ["alpha"]
    samples, failures, failure_observations = [], [], []
    stop, thread = threading.Event(), None
    lifecycle = cell["family"] == "M-L" and cell["search_mode"] == "hybrid"
    observed = threading.Event() if lifecycle else None
    tail_end = None
    pre_close = None
    stream = None
    observer = None
    if natural and cell["search_mode"] == "hybrid":
        from g_gpu import GpuObserver
        stream = CheckpointLog(attempt / "gpu.jsonl", max_bytes=CONTROL_MAX_BYTES)
        try:
            if lifecycle:
                probe_end = min(time.monotonic_ns()+500000000,deadline_ns)
                observer = GpuObserver(manifest["gpu"],deadline_ns=probe_end)
                if time.monotonic_ns() >= probe_end:
                    raise TimeoutError("selected GPU observation deadline")
            else:
                observer = GpuObserver(manifest["gpu"])
        except Exception as error:
            failures.append(f"{type(error).__name__}: {error}")
            observation = getattr(error, "observation", None)
            try:
                if observation is not None:
                    failure_observations.append(observation)
                    stream.write(observation)
            except OutputLimitError as persistence_error:
                failures.append(str(persistence_error))
            finally:
                stream.close()
            return _retain_result(attempt, {"complete": False, "pid": None, "cell": cell,
                "freeze_uuid": manifest["uuid"], "selected_build": manifest["builds"][variant],
                "scope": manifest.get("scope", "registered-M-cell"), "prepared": prepared,
                "gpu": dict(qualification(cell, [], None), failures=failures,
                            failure_observations=failure_observations, samples=len(failure_observations))})

    def started(pid):
        nonlocal thread
        if observer:
            def sample():
                assert observer is not None and stream is not None
                while not stop.is_set():
                    try:
                        if lifecycle:
                            probe_end = min(time.monotonic_ns()+500000000,deadline_ns,
                                            deadline_ns if tail_end is None else tail_end)
                            row = observer.sample(pid,deadline_ns=probe_end)
                            if time.monotonic_ns() >= probe_end:
                                raise TimeoutError("selected GPU observation deadline")
                        else:
                            row = observer.sample(pid)
                        samples.append(row)
                        stream.write(row)
                        if observed is not None:
                            observed.set()
                    except Exception as error:
                        failures.append(f"{type(error).__name__}: {error}")
                        observation = getattr(error, "observation", None)
                        if observation is not None:
                            failure_observations.append(observation)
                            try:
                                stream.write(observation)
                            except Exception as persistence_error:
                                failures.append(f"{type(persistence_error).__name__}: {persistence_error}")
                        if observed is not None:
                            observed.set()
                        return
                    stop.wait(.1 if lifecycle else 1)
            thread = threading.Thread(target=sample, daemon=True)
            thread.start()

    def payload_complete(pid, complete):
        nonlocal tail_end, pre_close
        assert observed is not None
        began = time.monotonic_ns()
        # The supplied work deadline already excludes the external 7s reserve.
        tail_end = min(began+2*10**9,deadline_ns)
        disposition = "payload_incomplete"
        while complete:
            now = time.monotonic_ns()
            if failures:
                disposition = "observer_error"
                break
            if now >= tail_end:
                disposition = "deadline" if tail_end == deadline_ns else "no_witness"
                break
            if qualification(cell,tuple(samples),pid)["complete"]:
                disposition = "witness"
                break
            observed.wait(min(.1,(tail_end-now)/10**9))
            observed.clear()
        ended = time.monotonic_ns()
        pre_close = tuple(samples)
        valid = (complete and disposition == "witness" and ended < tail_end
                 and not failures and qualification(cell,pre_close,pid)["complete"])
        return {"pid":pid,"payload_end_ns":began,"tail_started_ns":began,
                "tail_ended_ns":ended,"tail_ns":ended-began,"tail_deadline_ns":tail_end,
                "tail_limit_ns":2*10**9,"sample_interval_ns":100000000,"probe_budget_ns":500000000,
                "qualified_before_close":valid,"disposition":disposition,
                "qualification_samples":len(pre_close)}

    try:
        options = {"on_payload_complete":payload_complete} if lifecycle else {}
        result = execute(cell, command, attempt / "wire", queries=queries,
                         deadline_ns=deadline_ns, files=prepared["files"], control_path=socket,
                         on_spawn=started,**options)
    finally:
        stop.set()
        if thread:
            thread.join(timeout=6)
            if thread.is_alive():
                failures.append("GPU polling did not stop within its bound")
        if stream and (not thread or not thread.is_alive()):
            stream.close()
    result["gpu"] = qualification(cell, samples if not lifecycle else pre_close or (), result["pid"])
    if lifecycle:
        result["gpu"]["qualification_scope"] = "before_EOF"
        result["gpu"]["complete"] &= bool(result.get("gpu_observation",{}).get("qualified_before_close"))
    result["gpu"]["failures"] = failures
    result["gpu"]["failure_observations"] = failure_observations
    result["gpu"]["samples"] = len(samples) + len(failure_observations)
    result["complete"] &= result["gpu"]["complete"] and not failures
    result.update(scope=manifest.get("scope", "registered-M-cell"), cell=cell, freeze_uuid=manifest["uuid"],
                  selected_build=manifest["builds"][variant], prepared=prepared)
    if manifest.get("scope") == "bounded-natural-calibration":
        result["qualifies_primary"] = False
    return _retain_result(attempt, result)


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
        common_reserved_bytes=2 * manifest["forecasts"]["common_new_retained_bytes"],
        common_reserved_ns=2 * sum(manifest["forecasts"]["common_components_ns"].values()))
    (Path(root) / "results").mkdir(exist_ok=True)
    (Path(root) / "results/campaign-state.json").write_text(json.dumps(rows, indent=2) + "\n")
    return rows
