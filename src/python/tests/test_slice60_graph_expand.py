"""Slice 60 RED oracle for the public Python graph-expansion contract."""

from __future__ import annotations

from dataclasses import FrozenInstanceError, fields
import json
from pathlib import Path
import re
from types import SimpleNamespace
from typing import Any

import fathomdb
import pytest
from fathomdb.engine import _map_native_graph_expand_result


def _current(*, valid_as_of: int = 1_700_000_000) -> fathomdb.CurrentGraphReadContextV1:
    return fathomdb.CurrentGraphReadContextV1(
        schema_version=1,
        type="current",
        context=fathomdb.ReadContextV1(
            view=fathomdb.ReadView(valid_as_of=valid_as_of),
            eligibility=fathomdb.SearchFilter(),
            schema_version=1,
        ),
    )


def _explicit(
    *ids: str,
    direction: fathomdb.TraversalDirection = "outgoing",
    depth: int = 3,
    result_limit: int = 50,
    work: str = "10000",
    explain: bool = False,
) -> fathomdb.GraphExpandRequestV1:
    return fathomdb.GraphExpandRequestV1(
        schema_version=1,
        seed=fathomdb.GraphExplicitSeedV1(
            schema_version=1,
            type="explicit",
            logical_ids=tuple(fathomdb.IdSpace(space="logical", value=value) for value in ids),
        ),
        direction=direction,
        edge_kinds=(),
        target_kinds=(),
        context=_current(),
        max_depth=depth,
        result_limit=result_limit,
        max_work_units=work,
        include_explanation=explain,
    )


def _node(logical_id: str, kind: str = "fact", body: str | None = None) -> dict[str, Any]:
    return {
        "kind": kind,
        "body": body or logical_id,
        "source_id": "test:slice60-python",
        "logical_id": logical_id,
    }


def _edge(
    logical_id: str,
    source: str,
    target: str,
    kind: str = "link",
    **extra: Any,
) -> dict[str, Any]:
    return {
        "edge": {
            "kind": kind,
            "from": source,
            "to": target,
            "source_id": "test:slice60-python",
            "logical_id": logical_id,
            **extra,
        }
    }


def _native_response() -> SimpleNamespace:
    fixture = json.loads(
        (
            Path(__file__).resolve().parents[3]
            / "dev"
            / "fixtures"
            / "slice60-graph-expand-conformance-v1.json"
        ).read_text()
    )["response"]

    def native(value: Any) -> Any:
        if isinstance(value, list):
            return [native(item) for item in value]
        if isinstance(value, dict):
            return SimpleNamespace(
                **{
                    re.sub(r"(?<!^)(?=[A-Z])", "_", key).lower(): native(item)
                    for key, item in value.items()
                }
            )
        return value

    converted = native(fixture)
    assert isinstance(converted, SimpleNamespace)
    return converted


def test_graph_carriers_are_frozen_exact_and_public() -> None:
    seed = fathomdb.GraphQuerySeedV1(schema_version=1, type="query", text="needle", ranked_limit=3)
    assert [field.name for field in fields(seed)] == [
        "schema_version",
        "type",
        "text",
        "ranked_limit",
    ]
    assert [field.name for field in fields(_current())] == [
        "schema_version",
        "type",
        "context",
    ]
    with pytest.raises(FrozenInstanceError):
        seed.text = "changed"  # type: ignore[misc]
    assert fathomdb.GraphExpansionError.code == "FDB_GRAPH_EXPANSION"


def test_graph_expand_is_a_governed_consumer_surface() -> None:
    allowlist = json.loads(
        (
            Path(__file__).resolve().parents[2] / "conformance" / "governed-surface-allowlist.json"
        ).read_text()
    )["allowlist"]
    assert callable(fathomdb.graph.expand)
    assert "graph.expand" in allowlist


def test_expand_depth_zero_preserves_seed_order_and_canonical_u64(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write([_node("b", "seed"), _node("a", "seed"), _edge("ab", "a", "b")])
    result = fathomdb.graph.expand(engine, _explicit("b", "a", depth=0, work="1"))
    assert [(seed.logical_id, seed.seed_ordinal) for seed in result.seeds] == [("b", 0), ("a", 1)]
    assert result.targets == ()
    assert result.work_units == "0"
    assert result.complete is True
    assert result.explanation is None
    engine.close()


def test_expand_honors_direction_edge_and_return_only_target_kinds(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write(
        [
            _node("a", "x"),
            _node("b", "y"),
            _node("c", "x"),
            _node("d", "x"),
            _edge("ab", "a", "b", "supports"),
            _edge("bc", "b", "c", "supports"),
            _edge("da", "d", "a", "refutes"),
        ]
    )
    request = _explicit("a", depth=2)
    request = fathomdb.GraphExpandRequestV1(
        **{
            **request.__dict__,
            "edge_kinds": ("supports",),
            "target_kinds": ("x",),
        }
    )
    assert [target.logical_id for target in fathomdb.graph.expand(engine, request).targets] == ["c"]

    incoming = _explicit("a", direction="incoming", depth=1)
    result = fathomdb.graph.expand(engine, incoming)
    assert [target.logical_id for target in result.targets] == ["d"]
    assert result.targets[0].origin.terminal_direction == "incoming"
    engine.close()


@pytest.mark.parametrize("direction", ["incoming", "outgoing", "both"])
def test_exact_w_succeeds_and_w_plus_one_refuses_without_partial_result(
    tmp_path: Any, direction: fathomdb.TraversalDirection
) -> None:
    def run(count: int) -> fathomdb.GraphExpandResultV1:
        engine = fathomdb.Engine.open(
            str(tmp_path / f"{direction}-{count}.sqlite"), use_default_embedder=False
        )
        writes = [_node("root", "seed")]
        for index in range(count):
            child = f"n-{index}"
            writes.append(_node(child))
            if direction == "incoming":
                writes.append(_edge(f"e-{index}", child, "root"))
            else:
                writes.append(_edge(f"e-{index}", "root", child))
        engine.write(writes)
        try:
            return fathomdb.graph.expand(
                engine, _explicit("root", direction=direction, depth=1, work="3")
            )
        finally:
            engine.close()

    exact = run(3)
    assert exact.work_units == "3"
    assert len(exact.targets) == 3
    with pytest.raises(fathomdb.GraphExpansionError) as raised:
        run(4)
    assert raised.value.code == "FDB_GRAPH_EXPANSION"
    assert raised.value.reason == "graph_expansion_bound_exceeded"
    assert raised.value.field_path == "/maxWorkUnits"


def test_frozen_authentication_precedes_bad_seed_and_bound(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    tampered = fathomdb.FrozenReadContextV1(
        effective_valid_at=frozen.effective_valid_at,
        context=frozen.context,
        token=f"{frozen.token}0",
        schema_version=1,
    )
    request = _explicit("duplicate", "duplicate", depth=4, work="0")
    request = fathomdb.GraphExpandRequestV1(
        **{
            **request.__dict__,
            "context": fathomdb.FrozenGraphReadContextV1(
                schema_version=1, type="frozen", context=tampered
            ),
        }
    )
    with pytest.raises(fathomdb.FrozenReadError):
        fathomdb.graph.expand(engine, request)
    engine.close()


@pytest.mark.parametrize(
    ("mutate", "path"),
    [
        (
            lambda value: setattr(value.targets[0].origin, "seed_ordinal", 1),
            "/targets/0/origin/seedOrdinal",
        ),
        (
            lambda value: setattr(value.targets[0].origin, "seed_logical_id", "other"),
            "/targets/0/origin/seedLogicalId",
        ),
        (
            lambda value: setattr(value.targets[0].origin, "target_logical_id", "other"),
            "/targets/0/origin/targetLogicalId",
        ),
        (lambda value: setattr(value.explanation, "per_target", []), "/explanation/perTarget"),
        (
            lambda value: setattr(value.explanation.per_target[0], "target_index", 1),
            "/explanation/perTarget/0/targetIndex",
        ),
        (
            lambda value: setattr(
                value.explanation.per_target[0].origin, "predecessor_logical_id", "other"
            ),
            "/explanation/perTarget/0/origin",
        ),
        (
            lambda value: setattr(value.explanation, "degradation_codes", ["projection_degraded"]),
            "/explanation/degradationCodes",
        ),
    ],
)
def test_malformed_native_response_coherence_is_typed(mutate: Any, path: str) -> None:
    value = _native_response()
    mutate(value)
    with pytest.raises(fathomdb.GraphExpansionError) as raised:
        _map_native_graph_expand_result(value)
    assert raised.value.code == "FDB_GRAPH_EXPANSION"
    assert raised.value.reason == "graph_corrupt"
    assert raised.value.field_path == path
    assert str(raised.value) == f"graph_corrupt at {path}"


@pytest.mark.parametrize(
    ("value", "path"),
    [
        ("01", "/workUnits"),
        ("18446744073709551616", "/workUnits"),
        (1, "/workUnits"),
        (False, "/complete"),
    ],
)
def test_malformed_native_u64_and_completeness_fail_closed(value: Any, path: str) -> None:
    native = _native_response()
    if path == "/complete":
        native.complete = value
    else:
        native.work_units = value
    with pytest.raises(fathomdb.GraphExpansionError) as raised:
        _map_native_graph_expand_result(native)
    assert raised.value.reason == "graph_corrupt"
    assert raised.value.field_path == path


def test_additive_native_response_members_are_ignored() -> None:
    native = _native_response()
    native.future_field = {"ignored": True}
    native.targets[0].future_target_field = True
    mapped = _map_native_graph_expand_result(native)
    assert mapped.targets[0].logical_id == "target-c"


def test_legacy_graph_methods_remain_callable_and_no_constraint_is_ignored(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    engine.write([_node("root", "seed"), _node("target"), _edge("e", "root", "target")])
    assert fathomdb.graph.neighbors(engine, "root", 1, "outgoing")[0].logical_id == "target"
    assert hasattr(fathomdb.graph.search_expand(engine, "root", 0), "search_hits")
    with pytest.raises(fathomdb.GraphExpansionError):
        fathomdb.graph.expand(engine, _explicit("root", depth=4))
    engine.close()
