"""Bindings from registered M cells to actual owned MCP processes."""

from pathlib import Path
import time

from m_report import summarize_invocations
from m_schedule import closed_loop
from m_session import McpSession


def _loops(session, cell, queries, deadline_ns):
    phases = {}
    complete = True
    for phase, count, concurrency in (("first", cell["first"], 1),
                                      ("warmup", cell["warmups"], 1),
                                      ("timed", cell["timed"], cell["concurrency"])):
        session.timeout = min(45, max(0, (deadline_ns - time.monotonic_ns()) / 10**9))
        value = closed_loop(session, queries, concurrency, total=count, phase=phase,
                            mode=cell["search_mode"])
        session.phase_boundary()
        phases[phase] = value
        complete &= len(value["completed"]) == count
        complete &= all(session.records[key].get("ok") is True for key in value["submitted"])
        if not complete:
            break
    return {"phases": phases, "complete": complete}


def execute(cell, command, root, *, queries, deadline_ns, files=None, control_path=None):
    """Run a cell against its actual stdio process and retain decoded outcomes."""
    with McpSession(command, root, result_format=cell["result_format"]) as session:
        session.initialize()
        if cell["family"] in ("M-R", "M-C"):
            result = _loops(session, cell, queries, deadline_ns)
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
    result["complete"] &= session.exit["code"] == 0 and not session.failures
    result["failures"] = session.failures
    result["whole_process"] = summarize_invocations(session.records.values(),
        session.exit["exit_ns"] - session.started_ns,
        stderr=(Path(root) / "stderr.raw").read_text(errors="replace").splitlines(),
        log_level=cell["log"])
    for phase, value in result.get("phases", {}).items():
        value["summary"] = summarize_invocations(
            [session.records[key] for key in value["submitted"]], value["wall_ns"])
    return result
