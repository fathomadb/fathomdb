"""Bindings from registered M cells to actual owned MCP processes."""

from pathlib import Path
import time

from m_report import summarize_invocations
from m_diagnostics import PrivateControl, run_case
from m_lifecycle import alternating, first_calls, idle_cycle
from m_load import observe_load, summarize_load
from m_schedule import closed_loop
from m_session import McpSession


def _loops(session, cell, queries, deadline_ns):
    phases = {}
    complete = True
    for phase, count, concurrency in (("first", cell["first"], 1),
                                      ("warmup", cell["warmups"], 1),
                                      ("timed", cell["timed"], cell["concurrency"])):
        session.timeout = min(45, max(0, (deadline_ns - time.monotonic_ns()) / 10**9))
        phase_load = [observe_load()]
        value = closed_loop(session, queries, concurrency, total=count, phase=phase,
                            mode=cell["search_mode"])
        session.phase_boundary()
        phase_load.append(observe_load())
        value["load"] = summarize_load(phase_load)
        phases[phase] = value
        complete &= len(value["completed"]) == count
        complete &= all(session.records[key].get("ok") is True
                        and not session.records[key].get("payload", {}).get("degradation")
                        for key in value["submitted"])
        if not complete:
            break
    return {"phases": phases, "complete": complete}


def execute(cell, command, root, *, queries, deadline_ns, files=None, control_path=None, on_spawn=None):
    """Run a cell against its actual stdio process and retain decoded outcomes."""
    cell_load = [observe_load()]
    with McpSession(command, root, result_format=cell["result_format"]) as session:
        if on_spawn:
            on_spawn(session.child.pid)
        session.initialize()
        if cell["family"] in ("M-R", "M-C"):
            result = _loops(session, cell, queries, deadline_ns)
        elif cell["family"] == "M-L":
            if cell["case"] in ("lazy", "eager"):
                value = first_calls(session, queries[0], mode=cell["search_mode"])
            elif cell["case"] == "idle":
                value = idle_cycle(session, queries[0], mode=cell["search_mode"],
                                   deadline_ns=deadline_ns)
            elif cell["case"] == "alternating":
                value = alternating(session, queries[0], mode=cell["search_mode"], files=files)
            else:
                raise ValueError("unregistered lifecycle case")
            result = {"lifecycle": value, "complete": value["complete"]}
        elif cell["family"] == "M-E":
            if control_path is None:
                raise ValueError("private diagnostic control socket required")
            with PrivateControl(control_path, root) as control:
                value = run_case(cell["case"], session, control)
            result = {"diagnostic": value["diagnostic"], "outcome": value,
                      "control_failures": control.failures,
                      "complete": value["diagnostic"]["complete"] and not control.failures}
        else:
            search = session.tool("search", {"query": queries[0], "mode": cell["search_mode"], "limit": 10})
            fetched = session.tool("fetch", {"refs": ["l:alpha"]})
            orient = session.tool("orient", {})
            error = session.tool("search", {"store": "missing", "query": queries[0]})
            refs = [row["ref"] for row in search.get("payload", {}).get("results", [])]
            body = fetched.get("payload", {}).get("items", [{}])[0].get("record", {}).get("body")
            result = {"events": ["search", "fetch", "orient", "typed_error"],
                      "literal_refs": refs, "fetch_body": body, "typed_error": error.get("error")}
            result["complete"] = (refs == ["l:alpha"] and body == "alpha river crossing\né😀"
                and all(row["ok"] for row in (search, fetched, orient))
                and error.get("error") == "invalid_request/unknown_store")
    result["exit"] = session.exit
    cell_load.append(observe_load())
    result["load"] = summarize_load(cell_load)
    result["pid"] = session.child.pid
    result["complete"] &= session.exit["code"] == 0 and not session.failures
    if cell["family"] in ("M-R", "M-C", "M-L"):
        degraded = [row["rpc_id"] for row in session.records.values()
                    if row["request"]["params"].get("name") == "search"
                    and row.get("payload", {}).get("degradation")]
        result["unexpected_degradation"] = degraded
        result["complete"] &= not degraded
    result["failures"] = session.failures
    result["whole_process"] = summarize_invocations(session.records.values(),
        session.exit["exit_ns"] - session.started_ns,
        stderr=(Path(root) / "stderr.raw").read_text(errors="replace").splitlines(),
        log_level=cell["log"])
    for phase, value in result.get("phases", {}).items():
        value["summary"] = summarize_invocations(
            [session.records[key] for key in value["submitted"]], value["wall_ns"])
        del value["summary"]["queue"]
    return result
