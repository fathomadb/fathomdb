"""Conservative owned G/D resource accounting and bounded phase execution."""


def aggregate_roots(roots):
    """Return the sum of each disjoint root's larger allocated/apparent size."""
    return sum(max(root["allocated"], root["apparent"]) for root in roots)
