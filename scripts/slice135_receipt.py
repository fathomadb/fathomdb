#!/usr/bin/env python3
"""Validate provisional Slice 135 Phase 1 raw receipts and recompute summaries."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import re


SHA40 = re.compile(r"[0-9a-f]{40}\Z")
SHA64 = re.compile(r"[0-9a-f]{64}\Z")
STABLE_ENVIRONMENT_FIELDS = (
    "host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler",
)


def _hash(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _binding(name: str, value: object, expected: str, pattern: re.Pattern[str]) -> None:
    if not isinstance(value, str) or not pattern.fullmatch(value):
        raise ValueError(f"{name} missing or malformed")
    if value != expected:
        raise ValueError(f"{name} mismatch")


def _same_json(left: object, right: object) -> bool:
    try:
        options = {"sort_keys": True, "separators": (",", ":"), "allow_nan": False}
        return json.dumps(left, **options) == json.dumps(right, **options)
    except (TypeError, ValueError):
        return False


def _artifact_bindings(raw: dict, protocol: dict, artifact_root: Path) -> None:
    expected = protocol.get("artifact_sha256")
    actual = raw.get("artifact_sha256")
    if not isinstance(expected, dict) or not expected or actual != expected:
        raise ValueError("artifact_sha256 missing or mismatch")
    root = artifact_root.resolve()
    for name, expected_hash in expected.items():
        if (not isinstance(name, str) or not name
                or not isinstance(expected_hash, str) or not SHA64.fullmatch(expected_hash)):
            raise ValueError("artifact_sha256 contains malformed entry")
        path = (root / name).resolve()
        if not path.is_relative_to(root) or not path.is_file():
            raise ValueError(f"artifact {name}: absent or outside artifact root")
        if _hash(path.read_bytes()) != expected_hash:
            raise ValueError(f"artifact {name}: content hash mismatch")


def _environment(raw: dict, protocol: dict) -> None:
    environment = raw.get("environment")
    minimum = protocol.get("min_disk_free_bytes")
    if (not isinstance(environment, dict)
            or environment.get("invalidators") != []
            or not isinstance(minimum, int) or isinstance(minimum, bool) or minimum < 0):
        raise ValueError("environment invalid or unrecorded")
    start = environment.get("start")
    end = environment.get("end")
    if not isinstance(start, dict) or not isinstance(end, dict):
        raise ValueError("environment start/end missing")
    for state in (start, end):
        if any(not isinstance(state.get(field), str) or not state[field]
               for field in STABLE_ENVIRONMENT_FIELDS):
            raise ValueError("environment inventory incomplete")
        swap = state.get("swap_pages")
        free = state.get("disk_free_bytes")
        if (not isinstance(swap, int) or isinstance(swap, bool) or swap < 0
                or state.get("competing_jobs") != []
                or not isinstance(free, int) or isinstance(free, bool)
                or free < minimum):
            raise ValueError("environment invalid: swap, contention or disk pressure")
    if any(start[field] != end[field] for field in STABLE_ENVIRONMENT_FIELDS):
        raise ValueError("environment drift")
    if start["swap_pages"] != end["swap_pages"]:
        raise ValueError("environment swap drift")


def _nearest_rank(sorted_values: list[int], fraction: float) -> int:
    return sorted_values[math.ceil(fraction * len(sorted_values)) - 1]


def validate(raw: dict, protocol: dict, artifact_root: Path, *,
             expected_source_sha: str, protocol_sha256: str,
             runner_sha256: str) -> dict:
    """Reject malformed or semantically degraded cells; summarize raw valid attempts.

    The caller owns and retains the raw receipt. This function never changes it.
    Unsupported tails are omitted; no valid observation is trimmed.
    """
    if not isinstance(raw, dict) or not isinstance(protocol, dict):
        raise ValueError("receipt and protocol must be objects")
    if (type(raw.get("schema_version")) is not int or raw["schema_version"] != 1
            or type(protocol.get("schema_version")) is not int
            or protocol["schema_version"] != 1):
        raise ValueError("schema_version mismatch")
    if not isinstance(expected_source_sha, str) or not SHA40.fullmatch(expected_source_sha):
        raise ValueError("expected_source_sha malformed")
    if not isinstance(protocol_sha256, str) or not SHA64.fullmatch(protocol_sha256):
        raise ValueError("protocol_sha256 malformed")
    if not isinstance(runner_sha256, str) or not SHA64.fullmatch(runner_sha256):
        raise ValueError("runner_sha256 malformed")
    _binding("source_sha", protocol.get("source_sha"), expected_source_sha, SHA40)
    _binding("source_sha", raw.get("source_sha"), expected_source_sha, SHA40)
    _binding("runner_sha256", protocol.get("runner_sha256"), runner_sha256, SHA64)
    _binding("runner_sha256", raw.get("runner_sha256"), runner_sha256, SHA64)
    _binding("protocol_sha256", raw.get("protocol_sha256"), protocol_sha256, SHA64)
    _artifact_bindings(raw, protocol, artifact_root)
    for name in ("features", "settings"):
        expected = protocol.get(name)
        if not isinstance(expected, dict) or not _same_json(raw.get(name), expected):
            raise ValueError(f"{name} missing or changed")
    _environment(raw, protocol)
    expected_cells = protocol.get("cells")
    cells = raw.get("cells")
    if (not isinstance(expected_cells, dict) or not expected_cells
            or not isinstance(cells, dict) or set(cells) != set(expected_cells)):
        raise ValueError("cells missing or changed")
    result = {}
    for name, specification in expected_cells.items():
        if not isinstance(name, str) or not name or not isinstance(specification, dict):
            raise ValueError("cell specification malformed")
        kind = specification.get("kind")
        boundary = specification.get("boundary")
        expected_checks = specification.get("expected_checks")
        if (kind not in ("query", "lifecycle")
                or not isinstance(boundary, str) or not boundary
                or not isinstance(expected_checks, dict) or not expected_checks):
            raise ValueError(f"{name}: cell specification malformed")
        cell = cells[name]
        if not isinstance(cell, dict) or cell.get("boundary") != boundary:
            raise ValueError(f"{name}: boundary mismatch")
        attempts = cell.get("attempts")
        if not isinstance(attempts, list):
            raise ValueError(f"{name}: attempts missing")
        values = []
        invalid = 0
        for position, attempt in enumerate(attempts):
            if not isinstance(attempt, dict):
                raise ValueError(f"{name}: attempt {position} malformed")
            if "semantic_ok" in attempt and attempt["semantic_ok"] is not True:
                raise ValueError(f"{name}: attempt {position} semantic failure")
            if ("observed_checks" in attempt
                    and not _same_json(attempt["observed_checks"], expected_checks)):
                raise ValueError(f"{name}: attempt {position} observed checks mismatch")
            if attempt.get("valid") is False:
                reason = attempt.get("reason")
                if not isinstance(reason, str) or not reason.strip():
                    raise ValueError(f"{name}: invalid attempt {position} lacks reason")
                invalid += 1
                continue
            if attempt.get("valid") is not True:
                raise ValueError(f"{name}: attempt {position} validity missing")
            ns = attempt.get("latency_ns")
            if not isinstance(ns, int) or isinstance(ns, bool) or ns <= 0:
                raise ValueError(f"{name}: attempt {position} latency_ns invalid")
            if attempt.get("semantic_ok") is not True:
                raise ValueError(f"{name}: attempt {position} semantic failure")
            if not _same_json(attempt.get("observed_checks"), expected_checks):
                raise ValueError(f"{name}: attempt {position} observed checks mismatch")
            values.append(ns)
        if len(values) < 100:
            raise ValueError(f"{name}: 100 valid observations required, got {len(values)}")
        values.sort()
        cell_result = {
            "kind": kind,
            "valid_samples": len(values),
            "invalid_attempts": invalid,
            "p50_ns": _nearest_rank(values, 0.50),
            "p95_ns": _nearest_rank(values, 0.95),
            "maximum_ns": values[-1],
            "valid_attempt_fraction": len(values) / len(attempts),
        }
        if kind == "query":
            if len(values) >= 1000:
                cell_result["p99_ns"] = _nearest_rank(values, 0.99)
            else:
                cell_result["unsupported_statistics"] = ["p99"]
        result[name] = cell_result
    return {
        "schema_version": 1,
        "source_sha": expected_source_sha,
        "runner_sha256": runner_sha256,
        "protocol_sha256": protocol_sha256,
        "artifact_sha256": protocol["artifact_sha256"],
        "cells": result,
    }


def _reject_non_json_constant(value: str) -> None:
    raise ValueError(f"non-JSON numeric constant {value}")


def main() -> None:
    """Validate raw receipt against protocol and local artifacts, then write summary."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw", required=True, type=Path)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--artifacts-root", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--runner", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    raw_bytes = args.raw.read_bytes()
    protocol_bytes = args.protocol.read_bytes()
    raw = json.loads(raw_bytes, parse_constant=_reject_non_json_constant)
    protocol = json.loads(protocol_bytes, parse_constant=_reject_non_json_constant)
    result = validate(
        raw, protocol, args.artifacts_root,
        expected_source_sha=args.source_sha,
        protocol_sha256=_hash(protocol_bytes),
        runner_sha256=_hash(args.runner.read_bytes()),
    )
    protected = {args.raw.resolve(), args.protocol.resolve(), args.runner.resolve(),
                 Path(__file__).resolve()}
    protected.update((args.artifacts_root / name).resolve()
                     for name in protocol["artifact_sha256"])
    if args.output.resolve() in protected:
        raise ValueError("output path would overwrite a raw or bound input artifact")
    result["raw_sha256"] = _hash(raw_bytes)
    result["validator_sha256"] = _hash(Path(__file__).read_bytes())
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
