#!/usr/bin/env python3
"""Independently audit the retained persistent-provider fault and recovery runs."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sqlite3
import subprocess


EXPECTED_BODY = "persistent provider fault body"
EXPECTED_RUNS = [f"measured-{index:02d}" for index in range(1, 6)]
EVENT_PREFIX = "SLICE135_PERSISTENT_PROVIDER "


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def checked_source(checkout: Path, source_sha: str, relative: str, expected_hash: str) -> None:
    source = subprocess.check_output(
        ["git", "show", f"{source_sha}:{relative}"], cwd=checkout
    )
    require(hashlib.sha256(source).hexdigest() == expected_hash, f"{relative}: source mismatch")


def parse_event(stderr: str) -> dict:
    events = [line[len(EVENT_PREFIX) :] for line in stderr.splitlines() if line.startswith(EVENT_PREFIX)]
    require(len(events) == 1, "expected exactly one provider-fault event")
    return json.loads(events[0])


def validate_event(event: dict) -> None:
    require(event.get("case") == "persistent_provider_failure_and_explicit_rebuild", "wrong case")
    require(event.get("fault_point") == "all provider calls fail through configured projection retry budget", "wrong fault point")
    cursor = event.get("cursor")
    require(type(cursor) is int and cursor > 0, "invalid cursor")
    failed = event.get("failed", {})
    reopened = event.get("reopened_before_rebuild", {})
    recovered = event.get("recovered", {})
    final = event.get("final_reopen", {})
    for name, state in (("failed", failed), ("reopened", reopened)):
        require(state.get("body") == EXPECTED_BODY, f"{name}: canonical body changed")
        require(state.get("status") == "Failed", f"{name}: status not failed")
        require(state.get("failure_audit_rows") == 1, f"{name}: failure audit count")
        require(state.get("has_vector") is False, f"{name}: false ready vector")
    failure_calls = event.get("failure_body_calls")
    require(type(failure_calls) is int and failure_calls >= 4, "retry budget not exercised")
    require(reopened.get("projection_body_calls") == failure_calls, "terminal row silently retried")
    require(recovered.get("status") == "UpToDate" and recovered.get("has_vector") is True, "rebuild did not recover")
    require(final.get("body") == EXPECTED_BODY, "final canonical body changed")
    require(final.get("status") == "UpToDate" and final.get("has_vector") is True, "final reopen lost vector")
    require(event.get("physical_canonical_rows") == 1, "reported physical row count")
    require(event.get("integrity_check") == "ok", "reported integrity failure")


def database_state(path: Path) -> dict:
    with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
        canonical = [list(row) for row in connection.execute(
            "SELECT write_cursor, kind, logical_id, body FROM canonical_nodes"
        )]
        terminal = [list(row) for row in connection.execute(
            "SELECT write_cursor, state FROM _fathomdb_projection_terminal"
        )]
        vectors = [list(row) for row in connection.execute(
            "SELECT kind, write_cursor FROM _fathomdb_vector_rows"
        )]
        failures = connection.execute(
            "SELECT record_key, payload_json FROM operational_mutations "
            "WHERE collection_name = 'projection_failures'"
        ).fetchall()
        integrity = [row[0] for row in connection.execute("PRAGMA integrity_check")]
        foreign_keys = connection.execute("PRAGMA foreign_key_check").fetchall()
    return {
        "canonical": canonical,
        "terminal": terminal,
        "vectors": vectors,
        "failures": [[key, json.loads(payload)] for key, payload in failures],
        "integrity": integrity,
        "foreign_key_errors": foreign_keys,
    }


def validate_database(state: dict, cursor: int) -> None:
    require(state["canonical"] == [[cursor, "doc", "provider-fault", EXPECTED_BODY]], "persisted canonical row mismatch")
    require(state["terminal"] == [[cursor, "up_to_date"]], "persisted terminal state mismatch")
    require(state["vectors"] == [["doc", cursor]], "persisted vector row mismatch")
    failures = state["failures"]
    require(len(failures) == 1 and failures[0][0] == str(cursor), "durable failure audit missing")
    require(failures[0][1].get("failure_code") == "EmbedderError", "failure code mismatch")
    require(state["integrity"] == ["ok"] and state["foreign_key_errors"] == [], "SQLite integrity failure")


def resource(stderr: str) -> dict:
    def number(label: str) -> int:
        found = re.search(rf"^\s*{re.escape(label)}:\s*(\d+)\s*$", stderr, re.MULTILINE)
        require(found is not None, f"missing GNU Time {label}")
        return int(found.group(1))

    result = {
        "peak_rss_kib": number("Maximum resident set size (kbytes)"),
        "major_faults": number("Major (requiring I/O) page faults"),
        "swap_events": number("Swaps"),
        "fs_inputs": number("File system inputs"),
        "fs_outputs": number("File system outputs"),
        "exit_status": number("Exit status"),
    }
    require(result["peak_rss_kib"] > 0 and result["swap_events"] == 0 and result["exit_status"] == 0, "invalid child resource record")
    return result


def audit(root: Path, checkout: Path) -> dict:
    manifest = json.loads((root / "manifest.json").read_text())
    source_sha = manifest["source_sha"]
    require(re.fullmatch(r"[0-9a-f]{40}", source_sha) is not None, "invalid source SHA")
    for relative, key in (
        ("src/rust/crates/fathomdb-engine/tests/slice135_persistent_provider.rs", "test_sha256"),
        ("src/rust/crates/fathomdb-engine/Cargo.toml", "cargo_toml_sha256"),
        ("Cargo.lock", "cargo_lock_sha256"),
    ):
        checked_source(checkout, source_sha, relative, manifest[key])
    require(manifest["features"] == ["operator", "test-hooks", "default-embedder"], "wrong features")
    require([run["name"] for run in manifest["runs"]] == EXPECTED_RUNS, "missing or reordered runs")
    results = []
    for item in manifest["runs"]:
        directory = root / item["name"]
        stdout_path, stderr_path = directory / "stdout.log", directory / "stderr.log"
        database_path = directory / "provider-fault.sqlite"
        for path, key in ((stdout_path, "stdout_sha256"), (stderr_path, "stderr_sha256"), (database_path, "db_sha256")):
            require(digest(path) == item[key], f"{item['name']}: {path.name} hash mismatch")
        require(item["returncode"] == 0, f"{item['name']}: runner failed")
        stdout, stderr = stdout_path.read_text(), stderr_path.read_text()
        require("test result: ok. 1 passed; 0 failed" in stdout, f"{item['name']}: test did not pass")
        event = parse_event(stderr)
        validate_event(event)
        state = database_state(database_path)
        validate_database(state, event["cursor"])
        results.append({"name": item["name"], "event": event, "database": state, "resources": resource(stderr)})
    return {"status": "PASS", "source_sha": source_sha, "binary_sha256": manifest["binary_sha256"], "audited_runs": len(results), "runs": results}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.root, args.checkout)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"status": result["status"], "runs": result["audited_runs"]}))


if __name__ == "__main__":
    main()
