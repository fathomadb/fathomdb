"""Prospective whole G-W cell estimates from accepted current-program observations."""


def forecast_write(cell, observed, remaining_ns, retained_bytes):
    """Conservatively estimate every required synchronous phase before G-W preparation."""
    if cell.get("fresh_copy"):
        from g_closed_forecast import forecast_closed_write
        legacy = forecast_write(dict(cell, fresh_copy=False), observed, remaining_ns, retained_bytes)
        return forecast_closed_write(cell, observed, remaining_ns, retained_bytes, legacy)
    required = {"seed_wall_ns", "seed_bytes", "seed_write_ns", "seed_rows",
                "route_wall_ns", "route_requests", "open_ns", "close_ns", "text_max_ns",
                "point_probe_max_ns", "text_probe_max_ns", "readiness_max_ns",
                "write_row_max_ns", "raw_bytes_per_record"}
    missing = sorted(key for key in required if type(observed.get(key)) is not int or observed[key] < 0)
    if missing or not observed.get("seed_rows") or not observed.get("route_requests"):
        return {"allowed": False, "reason": "cost_observation_unavailable", "missing": missing,
                "components": {}, "total_ns": None, "new_bytes": None}
    def scale(value, count, denominator):
        return (2 * value * count + denominator - 1) // denominator
    cohorts = (cell["samples"] + cell["searches_per_batch"] - 1) // cell["searches_per_batch"]
    rows = cohorts * cell["batch_size"]
    proxy = observed["text_max_ns"]
    components = {
        "seed_ns": scale(observed["seed_wall_ns"], cell["n"], observed["seed_rows"]),
        "open_close_ns": 2 * (observed["open_ns"] + observed["close_ns"]),
        "warmup_ns": scale(observed["route_wall_ns"], cell["warmup"], observed["route_requests"]),
        "natural_ns": scale(observed["route_wall_ns"], cell["samples"], observed["route_requests"]),
        "write_ns": max(scale(observed["seed_write_ns"], rows, observed["seed_rows"]),
                        2 * observed["write_row_max_ns"] * rows),
        "readiness_ns": 2 * max(observed["readiness_max_ns"], proxy) * (cohorts + 1),
        "point_probe_ns": 2 * max(observed["point_probe_max_ns"], proxy) * rows,
        "text_probe_ns": 2 * max(observed["text_probe_max_ns"], proxy) * rows,
    }
    total_ns = sum(components.values())
    record_count = cell["samples"] + cell["warmup"] + 2 * rows + cohorts
    new_bytes = (scale(observed["seed_bytes"], cell["n"], observed["seed_rows"])
                 + 2 * observed["raw_bytes_per_record"] * record_count)
    reason = ("whole_cell_time_forecast" if total_ns > 900 * 10**9 else
              "shared_time_forecast" if total_ns > remaining_ns else
              "retained_storage_forecast" if new_bytes + retained_bytes > 16 * 1024**3 else None)
    return {"allowed": reason is None, "reason": reason, "total_ns": total_ns,
            "new_bytes": new_bytes, "components": components, "cohorts": cohorts,
            "replacement_rows": rows, "observed": observed,
            "method": "2x observed whole seed/route and commit costs; point/FTS/readiness use max(tiny cost,100k text max); descriptive forecast, not an exclusive timing decomposition"}
