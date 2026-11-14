"""M lifecycle records use observed replies/states, never sleep-only inference."""


def first_calls(session, query, *, store="a", mode="hybrid"):
    """Record first-use through the actual MCP session."""
    return {"first": session.tool("search", {"store": store, "query": query,
                                            "mode": mode, "limit": 10}, phase="first")}
