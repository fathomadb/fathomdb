"""M closed-loop submission order, independent of server reader concurrency."""

import time


def closed_loop(client, queries, concurrency, *, total=1000, phase="timed", store="a",
                mode="hybrid", method="tools/call"):
    """Submit cyclic query identities and retain every actual terminal disposition."""
    if not queries or concurrency not in (1, 4, 8, 16, 32) or total < 1:
        raise ValueError("nonempty queries, accepted concurrency and positive count required")
    began = time.monotonic_ns()
    submissions, completions = [], []
    pending = set()
    high_water = 0

    def submit(index):
        query_index = index % len(queries)
        request_id = client.submit(
            method, {"name": "search", "arguments": {"store": store, "query": queries[query_index],
                                                     "mode": mode, "limit": 10}},
            phase=phase, submission_index=index, query_index=query_index, store=store, mode=mode)
        submissions.append(request_id)
        pending.add(request_id)

    for index in range(min(concurrency, total)):
        submit(index)
    high_water = len(pending)
    while pending:
        request_id = client.wait_any(pending)
        if request_id is None:
            break
        completions.append(request_id)
        pending.remove(request_id)
        if len(submissions) < total:
            submit(len(submissions))
            high_water = max(high_water, len(pending))
    return {"submitted": submissions, "completed": completions,
            "outstanding_high_water": high_water,
            "wall_ns": time.monotonic_ns() - began}
