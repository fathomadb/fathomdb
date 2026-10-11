"""Private evidence operations for the Python Engine facade."""

from __future__ import annotations

import json
import math
import re
from typing import TYPE_CHECKING, Any, NoReturn, cast

from fathomdb._sdk_open import _validate_frozen_trace_context
from fathomdb._sdk_search import (
    _map_native_search_result,
    _to_native_frozen_context,
    _validate_ranked_result_limit,
)
from fathomdb.errors import (
    DependencyTraceError,
    EvidenceError,
    InvalidArgumentError,
)
from fathomdb.types import (
    DependencyTraceEdgeV1,
    DependencyTraceNodeV1,
    DependencyTraceRequestV1,
    DependencyTraceResultV1,
    EvidenceArtifactLifecycleV1,
    EvidenceContributionV1,
    EvidenceGraphOriginV1,
    EvidenceProjectionOriginV1,
    EvidenceResolveRequestV1,
    EvidenceSearchRequestV1,
    EvidenceSearchResultV1,
    EvidenceSidecarEntryV1,
    FrozenReadContextV1,
    ResolvedEvidenceV1,
    SourceDependencyV1,
    TraceNodeLifecycleV1,
    TraceReadBoundaryV1,
)

if TYPE_CHECKING:
    from fathomdb.engine import Engine


def _evidence_response_error(reason: str, path: str) -> NoReturn:
    raise EvidenceError(f"{reason} at {path}", reason=reason, field_path=path)


def _require_evidence_schema(value: object, path: str) -> None:
    if value != 1:
        _evidence_response_error("unsupported_schema_version", path)


def _require_evidence_variant(value: object, allowed: set[str], path: str) -> None:
    if not isinstance(value, str) or value not in allowed:
        _evidence_response_error("evidence_corrupt", path)


def _require_u32(value: object, path: str, *, optional: bool = True) -> None:
    if optional and value is None:
        return
    if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 2**32 - 1:
        _evidence_response_error("evidence_corrupt", path)


def _require_finite(value: object, path: str, *, optional: bool = True) -> None:
    if optional and value is None:
        return
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        _evidence_response_error("evidence_corrupt", path)
    if not math.isfinite(cast(float, value)):
        _evidence_response_error("evidence_corrupt", path)


def _require_nonempty_string(value: object, path: str) -> None:
    if not isinstance(value, str) or not value:
        _evidence_response_error("evidence_corrupt", path)


def _trace_response_error(reason: str, path: str) -> NoReturn:
    raise DependencyTraceError(f"{reason} at {path}", reason=reason, field_path=path)


def _trace_object(value: object, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _trace_response_error("trace_corrupt", path)
    return cast(dict[str, Any], value)


def _trace_array(value: object, path: str) -> list[Any]:
    if not isinstance(value, list):
        _trace_response_error("trace_corrupt", path)
    return cast(list[Any], value)


def _trace_required(value: dict[str, Any], name: str, path: str) -> Any:
    if name not in value:
        _trace_response_error("trace_corrupt", path)
    return value[name]


def _trace_schema(value: dict[str, Any], path: str) -> None:
    schema = _trace_required(value, "schemaVersion", path)
    if not isinstance(schema, int) or isinstance(schema, bool) or schema != 1:
        _trace_response_error("unsupported_schema_version", path)


def _trace_id(value: object, path: str) -> str:
    if (
        not isinstance(value, str)
        or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._:-]{0,127}", value) is None
        or value.startswith("_fdb:")
    ):
        _trace_response_error("trace_corrupt", path)
    return value


def _trace_u32(value: object, path: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 2**32 - 1:
        _trace_response_error("trace_corrupt", path)
    return value


def _trace_i64(value: object, path: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or not -(2**63) <= value <= 2**63 - 1:
        _trace_response_error("trace_corrupt", path)
    return value


def _trace_u64(value: object, path: str) -> str:
    maximum = "18446744073709551615"
    if (
        not isinstance(value, str)
        or re.fullmatch(r"0|[1-9][0-9]*", value) is None
        or len(value) > len(maximum)
        or (len(value) == len(maximum) and value > maximum)
    ):
        _trace_response_error("trace_corrupt", path)
    return value


def _trace_bool(value: object, path: str) -> bool:
    if not isinstance(value, bool):
        _trace_response_error("trace_corrupt", path)
    return value


def _decode_dependency_trace_response(encoded: str) -> DependencyTraceResultV1:
    """Decode and recursively validate one native dependency-trace response."""
    try:
        raw = json.loads(encoded)
    except (TypeError, ValueError):
        _trace_response_error("trace_corrupt", "")
    value = _trace_object(raw, "")
    _trace_schema(value, "/schemaVersion")
    root = _trace_id(
        _trace_required(value, "rootRevisionId", "/rootRevisionId"),
        "/rootRevisionId",
    )
    direction = _trace_required(value, "direction", "/direction")
    if direction not in ("to_source", "to_dependents"):
        _trace_response_error("trace_corrupt", "/direction")

    node_values = _trace_array(_trace_required(value, "nodes", "/nodes"), "/nodes")
    if not node_values or len(node_values) > 101:
        _trace_response_error("trace_corrupt", "/nodes")
    nodes: list[DependencyTraceNodeV1] = []
    revision_ids: set[str] = set()
    for index, raw_node in enumerate(node_values):
        base = f"/nodes/{index}"
        node = _trace_object(raw_node, base)
        _trace_schema(node, f"{base}/schemaVersion")
        revision = _trace_id(
            _trace_required(node, "artifactRevisionId", f"{base}/artifactRevisionId"),
            f"{base}/artifactRevisionId",
        )
        if revision in revision_ids:
            _trace_response_error("trace_corrupt", f"{base}/artifactRevisionId")
        revision_ids.add(revision)
        artifact_class = _trace_required(node, "artifactClass", f"{base}/artifactClass")
        if artifact_class not in ("node", "edge"):
            _trace_response_error("trace_corrupt", f"{base}/artifactClass")
        role = _trace_required(node, "role", f"{base}/role")
        if role not in ("canonical_source", "derived"):
            _trace_response_error("trace_corrupt", f"{base}/role")
        depth = _trace_u32(_trace_required(node, "depth", f"{base}/depth"), f"{base}/depth")
        lifecycle_path = f"{base}/lifecycle"
        lifecycle = _trace_object(
            _trace_required(node, "lifecycle", lifecycle_path), lifecycle_path
        )
        _trace_schema(lifecycle, f"{lifecycle_path}/schemaVersion")
        if (
            _trace_required(lifecycle, "artifactClass", f"{lifecycle_path}/artifactClass")
            != artifact_class
        ):
            _trace_response_error("trace_corrupt", f"{lifecycle_path}/artifactClass")
        state = lifecycle.get("state")
        if artifact_class == "node":
            if state not in ("pending", "active", "deleted"):
                _trace_response_error("trace_corrupt", f"{lifecycle_path}/state")
        elif "state" in lifecycle:
            _trace_response_error("trace_corrupt", f"{lifecycle_path}/state")
        nodes.append(
            DependencyTraceNodeV1(
                schema_version=1,
                artifact_revision_id=revision,
                artifact_class=artifact_class,
                role=role,
                depth=depth,
                lifecycle=TraceNodeLifecycleV1(
                    schema_version=1,
                    artifact_class=artifact_class,
                    state=state,
                    superseded=_trace_bool(
                        _trace_required(lifecycle, "superseded", f"{lifecycle_path}/superseded"),
                        f"{lifecycle_path}/superseded",
                    ),
                    valid_at_effective=_trace_bool(
                        _trace_required(
                            lifecycle,
                            "validAtEffective",
                            f"{lifecycle_path}/validAtEffective",
                        ),
                        f"{lifecycle_path}/validAtEffective",
                    ),
                ),
            )
        )

    edge_values = _trace_array(
        _trace_required(value, "dependencyEdges", "/dependencyEdges"),
        "/dependencyEdges",
    )
    if len(edge_values) > 100:
        _trace_response_error("trace_corrupt", "/dependencyEdges")
    edges: list[DependencyTraceEdgeV1] = []
    dependency_ids: set[str] = set()
    for index, raw_edge in enumerate(edge_values):
        base = f"/dependencyEdges/{index}"
        edge = _trace_object(raw_edge, base)
        _trace_schema(edge, f"{base}/schemaVersion")
        dependency_id = _trace_id(
            _trace_required(edge, "dependencyId", f"{base}/dependencyId"),
            f"{base}/dependencyId",
        )
        if dependency_id in dependency_ids:
            _trace_response_error("trace_corrupt", f"{base}/dependencyId")
        dependency_ids.add(dependency_id)
        edges.append(
            DependencyTraceEdgeV1(
                schema_version=1,
                dependency_id=dependency_id,
                source_revision_id=_trace_id(
                    _trace_required(edge, "sourceRevisionId", f"{base}/sourceRevisionId"),
                    f"{base}/sourceRevisionId",
                ),
                derived_revision_id=_trace_id(
                    _trace_required(edge, "derivedRevisionId", f"{base}/derivedRevisionId"),
                    f"{base}/derivedRevisionId",
                ),
                registered_dependency_generation=_trace_u64(
                    _trace_required(
                        edge,
                        "registeredDependencyGeneration",
                        f"{base}/registeredDependencyGeneration",
                    ),
                    f"{base}/registeredDependencyGeneration",
                ),
            )
        )

    checked = _trace_u32(
        _trace_required(value, "checkedWorkUnits", "/checkedWorkUnits"),
        "/checkedWorkUnits",
    )
    if checked != len(edges) + 1 or len(nodes) != len(edges) + 1:
        _trace_response_error("trace_corrupt", "/checkedWorkUnits")
    if _trace_required(value, "complete", "/complete") is not True:
        _trace_response_error("trace_corrupt", "/complete")
    expected_root_role = "derived" if direction == "to_source" else "canonical_source"
    if nodes[0].artifact_revision_id != root or nodes[0].depth != 0:
        _trace_response_error("trace_corrupt", "/nodes/0")
    if nodes[0].role != expected_root_role:
        _trace_response_error("trace_corrupt", "/nodes/0/role")
    for index, (node, edge) in enumerate(zip(nodes[1:], edges, strict=True)):
        if node.depth != 1:
            _trace_response_error("trace_corrupt", f"/nodes/{index + 1}/depth")
        if direction == "to_dependents":
            if node.role != "derived":
                _trace_response_error("trace_corrupt", f"/nodes/{index + 1}/role")
            if edge.source_revision_id != root:
                _trace_response_error("trace_corrupt", f"/dependencyEdges/{index}/sourceRevisionId")
            if edge.derived_revision_id != node.artifact_revision_id:
                _trace_response_error(
                    "trace_corrupt", f"/dependencyEdges/{index}/derivedRevisionId"
                )
        else:
            if node.role != "canonical_source":
                _trace_response_error("trace_corrupt", f"/nodes/{index + 1}/role")
            if edge.derived_revision_id != root:
                _trace_response_error(
                    "trace_corrupt", f"/dependencyEdges/{index}/derivedRevisionId"
                )
            if edge.source_revision_id != node.artifact_revision_id:
                _trace_response_error("trace_corrupt", f"/dependencyEdges/{index}/sourceRevisionId")
    if nodes[1:] != sorted(nodes[1:], key=lambda item: item.artifact_revision_id):
        _trace_response_error("trace_corrupt", "/nodes")
    if edges != sorted(edges, key=lambda item: (item.derived_revision_id, item.dependency_id)):
        _trace_response_error("trace_corrupt", "/dependencyEdges")

    boundary_value = _trace_object(
        _trace_required(value, "readBoundary", "/readBoundary"), "/readBoundary"
    )
    _trace_schema(boundary_value, "/readBoundary/schemaVersion")
    projection_generation_id = _trace_required(
        boundary_value, "projectionGenerationId", "/readBoundary/projectionGenerationId"
    )
    if (
        not isinstance(projection_generation_id, str)
        or re.fullmatch(r"pgen1:[0-9a-f]{32}", projection_generation_id) is None
    ):
        _trace_response_error("trace_corrupt", "/readBoundary/projectionGenerationId")
    boundary = TraceReadBoundaryV1(
        schema_version=1,
        effective_at_epoch_s=_trace_i64(
            _trace_required(boundary_value, "effectiveAtEpochS", "/readBoundary/effectiveAtEpochS"),
            "/readBoundary/effectiveAtEpochS",
        ),
        observed_write_boundary=_trace_u64(
            _trace_required(
                boundary_value,
                "observedWriteBoundary",
                "/readBoundary/observedWriteBoundary",
            ),
            "/readBoundary/observedWriteBoundary",
        ),
        dependency_generation=_trace_u64(
            _trace_required(
                boundary_value,
                "dependencyGeneration",
                "/readBoundary/dependencyGeneration",
            ),
            "/readBoundary/dependencyGeneration",
        ),
        projection_generation_id=projection_generation_id,
    )
    dependency_generation = int(boundary.dependency_generation)
    for index, edge in enumerate(edges):
        registered_generation = int(edge.registered_dependency_generation)
        if registered_generation == 0 or registered_generation > dependency_generation:
            _trace_response_error(
                "trace_corrupt",
                f"/dependencyEdges/{index}/registeredDependencyGeneration",
            )
    return DependencyTraceResultV1(
        schema_version=1,
        root_revision_id=root,
        direction=cast(Any, direction),
        nodes=tuple(nodes),
        dependency_edges=tuple(edges),
        checked_work_units=checked,
        complete=True,
        read_boundary=boundary,
    )


def _map_native_evidence_search(result: Any) -> EvidenceSearchResultV1:
    _require_evidence_schema(result.schema_version, "/schemaVersion")
    if len(result.evidence) != len(result.search_result.results):
        _evidence_response_error("evidence_corrupt", "/evidence")
    for index, item in enumerate(result.evidence):
        _require_evidence_schema(item.schema_version, f"/evidence/{index}/schemaVersion")
        _require_u32(item.result_index, f"/evidence/{index}/resultIndex", optional=False)
        if item.result_index != index:
            _evidence_response_error("evidence_corrupt", f"/evidence/{index}/resultIndex")
    for index, hit in enumerate(result.search_result.results):
        _require_evidence_variant(
            hit.branch,
            {"vector", "text", "text_edge", "graph_arm"},
            f"/searchResult/results/{index}/branch",
        )
        _require_finite(hit.score, f"/searchResult/results/{index}/score", optional=False)
        _require_finite(hit.ce_score, f"/searchResult/results/{index}/ceScore")
    return EvidenceSearchResultV1(
        schema_version=result.schema_version,
        search_result=_map_native_search_result(result.search_result),
        evidence=tuple(
            EvidenceSidecarEntryV1(
                schema_version=item.schema_version,
                result_index=item.result_index,
                artifact_revision_id=item.artifact_revision_id,
                evidence_ref=item.evidence_ref,
            )
            for item in result.evidence
        ),
    )


def _map_native_resolved_evidence(value: Any) -> ResolvedEvidenceV1:
    _require_evidence_schema(value.schema_version, "/schemaVersion")
    _require_evidence_schema(
        value.projection_origin.schema_version, "/projectionOrigin/schemaVersion"
    )
    contribution = value.retrieval_contribution
    _require_evidence_schema(contribution.schema_version, "/retrievalContribution/schemaVersion")
    dependency = value.dependency
    if dependency is not None:
        _require_evidence_schema(dependency.schema_version, "/dependency/schemaVersion")

    _require_evidence_variant(value.locator_kind, {"whole_body", "utf8_bytes"}, "/locator/kind")
    if value.locator_kind == "whole_body":
        if value.locator_start_inclusive is not None or value.locator_end_exclusive is not None:
            _evidence_response_error("evidence_corrupt", "/locator")
    else:
        for field, candidate in [
            ("/locator/startInclusive", value.locator_start_inclusive),
            ("/locator/endExclusive", value.locator_end_exclusive),
        ]:
            if (
                not isinstance(candidate, int)
                or isinstance(candidate, bool)
                or not 0 <= candidate <= 2**64 - 1
            ):
                _evidence_response_error("evidence_corrupt", field)
        if value.locator_start_inclusive > value.locator_end_exclusive:
            _evidence_response_error("evidence_corrupt", "/locator")

    _require_evidence_variant(
        value.artifact_lifecycle_kind, {"node", "edge"}, "/artifactLifecycle/kind"
    )
    if not isinstance(value.artifact_superseded, bool):
        _evidence_response_error("evidence_corrupt", "/artifactLifecycle/superseded")
    if value.artifact_lifecycle_kind == "node":
        _require_evidence_variant(
            value.artifact_lifecycle_state,
            {"pending", "active", "deleted", "purged"},
            "/artifactLifecycle/state",
        )
        if value.artifact_valid_at_effective is not None:
            _evidence_response_error("evidence_corrupt", "/artifactLifecycle/validAtEffective")
    else:
        if value.artifact_lifecycle_state is not None:
            _evidence_response_error("evidence_corrupt", "/artifactLifecycle/state")
        if not isinstance(value.artifact_valid_at_effective, bool):
            _evidence_response_error("evidence_corrupt", "/artifactLifecycle/validAtEffective")
    _require_evidence_variant(
        value.source_lifecycle_state,
        {"pending", "active", "deleted", "purged"},
        "/sourceLifecycleState",
    )
    _require_evidence_variant(
        value.projection_origin.artifact_class,
        {"node", "edge"},
        "/projectionOrigin/artifactClass",
    )
    if value.projection_origin.artifact_class != value.artifact_lifecycle_kind:
        _evidence_response_error("evidence_corrupt", "/projectionOrigin/artifactClass")
    _require_evidence_variant(
        value.projection_origin.representative_arm,
        {"vector", "text", "text_edge", "graph_arm"},
        "/projectionOrigin/representativeArm",
    )
    graph_kind = value.projection_origin.graph_origin_kind
    if value.projection_origin.representative_arm == "graph_arm":
        if graph_kind is None:
            _evidence_response_error("evidence_corrupt", "/projectionOrigin/graphOrigin")
    elif graph_kind is not None:
        _evidence_response_error("evidence_corrupt", "/projectionOrigin/graphOrigin")
    if graph_kind is not None:
        _require_evidence_variant(
            graph_kind, {"edge_seed", "traversal"}, "/projectionOrigin/graphOrigin/kind"
        )
        edge_revision = value.projection_origin.graph_edge_artifact_revision_id
        hop_count = value.projection_origin.graph_hop_count
        _require_nonempty_string(
            edge_revision, "/projectionOrigin/graphOrigin/edgeArtifactRevisionId"
        )
        if graph_kind == "edge_seed" and hop_count is not None:
            _evidence_response_error("evidence_corrupt", "/projectionOrigin/graphOrigin/hopCount")
        if graph_kind == "traversal":
            _require_u32(hop_count, "/projectionOrigin/graphOrigin/hopCount", optional=False)
    for name, wire_name in [
        ("vector_rank", "vectorRank"),
        ("text_rank", "textRank"),
        ("graph_rank", "graphRank"),
    ]:
        _require_u32(getattr(contribution, name), f"/retrievalContribution/{wire_name}")
    for name, wire_name in [
        ("fused_score", "fusedScore"),
        ("blended_score", "blendedScore"),
    ]:
        _require_finite(
            getattr(contribution, name),
            f"/retrievalContribution/{wire_name}",
            optional=False,
        )
    for name, wire_name in [
        ("ce_score", "ceScore"),
        ("importance", "importance"),
        ("confidence", "confidence"),
    ]:
        _require_finite(getattr(contribution, name), f"/retrievalContribution/{wire_name}")
    if dependency is not None:
        generation_text = dependency.registered_dependency_generation
        if (
            not isinstance(generation_text, str)
            or not generation_text.isascii()
            or not generation_text.isdecimal()
            or (generation_text != "0" and generation_text.startswith("0"))
            or int(generation_text) > 2**64 - 1
        ):
            _evidence_response_error(
                "evidence_corrupt", "/dependency/registeredDependencyGeneration"
            )
    locator: Any = {"kind": value.locator_kind}
    if value.locator_kind == "utf8_bytes":
        locator.update(
            start_inclusive=value.locator_start_inclusive,
            end_exclusive=value.locator_end_exclusive,
        )
    graph_origin = (
        None
        if value.projection_origin.graph_origin_kind is None
        else EvidenceGraphOriginV1(
            kind=value.projection_origin.graph_origin_kind,
            edge_artifact_revision_id=(value.projection_origin.graph_edge_artifact_revision_id),
            hop_count=value.projection_origin.graph_hop_count,
        )
    )
    return ResolvedEvidenceV1(
        schema_version=value.schema_version,
        logical_id=value.logical_id,
        artifact_revision_id=value.artifact_revision_id,
        source_id=value.source_id,
        source_version_id=value.source_version_id,
        source_revision_id=value.source_revision_id,
        locator=locator,
        canonical_source_body=value.canonical_source_body,
        evidence_text=value.evidence_text,
        canonical_source_hash=value.canonical_source_hash,
        effective_valid_at=value.effective_valid_at,
        artifact_lifecycle=EvidenceArtifactLifecycleV1(
            kind=value.artifact_lifecycle_kind,
            state=value.artifact_lifecycle_state,
            superseded=value.artifact_superseded,
            valid_at_effective=value.artifact_valid_at_effective,
        ),
        source_lifecycle_state=value.source_lifecycle_state,
        projection_origin=EvidenceProjectionOriginV1(
            schema_version=value.projection_origin.schema_version,
            artifact_class=value.projection_origin.artifact_class,
            representative_arm=value.projection_origin.representative_arm,
            projection_generation_id=value.projection_origin.projection_generation_id,
            graph_origin=graph_origin,
        ),
        retrieval_contribution=EvidenceContributionV1(
            schema_version=contribution.schema_version,
            vector_rank=contribution.vector_rank,
            text_rank=contribution.text_rank,
            graph_rank=contribution.graph_rank,
            fused_score=contribution.fused_score,
            ce_score=contribution.ce_score,
            blended_score=contribution.blended_score,
            importance=contribution.importance,
            confidence=contribution.confidence,
        ),
        dependency=(
            None
            if dependency is None
            else SourceDependencyV1(
                schema_version=dependency.schema_version,
                dependency_id=dependency.dependency_id,
                source_revision_id=dependency.source_revision_id,
                derived_revision_id=dependency.derived_revision_id,
                registered_dependency_generation=(dependency.registered_dependency_generation),
            )
        ),
    )


def trace_dependency(self: Engine, request: DependencyTraceRequestV1) -> DependencyTraceResultV1:
    if not isinstance(request, DependencyTraceRequestV1):
        raise TypeError("request must be a DependencyTraceRequestV1")
    request.__post_init__()
    if (
        not isinstance(request.root_revision_id, str)
        or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._:-]{0,127}", request.root_revision_id) is None
        or request.root_revision_id.startswith("_fdb:")
    ):
        _trace_response_error("trace_root_invalid", "/rootRevisionId")
    if request.direction not in ("to_source", "to_dependents"):
        _trace_response_error("trace_direction_invalid", "/direction")
    if not isinstance(request.context, FrozenReadContextV1):
        _trace_response_error("trace_corrupt", "/context")
    context = _validate_frozen_trace_context(request.context)
    if (
        not isinstance(request.max_relations, int)
        or isinstance(request.max_relations, bool)
        or not 1 <= request.max_relations <= 100
    ):
        _trace_response_error("trace_limit_invalid", "/maxRelations")
    if (
        not isinstance(request.max_work_units, int)
        or isinstance(request.max_work_units, bool)
        or not 1 <= request.max_work_units <= 101
    ):
        _trace_response_error("trace_limit_invalid", "/maxWorkUnits")
    encoded = self._native.trace_dependency(
        request.root_revision_id,
        request.direction,
        _to_native_frozen_context(context),
        request.max_relations,
        request.max_work_units,
    )
    return _decode_dependency_trace_response(encoded)


def search_with_evidence(
    self: Engine,
    request: EvidenceSearchRequestV1,
) -> EvidenceSearchResultV1:
    if not isinstance(request, EvidenceSearchRequestV1):
        raise TypeError("request must be an EvidenceSearchRequestV1")
    if request.schema_version != 1:
        raise EvidenceError(
            "unsupported_schema_version at /schemaVersion",
            reason="unsupported_schema_version",
            field_path="/schemaVersion",
        )
    _validate_ranked_result_limit("limit", request.limit)
    if not isinstance(request.rerank_depth, int) or isinstance(request.rerank_depth, bool):
        raise TypeError("rerank_depth must be a non-negative integer")
    if request.rerank_depth < 0:
        raise InvalidArgumentError(f"rerank_depth must be >= 0, got {request.rerank_depth!r}")
    if request.rerank_depth > 2**32 - 1:
        raise InvalidArgumentError(
            f"rerank_depth must be <= 4294967295, got {request.rerank_depth!r}"
        )
    if not isinstance(request.use_graph_arm, bool):
        raise TypeError("use_graph_arm must be a bool")
    if isinstance(request.alpha, bool) or not isinstance(request.alpha, (int, float)):
        raise TypeError("alpha must be a finite number")
    if not math.isfinite(request.alpha):
        raise ValueError(f"alpha must be a finite number, got {request.alpha!r}")
    if not isinstance(request.pool_n, int) or isinstance(request.pool_n, bool):
        raise TypeError("pool_n must be a non-negative integer")
    if request.pool_n < 0:
        raise InvalidArgumentError(f"pool_n must be >= 0, got {request.pool_n!r}")
    if request.pool_n > 2**32 - 1:
        raise InvalidArgumentError(f"pool_n must be <= 4294967295, got {request.pool_n!r}")
    if not isinstance(request.include_explanation, bool):
        raise TypeError("include_explanation must be a bool")
    native_context = _to_native_frozen_context(request.context)
    native = self._native.search_with_evidence(
        request.query,
        native_context,
        rerank_depth=request.rerank_depth,
        use_graph_arm=request.use_graph_arm,
        alpha=request.alpha,
        pool_n=request.pool_n,
        include_explanation=request.include_explanation,
        limit=request.limit,
    )
    return _map_native_evidence_search(native)


def resolve_evidence(self: Engine, request: EvidenceResolveRequestV1) -> ResolvedEvidenceV1:
    if not isinstance(request, EvidenceResolveRequestV1):
        raise TypeError("request must be an EvidenceResolveRequestV1")
    if request.schema_version != 1:
        raise EvidenceError(
            "unsupported_schema_version at /schemaVersion",
            reason="unsupported_schema_version",
            field_path="/schemaVersion",
        )
    native_context = _to_native_frozen_context(request.context)
    return _map_native_resolved_evidence(
        self._native.resolve_evidence(request.evidence_ref, native_context)
    )
