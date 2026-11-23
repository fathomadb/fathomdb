"""0.8.31 Slice 30 (`STRICT-GRAPH`) — Layer 4 Python SDK parity tests.

Verifies:
- `GraphExpandRequestV1.edge_predicates` and `target_predicates` (all 6 ops,
  all 4 scalar types, missing key, cross-type, exact `i64 > 2^53`, `Bool`
  ordering refusal, `> 32` refusal, lone-surrogate / NUL / `\\x1e` / non-finite
  refusal, dict inputs, byte-identical unpredicated wire JSON omission, and
  `GraphTargetV1.properties` / `GraphOriginV1.terminal_edge_properties`).
- `use_graph_arm` opt-in `graph_seed` (`"graph_fts"`, `"fused_top_k"`, `"both"`),
  body-less property-graph edge seeding, `graph_seed_k` bounds,
  `graph_edge_kinds`, and `graph_edge_predicates` across `Engine.search`,
  `Engine.search_frozen`, and `EvidenceSearchRequestV1`.
- `PerHitExplain.graph_seed_source` and `PerHitExplain.graph_hop_depth`, plus
  `EvidenceGraphOriginV1.seed_source` via `resolve_evidence`.
"""

from __future__ import annotations

import hashlib
import math
from typing import Any, cast

import pytest

from fathomdb import (
    CurrentGraphReadContextV1,
    Engine,
    EvidenceResolveRequestV1,
    EvidenceSearchRequestV1,
    FrozenGraphReadContextV1,
    GraphArmSeedSource,
    GraphExpandRequestV1,
    GraphExplicitSeedV1,
    GraphSeedMode,
    IdSpace,
    PropertyPredicate,
    ReadContextV1,
    SearchFilter,
    graph,
)
from fathomdb.errors import (
    GraphExpansionError,
    InvalidArgumentError,
    InvalidFilterError,
    WriteValidationError,
)
from fathomdb.graph import _request_wire

I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1
BEYOND_F64_LO = (1 << 53) + 1
BEYOND_F64_HI = (1 << 53) + 2


def _current_ctx() -> CurrentGraphReadContextV1:
    return CurrentGraphReadContextV1(schema_version=1, type="current", context=ReadContextV1())


def _canonical_prov(rev: str, ver: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "role": "canonical",
        "artifact_revision_id": rev,
        "source_version_id": ver,
    }


def _derived_prov(rev: str, src_rev: str, body: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "role": "derived",
        "artifact_revision_id": rev,
        "source_version_id": "v1",
        "source_revision_id": src_rev,
        "source_locator": {"kind": "whole_body"},
        "canonical_source_hash": {
            "algorithm": "sha256",
            "digest_hex": hashlib.sha256(body.encode("utf-8")).hexdigest(),
        },
    }


def test_graph_expand_predicates_matrix_types_and_hydration(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "entity",
                    "body": "root node",
                    "source_id": "src:g",
                    "logical_id": "n:root",
                    "properties": {"role": "seed"},
                },
                {
                    "kind": "entity",
                    "body": "target alpha",
                    "source_id": "src:g",
                    "logical_id": "n:a",
                    "properties": {
                        "tag": "alpha",
                        "score": 10,
                        "big": BEYOND_F64_LO,
                        "ratio": 1.5,
                        "active": True,
                    },
                },
                {
                    "kind": "entity",
                    "body": "target beta",
                    "source_id": "src:g",
                    "logical_id": "n:b",
                    "properties": {
                        "tag": "beta",
                        "score": 20,
                        "big": BEYOND_F64_HI,
                        "ratio": 2.5,
                        "active": False,
                    },
                },
                {
                    "kind": "entity",
                    # JSON body with key "tag" but properties=None: predicates must NOT read body!
                    "body": '{"tag": "alpha", "score": 10}',
                    "source_id": "src:g",
                    "logical_id": "n:body_only",
                },
                {
                    "edge": {
                        "kind": "rel",
                        "from": "n:root",
                        "to": "n:a",
                        "source_id": "src:g",
                        "logical_id": "e:a_low",
                        "properties": {"weight": 1, "verified": False, "label": "low"},
                    }
                },
                {
                    # Parallel edge to n:a with higher weight (must not be poisoned by e:a_low failing)
                    "edge": {
                        "kind": "rel",
                        "from": "n:root",
                        "to": "n:a",
                        "source_id": "src:g",
                        "logical_id": "e:a_hi",
                        "properties": {
                            "weight": 10,
                            "verified": True,
                            "label": "hi",
                            "big_edge": BEYOND_F64_HI,
                            "rate": 0.75,
                        },
                    }
                },
                {
                    "edge": {
                        "kind": "rel",
                        "from": "n:root",
                        "to": "n:b",
                        "source_id": "src:g",
                        "logical_id": "e:b",
                        "properties": {"weight": 5, "verified": False, "label": "mid"},
                    }
                },
                {
                    "edge": {
                        "kind": "rel",
                        "from": "n:root",
                        "to": "n:body_only",
                        "source_id": "src:g",
                        "logical_id": "e:body_only",
                        "properties": {"weight": 10, "verified": True},
                    }
                },
            ]
        )

        seed = GraphExplicitSeedV1(
            schema_version=1,
            type="explicit",
            logical_ids=(IdSpace(space="logical", value="n:root"),),
        )

        # Unpredicated wire JSON omits edgePredicates/targetPredicates completely.
        unpred = GraphExpandRequestV1(
            schema_version=1,
            seed=seed,
            direction="outgoing",
            edge_kinds=("rel",),
            target_kinds=(),
            context=_current_ctx(),
            max_depth=1,
            result_limit=10,
            max_work_units="100",
            include_explanation=False,
        )
        wire = _request_wire(unpred)
        assert "edgePredicates" not in wire
        assert "targetPredicates" not in wire

        # Parallel-edge non-poisoning + exact hydration on GraphTargetV1 & GraphOriginV1
        res = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=seed,
                direction="outgoing",
                edge_kinds=("rel",),
                target_kinds=(),
                context=_current_ctx(),
                max_depth=1,
                result_limit=10,
                max_work_units="100",
                include_explanation=False,
                edge_predicates=(
                    PropertyPredicate(key="weight", op="gte", value=10),
                    PropertyPredicate(key="verified", op="eq", value=True),
                ),
                target_predicates=(PropertyPredicate(key="tag", op="eq", value="alpha"),),
            ),
        )
        assert [t.logical_id for t in res.targets] == ["n:a"]
        target = res.targets[0]
        assert target.properties is not None
        assert target.properties["tag"] == "alpha"
        assert target.properties["big"] == BEYOND_F64_LO
        assert target.properties["active"] is True
        assert target.origin.terminal_edge_properties is not None
        assert target.origin.terminal_edge_properties["label"] == "hi"
        assert target.origin.terminal_edge_properties["big_edge"] == BEYOND_F64_HI
        assert target.origin.terminal_edge_properties["verified"] is True

        # Exact i64 > 2^53 precision (BEYOND_F64_LO < BEYOND_F64_HI)
        res_big = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=seed,
                direction="outgoing",
                edge_kinds=("rel",),
                target_kinds=(),
                context=_current_ctx(),
                max_depth=1,
                result_limit=10,
                max_work_units="100",
                include_explanation=False,
                target_predicates=(PropertyPredicate(key="big", op="gt", value=BEYOND_F64_LO),),
            ),
        )
        assert [t.logical_id for t in res_big.targets] == ["n:b"]

        # All 6 ops over String / Int / Float / Bool (eq/ne on Bool)
        def expand_targets(
            *,
            edge_preds: Any = (),
            target_preds: Any = (),
        ) -> list[str]:
            out = graph.expand(
                engine,
                GraphExpandRequestV1(
                    schema_version=1,
                    seed=seed,
                    direction="outgoing",
                    edge_kinds=("rel",),
                    target_kinds=(),
                    context=_current_ctx(),
                    max_depth=1,
                    result_limit=10,
                    max_work_units="100",
                    include_explanation=False,
                    edge_predicates=edge_preds,
                    target_predicates=target_preds,
                ),
            )
            return sorted(t.logical_id for t in out.targets)

        assert expand_targets(target_preds=(PropertyPredicate("tag", "ne", "alpha"),)) == ["n:b"]
        assert expand_targets(target_preds=(PropertyPredicate("score", "lt", 20),)) == ["n:a"]
        assert expand_targets(target_preds=(PropertyPredicate("score", "lte", 20),)) == [
            "n:a",
            "n:b",
        ]
        assert expand_targets(target_preds=(PropertyPredicate("ratio", "gt", 1.5),)) == ["n:b"]
        assert expand_targets(target_preds=(PropertyPredicate("ratio", "gte", 1.5),)) == [
            "n:a",
            "n:b",
        ]
        assert expand_targets(target_preds=(PropertyPredicate("active", "ne", True),)) == ["n:b"]

        # Missing key evaluates False for ALL 6 ops (including ne!), never reading JSON body.
        for op in ("eq", "ne", "lt", "lte", "gt", "gte"):
            assert (
                expand_targets(target_preds=(PropertyPredicate("missing_key", cast(Any, op), "x"),))
                == []
            )
            assert "n:body_only" not in expand_targets(
                target_preds=(PropertyPredicate("tag", cast(Any, op), "zzz"),)
            )

        # Cross-type comparisons evaluate False for all ops including ne.
        for op in ("eq", "ne", "lt", "lte", "gt", "gte"):
            assert (
                expand_targets(target_preds=(PropertyPredicate("tag", cast(Any, op), 10),)) == []
            )
        assert expand_targets(target_preds=(PropertyPredicate("active", "ne", 1),)) == []
        assert expand_targets(target_preds=(PropertyPredicate("score", "ne", True),)) == []

        # Dict predicate inputs accepted cleanly alongside PropertyPredicate.
        assert expand_targets(
            edge_preds=({"key": "weight", "op": "eq", "value": 5},),
            target_preds=({"key": "tag", "op": "eq", "value": "beta"},),
        ) == ["n:b"]
    finally:
        engine.close()


def test_graph_expand_predicates_validation_refusals(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        seed = GraphExplicitSeedV1(
            schema_version=1,
            type="explicit",
            logical_ids=(IdSpace(space="logical", value="n:root"),),
        )

        def check_refusal(
            *,
            edge_preds: Any = (),
            target_preds: Any = (),
            expected_reason: str,
            expected_path: str | None = None,
        ) -> None:
            with pytest.raises(GraphExpansionError) as exc:
                graph.expand(
                    engine,
                    GraphExpandRequestV1(
                        schema_version=1,
                        seed=seed,
                        direction="outgoing",
                        edge_kinds=(),
                        target_kinds=(),
                        context=_current_ctx(),
                        max_depth=1,
                        result_limit=10,
                        max_work_units="100",
                        include_explanation=False,
                        edge_predicates=edge_preds,
                        target_predicates=target_preds,
                    ),
                )
            assert exc.value.reason == expected_reason
            if expected_path is not None:
                assert exc.value.field_path == expected_path

        # Bool ordering refused
        for op in ("lt", "lte", "gt", "gte"):
            check_refusal(
                edge_preds=(PropertyPredicate("flag", cast(Any, op), True),),
                expected_reason="graph_edge_predicates_invalid",
            )
            check_refusal(
                target_preds=(PropertyPredicate("flag", cast(Any, op), False),),
                expected_reason="graph_target_predicates_invalid",
            )

        # > 32 predicates refused
        too_many = tuple(PropertyPredicate(f"k{i}", "eq", i) for i in range(33))
        check_refusal(edge_preds=too_many, expected_reason="graph_edge_predicates_invalid")
        check_refusal(target_preds=too_many, expected_reason="graph_target_predicates_invalid")

        # Malformed keys/values: empty key, NUL, RS, lone surrogate, non-finite float, > i64
        bad_preds: list[Any] = [
            PropertyPredicate("", "eq", "v"),
            PropertyPredicate("bad\x00key", "eq", "v"),
            PropertyPredicate("bad\x1ekey", "eq", "v"),
            PropertyPredicate("bad\ud800key", "eq", "v"),
            PropertyPredicate("k", "eq", "bad\x00val"),
            PropertyPredicate("k", "eq", "bad\x1eval"),
            PropertyPredicate("k", "eq", "bad\ud800val"),
            PropertyPredicate("k", "eq", math.nan),
            PropertyPredicate("k", "eq", math.inf),
            PropertyPredicate("k", "eq", I64_MAX + 1),
            PropertyPredicate("k", "eq", I64_MIN - 1),
            {"key": "k", "op": "like", "value": "v"},
        ]
        for bad in bad_preds:
            check_refusal(edge_preds=(bad,), expected_reason="graph_edge_predicates_invalid")
            check_refusal(target_preds=(bad,), expected_reason="graph_target_predicates_invalid")

        # Closed wire semantics parity with Rust/TS: unknown key -> unknown_field + RFC 6901 pointer,
        # missing required field -> predicate invalid + per-field pointer.
        extra_dict = {"key": "k", "op": "eq", "value": "v", "extra": 1}
        check_refusal(
            edge_preds=(extra_dict,),
            expected_reason="unknown_field",
            expected_path="/edgePredicates/0/extra",
        )
        check_refusal(
            target_preds=(extra_dict,),
            expected_reason="unknown_field",
            expected_path="/targetPredicates/0/extra",
        )
        check_refusal(
            edge_preds=({"key": "k", "op": "eq"},),
            expected_reason="graph_edge_predicates_invalid",
            expected_path="/edgePredicates/0/value",
        )
    finally:
        engine.close()


def test_search_graph_arm_seeding_predicates_explain_and_evidence(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        src_body = "quantum observatory telemetry anchor report"
        engine.write(
            [
                {
                    "kind": "note",
                    "body": src_body,
                    "source_id": "src:seed",
                    "logical_id": "n:anchor",
                    "provenance": _canonical_prov("rev:anchor", "v1"),
                },
                {
                    "kind": "note",
                    "body": "celestial neighbor delta",
                    "source_id": "src:seed",
                    "logical_id": "n:bodyless_neighbor",
                    "provenance": _derived_prov("rev:nbr1", "rev:anchor", src_body),
                },
                {
                    "kind": "note",
                    "body": "filtered out neighbor gamma",
                    "source_id": "src:seed",
                    "logical_id": "n:filtered_neighbor",
                    "provenance": _derived_prov("rev:nbr2", "rev:anchor", src_body),
                },
                {
                    "kind": "note",
                    "body": "edge fts companion epsilon",
                    "source_id": "src:seed",
                    "logical_id": "n:edge_fts_target",
                    "provenance": _derived_prov("rev:nbr3", "rev:anchor", src_body),
                },
                # Body-less property-graph edge (NOT in search_index_edges!)
                {
                    "edge": {
                        "kind": "supports",
                        "from": "n:anchor",
                        "to": "n:bodyless_neighbor",
                        "source_id": "src:seed",
                        "logical_id": "e:bodyless_hi",
                        "properties": {"strength": 9, "tier": "gold"},
                        "provenance": _derived_prov("rev:e1", "rev:anchor", src_body),
                    }
                },
                # Second body-less edge with lower strength and different kind
                {
                    "edge": {
                        "kind": "weak_link",
                        "from": "n:anchor",
                        "to": "n:filtered_neighbor",
                        "source_id": "src:seed",
                        "logical_id": "e:bodyless_lo",
                        "properties": {"strength": 2, "tier": "bronze"},
                        "provenance": _derived_prov("rev:e2", "rev:anchor", src_body),
                    }
                },
                # Edge WITH body matching the query (triggers EdgeFts under graph_fts / both)
                {
                    "edge": {
                        "kind": "mentions",
                        "from": "n:anchor",
                        "to": "n:edge_fts_target",
                        "body": "observatory telemetry citation link",
                        "source_id": "src:seed",
                        "logical_id": "e:fts",
                        "properties": {"strength": 7, "tier": "silver"},
                        "provenance": _derived_prov("rev:e3", "rev:anchor", src_body),
                    }
                },
            ]
        )

        # Confirm type aliases are importable and usable
        mode: GraphSeedMode = "fused_top_k"
        expected_source: GraphArmSeedSource = "fused_top_k"

        # 1. Default graph_seed="graph_fts" seeds via EntityFts on lexical node match,
        #    exposing graph_seed_source="entity_fts" and graph_hop_depth=1.
        default_res = engine.search(
            "observatory telemetry",
            use_graph_arm=True,
            explain=True,
            graph_seed="graph_fts",
            graph_edge_kinds=["supports"],
        )
        assert default_res.explanation is not None
        default_ids = [h.id.value for h in default_res.results]
        assert "n:bodyless_neighbor" in default_ids
        def_nbr_idx = default_ids.index("n:bodyless_neighbor")
        assert default_res.explanation.per_hit[def_nbr_idx].graph_seed_source == "entity_fts"
        assert default_res.explanation.per_hit[def_nbr_idx].graph_hop_depth == 1
        for hit, exp in zip(default_res.results, default_res.explanation.per_hit):
            if hit.branch != "graph_arm":
                assert exp.graph_seed_source is None
                assert exp.graph_hop_depth is None

        # 2. Opt-in graph_seed="fused_top_k" seeds from n:anchor and traverses the body-less edge!
        fused_res = engine.search(
            "observatory telemetry",
            use_graph_arm=True,
            explain=True,
            graph_seed=mode,
            graph_seed_k=5,
            graph_edge_kinds=["supports", "weak_link"],
            graph_edge_predicates=[PropertyPredicate("strength", "gte", 5)],
        )
        assert fused_res.explanation is not None
        hit_ids = [h.id.value for h in fused_res.results]
        assert "n:bodyless_neighbor" in hit_ids
        assert "n:filtered_neighbor" not in hit_ids
        nbr_idx = hit_ids.index("n:bodyless_neighbor")
        assert fused_res.results[nbr_idx].branch == "graph_arm"
        nbr_exp = fused_res.explanation.per_hit[nbr_idx]
        assert nbr_exp.graph_seed_source == expected_source
        assert nbr_exp.graph_hop_depth == 1

        # 3. graph_seed="both" combines FusedTopK and GraphFts deterministically.
        both_res = engine.search(
            "observatory telemetry",
            use_graph_arm=True,
            explain=True,
            graph_seed="both",
            graph_seed_k=5,
            graph_edge_kinds=["supports", "mentions"],
        )
        both_ids = [h.id.value for h in both_res.results]
        assert "n:bodyless_neighbor" in both_ids
        assert "n:edge_fts_target" in both_ids

        # 4. Engine.search_frozen supports all 4 graph-seeding kwargs identically.
        frozen = engine.freeze_read_context(ReadContextV1(eligibility=SearchFilter()))
        frozen_res = engine.search_frozen(
            "observatory telemetry",
            frozen,
            use_graph_arm=True,
            explain=True,
            graph_seed="fused_top_k",
            graph_seed_k=5,
            graph_edge_kinds=["supports"],
            graph_edge_predicates=[PropertyPredicate("tier", "eq", "gold")],
        )
        assert frozen_res.explanation is not None
        f_ids = [h.id.value for h in frozen_res.results]
        assert "n:bodyless_neighbor" in f_ids
        f_idx = f_ids.index("n:bodyless_neighbor")
        assert frozen_res.explanation.per_hit[f_idx].graph_seed_source == "fused_top_k"
        assert frozen_res.explanation.per_hit[f_idx].graph_hop_depth == 1

        # 5. EvidenceSearchRequestV1 + resolve_evidence exposes EvidenceGraphOriginV1.seed_source
        ev_res = engine.search_with_evidence(
            EvidenceSearchRequestV1(
                query="observatory telemetry",
                context=frozen,
                use_graph_arm=True,
                include_explanation=True,
                graph_seed="fused_top_k",
                graph_seed_k=5,
                graph_edge_kinds=["supports"],
                graph_edge_predicates=[PropertyPredicate("tier", "eq", "gold")],
            )
        )
        ev_ids = [h.id.value for h in ev_res.search_result.results]
        assert "n:bodyless_neighbor" in ev_ids
        ev_idx = ev_ids.index("n:bodyless_neighbor")
        resolved = engine.resolve_evidence(
            EvidenceResolveRequestV1(
                evidence_ref=ev_res.evidence[ev_idx].evidence_ref,
                context=frozen,
            )
        )
        assert resolved.projection_origin.representative_arm == "graph_arm"
        assert resolved.projection_origin.graph_origin is not None
        assert resolved.projection_origin.graph_origin.kind == "traversal"
        assert resolved.projection_origin.graph_origin.hop_count == 1
        assert resolved.projection_origin.graph_origin.seed_source == "fused_top_k"

        # EdgeFts evidence seed_source
        ev_fts = engine.search_with_evidence(
            EvidenceSearchRequestV1(
                query="observatory telemetry",
                context=frozen,
                use_graph_arm=True,
                graph_seed="graph_fts",
                graph_edge_kinds=["mentions"],
            )
        )
        fts_ids = [h.id.value for h in ev_fts.search_result.results]
        assert "n:edge_fts_target" in fts_ids
        fts_idx = fts_ids.index("n:edge_fts_target")
        resolved_fts = engine.resolve_evidence(
            EvidenceResolveRequestV1(
                evidence_ref=ev_fts.evidence[fts_idx].evidence_ref,
                context=frozen,
            )
        )
        assert resolved_fts.projection_origin.graph_origin is not None
        assert resolved_fts.projection_origin.graph_origin.seed_source == "edge_fts"

        # 6. Validation errors across search, search_frozen, and search_with_evidence
        with pytest.raises(InvalidArgumentError):
            engine.search("observatory", graph_seed=cast(Any, "invalid_mode"))
        for bad_k in (0, -1, 101):
            with pytest.raises(InvalidArgumentError):
                engine.search("observatory", graph_seed_k=bad_k)
            with pytest.raises(InvalidArgumentError):
                engine.search_frozen("observatory", frozen, graph_seed_k=bad_k)
            with pytest.raises(InvalidArgumentError):
                engine.search_with_evidence(
                    EvidenceSearchRequestV1(query="observatory", context=frozen, graph_seed_k=bad_k)
                )
        with pytest.raises(TypeError):
            engine.search("observatory", graph_seed_k=cast(Any, True))
        with pytest.raises(InvalidFilterError):
            engine.search("observatory", graph_edge_kinds=[""])
        with pytest.raises(InvalidFilterError):
            engine.search("observatory", graph_edge_kinds=["bad\x1ekind"])
        with pytest.raises(WriteValidationError):
            engine.search("observatory", graph_edge_kinds=["bad\x00kind"])
        with pytest.raises(InvalidFilterError):
            engine.search(
                "observatory",
                graph_edge_predicates=[PropertyPredicate("flag", "gt", True)],
            )
        with pytest.raises(InvalidFilterError):
            engine.search(
                "observatory",
                graph_edge_predicates=[PropertyPredicate(f"k{i}", "eq", i) for i in range(33)],
            )
    finally:
        engine.close()


def test_graph_expand_frozen_evidence_preserves_predicates_and_properties(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        src_body = "evidence graph predicate anchor"
        engine.write(
            [
                {
                    "kind": "entity",
                    "body": src_body,
                    "source_id": "src:ev",
                    "logical_id": "n:src",
                    "provenance": _canonical_prov("rev:src", "v1"),
                },
                {
                    "kind": "entity",
                    "body": "target with props",
                    "source_id": "src:ev",
                    "logical_id": "n:dst",
                    "properties": {"rank": 42, "big": BEYOND_F64_HI},
                    "provenance": _derived_prov("rev:dst", "rev:src", src_body),
                },
                {
                    "edge": {
                        "kind": "rel",
                        "from": "n:src",
                        "to": "n:dst",
                        "source_id": "src:ev",
                        "logical_id": "e:rel",
                        "properties": {"confidence": 0.95},
                        "provenance": _derived_prov("rev:erel", "rev:src", src_body),
                    }
                },
            ]
        )
        frozen = engine.freeze_read_context(ReadContextV1(eligibility=SearchFilter()))
        res = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=GraphExplicitSeedV1(
                    schema_version=1,
                    type="explicit",
                    logical_ids=(IdSpace(space="logical", value="n:src"),),
                ),
                direction="outgoing",
                edge_kinds=("rel",),
                target_kinds=(),
                context=FrozenGraphReadContextV1(schema_version=1, type="frozen", context=frozen),
                max_depth=1,
                result_limit=10,
                max_work_units="100",
                include_explanation=False,
                include_evidence=True,
                edge_predicates=(PropertyPredicate("confidence", "gt", 0.5),),
                target_predicates=(PropertyPredicate("rank", "eq", 42),),
            ),
        )
        assert len(res.targets) == 1
        assert res.targets[0].properties == {"rank": 42, "big": BEYOND_F64_HI}
        assert res.targets[0].origin.terminal_edge_properties == {"confidence": 0.95}
        assert res.evidence is not None and len(res.evidence.entries) == 1
    finally:
        engine.close()
