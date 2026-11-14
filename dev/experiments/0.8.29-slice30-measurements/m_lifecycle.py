"""M lifecycle records use observed replies/states, never sleep-only inference."""

from pathlib import Path
import time


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
    """Retain each explicit switch, observed ready/closed states and reopened epoch."""
    identities = {}
    for name, path in (files or {}).items():
        stat = Path(path).stat()
        identities[name] = {"path": str(path), "device": stat.st_dev,
                            "inode": stat.st_ino, "bytes": stat.st_size}
    events, issues, epochs = [], [], {}
    if set(identities) != set(stores):
        issues.append("physical file identities unavailable")
    for store in stores:
        search = session.tool("search", {"store": store, "query": query,
                              "mode": mode, "limit": 10}, store=store, mode=mode, query_index=0)
        listing = session.tool("list_stores", {})
        events.extend((dict(search, event="search", store=store),
                       dict(listing, event="states", store=store)))
        payload = search.get("payload", {})
        if (not search["ok"] or payload.get("store") != store
                or payload.get("epoch") != epochs.get(store, 0) + 1):
            issues.append(f"unexpected switch result/epoch for {store}")
        epochs[store] = payload.get("epoch", 0)
        states = {row["name"]: row for row in listing.get("payload", {}).get("stores", [])}
        for name in set(stores):
            expected = "ready" if name == store else "closed"
            if not listing["ok"] or states.get(name, {}).get("readiness") != expected:
                issues.append(f"unobserved {expected} state for {name}")
    session.phase_boundary()
    return {"events": events, "files": identities, "issues": issues,
            "switch_call_ns": [row["elapsed_ns"] for row in events if row["event"] == "search"],
            "complete": not issues}


def idle_cycle(session, query, *, deadline_ns, store="a", mode="hybrid"):
    """Poll actual closed state at250ms, then retain reopened/immediate epochs."""
    arguments = {"store": store, "query": query, "mode": mode, "limit": 10}
    first = session.tool("search", arguments, phase="first", store=store, mode=mode, query_index=0)
    ready = session.tool("list_stores", {})
    events = [dict(first, event="first"), dict(ready, event="ready")]

    def state(row):
        return next((entry["readiness"] for entry in row.get("payload", {}).get("stores", [])
                     if entry["name"] == store), None)

    result = {"events": events, "poll_interval_ms": 250, "complete": False,
              "disposition": "NO_OBSERVED_EVICTION"}
    if not first["ok"] or not ready["ok"] or state(ready) != "ready":
        result["disposition"] = "INCOMPLETE_READINESS"
        return result
    closed = False
    while time.monotonic_ns() < deadline_ns:
        remaining = (deadline_ns - time.monotonic_ns()) / 10**9
        if remaining <= 0:
            break
        time.sleep(min(0.25, remaining))
        if time.monotonic_ns() >= deadline_ns:
            break
        listing = session.tool("list_stores", {})
        closed = listing["ok"] and state(listing) == "closed"
        events.append(dict(listing, event="idle_closed" if closed else "idle_poll"))
        if closed:
            break
    if not closed:
        session.phase_boundary()
        return result
    reopened = session.tool("search", arguments, store=store, mode=mode, query_index=0)
    immediate = session.tool("search", arguments, store=store, mode=mode, query_index=0)
    events.extend((dict(reopened, event="reopened"), dict(immediate, event="immediate")))
    expected = first["payload"]["epoch"] + 1
    result["complete"] = (reopened["ok"] and immediate["ok"]
        and reopened.get("payload", {}).get("epoch") == expected
        and immediate.get("payload", {}).get("epoch") == expected
        and time.monotonic_ns() <= deadline_ns)
    result["disposition"] = "OBSERVED_REOPEN" if result["complete"] else "INCOMPLETE_REOPEN"
    result["whole_record_ns"] = time.monotonic_ns() - first["send_start_ns"]
    session.phase_boundary()
    return result
