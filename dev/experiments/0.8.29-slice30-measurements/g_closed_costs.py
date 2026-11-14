"""Matched100k calibration cost extraction; actual qualification precedes aggregation."""


def calibration_costs(records, adapter):
    """Aggregate qualified dense/sparse records for one adapter, or leave costs unavailable.

    Host remainder subtracts disjoint matched workload/warmup intervals from the
    same native host wall. It includes open/close/I/O and is not exclusive GPU
    attribution. Retained nonbaseline bytes may overlap per-row growth, so their
    separate forecast use is deliberately conservative.
    """
    expected = [f"g-cal-{adapter}-b10-s1", f"g-cal-{adapter}-b1-s100"]
    selected = {row["cell"]["id"]: row for row in records if row["cell"]["adapters"] == [adapter]}
    if set(selected) != set(expected):
        return {}
    costs = {"n": 100000, "adapter": adapter, "cells": expected}
    def retain(key, value):
        if type(value) is not int or value < 0:
            raise ValueError("negative or unavailable matched calibration bound")
        costs[key] = max(costs.get(key, 0), value)
    try:
        for identifier in expected:
            row = selected[identifier]
            cell, actual = row["cell"], row["actual"]
            if cell["n"] != 100000 or actual["preparation"]["validated"] is not True:
                return {}
            preparation, step = actual["preparation"], actual["steps"][0]
            result = step["result"]
            retain("copy_ns", preparation["copy_ns"])
            retain("validation_ns", preparation["validation_ns"])
            retain("copy_bytes", preparation["copied_bytes"])
            remainder = (step["host_wall_ns"] - sum(item["wall_ns"] for item in result["passes"])
                         - sum(item["elapsed_ns"] for item in result["warmups"]))
            retain("driver_remainder_ns", remainder)
            retain("observer_tail_max_ns", step["observation_max_wall_ns"])
            rows = ((cell["samples"] + cell["searches_per_batch"] - 1)
                    // cell["searches_per_batch"]) * cell["batch_size"]
            retain("physical_growth_per_row",
                   (row["peak_store_bytes"] - preparation["copied_bytes"] + rows - 1) // rows)
            retain("fixed_bytes", row["retained_bytes"] - preparation["copied_bytes"])
            for batch in result["batches"]:
                retain("write_row_max_ns", (batch["write_ns"] + batch["rows"] - 1) // batch["rows"])
                if batch["adjacent_readiness"]["available"] is not True:
                    return {}
                retain("readiness_max_ns", batch["adjacent_readiness"]["elapsed_ns"])
            for probe in result["probes"]:
                retain("point_probe_max_ns", probe["point_probe"]["elapsed_ns"])
                retain("text_probe_max_ns", probe["text_probe"]["elapsed_ns"])
    except (KeyError, TypeError, ValueError, ZeroDivisionError):
        return {}
    required = {"copy_ns", "validation_ns", "copy_bytes", "driver_remainder_ns", "observer_tail_max_ns",
                "physical_growth_per_row", "fixed_bytes", "write_row_max_ns",
                "readiness_max_ns", "point_probe_max_ns", "text_probe_max_ns"}
    return costs if required <= costs.keys() else {}
