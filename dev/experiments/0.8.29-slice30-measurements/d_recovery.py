"""Explicit recovery of one physically terminated D guardian; never dispatch work."""


def recover(options, *, task, observe=None, clock=None, after_publish=None):
    """Reconcile a proven historical D stop without retrying any cell."""
    return {"complete": False, "reason": "terminated D recovery unavailable"}
