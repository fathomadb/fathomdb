#!/usr/bin/env python3
"""Exercise bounded concurrent S02 calls through one installed Python wheel."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import threading
import time
from typing import Any

import slice135_python_s01 as s01
import slice135_python_s02 as s02


CONTENDED_SOURCE = "slice135-contended"
WRITER_IDS = [f"contend-{number:02d}" for number in range(8)]
EXPECTED_COUNTS = {
    "corpus_nodes": 32, "graph_nodes": 0,
    "graph_edges": 0, "contended_nodes": 0,
}


def sha(path: Path) -> str:
    """Hash exact runner and artifact bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def database_state(database: Path) -> tuple[dict[str, int], str]:
    """Inspect persisted canonical rows through an independent SQLite handle."""
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        counts = {
            "corpus_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id=?", (s01.SOURCE_ID,)
            ).fetchone()[0],
            "graph_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id=?", (s02.GRAPH_SOURCE,)
            ).fetchone()[0],
            "graph_edges": connection.execute(
                "SELECT COUNT(*) FROM canonical_edges WHERE source_id=?", (s02.GRAPH_SOURCE,)
            ).fetchone()[0],
            "contended_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id=?", (CONTENDED_SOURCE,)
            ).fetchone()[0],
        }
        integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
    return counts, integrity


def validate_observed(observed: dict[str, Any]) -> None:
    """Reject false concurrency, incomplete work and altered reopen state."""
    if observed.get("status") != "S02_CONTENTION_FUNCTIONAL_OK":
        raise ValueError("contention status")
    if observed.get("writer_ids") != WRITER_IDS:
        raise ValueError("writer IDs")
    if observed.get("reader_cycles") != 8 or observed.get("overlap_cycles", 0) < 1:
        raise ValueError("reader/writer overlap missing")
    if observed.get("reader_errors"):
        raise ValueError("reader error")
    if observed.get("writer_errors"):
        raise ValueError("writer error")
    if observed.get("readiness") != "ready":
        raise ValueError("projection readiness")
    if observed.get("evidence") != {
        "logical_id": "s02-claim", "source_body": s02.SOURCE_BODY,
    }:
        raise ValueError("evidence resolution")
    if observed.get("erasure") != {
        "graph_nodes": 3, "graph_edges": 1, "contended_nodes": 8,
    }:
        raise ValueError("erasure counts")
    if observed.get("before_reopen") != EXPECTED_COUNTS:
        raise ValueError("before reopen state")
    if observed.get("after_reopen") != EXPECTED_COUNTS:
        raise ValueError("after reopen state")
    if observed.get("integrity_check") != "ok":
        raise ValueError("SQLite integrity")
    if (
        observed.get("bounded_seconds") != 60
        or type(observed.get("elapsed_ns")) is not int
        or not 0 < observed["elapsed_ns"] < 60_000_000_000
    ):
        raise ValueError("bounded completion")


def run_once(database: Path) -> dict[str, Any]:
    """Run one synchronized shared-engine writer/reader schedule."""
    import fathomdb

    if database.exists():
        raise ValueError("contention database must be fresh")
    started = time.perf_counter_ns()
    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    observations: dict[str, Any] = {
        "status": "S02_CONTENTION_FUNCTIONAL_OK",
        "writer_ids": WRITER_IDS,
        "reader_cycles": 0,
        "overlap_cycles": 0,
        "reader_errors": [],
        "writer_errors": [],
        "bounded_seconds": 60,
    }
    timeline: dict[str, Any] = {"writer_calls": [], "reader_calls": []}
    try:
        corpus = s01.make_corpus(32)
        engine.write([*corpus, *s02.make_graph_records()])
        engine.configure_projections([
            fathomdb.ProjectionSpec(
                name="summary",
                roles=frozenset({fathomdb.ProjectionRole.SEARCHABLE}),
                vector=True,
            )
        ])
        engine.drain(timeout_s=60)
        barrier = threading.Barrier(3)
        reader_intervals: list[tuple[int, int]] = []

        def writer() -> None:
            try:
                barrier.wait(timeout=15)
                for number, logical_id in enumerate(WRITER_IDS):
                    call_start = time.perf_counter_ns()
                    engine.write([{
                        "kind": "doc",
                        "body": json.dumps({"summary": f"contended {number:02d}"}),
                        "source_id": CONTENDED_SOURCE,
                        "logical_id": logical_id,
                    }])
                    timeline["writer_calls"].append({
                        "logical_id": logical_id,
                        "start_ns": call_start,
                        "end_ns": time.perf_counter_ns(),
                    })
                    time.sleep(0.015)
            except Exception as error:  # noqa: BLE001 - retain exact worker failure
                observations["writer_errors"].append(repr(error))

        def reader() -> None:
            try:
                barrier.wait(timeout=15)
                for _ in range(8):
                    call_start = time.perf_counter_ns()
                    anchor = fathomdb.read.get(engine, "A")
                    if anchor is None or anchor.body != corpus[0]["body"]:
                        raise AssertionError("anchor changed during contention")
                    text = engine.search_text_only(s01.QUERIES["text"])
                    if [hit.id.value for hit in text.results] != ["A"]:
                        raise AssertionError("text query changed during contention")
                    vector = engine.search(s01.QUERIES["vector"])
                    if not vector.results or "vector" not in [hit.branch for hit in vector.results]:
                        raise AssertionError("vector query changed during contention")
                    neighbors = fathomdb.graph.neighbors(engine, "s02-root", depth=1)
                    if not neighbors:
                        raise AssertionError("graph disappeared during contention")
                    call_end = time.perf_counter_ns()
                    reader_intervals.append((call_start, call_end))
                    timeline["reader_calls"].append({
                        "start_ns": call_start, "end_ns": call_end,
                        "vector_hits": len(vector.results),
                    })
                observations["reader_cycles"] = len(reader_intervals)
            except Exception as error:  # noqa: BLE001 - retain exact worker failure
                observations["reader_errors"].append(repr(error))

        writer_thread = threading.Thread(target=writer, name="slice135-writer", daemon=True)
        reader_thread = threading.Thread(target=reader, name="slice135-reader", daemon=True)
        writer_thread.start()
        reader_thread.start()
        barrier.wait(timeout=15)
        writer_thread.join(timeout=30)
        reader_thread.join(timeout=30)
        if writer_thread.is_alive() or reader_thread.is_alive():
            raise TimeoutError("S02 contention worker exceeded 30 seconds")
        observations["overlap_cycles"] = sum(
            any(
                call["start_ns"] < end and call["end_ns"] > start
                for call in timeline["writer_calls"]
            )
            for start, end in reader_intervals
        )
        engine.drain(timeout_s=60)
        observations["readiness"] = next(
            spec.vector_dense_readiness
            for spec in fathomdb.read.projections(engine)
            if spec.name == "summary"
        )
        for logical_id in WRITER_IDS:
            if fathomdb.read.get(engine, logical_id) is None:
                raise AssertionError(f"committed writer row missing: {logical_id}")
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        found = engine.search_with_evidence(
            fathomdb.EvidenceSearchRequestV1(query=s02.CLAIM_TOKEN, context=frozen, limit=1)
        )
        resolved = engine.resolve_evidence(
            fathomdb.EvidenceResolveRequestV1(
                evidence_ref=found.evidence[0].evidence_ref, context=frozen
            )
        )
        if resolved.logical_id != "s02-claim" or resolved.canonical_source_body != s02.SOURCE_BODY:
            raise AssertionError("evidence changed under contention")
        observations["evidence"] = {
            "logical_id": resolved.logical_id,
            "source_body": resolved.canonical_source_body,
        }
        graph_erasure = engine.erase_source(s02.GRAPH_SOURCE)
        contended_erasure = engine.erase_source(CONTENDED_SOURCE)
        if (
            graph_erasure.nodes_excised != 3 or graph_erasure.edges_excised != 1
            or contended_erasure.nodes_excised != 8
        ):
            raise AssertionError("erasure counts changed under contention")
        observations["erasure"] = {
            "graph_nodes": graph_erasure.nodes_excised,
            "graph_edges": graph_erasure.edges_excised,
            "contended_nodes": contended_erasure.nodes_excised,
        }
    finally:
        engine.close()
    observations["before_reopen"], _ = database_state(database)
    reopened = fathomdb.Engine.open(str(database), use_default_embedder=True)
    try:
        if fathomdb.read.get(reopened, "A") is None:
            raise AssertionError("anchor missing after reopen")
        if fathomdb.read.get(reopened, WRITER_IDS[0]) is not None:
            raise AssertionError("erased writer row visible after reopen")
        observations["after_reopen"], observations["integrity_check"] = database_state(database)
    finally:
        reopened.close()
    observations["elapsed_ns"] = time.perf_counter_ns() - started
    validate_observed(observations)
    return {"observed": observations, "timeline": timeline}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", required=True, type=Path)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--comparison-protocol", required=True, type=Path)
    parser.add_argument("--role", required=True, choices=("baseline", "candidate"))
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    specification = json.loads(arguments.comparison_protocol.read_text())
    expected = specification[arguments.role]
    identity = s01._installed_wheel_identity(
        arguments.wheel.resolve(), expected["wheel_sha256"]
    )
    result = run_once(arguments.database.resolve())
    result.update({
        "schema_version": 1,
        "role": arguments.role,
        "source_sha": expected["source_sha"],
        "comparison_protocol_sha256": sha(arguments.comparison_protocol),
        "runner_sha256": sha(Path(__file__)),
        "artifact": identity,
    })
    arguments.output.write_text(json.dumps(result, indent=2) + "\n")
    print("S02_CONTENTION_FUNCTIONAL_OK")


if __name__ == "__main__":
    main()
