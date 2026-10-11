"""Private graph operations for the Python Engine facade."""

from __future__ import annotations

import json
import math
import re
from typing import TYPE_CHECKING, Any, NoReturn, cast

from fathomdb._sdk_search import (
    _map_native_node,
    _map_native_search_hit,
    _to_native_frozen_context,
    _validate_ranked_result_limit,
)
from fathomdb.errors import (
    EvidenceError,
    GraphExpansionError,
    InvalidArgumentError,
)
from fathomdb.types import (
    EvidenceArtifactLifecycleV1,
    ExpandedNode,
    FrozenReadContextV1,
    GraphEvidenceArtifactV1,
    GraphEvidenceResolveRequestV1,
    GraphEvidenceSidecarEntryV1,
    GraphEvidenceSidecarV1,
    GraphExpandResultV1,
    GraphExpansionDegradationCodeV1,
    GraphExpansionExplanationV1,
    GraphOriginV1,
    GraphTargetExplanationV1,
    GraphTargetV1,
    ResolvedGraphEvidenceV1,
    ResolvedGraphSeedV1,
    SearchExpandResult,
    SourceDependencyV1,
)

if TYPE_CHECKING:
    from fathomdb.engine import Engine


def _graph_refuse(reason: str, field_path: str) -> NoReturn:
    raise GraphExpansionError(f"{reason} at {field_path}", reason=reason, field_path=field_path)


def _graph_field(value: Any, name: str, path: str) -> Any:
    if isinstance(value, dict):
        if name not in value:
            _graph_refuse("graph_corrupt", path)
        return value[name]
    if not hasattr(value, name):
        _graph_refuse("graph_corrupt", path)
    return getattr(value, name)


def _graph_schema(value: Any, path: str) -> None:
    schema = _graph_field(value, "schema_version", path)
    if type(schema) is not int or schema != 1:
        _graph_refuse("unsupported_schema_version", path)


def _graph_string(value: Any, path: str) -> str:
    if not isinstance(value, str):
        _graph_refuse("graph_corrupt", path)
    return value


def _graph_u32(value: Any, path: str) -> int:
    if type(value) is not int or not 0 <= value <= 0xFFFF_FFFF:
        _graph_refuse("graph_corrupt", path)
    return value


def _graph_u64(value: Any, path: str) -> str:
    if (
        not isinstance(value, str)
        or re.fullmatch(r"0|[1-9][0-9]*", value) is None
        or int(value) > 0xFFFF_FFFF_FFFF_FFFF
    ):
        _graph_refuse("graph_corrupt", path)
    return value


def _graph_enum(value: Any, allowed: set[str], path: str) -> str:
    value = _graph_string(value, path)
    if value not in allowed:
        _graph_refuse("graph_corrupt", path)
    return value


def _graph_list(value: Any, path: str) -> list[Any]:
    if not isinstance(value, (list, tuple)):
        _graph_refuse("graph_corrupt", path)
    return list(value)


def _graph_json_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _graph_refuse("graph_corrupt", path)
    return value


def _graph_json_closed(value: dict[str, Any], allowed: set[str], path: str) -> None:
    unknown = sorted(set(value) - allowed)
    if unknown:
        escaped = unknown[0].replace("~", "~0").replace("/", "~1")
        _graph_refuse("graph_corrupt", f"{path}/{escaped}")


def _graph_json_field(value: dict[str, Any], name: str, path: str) -> Any:
    if name not in value:
        _graph_refuse("graph_corrupt", path)
    return value[name]


def _graph_revision(value: Any, path: str) -> str:
    value = _graph_string(value, path)
    if re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._:-]{0,127}", value) is None or value.startswith(
        "_fdb:"
    ):
        _graph_refuse("graph_corrupt", path)
    return value


def _graph_json_resolved_evidence(value: Any) -> ResolvedGraphEvidenceV1:
    root = _graph_json_object(value, "")
    _graph_json_closed(
        root,
        {
            "schemaVersion",
            "artifactRevisionId",
            "artifact",
            "sourceId",
            "sourceVersionId",
            "sourceRevisionId",
            "locator",
            "canonicalSourceBody",
            "evidenceText",
            "canonicalSourceHash",
            "effectiveValidAt",
            "artifactLifecycle",
            "sourceLifecycleState",
            "dependency",
        },
        "",
    )
    schema_version = _graph_json_field(root, "schemaVersion", "/schemaVersion")
    if type(schema_version) is not int or schema_version != 1:
        _graph_refuse("unsupported_schema_version", "/schemaVersion")

    artifact_raw = _graph_json_object(_graph_json_field(root, "artifact", "/artifact"), "/artifact")
    artifact_class = _graph_enum(
        _graph_json_field(artifact_raw, "artifactClass", "/artifact/artifactClass"),
        {"node", "edge"},
        "/artifact/artifactClass",
    )
    artifact_fields = {"artifactClass", "logicalId", "kind", "body"}
    if artifact_class == "edge":
        artifact_fields.update({"from", "to"})
    _graph_json_closed(artifact_raw, artifact_fields, "/artifact")
    logical_id_raw = _graph_json_field(artifact_raw, "logicalId", "/artifact/logicalId")
    body_raw = _graph_json_field(artifact_raw, "body", "/artifact/body")
    if artifact_class == "node" and (logical_id_raw is None or body_raw is None):
        _graph_refuse(
            "graph_corrupt",
            "/artifact/logicalId" if logical_id_raw is None else "/artifact/body",
        )
    artifact = GraphEvidenceArtifactV1(
        artifact_class=cast(Any, artifact_class),
        logical_id=(
            None if logical_id_raw is None else _graph_string(logical_id_raw, "/artifact/logicalId")
        ),
        kind=_graph_string(
            _graph_json_field(artifact_raw, "kind", "/artifact/kind"), "/artifact/kind"
        ),
        body=None if body_raw is None else _graph_string(body_raw, "/artifact/body"),
        from_id=(
            None
            if artifact_class == "node"
            else _graph_string(
                _graph_json_field(artifact_raw, "from", "/artifact/from"), "/artifact/from"
            )
        ),
        to_id=(
            None
            if artifact_class == "node"
            else _graph_string(
                _graph_json_field(artifact_raw, "to", "/artifact/to"), "/artifact/to"
            )
        ),
    )

    locator_raw = _graph_json_object(_graph_json_field(root, "locator", "/locator"), "/locator")
    _graph_json_closed(locator_raw, {"kind", "startInclusive", "endExclusive"}, "/locator")
    locator_kind = _graph_enum(
        _graph_json_field(locator_raw, "kind", "/locator/kind"),
        {"whole_body", "utf8_bytes"},
        "/locator/kind",
    )
    start = _graph_json_field(locator_raw, "startInclusive", "/locator/startInclusive")
    end = _graph_json_field(locator_raw, "endExclusive", "/locator/endExclusive")
    locator: Any = {"kind": locator_kind}
    if locator_kind == "whole_body":
        if start is not None:
            _graph_refuse("graph_corrupt", "/locator/startInclusive")
        if end is not None:
            _graph_refuse("graph_corrupt", "/locator/endExclusive")
    else:
        start_value = _graph_u64(start, "/locator/startInclusive")
        end_value = _graph_u64(end, "/locator/endExclusive")
        if int(start_value) > int(end_value):
            _graph_refuse("graph_corrupt", "/locator")
        locator.update(start_inclusive=start_value, end_exclusive=end_value)

    hash_raw = _graph_json_object(
        _graph_json_field(root, "canonicalSourceHash", "/canonicalSourceHash"),
        "/canonicalSourceHash",
    )
    _graph_json_closed(hash_raw, {"algorithm", "digestHex"}, "/canonicalSourceHash")
    if _graph_json_field(hash_raw, "algorithm", "/canonicalSourceHash/algorithm") != "sha256":
        _graph_refuse("graph_corrupt", "/canonicalSourceHash/algorithm")
    digest_hex = _graph_string(
        _graph_json_field(hash_raw, "digestHex", "/canonicalSourceHash/digestHex"),
        "/canonicalSourceHash/digestHex",
    )
    if re.fullmatch(r"[0-9a-f]{64}", digest_hex) is None:
        _graph_refuse("graph_corrupt", "/canonicalSourceHash/digestHex")

    lifecycle_raw = _graph_json_object(
        _graph_json_field(root, "artifactLifecycle", "/artifactLifecycle"),
        "/artifactLifecycle",
    )
    _graph_json_closed(
        lifecycle_raw,
        {"kind", "state", "superseded", "validAtEffective"},
        "/artifactLifecycle",
    )
    lifecycle_kind = _graph_enum(
        _graph_json_field(lifecycle_raw, "kind", "/artifactLifecycle/kind"),
        {"node", "edge"},
        "/artifactLifecycle/kind",
    )
    if lifecycle_kind != artifact_class:
        _graph_refuse("graph_corrupt", "/artifactLifecycle/kind")
    state = _graph_json_field(lifecycle_raw, "state", "/artifactLifecycle/state")
    valid_at = _graph_json_field(
        lifecycle_raw, "validAtEffective", "/artifactLifecycle/validAtEffective"
    )
    if lifecycle_kind == "node":
        state = _graph_enum(state, {"pending", "active", "deleted"}, "/artifactLifecycle/state")
        if valid_at is not None:
            _graph_refuse("graph_corrupt", "/artifactLifecycle/validAtEffective")
    else:
        if state is not None:
            _graph_refuse("graph_corrupt", "/artifactLifecycle/state")
        if type(valid_at) is not bool:
            _graph_refuse("graph_corrupt", "/artifactLifecycle/validAtEffective")
    superseded = _graph_json_field(lifecycle_raw, "superseded", "/artifactLifecycle/superseded")
    if type(superseded) is not bool:
        _graph_refuse("graph_corrupt", "/artifactLifecycle/superseded")
    lifecycle = EvidenceArtifactLifecycleV1(
        kind=cast(Any, lifecycle_kind),
        state=cast(Any, state),
        superseded=superseded,
        valid_at_effective=valid_at,
    )

    artifact_revision_id = _graph_revision(
        _graph_json_field(root, "artifactRevisionId", "/artifactRevisionId"),
        "/artifactRevisionId",
    )
    source_revision_id = _graph_revision(
        _graph_json_field(root, "sourceRevisionId", "/sourceRevisionId"),
        "/sourceRevisionId",
    )
    dependency_raw = _graph_json_field(root, "dependency", "/dependency")
    dependency = None
    if dependency_raw is not None:
        item = _graph_json_object(dependency_raw, "/dependency")
        _graph_json_closed(
            item,
            {
                "schemaVersion",
                "dependencyId",
                "sourceRevisionId",
                "derivedRevisionId",
                "registeredDependencyGeneration",
            },
            "/dependency",
        )
        dependency_schema_version = _graph_json_field(
            item, "schemaVersion", "/dependency/schemaVersion"
        )
        if type(dependency_schema_version) is not int or dependency_schema_version != 1:
            _graph_refuse("unsupported_schema_version", "/dependency/schemaVersion")
        dependency_source_revision_id = _graph_revision(
            _graph_json_field(item, "sourceRevisionId", "/dependency/sourceRevisionId"),
            "/dependency/sourceRevisionId",
        )
        if dependency_source_revision_id != source_revision_id:
            _graph_refuse("graph_corrupt", "/dependency/sourceRevisionId")
        dependency_derived_revision_id = _graph_revision(
            _graph_json_field(item, "derivedRevisionId", "/dependency/derivedRevisionId"),
            "/dependency/derivedRevisionId",
        )
        if dependency_derived_revision_id != artifact_revision_id:
            _graph_refuse("graph_corrupt", "/dependency/derivedRevisionId")
        dependency = SourceDependencyV1(
            schema_version=1,
            dependency_id=_graph_string(
                _graph_json_field(item, "dependencyId", "/dependency/dependencyId"),
                "/dependency/dependencyId",
            ),
            source_revision_id=dependency_source_revision_id,
            derived_revision_id=dependency_derived_revision_id,
            registered_dependency_generation=_graph_u64(
                _graph_json_field(
                    item,
                    "registeredDependencyGeneration",
                    "/dependency/registeredDependencyGeneration",
                ),
                "/dependency/registeredDependencyGeneration",
            ),
        )

    effective_valid_at = _graph_json_field(root, "effectiveValidAt", "/effectiveValidAt")
    if type(effective_valid_at) is not int or not -(2**63) <= effective_valid_at < 2**63:
        _graph_refuse("graph_corrupt", "/effectiveValidAt")
    source_state = _graph_enum(
        _graph_json_field(root, "sourceLifecycleState", "/sourceLifecycleState"),
        {"pending", "active", "deleted"},
        "/sourceLifecycleState",
    )
    return ResolvedGraphEvidenceV1(
        schema_version=1,
        artifact_revision_id=artifact_revision_id,
        artifact=artifact,
        source_id=_graph_string(_graph_json_field(root, "sourceId", "/sourceId"), "/sourceId"),
        source_version_id=_graph_string(
            _graph_json_field(root, "sourceVersionId", "/sourceVersionId"), "/sourceVersionId"
        ),
        source_revision_id=source_revision_id,
        locator=locator,
        canonical_source_body=_graph_string(
            _graph_json_field(root, "canonicalSourceBody", "/canonicalSourceBody"),
            "/canonicalSourceBody",
        ),
        evidence_text=_graph_string(
            _graph_json_field(root, "evidenceText", "/evidenceText"), "/evidenceText"
        ),
        canonical_source_hash={"algorithm": "sha256", "digest_hex": digest_hex},
        effective_valid_at=effective_valid_at,
        artifact_lifecycle=lifecycle,
        source_lifecycle_state=source_state,
        dependency=dependency,
    )


def _map_graph_origin(value: Any, path: str) -> GraphOriginV1:
    _graph_schema(value, f"{path}/schemaVersion")
    return GraphOriginV1(
        schema_version=1,
        seed_logical_id=_graph_string(
            _graph_field(value, "seed_logical_id", f"{path}/seedLogicalId"),
            f"{path}/seedLogicalId",
        ),
        seed_ordinal=_graph_u32(
            _graph_field(value, "seed_ordinal", f"{path}/seedOrdinal"),
            f"{path}/seedOrdinal",
        ),
        predecessor_logical_id=_graph_string(
            _graph_field(value, "predecessor_logical_id", f"{path}/predecessorLogicalId"),
            f"{path}/predecessorLogicalId",
        ),
        target_logical_id=_graph_string(
            _graph_field(value, "target_logical_id", f"{path}/targetLogicalId"),
            f"{path}/targetLogicalId",
        ),
        hop_count=_graph_u32(
            _graph_field(value, "hop_count", f"{path}/hopCount"),
            f"{path}/hopCount",
        ),
        terminal_edge_kind=_graph_string(
            _graph_field(value, "terminal_edge_kind", f"{path}/terminalEdgeKind"),
            f"{path}/terminalEdgeKind",
        ),
        terminal_direction=cast(
            Any,
            _graph_enum(
                _graph_field(value, "terminal_direction", f"{path}/terminalDirection"),
                {"incoming", "outgoing", "both"},
                f"{path}/terminalDirection",
            ),
        ),
    )


def _map_native_graph_expand_result(result: Any) -> GraphExpandResultV1:
    """Validate and map an additive native graph-expansion response."""

    _graph_schema(result, "/schemaVersion")
    seed_values = _graph_list(_graph_field(result, "seeds", "/seeds"), "/seeds")
    seeds: list[ResolvedGraphSeedV1] = []
    for index, seed in enumerate(seed_values):
        base = f"/seeds/{index}"
        _graph_schema(seed, f"{base}/schemaVersion")
        ordinal = _graph_u32(
            _graph_field(seed, "seed_ordinal", f"{base}/seedOrdinal"),
            f"{base}/seedOrdinal",
        )
        if ordinal != index:
            _graph_refuse("graph_corrupt", f"{base}/seedOrdinal")
        score = _graph_field(seed, "query_score", f"{base}/queryScore")
        if score is not None and (
            isinstance(score, bool)
            or not isinstance(score, (int, float))
            or not math.isfinite(float(score))
        ):
            _graph_refuse("graph_corrupt", f"{base}/queryScore")
        seeds.append(
            ResolvedGraphSeedV1(
                schema_version=1,
                logical_id=_graph_string(
                    _graph_field(seed, "logical_id", f"{base}/logicalId"),
                    f"{base}/logicalId",
                ),
                seed_ordinal=ordinal,
                query_score=None if score is None else float(score),
            )
        )

    target_values = _graph_list(_graph_field(result, "targets", "/targets"), "/targets")
    targets: list[GraphTargetV1] = []
    for index, target in enumerate(target_values):
        base = f"/targets/{index}"
        _graph_schema(target, f"{base}/schemaVersion")
        targets.append(
            GraphTargetV1(
                schema_version=1,
                logical_id=_graph_string(
                    _graph_field(target, "logical_id", f"{base}/logicalId"),
                    f"{base}/logicalId",
                ),
                kind=_graph_string(_graph_field(target, "kind", f"{base}/kind"), f"{base}/kind"),
                body=_graph_string(_graph_field(target, "body", f"{base}/body"), f"{base}/body"),
                write_cursor=_graph_u64(
                    _graph_field(target, "write_cursor", f"{base}/writeCursor"),
                    f"{base}/writeCursor",
                ),
                origin=_map_graph_origin(
                    _graph_field(target, "origin", f"{base}/origin"), f"{base}/origin"
                ),
            )
        )

    complete = _graph_field(result, "complete", "/complete")
    if complete is not True:
        _graph_refuse("graph_corrupt", "/complete")
    work_units = _graph_u64(_graph_field(result, "work_units", "/workUnits"), "/workUnits")
    degradation_values = _graph_list(
        _graph_field(result, "degradation_codes", "/degradationCodes"),
        "/degradationCodes",
    )
    degradation_codes: tuple[GraphExpansionDegradationCodeV1, ...] = tuple(
        cast(
            GraphExpansionDegradationCodeV1,
            _graph_enum(
                value,
                {
                    "query_seed_text_fallback",
                    "projection_legacy_unverified",
                    "projection_processing",
                    "projection_blocked",
                    "projection_deferred",
                    "projection_degraded",
                },
                f"/degradationCodes/{index}",
            ),
        )
        for index, value in enumerate(degradation_values)
    )

    for index, target in enumerate(targets):
        ordinal = target.origin.seed_ordinal
        if ordinal >= len(seeds):
            _graph_refuse("graph_corrupt", f"/targets/{index}/origin/seedOrdinal")
        if target.origin.seed_logical_id != seeds[ordinal].logical_id:
            _graph_refuse("graph_corrupt", f"/targets/{index}/origin/seedLogicalId")
        if target.origin.target_logical_id != target.logical_id:
            _graph_refuse("graph_corrupt", f"/targets/{index}/origin/targetLogicalId")

    explanation_value = _graph_field(result, "explanation", "/explanation")
    explanation: GraphExpansionExplanationV1 | None = None
    if explanation_value is not None:
        base = "/explanation"
        _graph_schema(explanation_value, f"{base}/schemaVersion")
        per_target_values = _graph_list(
            _graph_field(explanation_value, "per_target", f"{base}/perTarget"),
            f"{base}/perTarget",
        )
        if len(per_target_values) != len(targets):
            _graph_refuse("graph_corrupt", f"{base}/perTarget")
        per_target: list[GraphTargetExplanationV1] = []
        for index, item in enumerate(per_target_values):
            item_base = f"{base}/perTarget/{index}"
            _graph_schema(item, f"{item_base}/schemaVersion")
            target_index = _graph_u32(
                _graph_field(item, "target_index", f"{item_base}/targetIndex"),
                f"{item_base}/targetIndex",
            )
            if target_index != index:
                _graph_refuse("graph_corrupt", f"{item_base}/targetIndex")
            origin = _map_graph_origin(
                _graph_field(item, "origin", f"{item_base}/origin"),
                f"{item_base}/origin",
            )
            if origin != targets[index].origin:
                _graph_refuse("graph_corrupt", f"{item_base}/origin")
            per_target.append(
                GraphTargetExplanationV1(
                    schema_version=1,
                    target_index=target_index,
                    origin=origin,
                    lifecycle_state=cast(
                        Any,
                        _graph_enum(
                            _graph_field(item, "lifecycle_state", f"{item_base}/lifecycleState"),
                            {"node_pending", "node_active", "node_deleted", "edge_valid"},
                            f"{item_base}/lifecycleState",
                        ),
                    ),
                    dependency_state=cast(
                        Any,
                        _graph_enum(
                            _graph_field(item, "dependency_state", f"{item_base}/dependencyState"),
                            {"not_applicable", "not_registered", "registered"},
                            f"{item_base}/dependencyState",
                        ),
                    ),
                )
            )
        explanation_degradations = tuple(
            cast(GraphExpansionDegradationCodeV1, value)
            for value in _graph_list(
                _graph_field(
                    explanation_value,
                    "degradation_codes",
                    f"{base}/degradationCodes",
                ),
                f"{base}/degradationCodes",
            )
        )
        if explanation_degradations != degradation_codes:
            _graph_refuse("graph_corrupt", f"{base}/degradationCodes")
        explanation = GraphExpansionExplanationV1(
            schema_version=1,
            correlation_id=_graph_string(
                _graph_field(explanation_value, "correlation_id", f"{base}/correlationId"),
                f"{base}/correlationId",
            ),
            seed_source=cast(
                Any,
                _graph_enum(
                    _graph_field(explanation_value, "seed_source", f"{base}/seedSource"),
                    {"query", "explicit"},
                    f"{base}/seedSource",
                ),
            ),
            read_mode=cast(
                Any,
                _graph_enum(
                    _graph_field(explanation_value, "read_mode", f"{base}/readMode"),
                    {"current", "frozen"},
                    f"{base}/readMode",
                ),
            ),
            projection_generation_id=(
                None
                if _graph_field(
                    explanation_value,
                    "projection_generation_id",
                    f"{base}/projectionGenerationId",
                )
                is None
                else _graph_string(
                    _graph_field(
                        explanation_value,
                        "projection_generation_id",
                        f"{base}/projectionGenerationId",
                    ),
                    f"{base}/projectionGenerationId",
                )
            ),
            projection_origin=cast(
                Any,
                _graph_enum(
                    _graph_field(
                        explanation_value, "projection_origin", f"{base}/projectionOrigin"
                    ),
                    {"not_applicable", "fresh", "legacy_unverified", "configuration", "rebuild"},
                    f"{base}/projectionOrigin",
                ),
            ),
            projection_readiness=cast(
                Any,
                _graph_enum(
                    _graph_field(
                        explanation_value,
                        "projection_readiness",
                        f"{base}/projectionReadiness",
                    ),
                    {"not_applicable", "ready", "processing", "blocked", "deferred", "degraded"},
                    f"{base}/projectionReadiness",
                ),
            ),
            degradation_codes=degradation_codes,
            per_target=tuple(per_target),
        )

    evidence_value = getattr(result, "evidence", None)
    evidence: GraphEvidenceSidecarV1 | None = None
    if evidence_value is not None:
        _graph_schema(evidence_value, "/evidence/schemaVersion")
        raw_entries = _graph_list(
            _graph_field(evidence_value, "entries", "/evidence/entries"), "/evidence/entries"
        )
        if len(raw_entries) != len(targets):
            _graph_refuse("graph_corrupt", "/evidence")
        entries: list[GraphEvidenceSidecarEntryV1] = []
        for index, item in enumerate(raw_entries):
            base = f"/evidence/entries/{index}"
            _graph_schema(item, f"{base}/schemaVersion")
            target_index = _graph_u32(
                _graph_field(item, "target_index", f"{base}/targetIndex"),
                f"{base}/targetIndex",
            )
            if target_index != index:
                _graph_refuse("graph_corrupt", f"{base}/targetIndex")
            entries.append(
                GraphEvidenceSidecarEntryV1(
                    schema_version=1,
                    target_index=target_index,
                    target_artifact_revision_id=_graph_revision(
                        _graph_field(
                            item,
                            "target_artifact_revision_id",
                            f"{base}/targetArtifactRevisionId",
                        ),
                        f"{base}/targetArtifactRevisionId",
                    ),
                    target_evidence_ref=_graph_string(
                        _graph_field(item, "target_evidence_ref", f"{base}/targetEvidenceRef"),
                        f"{base}/targetEvidenceRef",
                    ),
                    terminal_edge_artifact_revision_id=_graph_revision(
                        _graph_field(
                            item,
                            "terminal_edge_artifact_revision_id",
                            f"{base}/terminalEdgeArtifactRevisionId",
                        ),
                        f"{base}/terminalEdgeArtifactRevisionId",
                    ),
                    terminal_edge_evidence_ref=_graph_string(
                        _graph_field(
                            item,
                            "terminal_edge_evidence_ref",
                            f"{base}/terminalEdgeEvidenceRef",
                        ),
                        f"{base}/terminalEdgeEvidenceRef",
                    ),
                )
            )
        evidence = GraphEvidenceSidecarV1(entries=tuple(entries))

    return GraphExpandResultV1(
        schema_version=1,
        seeds=tuple(seeds),
        targets=tuple(targets),
        complete=True,
        work_units=work_units,
        degradation_codes=degradation_codes,
        explanation=explanation,
        evidence=evidence,
    )


def resolve_graph_evidence(
    self: Engine, request: GraphEvidenceResolveRequestV1
) -> ResolvedGraphEvidenceV1:
    if not isinstance(request, GraphEvidenceResolveRequestV1):
        raise TypeError("request must be a GraphEvidenceResolveRequestV1")
    if request.schema_version != 1:
        raise EvidenceError(
            "unsupported_schema_version at /schemaVersion",
            reason="unsupported_schema_version",
            field_path="/schemaVersion",
        )
    native_context = _to_native_frozen_context(request.context)
    try:
        raw = json.loads(self._native.resolve_graph_evidence(request.evidence_ref, native_context))
    except (TypeError, ValueError):
        _graph_refuse("graph_corrupt", "")
    return _graph_json_resolved_evidence(raw)


def search_expand_frozen(
    self: Engine,
    query: str,
    context: FrozenReadContextV1,
    depth: int,
    *,
    limit: int = 10,
) -> SearchExpandResult:
    if not isinstance(context, FrozenReadContextV1):
        raise TypeError(f"context must be a FrozenReadContextV1, got {type(context).__name__!r}")
    native_context = _to_native_frozen_context(context)
    self._native.validate_frozen_read_context(native_context)
    if not isinstance(depth, int) or isinstance(depth, bool):
        raise TypeError("depth must be an integer in 0..=3")
    if not 0 <= depth <= 3:
        raise InvalidArgumentError(f"depth must be an integer in 0..=3, got {depth!r}")
    _validate_ranked_result_limit("limit", limit)
    native = self._native.search_expand_frozen(
        query,
        native_context,
        depth,
        limit=limit,
    )
    return SearchExpandResult(
        search_hits=[_map_native_search_hit(hit) for hit in native.search_hits],
        expanded=[
            ExpandedNode(node=_map_native_node(item.node), hop_count=item.hop_count)
            for item in native.expanded
        ],
        all_logical_ids=list(native.all_logical_ids),
    )
