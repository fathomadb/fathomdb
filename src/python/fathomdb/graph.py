"""Slice 20 (G5/G6) — graph traversal namespace.

Exposes bounded BFS and hybrid search-plus-expansion:

* ``graph.neighbors`` — G5 bounded BFS from a node, depth 1–3, direction-
  aware, cycle-guarded, hard-capped at 50 results. Edges with ``t_invalid``
  in the past are not traversed (valid-time filter).

* ``graph.search_expand`` — G6 composite: FTS/vector search (G1) + bounded
  BFS expansion from each hit. Nodes appearing in both the search hit set and
  the traversal reach appear only once in ``search_hits`` (deduplication:
  search score takes priority).

The native binding (``fathomdb._fathomdb``) performs all reads via the
ReaderWorkerPool with DEFERRED-tx snapshot isolation. Direction strings are
case-sensitive: ``"outgoing"``, ``"incoming"``, ``"both"``.
"""

from __future__ import annotations

import json
import math
import re
from types import SimpleNamespace
from typing import TYPE_CHECKING, Any, NoReturn, cast

from fathomdb._fathomdb import NodeRecord as _NativeNodeRecord
from fathomdb._fathomdb import SearchHit as _NativeSearchHit
from fathomdb._fathomdb import graph_neighbors as _native_graph_neighbors
from fathomdb._fathomdb import search_expand as _native_search_expand
from fathomdb._sdk_search import _map_native_degradations
from fathomdb.read import _to_native_view
from fathomdb.types import (
    CurrentGraphReadContextV1,
    ExpandedNode,
    FrozenGraphReadContextV1,
    GraphExpandRequestV1,
    GraphExpandResultV1,
    GraphExplicitSeedV1,
    GraphQuerySeedV1,
    IdSpace,
    NodeRecord,
    ReadContextV1,
    ReadView,
    SearchExpandResult,
    SearchHit,
    SoftFallbackBranch,
    TraversalDirection,
)

if TYPE_CHECKING:
    from fathomdb.engine import Engine


def _to_node_record(native: _NativeNodeRecord) -> NodeRecord:
    return NodeRecord(
        logical_id=native.logical_id,
        kind=native.kind,
        body=native.body,
        write_cursor=native.write_cursor,
        properties=getattr(native, "properties", None),
    )


def _to_search_hit(native: _NativeSearchHit) -> SearchHit:
    return SearchHit(
        id=IdSpace(space=native.id.space, value=native.id.value),
        kind=native.kind,
        body=native.body,
        score=native.score,
        branch=cast(SoftFallbackBranch, native.branch),
        source_id=native.source_id,
        ce_score=native.ce_score,
        properties=getattr(native, "properties", None),
    )


def neighbors(
    engine: "Engine",
    logical_id: str,
    depth: int,
    direction: TraversalDirection = "both",
    *,
    view: ReadView | None = None,
) -> list[NodeRecord]:
    """G5 — bounded BFS from ``logical_id`` over ``canonical_edges``.

    Args:
        engine:     An open FathomDB engine.
        logical_id: The root node's stable identity string.
        depth:      Hop limit — **must be 1, 2, or 3**. Depth > 3 raises
                    ``InvalidArgumentError``.
        direction:  Edge direction to follow: ``"outgoing"`` (from→to),
                    ``"incoming"`` (to→from), or ``"both"``.

    Returns:
        List of reachable ``NodeRecord``s (root excluded), hard-capped at 50.
        Edges with ``t_invalid`` in the past are silently skipped (valid-time
        filter). Returns an empty list when the root has no reachable neighbors
        within the given depth.

    Raises:
        ``InvalidArgumentError``: depth > 3 or an unrecognised direction string.
        ``EngineError`` subclasses: storage or engine-level failures.
    """
    if not logical_id:
        raise ValueError("graph.neighbors requires a non-empty logical_id")
    from fathomdb.errors import InvalidArgumentError

    if not isinstance(depth, int) or isinstance(depth, bool) or depth < 0:
        raise InvalidArgumentError(
            f"graph.neighbors depth must be a non-negative integer; got {depth!r}"
        )
    native_nodes = _native_graph_neighbors(
        engine._native, logical_id, depth, direction, _to_native_view(view)
    )
    return [_to_node_record(n) for n in native_nodes]


def search_expand(
    engine: "Engine",
    query: str,
    depth: int,
    *,
    source_type: str | None = None,
    kind: str | None = None,
    created_after: int | None = None,
    status: str | None = None,
    search_limit: int = 10,
) -> SearchExpandResult:
    """G6 — FTS/vector search followed by bounded BFS expansion.

    Runs ``engine.search(query, ...)`` (G1), then expands each hit via
    ``graph.neighbors(hit_logical_id, depth, both)``.  Nodes that appear in
    both the search hit set and the traversal reach appear **only** in
    ``search_hits`` (deduplication: search score takes priority).

    Args:
        engine:       An open FathomDB engine.
        query:        Free-text or embedding query string (same as ``engine.search``).
        depth:        BFS hop limit for expansion — **must be 0–3**. Depth 0
                      skips expansion (returns search hits only). Depth > 3
                      raises ``InvalidArgumentError``.
        source_type:  Optional metadata filter passed to the search step.
        kind:         Optional kind filter passed to the search step.
        created_after: Optional lower-bound (unix seconds) for the ``created_at``
                       column, passed to the search step.
        status:       Optional status string filter passed to the search step.

    Returns:
        ``SearchExpandResult`` with three fields:
        - ``search_hits``: original RRF-scored results from the search step.
        - ``expanded``: nodes reachable from any hit within ``depth`` hops
          that are NOT already in ``search_hits``.
        - ``all_logical_ids``: deduplicated union of both sets.

    Raises:
        ``InvalidArgumentError``: depth > 3.
        ``EngineError`` subclasses: storage or engine-level failures.
    """
    if not query:
        raise ValueError("graph.search_expand requires a non-empty query")
    from fathomdb.errors import InvalidArgumentError

    if not isinstance(depth, int) or isinstance(depth, bool) or depth < 0:
        raise InvalidArgumentError(
            f"graph.search_expand depth must be a non-negative integer; got {depth!r}"
        )
    if not isinstance(search_limit, int) or isinstance(search_limit, bool):
        raise TypeError(
            "graph.search_expand search_limit must be an integer in 1..=100, "
            f"got {type(search_limit).__name__!r}"
        )
    if not 1 <= search_limit <= 100:
        raise InvalidArgumentError(
            f"graph.search_expand search_limit must be an integer in 1..=100, got {search_limit!r}"
        )
    native_result = _native_search_expand(
        engine._native,
        query,
        depth,
        source_type,
        kind,
        created_after,
        status,
        search_limit,
    )
    return _to_search_expand_result(native_result)


def _to_search_expand_result(native_result: Any) -> SearchExpandResult:
    search_hits = [_to_search_hit(hit) for hit in native_result.search_hits]
    expanded = [
        ExpandedNode(
            node=_to_node_record(e.node),
            hop_count=e.hop_count,
        )
        for e in native_result.expanded
    ]
    return SearchExpandResult(
        search_hits=search_hits,
        expanded=expanded,
        all_logical_ids=list(native_result.all_logical_ids),
        degradations=_map_native_degradations(native_result),
    )


def _predicate_wire(pred: Any) -> dict[str, object]:
    if isinstance(pred, dict):
        key = pred["key"]
        op = pred["op"]
        val = pred["value"]
    else:
        key = pred.key
        op = pred.op
        val = pred.value
    return {
        "key": key,
        "op": op,
        "value": 0.0 if (isinstance(val, float) and val == 0.0) else val,
    }


def _read_context_wire(context: ReadContextV1) -> dict[str, object]:
    eligibility: dict[str, object] = {
        "sourceType": context.eligibility.source_type,
        "kind": context.eligibility.kind,
        "createdAfter": context.eligibility.created_after,
        "status": context.eligibility.status,
        "attributes": [list(pair) for pair in context.eligibility.attributes],
    }
    if context.eligibility.property_predicates:
        eligibility["propertyPredicates"] = [
            _predicate_wire(pred) for pred in context.eligibility.property_predicates
        ]
    return {
        "schemaVersion": context.schema_version,
        "view": {
            "includeSuperseded": context.view.include_superseded,
            "includeInactive": context.view.include_inactive,
            "includeOutOfWindow": context.view.include_out_of_window,
            "validAsOf": context.view.valid_as_of,
        },
        "eligibility": eligibility,
    }


def _request_wire(request: GraphExpandRequestV1) -> dict[str, object]:
    if not isinstance(request, GraphExpandRequestV1):
        raise TypeError("graph.expand request must be GraphExpandRequestV1")
    _validate_graph_expand_request(request)
    if isinstance(request.seed, GraphQuerySeedV1):
        seed: dict[str, object] = {
            "schemaVersion": request.seed.schema_version,
            "type": request.seed.type,
            "text": request.seed.text,
            "rankedLimit": request.seed.ranked_limit,
        }
    elif isinstance(request.seed, GraphExplicitSeedV1):
        seed = {
            "schemaVersion": request.seed.schema_version,
            "type": request.seed.type,
            "logicalIds": [
                {"space": logical_id.space, "value": logical_id.value}
                for logical_id in request.seed.logical_ids
            ],
        }
    else:
        raise TypeError("graph.expand seed must be a GraphSeedV1 carrier")

    if isinstance(request.context, CurrentGraphReadContextV1):
        context: dict[str, object] = {
            "schemaVersion": request.context.schema_version,
            "type": request.context.type,
            "context": _read_context_wire(request.context.context),
        }
    elif isinstance(request.context, FrozenGraphReadContextV1):
        frozen = request.context.context
        context = {
            "schemaVersion": request.context.schema_version,
            "type": request.context.type,
            "context": {
                "schemaVersion": frozen.schema_version,
                "effectiveValidAt": frozen.effective_valid_at,
                "context": _read_context_wire(frozen.context),
                "token": frozen.token,
            },
        }
    else:
        raise TypeError("graph.expand context must be a GraphReadContextV1 carrier")

    wire: dict[str, object] = {
        "schemaVersion": request.schema_version,
        "seed": seed,
        "direction": request.direction,
        "edgeKinds": list(request.edge_kinds),
        "targetKinds": list(request.target_kinds),
        "context": context,
        "maxDepth": request.max_depth,
        "resultLimit": request.result_limit,
        "maxWorkUnits": request.max_work_units,
        "includeExplanation": request.include_explanation,
    }
    if request.edge_predicates:
        wire["edgePredicates"] = [_predicate_wire(pred) for pred in request.edge_predicates]
    if request.target_predicates:
        wire["targetPredicates"] = [_predicate_wire(pred) for pred in request.target_predicates]
    if request.include_evidence:
        wire["includeEvidence"] = True
    if request.as_of_write_cursor is not None:
        wire["asOfWriteCursor"] = request.as_of_write_cursor
    if request.as_of_epoch_ms is not None:
        wire["asOfEpochMs"] = request.as_of_epoch_ms
    return wire


def _validate_graph_expand_request(request: GraphExpandRequestV1) -> None:
    """Reject non-transportable recursive graph request strings locally."""

    from fathomdb.errors import GraphExpansionError

    def refuse(reason: str, path: str) -> NoReturn:
        raise GraphExpansionError(f"{reason} at {path}", reason=reason, field_path=path)

    def string(value: object, reason: str, path: str) -> None:
        if not isinstance(value, str):
            refuse(reason, path)
        if "\x00" in value:  # embedded NUL
            refuse(reason, path)
        if any(0xD800 <= ord(char) <= 0xDFFF for char in value):  # lone surrogate
            refuse(reason, path)

    for index, value in enumerate(request.edge_kinds):
        string(value, "graph_edge_kinds_invalid", f"/edgeKinds/{index}")
    for index, value in enumerate(request.target_kinds):
        string(value, "graph_target_kinds_invalid", f"/targetKinds/{index}")
    if isinstance(request.seed, GraphQuerySeedV1):
        string(request.seed.text, "graph_seed_invalid", "/seed/text")
    elif isinstance(request.seed, GraphExplicitSeedV1):
        for index, logical_id in enumerate(request.seed.logical_ids):
            string(logical_id.space, "graph_seed_invalid", f"/seed/logicalIds/{index}/space")
            string(logical_id.value, "graph_seed_invalid", f"/seed/logicalIds/{index}/value")
    if not isinstance(request.context, (CurrentGraphReadContextV1, FrozenGraphReadContextV1)):
        raise TypeError("graph.expand context must be a GraphReadContextV1 carrier")
    if isinstance(request.context, FrozenGraphReadContextV1):
        string(request.context.context.token, "graph_context_invalid", "/context/context/token")
        context = request.context.context.context
        context_path = "/context/context/context"
    else:
        context = request.context.context
        context_path = "/context/context"
    eligibility = context.eligibility
    for name, value in (
        ("sourceType", eligibility.source_type),
        ("kind", eligibility.kind),
        ("status", eligibility.status),
    ):
        if value is not None:
            string(value, "graph_context_invalid", f"{context_path}/eligibility/{name}")
    if eligibility.created_after is not None and (type(eligibility.created_after) is not int):
        refuse("graph_context_invalid", f"{context_path}/eligibility/createdAfter")
    for index, pair in enumerate(eligibility.attributes):
        if not isinstance(pair, tuple) or len(pair) != 2:
            refuse("graph_context_invalid", f"{context_path}/eligibility/attributes/{index}")
        string(pair[0], "graph_context_invalid", f"{context_path}/eligibility/attributes/{index}/0")
        string(pair[1], "graph_context_invalid", f"{context_path}/eligibility/attributes/{index}/1")
    valid_ops = {"eq", "ne", "lt", "lte", "gt", "gte"}
    order_ops = {"lt", "lte", "gt", "gte"}
    i64_min = -(1 << 63)
    i64_max = (1 << 63) - 1

    def escape_pointer(token: str) -> str:
        return token.replace("~", "~0").replace("/", "~1")

    def validate_predicates(
        preds: object, reason: str, base: str, max_count: int
    ) -> None:
        if not isinstance(preds, (list, tuple)) or len(preds) > max_count:
            refuse(reason, base)
        for index, pred in enumerate(preds):
            pred_base = f"{base}/{index}"
            if isinstance(pred, dict):
                allowed = {"key", "op", "value"}
                extras = sorted(k for k in pred if not isinstance(k, str) or k not in allowed)
                if extras:
                    extra_key = extras[0]
                    refuse("unknown_field", f"{pred_base}/{escape_pointer(str(extra_key))}")
                for required_field in ("key", "op", "value"):
                    if required_field not in pred:
                        refuse(reason, f"{pred_base}/{required_field}")
                key = pred["key"]
                op = pred["op"]
                val = pred["value"]
            elif hasattr(pred, "key") and hasattr(pred, "op") and hasattr(pred, "value"):
                key = pred.key
                op = pred.op
                val = pred.value
            else:
                refuse(reason, pred_base)
            string(key, reason, f"{pred_base}/key")
            if len(key) == 0 or "\x1e" in key:
                refuse(reason, f"{pred_base}/key")
            if not isinstance(op, str) or op not in valid_ops:
                refuse(reason, f"{pred_base}/op")
            if isinstance(val, bool):
                if op in order_ops:
                    refuse(reason, f"{pred_base}/op")
            elif isinstance(val, int):
                if val < i64_min or val > i64_max:
                    refuse(reason, f"{pred_base}/value")
            elif isinstance(val, float):
                if not math.isfinite(val):
                    refuse(reason, f"{pred_base}/value")
            elif isinstance(val, str):
                string(val, reason, f"{pred_base}/value")
                if "\x1e" in val:
                    refuse(reason, f"{pred_base}/value")
            else:
                refuse(reason, f"{pred_base}/value")

    validate_predicates(
        eligibility.property_predicates,
        "graph_context_invalid",
        f"{context_path}/eligibility/propertyPredicates",
        64,
    )
    validate_predicates(
        request.edge_predicates,
        "graph_edge_predicates_invalid",
        "/edgePredicates",
        32,
    )
    validate_predicates(
        request.target_predicates,
        "graph_target_predicates_invalid",
        "/targetPredicates",
        32,
    )
    string(request.max_work_units, "graph_work_limit_invalid", "/maxWorkUnits")
    if type(request.include_evidence) is not bool:
        refuse("graph_context_invalid", "/includeEvidence")
    u64_max = (1 << 64) - 1
    if request.as_of_write_cursor is not None:
        if (
            type(request.as_of_write_cursor) is not str
            or len(request.as_of_write_cursor) > 20
            or re.fullmatch(r"0|[1-9][0-9]*", request.as_of_write_cursor) is None
            or int(request.as_of_write_cursor) > u64_max
        ):
            refuse("graph_context_invalid", "/asOfWriteCursor")
    if request.as_of_epoch_ms is not None:
        if (
            type(request.as_of_epoch_ms) is not int
            or request.as_of_epoch_ms < i64_min
            or request.as_of_epoch_ms > i64_max
        ):
            refuse("graph_context_invalid", "/asOfEpochMs")
    if request.as_of_write_cursor is not None and request.as_of_epoch_ms is not None:
        refuse("graph_context_invalid", "/asOfEpochMs")


def expand(engine: "Engine", request: GraphExpandRequestV1) -> GraphExpandResultV1:
    """Run one bounded, deterministic, all-or-nothing graph expansion."""

    request_json = json.dumps(_request_wire(request), separators=(",", ":"), ensure_ascii=False)
    response_json = engine._native.graph_expand(request_json)

    def to_namespace(value: object) -> object:
        if isinstance(value, list):
            return [to_namespace(item) for item in value]
        if isinstance(value, dict):
            return SimpleNamespace(
                **{
                    re.sub(r"(?<!^)(?=[A-Z])", "_", key).lower(): (
                        item
                        if key in ("properties", "terminalEdgeProperties")
                        else to_namespace(item)
                    )
                    for key, item in value.items()
                }
            )
        return value

    response = to_namespace(json.loads(response_json))
    from fathomdb._sdk_graph import _map_native_graph_expand_result

    return _map_native_graph_expand_result(response)


__all__ = ["TraversalDirection", "expand", "neighbors", "search_expand"]
