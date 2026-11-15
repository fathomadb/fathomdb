"""Comparable case envelopes and measured serial closure, never qualification authority."""

import json
from pathlib import Path
import stat

SCHEMA = "slice30-g-serial-retained-live/v1"
IDS = ("g-r-n100000-r0", "g-r-n100000-r1", "g-cal-engine-b10-s1", "g-cal-sdk-b10-s1",
       "g-cal-engine-b1-s100", "g-cal-sdk-b1-s100")
GiB = 1024**3


def _integer(value, minimum=0):
    if type(value) is not int or value < minimum:
        raise ValueError("serial storage observation is unavailable")
    return value


def _coefficients(records):
    if [row.get("id") for row in records] != list(IDS):
        raise ValueError("serial storage needs both reads and all four calibration cases")
    raw, residual, baselines, closed, live = [], [], set(), [], []
    for row in records:
        count = _integer(row.get("record_count"), 1)
        raw.append((_integer(row.get("serialized_bytes"))+count-1)//count)
        residual.append(_integer(row.get("residual_bytes")))
        baseline = _integer(row.get("baseline_bytes"))
        if baseline:
            baselines.add(baseline)
        replacements = _integer(row.get("replacement_rows"))
        if replacements:
            closed.append((_integer(row.get("closed_growth_bytes"))+replacements-1)//replacements)
            live.append((_integer(row.get("live_growth_bytes"))+replacements-1)//replacements)
    if len(baselines) != 1 or len(closed) != 4 or len(live) != 4:
        raise ValueError("serial storage baseline/growth observations are unavailable")
    return {"baseline_bytes": baselines.pop(), "raw_bytes_per_record": max(raw),
            "residual_bytes": max(residual), "closed_growth_per_row": max(closed),
            "live_growth_per_row": max(live)}


def validate_model(model, previous=None, calibration_origin=None):
    """Require the named measured model and its exact existing scientific origins."""
    if not isinstance(model, dict) or model.get("schema") != SCHEMA or model.get("factor") != 2:
        raise ValueError("unknown serial storage model")
    basis = model.get("basis") or {}
    if (not isinstance(basis.get("compiler_inputs"), str) or not basis["compiler_inputs"]
            or any(not isinstance(basis.get(key), dict) or not basis[key]
                   for key in ("r0", "r1", "calibration"))):
        raise ValueError("serial storage model lacks its measured origins")
    for key, value in _coefficients(basis.get("records", [])).items():
        if type(model.get(key)) is not int or model[key] != value:
            raise ValueError("serial storage coefficient differs from its observations: " + key)
    if model["live_growth_per_row"] < model["closed_growth_per_row"]:
        raise ValueError("live and closed storage are not comparable")
    if previous is not None and any(basis[key] != (previous.get(identifier) or {}).get("origin")
            for key, identifier in (("r0", IDS[0]), ("r1", IDS[1]))):
        raise ValueError("serial storage read origins differ")
    if calibration_origin is not None and basis["calibration"] != calibration_origin:
        raise ValueError("serial storage calibration origin differs")
    return model


def envelopes(cell, model):
    """Return total closed/live case bytes with all original conservative factors."""
    validate_model(model)
    cohorts = (cell["samples"]+cell["searches_per_batch"]-1)//cell["searches_per_batch"]
    rows = cohorts*cell["batch_size"]
    records = cell["warmup"]+cell["samples"]+2*rows+cohorts
    common = model["baseline_bytes"]+2*model["raw_bytes_per_record"]*records+2*model["residual_bytes"]
    closed = common+2*model["closed_growth_per_row"]*rows
    live = common+2*model["live_growth_per_row"]*rows
    return {"schema": SCHEMA, "closed_bytes": closed, "live_bytes": live,
            "live_excess_bytes": live-closed}


def _files(directory):
    files = {}
    for path in Path(directory).rglob("*"):
        observed = path.stat()
        if stat.S_ISREG(observed.st_mode):
            files[path.relative_to(directory).as_posix()] = observed.st_size
    return files


def _byte_tuple(files):
    main, wal, shm = (files.get("subject/store.sqlite"+suffix, 0) for suffix in ("", "-wal", "-shm"))
    total = sum(files.values())
    return {"main": main, "wal": wal, "shm": shm, "output": total-main-wal-shm, "total": total}


def _record_evidence(row, actual):
    attempt = Path(row["attempt"])
    files = _files(attempt)
    kind = "read" if row["id"] in IDS[:2] else "write"
    streams = [name for name in files if name.startswith(kind+"/") and name.endswith(".jsonl")]
    count = 0
    for name in streams:
        with (attempt/name).open("rb") as stream:
            for block in iter(lambda: stream.read(1024*1024), b""):
                count += block.count(b"\n")
    serialized = sum(files[name] for name in streams)
    serialized += sum(files[name] for name in (kind+"-native.log", kind+"/result.json", "result.json"))
    observed = _byte_tuple(files)
    baseline = actual.get("preparation", {}).get("copied_bytes", 0)
    cell = row["cell"]
    replacements = ((cell["samples"]+cell["searches_per_batch"]-1)//cell["searches_per_batch"]
                    *cell["batch_size"]) if kind == "write" else 0
    return {"id": row["id"], "serialized_bytes": serialized, "record_count": count,
            "residual_bytes": observed["output"]-serialized, "baseline_bytes": baseline,
            "closed_growth_bytes": observed["main"]+observed["wal"]+observed["shm"]-baseline,
            "replacement_rows": replacements,
            "live_growth_bytes": row.get("peak_store_bytes", baseline)-baseline}


def derive_model(manifest, calibration, references):
    """Recompute all six physical serialization/growth observations without asset hashes."""
    records = []
    for identifier in IDS[:2]:
        row = references[identifier]
        actual = json.loads(Path(row["attempt"], "result.json").read_text())
        records.append(_record_evidence(row, actual))
    records.extend(_record_evidence(row, row["actual"]) for row in calibration["records"])
    model = {"schema": SCHEMA, "factor": 2, **_coefficients(records), "basis": {
        "compiler_inputs": manifest["compiler_inputs"], "r0": references[IDS[0]]["origin"],
        "r1": references[IDS[1]]["origin"], "calibration": calibration["origin"], "records": records}}
    return validate_model(model, references, calibration["origin"])


def capture_closure(outcome):
    """Measure final case files only after the existing owned-writer termination proof."""
    if (outcome.get("cleanup_uncertain") is not False or outcome.get("live_owned_pids") != []
            or outcome.get("child_exit") is None and outcome.get("pid") is not None):
        raise ValueError("serial case writers are not known terminated")
    return {"schema": SCHEMA, "attempt": outcome["attempt"], "quiescent": True,
            "bytes": _byte_tuple(_files(Path(outcome["attempt"])))}


def closed_case(outcome, witness):
    """Recognize only an observed disposition for that exact terminal attempt."""
    if not isinstance(witness, dict) or witness.get("schema") != SCHEMA:
        return False
    sizes = witness.get("bytes") or {}
    return (witness.get("attempt") == outcome.get("attempt") and witness.get("quiescent") is True
            and outcome.get("cleanup_uncertain") is False and outcome.get("live_owned_pids") == []
            and outcome.get("child_exit") is not None
            and all(type(sizes.get(key)) is int and sizes[key] >= 0
                    for key in ("main", "wal", "shm", "output", "total"))
            and sizes["total"] == sum(sizes[key] for key in ("main", "wal", "shm", "output")))


def frontier(spec, plan, previous, budget, retained):
    """Recheck every unstarted required write using fresh whole-allocation retention."""
    review = spec["_reviewed_costs"]
    model = validate_model(review["storage_model"])
    if budget.get("serial_storage_blocked"):
        return {"allowed": False, "reason": "serial_closure_unavailable"}
    remaining = []
    closures = budget.get("serial_closed_cases", {})
    for cell in plan:
        if cell["stage"] != "G-W":
            continue
        old = previous.get(cell["id"]) or {}
        if old.get("attempt") and old.get("pid") is not None:
            if not closed_case(old, closures.get(cell["id"])):
                return {"allowed": False, "reason": "serial_closure_unavailable"}
            continue
        if cell["id"] in closures and not old.get("attempt"):
            return {"allowed": False, "reason": "serial_case_already_attempted"}
        prediction = review["g_table"][cell["id"]]["prediction"]
        storage = envelopes(cell, model)
        if prediction.get("storage") != storage or prediction.get("new_bytes") != storage["live_bytes"]:
            return {"allowed": False, "reason": "serial_table_unavailable"}
        time_ns = prediction.get("total_ns")
        prior = budget.get("cell_used_ns", {}).get(cell["id"], 0)
        if type(time_ns) is not int or time_ns < 0 or time_ns+40*10**9 > cell["cell_seconds"]*10**9-prior:
            return {"allowed": False, "reason": "serial_cell_time_forecast"}
        remaining.append((storage, time_ns))
    common_ns = max(0, 600*10**9-budget.get("common_used_ns", {}).get("closed-copy-write-matrix", 0))
    if sum(time_ns for _, time_ns in remaining)+common_ns > review["limits"]["campaign_seconds"]*10**9-budget["used_ns"]:
        return {"allowed": False, "reason": "serial_shared_time_forecast"}
    peak = (retained+GiB+sum(s["closed_bytes"] for s, _ in remaining)
            +max((s["live_excess_bytes"] for s, _ in remaining), default=0))
    if peak > review["limits"]["active_bytes"]:
        return {"allowed": False, "reason": "serial_retained_storage_forecast", "peak_bytes": peak}
    roots = spec.get("_serial_resource_roots")
    if roots:
        from g_resource import resource_sample
        sampled = resource_sample(Path(roots["source"]), Path(roots["task"]))
        growth = max(0, peak-retained)
        root_growth = growth if Path(roots["task"]).stat().st_dev == Path("/").stat().st_dev else 0
        if (sampled["owned"]+growth >= 200*GiB or sampled["task_free"]-growth < 250*GiB
                or sampled["root_free"]-root_growth < 32*GiB or sampled["memory_available"] < 16*GiB):
            return {"allowed": False, "reason": "serial_resource_headroom"}
    return {"allowed": True, "reason": None, "peak_bytes": peak, "remaining_rows": len(remaining)}
