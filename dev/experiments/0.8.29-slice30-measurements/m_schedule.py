"""M closed-loop submission order, independent of server reader concurrency."""

import time


def closed_loop(client, queries, concurrency, *, total=1000, phase="timed", store="a",
                mode="hybrid", method="tools/call"):
    """Submit cyclic query identities and retain every actual terminal disposition."""
    began = time.monotonic_ns()
    submissions, completions = [], []
    for index in range(total):
        query_index = index % len(queries)
        request_id = client.submit(
            method, {"name": "search", "arguments": {"store": store, "query": queries[query_index],
                                                     "mode": mode, "limit": 10}},
            phase=phase, submission_index=index, query_index=query_index, store=store, mode=mode)
        submissions.append(request_id)
        if client.wait(request_id) is None:
            break
        completions.append(request_id)
    return {"submitted": submissions, "completed": completions,
            "outstanding_high_water": min(1, len(submissions)),
            "wall_ns": time.monotonic_ns() - began}
