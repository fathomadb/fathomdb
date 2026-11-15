"""Actual MCP-PID GPU observations; direct-text query compute remains N/A."""


def qualification(cell, samples, pid):
    """Classify only the registered route's actual compute requirement."""
    required = cell["family"] in ("M-R", "M-C", "M-L") and cell["search_mode"] == "hybrid"
    witnesses = [row for row in samples if row.get("pid") == pid
                 and row.get("pid_memory_mib", 0) > 0
                 and row.get("gpu_memory_mib", 0) > row.get("baseline_gpu_memory_mib", 0)
                 and row.get("utilization_percent", 0) > 0]
    return {"requirement": "same_MCP_PID_memory_and_positive_utilization" if required else "N/A",
            "complete": bool(witnesses) if required else True,
            "witnesses": len(witnesses) if required else 0}
