#!/usr/bin/env python3
"""Independently audit a source-bound installed Python capability receipt."""

from __future__ import annotations

import argparse
from collections import Counter
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import subprocess
from zipfile import ZipFile


def digest(data: bytes) -> str:
    """Hash exact bytes."""
    return hashlib.sha256(data).hexdigest()


def git_bytes(repo: Path, source: str, path: str) -> bytes:
    """Read a file from the declared source commit without using the checkout."""
    return subprocess.check_output(["git", "-C", str(repo), "show", f"{source}:{path}"])


def git_tree(repo: Path, source: str, path: str) -> str:
    """Read a tree identity from the declared source commit."""
    return subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", f"{source}:{path}"], text=True
    ).strip()


def audit_snapshot(snapshot: dict) -> None:
    """Recompute retained SQLite row counts, digest and active-body projection."""
    rows = snapshot["persisted_rows"]
    counts = {name: len(items) for name, items in rows.items()}
    if counts != snapshot["table_counts"]:
        raise ValueError("persisted table counts changed")
    restored = {name: [tuple(row) for row in items] for name, items in rows.items()}
    if digest(repr(restored).encode()) != snapshot["canonical_operational_sha256"]:
        raise ValueError("snapshot digest changed")
    active = sorted(
        [row[4], row[2]]
        for row in rows.get("canonical_nodes", [])
        if len(row) >= 9
        and row[4] is not None
        and row[5] is None
        and row[8] == "active"
    )
    if (
        active != snapshot["active_bodies"]
        or len(active) != snapshot["active_bodies_checked"]
    ):
        raise ValueError("active bodies differ from persisted canonical rows")
    if snapshot["missing_id"] is not None:
        raise ValueError("guaranteed absent ID was returned")


def audit_operations(
    live_ids: list[str], operations: dict, cases: dict, counts: dict
) -> None:
    """Recompute the governed operation partition and executed-case links."""
    if len(live_ids) != len(set(live_ids)) or set(operations) != set(live_ids):
        raise ValueError("governed operation IDs changed")
    statuses = Counter(row["status"] for row in operations.values())
    if set(statuses) - {"executed", "failed", "gap", "unavailable"}:
        raise ValueError("unknown operation status")
    computed = {
        "supported": len(live_ids),
        "executed": statuses["executed"],
        "failed": statuses["failed"],
        "gap": statuses["gap"],
        "unavailable": statuses["unavailable"],
        "unexecuted": len(live_ids) - statuses["executed"],
    }
    if computed != counts:
        raise ValueError("operation counts changed")
    for operation, row in operations.items():
        if row["status"] == "executed":
            positive = row["positive"]["case"]
            negative = row["negative"]["case"]
            if any(
                name not in cases or cases[name]["status"] != "passed"
                for name in (positive, negative)
            ):
                raise ValueError(f"{operation} missing or failed case")
            if (
                row["reopen"]["case"] != positive
                or row["reopen"]["checks"] != cases[positive]["reopen"]
            ):
                raise ValueError(f"{operation} reopened-state link changed")
        elif row["status"] in ("gap", "unavailable") and not row.get("reason"):
            raise ValueError(f"{operation} lacks a gap reason")


def audit(raw_path: Path, wheel: Path, repo: Path) -> dict:
    """Verify exact-source, archive, case and SQLite-observation evidence."""
    raw = json.loads(raw_path.read_text())
    if raw["schema_version"] != "fathomdb.slice135-python-capabilities/v1":
        raise ValueError("capability receipt schema changed")
    if raw["status"] != "INTERIM_INSTALLED_WHEEL_CAPABILITY_EXERCISE":
        raise ValueError("capability receipt status changed")
    source = raw["product_source_sha"]
    if raw["product_rust_tree"] != git_tree(repo, source, "src/rust") or raw[
        "product_python_tree"
    ] != git_tree(repo, source, "src/python/fathomdb"):
        raise ValueError("product source tree changed")
    bound = {
        "cargo_lock_sha256": "Cargo.lock",
        "runner_sha256": "scripts/slice135_python_capabilities.py",
        "s01_helper_sha256": "scripts/slice135_python_s01.py",
        "s02_helper_sha256": "scripts/slice135_python_s02.py",
        "operation_map_sha256": "src/conformance/governed-operation-parity.json",
    }
    for key, path in bound.items():
        if raw[key] != digest(git_bytes(repo, source, path)):
            raise ValueError(f"{key} differs from source")
    for path, expected in raw["oracle_fixtures_sha256"].items():
        if digest(git_bytes(repo, source, path)) != expected:
            raise ValueError(f"oracle fixture changed: {path}")
    if digest(wheel.read_bytes()) != raw["artifact"]["wheel_sha256"]:
        raise ValueError("wheel bytes changed")
    with ZipFile(wheel) as archive:
        names = set(archive.namelist())
        if set(raw["artifact"]["verified_members"]) != {
            name for name in names if name.startswith("fathomdb/")
        }:
            raise ValueError("wheel member inventory changed")
        if any(
            "__pycache__" in Path(name).parts or name.endswith((".pyc", ".pyo"))
            for name in names
        ):
            raise ValueError("stale bytecode in wheel")
        if (
            digest(archive.read("fathomdb/_fathomdb.abi3.so"))
            != raw["artifact"]["native_sha256"]
        ):
            raise ValueError("native bytes changed")
        for name in names:
            if name.startswith("fathomdb/") and name.endswith(".py"):
                if archive.read(name) != git_bytes(repo, source, f"src/python/{name}"):
                    raise ValueError(f"wheel Python source changed: {name}")
    operation_map = json.loads(git_bytes(repo, source, bound["operation_map_sha256"]))
    live = [row["id"] for row in operation_map["operations"] if row["state"] == "live"]
    cases = raw["cases"]
    if not cases or any(
        case["status"] != "passed" or not case.get("reopen") for case in cases.values()
    ):
        raise ValueError("capability case missing, failed or not reopened")
    snapshots = 0
    for case in cases.values():
        if "test" in case:
            path = case["test"].split("::", 1)[0]
            if digest(git_bytes(repo, source, path)) != case["test_sha256"]:
                raise ValueError(f"selected test source changed: {path}")
        for observation in case["reopen"]:
            audit_snapshot(observation)
            snapshots += 1
    audit_operations(live, raw["operations"], cases, raw["counts"])
    wrong_state = deepcopy(next(iter(cases.values()))["reopen"][0])
    wrong_state["table_counts"][next(iter(wrong_state["table_counts"]))] += 1
    try:
        audit_snapshot(wrong_state)
    except ValueError as error:
        state_negative = str(error)
    else:
        raise AssertionError("altered persisted count passed")
    wrong_counts = raw["counts"] | {"executed": raw["counts"]["executed"] + 1}
    try:
        audit_operations(live, raw["operations"], cases, wrong_counts)
    except ValueError as error:
        count_negative = str(error)
    else:
        raise AssertionError("false executed count passed")
    return {
        "status": "PASS",
        "product_source_sha": source,
        "raw_sha256": digest(raw_path.read_bytes()),
        "wheel_sha256": digest(wheel.read_bytes()),
        "native_sha256": raw["artifact"]["native_sha256"],
        "counts": raw["counts"],
        "cases_checked": len(cases),
        "reopened_databases_checked": snapshots,
        "oracle_fixtures_checked": len(raw["oracle_fixtures_sha256"]),
        "negative_controls_rejected": [state_negative, count_negative],
        "limits": "Retained SQLite snapshots are recomputed; original temporary databases were deleted. Selected source tests do not cover every accepted contract condition.",
    }


def main() -> None:
    """Audit one retained raw capability result and write a separate report."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("refusing to overwrite audit")
    result = audit(args.raw, args.wheel, args.repo)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"status": result["status"], "counts": result["counts"]}))


if __name__ == "__main__":
    main()
