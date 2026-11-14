"""M lifecycle records use observed replies/states, never sleep-only inference."""


def first_calls(session, query, *, store="a", mode="hybrid"):
    """Record the exact listing/first/immediate sequence for lazy or eager launch."""
    before = session.tool("list_stores", {})
    arguments = {"store": store, "query": query, "mode": mode, "limit": 10}
    first = session.tool("search", arguments, phase="first", query_index=0, store=store, mode=mode)
    immediate = session.tool("search", arguments, phase="control", query_index=0, store=store, mode=mode)
    session.phase_boundary()
    return {"events": [dict(row, event=label) for label, row in
                       (("before", before), ("first", first), ("immediate", immediate))],
            "first_call_ns": first["elapsed_ns"], "immediate_call_ns": immediate["elapsed_ns"],
            "spawn_to_handshake_ns": session.handshake_complete_ns - session.started_ns,
            "spawn_to_first_ns": first["arrival_ns"] - session.started_ns,
            "complete": all(row["ok"] for row in (before, first, immediate))}


def alternating(session, query, *, stores=("a", "b", "a"), mode="hybrid", files=None):
    """Run the explicit A/B/A calls through the actual session."""
    return {"searches": [session.tool("search", {"store": store, "query": query,
                                                "mode": mode, "limit": 10}, store=store)
                         for store in stores]}
