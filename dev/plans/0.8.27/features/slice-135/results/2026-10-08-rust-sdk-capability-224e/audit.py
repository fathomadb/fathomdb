"""Audit every live governed Rust SDK operation against an external receipt."""

import hashlib
import json
from pathlib import Path
import sqlite3
import tomllib

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[6]
OPERATIONS = json.loads((REPO / "src/conformance/governed-operation-parity.json").read_text())["operations"]
EXPECTED = {item["id"] for item in OPERATIONS if item["state"] == "live"}
SOURCE_COMMIT = "224e44c593c13d86ece648adabe445723db04070"
EXPECTED_STATE = {
    "corpus_nodes": 32,
    "graph_nodes": 0,
    "graph_edges": 0,
    "actuation_nodes": 1,
    "refused_nodes": 0,
    "lifecycle_nodes": 0,
    "closure_nodes": 2,
    "events_mutations": 2,
    "settings_state": 2,
}


def sha256(path):
    """Hash retained receipt or source bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit(rows):
    ids = [row["id"] for row in rows]
    if len(ids) != len(set(ids)) or set(ids) != EXPECTED:
        raise ValueError("operation accounting differs from canonical live set")
    for row in rows:
        if row["status"] not in {"executed", "gap", "unavailable"}:
            raise ValueError(f"invalid status: {row['id']}")
        if not row.get("detail"):
            raise ValueError(f"missing detail: {row['id']}")
        if row["status"] == "gap" and not row["detail"].startswith("gap:"):
            raise ValueError(f"unexplained gap: {row['id']}")
        if row["status"] == "unavailable" and not row["detail"].startswith("unavailable:"):
            raise ValueError(f"unexplained unavailable case: {row['id']}")


def check_state(state):
    """Enforce independent canonical and operational persistence checks."""
    if state != EXPECTED_STATE:
        raise ValueError(f"persisted state differs: {state}")


def read_state():
    """Read the retained SQLite file without using the Rust SDK or consumer."""
    with sqlite3.connect(f"file:{ROOT / 'consumer.sqlite'}?mode=ro&immutable=1", uri=True) as db:
        def count(table, column, value):
            return db.execute(f"SELECT count(*) FROM {table} WHERE {column}=?", (value,)).fetchone()[0]

        return {
            "corpus_nodes": count("canonical_nodes", "source_id", "slice135-rust-corpus"),
            "graph_nodes": count("canonical_nodes", "source_id", "slice135-rust-graph"),
            "graph_edges": count("canonical_edges", "source_id", "slice135-rust-graph"),
            "actuation_nodes": count("canonical_nodes", "logical_id", "act-node"),
            "refused_nodes": count("canonical_nodes", "logical_id", "refused-node"),
            "lifecycle_nodes": count("canonical_nodes", "logical_id", "lifecycle-node"),
            "closure_nodes": count("canonical_nodes", "source_id", "slice135-closure"),
            "events_mutations": count("operational_mutations", "collection_name", "events"),
            "settings_state": count("operational_state", "collection_name", "settings"),
        }


if __name__ == "__main__":
    rows = json.loads((ROOT / "operations.json").read_text())
    audit(rows)
    stdout = (ROOT / "stdout.log").read_text()
    markers = [line.split("|", 2)[1:] for line in stdout.splitlines() if line.startswith("OP|")]
    if len(markers) != len(rows) or {tuple(marker) for marker in markers} != {
        (row["id"], row["detail"]) for row in rows
    }:
        raise ValueError("operation rows differ from completed consumer output")
    if not stdout.rstrip().endswith("RUST_SDK_S02_FUNCTIONAL_OK corpus=32 graph_erased_nodes=3 graph_erased_edges=1 evidence=true reopened=true"):
        raise ValueError("consumer did not reach its completion marker")
    if (ROOT / "stderr.log").read_text():
        raise ValueError("consumer stderr is not empty")
    if (ROOT / "source-commit.txt").read_text().strip() != SOURCE_COMMIT:
        raise ValueError("candidate source commit changed")
    manifest = (ROOT / "Cargo.toml.snapshot").read_text()
    tree = (ROOT / "cargo-tree.txt").read_text()
    package = tomllib.loads(manifest)
    sdk_dependency = package["dependencies"]["fathomdb-sdk"]
    if (
        "workspace" not in package
        or "default-embedder" not in sdk_dependency["features"]
        or (ROOT / sdk_dependency["path"]).resolve()
        != REPO / "src/rust/crates/fathomdb-sdk"
    ):
        raise ValueError("external Cargo manifest changed")
    if not any(
        "fathomdb-sdk v0.8.26 (" in line
        and line.rstrip().endswith("/src/rust/crates/fathomdb-sdk)")
        for line in tree.splitlines()
    ):
        raise ValueError("SDK did not resolve to the candidate source")
    state = read_state()
    check_state(state)
    try:
        check_state({**state, "graph_edges": 1})
    except ValueError as error:
        negative_control = str(error)
    else:
        raise AssertionError("retained-edge negative control was accepted")
    files = ["main.rs", "Cargo.toml.snapshot", "Cargo.lock", "stdout.log", "stderr.log",
             "consumer.sqlite", "cargo-tree.txt", "operations.json", "build.stdout",
             "build.stderr", "rustc-version.txt", "source-commit.txt", "binary-sha256.txt"]
    result = {
        "schema_version": 1,
        "source_bound_external_consumer": True,
        "published_crate_install": False,
        "candidate_source_commit": SOURCE_COMMIT,
        "canonical_operation_map_sha256": sha256(REPO / "src/conformance/governed-operation-parity.json"),
        "sdk_source_sha256": {str(path.relative_to(REPO)): sha256(path) for path in sorted(
            (REPO / "src/rust/crates/fathomdb-sdk/src").glob("*.rs"))},
        "dependencies": {name: sha256(ROOT / name) for name in ("Cargo.toml.snapshot", "Cargo.lock", "cargo-tree.txt")},
        "files": {name: sha256(ROOT / name) for name in files},
        "counts": {status: sum(row["status"] == status for row in rows)
                   for status in ("executed", "gap", "unavailable")},
        "sqlite_state": state,
        "negative_control_rejected": negative_control,
    }
    if result != json.loads((ROOT / "audit.json").read_text()):
        raise ValueError("retained independent audit differs from recomputation")
    print(json.dumps(result, indent=2))
