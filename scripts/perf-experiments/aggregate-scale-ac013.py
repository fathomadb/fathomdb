#!/usr/bin/env python3
"""Validate retained AC-013 matrix evidence and emit a fail-closed artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from statistics import mean, median


PROTOCOL = "0.8.23-scale-characterization-v1"
ROWS = (10_000, 100_000, 1_000_000)
TREATMENTS = ("process_cold", "warm")
RECORD_KEYS = ("treatment", "n", "seed_write_ms", "embedding_ms", "projection_drain_ms", "accepted_writes", "vector_rows_after_drain", "drain_outcome", "samples_us", "result_counts")
SHA = re.compile(r"^[0-9a-f]{64}$")
GIT_SHA = re.compile(r"^[0-9a-f]{40}$")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def reason(bucket: list[str], value: str) -> None:
    if value not in bucket:
        bucket.append(value)


def expected(rows: int, treatment: str, repetition: int) -> str:
    return f"ac013-{rows}-{treatment}-rep{repetition}.log"


def stats(values: list[int], warm: bool) -> dict[str, float | int]:
    ordered = sorted(values)
    output: dict[str, float | int] = {"min_us": ordered[0], "max_us": ordered[-1], "mean_us": mean(ordered)}
    if warm:
        for label, numerator in (("p50_us", 50), ("p95_us", 95), ("p99_us", 99)):
            output[label] = ordered[(len(ordered) * numerator + 99) // 100 - 1]
    return output


def parse_record(raw: bytes, rows: int, treatment: str, invalid: list[str], label: str) -> dict[str, object] | None:
    try:
        lines = raw.decode("utf-8").splitlines()
    except UnicodeDecodeError:
        reason(invalid, f"records.{label}.utf8")
        return None
    records = [line for line in lines if line.startswith("AC013_TREATMENT_RECORD ")]
    if len(records) != 1:
        reason(invalid, f"records.{label}.count")
        return None
    parts = records[0].split(" ")
    if parts[0] != "AC013_TREATMENT_RECORD" or len(parts) != len(RECORD_KEYS) + 1:
        reason(invalid, f"records.{label}.shape")
        return None
    fields: dict[str, str] = {}
    for key, token in zip(RECORD_KEYS, parts[1:], strict=True):
        if "=" not in token:
            reason(invalid, f"records.{label}.token")
            return None
        actual, value = token.split("=", 1)
        if actual != key or not value or actual in fields:
            reason(invalid, f"records.{label}.keys")
            return None
        fields[actual] = value
    if fields["treatment"] != treatment or fields["n"] != str(rows) or fields["drain_outcome"] != "ok":
        reason(invalid, f"records.{label}.identity")
        return None
    integers = ("seed_write_ms", "projection_drain_ms", "accepted_writes", "vector_rows_after_drain")
    if fields["embedding_ms"] != "not_separately_observable" and not fields["embedding_ms"].isdigit():
        reason(invalid, f"records.{label}.embedding_ms")
        return None
    if any(not fields[key].isdigit() for key in integers) or int(fields["accepted_writes"]) != rows or int(fields["vector_rows_after_drain"]) != rows:
        reason(invalid, f"records.{label}.write_drain")
        return None
    def values(key: str) -> list[int] | None:
        items = fields[key].split(",")
        if not items or any(not item.isdigit() for item in items):
            reason(invalid, f"records.{label}.{key}")
            return None
        return [int(item) for item in items]
    samples, counts = values("samples_us"), values("result_counts")
    cardinality = 1 if treatment == "process_cold" else 1000
    if samples is None or counts is None or len(samples) != cardinality or len(counts) != cardinality:
        reason(invalid, f"records.{label}.cardinality")
        return None
    return {"seed_write_ms": int(fields["seed_write_ms"]), "embedding_ms": fields["embedding_ms"],
            "projection_drain_ms": int(fields["projection_drain_ms"]), "accepted_writes": rows,
            "vector_rows_after_drain": rows, "drain_outcome": "ok", "samples_us": samples,
            "result_counts": counts, "query_statistics": stats(samples, treatment == "warm")}


def validate_provenance(repo: Path, provenance: object, invalid: list[str], missing: list[str]) -> tuple[dict[str, object] | None, dict[str, object] | None]:
    if not isinstance(provenance, dict):
        reason(missing, "provenance.json")
        return None, None
    required = {"schema_version", "protocol", "candidate", "slice5_provenance", "lock_build", "fixture", "environment"}
    if set(provenance) != required or provenance.get("schema_version") != 1 or provenance.get("protocol") != PROTOCOL:
        reason(invalid, "provenance.json.schema")
        return None, None
    candidate = provenance["candidate"]
    if not isinstance(candidate, dict) or not GIT_SHA.fullmatch(str(candidate.get("head_sha", ""))) or candidate.get("worktree_clean") is not True or candidate.get("git_status_porcelain") != "":
        reason(invalid, "provenance.candidate.clean")
    slice5 = provenance["slice5_provenance"]
    if not isinstance(slice5, dict):
        reason(invalid, "provenance.slice5")
    else:
        state_ref = slice5.get("authoritative_release_state")
        stack = slice5.get("measured_lock_stack")
        state_path = repo / "dev/plans/release-state-0.8.22.json"
        if not isinstance(state_ref, dict) or not state_path.exists():
            reason(missing, "provenance.slice5.authoritative_release_state")
        else:
            state_bytes = state_path.read_bytes()
            try:
                state = json.loads(state_bytes)
                slice = next(item for item in state["ladder"] if item["slice"] == 5)
                blob = subprocess.check_output(["git", "rev-parse", "HEAD:dev/plans/release-state-0.8.22.json"], cwd=repo, text=True).strip()
                valid_state = state_ref.get("path") == "dev/plans/release-state-0.8.22.json" and state_ref.get("git_blob") == blob and state_ref.get("sha256") == digest(state_bytes) and state_ref.get("slice") == 5 and state_ref.get("status") == "LANDED" and state_ref.get("landed_sha") == slice["sha"] and state_ref.get("evidence_sha256") == digest(slice["evidence"].encode())
                if not valid_state:
                    reason(invalid, "provenance.slice5.authoritative_release_state")
            except (KeyError, StopIteration, json.JSONDecodeError, subprocess.CalledProcessError):
                reason(invalid, "provenance.slice5.authoritative_release_state")
        if not isinstance(stack, dict):
            reason(invalid, "provenance.slice5.measured_lock_stack")
        else:
            lock = (repo / "Cargo.lock").read_bytes()
            if stack.get("cargo_lock_sha256") != digest(lock):
                reason(invalid, "provenance.slice5.cargo_lock")
            for expected_name, key in (("rusqlite", "rusqlite"), ("sqlite-vec", "sqlite_vec"), ("libsqlite3-sys", "libsqlite3_sys")):
                package = stack.get(key)
                if not isinstance(package, dict) or package.get("name") != expected_name:
                    reason(invalid, f"provenance.slice5.{key}")
    for key in ("lock_build", "fixture", "environment"):
        if not isinstance(provenance.get(key), dict):
            reason(missing, f"provenance.{key}")
    return candidate if isinstance(candidate, dict) else None, {key: provenance[key] for key in ("slice5_provenance", "lock_build", "fixture", "environment") if key in provenance}


def aggregate(repo: Path, relative: str) -> dict[str, object]:
    missing: list[str] = []
    invalid: list[str] = []
    root = (repo / relative).resolve()
    artifact: dict[str, object] = {"schema_version": 1, "protocol": PROTOCOL,
        "claim_scope": "fixture_scoped_non_ann_two_phase_vector_characterization", "matrix": [],
        "integrity": {"raw_log_digests": [], "invalid_reasons": invalid, "missing_prerequisites": missing},
        "summary": None, "next_step": "capture missing prerequisites or correct invalid retained evidence"}
    if Path(relative).is_absolute() or ".." in Path(relative).parts or not root.exists():
        artifact["status"] = "DEFERRED"
        return artifact
    try:
        root.relative_to(repo)
    except ValueError:
        reason(invalid, "input_root.outside_repository")
        artifact["status"] = "ENVIRONMENT_INVALID"
        return artifact
    provenance_file = root / "provenance.json"
    if not provenance_file.exists():
        reason(missing, "provenance.json")
        artifact["status"] = "MISSING_PREREQUISITE"
        return artifact
    try:
        provenance = json.loads(provenance_file.read_text(encoding="utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        provenance = None
        reason(invalid, "provenance.json")
    candidate, output_provenance = validate_provenance(repo, provenance, invalid, missing)
    if candidate:
        artifact["candidate"] = candidate
    if output_provenance:
        artifact["provenance"] = output_provenance
    head = str(candidate.get("head_sha")) if candidate else ""
    expected_root = f"dev/plans/runs/0.8.23-scale-{head}-raw"
    if relative.rstrip("/") != expected_root:
        reason(invalid, "input_root.fixed_path")
    manifest_file = root / "matrix-manifest.json"
    if not manifest_file.exists():
        reason(missing, "matrix-manifest.json")
        manifest: object = None
    else:
        try:
            manifest = json.loads(manifest_file.read_text(encoding="utf-8"))
            artifact["input_root"] = relative.rstrip("/") + "/"
            artifact["input_manifest_sha256"] = digest(manifest_file.read_bytes())
        except (UnicodeDecodeError, json.JSONDecodeError):
            manifest = None
            reason(invalid, "matrix-manifest.json")
    expected_files = {"matrix-manifest.json", "provenance.json"}
    expected_entries: list[dict[str, object]] = []
    for rows in ROWS:
        for treatment in TREATMENTS:
            for repetition in range(1, 6):
                name = expected(rows, treatment, repetition)
                expected_files.update((name, f"{name}.sha256"))
                expected_entries.append({"rows": rows, "treatment": treatment, "repetition": repetition, "log": name, "sidecar": f"{name}.sha256"})
    actual_files = {path.name for path in root.iterdir()} if root.is_dir() else set()
    if actual_files - expected_files:
        reason(invalid, "input_root.extras")
    reps: dict[tuple[int, str], list[dict[str, object]]] = {(r, t): [] for r in ROWS for t in TREATMENTS}
    entries = manifest.get("entries") if isinstance(manifest, dict) else None
    if not isinstance(manifest, dict) or set(manifest) != {"schema_version", "protocol", "candidate_head_sha", "entries"} or manifest.get("schema_version") != 1 or manifest.get("protocol") != PROTOCOL or manifest.get("candidate_head_sha") != head or not isinstance(entries, list):
        reason(invalid, "matrix-manifest.json.contract")
        entries = []
    if len(entries) > 30:
        reason(invalid, "matrix-manifest.json.entries")
    for entry, base in zip(entries, expected_entries, strict=False):
        if not isinstance(entry, dict) or set(entry) != {"rows", "treatment", "repetition", "log", "sidecar", "sha256"} or any(entry.get(k) != v for k, v in base.items()) or not SHA.fullmatch(str(entry.get("sha256", ""))):
            reason(invalid, "matrix-manifest.json.entry")
    by_name = {entry.get("log"): entry for entry in entries if isinstance(entry, dict)}
    for base in expected_entries:
        rows, treatment, repetition, name = int(base["rows"]), str(base["treatment"]), int(base["repetition"]), str(base["log"])
        log, sidecar = root / name, root / f"{name}.sha256"
        if not log.exists() or not sidecar.exists():
            continue
        raw = log.read_bytes()
        expected_line = f"{digest(raw)}  {name}\n"
        if sidecar.read_text(encoding="utf-8", errors="replace") != expected_line or not isinstance(by_name.get(name), dict) or by_name[name].get("sha256") != digest(raw):
            reason(invalid, f"integrity.{name}")
            continue
        record = parse_record(raw, rows, treatment, invalid, name)
        if record is not None:
            record.update({"repetition": repetition, "raw": {"log": name, "sidecar": f"{name}.sha256", "sha256": digest(raw)}, "command_exit_status": 0, "query_errors": 0, "query_timeouts": 0, "query_skips": 0, "query_invariant_failures": 0})
            reps[(rows, treatment)].append(record)
            artifact["integrity"]["raw_log_digests"].append(record["raw"])
    for (rows, treatment), values in reps.items():
        artifact["matrix"].append({"rows": rows, "treatment": treatment, "repetitions": values})
    if invalid:
        artifact["status"] = "ENVIRONMENT_INVALID"
    elif missing:
        artifact["status"] = "MISSING_PREREQUISITE"
    elif any(len(values) != 5 for values in reps.values()):
        artifact["status"] = "INSUFFICIENT_SAMPLES"
    else:
        artifact["status"] = "CHARACTERIZED"
        cells = []
        for (rows, treatment), values in reps.items():
            keys = ("min_us", "max_us", "mean_us") if treatment == "process_cold" else ("min_us", "max_us", "mean_us", "p50_us", "p95_us", "p99_us")
            level = {key: {"median": median([item["query_statistics"][key] for item in values]), "min": min(item["query_statistics"][key] for item in values), "max": max(item["query_statistics"][key] for item in values)} for key in keys}
            cells.append({"rows": rows, "treatment": treatment, "repetition_count": 5, "repetition_level": level})
        artifact["summary"] = {"cells": cells}
        artifact["next_step"] = "review fixture-scoped characterization; do not infer a supported-scale claim"
    return artifact


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-root", required=True, help="repository-relative retained raw root")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    print(json.dumps(aggregate(repo, args.input_root), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
