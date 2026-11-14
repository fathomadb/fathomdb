"""Bindings from registered M cells to actual owned MCP processes."""

from m_session import McpSession


def execute(cell, command, root, *, queries, deadline_ns, files=None):
    """Run a cell against its actual stdio process and retain decoded outcomes."""
    with McpSession(command, root, result_format=cell["result_format"]) as session:
        session.initialize()
        session.tool("search", {"query": queries[0], "mode": cell["search_mode"], "limit": 10})
    return {"complete": session.exit["code"] == 0, "events": ["search"],
            "exit": session.exit}
