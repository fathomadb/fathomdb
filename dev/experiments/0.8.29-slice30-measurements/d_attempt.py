"""D-owned subprocess binding; no campaign admission or allocation reset."""


def run_owned_cell(cell, root, command, deadline_ns, *, monitor=None):
    """Retain one D child and its native result under the caller's absolute deadline.

    The caller owns admission/locking. Return actual cleanup, elapsed and retained
    cost facts; unavailable results stay unavailable. This seam grants no runtime
    permission and cannot qualify a native report by child exit alone.
    """
    return {"id": cell["id"], "status": "not_run", "reason": "D subprocess binding is not implemented",
            "native": None, "peak_bytes": None, "deadline_ns": deadline_ns,
            "cleanup_uncertain": False, "live_owned_pids": []}
