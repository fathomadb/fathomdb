"""Independently audit the source-bound Rust SDK S02 consumer receipt."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sqlite3


ROOT = Path(__file__).resolve().parent
EXPECTED_HASHES = {
    "main.rs": "dd885f029f003635726c494cd69f565ef2241a5e3e7a432c2722fd0a16879075",
    "Cargo.toml.snapshot": "782b26607f9c83382da3a07b75627a04ab3fedfb3815c17dc721f1756d2bba40",
    "Cargo.lock": "8e26ebd7ec4c910d69177b736e6a5040c346ec7b9c1250b68a5ded4e24974c52",
    "consumer.bin": "493512a1ac189c483507048ff7eb0362c7d5532c2425b9e878f7b2752be44621",
    "consumer.sqlite": "bb5fdbe6e809fac7c57ff7acb6c8e34860798d1588fb46588acc3c4b8d6d4278",
}
EXPECTED_COUNTS = {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32}
EXPECTED_STDOUT = (
    "RUST_SDK_S02_FUNCTIONAL_OK corpus=32 graph_erased_nodes=3 "
    "graph_erased_edges=1 evidence=true reopened=true\n"
)


def sha(path: Path) -> str:
    """Return a retained file's SHA-256 digest."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def counts_from_db() -> dict[str, int]:
    """Read persisted canonical state directly from the retained SQLite database."""
    with sqlite3.connect(ROOT / "consumer.sqlite") as connection:
        return {
            "graph_nodes": connection.execute(
                "SELECT count(*) FROM canonical_nodes WHERE source_id=?",
                ("slice135-rust-graph",),
            ).fetchone()[0],
            "graph_edges": connection.execute(
                "SELECT count(*) FROM canonical_edges WHERE source_id=?",
                ("slice135-rust-graph",),
            ).fetchone()[0],
            "corpus_nodes": connection.execute(
                "SELECT count(*) FROM canonical_nodes WHERE source_id=?",
                ("slice135-rust-corpus",),
            ).fetchone()[0],
        }


def check_counts(counts: dict[str, int]) -> None:
    """Require full graph erasure and retention of the independent corpus."""
    if counts != EXPECTED_COUNTS:
        raise ValueError(f"canonical persistence changed: {counts}")


def main() -> None:
    """Check source, executable, Cargo provenance, raw output and state."""
    for name, expected in EXPECTED_HASHES.items():
        if sha(ROOT / name) != expected:
            raise ValueError(f"{name} changed")
    manifest = (ROOT / "Cargo.toml.snapshot").read_text()
    tree = (ROOT / "cargo-tree.txt").read_text()
    if 'fathomdb-sdk = { path = ' not in manifest or 'features = ["default-embedder"]' not in manifest:
        raise ValueError("external Cargo boundary changed")
    if "fathomdb-sdk v0.8.26 (/home/coreyt/projects/fathomdb-worktrees/release-0.8.27-slice-135/" not in tree:
        raise ValueError("SDK dependency did not resolve to candidate source")
    if (ROOT / "stdout.log").read_text() != EXPECTED_STDOUT or (ROOT / "stderr.log").stat().st_size:
        raise ValueError("consumer process did not complete cleanly")
    resource = (ROOT / "resource.txt").read_text()
    if "Exit status: 0" not in resource or "Swaps: 0" not in resource:
        raise ValueError("process resource status changed")
    persisted = counts_from_db()
    check_counts(persisted)
    if json.loads((ROOT / "independent-state.json").read_text()) != persisted:
        raise ValueError("independent state output differs from retained database")
    mutant = {**persisted, "graph_edges": 1}
    try:
        check_counts(mutant)
    except ValueError as error:
        negative_control = str(error)
    else:
        raise AssertionError("retained-edge negative control was accepted")
    output = {
        "schema_version": 1,
        "candidate_source_sha": "febc62b5e792937d5312c0d7b8b97db963917784",
        "source_bound_external_consumer": True,
        "published_crate_install": False,
        "consumer_binary_sha256": sha(ROOT / "consumer.bin"),
        "database_sha256": sha(ROOT / "consumer.sqlite"),
        "canonical_counts": persisted,
        "semantic_checks_passed": True,
        "negative_control_rejected": negative_control,
    }
    (ROOT / "audit.json").write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps(output))


if __name__ == "__main__":
    main()
