import hashlib
import json
from types import SimpleNamespace
from typing import Any, Callable, cast

import fathomdb
import pytest
from fathomdb.engine import _map_native_graph_expand_result


def _native_result() -> SimpleNamespace:
    origin = SimpleNamespace(
        schema_version=1,
        seed_logical_id="root",
        seed_ordinal=0,
        predecessor_logical_id="root",
        target_logical_id="target",
        hop_count=1,
        terminal_edge_kind="supports",
        terminal_direction="outgoing",
    )
    return SimpleNamespace(
        schema_version=1,
        seeds=[
            SimpleNamespace(schema_version=1, logical_id="root", seed_ordinal=0, query_score=None)
        ],
        targets=[
            SimpleNamespace(
                schema_version=1,
                logical_id="target",
                kind="claim",
                body="target body",
                write_cursor="2",
                origin=origin,
            )
        ],
        work_units="1",
        complete=True,
        degradation_codes=[],
        explanation=None,
        evidence=SimpleNamespace(
            schema_version=1,
            entries=[
                SimpleNamespace(
                    schema_version=1,
                    target_index=0,
                    target_artifact_revision_id="target-r1",
                    target_evidence_ref="fdbgev1.target",
                    terminal_edge_artifact_revision_id="edge-r1",
                    terminal_edge_evidence_ref="fdbgev1.edge",
                )
            ],
        ),
    )


def test_graph_evidence_sidecar_maps_positionally() -> None:
    result = _map_native_graph_expand_result(_native_result())
    assert isinstance(result.evidence, fathomdb.GraphEvidenceSidecarV1)
    assert result.evidence.entries[0].target_index == 0
    assert result.evidence.entries[0].target_artifact_revision_id == "target-r1"


@pytest.mark.parametrize(
    ("field", "path"),
    [
        ("target_artifact_revision_id", "/evidence/entries/0/targetArtifactRevisionId"),
        (
            "terminal_edge_artifact_revision_id",
            "/evidence/entries/0/terminalEdgeArtifactRevisionId",
        ),
    ],
)
def test_graph_evidence_sidecar_revisions_use_artifact_revision_grammar(
    field: str, path: str
) -> None:
    native = _native_result()
    setattr(native.evidence.entries[0], field, "_fdb:reserved")
    with pytest.raises(fathomdb.GraphExpansionError) as captured:
        _map_native_graph_expand_result(native)
    assert captured.value.reason == "graph_corrupt"
    assert captured.value.field_path == path


def test_graph_expand_request_defaults_to_no_evidence() -> None:
    request = fathomdb.GraphExpandRequestV1(
        schema_version=1,
        seed=fathomdb.GraphExplicitSeedV1(
            schema_version=1,
            type="explicit",
            logical_ids=(fathomdb.IdSpace(space="logical", value="root"),),
        ),
        direction="outgoing",
        edge_kinds=(),
        target_kinds=(),
        context=fathomdb.CurrentGraphReadContextV1(
            schema_version=1,
            type="current",
            context=fathomdb.ReadContextV1(
                view=fathomdb.ReadView(), eligibility=fathomdb.SearchFilter()
            ),
        ),
        max_depth=1,
        result_limit=1,
        max_work_units="1",
        include_explanation=False,
    )
    assert request.include_evidence is False


def test_real_engine_resolves_exact_target_and_terminal_edge(db_path: str) -> None:
    source_body = "canonical graph evidence bytes"
    digest = hashlib.sha256(source_body.encode()).hexdigest()

    def derived(revision: str) -> dict[str, object]:
        return {
            "schema_version": 1,
            "role": "derived",
            "artifact_revision_id": revision,
            "source_version_id": "source-v1",
            "source_revision_id": "source-r1",
            "source_locator": {"kind": "whole_body"},
            "canonical_source_hash": {"algorithm": "sha256", "digest_hex": digest},
        }

    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "document",
                    "body": source_body,
                    "source_id": "owner",
                    "logical_id": "source",
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": "source-r1",
                        "source_version_id": "source-v1",
                    },
                },
                {
                    "kind": "claim",
                    "body": "root",
                    "source_id": "owner",
                    "logical_id": "root",
                    "provenance": derived("root-r1"),
                },
                {
                    "kind": "claim",
                    "body": "target",
                    "source_id": "owner",
                    "logical_id": "target",
                    "provenance": derived("target-r1"),
                },
                {
                    "edge": {
                        "kind": "supports",
                        "from": "root",
                        "to": "target",
                        "source_id": "owner",
                        "logical_id": "winner",
                        "provenance": derived("edge-r1"),
                    }
                },
            ]
        )
        engine.drain(timeout_s=30)
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        result = fathomdb.graph.expand(
            engine,
            fathomdb.GraphExpandRequestV1(
                schema_version=1,
                seed=fathomdb.GraphExplicitSeedV1(
                    schema_version=1,
                    type="explicit",
                    logical_ids=(fathomdb.IdSpace(space="logical", value="root"),),
                ),
                direction="outgoing",
                edge_kinds=("supports",),
                target_kinds=("claim",),
                context=fathomdb.FrozenGraphReadContextV1(
                    schema_version=1, type="frozen", context=frozen
                ),
                max_depth=1,
                result_limit=1,
                max_work_units="10",
                include_explanation=False,
                include_evidence=True,
            ),
        )
        assert result.evidence is not None
        entry = result.evidence.entries[0]
        target = engine.resolve_graph_evidence(
            fathomdb.GraphEvidenceResolveRequestV1(
                evidence_ref=entry.target_evidence_ref, context=frozen
            )
        )
        edge = engine.resolve_graph_evidence(
            fathomdb.GraphEvidenceResolveRequestV1(
                evidence_ref=entry.terminal_edge_evidence_ref, context=frozen
            )
        )
        assert (target.artifact.artifact_class, target.artifact.logical_id) == (
            "node",
            "target",
        )
        assert (edge.artifact.artifact_class, edge.artifact.from_id, edge.artifact.to_id) == (
            "edge",
            "root",
            "target",
        )
        assert target.canonical_source_body == source_body
    finally:
        engine.close()


def _resolved_payload() -> dict[str, Any]:
    return {
        "schemaVersion": 1,
        "artifactRevisionId": "target-r1",
        "artifact": {
            "artifactClass": "node",
            "logicalId": "target",
            "kind": "claim",
            "body": "target",
        },
        "sourceId": "owner",
        "sourceVersionId": "source-v1",
        "sourceRevisionId": "source-r1",
        "locator": {"kind": "whole_body", "startInclusive": None, "endExclusive": None},
        "canonicalSourceBody": "source",
        "evidenceText": "source",
        "canonicalSourceHash": {"algorithm": "sha256", "digestHex": "0" * 64},
        "effectiveValidAt": 1_700_000_000,
        "artifactLifecycle": {
            "kind": "node",
            "state": "active",
            "superseded": False,
            "validAtEffective": None,
        },
        "sourceLifecycleState": "active",
        "dependency": None,
    }


@pytest.mark.parametrize(
    ("mutate", "path"),
    [
        (lambda value: value.update({"z/future~field": True}), "/z~1future~0field"),
        (
            lambda value: value["artifact"].update({"from": "root"}),
            "/artifact/from",
        ),
        (lambda value: value.update({"artifactRevisionId": ""}), "/artifactRevisionId"),
        (
            lambda value: value.update(
                {"locator": {"kind": "utf8_bytes", "startInclusive": 0, "endExclusive": "1"}}
            ),
            "/locator/startInclusive",
        ),
        (
            lambda value: value["artifactLifecycle"].update({"validAtEffective": True}),
            "/artifactLifecycle/validAtEffective",
        ),
        (
            lambda value: value.update(
                {
                    "dependency": {
                        "schemaVersion": 1,
                        "dependencyId": "dep-1",
                        "sourceRevisionId": "source-r1",
                        "derivedRevisionId": "target-r1",
                        "registeredDependencyGeneration": "1",
                        "future": True,
                    }
                }
            ),
            "/dependency/future",
        ),
        (
            lambda value: value.update(
                {
                    "dependency": {
                        "schemaVersion": 1,
                        "dependencyId": "dep-1",
                        "sourceRevisionId": "other-source-r1",
                        "derivedRevisionId": "target-r1",
                        "registeredDependencyGeneration": "1",
                    }
                }
            ),
            "/dependency/sourceRevisionId",
        ),
        (
            lambda value: value.update(
                {
                    "dependency": {
                        "schemaVersion": 1,
                        "dependencyId": "dep-1",
                        "sourceRevisionId": "source-r1",
                        "derivedRevisionId": "other-target-r1",
                        "registeredDependencyGeneration": "1",
                    }
                }
            ),
            "/dependency/derivedRevisionId",
        ),
    ],
)
def test_resolved_graph_evidence_json_is_recursively_closed_and_coherent(
    mutate: Callable[[dict[str, Any]], None], path: str
) -> None:
    payload = _resolved_payload()
    mutate(payload)

    class Native:
        def resolve_graph_evidence(self, _reference: str, _context: object) -> str:
            return json.dumps(payload)

    engine = fathomdb.Engine(cast(Any, Native()), path="unused", config=fathomdb.EngineConfig())
    frozen = fathomdb.FrozenReadContextV1(
        effective_valid_at=1_700_000_000,
        context=fathomdb.ReadContextV1(),
        token="frozen",
    )
    with pytest.raises(fathomdb.GraphExpansionError) as captured:
        engine.resolve_graph_evidence(
            fathomdb.GraphEvidenceResolveRequestV1(evidence_ref="fdbgev1.ref", context=frozen)
        )
    assert captured.value.reason == "graph_corrupt"
    assert captured.value.field_path == path


@pytest.mark.parametrize("schema_version", [True, False, 1.0, 0.0, "1", 2])
@pytest.mark.parametrize("location", ["root", "dependency"])
def test_resolved_graph_evidence_requires_exact_integer_schema_versions(
    schema_version: object, location: str
) -> None:
    payload = _resolved_payload()
    if location == "root":
        payload["schemaVersion"] = schema_version
        expected_path = "/schemaVersion"
    else:
        payload["dependency"] = {
            "schemaVersion": schema_version,
            "dependencyId": "dep-1",
            "sourceRevisionId": "source-r1",
            "derivedRevisionId": "target-r1",
            "registeredDependencyGeneration": "1",
        }
        expected_path = "/dependency/schemaVersion"

    class Native:
        def resolve_graph_evidence(self, _reference: str, _context: object) -> str:
            return json.dumps(payload)

    engine = fathomdb.Engine(cast(Any, Native()), path="unused", config=fathomdb.EngineConfig())
    frozen = fathomdb.FrozenReadContextV1(
        effective_valid_at=1_700_000_000,
        context=fathomdb.ReadContextV1(),
        token="frozen",
    )
    with pytest.raises(fathomdb.GraphExpansionError) as captured:
        engine.resolve_graph_evidence(
            fathomdb.GraphEvidenceResolveRequestV1(evidence_ref="fdbgev1.ref", context=frozen)
        )
    assert captured.value.reason == "unsupported_schema_version"
    assert captured.value.field_path == expected_path
