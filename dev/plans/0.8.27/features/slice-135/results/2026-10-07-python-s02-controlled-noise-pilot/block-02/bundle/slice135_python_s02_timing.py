#!/usr/bin/env python3
"""Time an installed Python S02 sequence, then verify its real-database state."""

from __future__ import annotations

import argparse
from collections.abc import Callable
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory
from time import perf_counter_ns
from typing import Any, TypeVar

import slice135_python_s01 as s01
import slice135_python_s02 as s02


T = TypeVar("T")
U = TypeVar("U")


def measure_product_then_verify(
    product: Callable[[], T],
    verify: Callable[[T], U],
    *,
    clock: Callable[[], int] = perf_counter_ns,
) -> tuple[T, U, int, int]:
    """Stop the client timer before the independent state oracle runs."""
    started = clock()
    result = product()
    product_end = clock()
    check = verify(result)
    verification_end = clock()
    return result, check, product_end - started, verification_end - product_end


def require_reopened_source_absent(observed: dict[str, Any]) -> None:
    """Reject a surviving erased source even when the sequence returned."""
    if observed["after_reopen"]["source_absent"] is not True:
        raise ValueError("reopened source remains")


def _hits(result: Any) -> dict[str, list[str]]:
    return {
        "ids": [hit.id.value for hit in result.results],
        "branches": [hit.branch for hit in result.results],
    }


def _post_state(engine: Any, anchor_body: str) -> dict[str, bool]:
    import fathomdb

    anchor = fathomdb.read.get(engine, "A")
    return {
        "source_absent": fathomdb.read.get(engine, "s02-source") is None,
        "root_absent": fathomdb.read.get(engine, "s02-root") is None,
        "claim_absent": fathomdb.read.get(engine, "s02-claim") is None,
        "anchor_retained": anchor is not None and anchor.body == anchor_body,
        "evidence_query_empty": not engine.search_text_only(s02.CLAIM_TOKEN).results,
        "graph_empty": not fathomdb.graph.neighbors(engine, "s02-root", depth=1),
    }


def _run_product(database: str) -> dict[str, Any]:
    import fathomdb

    corpus = s01.make_corpus(32)
    records = [*corpus, *s02.make_graph_records()]
    stages: dict[str, int] = {}

    def timed(name: str, call: Callable[[], T]) -> T:
        started = perf_counter_ns()
        try:
            return call()
        finally:
            stages[name] = perf_counter_ns() - started

    engine = timed(
        "open", lambda: fathomdb.Engine.open(database, use_default_embedder=True)
    )
    try:
        embedder = engine.open_report().default_embedder.name
        timed("write", lambda: engine.write(records))
        delta = timed(
            "configure_projection",
            lambda: engine.configure_projections(
                [
                    fathomdb.ProjectionSpec(
                        name="summary",
                        roles=frozenset({fathomdb.ProjectionRole.SEARCHABLE}),
                        vector=True,
                    )
                ]
            ),
        )
        timed("drain", lambda: engine.drain(timeout_s=120))
        readiness = next(
            spec.vector_dense_readiness
            for spec in fathomdb.read.projections(engine)
            if spec.name == "summary"
        )
        anchor = fathomdb.read.get(engine, "A")
        text = _hits(
            timed("text", lambda: engine.search_text_only(s01.QUERIES["text"]))
        )
        vector = _hits(timed("vector", lambda: engine.search(s01.QUERIES["vector"])))
        hybrid = _hits(timed("hybrid", lambda: engine.search(s01.QUERIES["hybrid"])))
        hybrid_lexical_ids = [
            hit.id.value
            for hit in timed(
                "hybrid_lexical_control",
                lambda: engine.search_text_only(s01.QUERIES["hybrid"]),
            ).results
        ]
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        evidence_result = timed(
            "evidence_search",
            lambda: engine.search_with_evidence(
                fathomdb.EvidenceSearchRequestV1(
                    query=s02.CLAIM_TOKEN, context=frozen, limit=1
                )
            ),
        )
        resolved = timed(
            "evidence_resolve",
            lambda: engine.resolve_evidence(
                fathomdb.EvidenceResolveRequestV1(
                    evidence_ref=evidence_result.evidence[0].evidence_ref,
                    context=frozen,
                )
            ),
        )
        expanded = timed(
            "graph_expand",
            lambda: fathomdb.graph.expand(
                engine,
                fathomdb.GraphExpandRequestV1(
                    schema_version=1,
                    seed=fathomdb.GraphExplicitSeedV1(
                        schema_version=1,
                        type="explicit",
                        logical_ids=(
                            fathomdb.IdSpace(space="logical", value="s02-root"),
                        ),
                    ),
                    direction="outgoing",
                    edge_kinds=("supports",),
                    target_kinds=("doc",),
                    context=fathomdb.FrozenGraphReadContextV1(
                        schema_version=1, type="frozen", context=frozen
                    ),
                    max_depth=1,
                    result_limit=1,
                    max_work_units="10",
                    include_explanation=False,
                    include_evidence=True,
                ),
            ),
        )
        entry = expanded.evidence.entries[0]
        target = timed(
            "graph_target_resolve",
            lambda: engine.resolve_graph_evidence(
                fathomdb.GraphEvidenceResolveRequestV1(
                    evidence_ref=entry.target_evidence_ref, context=frozen
                )
            ),
        )
        edge = timed(
            "graph_edge_resolve",
            lambda: engine.resolve_graph_evidence(
                fathomdb.GraphEvidenceResolveRequestV1(
                    evidence_ref=entry.terminal_edge_evidence_ref, context=frozen
                )
            ),
        )
        report = timed("erase", lambda: engine.erase_source(s02.GRAPH_SOURCE))
        second = timed("erase_again", lambda: engine.erase_source(s02.GRAPH_SOURCE))
        after_erasure = _post_state(engine, corpus[0]["body"])
        observed: dict[str, Any] = {
            "embedder": embedder,
            "unsupported_kinds": delta.vector_unsupported_kinds,
            "readiness": readiness,
            "anchor_before": anchor is not None and anchor.body == corpus[0]["body"],
            "text": text,
            "vector": vector,
            "hybrid": hybrid,
            "hybrid_lexical_ids": hybrid_lexical_ids,
            "evidence": {
                "logical_id": resolved.logical_id,
                "source_body": resolved.canonical_source_body,
            },
            "graph": {
                "target_id": target.artifact.logical_id,
                "target_revision": target.artifact_revision_id,
                "edge_revision": edge.artifact_revision_id,
                "edge_from": edge.artifact.from_id,
                "edge_to": edge.artifact.to_id,
                "source_body": target.canonical_source_body,
            },
            "erasure": {
                "source_ref": report.source_ref,
                "nodes_excised": report.nodes_excised,
                "edges_excised": report.edges_excised,
            },
            "second_erasure": {
                "nodes_excised": second.nodes_excised,
                "edges_excised": second.edges_excised,
            },
            "after_erasure": after_erasure,
        }
    finally:
        timed("close", engine.close)
    reopened = timed(
        "reopen", lambda: fathomdb.Engine.open(database, use_default_embedder=True)
    )
    try:
        observed["after_reopen"] = {
            **_post_state(reopened, corpus[0]["body"]),
            "readiness": next(
                spec.vector_dense_readiness
                for spec in fathomdb.read.projections(reopened)
                if spec.name == "summary"
            ),
        }
    finally:
        timed("reopened_close", reopened.close)
    return {"observed": observed, "stage_ns": stages}


def _verify_product(database: str, result: dict[str, Any]) -> dict[str, int]:
    observed = result["observed"]
    require_reopened_source_absent(observed)
    if observed["embedder"] != "fathomdb-bge-small-en-v1.5":
        raise ValueError("embedder identity changed")
    if observed["unsupported_kinds"] != [] or observed["readiness"] != "ready":
        raise ValueError("projection not ready")
    if observed["anchor_before"] is not True:
        raise ValueError("anchor missing")
    if observed["text"] != {"ids": ["A"], "branches": ["text"]}:
        raise ValueError("text anchor changed")
    if not observed["vector"]["ids"] or "vector" not in observed["vector"]["branches"]:
        raise ValueError("vector branch missing")
    if (
        "A" not in observed["hybrid"]["ids"]
        or "A" not in observed["hybrid_lexical_ids"]
    ):
        raise ValueError("hybrid anchor missing")
    if observed["evidence"] != {
        "logical_id": "s02-claim",
        "source_body": s02.SOURCE_BODY,
    }:
        raise ValueError("evidence changed")
    if observed["graph"] != {
        "target_id": "s02-claim",
        "target_revision": "s02-claim-r1",
        "edge_revision": "s02-edge-r1",
        "edge_from": "s02-root",
        "edge_to": "s02-claim",
        "source_body": s02.SOURCE_BODY,
    }:
        raise ValueError("graph evidence changed")
    if observed["erasure"] != {
        "source_ref": s02.GRAPH_SOURCE,
        "nodes_excised": 3,
        "edges_excised": 1,
    } or observed["second_erasure"] != {"nodes_excised": 0, "edges_excised": 0}:
        raise ValueError("erasure report changed")
    for label in ("after_erasure", "after_reopen"):
        for key in (
            "source_absent",
            "root_absent",
            "claim_absent",
            "anchor_retained",
            "evidence_query_empty",
            "graph_empty",
        ):
            if observed[label][key] is not True:
                raise ValueError(f"{label} {key} failed")
    if observed["after_reopen"]["readiness"] != "ready":
        raise ValueError("reopened projection not ready")
    with sqlite3.connect(database) as connection:
        counts = {
            "graph_nodes": connection.execute(
                "SELECT count(*) FROM canonical_nodes WHERE source_id=?",
                (s02.GRAPH_SOURCE,),
            ).fetchone()[0],
            "graph_edges": connection.execute(
                "SELECT count(*) FROM canonical_edges WHERE source_id=?",
                (s02.GRAPH_SOURCE,),
            ).fetchone()[0],
            "corpus_nodes": connection.execute(
                "SELECT count(*) FROM canonical_nodes WHERE source_id=?",
                (s01.SOURCE_ID,),
            ).fetchone()[0],
        }
    if counts != {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32}:
        raise ValueError("reopened canonical state changed")
    return counts


def run_once(*, wheel: Path, wheel_sha256: str, source_sha: str) -> dict[str, Any]:
    """Return one product-only duration with a separate state-check duration."""
    if len(source_sha) != 40 or any(
        char not in "0123456789abcdef" for char in source_sha
    ):
        raise ValueError("source SHA must be forty lowercase hex digits")
    identity = s01._installed_wheel_identity(wheel, wheel_sha256)
    with TemporaryDirectory(prefix="slice135-python-s02-timing-") as directory:
        database = str(Path(directory) / "s02.sqlite")
        result, counts, product_ns, verification_ns = measure_product_then_verify(
            lambda: _run_product(database),
            lambda product: _verify_product(database, product),
        )
    return {
        "schema_version": 1,
        "status": "BASELINE_ONLY_S02_TIMING_PILOT",
        "finished_utc": datetime.now(timezone.utc).isoformat(),
        "source_sha": source_sha,
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "s01_helper_sha256": hashlib.sha256(
            Path(s01.__file__).read_bytes()
        ).hexdigest(),
        "s02_helper_sha256": hashlib.sha256(
            Path(s02.__file__).read_bytes()
        ).hexdigest(),
        "artifact": identity,
        "corpus_sha256": hashlib.sha256(
            json.dumps(s01.make_corpus(32), sort_keys=True).encode()
        ).hexdigest(),
        "graph_records_sha256": hashlib.sha256(
            json.dumps(s02.make_graph_records(), sort_keys=True).encode()
        ).hexdigest(),
        "whole_product_ns": product_ns,
        "verification_ns": verification_ns,
        "stage_ns": result["stage_ns"],
        "observed": result["observed"],
        "final_canonical_counts": counts,
        "semantic_ok": True,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    result = run_once(
        wheel=args.wheel,
        wheel_sha256=args.wheel_sha256,
        source_sha=args.source_sha,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
