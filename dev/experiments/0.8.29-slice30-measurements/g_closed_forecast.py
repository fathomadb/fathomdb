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
