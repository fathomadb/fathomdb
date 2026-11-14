"""Common retained metadata includes registered frozen roots but excludes cell results."""

import json
from pathlib import Path


def register_root(allocation, root):
    """Bind an owned frozen metadata directory before any blocking common work."""
    from g_closed_guard import write_budget
    allocation, root = Path(allocation).resolve(), Path(root).resolve()
    if not root.is_relative_to(allocation) or root == allocation:
        raise ValueError("common metadata root must be inside the owned allocation")
    path = allocation / "g-budget.json"
    budget = json.loads(path.read_text()) if path.exists() else {
        "schema": "slice30-gd-budget/v1", "used_ns": 0, "inflight": None, "cell_used_ns": {}}
    roots = budget.setdefault("closed_common_roots", [])
    if str(root) not in roots:
        roots.append(str(root))
        write_budget(path, budget)


def common_bytes(allocation):
    """Count all common files once, including manifests/prepared files/reports outside common/."""
    allocation = Path(allocation).resolve()
    path = allocation / "g-budget.json"
    budget = json.loads(path.read_text()) if path.exists() else {}
    files = {item for item in (allocation / "common").rglob("*") if item.is_file()}
    files.update(item for item in (allocation / "g-budget.json", allocation / "g-campaign.lock")
                 if item.is_file())
    for value in budget.get("closed_common_roots", []):
        root = Path(value).resolve()
        if not root.is_relative_to(allocation) or root == allocation:
            raise ValueError("retained common metadata root is outside its owned allocation")
        files.update(item for item in root.rglob("*") if item.is_file()
                     and not item.is_relative_to(root / "results"))
    return sum(item.stat().st_size for item in files)
