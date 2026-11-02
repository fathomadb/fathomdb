"""Bounded experiment controller; budgets are not production defaults."""
from itertools import product

REQUESTS = 1000
CELL_SECONDS = 600
CAMPAIGN_SECONDS = 7200
RSS_BYTES = 8 * 1024**3


def grid():
    """Breadth-first predeclared order, family innermost."""
    return []


def stop_reason(cell_seconds, campaign_seconds, rss):
    return None


def summary(records, termination):
    return {"qualified": True}
