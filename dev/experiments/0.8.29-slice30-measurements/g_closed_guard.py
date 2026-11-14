"""Closed-profile guardian accounting; ordinary verification phases remain unchanged."""

import json
import os
from pathlib import Path
import time


def discover(command):
    """Recognize only normal G runner commands with the closed-copy profile."""
    runner = Path(__file__).with_name("g_runner.py").resolve()
    if len(command) < 3 or Path(command[1]).resolve() != runner:
        return None
    action = command[2]
    if action == "freeze":
        values = dict(zip(command[3::2], command[4::2], strict=True))
        if values.get("--preparation") != "closed-copy-v1":
            return None
        profile = {"allocation": values["--allocation"], "mode": values["--stage"], "root": values["--out"]}
        if profile["mode"] == "closed-copy-write-matrix":
            from g_closed_limits import reviewed_limits
            profile["limits"] = reviewed_limits(values["--cost-review-from"])
        return profile
    if action in ("run", "report"):
        root = Path(command[3]).resolve()
        manifest = json.loads((root / "manifest.json").read_text())
        if manifest.get("preparation") == "closed-copy-v1":
            profile = {"allocation": manifest["allocation"], "mode": manifest["stage"], "root": str(root)}
            if profile["mode"] == "closed-copy-write-matrix":
                from g_closed_limits import reviewed_limits
                profile["limits"] = reviewed_limits(manifest["cost_review_from"])
                if profile["limits"] != {key: manifest["closed_limits"][key] for key in profile["limits"]}:
                    raise ValueError("outer prospective limits differ from frozen reviewed totals")
            return profile
    return None


def write_budget(path, value):
    """Publish complete budget observations atomically for the live guardian."""
    path = Path(path)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    os.replace(temporary, path)


class ClosedGuard:
    """Fund outer work without recharging nested common/cell wall intervals."""

    def __init__(self, profile, started_ns, task):
        self.started = started_ns
        self.allocation = Path(profile["allocation"]).resolve()
        self.root = Path(profile["root"]).resolve()
        self.mode = profile["mode"]
        if (self.mode not in ("closed-copy-calibration", "closed-copy-read-r1", "closed-copy-write-matrix")
                or not self.allocation.is_relative_to(Path(task).resolve())
                or not self.root.is_relative_to(self.allocation)):
            raise ValueError("closed guardian requires a known mode and owned allocation")
        from g_closed_limits import DEFAULT, validate_limits
        self.limits = validate_limits(profile.get("limits") or DEFAULT)
        if self.mode != "closed-copy-write-matrix" and self.limits != DEFAULT:
            raise ValueError("only the reviewed successor write matrix can change shared totals")
        self.path = self.allocation / "g-budget.json"
        budget = self.read()
        self.prior = budget["used_ns"]
        self.prior_common = budget.get("common_used_ns", {}).get(self.mode, 0)
        self.prior_cells = dict(budget.get("cell_used_ns", {}))
        self.owned = False
        self.directory = self.allocation / "common" / "guardian" / self.mode

    def read(self):
        if self.path.exists():
            return json.loads(self.path.read_text())
        return {"schema": "slice30-gd-budget/v1", "used_ns": 0, "inflight": None, "cell_used_ns": {}}

    def admit(self):
        """Refuse before child/cgroup creation when common ownership or reserve is unavailable."""
        budget = self.read()
        if budget.get("closed_guardian"):
            raise RuntimeError("previous closed guardian has not established quiescence")
        if budget.get("inflight"):
            raise RuntimeError("unfinished shared attempt blocks closed guardian")
        if self.prior_common >= 560 * 10**9:
            raise RuntimeError("common reserve exhausted or cleanup reserve unavailable")
        roots = budget.setdefault("closed_common_roots", [])
        if str(self.root) not in roots:
            roots.append(str(self.root))
        self.owned = True
        budget["closed_guardian"] = {"pid": os.getpid(), "mode": self.mode,
                                    "started_monotonic_ns": self.started, "limits": self.limits}
        write_budget(self.path, budget)

    def residual(self, budget, now):
        """Subtract completed charges and the currently active cell interval only."""
        charged = budget["used_ns"] - self.prior
        active = budget.get("inflight") or {}
        active_cell = 0
        if (active and active.get("status") != "guardian_stopped"
                and not active.get("cell", "").startswith("g-common-")):
            origin = active.get("started_monotonic_ns")
            if type(origin) is not int or origin < self.started:
                raise RuntimeError("active closed cell has no matching monotonic accounting")
            committed = budget.get("cell_used_ns", {}).get(active["cell"], 0) - self.prior_cells.get(active["cell"], 0)
            active_cell = max(0, now - origin - committed)
        return max(0, now - self.started - charged - active_cell)

    def check(self):
        """Return a common/whole-phase/storage stop without changing resource floors."""
        from g_campaign import active_bytes
        from g_common_storage import common_bytes
        budget = self.read()
        now = time.monotonic_ns()
        common = budget.get("common_used_ns", {}).get(self.mode, 0) + self.residual(budget, now)
        if common >= 560 * 10**9:
            return "closed_common_cleanup_reserve"
        if budget["used_ns"] + self.residual(budget, now) >= self.limits["campaign_seconds"] * 10**9:
            return "closed_shared_time_exhausted"
        if common_bytes(self.allocation) >= 1024**3:
            return "closed_common_storage_exhausted"
        if active_bytes(self.allocation) >= self.limits["active_bytes"]:
            return "closed_shared_storage_exhausted"
        return None

    def finish(self, uncertain, stopped_ns=None):
        """Charge observed residual wall after exact owned quiescence and final probes."""
        if not self.owned:
            return {"admitted": False, "charged_phase_ns": 0}
        now = time.monotonic_ns()
        budget = self.read()
        recovered = 0
        active = budget.get("inflight") or {}
        if active and not active.get("cell", "").startswith("g-common-"):
            origin = active.get("started_monotonic_ns")
            if type(origin) is not int or origin < self.started:
                raise RuntimeError("uncommitted closed cell has no owned monotonic interval")
            through = now if uncertain else stopped_ns or now
            committed = budget.get("cell_used_ns", {}).get(active["cell"], 0) - self.prior_cells.get(active["cell"], 0)
            recovered = max(0, through - origin - committed)
            usage = budget.setdefault("cell_used_ns", {})
            usage[active["cell"]] = usage.get(active["cell"], 0) + recovered
            budget["used_ns"] += recovered
            active.update(status="cleanup_uncertain" if uncertain else "guardian_stopped",
                          accounted_through_monotonic_ns=through, quiescence_known=not uncertain)
        residual = self.residual(budget, now)
        budget["used_ns"] += residual
        common = budget.setdefault("common_used_ns", {})
        common[self.mode] = common.get(self.mode, 0) + residual
        exceeded = common[self.mode] > 600 * 10**9
        if uncertain:
            budget["closed_guardian"]["status"] = "cleanup_uncertain"
        else:
            budget["closed_guardian"] = None
        write_budget(self.path, budget)
        return {"admitted": True, "residual_ns": residual, "recovered_cell_ns": recovered,
                "charged_phase_ns": budget["used_ns"] - self.prior,
                "common_used_ns": common[self.mode], "reserve_exceeded": exceeded}
