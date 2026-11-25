"""0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 5 Python SDK parity & Hypothesis tests.

Verifies:
- Inline typed properties (`str`, exact 64-bit `int`, finite `float` with
  `-0.0 -> 0.0`, `bool` distinct from `int`), empty `{}` -> `None`, and atomic
  `WriteValidationError` rejections.
- Hypothesis property-based round-trip over canonical `PropertyMap` scalars.
- O(1) read hydration on `NodeRecord`, `SearchHit`, `GraphTargetV1`,
  `GraphOriginV1.terminal_edge_properties`, `ResolvedEvidenceV1`, and
  `ResolvedGraphEvidenceV1`.
- `SearchFilter.property_predicates`, `ProjectionSpec.entity_class`, and
  explicit opt-in `ProjectionSpec.vector_kinds` (preserving `vector_unsupported_kinds`).
- `read.operational_collections` / `read.operational_collection` and `AdminSchema`
  kind-conflict refusal.
- `EraseReport.completed_pending_erasure`.
"""

from __future__ import annotations

import hashlib
import math
from typing import Any

import pytest
from hypothesis import given, settings, strategies as st

from fathomdb import (
    CurrentGraphReadContextV1,
    Engine,
    EvidenceResolveRequestV1,
    EvidenceSearchRequestV1,
    FrozenGraphReadContextV1,
    GraphEvidenceResolveRequestV1,
    GraphExpandRequestV1,
    GraphExplicitSeedV1,
    IdSpace,
    OperationalCollectionInfo,
    ProjectionRole,
    ProjectionSpec,
    PropertyPredicate,
    ReadContextV1,
    SearchFilter,
    graph,
    read,
)
from fathomdb.errors import InvalidFilterError, WriteValidationError

I64_MIN = -(1 << 63)
I64_MAX = (1 << 63) - 1
BEYOND_F64_POS = (1 << 53) + 1
BEYOND_F64_NEG = -((1 << 53) + 1)


def _canonical_prop_text(min_size: int = 0) -> st.SearchStrategy[str]:
    return st.text(
        alphabet=st.characters(
            blacklist_characters="\x00\x1e",
            blacklist_categories=("Cs",),
        ),
        min_size=min_size,
        max_size=24,
    )


_PROP_KEYS = _canonical_prop_text(min_size=1)
_PROP_VALUES = st.one_of(
    _canonical_prop_text(min_size=0),
    st.booleans(),
    st.integers(min_value=I64_MIN, max_value=I64_MAX),
    st.floats(allow_nan=False, allow_infinity=False),
)
_PROP_MAPS = st.dictionaries(keys=_PROP_KEYS, values=_PROP_VALUES, min_size=1, max_size=8)


def _normalize_expected(props: dict[str, Any]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for k, v in props.items():
        if isinstance(v, float) and v == 0.0:
            out[k] = 0.0
        else:
            out[k] = v
    return out


def _assert_typed_props_equal(actual: dict[str, Any] | None, expected: dict[str, Any]) -> None:
    assert actual is not None
    assert set(actual.keys()) == set(expected.keys())
    for key, exp in expected.items():
        got = actual[key]
        if isinstance(exp, bool):
            assert type(got) is bool
            assert got is exp
        elif isinstance(exp, int):
            assert type(got) is int
            assert got == exp
        elif isinstance(exp, float):
            # Integral floats may round-trip via JSON as float or int; finite value matches.
            assert isinstance(got, (int, float)) and not isinstance(got, bool)
            if exp == 0.0:
                assert got == 0.0
                if isinstance(got, float):
                    assert not math.copysign(1.0, got) < 0
            else:
                assert math.isclose(float(got), exp, rel_tol=1e-12, abs_tol=0.0)
        else:
            assert type(got) is str
            assert got == exp


def test_slice38_property_round_trip_and_invalid_rejections(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        props = {
            "str_val": "hello world",
            "int_min": I64_MIN,
            "int_max": I64_MAX,
            "int_hi": BEYOND_F64_POS,
            "int_lo": BEYOND_F64_NEG,
            "float_val": 3.25,
            "neg_zero": -0.0,
            "flag_true": True,
            "flag_false": False,
            "zero_int": 0,
            "one_int": 1,
        }
        engine.write(
            [
                {
                    "kind": "note",
                    "body": "node with typed properties",
                    "source_id": "src:props",
                    "logical_id": "node:typed",
                    "properties": props,
                },
                {
                    "kind": "note",
                    "body": "node with empty properties",
                    "source_id": "src:props",
                    "logical_id": "node:empty",
                    "properties": {},
                },
            ]
        )

        rec = read.get(engine, "node:typed")
        assert rec is not None
        assert rec.properties is not None
        assert rec.properties["str_val"] == "hello world"
        assert type(rec.properties["int_min"]) is int and rec.properties["int_min"] == I64_MIN
        assert type(rec.properties["int_max"]) is int and rec.properties["int_max"] == I64_MAX
        assert type(rec.properties["int_hi"]) is int and rec.properties["int_hi"] == BEYOND_F64_POS
        assert type(rec.properties["int_lo"]) is int and rec.properties["int_lo"] == BEYOND_F64_NEG
        assert isinstance(rec.properties["float_val"], float)
        assert rec.properties["float_val"] == pytest.approx(3.25)
        nz = rec.properties["neg_zero"]
        assert not isinstance(nz, bool) and nz == 0.0
        if isinstance(nz, float):
            assert math.copysign(1.0, nz) > 0
        assert type(rec.properties["flag_true"]) is bool and rec.properties["flag_true"] is True
        assert type(rec.properties["flag_false"]) is bool and rec.properties["flag_false"] is False
        assert type(rec.properties["zero_int"]) is int and rec.properties["zero_int"] == 0
        assert type(rec.properties["one_int"]) is int and rec.properties["one_int"] == 1

        empty_rec = read.get(engine, "node:empty")
        assert empty_rec is not None
        assert empty_rec.properties is None

        invalid_maps: list[dict[str, Any]] = [
            {"": "empty-key"},
            {"bad\0key": "nul"},
            {"bad\x1ekey": "rs"},
            {"k": "bad\0val"},
            {"k": "bad\x1eval"},
            {"k": None},
            {"k": {"nested": 1}},
            {"k": [1, 2]},
            {"k": float("nan")},
            {"k": float("inf")},
            {"k": float("-inf")},
            {"k": I64_MAX + 1},
            {"k": I64_MIN - 1},
        ]
        for idx, bad_props in enumerate(invalid_maps):
            with pytest.raises(WriteValidationError):
                engine.write(
                    [
                        {
                            "kind": "note",
                            "body": f"invalid-{idx}",
                            "source_id": "src:invalid",
                            "logical_id": f"node:invalid:{idx}",
                            "properties": bad_props,
                        }
                    ]
                )
            assert read.get(engine, f"node:invalid:{idx}") is None
    finally:
        engine.close()


@settings(max_examples=25, deadline=None)
@given(props=_PROP_MAPS)
def test_slice38_property_round_trip_hypothesis(tmp_path_factory, props: dict[str, Any]) -> None:
    db = tmp_path_factory.mktemp("hyp") / "props.sqlite"
    engine = Engine.open(str(db), use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "note",
                    "body": "hypothesis round trip body",
                    "source_id": "src:hyp",
                    "logical_id": "node:hyp",
                    "properties": props,
                }
            ]
        )
        rec = read.get(engine, "node:hyp")
        assert rec is not None
        _assert_typed_props_equal(rec.properties, _normalize_expected(props))
    finally:
        engine.close()


def test_slice38_o1_hydration_across_reads_search_graph_and_evidence(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        src_body = "anchor source alpha"
        src_hash = hashlib.sha256(src_body.encode("utf-8")).hexdigest()
        seed_props = {"role": "seed", "rank": 1, "active": True}
        target_props = {"role": "target", "big_id": BEYOND_F64_POS, "weight": 2.5}
        edge_props = {"relation": "supports", "strength": 7, "verified": True}

        engine.write(
            [
                {
                    "kind": "note",
                    "body": src_body,
                    "source_id": "src:hydrate",
                    "logical_id": "node:seed",
                    "properties": seed_props,
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": "rev-seed-1",
                        "source_version_id": "sv-1",
                    },
                },
                {
                    "kind": "note",
                    "body": "target beta evidence payload",
                    "source_id": "src:hydrate",
                    "logical_id": "node:target",
                    "properties": target_props,
                    "provenance": {
                        "schema_version": 1,
                        "role": "derived",
                        "artifact_revision_id": "rev-target-1",
                        "source_version_id": "sv-1",
                        "source_revision_id": "rev-seed-1",
                        "source_locator": {"kind": "whole_body"},
                        "canonical_source_hash": {"algorithm": "sha256", "digest_hex": src_hash},
                    },
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "node:seed",
                        "to": "node:target",
                        "source_id": "src:hydrate",
                        "logical_id": "edge:seed-target",
                        "body": "supporting link",
                        "properties": edge_props,
                        "provenance": {
                            "schema_version": 1,
                            "role": "derived",
                            "artifact_revision_id": "rev-edge-1",
                            "source_version_id": "sv-1",
                            "source_revision_id": "rev-seed-1",
                            "source_locator": {"kind": "whole_body"},
                            "canonical_source_hash": {"algorithm": "sha256", "digest_hex": src_hash},
                        },
                    }
                },
            ]
        )

        # NodeRecord hydration
        got = read.get(engine, "node:target")
        assert got is not None and got.properties == target_props
        listed = read.list(engine, "note")
        by_id = {n.logical_id: n for n in listed}
        assert by_id["node:seed"].properties == seed_props
        assert by_id["node:target"].properties == target_props

        # SearchHit hydration
        res = engine.search_text_only("target beta evidence")
        assert res.results
        hit = next(h for h in res.results if h.id.value == "node:target")
        assert hit.properties == target_props

        # Frozen evidence hydration (ResolvedEvidenceV1)
        frozen = engine.freeze_read_context(ReadContextV1())
        ev_search = engine.search_with_evidence(
            EvidenceSearchRequestV1(
                query="target beta evidence",
                context=frozen,
                limit=5,
            )
        )
        target_sidecar = next(
            e for e in ev_search.evidence if e.artifact_revision_id == "rev-target-1"
        )
        resolved = engine.resolve_evidence(
            EvidenceResolveRequestV1(
                evidence_ref=target_sidecar.evidence_ref,
                context=frozen,
            )
        )
        assert resolved.properties == target_props

        # GraphTargetV1 + GraphOriginV1.terminal_edge_properties + ResolvedGraphEvidenceV1
        expanded = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=GraphExplicitSeedV1(
                    schema_version=1,
                    type="explicit",
                    logical_ids=(IdSpace(space="logical", value="node:seed"),),
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
                max_work_units="100",
                include_explanation=True,
                include_evidence=True,
            ),
        )
        assert len(expanded.targets) == 1
        target = expanded.targets[0]
        assert target.properties == target_props
        assert target.origin.terminal_edge_properties == edge_props

        assert expanded.evidence is not None and len(expanded.evidence.entries) == 1
        entry = expanded.evidence.entries[0]
        resolved_node_ev = engine.resolve_graph_evidence(
            GraphEvidenceResolveRequestV1(
                evidence_ref=entry.target_evidence_ref,
                context=frozen,
            )
        )
        assert resolved_node_ev.artifact.artifact_class == "node"
        assert resolved_node_ev.artifact.properties == target_props

        resolved_edge_ev = engine.resolve_graph_evidence(
            GraphEvidenceResolveRequestV1(
                evidence_ref=entry.terminal_edge_evidence_ref,
                context=frozen,
            )
        )
        assert resolved_edge_ev.artifact.artifact_class == "edge"
        assert resolved_edge_ev.artifact.properties == edge_props

        # Current context graph expand with property_predicates also works
        cur_expanded = graph.expand(
            engine,
            GraphExpandRequestV1(
                schema_version=1,
                seed=GraphExplicitSeedV1(
                    schema_version=1,
                    type="explicit",
                    logical_ids=(IdSpace(space="logical", value="node:seed"),),
                ),
                direction="outgoing",
                edge_kinds=("supports",),
                target_kinds=(),
                context=CurrentGraphReadContextV1(
                    schema_version=1,
                    type="current",
                    context=ReadContextV1(),
                ),
                max_depth=1,
                result_limit=10,
                max_work_units="100",
                include_explanation=False,
            ),
        )
        assert cur_expanded.targets[0].properties == target_props
        assert cur_expanded.targets[0].origin.terminal_edge_properties == edge_props
    finally:
        engine.close()


def test_slice38_property_predicates_entity_class_and_vector_kinds(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        delta = engine.configure_projections(
            [
                ProjectionSpec(
                    name="priority",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="verified",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="edge_tag",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="edge",
                ),
                ProjectionSpec(
                    name="body",
                    roles=frozenset({ProjectionRole.SEARCHABLE}),
                    vector=True,
                    vector_kinds=["custom:memory"],
                ),
            ]
        )
        assert not delta.unchanged

        # Verify read.projections round-trips entity_class and vector_kinds
        specs = {s.name: s for s in read.projections(engine)}
        assert specs["priority"].entity_class == "node"
        assert specs["edge_tag"].entity_class == "edge"
        assert specs["body"].vector_kinds == ["custom:memory"]

        # Reject reserved "edge_fact" vector kind
        with pytest.raises(WriteValidationError):
            engine.configure_projections(
                [
                    ProjectionSpec(
                        name="bad_vec",
                        roles=frozenset({ProjectionRole.SEARCHABLE}),
                        vector=True,
                        vector_kinds=["edge_fact"],
                    )
                ]
            )

        engine.write(
            [
                {
                    "kind": "custom:memory",
                    "body": "shared search needle alpha",
                    "source_id": "src:pred",
                    "logical_id": "n:1",
                    "properties": {"priority": 10, "verified": True},
                },
                {
                    "kind": "custom:memory",
                    "body": "shared search needle beta",
                    "source_id": "src:pred",
                    "logical_id": "n:2",
                    "properties": {"priority": 20, "verified": False},
                },
                {
                    "kind": "invoice",
                    "body": "shared search needle gamma",
                    "source_id": "src:pred",
                    "logical_id": "n:3",
                    "properties": {"priority": 30, "verified": True},
                },
            ]
        )

        # Unenrolled kind "invoice" stays in vector_unsupported_kinds while "custom:memory" does not
        reapply = engine.configure_projections(
            [
                ProjectionSpec(
                    name="priority",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="verified",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="edge_tag",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="edge",
                ),
                ProjectionSpec(
                    name="body",
                    roles=frozenset({ProjectionRole.SEARCHABLE}),
                    vector=True,
                    vector_kinds=["custom:memory"],
                ),
            ]
        )
        assert "invoice" in reapply.vector_unsupported_kinds
        assert "custom:memory" not in reapply.vector_unsupported_kinds

        # Eq / Ne / Lt / Lte / Gt / Gte on declared node Filterable properties
        def hit_ids(preds: list[PropertyPredicate]) -> set[str]:
            sf = SearchFilter(property_predicates=preds)
            return {h.id.value for h in engine.search_text_only("shared search needle", filter=sf).results}

        assert hit_ids([PropertyPredicate("priority", "eq", 20)]) == {"n:2"}
        assert hit_ids([PropertyPredicate("priority", "ne", 20)]) == {"n:1", "n:3"}
        assert hit_ids([PropertyPredicate("priority", "lt", 20)]) == {"n:1"}
        assert hit_ids([PropertyPredicate("priority", "lte", 20)]) == {"n:1", "n:2"}
        assert hit_ids([PropertyPredicate("priority", "gt", 20)]) == {"n:3"}
        assert hit_ids([PropertyPredicate("priority", "gte", 20)]) == {"n:2", "n:3"}
        assert hit_ids([PropertyPredicate("verified", "eq", True)]) == {"n:1", "n:3"}
        assert hit_ids([PropertyPredicate("verified", "ne", True)]) == {"n:2"}

        # Bool range rejection -> InvalidFilterError
        for op in ("lt", "lte", "gt", "gte"):
            with pytest.raises(InvalidFilterError):
                engine.search_text_only(
                    "shared search needle",
                    filter=SearchFilter(property_predicates=[PropertyPredicate("verified", op, True)]),  # type: ignore[arg-type]
                )

        # Undeclared key & edge-only key rejection -> InvalidFilterError
        with pytest.raises(InvalidFilterError):
            engine.search_text_only(
                "shared search needle",
                filter=SearchFilter(property_predicates=[PropertyPredicate("undeclared", "eq", 1)]),
            )
        with pytest.raises(InvalidFilterError):
            engine.search_text_only(
                "shared search needle",
                filter=SearchFilter(property_predicates=[PropertyPredicate("edge_tag", "eq", "x")]),
            )
    finally:
        engine.close()


def test_slice38_operational_collections_and_erase_report_flag(db_path: str) -> None:
    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        # Fresh DB seeds projection_failures
        initial = read.operational_collections(engine)
        assert [c.name for c in initial] == ["projection_failures"]
        assert isinstance(initial[0], OperationalCollectionInfo)
        assert read.operational_collection(engine, "missing") is None

        engine.write(
            [
                {
                    "admin_schema": {
                        "name": "alpha_log",
                        "kind": "append_only_log",
                        "schema_json": '{"type":"object"}',
                        "retention_json": '{"max_rows":10}',
                    }
                }
            ]
        )

        cols = read.operational_collections(engine)
        assert [c.name for c in cols] == ["alpha_log", "projection_failures"]
        alpha = read.operational_collection(engine, "alpha_log")
        assert alpha is not None
        assert alpha.name == "alpha_log"
        assert alpha.kind == "append_only_log"
        assert alpha.schema_json == '{"type":"object"}'
        assert alpha.retention_json == '{"max_rows":10}'
        assert alpha.format_version == 1

        # AdminSchema kind conflict raises WriteValidationError
        with pytest.raises(WriteValidationError):
            engine.write(
                [
                    {
                        "admin_schema": {
                            "name": "alpha_log",
                            "kind": "latest_state",
                            "schema_json": "{}",
                            "retention_json": "{}",
                        }
                    }
                ]
            )

        # EraseReport.completed_pending_erasure is False on normal erasure and no-op
        engine.write([{"kind": "note", "body": "erase check", "source_id": "src:erase"}])
        report = engine.erase_source("src:erase")
        assert report.nodes_excised == 1
        assert report.completed_pending_erasure is False
        noop = engine.erase_source("src:erase")
        assert noop.nodes_excised == 0
        assert noop.completed_pending_erasure is False
    finally:
        engine.close()


def test_slice38_graph_expand_rejects_lone_surrogate_and_nul_in_property_predicates(
    db_path: str,
) -> None:
    from fathomdb.errors import GraphExpansionError

    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.configure_projections(
            [ProjectionSpec(name="tag", roles=frozenset([ProjectionRole.FILTERABLE]))]
        )
        engine.write(
            [
                {
                    "kind": "note",
                    "body": "graph expand python ffi string seed",
                    "source_id": "src:py-gffi",
                    "logical_id": "py-gffi:1",
                    "properties": {"tag": "ok"},
                }
            ]
        )

        bad_preds = [
            PropertyPredicate("tag\ud800", "eq", "ok"),
            PropertyPredicate("tag\x00", "eq", "ok"),
            PropertyPredicate("tag", "eq", "bad\ud800val"),
            PropertyPredicate("tag", "eq", "bad\x00val"),
        ]
        for pred in bad_preds:
            with pytest.raises(GraphExpansionError) as exc_info:
                graph.expand(
                    engine,
                    GraphExpandRequestV1(
                        schema_version=1,
                        seed=GraphExplicitSeedV1(
                            schema_version=1,
                            type="explicit",
                            logical_ids=(IdSpace(space="logical", value="py-gffi:1"),),
                        ),
                        direction="outgoing",
                        edge_kinds=(),
                        target_kinds=(),
                        context=CurrentGraphReadContextV1(
                            schema_version=1,
                            type="current",
                            context=ReadContextV1(
                                eligibility=SearchFilter(property_predicates=[pred]),
                            ),
                        ),
                        max_depth=1,
                        result_limit=10,
                        max_work_units="100",
                        include_explanation=False,
                    ),
                )
            assert exc_info.value.reason == "graph_context_invalid"
    finally:
        engine.close()


def test_slice40_same_name_node_and_edge_projection_namespace_and_strict_target(
    db_path: str,
) -> None:
    """Slice 40 WP-40A1 (Gate1-D9 Option B2-strict): simultaneous (node, 'status')
    and (edge, 'status'), isolated drop, ProjectionRuntimeStatusEntry.entity_class,
    and TypeError on bare-string drop."""
    from fathomdb import ProjectionTarget

    engine = Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "note",
                    "body": '{"status":"open","text":"node status body"}',
                    "source_id": "src:s40",
                    "logical_id": "s40:n1",
                    "properties": {"status": "open"},
                },
                {
                    "kind": "note",
                    "body": '{"status":"closed","text":"node closed body"}',
                    "source_id": "src:s40",
                    "logical_id": "s40:n2",
                    "properties": {"status": "closed"},
                },
                {
                    "type": "edge",
                    "kind": "links",
                    "from": "s40:n1",
                    "to": "s40:n2",
                    "source_id": "src:s40",
                    "logical_id": "s40:e1",
                    "properties": {"status": "verified"},
                },
            ]
        )

        delta = engine.configure_projections(
            [
                ProjectionSpec(
                    name="status",
                    roles=frozenset({ProjectionRole.FILTERABLE, ProjectionRole.SEARCHABLE}),
                    fts=True,
                    entity_class="node",
                ),
                ProjectionSpec(
                    name="status",
                    roles=frozenset({ProjectionRole.FILTERABLE}),
                    entity_class="edge",
                ),
            ]
        )
        assert not delta.unchanged
        assert delta.built == [
            ProjectionTarget(name="status", entity_class="node"),
            ProjectionTarget(name="status", entity_class="edge"),
        ]

        # ProjectionRuntimeStatusEntry carries entity_class
        status = read.projection_status(engine)
        assert [(p.name, p.entity_class) for p in status.projections] == [
            ("status", "node"),
            ("status", "edge"),
        ]

        # Strictly reject bare strings in drop with TypeError
        with pytest.raises(TypeError):
            engine.configure_projections([], drop=["status"])  # type: ignore[list-item]
        with pytest.raises(TypeError):
            engine.configure_projections([], drop="status")  # type: ignore[arg-type]

        # Drop ONLY the edge projection; node projection stays intact and searchable
        dropped = engine.configure_projections(
            [], drop=[ProjectionTarget(name="status", entity_class="edge")]
        )
        assert dropped.dropped == [ProjectionTarget(name="status", entity_class="edge")]
        remaining = read.projections(engine)
        assert [(s.name, s.entity_class) for s in remaining] == [("status", "node")]

        hits = engine.search_projected_text("open", "status")
        assert [h.id.value for h in hits.results] == ["s40:n1"]
    finally:
        engine.close()
