"""Bindings from registered M cells to actual owned MCP processes."""

from m_session import McpSession


def execute(cell, command, root, *, queries, deadline_ns, files=None):
    """Run a cell against its actual stdio process and retain decoded outcomes."""
    with McpSession(command, root, result_format=cell["result_format"]) as session:
        session.initialize()
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
    return result
