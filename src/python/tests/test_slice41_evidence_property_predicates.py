"""0.8.32 Slice 41 (`WP-41C`, `#269` Option A) — Python cross-binding parity tests.

Covers `AC32-EVIDENCE-PROP-01..03`:
- Artifact-scoped `SearchFilter.property_predicates` on evidence `source_filter`
  across `search_with_evidence`, `resolve_evidence`, `resolve_evidence_many`,
  and `graph.expand(include_evidence=True)` + `resolve_graph_evidence`.
- Source-enforced access-bearing `attributes` and Schema 39 `ScopePolicy`
  failing closed with non-disclosing `EvidenceError("evidence_unavailable")`.
- Intentional `trace_dependency` symmetric endpoint visibility and `GraphArm`
  `SearchHit.source_id` vs. `ResolvedEvidenceV1.source_id` semantics.
"""

from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Any

import pytest

from fathomdb import (
    DependencyTraceRequestV1,
    Engine,
    EvidenceResolveManyRequestV1,
    EvidenceResolveRequestV1,
    EvidenceSearchRequestV1,
    FrozenGraphReadContextV1,
    GraphEvidenceResolveRequestV1,
    GraphExpandRequestV1,
    GraphExplicitSeedV1,
    IdSpace,
    ProjectionRole,
    ProjectionSpec,
    PropertyPredicate,
    ReadContextV1,
    ReadView,
    ScopePolicy,
    SearchFilter,
    graph,
)
from fathomdb.errors import DependencyTraceError, EvidenceError


def _sha256_hex(body: str) -> str:
    return hashlib.sha256(body.encode()).hexdigest()


def _configure_acl_and_confidence(engine: Engine) -> None:
    engine.configure_projections(
        [
            ProjectionSpec(name="acl", roles=frozenset({ProjectionRole.FILTERABLE})),
            ProjectionSpec(name="confidence", roles=frozenset({ProjectionRole.FILTERABLE})),
        ]
    )


def _canonical_node(
    *,
    logical_id: str,
    kind: str,
    revision_id: str,
    source_version_id: str,
    source_id: str,
    body: str,
    properties: dict[str, Any],
) -> dict[str, Any]:
    return {
        "kind": kind,
        "body": body,
        "source_id": source_id,
        "logical_id": logical_id,
        "properties": properties,
        "provenance": {
            "schema_version": 1,
            "role": "canonical",
            "artifact_revision_id": revision_id,
            "source_version_id": source_version_id,
        },
    }


def _derived_node(
    *,
    logical_id: str,
    kind: str,
    revision_id: str,
    source_version_id: str,
    source_revision_id: str,
    source_id: str,
    source_body: str,
    body: str,
    properties: dict[str, Any],
) -> dict[str, Any]:
    return {
        "kind": kind,
        "body": body,
        "source_id": source_id,
        "logical_id": logical_id,
        "properties": properties,
        "provenance": {
            "schema_version": 1,
            "role": "derived",
            "artifact_revision_id": revision_id,
            "source_version_id": source_version_id,
            "source_revision_id": source_revision_id,
            "source_locator": {"kind": "whole_body"},
            "canonical_source_hash": {
                "algorithm": "sha256",
                "digest_hex": _sha256_hex(source_body),
            },
        },
    }


def _derived_edge(
    *,
    logical_id: str,
    kind: str,
    from_id: str,
    to_id: str,
    revision_id: str,
    source_version_id: str,
    source_revision_id: str,
    source_id: str,
    source_body: str,
) -> dict[str, Any]:
    return {
        "edge": {
            "from": from_id,
            "to": to_id,
            "kind": kind,
            "source_id": source_id,
            "logical_id": logical_id,
            "confidence": 0.95,
            "provenance": {
                "schema_version": 1,
                "role": "derived",
                "artifact_revision_id": revision_id,
                "source_version_id": source_version_id,
                "source_revision_id": source_revision_id,
                "source_locator": {"kind": "whole_body"},
                "canonical_source_hash": {
                    "algorithm": "sha256",
                    "digest_hex": _sha256_hex(source_body),
                },
            },
        }
    }


def test_ac32_evidence_prop_01_artifact_scoped_property_predicates_all_surfaces(
    db_path: str,
) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        _configure_acl_and_confidence(engine)
        source_body = "canonical team-a raw source without confidence property"
        engine.write(
            [
                _canonical_node(
                    logical_id="src-team-a",
                    kind="SourceDoc",
                    revision_id="src-team-a-r1",
                    source_version_id="src-team-a-v1",
                    source_id="team-a-bucket",
                    body=source_body,
                    properties={"acl": "team-a"},
                ),
                _derived_node(
                    logical_id="doc-root",
                    kind="Doc",
                    revision_id="doc-root-r1",
                    source_version_id="src-team-a-v1",
                    source_revision_id="src-team-a-r1",
                    source_id="team-a-bucket",
                    source_body=source_body,
                    body="root anchor document",
                    properties={"acl": "team-a", "confidence": 0.99},
                ),
                # Shorter body for low-confidence artifact so limit=1 proves filtering precedes truncation.
                _derived_node(
                    logical_id="doc-low",
                    kind="Doc",
                    revision_id="doc-low-r1",
                    source_version_id="src-team-a-v1",
                    source_revision_id="src-team-a-r1",
                    source_id="team-a-bucket",
                    source_body=source_body,
                    body="pyheliosneedle",
                    properties={"acl": "team-a", "confidence": 0.25},
                ),
                _derived_node(
                    logical_id="doc-high",
                    kind="Doc",
                    revision_id="doc-high-r1",
                    source_version_id="src-team-a-v1",
                    source_revision_id="src-team-a-r1",
                    source_id="team-a-bucket",
                    source_body=source_body,
                    body="pyheliosneedle high confidence extracted claim with extra context words",
                    properties={"acl": "team-a", "confidence": 0.95},
                ),
                _derived_edge(
                    logical_id="edge-high",
                    kind="supports",
                    from_id="doc-root",
                    to_id="doc-high",
                    revision_id="edge-high-r1",
                    source_version_id="src-team-a-v1",
                    source_revision_id="src-team-a-r1",
                    source_id="team-a-bucket",
                    source_body=source_body,
                ),
                _derived_edge(
                    logical_id="edge-low",
                    kind="weakly_supports",
                    from_id="doc-root",
                    to_id="doc-low",
                    revision_id="edge-low-r1",
                    source_version_id="src-team-a-v1",
                    source_revision_id="src-team-a-r1",
                    source_id="team-a-bucket",
                    source_body=source_body,
                ),
            ]
        )
        engine.drain()

        eligibility = SearchFilter(
            kind="Doc",
            attributes=(("acl", "team-a"),),
            property_predicates=[PropertyPredicate("confidence", "gte", 0.8)],
        )
        frozen = engine.freeze_read_context(
            ReadContextV1(view=ReadView(), eligibility=eligibility)
        )

        # 1. search_with_evidence
        search_res = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="pyheliosneedle", context=frozen, limit=1)
        )
        assert len(search_res.search_result.results) == 1
        assert search_res.search_result.results[0].id.value == "doc-high"
        assert len(search_res.evidence) == 1
        assert search_res.evidence[0].artifact_revision_id == "doc-high-r1"

        # 2. resolve_evidence
        resolved = engine.resolve_evidence(
            EvidenceResolveRequestV1(
                evidence_ref=search_res.evidence[0].evidence_ref,
                context=frozen,
            )
        )
        assert resolved.artifact_revision_id == "doc-high-r1"
        assert resolved.source_revision_id == "src-team-a-r1"
        assert resolved.canonical_source_body == source_body

        # 3. resolve_evidence_many
        many = engine.resolve_evidence_many(
            EvidenceResolveManyRequestV1(
                evidence_refs=[search_res.evidence[0].evidence_ref],
                context=frozen,
            )
        )
        assert len(many) == 1
        assert many[0].status == "resolved"
        assert many[0].resolved is not None
        assert many[0].resolved.source_revision_id == "src-team-a-r1"
        assert many[0].resolved.canonical_source_body == source_body

        # 4. graph.expand(include_evidence=True) + resolve_graph_evidence
        expanded = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=GraphExplicitSeedV1(
                    schema_version=1,
                    type="explicit",
                    logical_ids=(IdSpace(space="logical", value="doc-root"),),
                ),
                direction="outgoing",
                edge_kinds=("supports", "weakly_supports"),
                target_kinds=(),
                context=FrozenGraphReadContextV1(
                    schema_version=1,
                    type="frozen",
                    context=frozen,
                ),
                max_depth=1,
                result_limit=10,
                max_work_units="10000",
                include_explanation=False,
                include_evidence=True,
            ),
        )
        assert [target.logical_id for target in expanded.targets] == ["doc-high"]
        assert expanded.evidence is not None
        entry = expanded.evidence.entries[0]

        resolved_target = engine.resolve_graph_evidence(
            GraphEvidenceResolveRequestV1(
                evidence_ref=entry.target_evidence_ref,
                context=frozen,
            )
        )
        assert resolved_target.artifact_revision_id == "doc-high-r1"
        assert resolved_target.source_revision_id == "src-team-a-r1"
        assert resolved_target.canonical_source_body == source_body

        resolved_edge = engine.resolve_graph_evidence(
            GraphEvidenceResolveRequestV1(
                evidence_ref=entry.terminal_edge_evidence_ref,
                context=frozen,
            )
        )
        assert resolved_edge.artifact_revision_id == "edge-high-r1"
        assert resolved_edge.source_revision_id == "src-team-a-r1"
    finally:
        engine.close()


def test_ac32_evidence_prop_02_source_attributes_and_scope_remain_fail_closed(
    db_path: str,
    tmp_path: Path,
) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        _configure_acl_and_confidence(engine)
        src_a_body = "team-a canonical source"
        src_b_body = "team-b restricted canonical source"
        engine.write(
            [
                _canonical_node(
                    logical_id="src-a",
                    kind="SourceDoc",
                    revision_id="src-a-r1",
                    source_version_id="src-a-v1",
                    source_id="bucket-a",
                    body=src_a_body,
                    properties={"acl": "team-a"},
                ),
                _canonical_node(
                    logical_id="src-b",
                    kind="SourceDoc",
                    revision_id="src-b-r1",
                    source_version_id="src-b-v1",
                    source_id="bucket-b",
                    body=src_b_body,
                    properties={"acl": "team-b"},
                ),
                _derived_node(
                    logical_id="doc-root",
                    kind="Doc",
                    revision_id="doc-root-r1",
                    source_version_id="src-a-v1",
                    source_revision_id="src-a-r1",
                    source_id="bucket-a",
                    source_body=src_a_body,
                    body="root anchor",
                    properties={"acl": "team-a", "confidence": 0.99},
                ),
                _derived_node(
                    logical_id="doc-from-b",
                    kind="Doc",
                    revision_id="doc-from-b-r1",
                    source_version_id="src-b-v1",
                    source_revision_id="src-b-r1",
                    source_id="bucket-b",
                    source_body=src_b_body,
                    body="pyrestrictedneedle",
                    properties={"acl": "team-a", "confidence": 0.95},
                ),
                _derived_edge(
                    logical_id="edge-to-b",
                    kind="supports",
                    from_id="doc-root",
                    to_id="doc-from-b",
                    revision_id="edge-to-b-r1",
                    source_version_id="src-a-v1",
                    source_revision_id="src-a-r1",
                    source_id="bucket-a",
                    source_body=src_a_body,
                ),
            ]
        )
        engine.drain()

        eligibility = SearchFilter(
            kind="Doc",
            attributes=(("acl", "team-a"),),
            property_predicates=[PropertyPredicate("confidence", "gte", 0.8)],
        )
        frozen = engine.freeze_read_context(
            ReadContextV1(view=ReadView(), eligibility=eligibility)
        )

        with pytest.raises(EvidenceError) as ranked_err:
            engine.search_with_evidence(
                EvidenceSearchRequestV1(query="pyrestrictedneedle", context=frozen)
            )
        assert ranked_err.value.reason == "evidence_unavailable"
        assert ranked_err.value.field_path == "/evidenceRef"

        with pytest.raises(EvidenceError) as graph_err:
            graph.expand(
                engine,
                GraphExpandRequestV1(
                    schema_version=1,
                    seed=GraphExplicitSeedV1(
                        schema_version=1,
                        type="explicit",
                        logical_ids=(IdSpace(space="logical", value="doc-root"),),
                    ),
                    direction="outgoing",
                    edge_kinds=("supports",),
                    target_kinds=(),
                    context=FrozenGraphReadContextV1(
                        schema_version=1,
                        type="frozen",
                        context=frozen,
                    ),
                    max_depth=1,
                    result_limit=10,
                    max_work_units="10000",
                    include_explanation=False,
                    include_evidence=True,
                ),
            )
        assert graph_err.value.reason == "evidence_unavailable"
        assert graph_err.value.field_path == "/evidence"
    finally:
        engine.close()

    # Cross-scope isolation under Option A
    scoped_db = str(tmp_path / "scoped-prop.fathom")
    eng_a = Engine.open(
        scoped_db,
        use_default_embedder=False,
        scope_policy=ScopePolicy.single("tenant-a"),
    )
    try:
        _configure_acl_and_confidence(eng_a)
        tenant_a_body = "tenant-a canonical source bytes"
        eng_a.write(
            [
                _canonical_node(
                    logical_id="src-ta",
                    kind="SourceDoc",
                    revision_id="src-ta-r1",
                    source_version_id="src-ta-v1",
                    source_id="bucket-ta",
                    body=tenant_a_body,
                    properties={"acl": "team-a"},
                ),
                _derived_node(
                    logical_id="doc-ta",
                    kind="Doc",
                    revision_id="doc-ta-r1",
                    source_version_id="src-ta-v1",
                    source_revision_id="src-ta-r1",
                    source_id="bucket-ta",
                    source_body=tenant_a_body,
                    body="pyscopedoptionaneedle",
                    properties={"acl": "team-a", "confidence": 0.95},
                ),
            ]
        )
        eng_a.drain()
        elig = SearchFilter(
            kind="Doc",
            attributes=(("acl", "team-a"),),
            property_predicates=[PropertyPredicate("confidence", "gte", 0.8)],
        )
        frozen_a = eng_a.freeze_read_context(ReadContextV1(view=ReadView(), eligibility=elig))
        res_a = eng_a.search_with_evidence(
            EvidenceSearchRequestV1(query="pyscopedoptionaneedle", context=frozen_a)
        )
        ref_a = res_a.evidence[0].evidence_ref
        resolved_a = eng_a.resolve_evidence(
            EvidenceResolveRequestV1(evidence_ref=ref_a, context=frozen_a)
        )
        assert resolved_a.canonical_source_body == tenant_a_body
    finally:
        eng_a.close()

    eng_b = Engine.open(
        scoped_db,
        use_default_embedder=False,
        scope_policy=ScopePolicy.single("tenant-b"),
    )
    try:
        frozen_b = eng_b.freeze_read_context(
            ReadContextV1(
                view=ReadView(),
                eligibility=SearchFilter(
                    kind="Doc",
                    attributes=(("acl", "team-a"),),
                    property_predicates=[PropertyPredicate("confidence", "gte", 0.8)],
                ),
            )
        )
        res_b = eng_b.search_with_evidence(
            EvidenceSearchRequestV1(query="pyscopedoptionaneedle", context=frozen_b)
        )
        assert res_b.search_result.results == []
        with pytest.raises(EvidenceError) as cross_scope_err:
            eng_b.resolve_evidence(
                EvidenceResolveRequestV1(evidence_ref=ref_a, context=frozen_b)
            )
        assert cross_scope_err.value.reason == "evidence_unavailable"
        assert cross_scope_err.value.field_path == "/evidenceRef"
    finally:
        eng_b.close()


def test_ac32_evidence_prop_03_trace_symmetry_and_graph_arm_source_id_distinction(
    db_path: str,
) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        _configure_acl_and_confidence(engine)
        node_src_body = "canonical node provenance without confidence"
        edge_src_body = "canonical edge provenance without confidence"
        engine.write(
            [
                _canonical_node(
                    logical_id="node-src",
                    kind="SourceDoc",
                    revision_id="node-src-r1",
                    source_version_id="node-src-v1",
                    source_id="node-source-bucket",
                    body=node_src_body,
                    properties={"acl": "team-a"},
                ),
                _canonical_node(
                    logical_id="edge-src",
                    kind="SourceDoc",
                    revision_id="edge-src-r1",
                    source_version_id="edge-src-v1",
                    source_id="edge-source-bucket",
                    body=edge_src_body,
                    properties={"acl": "team-a"},
                ),
                _derived_node(
                    logical_id="ent-seed",
                    kind="Doc",
                    revision_id="ent-seed-r1",
                    source_version_id="node-src-v1",
                    source_revision_id="node-src-r1",
                    source_id="node-source-bucket",
                    source_body=node_src_body,
                    body="pyquasarseedtoken",
                    properties={"acl": "team-a", "confidence": 0.95},
                ),
                _derived_node(
                    logical_id="ent-neighbor",
                    kind="Doc",
                    revision_id="ent-neighbor-r1",
                    source_version_id="node-src-v1",
                    source_revision_id="node-src-r1",
                    source_id="node-source-bucket",
                    source_body=node_src_body,
                    body="unrelated neighbor text",
                    properties={"acl": "team-a", "confidence": 0.92},
                ),
                _derived_edge(
                    logical_id="ent-link",
                    kind="links",
                    from_id="ent-seed",
                    to_id="ent-neighbor",
                    revision_id="ent-link-r1",
                    source_version_id="edge-src-v1",
                    source_revision_id="edge-src-r1",
                    source_id="edge-source-bucket",
                    source_body=edge_src_body,
                ),
            ]
        )
        engine.register_source_dependency(
            {
                "schema_version": 1,
                "dependency_id": "dep-asym",
                "source_revision_id": "node-src-r1",
                "derived_revision_id": "ent-seed-r1",
            }
        )
        engine.drain()

        # trace_dependency remains symmetric (source omits confidence -> omitted from to_source)
        trace_frozen = engine.freeze_read_context(
            ReadContextV1(
                view=ReadView(),
                eligibility=SearchFilter(
                    attributes=(("acl", "team-a"),),
                    property_predicates=[PropertyPredicate("confidence", "gte", 0.8)],
                ),
            )
        )
        trace_res = engine.trace_dependency(
            DependencyTraceRequestV1(
                root_revision_id="ent-seed-r1",
                direction="to_source",
                context=trace_frozen,
            )
        )
        assert [n.artifact_revision_id for n in trace_res.nodes] == ["ent-seed-r1"]
        assert trace_res.dependency_edges == ()

        with pytest.raises(DependencyTraceError) as trace_err:
            engine.trace_dependency(
                DependencyTraceRequestV1(
                    root_revision_id="node-src-r1",
                    direction="to_dependents",
                    context=trace_frozen,
                )
            )
        assert trace_err.value.reason == "trace_unavailable"

        # GraphArm: SearchHit.source_id is traversed edge source_id while
        # ResolvedEvidenceV1.source_id is returned node artifact canonical provenance.
        graph_arm_frozen = engine.freeze_read_context(
            ReadContextV1(
                view=ReadView(),
                eligibility=SearchFilter(
                    kind="Doc",
                    attributes=(("acl", "team-a"),),
                    property_predicates=[PropertyPredicate("confidence", "gte", 0.8)],
                ),
            )
        )
        ga_res = engine.search_with_evidence(
            EvidenceSearchRequestV1(
                query="pyquasarseedtoken",
                context=graph_arm_frozen,
                use_graph_arm=True,
            )
        )
        neighbor_hits = [
            (idx, hit)
            for idx, hit in enumerate(ga_res.search_result.results)
            if hit.id.value == "ent-neighbor" and hit.branch == "graph_arm"
        ]
        assert len(neighbor_hits) == 1
        neighbor_idx, neighbor_hit = neighbor_hits[0]
        assert neighbor_hit.source_id == "edge-source-bucket"

        resolved_neighbor = engine.resolve_evidence(
            EvidenceResolveRequestV1(
                evidence_ref=ga_res.evidence[neighbor_idx].evidence_ref,
                context=graph_arm_frozen,
            )
        )
        assert resolved_neighbor.artifact_revision_id == "ent-neighbor-r1"
        assert resolved_neighbor.source_id == "node-source-bucket"
        assert resolved_neighbor.source_id != neighbor_hit.source_id
        assert resolved_neighbor.canonical_source_body == node_src_body
    finally:
        engine.close()
