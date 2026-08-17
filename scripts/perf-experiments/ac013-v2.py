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
from statistics import median
from typing import Any

PROTOCOL = "0.8.23-scale-characterization-v2"
ROWS = (10_000, 100_000, 1_000_000)
TREATMENTS = ("process_cold", "warm")
REPETITIONS = (1, 2, 3, 4, 5)
ROOT_NAME = re.compile(r"0\.8\.23-scale-([0-9a-f]{40})-raw\Z")
RECORD_KEYS = (
    "treatment", "n", "seed_write_ms", "embedding_ms", "projection_drain_ms",
    "accepted_writes", "vector_rows_after_drain", "drain_outcome", "samples_us",
    "result_counts", "query_errors", "query_timeouts", "query_skips",
    "query_invariant_failures",
)


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


def number(value: str, field: str) -> int:
    if not value.isascii() or not value.isdecimal():
        raise SystemExit(f"AC013 record {field} must be a nonnegative integer")
    return int(value)


def number_list(value: str, field: str, expected: int) -> list[int]:
    values = [number(part, field) for part in value.split(",")]
    if len(values) != expected:
        raise SystemExit(f"AC013 record {field} requires exactly {expected} values")
    return values


def parse_record(path: Path, rows: int, treatment: str) -> dict[str, Any]:
    try:
        lines = [line.rstrip("\n") for line in path.read_text(encoding="utf-8").splitlines() if line.startswith("AC013_TREATMENT_RECORD ")]
    except UnicodeDecodeError as error:
        raise SystemExit(f"AC013 record log is not UTF-8: {path.name}") from error
    if len(lines) != 1:
        raise SystemExit(f"AC013 record requires exactly one treatment line: {path.name}")
    parts = lines[0].split(" ")[1:]
    if len(parts) != len(RECORD_KEYS) or any("=" not in part for part in parts):
        raise SystemExit(f"AC013 record has malformed fields: {path.name}")
    keys = tuple(part.split("=", 1)[0] for part in parts)
    if keys != RECORD_KEYS:
        raise SystemExit(f"AC013 record fields are unknown, duplicate, or out of order: {path.name}")
    raw = {key: part.split("=", 1)[1] for key, part in zip(keys, parts, strict=True)}
    if raw["treatment"] != treatment or number(raw["n"], "n") != rows:
        raise SystemExit(f"AC013 record tuple disagrees with filename: {path.name}")
    if raw["embedding_ms"] != "not_separately_observable":
        number(raw["embedding_ms"], "embedding_ms")
    record: dict[str, Any] = {
        "treatment": treatment,
        "n": rows,
        "seed_write_ms": number(raw["seed_write_ms"], "seed_write_ms"),
        "embedding_ms": raw["embedding_ms"] if raw["embedding_ms"] == "not_separately_observable" else number(raw["embedding_ms"], "embedding_ms"),
        "projection_drain_ms": number(raw["projection_drain_ms"], "projection_drain_ms"),
        "accepted_writes": number(raw["accepted_writes"], "accepted_writes"),
        "vector_rows_after_drain": number(raw["vector_rows_after_drain"], "vector_rows_after_drain"),
        "drain_outcome": raw["drain_outcome"],
        "samples_us": number_list(raw["samples_us"], "samples_us", 1 if treatment == "process_cold" else 1000),
        "result_counts": number_list(raw["result_counts"], "result_counts", 1 if treatment == "process_cold" else 1000),
        "query_errors": number(raw["query_errors"], "query_errors"),
        "query_timeouts": number(raw["query_timeouts"], "query_timeouts"),
        "query_skips": number(raw["query_skips"], "query_skips"),
        "query_invariant_failures": number(raw["query_invariant_failures"], "query_invariant_failures"),
    }
    if record["accepted_writes"] != rows or record["vector_rows_after_drain"] != rows or record["drain_outcome"] != "ok":
        raise SystemExit(f"AC013 record write/drain invariant failed: {path.name}")
    if any(record[field] != 0 for field in ("query_errors", "query_timeouts", "query_skips", "query_invariant_failures")):
        raise SystemExit(f"AC013 record query counter is nonzero: {path.name}")
    return record


def statistics(samples: list[int], treatment: str) -> dict[str, Any]:
    ordered = sorted(samples)
    values: dict[str, Any] = {"min_us": ordered[0], "max_us": ordered[-1], "mean_us": sum(ordered) / len(ordered)}
    if treatment == "warm":
        for percentile in (50, 95, 99):
            values[f"p{percentile}_us"] = ordered[(len(ordered) - 1) * percentile // 100]
    return values


def manifest_entry(root: Path, item: tuple[int, str, int], command_exit_status: int = 0) -> dict[str, Any]:
    rows, treatment, repetition = item
    log = log_name(*item)
    entry: dict[str, Any] = {"rows": rows, "treatment": treatment, "repetition": repetition, "log": log, "raw_sha256": digest(root / log), "command_exit_status": command_exit_status}
    if command_exit_status == 0:
        entry["sidecar"] = f"{log}.sha256"
        entry["record"] = parse_record(root / log, rows, treatment)
    return entry


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
        entry = manifest_entry(root, item, child_status)
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
    partial = root / "partial-manifest.json"
    if partial.exists():
        if not partial.is_file() or partial.is_symlink():
            raise SystemExit("V2 partial manifest is not a regular file")
        partial.unlink()
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
        entries.append(manifest_entry(root, (rows, treatment, repetition)))
    write_json(root / "matrix-manifest.json", {"schema_version": 2, "protocol": PROTOCOL, "candidate_head_sha": candidate, "entries": entries})
    closed = {path.name for path in root.iterdir()}
    if len(closed) != 62 or closed != required | {"matrix-manifest.json"}:
        raise SystemExit("V2 root did not achieve exact 62-file closure")


def root_reference(root: Path) -> str:
    if not is_test():
        return root.relative_to(repository()).as_posix() + "/"
    return f"dev/plans/runs/{root.name}/"


def status_path(root: Path) -> Path:
    return root.with_name(f"{root.name}.status.json")


def partial_manifest(root: Path) -> dict[str, Any]:
    try:
        value = json.loads((root / "partial-manifest.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"cannot read V2 partial manifest: {error}") from error
    if not isinstance(value, dict):
        raise SystemExit("V2 partial manifest must be an object")
    return value


def cells(entries: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[tuple[int, str], list[dict[str, Any]]] = {}
    for entry in entries:
        if entry["command_exit_status"] != 0:
            continue
        grouped.setdefault((entry["rows"], entry["treatment"]), []).append(entry)
    result = []
    for (rows, treatment), group in sorted(grouped.items()):
        repetitions = []
        for entry in sorted(group, key=lambda item: item["repetition"]):
            record = entry["record"]
            repetitions.append({
                "repetition": entry["repetition"],
                "raw": {"log": entry["log"], "sidecar": entry["sidecar"], "sha256": entry["raw_sha256"]},
                "command_exit_status": 0,
                "record": record,
                "query_statistics": statistics(record["samples_us"], treatment),
            })
        result.append({"rows": rows, "treatment": treatment, "repetitions": repetitions})
    return result


def summarize(matrix: list[dict[str, Any]]) -> dict[str, Any]:
    summary = []
    for cell in matrix:
        treatment = cell["treatment"]
        keys = ("min_us", "max_us", "mean_us")
        if treatment == "warm":
            keys += ("p50_us", "p95_us", "p99_us")
        repetitions = cell["repetitions"]
        level = {}
        for key in keys:
            values = [repetition["query_statistics"][key] for repetition in repetitions]
            level[key] = {"median": median(values), "min": min(values), "max": max(values)}
        summary.append({
            "rows": cell["rows"],
            "treatment": treatment,
            "repetition_count": len(repetitions),
            "repetition_level": level,
        })
    return {"cells": summary}


def validate_artifact(artifact: dict[str, Any]) -> None:
    try:
        import jsonschema
    except ImportError as error:
        raise SystemExit(f"jsonschema is required for V2 status artifacts: {error}") from error
    schema = json.loads((repository() / "dev/design/0.8.23-scale-artifact-v2.schema.json").read_text(encoding="utf-8"))
    try:
        jsonschema.Draft202012Validator(schema).validate(artifact)
    except jsonschema.ValidationError as error:
        raise SystemExit(f"invalid V2 status artifact: {error.message}") from error


def emit_status(root: Path, status: str, reason: str) -> None:
    manifest = partial_manifest(root)
    mode = manifest.get("execution_mode")
    if status == "ENVIRONMENT_INVALID" and mode != "failed_child":
        raise SystemExit("ENVIRONMENT_INVALID status requires a failed-child partial root")
    if status == "INSUFFICIENT_SAMPLES" and mode != "partial_retained":
        raise SystemExit("INSUFFICIENT_SAMPLES status requires a retained partial root")
    successful = [entry for entry in manifest["attempted_entries"] if entry["command_exit_status"] == 0]
    try:
        provenance = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"cannot read V2 provenance for status artifact: {error}") from error
    artifact = {
        "schema_version": 2,
        "protocol": PROTOCOL,
        "status": status,
        "claim_scope": "fixture_scoped_non_ann_two_phase_vector_characterization",
        "input": {"kind": "partial", "root": root_reference(root), "partial_manifest": manifest},
        "provenance": provenance,
        "matrix": [] if status == "ENVIRONMENT_INVALID" else cells(successful),
        "integrity": {"raw_log_digests": successful, "invalid_reasons": [reason] if status == "ENVIRONMENT_INVALID" else [], "missing_prerequisites": []},
        "summary": None,
        "next_step": reason,
    }
    validate_artifact(artifact)
    write_json(status_path(root), artifact)


def validate_root(root: Path, allow_test_fixture: bool) -> None:
    candidate = candidate_from(root)
    match = ROOT_NAME.fullmatch(root.name)
    if match is None or (not allow_test_fixture and match.group(1) != candidate):
        raise SystemExit("V2 root candidate SHA does not agree with root name")
    try:
        provenance = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"cannot read V2 root provenance: {error}") from error
    validate_provenance(provenance, candidate)
    logs = [log_name(*item) for item in matrix_tuples()]
    required = {"provenance.json", "matrix-manifest.json", *logs, *(f"{log}.sha256" for log in logs)}
    found = {path.name for path in root.iterdir()}
    if len(found) != 62 or found != required or any(not path.is_file() or path.is_symlink() for path in root.iterdir()):
        raise SystemExit("V2 root is not the exact 62-file regular closure")
    try:
        manifest = json.loads((root / "matrix-manifest.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"cannot read V2 matrix manifest: {error}") from error
    if manifest.get("candidate_head_sha") != candidate or manifest.get("protocol") != PROTOCOL or manifest.get("schema_version") != 2:
        raise SystemExit("V2 manifest provenance disagrees with root")
    entries = manifest.get("entries")
    if not isinstance(entries, list) or len(entries) != 30:
        raise SystemExit("V2 manifest does not contain 30 entries")
    for item, entry in zip(matrix_tuples(), entries, strict=True):
        rows, treatment, repetition = item
        log = log_name(*item)
        if entry.get("rows") != rows or entry.get("treatment") != treatment or entry.get("repetition") != repetition or entry.get("log") != log or entry.get("sidecar") != f"{log}.sha256" or entry.get("raw_sha256") != digest(root / log) or entry.get("command_exit_status") != 0:
            raise SystemExit(f"V2 manifest tuple does not agree with root: {log}")
        if not valid_sidecar(root, log) or entry.get("record") != parse_record(root / log, rows, treatment):
            raise SystemExit(f"V2 record or sidecar invalid: {log}")


def emit_complete(root: Path) -> None:
    validate_root(root, is_test())
    candidate = candidate_from(root)
    try:
        provenance = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
        manifest = json.loads((root / "matrix-manifest.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"cannot read V2 complete root: {error}") from error
    entries = manifest["entries"]
    matrix = cells(entries)
    artifact = {
        "schema_version": 2,
        "protocol": PROTOCOL,
        "status": "CHARACTERIZED",
        "claim_scope": "fixture_scoped_non_ann_two_phase_vector_characterization",
        "input": {
            "kind": "complete",
            "root": root_reference(root),
            "manifest": manifest,
            "regular_file_count": 62,
        },
        "provenance": provenance,
        "matrix": matrix,
        "integrity": {"raw_log_digests": entries, "invalid_reasons": [], "missing_prerequisites": []},
        "summary": summarize(matrix),
        "next_step": "review fixture-scoped characterization; do not infer a supported-scale claim",
    }
    if artifact["input"]["manifest"]["candidate_head_sha"] != candidate:
        raise SystemExit("V2 complete manifest candidate disagrees with root")
    validate_artifact(artifact)
    write_json(status_path(root), artifact)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("begin", "partial", "seal", "validate-record", "emit-status", "validate-root", "emit-complete"))
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--failed-rows", type=int)
    parser.add_argument("--failed-treatment", choices=TREATMENTS)
    parser.add_argument("--failed-repetition", type=int, choices=REPETITIONS)
    parser.add_argument("--exit-status", type=int, default=0)
    parser.add_argument("--rows", type=int, choices=ROWS)
    parser.add_argument("--treatment", choices=TREATMENTS)
    parser.add_argument("--log", type=Path)
    parser.add_argument("--status", choices=("ENVIRONMENT_INVALID", "INSUFFICIENT_SAMPLES"))
    parser.add_argument("--reason")
    parser.add_argument("--test-fixture", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    if args.command == "begin":
        begin(root)
    elif args.command == "seal":
        seal(root)
    elif args.command == "validate-record":
        if args.rows is None or args.treatment is None or args.log is None:
            raise SystemExit("validate-record requires --rows, --treatment, and --log")
        print(json.dumps(parse_record(args.log, args.rows, args.treatment), sort_keys=True, separators=(",", ":")))
    elif args.command == "emit-status":
        if args.status is None or not args.reason:
            raise SystemExit("emit-status requires --status and --reason")
        emit_status(root, args.status, args.reason)
    elif args.command == "validate-root":
        validate_root(root, args.test_fixture)
    elif args.command == "emit-complete":
        emit_complete(root)
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
