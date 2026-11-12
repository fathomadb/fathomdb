"""Conservative owned G/D resource accounting and bounded phase execution."""


def aggregate_roots(roots):
    """Return the sum of each disjoint root's larger allocated/apparent size."""
    return sum(max(root["allocated"], root["apparent"]) for root in roots)


def setup_growth(roots, task_path, raw_roots, baseline):
    """Account setup growth after removing raw subtrees in both byte metrics."""
    remaining = []
    for root in roots:
        row = dict(root)
        if row["path"] == task_path:
            for metric in ("allocated", "apparent"):
                row[metric] = max(0, row[metric] - sum(raw[metric] for raw in raw_roots))
        remaining.append(row)
    return max(0, aggregate_roots(remaining) - baseline)


GiB = 1024**3


def reservation_ok(owned, setup_used, g_raw, d_raw):
    """Preserve accepted setup and both raw-family reservations under200GiB."""
    if owned >= 200*GiB or setup_used > 96*GiB or g_raw > 16*GiB or d_raw > 16*GiB:
        return False
    forecast = owned + (96*GiB - setup_used) + (16*GiB - g_raw) + (16*GiB - d_raw)
    return forecast <= 200*GiB
