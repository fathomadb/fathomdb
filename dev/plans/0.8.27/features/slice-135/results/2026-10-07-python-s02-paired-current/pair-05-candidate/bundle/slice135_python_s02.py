#!/usr/bin/env python3
"""Exercise one installed-wheel S02 whole-product sequence on a fresh database."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory
from time import perf_counter_ns
from typing import Any, Callable, TypeVar

import slice135_python_s01 as s01


GRAPH_SOURCE = "slice135-s02-graph"
SOURCE_BODY = json.dumps({"summary": "s02 canonical source evidence bytes"})
CLAIM_TOKEN = "slice135s02evidenceneedle"
T = TypeVar("T")


def make_graph_records() -> list[dict[str, Any]]:
    """Build one canonical source, two derived nodes and their evidence edge."""
    digest = hashlib.sha256(SOURCE_BODY.encode()).hexdigest()
    canonical = {
        "schema_version": 1,
        "role": "canonical",
        "artifact_revision_id": "s02-source-r1",
        "source_version_id": "s02-source-v1",
    }

    def derived(revision: str) -> dict[str, Any]:
        return {
            "schema_version": 1,
            "role": "derived",
            "artifact_revision_id": revision,
            "source_version_id": "s02-source-v1",
            "source_revision_id": "s02-source-r1",
            "source_locator": {"kind": "whole_body"},
            "canonical_source_hash": {"algorithm": "sha256", "digest_hex": digest},
        }

    return [
        {
            "kind": "doc",
            "body": SOURCE_BODY,
            "source_id": GRAPH_SOURCE,
            "logical_id": "s02-source",
            "provenance": canonical,
        },
        {
            "kind": "doc",
            "body": json.dumps({"summary": "s02 graph root"}),
            "source_id": GRAPH_SOURCE,
            "logical_id": "s02-root",
            "provenance": derived("s02-root-r1"),
        },
        {
            "kind": "doc",
            "body": json.dumps({"summary": CLAIM_TOKEN}),
            "source_id": GRAPH_SOURCE,
            "logical_id": "s02-claim",
            "provenance": derived("s02-claim-r1"),
        },
        {
            "edge": {
                "kind": "supports",
                "from": "s02-root",
                "to": "s02-claim",
                "source_id": GRAPH_SOURCE,
                "logical_id": "s02-edge",
                "provenance": derived("s02-edge-r1"),
            }
        },
    ]


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def validate_observations(observed: dict[str, Any]) -> None:
    """Validate materialized state without treating version agreement as gold."""
    _require(observed["embedder"] == "fathomdb-bge-small-en-v1.5", "embedder identity")
    _require(observed["unsupported_kinds"] == [], "unsupported vector kind")
    _require(observed["readiness"] == "ready", "projection not ready")
    _require(observed["anchor_before"] is True, "anchor changed before queries")
    _require(observed["text"] == {"ids": ["A"], "branches": ["text"]}, "text anchor")
    _require(
        observed["vector"]["ids"] and "vector" in observed["vector"]["branches"],
        "vector branch",
    )
    eligible_ids = {row["logical_id"] for row in s01.make_corpus(32)}
    eligible_ids.update(("s02-source", "s02-root", "s02-claim"))
    for kind in ("text", "vector", "hybrid"):
        _require(
            all(item in eligible_ids for item in observed[kind]["ids"]),
            "unknown retrieval id",
        )
    _require("A" in observed["hybrid"]["ids"], "hybrid anchor")
    _require("A" in observed["hybrid_lexical_ids"], "hybrid lexical eligibility")
    _require(
        set(observed["hybrid"]["branches"]) <= {"text", "vector"},
        "hybrid branch",
    )
    _require(
        observed["evidence"]["logical_id"] == "s02-claim"
        and observed["evidence"]["source_body"] == SOURCE_BODY,
        "canonical evidence",
    )
    graph = observed["graph"]
    _require(
        graph
        == {
            "target_id": "s02-claim",
            "target_revision": "s02-claim-r1",
            "edge_revision": "s02-edge-r1",
            "edge_from": "s02-root",
            "edge_to": "s02-claim",
            "source_body": SOURCE_BODY,
        },
        "graph evidence",
    )
    erasure = observed["erasure"]
    _require(
        erasure == {"source_ref": GRAPH_SOURCE, "nodes_excised": 3, "edges_excised": 1},
        "erasure count",
    )
    _require(
        observed["second_erasure"] == {"nodes_excised": 0, "edges_excised": 0},
        "erasure idempotence",
    )
    for label in ("after_erasure", "after_reopen"):
        state = observed[label]
        _require(
            all(
                state[name] is True
                for name in (
                    "source_absent",
                    "root_absent",
                    "claim_absent",
                    "anchor_retained",
                    "evidence_query_empty",
                    "graph_empty",
                )
            ),
            "reopened erased state" if label == "after_reopen" else "erased state",
        )
        _require(
            state["canonical_counts"]
            == {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32},
            "canonical persistence",
        )
    _require(observed["after_reopen"]["readiness"] == "ready", "reopened readiness")


def _hits(result: Any) -> dict[str, list[str]]:
    return {
        "ids": [hit.id.value for hit in result.results],
        "branches": [hit.branch for hit in result.results],
    }


def _canonical_counts(database: str) -> dict[str, int]:
    with sqlite3.connect(database) as connection:
        return {
            "graph_nodes": connection.execute(
                "SELECT count(*) FROM canonical_nodes WHERE source_id=?",
                (GRAPH_SOURCE,),
            ).fetchone()[0],
            "graph_edges": connection.execute(
                "SELECT count(*) FROM canonical_edges WHERE source_id=?",
                (GRAPH_SOURCE,),
            ).fetchone()[0],
            "corpus_nodes": connection.execute(
                "SELECT count(*) FROM canonical_nodes WHERE source_id=?",
                (s01.SOURCE_ID,),
            ).fetchone()[0],
        }


def _post_state(engine: Any, database: str, anchor_body: str) -> dict[str, Any]:
    import fathomdb

    return {
        "source_absent": fathomdb.read.get(engine, "s02-source") is None,
        "root_absent": fathomdb.read.get(engine, "s02-root") is None,
        "claim_absent": fathomdb.read.get(engine, "s02-claim") is None,
        "anchor_retained": fathomdb.read.get(engine, "A").body == anchor_body,
        "evidence_query_empty": not engine.search_text_only(CLAIM_TOKEN).results,
        "graph_empty": not fathomdb.graph.neighbors(engine, "s02-root", depth=1),
        "canonical_counts": _canonical_counts(database),
    }


def run_once(*, wheel: Path, wheel_sha256: str, source_sha: str) -> dict[str, Any]:
    """Time open through reopened close and retain each materialized state."""
    import fathomdb

    if len(source_sha) != 40 or any(
        char not in "0123456789abcdef" for char in source_sha
    ):
        raise ValueError("source SHA must be forty lowercase hex digits")
    identity = s01._installed_wheel_identity(wheel, wheel_sha256)
    corpus = s01.make_corpus(32)
    records = [*corpus, *make_graph_records()]
    stage_ns: dict[str, int] = {}

    def timed(name: str, call: Callable[[], T]) -> T:
        started = perf_counter_ns()
        try:
            return call()
        finally:
            stage_ns[name] = perf_counter_ns() - started

    with TemporaryDirectory(prefix="slice135-python-s02-") as temporary:
        database = str(Path(temporary) / "s02.sqlite")
        whole_start = perf_counter_ns()
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
            anchor_before = all(
                fathomdb.read.get(engine, item["logical_id"]).body == item["body"]
                for item in corpus[:2]
            )
            text = _hits(
                timed("text", lambda: engine.search_text_only(s01.QUERIES["text"]))
            )
            vector = _hits(
                timed("vector", lambda: engine.search(s01.QUERIES["vector"]))
            )
            hybrid = _hits(
                timed("hybrid", lambda: engine.search(s01.QUERIES["hybrid"]))
            )
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
                        query=CLAIM_TOKEN, context=frozen, limit=1
                    )
                ),
            )
            _require(bool(evidence_result.evidence), "evidence sidecar empty")
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
                            schema_version=1,
                            type="frozen",
                            context=frozen,
                        ),
                        max_depth=1,
                        result_limit=1,
                        max_work_units="10",
                        include_explanation=False,
                        include_evidence=True,
                    ),
                ),
            )
            _require(
                bool(expanded.targets) and expanded.evidence is not None, "graph empty"
            )
            entry = expanded.evidence.entries[0]
            target = timed(
                "graph_target_resolve",
                lambda: engine.resolve_graph_evidence(
                    fathomdb.GraphEvidenceResolveRequestV1(
                        evidence_ref=entry.target_evidence_ref,
                        context=frozen,
                    )
                ),
            )
            edge = timed(
                "graph_edge_resolve",
                lambda: engine.resolve_graph_evidence(
                    fathomdb.GraphEvidenceResolveRequestV1(
                        evidence_ref=entry.terminal_edge_evidence_ref,
                        context=frozen,
                    )
                ),
            )
            report = timed("erase", lambda: engine.erase_source(GRAPH_SOURCE))
            second = timed("erase_again", lambda: engine.erase_source(GRAPH_SOURCE))
            after_erasure = _post_state(engine, database, corpus[0]["body"])
            observed: dict[str, Any] = {
                "embedder": embedder,
                "unsupported_kinds": delta.vector_unsupported_kinds,
                "readiness": readiness,
                "anchor_before": anchor_before,
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
                **_post_state(reopened, database, corpus[0]["body"]),
                "readiness": next(
                    spec.vector_dense_readiness
                    for spec in fathomdb.read.projections(reopened)
                    if spec.name == "summary"
                ),
            }
        finally:
            timed("reopened_close", reopened.close)
        whole_sequence_including_checks_ns = perf_counter_ns() - whole_start
        validate_observations(observed)
        return {
            "schema_version": 1,
            "status": "UNFROZEN_PYTHON_S02_FUNCTIONAL_FEASIBILITY",
            "finished_utc": datetime.now(timezone.utc).isoformat(),
            "source_sha": source_sha,
            "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "s01_helper_sha256": hashlib.sha256(
                Path(s01.__file__).read_bytes()
            ).hexdigest(),
            "artifact": identity,
            "corpus_sha256": hashlib.sha256(
                json.dumps(corpus, sort_keys=True).encode()
            ).hexdigest(),
            "graph_records_sha256": hashlib.sha256(
                json.dumps(make_graph_records(), sort_keys=True).encode()
            ).hexdigest(),
            "whole_sequence_including_checks_ns": whole_sequence_including_checks_ns,
            "stage_ns": stage_ns,
            "observed": observed,
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
    output = run_once(
        wheel=args.wheel,
        wheel_sha256=args.wheel_sha256,
        source_sha=args.source_sha,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
