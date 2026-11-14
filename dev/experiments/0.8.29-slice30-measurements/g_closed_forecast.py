"""Prospective2x costs from matched100k CLOSED-copy calibration, never latency correction."""

def forecast_closed_write(cell, observed, remaining_ns, retained_bytes, legacy):
    """Keep original refusal visible and require matched actual calibration/repeat inputs."""
    cal = observed.get("calibration") or {}
    fields = ("copy_ns", "validation_ns", "copy_bytes", "driver_remainder_ns",
              "point_probe_max_ns", "text_probe_max_ns", "readiness_max_ns",
              "write_row_max_ns", "observer_tail_max_ns", "physical_growth_per_row", "fixed_bytes")
    route_fields = ("route_wall_ns", "route_requests", "raw_bytes_per_record")
    adapter, route = cell["adapters"][0], cell["routes"][0]
    expected = [f"g-cal-{adapter}-b10-s1", f"g-cal-{adapter}-b1-s100"]
    if (any(type(cal.get(key)) is not int or cal[key] < 0 for key in fields)
            or any(type(observed.get(key)) is not int or observed[key] < 0 for key in route_fields)
            or not observed.get("route_requests") or cal.get("n") != 100000
            or cal.get("adapter") != adapter or cal.get("cells") != expected
            or (observed.get("read_repeat"), observed.get("read_adapter"), observed.get("read_route"))
            != (cell["repeat"], adapter, route)):
        return {"allowed": False, "reason": "calibration_cost_unavailable", "total_ns": None,
                "new_bytes": None, "components": {}, "legacy_forecast": legacy}
    cohorts = (cell["samples"] + cell["searches_per_batch"] - 1) // cell["searches_per_batch"]
    rows = cohorts * cell["batch_size"]
    def route_cost(count):
        return (2 * observed["route_wall_ns"] * count + observed["route_requests"] - 1) // observed["route_requests"]
    components = {
        "copy_validation_ns": 2 * (cal["copy_ns"] + cal["validation_ns"]),
        "driver_remainder_ns": 2 * cal["driver_remainder_ns"],
        "warmup_ns": route_cost(cell["warmup"]), "natural_ns": route_cost(cell["samples"]),
        "write_ns": 2 * cal["write_row_max_ns"] * rows,
        "readiness_ns": 2 * cal["readiness_max_ns"] * (cohorts + 1),
        "point_probe_ns": 2 * cal["point_probe_max_ns"] * rows,
        "text_probe_ns": 2 * cal["text_probe_max_ns"] * rows,
        "observer_tail_ns": 2 * cal["observer_tail_max_ns"],
    }
    total_ns = sum(components.values())
    records = cell["samples"] + cell["warmup"] + 2 * rows + cohorts
    new_bytes = (cal["copy_bytes"] + 2 * cal["physical_growth_per_row"] * rows
                 + 2 * observed["raw_bytes_per_record"] * records + 2 * cal["fixed_bytes"])
    reason = ("whole_cell_time_forecast" if total_ns > cell["cell_seconds"] * 10**9 else
              "shared_time_forecast" if total_ns > remaining_ns else
              "retained_storage_forecast" if new_bytes + retained_bytes > cell["active_bytes"] else None)
    return {"allowed": reason is None, "reason": reason, "components": components,
            "total_ns": total_ns, "new_bytes": new_bytes, "cohorts": cohorts,
            "replacement_rows": rows, "observed": observed, "legacy_forecast": legacy,
            "method": "closed-copy-v1: exact baseline bytes;2x matched100k copy/validation,route,write,probe,readiness,remainder,growth/raw bounds; observer tail may overlap remainder, conservatively counted again; not exclusive GPU attribution"}


def forecast_closed_read(cell, observed, costs, remaining_ns, retained_bytes, legacy_seed_ns):
    """Forecast the required reversed R1 read on a new physical copy, never a measured R1 result."""
    required = ("copy_ns", "validation_ns", "copy_bytes", "fixed_bytes")
    if (set(costs) != {"engine", "sdk"} or observed.get("n") != 100000 or observed.get("repeat") != 0
            or type(observed.get("elapsed_ns")) is not int or observed["elapsed_ns"] <= 0
            or type(observed.get("retained_bytes")) is not int or observed["retained_bytes"] < 0
            or any(any(type(value.get(key)) is not int or value[key] < 0 for key in required)
                   for value in costs.values())
            or len({value["copy_bytes"] for value in costs.values()}) != 1):
        return {"allowed": False, "reason": "calibration_cost_unavailable", "total_ns": None,
                "new_bytes": None, "legacy_seed_ns": legacy_seed_ns}
    components = {"copy_validation_ns": 2 * max(value["copy_ns"] + value["validation_ns"] for value in costs.values()),
                  "ordinary_r0_ns": 2 * observed["elapsed_ns"]}
    total_ns = sum(components.values())
    new_bytes = (costs["engine"]["copy_bytes"] + 2 * observed["retained_bytes"]
                 + 2 * max(value["fixed_bytes"] for value in costs.values()))
    reason = ("whole_cell_time_forecast" if total_ns > cell["cell_seconds"] * 10**9 else
              "shared_time_forecast" if total_ns > remaining_ns else
              "retained_storage_forecast" if retained_bytes + new_bytes > cell["active_bytes"] else None)
    return {"allowed": reason is None, "reason": reason, "components": components,
            "total_ns": total_ns, "new_bytes": new_bytes, "legacy_seed_ns": legacy_seed_ns,
            "legacy_seed_time_feasible_900s": legacy_seed_ns <= 900 * 10**9,
            "method": "2x entire matched ordinary R0 wall including warmup/open/close plus2x copy/validation; reversed R1 remains a new physical-copy execution, not independently regenerated projection evidence"}
