"""One explicitly selected bounded M calibration, separate from primary admission."""

import json
from pathlib import Path
import sys
import time

from g_campaign import active_bytes
from m_attempt import run_attempt
from m_load import observe_load, summarize_load
from m_record import CONTROL_MAX_BYTES, JSONL_MAX_BYTES, JSON_MAX_BYTES, write_json

CELL_ID = "M-R-1-1-warn"
CALIBRATION_ID = "M-CAL:" + CELL_ID


def control(root, owner, manifest=None):
    """Dispatch one admitted selected configuration through existing preparation/payload."""
    from m_freeze import validate_calibration
    from m_guard import write_budget
    from m_runtime import prepare
    manifest = validate_calibration(root) if manifest is None else manifest
    root = Path(root)
    ledger_path = Path(manifest["allocation"]) / "g-budget.json"
    budget = json.loads(ledger_path.read_text())
    if budget.get("m_guardian") != owner:
        raise RuntimeError("calibration lacks its current owning M guardian")
    primary = manifest.get("scope") == "registered-M-selected-cell"
    cell = registered_cell(manifest["plan"][0]["id"]) if primary else selected_cell(CELL_ID)
    identity = cell["id"]
    usage_key = "selected_cell_usage" if primary else "calibration_usage"
    if manifest["plan"] != [cell]:
        raise ValueError("only the selected calibration configuration can dispatch")
    limits = manifest["cell_allowance" if primary else "calibration"]
    usage = budget[usage_key][identity]
    prior = budget.get("cell_used_ns", {}).get(identity, 0)
    began = time.monotonic_ns()
    phase_end = owner["started_monotonic_ns"] + limits["phase_ns"] - usage["wall_ns"]
    deadline = min(began + limits["payload_ns"] - prior,
                   began + 900 * 10**9 - prior, phase_end - 20 * 10**9,
                   began + manifest["profile"]["total_shared_ns"]
                   - manifest["profile"]["reserved_other_work_ns"] - budget["used_ns"] - 20 * 10**9)
    baseline = active_bytes(manifest["allocation"])
    remaining_bytes = limits["new_retained_bytes"] - usage["new_retained_bytes"]
    bound = preparation_bound(manifest)

    load_observations = []

    def authorize(prepared):
        load_observations.append(observe_load())
        if time.monotonic_ns() + 7 * 10**9 >= deadline:
            return "remaining calibration work/cleanup allowance unavailable"
        if not prepared and bound > remaining_bytes:
            return "closed packaging plus bounded output exceeds remaining calibration growth"
        return monitor()

    def monitor():
        current = active_bytes(manifest["allocation"])
        if current - baseline >= remaining_bytes:
            return "calibration retained growth exhausted"
        if current >= (manifest["profile"]["total_shared_retained_bytes"]
                       - manifest["profile"].get("reserved_other_work_bytes", 0)):
            return "shared retained storage exhausted"
        return None

    def register(selected, attempt):
        value = json.loads(ledger_path.read_text())
        value["inflight"] = {"cell": selected["id"], "started_monotonic_ns": began,
            "started_at_ns": time.time_ns(), "attempt": str(Path(attempt).resolve()),
            "freeze_uuid": manifest["uuid"]}
        write_budget(ledger_path, value)

    def command(selected, attempt, work_deadline):
        return [sys.executable, str(Path(__file__).with_name("m_runner.py")), "_cell",
                "--root", str(root), "--deadline-ns", str(work_deadline)]

    outcome = run_attempt(cell, root / "results", command, deadline_ns=deadline,
        prepare=lambda selected, attempt, end: prepare(manifest, selected, attempt, end),
        authorize=authorize, monitor=monitor, register=register)
    elapsed = time.monotonic_ns() - began
    budget = json.loads(ledger_path.read_text())
    budget["used_ns"] += elapsed
    budget.setdefault("cell_used_ns", {})[identity] = prior + elapsed
    clean = not outcome["cleanup_uncertain"] and outcome["live_owned_pids"] == []
    if clean:
        budget["inflight"] = None
    else:
        budget["inflight"]["status"] = "cleanup_uncertain"
    write_budget(ledger_path, budget)
    load_observations.append(observe_load())
    outcome.update(charged_ns=elapsed, scope=manifest["scope"], qualifies_primary=primary,
                   load=summarize_load(load_observations))
    (root / "results").mkdir(exist_ok=True)
    write_json(root / "results/campaign-state.json", [outcome], limit=CONTROL_MAX_BYTES)
    return outcome


def preparation_bound(manifest):
    """Bound the selected closed package and existing bounded retained producers."""
    from m_runtime import read_queries
    queries = read_queries(manifest["inputs"]["queries"]["path"])
    if len(queries) != 500:
        raise ValueError("exact500 registered calibration queries required")
    cell = manifest.get("plan", [{}])[0]
    copies = cell.get("copies", 1) if cell.get("family") not in ("M0", "M-E") else 0
    package = copies * sum(item["bytes"] for item in manifest["package"])
    if manifest.get("scope") == "registered-M-selected-cell":
        # Six wire files; one result plus provisional/final reports;24 bounded
        # metadata/GPU/control/log/seed files, directories and atomic copies.
        capture = 6 * JSONL_MAX_BYTES + 3 * JSON_MAX_BYTES + 24 * CONTROL_MAX_BYTES
        # The pinned healthy G subject needs no migration/backfill. Read-only M
        # calls can change only the small equivalence verdict on open. At most
        #15 idle evictions plus3 explicit opens fit900s/default60s;20 is an upper
        # bound. An8MiB envelope per open covers b-tree path/split/WAL/SHM pages
        # for this single small marker, including main-file allocation.
        native = (900 // 60 + 3 + 2) * 8 * 1024**2
        ledger = Path(manifest["allocation"]) / "g-budget.json"
        old = json.loads(ledger.read_text())
        encoded = len((json.dumps(old, indent=2) + "\n").encode("utf-8"))
        # Existing history is finite input, never capped/deleted. New M owner,
        # attempt and root fields add fewer than128KiB even with JSON escaping.
        shared_atomic = 2 * (encoded + 128 * 1024)
        return package + capture + native + shared_atomic
    maximum = max(len(json.dumps({"jsonrpc": "2.0", "id": 10000, "method": "tools/call",
        "params": {"name": "search", "arguments": {"store": "a", "query": query,
                  "mode": "hybrid", "limit": 10}}}, ensure_ascii=False,
        separators=(",", ":")).encode("utf-8")) + 1 for query in queries)
    # Existing64MiB raw streams and their escaped/decoded snapshots fit below1GiB;
    # request copies are additionally bounded by the actual selected query sizes.
    return package + 1024**3 + 12 * 1504 * maximum


def selected_cell(cell_id):
    """Return the unchanged selected workload under its separate calibration ID."""
    from m_controller import load_plan
    if cell_id != CELL_ID:
        raise ValueError("only the explicitly selected first calibration is implemented")
    cell = next(row for row in load_plan() if row["id"] == cell_id)
    return dict(cell, id=CALIBRATION_ID, origin_id=cell_id)


def validate_limits(limits):
    """Require a positive selected allowance that includes bounded inner/outer cleanup."""
    keys = {"phase_ns", "payload_ns", "new_retained_bytes"}
    if (set(limits) != keys or any(type(limits[k]) is not int or limits[k] <= 0 for k in keys)
            or not 7 * 10**9 < limits["payload_ns"] <= limits["phase_ns"] - 20 * 10**9
            or limits["phase_ns"] > 900 * 10**9):
        raise ValueError("positive bounded calibration allowance required, including cleanup")


def admit(profile, ledger, retained_bytes, limits, *, calibration_id=CALIBRATION_ID):
    """Reserve positive remaining calibration allowances on the original shared budget."""
    validate_limits(limits)
    usage = ledger.get("calibration_usage", {}).get(calibration_id, {})
    if usage and usage.get("limits") != limits:
        raise ValueError("selected cumulative calibration allowance changed")
    prior_ns, prior_bytes = usage.get("wall_ns", 0), usage.get("new_retained_bytes", 0)
    if any(type(v) is not int or v < 0 for v in (prior_ns, prior_bytes, ledger["used_ns"], retained_bytes)):
        raise ValueError("bounded prior calibration/shared usage required")
    phase_ns = limits["phase_ns"] - prior_ns
    growth = limits["new_retained_bytes"] - prior_bytes
    if phase_ns <= 20 * 10**9 or growth <= 0:
        raise ValueError("remaining calibration allowance cannot fund work and cleanup")
    if (ledger["used_ns"] + 2 * phase_ns + profile["reserved_other_work_ns"] > profile["total_shared_ns"]
            or retained_bytes + 2 * growth + profile["reserved_other_work_bytes"]
            > profile["total_shared_retained_bytes"]):
        raise ValueError("positive twofold calibration headroom unavailable")
    return {"remaining_phase_ns": phase_ns, "remaining_retained_bytes": growth,
            "prior_wall_ns": prior_ns, "prior_retained_bytes": prior_bytes}


def admit_selected(profile, ledger, retained_bytes, limits, cell_id):
    """Admit one canonical required cell under selected cumulative shared TOTAL caps."""
    from m_controller import load_plan
    if cell_id not in {cell["id"] for cell in load_plan()}:
        raise ValueError("canonical required M cell selection required")
    if any(type(profile.get(key)) is not int or not 0 < profile[key] <= bound
           for key, bound in (("total_shared_ns", 48 * 3600 * 10**9),
                              ("total_shared_retained_bytes", 192 * 1024**3))):
        raise ValueError("selected M shared maxima are48h/192GiB")
    usage = ledger.get("selected_cell_usage", {}).get(cell_id, {})
    selected = dict(ledger, calibration_usage={CALIBRATION_ID: usage})
    return admit(profile, selected, retained_bytes, limits)


def registered_cell(cell_id):
    """Return one exact canonical required M configuration; caller cannot edit its workload."""
    from m_controller import load_plan
    return next((cell for cell in load_plan() if cell["id"] == cell_id), None) or _unknown_cell()


def _unknown_cell():
    raise ValueError("canonical required M cell selection required")
