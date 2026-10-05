#!/usr/bin/env python3
"""Validate Slice 115 raw cells and derive nearest-rank latency summaries."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import re


PATHS = (
    "open_fresh", "open_populated", "close", "canonical_write",
    "projection", "model_cpu", "text", "vector_stage", "hybrid",
    "graph_expand", "graph_evidence", "erasure",
)


def digest(path: Path) -> str:
    """Return a file's SHA-256 digest, raising if it is absent."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _binding(data: dict, key: str, expected: str | None = None) -> None:
    value = data.get(key)
    width = 40 if key == "source_sha" else 64
    if not isinstance(value, str) or not re.fullmatch(rf"[0-9a-f]{{{width}}}", value):
        raise ValueError(f"{key} missing or malformed")
    if expected is not None and value != expected:
        raise ValueError(f"{key} mismatch: raw {value}, expected {expected}")


def _rank(values: list[int], percentile: float) -> int:
    return values[max(0, math.ceil(percentile * len(values)) - 1)]


def validate(data: dict, source_sha: str, protocol_sha256: str,
             runner_sha256: str) -> dict:
    """Reject invalid/binding-mismatched raw cells and summarize valid samples.

    Invalid attempts remain in the input receipt. No valid slow sample is
    discarded. The caller must retain the raw file alongside this summary.
    """
    for key, expected in (("source_sha", source_sha),
                          ("protocol_sha256", protocol_sha256),
                          ("runner_sha256", runner_sha256)):
        _binding(data, key, expected)
    for key in ("binary_sha256", "corpus_sha256"):
        _binding(data, key)
    if data.get("environment_valid") is not True:
        raise ValueError("environment invalid or unrecorded")
    cells = data.get("cells")
    if not isinstance(cells, dict) or set(cells) != set(PATHS):
        raise ValueError(f"cells mismatch; expected {PATHS}")
    summary = {}
    for path in PATHS:
        samples = cells[path]
        if not isinstance(samples, list):
            raise ValueError(f"{path}: samples must be a list")
        valid = [s for s in samples if isinstance(s, dict) and s.get("valid") is True]
        invalid = [s for s in samples if not isinstance(s, dict) or s.get("valid") is not True]
        if len(valid) < 7:
            raise ValueError(f"{path}: seven valid samples required, got {len(valid)}")
        for sample in invalid:
            if not isinstance(sample, dict) or not sample.get("reason"):
                raise ValueError(f"{path}: invalid attempt lacks reason")
        timings = []
        prestate = None
        for sample in valid:
            ns = sample.get("latency_ns")
            stage_ns = sample.get("stage_ns")
            if not isinstance(ns, int) or isinstance(ns, bool) or ns <= 0:
                raise ValueError(f"{path}: latency_ns invalid")
            stage_valid = (isinstance(stage_ns, int) and not isinstance(stage_ns, bool)
                           and 0 < stage_ns <= ns and bool(sample.get("stage")))
            if not stage_valid:
                profile = data.get("profiles", {}).get(path, {})
                profile_valid = (sample.get("profile_ref") == f"profiles/{path}.txt"
                                 and profile.get("method") == "gdb-interrupt-stack"
                                 and isinstance(profile.get("operation_samples"), int)
                                 and profile["operation_samples"] >= 3
                                 and isinstance(profile.get("sha256"), str)
                                 and bool(re.fullmatch(r"[0-9a-f]{64}", profile["sha256"])))
                if not profile_valid:
                    raise ValueError(f"{path}: stage attribution or usable profile missing")
            if (not isinstance(sample.get("correctness_count"), int)
                    or sample["correctness_count"] <= 0):
                raise ValueError(f"{path}: semantic correctness count invalid")
            if path == "projection":
                write_ns = sample.get("write_ns")
                drain_ns = sample.get("drain_ns")
                if (not isinstance(write_ns, int) or isinstance(write_ns, bool)
                        or write_ns <= 0 or not isinstance(drain_ns, int)
                        or isinstance(drain_ns, bool) or drain_ns <= 0
                        or write_ns + drain_ns > ns):
                    raise ValueError("projection: write-to-ready timing missing or inconsistent")
            if path == "model_cpu":
                if (sample.get("model_projection_rows") != 1
                        or not isinstance(sample.get("projection_ns"), int)
                        or sample["projection_ns"] <= 0):
                    raise ValueError("model projection evidence missing")
            if path in ("canonical_write", "projection", "erasure"):
                before = sample.get("prestate")
                after = sample.get("poststate")
                expected_rows = 1 if path == "erasure" else 0
                expected_sources = ["slice115:erase"] if path == "erasure" else []
                if (not isinstance(before, dict) or before.get("canonical_rows") != expected_rows
                        or before.get("projection_rows") != expected_rows
                        or before.get("source_ids") != expected_sources
                        or not isinstance(before.get("digest"), str)
                        or not re.fullmatch(r"[0-9a-f]{64}", before["digest"])
                        or (prestate is not None and before != prestate)
                        or not isinstance(after, dict)
                        or after.get("canonical_rows") != (0 if path == "erasure" else 1)):
                    raise ValueError(f"{path}: prestate or poststate mismatch")
                prestate = before
            timings.append(ns)
        timings.sort()
        summary[path] = {
            "valid_samples": len(valid), "invalid_attempts": len(invalid),
            "p50_ns": _rank(timings, 0.50), "p90_ns": _rank(timings, 0.90),
            "p99_ns": _rank(timings, 0.99), "maximum_ns": timings[-1],
            "semantic_success_fraction": len(valid) / len(samples),
        }
    return {"source_sha": source_sha, "protocol_sha256": protocol_sha256,
            "runner_sha256": runner_sha256, "cells": summary}


def main() -> None:
    """Validate a raw receipt against frozen files and write a summary."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw", type=Path, required=True)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    protocol = json.loads(args.protocol.read_text())
    source_sha = protocol["engine_source_sha"]
    data = json.loads(args.raw.read_text())
    if data["corpus_sha256"] != digest(args.raw.parent / "corpus.txt"):
        raise ValueError("corpus artifact hash mismatch")
    needed_profiles = {path for path, samples in data["cells"].items()
                       if any("profile_ref" in sample for sample in samples)}
    profiles = {}
    for path in needed_profiles:
        metadata = json.loads((args.raw.parent / "profiles" / f"{path}.json").read_text())
        if metadata.get("path") != path or metadata.get("binary_sha256") != data["binary_sha256"]:
            raise ValueError(f"{path}: profile candidate mismatch")
        profiles[path] = metadata
    data["profiles"] = profiles
    for path, profile in data.get("profiles", {}).items():
        if path not in PATHS or digest(args.raw.parent / "profiles" / f"{path}.txt") != profile["sha256"]:
            raise ValueError(f"{path}: profile artifact hash mismatch")
    result = validate(data, source_sha,
                      digest(args.protocol), digest(args.runner))
    ordered = sorted((path for path in PATHS if path != "model_cpu"),
                     key=lambda path: (-result["cells"][path]["p90_ns"], PATHS.index(path)))
    deep = list(dict.fromkeys([*ordered[:2], "graph_evidence"]))
    for path in deep:
        profile = profiles.get(path)
        if (not profile or not isinstance(profile.get("control_operations"), int)
                or profile["control_operations"] <= 0
                or not isinstance(profile.get("profiled_operations"), int)
                or profile["profiled_operations"] <= 0):
            raise ValueError(f"{path}: deep profile or unprofiled control missing")
    result["deep_profiles"] = deep
    result["profile_method"] = "gdb-interrupt-stack"
    raw_cells = args.raw.with_name("raw-cells.json")
    if not raw_cells.exists():
        raw_cells.write_bytes(args.raw.read_bytes())
    args.raw.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
