"""0.8.32 Slice 41 (`WP-41D`, `ADR-0.8.32 #103`) — Python cross-binding tests.

Covers `AC32-FROZEN-STAB-01..03`, `AC32-FROZEN-STAB-05`, and `AC32-FROZEN-STAB-06`:
- Disjoint-kind additive `engine.write` and `engine.actuate` produce zero drift on
  frozen `kind="Doc"` search, evidence, pagination, and graph expansion contexts.
- `AppendOnlyLog` (`operational_mutations`) appends produce zero drift across all
  surfaces including `read.operational_state`; `LatestState` (`operational_state`)
  upserts produce zero drift on canonical/search/evidence/graph (including
  unfiltered `SearchFilter()`) while drifting `read.operational_state`.
- Cross-scope `tenant-b` additive writes, `transition`, `purge`, and `erase_source`
  produce zero drift on `tenant-a` `FrozenReadContextV1` across close/reopen.
- Fail-closed checks: same-`(scope, kind)` write, same-scope edge write on graph
  surfaces, and destructive mutations on any kind in `scope` fail closed.
- Schema 39 `fdbfr1` token lengths (`492 B` unscoped / `556 B` scoped, `<= 1024 B`).
"""

from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Any

import pytest

from fathomdb import (
    Engine,
    EvidenceResolveManyRequestV1,
    EvidenceResolveRequestV1,
    EvidenceSearchRequestV1,
    FrozenGraphReadContextV1,
    GraphEvidenceResolveRequestV1,
    GraphExpandRequestV1,
    GraphExplicitSeedV1,
    IdSpace,
    PageRequestV1,
    ReadContextV1,
    ScopePolicy,
    SearchFilter,
    graph,
    read,
)
from fathomdb.errors import EvidenceError, FrozenReadError


def _sha256_hex(body: str) -> str:
    return hashlib.sha256(body.encode()).hexdigest()


def _canonical_node(
    *,
    logical_id: str,
    kind: str,
    revision_id: str,
    source_version_id: str,
    source_id: str,
    body: str,
) -> dict[str, Any]:
    return {
        "kind": kind,
        "body": body,
        "source_id": source_id,
        "logical_id": logical_id,
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
) -> dict[str, Any]:
    return {
        "kind": kind,
        "body": body,
        "source_id": source_id,
        "logical_id": logical_id,
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


def _register_collection(engine: Engine, name: str, kind: str) -> None:
    engine.write(
        [
            {
                "admin_schema": {
                    "name": name,
                    "kind": kind,
                    "schema_json": "{}",
                    "retention_json": "{}",
                }
            }
        ]
    )


def _seed_doc_corpus(engine: Engine, prefix: str) -> str:
    source_body = f"{prefix} canonical aurora telemetry source"
    engine.write(
        [
            _canonical_node(
                logical_id=f"{prefix}-src-1",
                kind="SourceDoc",
                revision_id=f"{prefix}-src-r1",
                source_version_id=f"{prefix}-src-v1",
                source_id=f"{prefix}-source",
                body=source_body,
            ),
            _derived_node(
                logical_id=f"{prefix}-doc-1",
                kind="Doc",
                revision_id=f"{prefix}-doc-r1",
                source_version_id=f"{prefix}-src-v1",
                source_revision_id=f"{prefix}-src-r1",
                source_id=f"{prefix}-source",
                source_body=source_body,
                body=f"{prefix} aurora doc primary needle",
            ),
            _derived_node(
                logical_id=f"{prefix}-doc-2",
                kind="Doc",
                revision_id=f"{prefix}-doc-r2",
                source_version_id=f"{prefix}-src-v1",
                source_revision_id=f"{prefix}-src-r1",
                source_id=f"{prefix}-source",
                source_body=source_body,
                body=f"{prefix} aurora doc linked target",
            ),
            _derived_edge(
                logical_id=f"{prefix}-edge-1",
                kind="supports",
                from_id=f"{prefix}-doc-1",
                to_id=f"{prefix}-doc-2",
                revision_id=f"{prefix}-edge-r1",
                source_version_id=f"{prefix}-src-v1",
                source_revision_id=f"{prefix}-src-r1",
                source_id=f"{prefix}-source",
                source_body=source_body,
            ),
        ]
    )
    return source_body


def test_ac32_frozen_stab_01_disjoint_kind_write_and_actuate_zero_drift(
    db_path: str,
) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        _seed_doc_corpus(engine, "py1")
        ctx = engine.freeze_read_context(
            ReadContextV1(eligibility=SearchFilter(kind="Doc"))
        )
        # AC32-FROZEN-STAB-06: Schema 39 unscoped token size is 492 B (<= 1,024 B)
        assert len(ctx.token) == 492
        assert len(ctx.token) <= 1024

        # Disjoint-kind additive write
        engine.write(
            [
                _canonical_node(
                    logical_id="py1-turn-w1",
                    kind="AppTurn",
                    revision_id="py1-turn-w1-r1",
                    source_version_id="py1-turn-w1-v1",
                    source_id="py1-turns",
                    body="py1 disjoint write turn",
                )
            ]
        )

        # Disjoint-kind additive actuate
        receipt = engine.actuate(
            {
                "schema_version": 1,
                "operation_id": "py1-act-turn-1",
                "operations": [
                    {
                        "type": "put_canonical_node",
                        "record": _canonical_node(
                            logical_id="py1-turn-a1",
                            kind="AppTurn",
                            revision_id="py1-turn-a1-r1",
                            source_version_id="py1-turn-a1-v1",
                            source_id="py1-turns",
                            body="py1 disjoint actuated turn",
                        ),
                    }
                ],
            }  # type: ignore[arg-type]
        )
        assert receipt.outcome == "committed"

        # All frozen Doc surfaces remain stable
        search_res = engine.search_frozen("aurora", ctx)
        assert len(search_res.results) >= 1

        ev_res = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="aurora", context=ctx)
        )
        assert len(ev_res.evidence) >= 1
        single = engine.resolve_evidence(
            EvidenceResolveRequestV1(
                evidence_ref=ev_res.evidence[0].evidence_ref, context=ctx
            )
        )
        assert single.artifact_revision_id.startswith("py1-doc-r")

        many = engine.resolve_evidence_many(
            EvidenceResolveManyRequestV1(
                evidence_refs=[e.evidence_ref for e in ev_res.evidence],
                context=ctx,
            )
        )
        assert all(item.status == "resolved" for item in many)

        page = read.canonical_page(engine, "Doc", ctx, PageRequestV1(limit=10))
        assert len(page.items) == 2

        g_res = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=GraphExplicitSeedV1(
                    schema_version=1,
                    type="explicit",
                    logical_ids=(IdSpace(space="logical", value="py1-doc-1"),),
                ),
                direction="outgoing",
                edge_kinds=("supports",),
                target_kinds=(),
                context=FrozenGraphReadContextV1(
                    schema_version=1,
                    type="frozen",
                    context=ctx,
                ),
                max_depth=1,
                result_limit=10,
                max_work_units="10000",
                include_explanation=False,
                include_evidence=True,
            ),
        )
        assert len(g_res.targets) >= 1
        assert g_res.evidence is not None
        g_ev = engine.resolve_graph_evidence(
            GraphEvidenceResolveRequestV1(
                evidence_ref=g_res.evidence.entries[0].target_evidence_ref,
                context=ctx,
            )
        )
        assert g_ev.artifact_revision_id == "py1-doc-r2"
    finally:
        engine.close()


def test_ac32_frozen_stab_02_operational_mutations_and_latest_state_isolation(
    db_path: str,
) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        _seed_doc_corpus(engine, "py2")
        _register_collection(engine, "audit_log", "append_only_log")
        _register_collection(engine, "checkpoints", "latest_state")
        engine.write(
            [
                {
                    "op_store": {
                        "collection": "checkpoints",
                        "record_key": "cursor-1",
                        "body": '{"step":1}',
                    }
                }
            ]
        )

        ctx_doc = engine.freeze_read_context(
            ReadContextV1(eligibility=SearchFilter(kind="Doc"))
        )
        ctx_all = engine.freeze_read_context(
            ReadContextV1(eligibility=SearchFilter())
        )

        # AppendOnlyLog writes must not drift ANY surface (including operational_state)
        for i in range(10):
            engine.write(
                [
                    {
                        "op_store": {
                            "collection": "audit_log",
                            "record_key": f"evt-{i}",
                            "body": f'{{"seq":{i}}}',
                        }
                    }
                ]
            )

        assert len(engine.search_frozen("aurora", ctx_doc).results) >= 1
        assert len(engine.search_frozen("aurora", ctx_all).results) >= 1
        op_before = read.operational_state(
            engine, "checkpoints", "cursor-1", ctx_all
        )
        assert op_before is not None
        assert op_before.payload == '{"step":1}'

        # LatestState upsert (both via write and actuate PutLatestState):
        # zero drift on canonical/search/evidence/graph (even unfiltered),
        # but drifts ReadSurface::OperationalState!
        engine.write(
            [
                {
                    "op_store": {
                        "collection": "checkpoints",
                        "record_key": "cursor-1",
                        "body": '{"step":2}',
                    }
                }
            ]
        )
        act_receipt = engine.actuate(
            {
                "schema_version": 1,
                "operation_id": "py2-ls-act-1",
                "operations": [
                    {
                        "type": "put_latest_state",
                        "collection": "checkpoints",
                        "record_key": "cursor-1",
                        "payload_json": '{"step":3}',
                    }
                ],
            }  # type: ignore[arg-type]
        )
        assert act_receipt.outcome == "committed"

        assert len(engine.search_frozen("aurora", ctx_doc).results) >= 1
        assert len(engine.search_frozen("aurora", ctx_all).results) >= 1
        ev_all = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="aurora", context=ctx_all)
        )
        assert len(ev_all.evidence) >= 1
        assert len(read.canonical_page(engine, "Doc", ctx_doc, PageRequestV1(limit=10)).items) == 2

        with pytest.raises(FrozenReadError):
            read.operational_state(engine, "checkpoints", "cursor-1", ctx_all)
    finally:
        engine.close()


def test_ac32_frozen_stab_03_cross_scope_isolation_and_reopen(
    tmp_path: Path,
) -> None:
    db_file = str(tmp_path / "py_cross_scope.sqlite")
    policy_a = ScopePolicy.single("tenant-a")
    policy_b = ScopePolicy.single("tenant-b")

    # Seed tenant-a and freeze context
    eng_a = Engine.open(
        db_file, use_default_embedder=False, scope_policy=policy_a
    )
    try:
        _seed_doc_corpus(eng_a, "ta")
        ctx_a = eng_a.freeze_read_context(
            ReadContextV1(eligibility=SearchFilter(kind="Doc"))
        )
        # AC32-FROZEN-STAB-06: Schema 39 scoped token size is 556 B (<= 1,024 B)
        assert len(ctx_a.token) == 556
        assert len(ctx_a.token) <= 1024
        ev_before = eng_a.search_with_evidence(
            EvidenceSearchRequestV1(query="aurora", context=ctx_a)
        )
        assert len(ev_before.evidence) >= 1
        saved_ref = ev_before.evidence[0].evidence_ref
    finally:
        eng_a.close()

    # Mutate tenant-b: additive write, transition, purge, erase_source
    eng_b = Engine.open(
        db_file, use_default_embedder=False, scope_policy=policy_b
    )
    try:
        _seed_doc_corpus(eng_b, "tb")
        eng_b.write(
            [
                _canonical_node(
                    logical_id="tb-extra",
                    kind="Doc",
                    revision_id="tb-extra-r1",
                    source_version_id="tb-extra-v1",
                    source_id="tb-extra-src",
                    body="tb extra doc to transition and purge",
                )
            ]
        )
        eng_b.transition("tb-extra", "deleted", "tenant-b churn")
        eng_b.purge("tb-extra")
        eng_b.erase_source("tb-source")
    finally:
        eng_b.close()

    # Reopen tenant-a and verify zero drift on saved FrozenReadContextV1 & evidence ref
    eng_a2 = Engine.open(
        db_file, use_default_embedder=False, scope_policy=policy_a
    )
    try:
        assert len(eng_a2.search_frozen("aurora", ctx_a).results) >= 1
        resolved = eng_a2.resolve_evidence(
            EvidenceResolveRequestV1(evidence_ref=saved_ref, context=ctx_a)
        )
        assert resolved.artifact_revision_id.startswith("ta-doc-r")
        page = read.canonical_page(eng_a2, "Doc", ctx_a, PageRequestV1(limit=10))
        assert len(page.items) == 2
    finally:
        eng_a2.close()


def test_ac32_frozen_stab_05_fail_closed_same_kind_edges_and_destructive(
    db_path: str,
) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        source_body = _seed_doc_corpus(engine, "py5")
        ctx_doc = engine.freeze_read_context(
            ReadContextV1(eligibility=SearchFilter(kind="Doc"))
        )
        ev_res = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="aurora", context=ctx_doc)
        )
        doc_ref = ev_res.evidence[0].evidence_ref

        # Same-scope edge write of disjoint edge kind:
        # Graph surfaces fail closed, while node-only SearchNoGraph/EvidenceNode stay stable!
        engine.write(
            [
                _derived_edge(
                    logical_id="py5-edge-2",
                    kind="cites",
                    from_id="py5-doc-1",
                    to_id="py5-doc-2",
                    revision_id="py5-edge-r2",
                    source_version_id="py5-src-v1",
                    source_revision_id="py5-src-r1",
                    source_id="py5-source",
                    source_body=source_body,
                )
            ]
        )
        assert len(engine.search_frozen("aurora", ctx_doc).results) >= 1
        assert (
            engine.resolve_evidence(
                EvidenceResolveRequestV1(evidence_ref=doc_ref, context=ctx_doc)
            ).artifact_revision_id.startswith("py5-doc-r")
        )
        with pytest.raises(FrozenReadError):
            graph.expand(
                engine,
                GraphExpandRequestV1(
                    schema_version=1,
                    seed=GraphExplicitSeedV1(
                        schema_version=1,
                        type="explicit",
                        logical_ids=(IdSpace(space="logical", value="py5-doc-1"),),
                    ),
                    direction="outgoing",
                    edge_kinds=("supports",),
                    target_kinds=(),
                    context=FrozenGraphReadContextV1(
                        schema_version=1,
                        type="frozen",
                        context=ctx_doc,
                    ),
                    max_depth=1,
                    result_limit=10,
                    max_work_units="10000",
                    include_explanation=False,
                    include_evidence=False,
                ),
            )
        with pytest.raises(FrozenReadError):
            engine.search_expand_frozen("aurora", ctx_doc, 1)

        # Same-(scope, kind) write fails closed on search_frozen, resolve_evidence, canonical_page
        ctx_doc2 = engine.freeze_read_context(
            ReadContextV1(eligibility=SearchFilter(kind="Doc"))
        )
        ev_res2 = engine.search_with_evidence(
            EvidenceSearchRequestV1(query="aurora", context=ctx_doc2)
        )
        engine.write(
            [
                _canonical_node(
                    logical_id="py5-doc-new",
                    kind="Doc",
                    revision_id="py5-doc-new-r1",
                    source_version_id="py5-doc-new-v1",
                    source_id="py5-source",
                    body="py5 aurora same-kind doc mutation",
                )
            ]
        )
        with pytest.raises(FrozenReadError):
            engine.search_frozen("aurora", ctx_doc2)
        with pytest.raises(EvidenceError):
            engine.resolve_evidence(
                EvidenceResolveRequestV1(
                    evidence_ref=ev_res2.evidence[0].evidence_ref,
                    context=ctx_doc2,
                )
            )
        with pytest.raises(FrozenReadError):
            read.canonical_page(engine, "Doc", ctx_doc2, PageRequestV1(limit=10))

        # Destructive transition on a disjoint kind in the SAME scope fails closed
        engine.write(
            [
                _canonical_node(
                    logical_id="py5-turn-dest",
                    kind="AppTurn",
                    revision_id="py5-turn-dest-r1",
                    source_version_id="py5-turn-dest-v1",
                    source_id="py5-turns",
                    body="py5 turn to deprecate",
                )
            ]
        )
        ctx_doc3 = engine.freeze_read_context(
            ReadContextV1(eligibility=SearchFilter(kind="Doc"))
        )
        engine.transition("py5-turn-dest", "deleted", "destructive test")
        with pytest.raises(FrozenReadError):
            engine.search_frozen("aurora", ctx_doc3)
    finally:
        engine.close()
