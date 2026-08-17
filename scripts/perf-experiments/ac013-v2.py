#!/usr/bin/env python3
"""Durably create and seal AC-013 V2 matrix roots (never a measurement)."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any

PROTOCOL = "0.8.23-scale-characterization-v2"
ROWS = (10_000, 100_000, 1_000_000)
TREATMENTS = ("process_cold", "warm")
REPETITIONS = (1, 2, 3, 4, 5)
ROOT_NAME = re.compile(r"0\.8\.23-scale-([0-9a-f]{40})-raw\Z")


def git(*args: str) -> str:
    return subprocess.check_output(("git", *args), text=True).strip()


def repository() -> Path:
    return Path(git("rev-parse", "--show-toplevel")).resolve()


def head() -> str:
    return git("rev-parse", "HEAD")


def is_test() -> bool:
    return os.environ.get("AC013_V2_TEST_MODE") == "1"


def matrix_tuples() -> list[tuple[int, str, int]]:
    return [(rows, treatment, repetition) for rows in ROWS for treatment in TREATMENTS for repetition in REPETITIONS]


def log_name(rows: int, treatment: str, repetition: int) -> str:
    return f"ac013-{rows}-{treatment}-rep{repetition}.log"


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def write_json(path: Path, value: object) -> None:
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
    temporary.replace(path)


def required_root(root: Path, candidate: str) -> None:
    if is_test():
        return
    match = ROOT_NAME.fullmatch(root.name)
    if match is None:
        raise SystemExit(f"V2 root has a noncanonical basename: {root}")
    expected = repository() / "dev/plans/runs" / f"0.8.23-scale-{candidate}-raw"
    if root != expected or match.group(1) != candidate:
        raise SystemExit(f"V2 root must be exactly {expected}")
    if git("status", "--porcelain", "--untracked-files=normal"):
        raise SystemExit("V2 candidate worktree must be clean before root creation")


def fixture_provenance(candidate: str) -> dict[str, Any]:
    """Schema-shaped fixture solely for the behavioral fake-runner test."""
    h = "a" * 64
    return {
        "availability": "complete",
        "candidate": {"head_sha": candidate, "ref": "test-fixture", "commit_timestamp": "2026-08-17T00:00:00Z", "git_status_porcelain": "", "worktree_clean": True, "schema_version": 0},
        "slice5_provenance": {
            "release": "0.8.22",
            "authoritative_release_state": {"path": "dev/plans/release-state-0.8.22.json", "git_blob": candidate, "sha256": h, "slice": 5, "status": "LANDED", "landed_sha": candidate, "evidence_sha256": h},
            "measured_lock_stack": {
                "cargo_lock_sha256": h,
                "rusqlite": {"name": "rusqlite", "version": "test", "source": "test", "checksum": h},
                "sqlite_vec": {"name": "sqlite-vec", "version": "test", "source": "test", "checksum": h},
                "libsqlite3_sys": {"name": "libsqlite3-sys", "version": "test", "source": "test", "checksum": h},
            },
        },
        "lock_build": {"cargo_lock_git_blob": candidate, "cargo_lock_sha256": h, "cargo_metadata_locked_sha256": h, "schema_version": 0, "schema_source_git_blob": candidate, "rustc_vv": "test", "cargo_v": "test", "target": "test", "profile": "release", "enabled_features": [], "command": "test fixture only", "built_process_sqlite_version": "test"},
        "fixture": {"id": "ac013-deterministic-vector-v1", "generator": "seed_ac013_corpus", "corpus_seed": "0x0AC0_13D0_13D0", "query_generator": "ac013_query_bodies", "query_seed": "0x0AC0_130D_EC0D_E000", "source_identity": "test:fixture", "vector_dimension": 384, "query_count": 1000, "scale_points": [10000, 100000, 1000000], "perf_gates_git_blob": candidate, "descriptor_sha256": h},
        "environment": {"host": {"collect_host_spec_sha256": h, "total_ram_bytes": 1, "available_ram_bytes": 1, "database_filesystem": "test", "database_free_bytes": 1, "cpu_governor_frequency_policy": "test"}, "gpu": {"state": "not_present_or_unavailable"}, "isolation": {"timestamp": "2026-08-17T00:00:00Z", "load_evidence_sha256": h, "process_inventory_sha256": h, "cpu_affinity": "test", "governor_policy": "test", "power_policy": "test", "container_or_vm": "test", "no_concurrent_benchmark": True}},
    }


def validate_provenance(provenance: object, candidate: str) -> dict[str, Any]:
    if not isinstance(provenance, dict):
        raise SystemExit("V2 provenance must be a JSON object")
    try:
        import jsonschema
    except ImportError as error:
        raise SystemExit(f"jsonschema is required for complete V2 provenance: {error}") from error
    schema = json.loads((repository() / "dev/design/0.8.23-scale-artifact-v2.schema.json").read_text(encoding="utf-8"))
    sub_schema = {"$schema": schema["$schema"], "$defs": schema["$defs"], "$ref": "#/$defs/complete_provenance"}
    try:
        jsonschema.Draft202012Validator(sub_schema).validate(provenance)
    except jsonschema.ValidationError as error:
        raise SystemExit(f"invalid complete V2 provenance: {error.message}") from error
    captured = provenance["candidate"]
    if captured["head_sha"] != candidate or captured["git_status_porcelain"] != "" or captured["worktree_clean"] is not True:
        raise SystemExit("V2 provenance does not attest this clean candidate")
    return provenance


def begin(root: Path) -> None:
    candidate = head()
    required_root(root, candidate)
    if root.exists():
        raise SystemExit(f"refusing to reuse matrix output directory: {root}")
    if is_test():
        provenance = fixture_provenance(candidate)
    else:
        capture = os.environ.get("AC013_V2_PROVENANCE_FILE")
        if not capture:
            raise SystemExit("AC013_V2_PROVENANCE_FILE is required before any V2 timing child")
        path = Path(capture)
        if not path.is_file() or path.is_symlink():
            raise SystemExit("AC013_V2_PROVENANCE_FILE must name a regular file")
        try:
            provenance = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as error:
            raise SystemExit(f"V2 provenance is not JSON: {error}") from error
    provenance = validate_provenance(provenance, candidate)
    root.mkdir(parents=True)
    write_json(root / "provenance.json", provenance)


def candidate_from(root: Path) -> str:
    try:
        loaded = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
        return loaded["candidate"]["head_sha"]
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise SystemExit(f"cannot read V2 root provenance: {error}") from error


def valid_sidecar(root: Path, log: str) -> bool:
    sidecar = root / f"{log}.sha256"
    if not sidecar.is_file() or sidecar.is_symlink():
        return False
    try:
        words = sidecar.read_text(encoding="utf-8").split()
    except OSError:
        return False
    return len(words) == 2 and words[1] == log and words[0] == digest(root / log)


def write_partial(root: Path, failed: tuple[int, str, int] | None, status: int) -> None:
    candidate = candidate_from(root)
    all_tuples = matrix_tuples()
    attempted: list[dict[str, Any]] = []
    for index, item in enumerate(all_tuples):
        rows, treatment, repetition = item
        log = log_name(*item)
        raw = root / log
        if not raw.exists():
            break
        if not raw.is_file() or raw.is_symlink():
            raise SystemExit(f"partial V2 root has nonregular log: {log}")
        child_status = status if item == failed else 0
        if child_status == 0 and not valid_sidecar(root, log):
            raise SystemExit(f"successful child lacks a valid basename sidecar: {log}")
        entry: dict[str, Any] = {"rows": rows, "treatment": treatment, "repetition": repetition, "log": log, "raw_sha256": digest(raw), "command_exit_status": child_status}
        if child_status == 0:
            entry["sidecar"] = f"{log}.sha256"
        attempted.append(entry)
        if child_status != 0:
            if any((root / log_name(*later)).exists() for later in all_tuples[index + 1 :]):
                raise SystemExit("a V2 child ran after a failed child")
            break
    present = {path.name for path in root.glob("ac013-*.log")}
    expected_present = {entry["log"] for entry in attempted}
    if present != expected_present:
        raise SystemExit("partial V2 logs are not an ordered Cartesian prefix")
    if not attempted:
        raise SystemExit("partial V2 manifest requires an attempted child")
    if failed is not None and tuple(attempted[-1][key] for key in ("rows", "treatment", "repetition")) != failed:
        raise SystemExit("failed V2 child did not leave its expected raw log")
    seen = {(entry["rows"], entry["treatment"], entry["repetition"]) for entry in attempted}
    omitted = [{"rows": r, "treatment": t, "repetition": n} for r, t, n in all_tuples if (r, t, n) not in seen]
    write_json(root / "partial-manifest.json", {"schema_version": 2, "protocol": PROTOCOL, "candidate_head_sha": candidate, "execution_mode": "failed_child" if failed else "partial_retained", "attempted_entries": attempted, "unrun_repetitions": omitted})


def seal(root: Path) -> None:
    candidate = candidate_from(root)
    logs = [log_name(*item) for item in matrix_tuples()]
    required = {"provenance.json", *logs, *(f"{log}.sha256" for log in logs)}
    paths = list(root.iterdir())
    if any(not path.is_file() or path.is_symlink() for path in paths):
        raise SystemExit("V2 root must contain regular non-symlink files only")
    found = {path.name for path in paths}
    if found != required:
        raise SystemExit(f"refusing to seal nonclosed V2 root: missing={sorted(required - found)} extra={sorted(found - required)}")
    entries = []
    for rows, treatment, repetition in matrix_tuples():
        log = log_name(rows, treatment, repetition)
        if not valid_sidecar(root, log):
            raise SystemExit(f"invalid basename-only sidecar: {log}.sha256")
        entries.append({"rows": rows, "treatment": treatment, "repetition": repetition, "log": log, "sidecar": f"{log}.sha256", "raw_sha256": digest(root / log), "command_exit_status": 0})
    write_json(root / "matrix-manifest.json", {"schema_version": 2, "protocol": PROTOCOL, "candidate_head_sha": candidate, "entries": entries})
    closed = {path.name for path in root.iterdir()}
    if len(closed) != 62 or closed != required | {"matrix-manifest.json"}:
        raise SystemExit("V2 root did not achieve exact 62-file closure")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("begin", "partial", "seal"))
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--failed-rows", type=int)
    parser.add_argument("--failed-treatment", choices=TREATMENTS)
    parser.add_argument("--failed-repetition", type=int, choices=REPETITIONS)
    parser.add_argument("--exit-status", type=int, default=0)
    args = parser.parse_args()
    root = args.root.resolve()
    if args.command == "begin":
        begin(root)
    elif args.command == "seal":
        seal(root)
    else:
        values = (args.failed_rows, args.failed_treatment, args.failed_repetition)
        if any(value is not None for value in values) and any(value is None for value in values):
            raise SystemExit("failed child tuple must be complete")
        failed = None if all(value is None for value in values) else values
        if failed and args.exit_status == 0:
            raise SystemExit("failed child must have a nonzero exit status")
        write_partial(root, failed, args.exit_status)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
