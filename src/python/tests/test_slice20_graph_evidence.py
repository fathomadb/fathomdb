from types import SimpleNamespace

import fathomdb
from fathomdb.engine import _map_native_graph_expand_result


def _native_result() -> SimpleNamespace:
    origin = SimpleNamespace(
        seed_logical_id="root",
        target_logical_id="target",
        hop_depth=1,
        terminal_edge_kind="supports",
        terminal_direction="outgoing",
    )
    return SimpleNamespace(
        schema_version=1,
        seeds=[],
        targets=[
            SimpleNamespace(
                logical_id="target",
                kind="claim",
                body="target body",
                write_cursor="2",
                origin=origin,
            )
        ],
        work_units="1",
        complete=True,
        explanation=None,
        evidence=SimpleNamespace(
            schema_version=1,
            entries=[
                SimpleNamespace(
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


def test_graph_expand_request_defaults_to_no_evidence() -> None:
    request = fathomdb.GraphExpandRequestV1(
        seed=fathomdb.GraphSeedExplicitV1(logical_ids=("root",)),
        direction="outgoing",
        edge_kinds=(),
        target_kinds=(),
        context=fathomdb.GraphReadContextCurrentV1(
            context=fathomdb.ReadContextV1(view=fathomdb.ReadView(), eligibility=fathomdb.SearchFilter())
        ),
        max_depth=1,
        result_limit=1,
        max_work_units="1",
        include_explanation=False,
    )
    assert request.include_evidence is False

