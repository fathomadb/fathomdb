"""0.8.31 Slice 40 (`HISTORY-GRAPH`) — Layer 3 Python SDK parity tests.

Verifies:
- Opt-in bitemporal `graph.expand` via `GraphExpandRequestV1.as_of_write_cursor`
  and `GraphExpandRequestV1.as_of_epoch_ms` composing orthogonally with
  `ReadView.valid_as_of` across the 4-quadrant matrix.
- Synchronized historical node/edge revision hydration (`kind`, `body`,
  `properties`, `write_cursor`, `origin.terminal_edge_properties`).
- `as_of_epoch_ms` commit-timestamp lookup, cursor-`0` empty behavior, and
  future-cursor / `u64::MAX` snapshot-bounded monotonicity.
- Declared `Filterable` inline eligibility fallback (`eligibility.attributes`,
  `eligibility.property_predicates`, `edge_predicates`, `target_predicates`)
  on superseded historical revisions.
- Historical frozen evidence (`include_evidence=True` + `resolve_graph_evidence`)
  reporting `artifact_lifecycle.superseded == True`, historical explanation
  `dependency_state == "registered"`, and revocation on `transition("deleted")`,
  `purge`, and `erase_source`.
- Closed wire validation: canonical wire omission when unset, mutual exclusion
  (`/asOfEpochMs`), explicit JSON `null` refusal, invalid cursor/epoch bounds,
  and preserved `ReadView.include_superseded` / `include_inactive` refusals.
"""

from __future__ import annotations

import hashlib
import json
import sqlite3
import time
from typing import Any, cast

import pytest

from fathomdb import (
    CurrentGraphReadContextV1,
    Engine,
    FrozenGraphReadContextV1,
    GraphEvidenceResolveRequestV1,
    GraphExpandRequestV1,
    GraphExplicitSeedV1,
    GraphQuerySeedV1,
    IdSpace,
    ProjectionRole,
    ProjectionSpec,
    PropertyPredicate,
    ReadContextV1,
    ReadView,
    SearchFilter,
    graph,
)
from fathomdb.errors import EvidenceError, GraphExpansionError
from fathomdb.graph import _request_wire

U64_MAX_STR = str((1 << 64) - 1)
U64_OVERFLOW_STR = str(1 << 64)
I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1


def _current_ctx(
    *,
    valid_as_of: int | None = None,
    eligibility: SearchFilter | None = None,
    include_superseded: bool = False,
    include_inactive: bool = False,
) -> CurrentGraphReadContextV1:
    return CurrentGraphReadContextV1(
        schema_version=1,
        type="current",
        context=ReadContextV1(
            view=ReadView(
                include_superseded=include_superseded,
                include_inactive=include_inactive,
                valid_as_of=valid_as_of,
            ),
            eligibility=eligibility or SearchFilter(),
        ),
    )


def _explicit_req(
    *logical_ids: str,
    context: CurrentGraphReadContextV1 | FrozenGraphReadContextV1 | None = None,
    direction: str = "outgoing",
    edge_kinds: tuple[str, ...] = (),
    target_kinds: tuple[str, ...] = (),
    edge_predicates: tuple[PropertyPredicate, ...] = (),
    target_predicates: tuple[PropertyPredicate, ...] = (),
    max_depth: int = 2,
    result_limit: int = 20,
    max_work_units: str = "1000",
    include_explanation: bool = False,
    include_evidence: bool = False,
    as_of_write_cursor: str | None = None,
    as_of_epoch_ms: int | None = None,
) -> GraphExpandRequestV1:
    return GraphExpandRequestV1(
        schema_version=1,
        seed=GraphExplicitSeedV1(
            schema_version=1,
            type="explicit",
            logical_ids=tuple(IdSpace(space="logical", value=lid) for lid in logical_ids),
        ),
        direction=cast(Any, direction),
        edge_kinds=edge_kinds,
        target_kinds=target_kinds,
        context=context or _current_ctx(),
        max_depth=max_depth,
        result_limit=result_limit,
        max_work_units=max_work_units,
        include_explanation=include_explanation,
        include_evidence=include_evidence,
        edge_predicates=edge_predicates,
        target_predicates=target_predicates,
        as_of_write_cursor=as_of_write_cursor,
        as_of_epoch_ms=as_of_epoch_ms,
    )


def _canonical_prov(rev: str, ver: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "role": "canonical",
        "artifact_revision_id": rev,
        "source_version_id": ver,
    }


def _derived_prov(rev: str, src_rev: str, body: str, ver: str = "sv-1") -> dict[str, Any]:
    return {
        "schema_version": 1,
        "role": "derived",
        "artifact_revision_id": rev,
        "source_version_id": ver,
        "source_revision_id": src_rev,
        "source_locator": {"kind": "whole_body"},
        "canonical_source_hash": {
            "algorithm": "sha256",
            "digest_hex": hashlib.sha256(body.encode("utf-8")).hexdigest(),
        },
    }


def test_bitemporal_four_quadrant_hydration_and_cursor_bounds(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        r1 = engine.write(
            [
                {
                    "kind": "doc",
                    "body": "root alpha v1",
                    "source_id": "src:bi",
                    "logical_id": "root",
                    "valid_from": 50,
                    "valid_until": 500,
                },
                {
                    "kind": "claim",
                    "body": "target alpha v1",
                    "source_id": "src:bi",
                    "logical_id": "target_a",
                    "valid_from": 100,
                    "valid_until": 200,
                    "properties": {"version": 1, "label": "alpha-v1"},
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "root",
                        "to": "target_a",
                        "source_id": "src:bi",
                        "logical_id": "e-root-a",
                        "t_valid": 100,
                        "t_invalid": 200,
                        "properties": {"weight": 10},
                    }
                },
            ]
        )
        c1 = r1.cursor

        r2 = engine.write(
            [
                {
                    "kind": "finding",
                    "body": "target alpha v2",
                    "source_id": "src:bi",
                    "logical_id": "target_a",
                    "valid_from": 200,
                    "valid_until": 300,
                    "properties": {"version": 2, "label": "alpha-v2"},
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "root",
                        "to": "target_a",
                        "source_id": "src:bi",
                        "logical_id": "e-root-a",
                        "t_valid": 200,
                        "t_invalid": 300,
                        "properties": {"weight": 20},
                    }
                },
            ]
        )
        c2 = r2.cursor

        # Default current-time wire omits asOfWriteCursor and asOfEpochMs.
        default_wire = _request_wire(_explicit_req("root"))
        assert "asOfWriteCursor" not in default_wire
        assert "asOfEpochMs" not in default_wire

        # Quadrant 1: (as_of=c1, valid_as_of=150) -> historical v1
        q1 = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(valid_as_of=150),
                as_of_write_cursor=str(c1),
            ),
        )
        assert len(q1.targets) == 1
        t1 = q1.targets[0]
        assert (t1.logical_id, t1.kind, t1.body) == ("target_a", "claim", "target alpha v1")
        assert t1.properties == {"version": 1, "label": "alpha-v1"}
        assert t1.origin.terminal_edge_properties == {"weight": 10}
        assert int(t1.write_cursor) <= c1

        # Quadrant 2: (as_of=c1, valid_as_of=250) -> empty (v1 not valid at 250)
        q2 = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(valid_as_of=250),
                as_of_write_cursor=str(c1),
            ),
        )
        assert q2.targets == ()

        # Quadrant 3: (as_of=c2, valid_as_of=150) -> empty (v1 superseded, v2 not valid at 150)
        q3 = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(valid_as_of=150),
                as_of_write_cursor=str(c2),
            ),
        )
        assert q3.targets == ()

        # Quadrant 4: (as_of=c2, valid_as_of=250) -> v2
        q4 = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(valid_as_of=250),
                as_of_write_cursor=str(c2),
            ),
        )
        assert len(q4.targets) == 1
        t2 = q4.targets[0]
        assert (t2.logical_id, t2.kind, t2.body) == ("target_a", "finding", "target alpha v2")
        assert t2.properties == {"version": 2, "label": "alpha-v2"}
        assert t2.origin.terminal_edge_properties == {"weight": 20}

        # Future cursor (including u64::MAX) acts as snapshot-bounded upper bound.
        future = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(valid_as_of=250),
                as_of_write_cursor=U64_MAX_STR,
            ),
        )
        assert [t.logical_id for t in future.targets] == ["target_a"]
        assert future.targets[0].body == "target alpha v2"

        # Cursor 0: empty query seeds, explicit seed unavailable.
        q_zero = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=GraphQuerySeedV1(
                    schema_version=1, type="query", text="root alpha", ranked_limit=5
                ),
                direction="outgoing",
                edge_kinds=(),
                target_kinds=(),
                context=_current_ctx(valid_as_of=150),
                max_depth=1,
                result_limit=10,
                max_work_units="100",
                include_explanation=False,
                as_of_write_cursor="0",
            ),
        )
        assert q_zero.seeds == ()
        assert q_zero.targets == ()

        with pytest.raises(GraphExpansionError) as exc:
            graph.expand(
                engine,
                _explicit_req(
                    "root",
                    context=_current_ctx(valid_as_of=150),
                    as_of_write_cursor="0",
                ),
            )
        assert exc.value.reason == "graph_seed_unavailable"
        assert exc.value.field_path == "/seed/logicalIds/0"
    finally:
        engine.close()


def test_as_of_epoch_ms_resolution_and_before_first_commit(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        r1 = engine.write(
            [
                {"kind": "doc", "body": "epoch root", "source_id": "src:ep", "logical_id": "root"},
                {
                    "kind": "claim",
                    "body": "epoch target v1",
                    "source_id": "src:ep",
                    "logical_id": "t1",
                    "properties": {"rev": 1},
                },
                {
                    "edge": {
                        "kind": "links",
                        "from": "root",
                        "to": "t1",
                        "source_id": "src:ep",
                        "logical_id": "e1",
                        "properties": {"step": 1},
                    }
                },
            ]
        )
        time.sleep(0.015)
        r2 = engine.write(
            [
                {
                    "kind": "claim",
                    "body": "epoch target v2",
                    "source_id": "src:ep",
                    "logical_id": "t1",
                    "properties": {"rev": 2},
                },
                {
                    "edge": {
                        "kind": "links",
                        "from": "root",
                        "to": "t1",
                        "source_id": "src:ep",
                        "logical_id": "e1",
                        "properties": {"step": 2},
                    }
                },
            ]
        )

        with sqlite3.connect(db_path) as conn:
            row1 = conn.execute(
                "SELECT committed_at_ms FROM _fathomdb_commit_timestamps WHERE write_cursor = ?",
                (r1.cursor,),
            ).fetchone()
            row2 = conn.execute(
                "SELECT committed_at_ms FROM _fathomdb_commit_timestamps WHERE write_cursor = ?",
                (r2.cursor,),
            ).fetchone()
        assert row1 is not None and row2 is not None
        ts1, ts2 = int(row1[0]), int(row2[0])
        assert ts1 < ts2

        # Before first recorded commit -> cursor 0 (empty query seeds)
        res_before = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=GraphQuerySeedV1(
                    schema_version=1, type="query", text="epoch root", ranked_limit=5
                ),
                direction="outgoing",
                edge_kinds=(),
                target_kinds=(),
                context=_current_ctx(),
                max_depth=1,
                result_limit=10,
                max_work_units="100",
                include_explanation=False,
                as_of_epoch_ms=ts1 - 1,
            ),
        )
        assert res_before.seeds == ()
        assert res_before.targets == ()

        # At ts1 -> historical v1
        res_v1 = graph.expand(engine, _explicit_req("root", as_of_epoch_ms=ts1))
        assert len(res_v1.targets) == 1
        assert res_v1.targets[0].body == "epoch target v1"
        assert res_v1.targets[0].properties == {"rev": 1}
        assert res_v1.targets[0].origin.terminal_edge_properties == {"step": 1}

        # At ts2 and beyond -> v2
        for ts in (ts2, ts2 + 60_000):
            res_v2 = graph.expand(engine, _explicit_req("root", as_of_epoch_ms=ts))
            assert len(res_v2.targets) == 1
            assert res_v2.targets[0].body == "epoch target v2"
            assert res_v2.targets[0].properties == {"rev": 2}
            assert res_v2.targets[0].origin.terminal_edge_properties == {"step": 2}
    finally:
        engine.close()


def test_declared_filterable_inline_eligibility_on_superseded_revisions(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.configure_projections(
            [
                ProjectionSpec(
                    name="tier",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="region",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="weight",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="edge",
                ),
            ]
        )

        r1 = engine.write(
            [
                {
                    "kind": "entity",
                    "body": "root",
                    "source_id": "src:filt",
                    "logical_id": "root",
                    "properties": {"tier": "gold", "region": "us"},
                },
                {
                    "kind": "entity",
                    "body": "target gold v1",
                    "source_id": "src:filt",
                    "logical_id": "t_gold",
                    "properties": {"tier": "gold", "region": "us"},
                },
                {
                    "edge": {
                        "kind": "rel",
                        "from": "root",
                        "to": "t_gold",
                        "source_id": "src:filt",
                        "logical_id": "e_gold",
                        "properties": {"weight": 10},
                    }
                },
            ]
        )
        c1 = r1.cursor

        # Supersede with completely different values so active-only cpv no longer matches gold/us/10.
        engine.write(
            [
                {
                    "kind": "entity",
                    "body": "target bronze v2",
                    "source_id": "src:filt",
                    "logical_id": "t_gold",
                    "properties": {"tier": "bronze", "region": "eu"},
                },
                {
                    "edge": {
                        "kind": "rel",
                        "from": "root",
                        "to": "t_gold",
                        "source_id": "src:filt",
                        "logical_id": "e_gold",
                        "properties": {"weight": 1},
                    }
                },
            ]
        )
        engine.drain(timeout_s=30)

        elig = SearchFilter(
            attributes=(("region", "us"),),
            property_predicates=[PropertyPredicate(key="tier", op="eq", value="gold")],
        )
        # Current time excludes t_gold because current revision is bronze/eu/1.
        cur = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(eligibility=elig),
                edge_predicates=(PropertyPredicate(key="weight", op="gte", value=10),),
                target_predicates=(PropertyPredicate(key="tier", op="eq", value="gold"),),
            ),
        )
        assert cur.targets == ()

        # Historical as_of=c1 must evaluate declared Filterable keys inline on superseded v1 rows!
        hist = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(eligibility=elig),
                edge_predicates=(PropertyPredicate(key="weight", op="gte", value=10),),
                target_predicates=(PropertyPredicate(key="tier", op="eq", value="gold"),),
                as_of_write_cursor=str(c1),
            ),
        )
        assert len(hist.targets) == 1
        assert hist.targets[0].logical_id == "t_gold"
        assert hist.targets[0].body == "target gold v1"
        assert hist.targets[0].properties == {"tier": "gold", "region": "us"}
        assert hist.targets[0].origin.terminal_edge_properties == {"weight": 10}
    finally:
        engine.close()


def test_historical_frozen_evidence_superseded_and_revocation(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        src_body_v1 = "canonical source body v1"
        src_body_v2 = "canonical source body v2"

        r1 = engine.write(
            [
                {
                    "kind": "document",
                    "body": src_body_v1,
                    "source_id": "src:ev",
                    "logical_id": "src_doc",
                    "provenance": _canonical_prov("src-r1", "sv-1"),
                },
                {
                    "kind": "claim",
                    "body": "root v1",
                    "source_id": "src:ev",
                    "logical_id": "root",
                    "provenance": _derived_prov("root-r1", "src-r1", src_body_v1),
                },
                {
                    "kind": "claim",
                    "body": "target v1",
                    "source_id": "src:ev",
                    "logical_id": "target",
                    "properties": {"stage": "v1"},
                    "provenance": _derived_prov("target-r1", "src-r1", src_body_v1),
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "root",
                        "to": "target",
                        "source_id": "src:ev",
                        "logical_id": "edge_rt",
                        "properties": {"score": 1},
                        "provenance": _derived_prov("edge-r1", "src-r1", src_body_v1),
                    }
                },
                {
                    "kind": "claim",
                    "body": "erased target v1",
                    "source_id": "src:erase_me",
                    "logical_id": "erased_target",
                    "provenance": _canonical_prov("erased-r1", "sv-1"),
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "root",
                        "to": "erased_target",
                        "source_id": "src:erase_me",
                        "logical_id": "edge_erased",
                        "provenance": _derived_prov(
                            "edge-erased-r1", "erased-r1", "erased target v1"
                        ),
                    }
                },
            ]
        )
        c1 = r1.cursor

        engine.register_source_dependency(
            {
                "schema_version": 1,
                "dependency_id": "dep-target-1",
                "source_revision_id": "src-r1",
                "derived_revision_id": "target-r1",
            }
        )
        engine.register_source_dependency(
            {
                "schema_version": 1,
                "dependency_id": "dep-edge-1",
                "source_revision_id": "src-r1",
                "derived_revision_id": "edge-r1",
            }
        )

        # Supersede target and edge first (so target-r1 / edge-r1 remain active historical
        # revisions rather than being tombstoned by source supersession), then supersede src_doc.
        engine.write(
            [
                {
                    "kind": "document",
                    "body": src_body_v2,
                    "source_id": "src:ev",
                    "logical_id": "src_doc_b",
                    "provenance": _canonical_prov("src-b-r1", "sv-b1"),
                },
                {
                    "kind": "claim",
                    "body": "target v2",
                    "source_id": "src:ev",
                    "logical_id": "target",
                    "properties": {"stage": "v2"},
                    "provenance": _derived_prov("target-r2", "src-b-r1", src_body_v2, "sv-b1"),
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "root",
                        "to": "target",
                        "source_id": "src:ev",
                        "logical_id": "edge_rt",
                        "properties": {"score": 2},
                        "provenance": _derived_prov("edge-r2", "src-b-r1", src_body_v2, "sv-b1"),
                    }
                },
            ]
        )
        engine.write(
            [
                {
                    "kind": "document",
                    "body": src_body_v2,
                    "source_id": "src:ev",
                    "logical_id": "src_doc",
                    "provenance": _canonical_prov("src-r2", "sv-2"),
                }
            ]
        )
        engine.drain(timeout_s=30)

        frozen = engine.freeze_read_context(ReadContextV1())
        frozen_ctx = FrozenGraphReadContextV1(schema_version=1, type="frozen", context=frozen)

        res = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=frozen_ctx,
                edge_kinds=("supports",),
                target_kinds=("claim",),
                include_explanation=True,
                include_evidence=True,
                as_of_write_cursor=str(c1),
            ),
        )
        by_id = {t.logical_id: (idx, t) for idx, t in enumerate(res.targets)}
        assert "target" in by_id and "erased_target" in by_id
        target_idx, target_node = by_id["target"]
        erased_idx, _ = by_id["erased_target"]
        assert target_node.body == "target v1"
        assert target_node.properties == {"stage": "v1"}
        assert res.explanation is not None
        assert res.explanation.per_target[target_idx].dependency_state == "registered"
        assert res.evidence is not None

        target_entry = res.evidence.entries[target_idx]
        erased_entry = res.evidence.entries[erased_idx]
        assert target_entry.target_evidence_ref.startswith("fdbgev1.")
        assert len(target_entry.target_evidence_ref) == 704
        assert target_entry.terminal_edge_evidence_ref.startswith("fdbgev1.")
        assert len(target_entry.terminal_edge_evidence_ref) == 704

        resolved_target = engine.resolve_graph_evidence(
            GraphEvidenceResolveRequestV1(
                evidence_ref=target_entry.target_evidence_ref, context=frozen
            )
        )
        assert resolved_target.artifact.artifact_class == "node"
        assert resolved_target.artifact.body == "target v1"
        assert resolved_target.artifact.properties == {"stage": "v1"}
        assert resolved_target.canonical_source_body == src_body_v1
        assert resolved_target.artifact_lifecycle.superseded is True

        resolved_edge = engine.resolve_graph_evidence(
            GraphEvidenceResolveRequestV1(
                evidence_ref=target_entry.terminal_edge_evidence_ref, context=frozen
            )
        )
        assert resolved_edge.artifact.artifact_class == "edge"
        assert resolved_edge.artifact.properties == {"score": 1}
        assert resolved_edge.artifact_lifecycle.superseded is True

        # Transitioning active head of "target" to deleted removes it from historical as_of
        # and revokes its previously minted evidence reference.
        engine.transition("target", "deleted")
        after_delete = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(),
                edge_kinds=("supports",),
                target_kinds=("claim",),
                as_of_write_cursor=str(c1),
            ),
        )
        assert [t.logical_id for t in after_delete.targets] == ["erased_target"]
        reminted_del = engine.freeze_read_context(ReadContextV1())
        for ctx in (frozen, reminted_del):
            with pytest.raises(EvidenceError) as ev_del:
                engine.resolve_graph_evidence(
                    GraphEvidenceResolveRequestV1(
                        evidence_ref=target_entry.target_evidence_ref, context=ctx
                    )
                )
            assert ev_del.value.reason == "evidence_unavailable"

        # Physical erase_source revokes both historical replay and previously minted evidence.
        engine.erase_source("src:erase_me")
        after_erase = graph.expand(
            engine,
            _explicit_req(
                "root",
                context=_current_ctx(),
                edge_kinds=("supports",),
                target_kinds=("claim",),
                as_of_write_cursor=str(c1),
            ),
        )
        assert after_erase.targets == ()
        reminted_erase = engine.freeze_read_context(ReadContextV1())
        for ctx in (frozen, reminted_erase):
            with pytest.raises(EvidenceError) as ev_erased:
                engine.resolve_graph_evidence(
                    GraphEvidenceResolveRequestV1(
                        evidence_ref=erased_entry.target_evidence_ref, context=ctx
                    )
                )
            assert ev_erased.value.reason == "evidence_unavailable"
    finally:
        engine.close()


def test_as_of_validation_mutual_exclusion_null_and_bounds(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(
            [{"kind": "doc", "body": "seed", "source_id": "src:val", "logical_id": "root"}]
        )

        # Mutual exclusion -> GraphContextInvalid at /asOfEpochMs
        with pytest.raises(GraphExpansionError) as exc:
            graph.expand(
                engine,
                _explicit_req("root", as_of_write_cursor="1", as_of_epoch_ms=1_700_000_000_000),
            )
        assert (exc.value.reason, exc.value.field_path) == ("graph_context_invalid", "/asOfEpochMs")

        # Invalid as_of_write_cursor values
        for bad_cursor in (
            "",
            "01",
            "-1",
            "abc",
            "1\n",
            U64_OVERFLOW_STR,
            "9" * 30,
            123,
            True,
            1.5,
        ):
            with pytest.raises(GraphExpansionError) as exc:
                graph.expand(
                    engine,
                    _explicit_req("root", as_of_write_cursor=cast(Any, bad_cursor)),
                )
            assert (exc.value.reason, exc.value.field_path) == (
                "graph_context_invalid",
                "/asOfWriteCursor",
            ), f"failed for cursor={bad_cursor!r}"

        # Invalid as_of_epoch_ms values
        for bad_epoch in (True, False, 1.5, "100", I64_MIN - 1, I64_MAX + 1):
            with pytest.raises(GraphExpansionError) as exc:
                graph.expand(
                    engine,
                    _explicit_req("root", as_of_epoch_ms=cast(Any, bad_epoch)),
                )
            assert (exc.value.reason, exc.value.field_path) == (
                "graph_context_invalid",
                "/asOfEpochMs",
            ), f"failed for epoch={bad_epoch!r}"

        # Raw wire JSON explicit null -> refused by closed Rust codec at exact pointer
        base_wire = _request_wire(_explicit_req("root"))
        for key, pointer in (
            ("asOfWriteCursor", "/asOfWriteCursor"),
            ("asOfEpochMs", "/asOfEpochMs"),
        ):
            null_wire = {**base_wire, key: None}
            with pytest.raises(GraphExpansionError) as exc:
                engine._native.graph_expand(json.dumps(null_wire))
            assert (exc.value.reason, exc.value.field_path) == ("graph_context_invalid", pointer)

        # ReadView.include_superseded / include_inactive remain refused on graph.expand
        for bad_ctx in (
            _current_ctx(include_superseded=True),
            _current_ctx(include_inactive=True),
        ):
            with pytest.raises(GraphExpansionError) as exc:
                graph.expand(
                    engine,
                    _explicit_req("root", context=bad_ctx, as_of_write_cursor="1"),
                )
            assert (exc.value.reason, exc.value.field_path) == ("graph_context_invalid", "/context")
    finally:
        engine.close()
