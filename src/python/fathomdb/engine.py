"""Python wrapper around the native PyO3 engine handle.

`Engine` mirrors the public five-verb surface owned by
`dev/interfaces/python.md`. The native PyO3 class
(`fathomdb._fathomdb.Engine`) holds the `Arc<fathomdb_engine::Engine>`
and runs every blocking call under `py.allow_threads`; this Python
wrapper converts native return values into the dataclasses in
`fathomdb.types` and rejects unknown `open()` kwargs.
"""

from __future__ import annotations

import logging
import math
import json
import re
from collections.abc import Sequence
from typing import Any, Literal, NoReturn, cast

from fathomdb._fathomdb import ConsolidateReceipt
from fathomdb._fathomdb import Engine as _NativeEngine
from fathomdb._fathomdb import FrozenReadContextV1 as _NativeFrozenReadContextV1
from fathomdb._fathomdb import ReadContextV1 as _NativeReadContextV1
from fathomdb._fathomdb import EraseReport
from fathomdb._fathomdb import IngestWithExtractorReceipt
from fathomdb._fathomdb import ProjectionSpec as _NativeProjectionSpec
from fathomdb._fathomdb import configure_projections as _native_configure_projections
from fathomdb._fathomdb import erase_source as _native_erase_source
from fathomdb._fathomdb import purge as _native_purge
from fathomdb._fathomdb import transition as _native_transition
from fathomdb.config import EngineConfig
from fathomdb.types import (
    ActuationBatchV1,
    ActuationReceiptV1,
    CounterSnapshot,
    ClosureLookupV1,
    ClosureProofV1,
    ClosureStatusV1,
    CudaDeviceInfo,
    CudaVisibleDevice,
    DeviceResolution,
    EmbedderIdentity,
    EffectiveEmbedDevice,
    EvidenceArtifactLifecycleV1,
    EvidenceContributionV1,
    EvidenceGraphOriginV1,
    EvidenceProjectionOriginV1,
    EvidenceResolveRequestV1,
    EvidenceSearchRequestV1,
    EvidenceSearchResultV1,
    EvidenceSidecarEntryV1,
    Explanation,
    ExpandedNode,
    FrozenReadContextV1,
    GpuAllocationWitness,
    GraphExpandResultV1,
    GraphEvidenceSidecarEntryV1,
    GraphEvidenceSidecarV1,
    GraphEvidenceResolveRequestV1,
    GraphEvidenceArtifactV1,
    GraphExpansionDegradationCodeV1,
    GraphExpansionExplanationV1,
    GraphOriginV1,
    GraphTargetExplanationV1,
    GraphTargetV1,
    IdSpace,
    MigrationStepReport,
    NodeRecord,
    OpenReport,
    PerHitExplain,
    ProjectionDelta,
    ProjectionSpec,
    QueryTrace,
    ReadContextV1,
    ReadView,
    ResolvedGraphSeedV1,
    ResolvedEvidenceV1,
    ResolvedGraphEvidenceV1,
    SearchExpandResult,
    SearchFilter,
    SearchHit,
    SearchResult,
    SoftFallback,
    SoftFallbackBranch,
    WriteReceipt,
    SourceDependencyRegistrationV1,
    DependencySourceLookupV1,
    DependencyDerivedLookupV1,
    SourceDependencyV1,
    StructuralInclusionV1,
    DependencyListV1,
    DependencyTraceEdgeV1,
    DependencyTraceNodeV1,
    DependencyTraceRequestV1,
    DependencyTraceResultV1,
    TraceNodeLifecycleV1,
    TraceReadBoundaryV1,
)
from fathomdb.filter import Filter
from fathomdb.errors import (
    DependencyTraceError,
    EvidenceError,
    FrozenReadError,
    GraphExpansionError,
    InvalidArgumentError,
)

# 0.8.20 Slice 15b fix-2 — reuse the read namespace's dataclass -> native
# ReadView translator rather than duplicating it here, so the two search entry
# points can never drift from the five read verbs. `fathomdb.read` imports
# `fathomdb.engine` only under TYPE_CHECKING, so this is not circular at runtime.
from fathomdb.read import _to_native_view


def _map_native_search_result(result: Any) -> SearchResult:
    fallback = result.soft_fallback
    soft = (
        SoftFallback(branch=cast(SoftFallbackBranch, fallback.branch))
        if fallback is not None
        else None
    )
    native_exp = result.explanation
    explanation = (
        _map_candidate_native_explanation(native_exp, result.results)
        if native_exp is not None
        else None
    )
    return SearchResult(
        projection_cursor=result.projection_cursor,
        soft_fallback=soft,
        results=[
            SearchHit(
                id=IdSpace(space=hit.id.space, value=hit.id.value),
                kind=hit.kind,
                body=hit.body,
                score=hit.score,
                branch=cast(SoftFallbackBranch, hit.branch),
                source_id=hit.source_id,
                ce_score=hit.ce_score,
            )
            for hit in result.results
        ],
        explanation=explanation,
    )


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
    if _graph_json_field(root, "schemaVersion", "/schemaVersion") != 1:
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
        if _graph_json_field(item, "schemaVersion", "/dependency/schemaVersion") != 1:
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


def _to_native_read_context(context: ReadContextV1) -> Any:
    eligibility = context.eligibility
    return _NativeReadContextV1(
        view=_to_native_view(context.view),
        source_type=eligibility.source_type,
        kind=eligibility.kind,
        created_after=eligibility.created_after,
        status=eligibility.status,
        attributes=list(eligibility.attributes),
        schema_version=context.schema_version,
    )


def _to_native_frozen_context(context: FrozenReadContextV1) -> Any:
    return _NativeFrozenReadContextV1(
        context.effective_valid_at,
        _to_native_read_context(context.context),
        context.token,
        schema_version=context.schema_version,
    )


def _map_native_search_hit(hit: Any) -> SearchHit:
    return SearchHit(
        id=IdSpace(space=hit.id.space, value=hit.id.value),
        kind=hit.kind,
        body=hit.body,
        score=hit.score,
        branch=cast(SoftFallbackBranch, hit.branch),
        source_id=hit.source_id,
        ce_score=hit.ce_score,
    )


def _map_native_node(node: Any) -> NodeRecord:
    return NodeRecord(
        logical_id=node.logical_id,
        kind=node.kind,
        body=node.body,
        write_cursor=node.write_cursor,
    )


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


_KWARG_FIELDS = {
    "embedder_pool_size",
    "scheduler_runtime_threads",
    "provenance_row_cap",
    "embedder_call_timeout_ms",
    "slow_threshold_ms",
}


def _validate_ranked_result_limit(name: str, limit: object) -> int:
    """Return a public ranked-result limit or raise the SDK's typed error."""
    if not isinstance(limit, int) or isinstance(limit, bool):
        raise TypeError(f"{name} must be an integer in 1..=100, got {type(limit).__name__!r}")
    if not 1 <= limit <= 100:
        raise InvalidArgumentError(f"{name} must be an integer in 1..=100, got {limit!r}")
    return limit


def _validate_id_list(name: str, value: object) -> list[int]:
    """0.8.8 Slice 15 — validate a relevance-label id list before the native
    call (mirrors the TS ``validateIdArray`` guard for cross-SDK parity). Ids
    are non-negative ints — the telemetry ``result_ids`` / ``write_cursor`` key
    space (the pre-0.8.19 ``SearchHit.id``), NOT the post-C-2 typed
    ``SearchHit.id``. ``bool`` is rejected explicitly (it is an int subclass that
    PyO3 would otherwise coerce silently)."""
    if not isinstance(value, list):
        raise TypeError(f"{name} must be a list of non-negative ints, got {type(value).__name__!r}")
    for item in value:
        if not isinstance(item, int) or isinstance(item, bool):
            raise TypeError(
                f"{name} must contain only non-negative ints, got {type(item).__name__!r}"
            )
        if item < 0:
            raise ValueError(f"{name} must contain only non-negative ints, got {item!r}")
    return value


def _projection_source_segments(source: object) -> list[str] | None:
    """Validate and normalize a nested projection's literal member path."""
    if source is None:
        return None
    if isinstance(source, str) or not isinstance(source, Sequence):
        raise TypeError("ProjectionSpec.source must be a non-string sequence of strings")
    if not all(isinstance(segment, str) for segment in source):
        raise TypeError("ProjectionSpec.source must be a non-string sequence of strings")
    return list(source)


def _map_per_hit_explain(p: Any) -> PerHitExplain:
    """Map one native per-hit explain object into the public
    :class:`fathomdb.types.PerHitExplain` dataclass.

    Factored out of :meth:`Engine.search` so the mapping is unit-testable
    against a fake native per-hit object without the compiled ``_fathomdb``
    extension (0.8.16 Slice 5 / F9, codex §9 fix-2). ``importance``/``confidence``
    are the additive F9 fields (node importance / edge confidence applied to this
    hit's contribution; ``None`` = graceful-absent / neutral), symmetric with the
    TypeScript ``perHit`` mapping.
    """

    def require_u64(value: object, path: str, *, optional: bool = False) -> None:
        if optional and value is None:
            return
        if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 2**64 - 1:
            _invalid_explanation(path)

    def require_u32(value: object, path: str) -> None:
        if value is None:
            return
        if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 2**32 - 1:
            _invalid_explanation(path)

    def require_finite(value: object, path: str, *, optional: bool = False) -> None:
        if optional and value is None:
            return
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
        ):
            _invalid_explanation(path)

    require_u64(getattr(p, "id", None), "/id")
    arm = getattr(p, "arm", None)
    if arm not in ("vector", "text", "text_edge", "graph_arm"):
        _invalid_explanation("/arm")
    require_u32(getattr(p, "vector_rank", None), "/vectorRank")
    require_u32(getattr(p, "text_rank", None), "/textRank")
    require_u32(getattr(p, "graph_rank", None), "/graphRank")
    require_finite(getattr(p, "fused_score", None), "/fusedScore")
    require_finite(getattr(p, "ce_score", None), "/ceScore", optional=True)
    require_finite(getattr(p, "blended", None), "/blended")
    require_finite(getattr(p, "importance", None), "/importance", optional=True)
    require_finite(getattr(p, "confidence", None), "/confidence", optional=True)
    native_structural = getattr(p, "structural", None)
    structural = (
        _map_structural_explanation(native_structural) if native_structural is not None else None
    )
    return PerHitExplain(
        id=p.id,
        arm=cast(SoftFallbackBranch, p.arm),
        vector_rank=p.vector_rank,
        text_rank=p.text_rank,
        graph_rank=p.graph_rank,
        fused_score=p.fused_score,
        ce_score=p.ce_score,
        blended=p.blended,
        importance=p.importance,
        confidence=p.confidence,
        structural=structural,
    )


def _invalid_explanation(path: str) -> NoReturn:
    raise ValueError(f"invalid explanation response at {path}")


def _map_candidate_native_explanation(value: Any, native_results: Sequence[Any]) -> Explanation:
    def require_u32(candidate: object, path: str, *, positive: bool = False) -> int:
        if (
            not isinstance(candidate, int)
            or isinstance(candidate, bool)
            or not 0 <= candidate <= 2**32 - 1
            or (positive and candidate == 0)
        ):
            _invalid_explanation(path)
        return candidate

    trace = getattr(value, "trace", None)
    if trace is None:
        _invalid_explanation("/trace")
    query_chars = require_u32(getattr(trace, "query_chars", None), "/trace/queryChars")
    k = require_u32(getattr(trace, "k", None), "/trace/k", positive=True)
    if k > 100:
        _invalid_explanation("/trace/k")
    rerank_depth = require_u32(getattr(trace, "rerank_depth", None), "/trace/rerankDepth")
    pool_n = require_u32(getattr(trace, "pool_n", None), "/trace/poolN")
    alpha = getattr(trace, "alpha", None)
    if isinstance(alpha, bool) or not isinstance(alpha, (int, float)) or not math.isfinite(alpha):
        _invalid_explanation("/trace/alpha")
    for name, path in (
        ("use_graph_arm", "/trace/useGraphArm"),
        ("recency", "/trace/recency"),
        ("ce_active", "/trace/ceActive"),
    ):
        if not isinstance(getattr(trace, name, None), bool):
            _invalid_explanation(path)
    embedder_id = getattr(trace, "embedder_id", None)
    if not isinstance(embedder_id, str):
        _invalid_explanation("/trace/embedderId")
    counts = [
        require_u32(getattr(trace, name, None), path)
        for name, path in (
            ("vector_hits", "/trace/vectorHits"),
            ("text_hits", "/trace/textHits"),
            ("graph_hits", "/trace/graphHits"),
            ("dropped_edge_hits", "/trace/droppedEdgeHits"),
        )
    ]
    correlation = getattr(value, "correlation_id", "")
    if hasattr(value, "correlation_id") and (
        not isinstance(correlation, str)
        or not re.fullmatch(r"(?:q[0-9]+|x[0-9a-f]{32})-(?:0|[1-9][0-9]*)", correlation)
    ):
        _invalid_explanation("/correlationId")
    native_per_hit = getattr(value, "per_hit", None)
    if not isinstance(native_per_hit, (list, tuple)) or len(native_per_hit) != len(native_results):
        _invalid_explanation("/perHit")
    mapped: list[PerHitExplain] = []
    seen_ids: set[int] = set()
    allowed_arms = ("vector", "text", "text_edge", "graph_arm")
    for index, (native_explain, native_hit) in enumerate(zip(native_per_hit, native_results)):
        prefix = f"/perHit/{index}"
        try:
            item = _map_per_hit_explain(native_explain)
        except ValueError as error:
            marker = "invalid explanation response at "
            suffix = str(error).removeprefix(marker)
            _invalid_explanation(f"{prefix}{suffix}")
        if item.id in seen_ids:
            _invalid_explanation(f"{prefix}/id")
        seen_ids.add(item.id)
        hit_branch = getattr(native_hit, "branch", None)
        if hit_branch not in allowed_arms:
            _invalid_explanation(f"/results/{index}/branch")
        hit_score = getattr(native_hit, "score", None)
        if (
            isinstance(hit_score, bool)
            or not isinstance(hit_score, (int, float))
            or not math.isfinite(hit_score)
        ):
            _invalid_explanation(f"/results/{index}/score")
        hit_ce_score = getattr(native_hit, "ce_score", None)
        if hit_ce_score is not None and (
            isinstance(hit_ce_score, bool)
            or not isinstance(hit_ce_score, (int, float))
            or not math.isfinite(hit_ce_score)
            or not 0 <= hit_ce_score <= 1
        ):
            _invalid_explanation(f"/results/{index}/ceScore")
        if item.arm != hit_branch:
            _invalid_explanation(f"{prefix}/arm")
        if item.blended != hit_score:
            _invalid_explanation(f"{prefix}/blended")
        if item.ce_score != hit_ce_score:
            _invalid_explanation(f"{prefix}/ceScore")
        mapped.append(item)
    return Explanation(
        trace=QueryTrace(
            query_chars=query_chars,
            k=k,
            rerank_depth=rerank_depth,
            pool_n=pool_n,
            alpha=float(alpha),
            use_graph_arm=trace.use_graph_arm,
            recency=trace.recency,
            embedder_id=embedder_id,
            ce_active=trace.ce_active,
            vector_hits=counts[0],
            text_hits=counts[1],
            graph_hits=counts[2],
            dropped_edge_hits=counts[3],
        ),
        per_hit=mapped,
        correlation_id=correlation,
    )


def _frozen_trace_error(reason: str, path: str) -> NoReturn:
    raise FrozenReadError(f"{reason} at {path}", reason=reason, field_path=path)


def _validate_frozen_trace_keys(value: Any, allowed: set[str], path: str) -> None:
    unknown = sorted(set(vars(value)) - allowed)
    if unknown:
        escaped = unknown[0].replace("~", "~0").replace("/", "~1")
        _frozen_trace_error("context_invalid", f"{path}/{escaped}")


def _validate_frozen_trace_context(value: Any) -> FrozenReadContextV1:
    if not isinstance(value, FrozenReadContextV1):
        _frozen_trace_error("context_invalid", "/context")
    _validate_frozen_trace_keys(
        value,
        {"schema_version", "effective_valid_at", "context", "token"},
        "/context",
    )
    if (
        not isinstance(value.schema_version, int)
        or isinstance(value.schema_version, bool)
        or value.schema_version != 1
    ):
        _frozen_trace_error("unsupported_schema_version", "/context/schemaVersion")
    if (
        not isinstance(value.effective_valid_at, int)
        or isinstance(value.effective_valid_at, bool)
        or not -(2**63) <= value.effective_valid_at < 2**63
    ):
        _frozen_trace_error("context_invalid", "/context/effectiveValidAt")
    if not isinstance(value.token, str):
        _frozen_trace_error("token_malformed", "/context/token")
    if len(value.token.encode("utf-8")) > 1024:
        _frozen_trace_error("token_too_large", "/context/token")
    context = value.context
    if not isinstance(context, ReadContextV1):
        _frozen_trace_error("context_invalid", "/context/context")
    _validate_frozen_trace_keys(
        context,
        {"schema_version", "view", "eligibility"},
        "/context/context",
    )
    if (
        not isinstance(context.schema_version, int)
        or isinstance(context.schema_version, bool)
        or context.schema_version != 1
    ):
        _frozen_trace_error("unsupported_schema_version", "/context/context/schemaVersion")
    view = context.view
    if not isinstance(view, ReadView):
        _frozen_trace_error("context_invalid", "/context/context/view")
    _validate_frozen_trace_keys(
        view,
        {
            "include_superseded",
            "include_inactive",
            "include_out_of_window",
            "valid_as_of",
        },
        "/context/context/view",
    )
    for name, path in (
        ("include_superseded", "includeSuperseded"),
        ("include_inactive", "includeInactive"),
        ("include_out_of_window", "includeOutOfWindow"),
    ):
        if not isinstance(getattr(view, name, None), bool):
            _frozen_trace_error("context_invalid", f"/context/context/view/{path}")
    if view.valid_as_of is not None and (
        not isinstance(view.valid_as_of, int)
        or isinstance(view.valid_as_of, bool)
        or not -(2**63) <= view.valid_as_of < 2**63
    ):
        _frozen_trace_error("context_invalid", "/context/context/view/validAsOf")
    eligibility = context.eligibility
    if not isinstance(eligibility, SearchFilter):
        _frozen_trace_error("context_invalid", "/context/context/eligibility")
    _validate_frozen_trace_keys(
        eligibility,
        {"source_type", "kind", "created_after", "status", "attributes"},
        "/context/context/eligibility",
    )
    for name, path in (
        ("source_type", "sourceType"),
        ("kind", "kind"),
        ("status", "status"),
    ):
        candidate = getattr(eligibility, name, None)
        if candidate is not None and not isinstance(candidate, str):
            _frozen_trace_error("context_invalid", f"/context/context/eligibility/{path}")
    if eligibility.created_after is not None and (
        not isinstance(eligibility.created_after, int)
        or isinstance(eligibility.created_after, bool)
        or not -(2**63) <= eligibility.created_after < 2**63
    ):
        _frozen_trace_error("context_invalid", "/context/context/eligibility/createdAfter")
    attributes = eligibility.attributes
    if not isinstance(attributes, (list, tuple)):
        _frozen_trace_error("context_invalid", "/context/context/eligibility/attributes")
    if len(attributes) > 64:
        _frozen_trace_error("context_invalid", "/context/context/eligibility/attributes")
    for index, pair in enumerate(attributes):
        if (
            not isinstance(pair, (list, tuple))
            or len(pair) != 2
            or not all(isinstance(item, str) for item in pair)
        ):
            _frozen_trace_error(
                "context_invalid",
                f"/context/context/eligibility/attributes/{index}",
            )
    return value


def _map_structural_explanation(value: Any) -> StructuralInclusionV1:
    if getattr(value, "schema_version", None) != 1:
        _invalid_explanation("/structural/schemaVersion")
    inclusion = getattr(value, "inclusion_state", None)
    if inclusion not in ("included", "degraded"):
        _invalid_explanation("/structural/inclusionState")
    projection = getattr(value, "projection_origin", None)
    if projection not in (
        "synchronous_body_fts",
        "current_dense_generation",
        "graph_traversal",
    ):
        _invalid_explanation("/structural/projectionOrigin")
    dependency = getattr(value, "dependency_state", None)
    if dependency not in ("not_applicable", "not_registered", "registered"):
        _invalid_explanation("/structural/dependencyState")
    lifecycle = getattr(value, "lifecycle_state", None)
    if lifecycle not in ("node_pending", "node_active", "node_deleted", "edge_valid"):
        _invalid_explanation("/structural/lifecycleState")
    codes = getattr(value, "degradation_codes", None)
    if not isinstance(codes, (list, tuple)):
        _invalid_explanation("/structural/degradationCodes")
    order = (
        "soft_fallback_text",
        "soft_fallback_text_edge",
        "projection_legacy_unverified",
        "projection_blocked",
        "projection_deferred",
        "graph_bound_reached",
    )
    previous = -1
    for index, code in enumerate(codes):
        if code not in order:
            _invalid_explanation(f"/structural/degradationCodes/{index}")
        ordinal = order.index(code)
        if ordinal <= previous:
            _invalid_explanation(f"/structural/degradationCodes/{index}")
        previous = ordinal
    if (inclusion == "included") != (len(codes) == 0):
        _invalid_explanation("/structural/inclusionState")
    return StructuralInclusionV1(
        schema_version=1,
        inclusion_state=inclusion,
        projection_origin=projection,
        dependency_state=dependency,
        lifecycle_state=lifecycle,
        degradation_codes=tuple(codes),
    )


def _map_gpu_allocation_witness(native: Any) -> GpuAllocationWitness:
    """Map the native GPU allocation witness field-for-field.

    Deliberately exhaustive: R80-13 requires the record stay re-derivable, so
    nothing here summarizes, rounds, or drops a number.
    """

    return GpuAllocationWitness(
        schema=native.schema,
        sole_gpu_consumer_precondition=native.sole_gpu_consumer_precondition,
        device_ordinal_requested=native.device_ordinal_requested,
        device_ordinal_actual=native.device_ordinal_actual,
        device_uuid=native.device_uuid,
        device_name=native.device_name,
        compute_capability=native.compute_capability,
        free_before_bytes=native.free_before_bytes,
        free_after_bytes=native.free_after_bytes,
        total_bytes=native.total_bytes,
        delta_bytes=native.delta_bytes,
        delta_floor_bytes=native.delta_floor_bytes,
        control_allocation_request_bytes=native.control_allocation_request_bytes,
        control_block_count=native.control_block_count,
        control_free_before_bytes=native.control_free_before_bytes,
        control_free_after_bytes=native.control_free_after_bytes,
        control_delta_bytes=native.control_delta_bytes,
        embedded_vector_dim=native.embedded_vector_dim,
    )


def _map_open_report(native: Any) -> OpenReport:
    """Map one native open-time snapshot into the public Python contract."""

    device_resolution = native.embedder_device_resolution
    reranker_device_resolution = native.reranker_device_resolution
    gpu_allocation_witness = native.embedder_gpu_allocation_witness
    return OpenReport(
        schema_version_before=native.schema_version_before,
        schema_version_after=native.schema_version_after,
        migration_steps=[
            MigrationStepReport(
                step_id=step.step_id,
                duration_ms=step.duration_ms,
                failed=step.failed,
            )
            for step in native.migration_steps
        ],
        embedder_warmup_ms=native.embedder_warmup_ms,
        query_backend=native.query_backend,
        default_embedder=EmbedderIdentity(
            name=native.default_embedder.name,
            revision=native.default_embedder.revision,
            dimension=native.default_embedder.dimension,
        ),
        embedder_download_ms=native.embedder_download_ms,
        embedder_events=list(native.embedder_events),
        embedder_mean_centering_required=native.embedder_mean_centering_required,
        embedder_mean_vec_pinned=native.embedder_mean_vec_pinned,
        dense_disabled=native.dense_disabled,
        dense_disabled_reason=native.dense_disabled_reason,
        embedder_device_resolution=(
            None
            if device_resolution is None
            else DeviceResolution(
                requested_policy=device_resolution.requested_policy,
                cuda_compiled=device_resolution.cuda_compiled,
                effective_device=EffectiveEmbedDevice(
                    kind=cast(Literal["cpu", "cuda"], device_resolution.effective_device.kind),
                    cuda_device=(
                        None
                        if device_resolution.effective_device.cuda_device is None
                        else CudaDeviceInfo(
                            ordinal=device_resolution.effective_device.cuda_device.ordinal,
                            uuid=device_resolution.effective_device.cuda_device.uuid,
                            name=device_resolution.effective_device.cuda_device.name,
                            driver_version=device_resolution.effective_device.cuda_device.driver_version,
                            compute_capability=device_resolution.effective_device.cuda_device.compute_capability,
                            cuda_toolkit_version=(
                                device_resolution.effective_device.cuda_device.cuda_toolkit_version
                            ),
                        )
                    ),
                ),
                visible_cuda_devices=tuple(
                    CudaVisibleDevice(
                        visible_ordinal=device.visible_ordinal,
                        uuid=device.uuid,
                        name=device.name,
                        compute_capability=device.compute_capability,
                    )
                    for device in device_resolution.visible_cuda_devices
                ),
                selected_cuda_uuid=device_resolution.selected_cuda_uuid,
                reason=device_resolution.reason,
            )
        ),
        reranker_device_resolution=(
            None
            if reranker_device_resolution is None
            else DeviceResolution(
                requested_policy=reranker_device_resolution.requested_policy,
                cuda_compiled=reranker_device_resolution.cuda_compiled,
                effective_device=EffectiveEmbedDevice(
                    kind=cast(
                        Literal["cpu", "cuda"],
                        reranker_device_resolution.effective_device.kind,
                    ),
                    cuda_device=(
                        None
                        if reranker_device_resolution.effective_device.cuda_device is None
                        else CudaDeviceInfo(
                            ordinal=reranker_device_resolution.effective_device.cuda_device.ordinal,
                            uuid=reranker_device_resolution.effective_device.cuda_device.uuid,
                            name=reranker_device_resolution.effective_device.cuda_device.name,
                            driver_version=reranker_device_resolution.effective_device.cuda_device.driver_version,
                            compute_capability=reranker_device_resolution.effective_device.cuda_device.compute_capability,
                            cuda_toolkit_version=reranker_device_resolution.effective_device.cuda_device.cuda_toolkit_version,
                        )
                    ),
                ),
                visible_cuda_devices=tuple(
                    CudaVisibleDevice(
                        visible_ordinal=device.visible_ordinal,
                        uuid=device.uuid,
                        name=device.name,
                        compute_capability=device.compute_capability,
                    )
                    for device in reranker_device_resolution.visible_cuda_devices
                ),
                selected_cuda_uuid=reranker_device_resolution.selected_cuda_uuid,
                reason=reranker_device_resolution.reason,
            )
        ),
        embedder_gpu_allocation_witness=(
            None
            if gpu_allocation_witness is None
            else _map_gpu_allocation_witness(gpu_allocation_witness)
        ),
    )


class Engine:
    """Python handle that wraps the native PyO3 engine."""

    __slots__ = ("_native", "_path", "_config")

    def __init__(
        self,
        native: _NativeEngine,
        *,
        path: str,
        config: EngineConfig,
    ) -> None:
        self._native = native
        self._path = path
        self._config = config

    @classmethod
    def open(
        cls,
        path: str,
        *,
        config: EngineConfig | None = None,
        use_default_embedder: bool = False,
        **engine_config: Any,
    ) -> "Engine":
        """Open the database at `path`.

        Either `config` or per-knob keyword arguments may be supplied,
        but not both. Unknown keyword arguments are rejected.

        EU-6: ``use_default_embedder`` opts into the engine's pinned
        default embedder (``fathomdb-bge-small-en-v1.5``). On first use,
        weights are downloaded from HuggingFace and cached under
        ``~/.cache/fathomdb/embedders/``. The default (``False``) opens
        without an embedder; subsequent vector writes fail with
        ``EmbedderNotConfiguredError``. Caller-supplied custom embedders
        are deferred to a later release (see ``dev/interfaces/python.md``).
        """

        if config is not None and engine_config:
            raise ValueError(
                "Engine.open accepts either config= or per-knob keyword arguments, not both",
            )

        unknown = set(engine_config) - _KWARG_FIELDS
        if unknown:
            raise TypeError(
                f"Engine.open got unexpected keyword arguments: {sorted(unknown)!r}",
            )

        resolved = config if config is not None else EngineConfig(**engine_config)
        native = _NativeEngine.open(path, use_default_embedder=use_default_embedder)
        return cls(native, path=path, config=resolved)

    @property
    def path(self) -> str:
        return self._path

    @property
    def config(self) -> EngineConfig:
        return self._config

    def write(self, batch: list[Any] | None = None) -> WriteReceipt:
        """Write a batch of items.

        ``source_id`` is MANDATORY on every canonical item (0.8.20 R-20-E3) —
        a row written without it can never be erased by :meth:`erase_source`.

        A node item is ``{"kind", "body", "source_id", "logical_id"?, "state"?,
        "reason"?, "valid_from"?, "valid_until"?}``; an edge item is
        ``{"edge": {"kind", "from", "to", "source_id", ...}}``.

        ``valid_from`` / ``valid_until`` (0.8.20 Slice 15b, TC-34) author the
        node's WORLD-TIME validity window as INTEGER epoch SECONDS. The window
        is HALF-OPEN — ``valid_from`` is inclusive, ``valid_until`` is exclusive
        — and an omitted (or ``None``) bound means unbounded on that side, so
        omitting both (the default) makes the node valid at every instant. Read
        it back through the ``valid_as_of`` field of a
        :class:`~fathomdb.types.ReadView`, or ask which nodes crossed a boundary
        with :func:`fathomdb.read.crossed_boundary_since`.

        Because the window is half-open, ``valid_from >= valid_until`` describes
        a window no instant can satisfy; that pair raises
        ``WriteValidationError`` rather than being silently stored. A
        non-integer bound raises ``WriteValidationError`` too — it is never
        coerced (``True`` is rejected too, even though ``bool`` subclasses
        ``int``). One family for the whole write-validation boundary.

        **BREAKING (0.8.20 Slice 22, decision #18).** The unsatisfiable-window
        pair used to raise ``InvalidArgumentError`` carrying both bounds. It is
        now ``WriteValidationError``, and that error is **message-less** — the
        offending bounds are no longer recoverable from it, so validate the
        pair before calling.
        """
        receipt = self._native.write(batch or [])
        return WriteReceipt(
            cursor=receipt.cursor,
            row_cursors=tuple(receipt.row_cursors),
            dangling_edge_endpoints=receipt.dangling_edge_endpoints,
        )

    def register_source_dependency(
        self, request: SourceDependencyRegistrationV1
    ) -> SourceDependencyV1:
        """Register one pinned dependency; exact replay is a no-op success."""
        value = self._native.register_source_dependency(request)
        return SourceDependencyV1(
            schema_version=value.schema_version,
            dependency_id=value.dependency_id,
            source_revision_id=value.source_revision_id,
            derived_revision_id=value.derived_revision_id,
            registered_dependency_generation=value.registered_dependency_generation,
        )

    def actuate(self, request: ActuationBatchV1) -> ActuationReceiptV1:
        """Atomically apply a bounded set of caller-decided memory operations."""
        value = self._native.actuate(request)
        return ActuationReceiptV1(
            schema_version=value.schema_version,
            operation_id=value.operation_id,
            request_sha256=value.request_sha256,
            outcome=cast(
                Literal["committed", "committed_closure_pending", "refused"],
                value.outcome,
            ),
            refused_operation_index=value.refused_operation_index,
            refused_field_path=value.refused_field_path,
            reason_codes=tuple(value.reason_codes),
            affected_revision_ids=tuple(value.affected_revision_ids),
            resulting_write_boundary=value.resulting_write_boundary,
            resulting_dependency_generation=value.resulting_dependency_generation,
            pending_projection_write_cursors=tuple(value.pending_projection_write_cursors),
            projection_generation_id=value.projection_generation_id,
            closure_operation_ids=tuple(value.closure_operation_ids),
        )

    def dependencies_for_source(self, request: DependencySourceLookupV1) -> DependencyListV1:
        """Return at most 100 dependencies in stable derived-revision order."""
        value = self._native.dependencies_for_source(request)
        return DependencyListV1(
            schema_version=value.schema_version,
            items=tuple(
                SourceDependencyV1(
                    schema_version=item.schema_version,
                    dependency_id=item.dependency_id,
                    source_revision_id=item.source_revision_id,
                    derived_revision_id=item.derived_revision_id,
                    registered_dependency_generation=item.registered_dependency_generation,
                )
                for item in value.items
            ),
        )

    def dependency_for_derived(
        self, request: DependencyDerivedLookupV1
    ) -> SourceDependencyV1 | None:
        """Return the dependency for one derived revision, or ``None``."""
        value = self._native.dependency_for_derived(request)
        if value is None:
            return None
        return SourceDependencyV1(
            schema_version=value.schema_version,
            dependency_id=value.dependency_id,
            source_revision_id=value.source_revision_id,
            derived_revision_id=value.derived_revision_id,
            registered_dependency_generation=value.registered_dependency_generation,
        )

    def read_dependency_closure(self, request: ClosureLookupV1) -> ClosureStatusV1 | None:
        """Return current closure status, or ``None`` for an absent opaque ID."""
        value = self._native.read_dependency_closure(request)
        if value is None:
            return None
        root = (
            {"type": "source_revision", "source_revision_id": value.source_revision_id}
            if value.root_type == "source_revision"
            else {"type": "source_bucket", "source_id": value.source_id}
        )
        proof = value.proof
        return ClosureStatusV1(
            schema_version=value.schema_version,
            closure_operation_id=value.closure_operation_id,
            root=root,
            cause=value.cause,
            phase=value.phase,
            effective_at_epoch_s=value.effective_at_epoch_s,
            admitted_write_boundary=value.admitted_write_boundary,
            admitted_dependency_generation=value.admitted_dependency_generation,
            affected_count=value.affected_count,
            blocker_code=value.blocker_code,
            proof=None
            if proof is None
            else ClosureProofV1(
                schema_version=proof.schema_version,
                proof_write_boundary=proof.proof_write_boundary,
                current_active_dependent_nodes=proof.current_active_dependent_nodes,
                current_derived_edges=proof.current_derived_edges,
                view_eligible_dependents=proof.view_eligible_dependents,
                ownerless_projection_rows=proof.ownerless_projection_rows,
                post_admission_registrations=proof.post_admission_registrations,
                remaining_dependency_rows=proof.remaining_dependency_rows,
                remaining_canonical_rows=proof.remaining_canonical_rows,
                remaining_projection_rows=proof.remaining_projection_rows,
                remaining_receipt_reference_rows=proof.remaining_receipt_reference_rows,
            ),
        )

    def transition(self, logical_id: str, to_state: str, reason: str | None = None) -> None:
        """OPP-12 Phase-1 (0.8.19 Slice 10) — the ``transition`` lifecycle verb.

        Move a governed node between existence states per the engine-enforced
        legal-transition table: promote ``pending``→``active``, reject
        ``pending``→``deleted``, soft-delete ``active``→``deleted``, undelete
        ``deleted``→``active``. Promote/undelete CLEAR ``reason``;
        reject/soft-delete SET it (``reason`` is advisory, never engine-
        interpreted). Keys on the bare ``logical_id`` (``l:`` space only) — a
        non-``l:`` id raises ``NotLifecycleAddressableError``; an illegal move
        (``purged``/``pending`` targets, self-loops, an absent node) raises
        ``IllegalTransitionError`` with ``from_state``/``to_state``/``legal``.
        Thin pass-through (no client-side logic)."""
        _native_transition(self._native, logical_id, to_state, reason)

    def purge(self, logical_id: str) -> None:
        """OPP-12 Phase-1 (0.8.19 Slice 10) — the ``purge`` lifecycle verb.

        Irreversibly hard-erase a governed node across every row-owned target
        (all versions + FTS/vector shadows + touching edges, cascade-removed).
        A SEPARATE verb from ``transition`` (NOT a recovery-denylist name).
        Precondition: DELETED-FIRST (legal only from ``deleted``; else
        ``IllegalTransitionError``); IDEMPOTENT (purging an absent/already-purged
        id is a no-op success). Keys on the bare ``logical_id`` (``l:`` only) — a
        non-``l:`` id raises ``NotLifecycleAddressableError``. Thin pass-through."""
        _native_purge(self._native, logical_id)

    def erase_source(self, source_id: str) -> EraseReport:
        """0.8.20 (R-20-E4) — the ``erase_source`` lifecycle verb.

        Erase every canonical row carrying ``source_id``, together with its
        row-owned projections (FTS5, vec0, ``search_index_v2``), and finish the
        erasure at rest (telemetry redaction + WAL truncation).

        The COMPANION to :meth:`purge`, not a duplicate of it. ``purge``
        addresses a *governed* node by ``logical_id``; ``erase_source``
        addresses *anonymous* content — rows written with no ``logical_id``,
        which ``purge`` cannot reach at all. Together they make every canonical
        row erasable from the SDK alone, with no CLI on ``PATH``.

        Idempotent: erasing an absent or already-erased source is a zero-count
        success, so an interrupted erasure obligation can be retried without a
        pre-check.

        Raises ``WriteValidationError`` for an empty, whitespace-only or
        reserved (``_``-prefixed) ``source_id``. The engine's reserved
        namespace (``_engine:*`` substrate and the ``_legacy:pre-0.8.20``
        migration cohort) is reachable ONLY through the CLI recovery seam
        ``fathomdb recover --excise-source``; a single governed call against it
        would erase every pre-0.8.20 anonymous row.

        NOT a recovery verb: ``erase_source`` carries no REQ-054
        recovery-denylist name, so AC-041 is unaffected. Thin pass-through."""
        return _native_erase_source(self._native, source_id)

    def configure_projections(
        self,
        specs: list[ProjectionSpec],
        drop: list[str] | None = None,
    ) -> ProjectionDelta:
        """0.8.20 Slice 15d (R-20-PR / C-1) — the ``configure_projections`` verb.

        Declaratively apply projection declarations. The engine is the SOLE
        projection authority: it diffs ``specs`` against the durable registry and
        backfills the difference in ONE transaction. Cheap projections
        (``filterable``, ``searchable→FTS``) build same-transaction; ``rankable``
        and the ``searchable→vector`` sub-target are persisted-but-deferred (F9 /
        Slice 20).

        ``drop`` is EXPLICIT: omitting a live projection from ``specs`` does NOT
        drop it; removal requires naming it in ``drop``. A destructive change to a
        live projection (a role removal or a tokenizer/embedder change) that is
        NOT in ``drop`` raises ``ProjectionDestructiveError`` with the destructive
        delta — never silent data loss. Re-applying an unchanged spec returns a
        ``ProjectionDelta`` with ``unchanged=True``.

        Pair with :func:`fathomdb.read.projections` to inspect current state
        first. Thin pass-through."""
        native_specs = [
            _NativeProjectionSpec(
                s.name,
                list(s.roles),
                s.fts,
                s.fts_tokenizer,
                s.vector,
                s.vector_embedder,
                # 0.8.20 Slice 20 (R-20-DR) — the engine-set readiness field is
                # carried ACROSS rather than dropped here, so the binding's
                # round-trip gate sees what the caller actually sent (a readiness
                # with ``vector=False``, or an unknown spelling, is refused).
                # Its VALUE is inert engine-side, which is what keeps
                # ``read.projections`` output re-appliable as a no-op.
                s.vector_dense_readiness,
                _projection_source_segments(s.source),
            )
            for s in specs
        ]
        delta = _native_configure_projections(self._native, native_specs, drop)
        return ProjectionDelta(
            built=list(delta.built),
            dropped=list(delta.dropped),
            deferred=list(delta.deferred),
            unchanged=delta.unchanged,
            # 0.8.20 Slice 22 (R-20-VC / TC-67) — the typed report that replaces
            # the silent drop of a kind the vector writer can never commit.
            vector_unsupported_kinds=list(delta.vector_unsupported_kinds),
        )

    def embed(self, text: str) -> list[float]:
        """Embed ``text`` with the engine's pinned default embedder
        (``fathomdb-bge-small-en-v1.5``) and return the raw vector.

        Read-path primitive for callers that need vectors under the engine's
        own embedder identity (e.g. coverage-index clustering) rather than a
        parallel, possibly-divergent embedder. Raises
        ``EmbedderNotConfiguredError`` if the engine was opened without an
        embedder (``use_default_embedder=False``)."""
        return list(self._native.embed(text))

    def search(
        self,
        query: str,
        filter: SearchFilter | Filter | None = None,
        *,
        rerank_depth: int = 0,
        use_graph_arm: bool = False,
        alpha: float | None = None,
        pool_n: int | None = None,
        explain: bool = False,
        view: ReadView | None = None,
        limit: int = 10,
    ) -> SearchResult:
        """Hybrid search with optional CE reranking and optional graph-BFS arm.

        Args:
            query: Free-text search query.
            filter: Optional closed metadata filter (``SearchFilter``).
            rerank_depth: 0 (default) = soft-fallback / identity (no CE).
                N > 0 = rerank the top-N fused hits with the cross-encoder.
                Must be a non-negative integer. Negative values raise
                ``ValueError``.
            use_graph_arm: When ``True``, seed a BFS over temporal fact-edges
                from the top-10 fused hits and fuse reachable nodes as a third
                RRF arm (Slice 30 R3). Default ``False`` → byte-identical to
                the pre-Slice-30 two-arm pipeline.
            alpha: 0.8.5 (EXP-0) CE-blend weight, clamped to ``[0, 1]`` in the
                engine. ``None`` (default) ⇒ 0.3, the C6 factoid-guard default;
                ``1.0`` is the measured Mem0-parity config. Opt-in for the
                agentic-answer/memory path — the default protects naive lookups.
            pool_n: 0.8.5 (EXP-0) reranked-pool size. ``None`` (default) ⇒
                ``rerank_depth`` (preserves today's pool == depth semantics).
            view: 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — optional validity
                view, the same keyword the five read verbs take. ``None``
                (default) is the STRICT view: active-only, non-superseded, and
                valid AT QUERY TIME. ``ReadView(include_out_of_window=True)``
                returns hits whatever their ``[valid_from, valid_until)``
                window; ``ReadView(valid_as_of=t)`` evaluates validity at the
                bound instant ``t``.

                Only the VALIDITY axis is honoured here. The existence flags
                (``include_superseded`` / ``include_inactive``) raise
                ``InvalidArgumentError`` on the search path rather than being
                silently ignored: search hydrates from projection indexes that
                are not version-complete, so they have no truthful answer.
                Use ``read.list`` to enumerate history.

        Returns:
            ``SearchResult`` with RRF-fused (and optionally CE-reranked) hits.
            Each hit carries ``ce_score`` (the CE score for in-pool reranked
            hits, ``None`` otherwise).
        """
        # FIX-3: reject bool and non-int before the negative check.
        # bool is a subclass of int in Python so it passes isinstance(x, int);
        # we reject it explicitly for X1 parity with TypeScript.
        if not isinstance(rerank_depth, int) or isinstance(rerank_depth, bool):
            raise TypeError(
                f"rerank_depth must be a non-negative integer, got {type(rerank_depth).__name__!r}"
            )
        if rerank_depth < 0:
            raise ValueError(f"rerank_depth must be >= 0, got {rerank_depth!r}")
        if not isinstance(use_graph_arm, bool):
            raise TypeError(f"use_graph_arm must be a bool, got {type(use_graph_arm).__name__!r}")
        # 0.8.8 EXP-OBS (Slice 10) — validate `explain` before the native call,
        # mirroring use_graph_arm + the TS `search` guard (cross-SDK parity).
        if not isinstance(explain, bool):
            raise TypeError(f"explain must be a bool, got {type(explain).__name__!r}")
        # 0.8.5 (codex §9 P2-2) — validate the new α/pool_n knobs before the native
        # call, mirroring the rerank_depth guard and the TS `search` validation
        # (cross-SDK parity). bool is rejected explicitly (it is an int/float
        # subclass that PyO3 would otherwise coerce silently).
        if alpha is not None:
            if isinstance(alpha, bool) or not isinstance(alpha, (int, float)):
                raise TypeError(f"alpha must be a finite number, got {type(alpha).__name__!r}")
            if not math.isfinite(alpha):
                raise ValueError(f"alpha must be a finite number, got {alpha!r}")
        if pool_n is not None:
            if not isinstance(pool_n, int) or isinstance(pool_n, bool):
                raise TypeError(
                    f"pool_n must be a non-negative integer, got {type(pool_n).__name__!r}"
                )
            if pool_n < 0:
                raise ValueError(f"pool_n must be >= 0, got {pool_n!r}")
        if not isinstance(view, (ReadView, type(None))):
            raise TypeError(f"view must be a ReadView or None, got {type(view).__name__!r}")
        limit = _validate_ranked_result_limit("limit", limit)
        native_view = _to_native_view(view)
        # 0.8.11 Slice 40 (#17) — accept the unified Filter on the vec0 search
        # path; lower to the SearchFilter sugar (typed-rejects a Json term, D3).
        if isinstance(filter, Filter):
            filter = filter.to_search_filter()
        if filter is None:
            result = self._native.search(
                query,
                rerank_depth=rerank_depth,
                use_graph_arm=use_graph_arm,
                alpha=alpha,
                pool_n=pool_n,
                explain=explain,
                view=native_view,
                limit=limit,
            )
        else:
            result = self._native.search(
                query,
                source_type=filter.source_type,
                kind=filter.kind,
                created_after=filter.created_after,
                status=filter.status,
                attributes=list(filter.attributes),
                rerank_depth=rerank_depth,
                use_graph_arm=use_graph_arm,
                alpha=alpha,
                pool_n=pool_n,
                explain=explain,
                view=native_view,
                limit=limit,
            )
        fallback = result.soft_fallback
        soft = (
            SoftFallback(branch=cast(SoftFallbackBranch, fallback.branch))
            if fallback is not None
            else None
        )
        # 0.8.8 EXP-OBS (Slice 10) — convert the opt-in native explanation sidecar
        # into dataclasses; `None` (default explain=False) stays `None`.
        native_exp = result.explanation
        explanation = (
            _map_candidate_native_explanation(native_exp, result.results)
            if native_exp is not None
            else None
        )
        return SearchResult(
            projection_cursor=result.projection_cursor,
            soft_fallback=soft,
            results=[
                SearchHit(
                    id=IdSpace(space=hit.id.space, value=hit.id.value),
                    kind=hit.kind,
                    body=hit.body,
                    score=hit.score,
                    branch=cast(SoftFallbackBranch, hit.branch),
                    source_id=hit.source_id,
                    ce_score=hit.ce_score,
                )
                for hit in result.results
            ],
            explanation=explanation,
        )

    def freeze_read_context(self, context: ReadContextV1) -> FrozenReadContextV1:
        """Mint a restart-stable context bound to this database's read state."""
        if not isinstance(context, ReadContextV1):
            raise TypeError(f"context must be a ReadContextV1, got {type(context).__name__!r}")
        native = self._native.freeze_read_context(_to_native_read_context(context))
        if native.schema_version != 1 or native.context.schema_version != 1:
            raise FrozenReadError(
                "unsupported_schema_version at /schemaVersion",
                reason="unsupported_schema_version",
                field_path="/schemaVersion",
            )
        resolved_context = ReadContextV1(
            view=ReadView(
                include_superseded=context.view.include_superseded,
                include_inactive=context.view.include_inactive,
                include_out_of_window=context.view.include_out_of_window,
                valid_as_of=native.effective_valid_at,
            ),
            eligibility=context.eligibility,
            schema_version=context.schema_version,
        )
        return FrozenReadContextV1(
            effective_valid_at=native.effective_valid_at,
            context=resolved_context,
            token=native.token,
            schema_version=native.schema_version,
        )

    def trace_dependency(self, request: DependencyTraceRequestV1) -> DependencyTraceResultV1:
        """Trace one reciprocal registered dependency under a frozen context."""
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

    def search_frozen(
        self,
        query: str,
        context: FrozenReadContextV1,
        *,
        rerank_depth: int = 0,
        use_graph_arm: bool = False,
        alpha: float = 0.3,
        pool_n: int = 0,
        explain: bool = False,
        limit: int = 10,
    ) -> SearchResult:
        """Search under an Engine-authenticated frozen context.

        ``explain=True`` returns a finalized non-empty correlation identity.
        """
        if not isinstance(context, FrozenReadContextV1):
            raise TypeError(
                f"context must be a FrozenReadContextV1, got {type(context).__name__!r}"
            )
        native_context = _to_native_frozen_context(context)
        self._native.validate_frozen_read_context(native_context)
        if not isinstance(rerank_depth, int) or isinstance(rerank_depth, bool):
            raise TypeError("rerank_depth must be a non-negative integer")
        if rerank_depth < 0:
            raise InvalidArgumentError(f"rerank_depth must be >= 0, got {rerank_depth!r}")
        if not isinstance(use_graph_arm, bool):
            raise TypeError("use_graph_arm must be a bool")
        if isinstance(alpha, bool) or not isinstance(alpha, (int, float)):
            raise TypeError("alpha must be a finite number")
        if not math.isfinite(alpha):
            raise ValueError(f"alpha must be a finite number, got {alpha!r}")
        if not isinstance(pool_n, int) or isinstance(pool_n, bool):
            raise TypeError("pool_n must be a non-negative integer")
        if pool_n < 0:
            raise InvalidArgumentError(f"pool_n must be >= 0, got {pool_n!r}")
        if not isinstance(explain, bool):
            raise TypeError("explain must be a bool")
        _validate_ranked_result_limit("limit", limit)
        native = self._native.search_frozen(
            query,
            native_context,
            rerank_depth=rerank_depth,
            use_graph_arm=use_graph_arm,
            alpha=alpha,
            pool_n=pool_n,
            explain=explain,
            limit=limit,
        )
        return _map_native_search_result(native)

    def search_with_evidence(
        self,
        request: EvidenceSearchRequestV1,
    ) -> EvidenceSearchResultV1:
        """Attach one evidence reference per frozen hit.

        ``include_explanation=True`` finalizes the nested correlation identity.
        """
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

    def resolve_evidence(self, request: EvidenceResolveRequestV1) -> ResolvedEvidenceV1:
        """Resolve exact source bytes under an equivalent frozen context."""
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

    def resolve_graph_evidence(
        self, request: GraphEvidenceResolveRequestV1
    ) -> ResolvedGraphEvidenceV1:
        """Resolve one exact artifact disclosed by frozen graph expansion."""
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
            raw = json.loads(
                self._native.resolve_graph_evidence(request.evidence_ref, native_context)
            )
        except (TypeError, ValueError):
            _graph_refuse("graph_corrupt", "")
        return _graph_json_resolved_evidence(raw)

    def search_expand_frozen(
        self,
        query: str,
        context: FrozenReadContextV1,
        depth: int,
        *,
        limit: int = 10,
    ) -> SearchExpandResult:
        """Search and expand while enforcing one frozen read context."""
        if not isinstance(context, FrozenReadContextV1):
            raise TypeError(
                f"context must be a FrozenReadContextV1, got {type(context).__name__!r}"
            )
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

    def search_projected_text(
        self,
        query: str,
        name: str,
        filter: SearchFilter | None = None,
        *,
        view: ReadView | None = None,
        limit: int = 10,
    ) -> SearchResult:
        """Search one declared ``searchable`` property-FTS projection.

        The projection ``name`` is the public query key; its nested source path
        is never accepted from a query caller. This path does not body-scan,
        invoke vector search, or fuse scores.
        """
        if not isinstance(filter, (SearchFilter, type(None))):
            raise TypeError(f"filter must be a SearchFilter or None, got {type(filter).__name__!r}")
        if not isinstance(view, (ReadView, type(None))):
            raise TypeError(f"view must be a ReadView or None, got {type(view).__name__!r}")
        limit = _validate_ranked_result_limit("limit", limit)
        kwargs: dict[str, Any] = {"view": _to_native_view(view), "limit": limit}
        if filter is not None:
            kwargs.update(
                source_type=filter.source_type,
                kind=filter.kind,
                created_after=filter.created_after,
                status=filter.status,
                attributes=list(filter.attributes),
            )
        result = self._native.search_projected_text(query, name, **kwargs)
        return SearchResult(
            projection_cursor=result.projection_cursor,
            soft_fallback=None,
            results=[
                SearchHit(
                    id=IdSpace(space=hit.id.space, value=hit.id.value),
                    kind=hit.kind,
                    body=hit.body,
                    score=hit.score,
                    branch=cast(SoftFallbackBranch, hit.branch),
                    source_id=hit.source_id,
                    ce_score=hit.ce_score,
                )
                for hit in result.results
            ],
            explanation=None,
        )

    def search_text_only(
        self, query: str, view: ReadView | None = None, *, limit: int = 10
    ) -> SearchResult:
        """0.8.18 Slice 5 (#5 vector-equivalence probe) — text-only / FTS-only search.

        Does NOT embed the query and NEVER raises
        ``VectorEquivalenceMismatchError``, so it stays serviceable when the engine
        opened in the degraded ``dense_disabled`` state (the D2 "keep FTS servable"
        contract). It does not invoke vector recall, CE reranking, or the graph
        arm. Matching node- and edge-body FTS candidates are deterministically
        body-deduplicated and ranked before ``limit`` is applied. For one
        immutable selection and effective validity time, smaller accepted limits
        are prefixes of larger limits; this does not extend to hybrid search.
        """
        if not isinstance(view, (ReadView, type(None))):
            raise TypeError(f"view must be a ReadView or None, got {type(view).__name__!r}")
        limit = _validate_ranked_result_limit("limit", limit)
        result = self._native.search_text_only(query, view=_to_native_view(view), limit=limit)
        fallback = result.soft_fallback
        soft = (
            SoftFallback(branch=cast(SoftFallbackBranch, fallback.branch))
            if fallback is not None
            else None
        )
        return SearchResult(
            projection_cursor=result.projection_cursor,
            soft_fallback=soft,
            results=[
                SearchHit(
                    id=IdSpace(space=hit.id.space, value=hit.id.value),
                    kind=hit.kind,
                    body=hit.body,
                    score=hit.score,
                    branch=cast(SoftFallbackBranch, hit.branch),
                    source_id=hit.source_id,
                    ce_score=hit.ce_score,
                )
                for hit in result.results
            ],
            explanation=None,
        )

    def dense_disabled(self) -> bool:
        """0.8.18 Slice 5 (R-VEQ-6) — ``True`` iff the engine opened degraded.

        The open-time #5 self-check found a vector-equivalence divergence and every
        vector-dependent arm now refuses at query time with
        ``VectorEquivalenceMismatchError``. Mirrors ``OpenReport.dense_disabled``.
        """
        return self._native.dense_disabled()

    def dense_disabled_reason(self) -> str | None:
        """0.8.18 Slice 5 (R-VEQ-6) — reason for the degraded state, or ``None``."""
        return self._native.dense_disabled_reason()

    def vector_equivalence_refusal_count(self) -> int:
        """0.8.18 Slice 5 (R-VEQ-6) — count of query-time dense-arm refusals."""
        return self._native.vector_equivalence_refusal_count()

    def enable_telemetry(self, sink_path: str) -> None:
        """0.8.8 Slice 15 (OPP-9) — enable opt-in local telemetry capture to a
        JSONL ``sink_path``. Off by default; local file only (no egress). Once
        enabled, each ``search`` records a query→result event keyed on the
        stable id, and ``record_feedback`` appends correlated agent labels.
        The query text and ``source_id`` are NEVER written (privacy, ADR §C)."""
        if not isinstance(sink_path, str):
            raise TypeError(f"sink_path must be a str, got {type(sink_path).__name__!r}")
        self._native.enable_telemetry(sink_path)

    def last_telemetry_query_id(self) -> str | None:
        """0.8.8 Slice 15 — the most-recent captured ``query_id`` (for
        ``record_feedback``), or ``None`` when telemetry is off / no query has
        been captured yet."""
        return self._native.last_telemetry_query_id()

    def record_feedback(
        self,
        query_id: str,
        relevant_ids: list[int],
        irrelevant_ids: list[int],
        label_source: str,
    ) -> None:
        """0.8.8 Slice 15 — attach agent relevance labels for a previously
        captured ``query_id``. ``relevant_ids`` / ``irrelevant_ids`` are the
        telemetry ``result_ids`` / ``write_cursor`` keys (the pre-0.8.19
        ``SearchHit.id`` space), NOT the post-C-2 typed ``SearchHit.id``;
        ``label_source`` is the caller-declared label origin (e.g.
        ``"agent:hermes"``). Raises when telemetry is off."""
        if not isinstance(query_id, str):
            raise TypeError(f"query_id must be a str, got {type(query_id).__name__!r}")
        if not isinstance(label_source, str):
            raise TypeError(f"label_source must be a str, got {type(label_source).__name__!r}")
        relevant = _validate_id_list("relevant_ids", relevant_ids)
        irrelevant = _validate_id_list("irrelevant_ids", irrelevant_ids)
        self._native.record_feedback(query_id, relevant, irrelevant, label_source)

    def close(self) -> None:
        self._native.close()

    def drain(self, *, timeout_s: float | int = 0) -> None:
        """Block until in-flight writes drain or `timeout_s` elapses."""

        self._native.drain(timeout_s=float(timeout_s))

    def ingest_with_extractor(
        self,
        cmd: list[str],
        documents: list[dict[str, str]],
    ) -> IngestWithExtractorReceipt:
        """G11 (Slice 15) — BYO-LLM ingest via the fathomdb.extract.v1 protocol.

        ``cmd`` is argv (first element = program, rest = args).
        ``documents`` is a list of dicts with ``source_doc_id`` and ``body`` keys.
        """

        return self._native.ingest_with_extractor(cmd, documents)

    def consolidate_with_provider(
        self,
        cmd: list[str],
        axes: list[dict[str, str]],
    ) -> ConsolidateReceipt:
        """0.8.12 Slice 15 (OPP-2) — consolidation / recency via a BYO-LLM
        harness speaking the ``fathomdb.consolidate.v1`` protocol.

        ``cmd`` is argv (first element = program, rest = args).
        ``axes`` is a list of dicts with ``subject_logical_id`` and ``relation``
        keys; each names one (subject, relation) cluster to consolidate.
        """

        return self._native.consolidate_with_provider(cmd, axes)

    def open_report(self) -> OpenReport:
        """Return the structured open-time report captured at `Engine.open`.

        Shape D (locked HITL 2026-05-24): the report is exposed as an
        engine-attached accessor, not a return-shape change on
        `Engine.open`. Idempotent — repeat calls return the same data;
        the report is a snapshot from open time, not live state.
        """

        return _map_open_report(self._native.open_report())

    def counters(self) -> CounterSnapshot:
        snap = self._native.counters()
        return CounterSnapshot(
            queries=snap.queries,
            writes=snap.writes,
            write_rows=snap.write_rows,
            admin_ops=snap.admin_ops,
            cache_hit=snap.cache_hit,
            cache_miss=snap.cache_miss,
        )

    def set_profiling(self, *, enabled: bool) -> None:
        self._native.set_profiling(enabled)

    def set_slow_threshold_ms(self, *, value: int) -> None:
        self._native.set_slow_threshold_ms(value)

    def attach_logging_subscriber(
        self,
        logger: logging.Logger,
        *,
        heartbeat_interval_ms: int | None = None,
    ) -> None:
        """Bind engine events into the supplied `logging.Logger`.

        Subscriber wiring lands in a later 0.6.x slice; the native call
        accepts the parameters so callers can wire a logger against the
        public surface.
        """

        self._native.attach_logging_subscriber(
            logger,
            heartbeat_interval_ms=heartbeat_interval_ms,
        )


__all__ = ["Engine"]
