"""Source-independent installed-wheel witness for frozen evidence retrieval."""

from __future__ import annotations

import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory

import fathomdb
from fathomdb import _fathomdb
from fathomdb.errors import IncompatibleSchemaVersionError


def _assert_import_provenance() -> tuple[Path, Path, bool]:
    module_path = Path(fathomdb.__file__).resolve()
    native_path = Path(_fathomdb.__file__).resolve()
    distribution = importlib.metadata.distribution("fathomdb")
    direct_url = next(
        (
            Path(distribution.locate_file(item))
            for item in (distribution.files or ())
            if item.name == "direct_url.json"
            and any(part.endswith(".dist-info") for part in item.parts)
        ),
        None,
    )
    editable = False
    if direct_url is not None and direct_url.exists():
        data = json.loads(direct_url.read_text(encoding="utf-8"))
        editable = bool(data.get("dir_info", {}).get("editable"))
    return module_path, native_path, editable


def _seed(engine: fathomdb.Engine, source_body: str) -> None:
    digest = hashlib.sha256(source_body.encode()).hexdigest()
    engine.write(
        [
            {
                "kind": "document",
                "body": source_body,
                "source_id": "slice10-wheel-source",
                "logical_id": "slice10-wheel-source",
                "provenance": {
                    "schema_version": 1,
                    "role": "canonical",
                    "artifact_revision_id": "slice10-wheel-source-r1",
                    "source_version_id": "slice10-wheel-v1",
                },
            },
            {
                "kind": "claim",
                "body": "slice10wheelevidenceneedle",
                "source_id": "slice10-wheel-source",
                "logical_id": "slice10-wheel-claim",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "slice10-wheel-claim-r1",
                    "source_version_id": "slice10-wheel-v1",
                    "source_revision_id": "slice10-wheel-source-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": digest,
                    },
                },
            },
            {
                "kind": "claim",
                "body": "slice20 wheel graph root",
                "source_id": "slice10-wheel-source",
                "logical_id": "slice20-wheel-root",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "slice20-wheel-root-r1",
                    "source_version_id": "slice10-wheel-v1",
                    "source_revision_id": "slice10-wheel-source-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": digest,
                    },
                },
            },
            {
                "kind": "claim",
                "body": "slice50 graph outgoing terminal",
                "source_id": "slice10-wheel-source",
                "logical_id": "slice50-wheel-out2",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "slice50-wheel-out2-r1",
                    "source_version_id": "slice10-wheel-v1",
                    "source_revision_id": "slice10-wheel-source-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": digest,
                    },
                },
            },
            {
                "kind": "claim",
                "body": "slice50 graph incoming first",
                "source_id": "slice10-wheel-source",
                "logical_id": "slice50-wheel-in1",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "slice50-wheel-in1-r1",
                    "source_version_id": "slice10-wheel-v1",
                    "source_revision_id": "slice10-wheel-source-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": digest,
                    },
                },
            },
            {
                "kind": "claim",
                "body": "slice50 graph incoming terminal",
                "source_id": "slice10-wheel-source",
                "logical_id": "slice50-wheel-in2",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": "slice50-wheel-in2-r1",
                    "source_version_id": "slice10-wheel-v1",
                    "source_revision_id": "slice10-wheel-source-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": digest,
                    },
                },
            },
            {
                "edge": {
                    "kind": "supports",
                    "from": "slice20-wheel-root",
                    "to": "slice10-wheel-claim",
                    "source_id": "slice10-wheel-source",
                    "logical_id": "slice20-wheel-edge",
                    "provenance": {
                        "schema_version": 1,
                        "role": "derived",
                        "artifact_revision_id": "slice20-wheel-edge-r1",
                        "source_version_id": "slice10-wheel-v1",
                        "source_revision_id": "slice10-wheel-source-r1",
                        "source_locator": {"kind": "whole_body"},
                        "canonical_source_hash": {
                            "algorithm": "sha256",
                            "digest_hex": digest,
                        },
                    },
                }
            },
            {
                "edge": {
                    "kind": "supports",
                    "from": "slice50-wheel-in1",
                    "to": "slice20-wheel-root",
                    "source_id": "slice10-wheel-source",
                    "logical_id": "slice50-wheel-in-edge1",
                    "provenance": {
                        "schema_version": 1,
                        "role": "derived",
                        "artifact_revision_id": "slice50-wheel-in-edge1-r1",
                        "source_version_id": "slice10-wheel-v1",
                        "source_revision_id": "slice10-wheel-source-r1",
                        "source_locator": {"kind": "whole_body"},
                        "canonical_source_hash": {
                            "algorithm": "sha256",
                            "digest_hex": digest,
                        },
                    },
                }
            },
            {
                "edge": {
                    "kind": "supports",
                    "from": "slice50-wheel-in2",
                    "to": "slice50-wheel-in1",
                    "source_id": "slice10-wheel-source",
                    "logical_id": "slice50-wheel-in-edge2",
                    "provenance": {
                        "schema_version": 1,
                        "role": "derived",
                        "artifact_revision_id": "slice50-wheel-in-edge2-r1",
                        "source_version_id": "slice10-wheel-v1",
                        "source_revision_id": "slice10-wheel-source-r1",
                        "source_locator": {"kind": "whole_body"},
                        "canonical_source_hash": {
                            "algorithm": "sha256",
                            "digest_hex": digest,
                        },
                    },
                }
            },
        ]
    )
    engine.register_source_dependency(
        {
            "schema_version": 1,
            "dependency_id": "slice10-wheel-dependency",
            "source_revision_id": "slice10-wheel-source-r1",
            "derived_revision_id": "slice10-wheel-claim-r1",
        }
    )
    engine.drain(timeout_s=30.0)
    receipt = engine.actuate(
        {
            "schema_version": 1,
            "operation_id": "slice50-wheel-actuated-edge",
            "operations": [
                {
                    "type": "put_derived_edge",
                    "record": {
                        "kind": "supports",
                        "from": "slice10-wheel-claim",
                        "to": "slice50-wheel-out2",
                        "source_id": "slice10-wheel-source",
                        "logical_id": "slice50-wheel-actuated-edge",
                        "provenance": {
                            "schema_version": 1,
                            "role": "derived",
                            "artifact_revision_id": "slice50-wheel-actuated-edge-r1",
                            "source_version_id": "slice10-wheel-v1",
                            "source_revision_id": "slice10-wheel-source-r1",
                            "source_locator": {"kind": "whole_body"},
                            "canonical_source_hash": {
                                "algorithm": "sha256",
                                "digest_hex": digest,
                            },
                        },
                    },
                }
            ],
        }
    )
    assert receipt.outcome in ("committed", "committed_closure_pending")
    assert receipt.affected_revision_ids == ("slice50-wheel-actuated-edge-r1",)
    engine.drain(timeout_s=30.0)


def _graph_evidence_matrix(
    engine: fathomdb.Engine, frozen: fathomdb.FrozenReadContextV1
) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for seed_name in ("explicit", "query"):
        seed = (
            fathomdb.GraphExplicitSeedV1(
                schema_version=1,
                type="explicit",
                logical_ids=(fathomdb.IdSpace(space="logical", value="slice20-wheel-root"),),
            )
            if seed_name == "explicit"
            else fathomdb.GraphQuerySeedV1(
                schema_version=1,
                type="query",
                text="slice20 wheel graph root",
                ranked_limit=1,
            )
        )
        for direction in ("outgoing", "incoming", "both"):
            for depth in (1, 2):
                result = fathomdb.graph.expand(
                    engine,
                    fathomdb.GraphExpandRequestV1(
                        schema_version=1,
                        seed=seed,
                        direction=direction,
                        edge_kinds=("supports",),
                        target_kinds=("claim",),
                        context=fathomdb.FrozenGraphReadContextV1(
                            schema_version=1, type="frozen", context=frozen
                        ),
                        max_depth=depth,
                        result_limit=10,
                        max_work_units="100",
                        include_explanation=False,
                        include_evidence=True,
                    ),
                )
                assert result.evidence is not None and result.evidence.entries
                for entry in result.evidence.entries:
                    target = result.targets[entry.target_index]
                    resolved_target = engine.resolve_graph_evidence(
                        fathomdb.GraphEvidenceResolveRequestV1(
                            evidence_ref=entry.target_evidence_ref, context=frozen
                        )
                    )
                    resolved_edge = engine.resolve_graph_evidence(
                        fathomdb.GraphEvidenceResolveRequestV1(
                            evidence_ref=entry.terminal_edge_evidence_ref, context=frozen
                        )
                    )
                    rows.append(
                        {
                            "seed": seed_name,
                            "direction": direction,
                            "depth": depth,
                            "hop_count": target.origin.hop_count,
                            "target_index": entry.target_index,
                            "target_ref": entry.target_evidence_ref,
                            "terminal_ref": entry.terminal_edge_evidence_ref,
                            "resolved_target_revision": resolved_target.artifact_revision_id,
                            "resolved_edge_revision": resolved_edge.artifact_revision_id,
                            "edge_source": (
                                "actuated"
                                if resolved_edge.artifact_revision_id
                                == "slice50-wheel-actuated-edge-r1"
                                else "ordinary"
                            ),
                            "route_provenance": [
                                target.origin.seed_logical_id,
                                target.origin.predecessor_logical_id,
                                target.origin.target_logical_id,
                                target.origin.terminal_edge_kind,
                                target.origin.terminal_direction,
                            ],
                            "intrinsic_evidence": [
                                entry.target_artifact_revision_id,
                                entry.terminal_edge_artifact_revision_id,
                            ],
                        }
                    )
    required = {
        (seed, direction, depth)
        for seed in ("explicit", "query")
        for direction in ("outgoing", "incoming", "both")
        for depth in (1, 2)
    }
    assert {(row["seed"], row["direction"], row["depth"]) for row in rows} == required
    assert any(row["edge_source"] == "actuated" for row in rows)
    assert all("ranking_contribution" not in row for row in rows)
    return rows


def _directory_snapshot(root: Path) -> list[dict[str, object]]:
    return [
        {
            "name": path.name,
            "size": path.stat().st_size,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }
        for path in sorted(root.iterdir())
        if path.is_file()
    ]


def _assert_schema_33_refusal(root: Path) -> dict[str, object]:
    database = root / "schema-33.sqlite"
    connection = sqlite3.connect(database)
    try:
        connection.execute("PRAGMA user_version = 33")
        connection.execute("CREATE TABLE historical(value TEXT NOT NULL)")
        connection.execute("INSERT INTO historical VALUES ('immutable')")
        connection.commit()
    finally:
        connection.close()
    database.with_name(database.name + ".lock").write_bytes(b"historical-lock-bytes")
    before = _directory_snapshot(root)
    try:
        fathomdb.Engine.open(str(database), use_default_embedder=False)
    except IncompatibleSchemaVersionError as error:
        diagnostic = str(error)
        assert "schema version 33" in diagnostic
        assert "supported version 34" in diagnostic
    else:
        raise AssertionError("installed artifact accepted schema 33")
    after = _directory_snapshot(root)
    assert after == before
    return {
        "database": database.name,
        "expected_schema": 33,
        "supported_schema": 34,
        "outcome": "typed_refusal",
        "before": before,
        "after": after,
    }


def _run_profile() -> None:
    module_path, native_path, editable = _assert_import_provenance()
    source_body = "slice10 canonical wheel evidence bytes"

    with TemporaryDirectory() as root:
        root_path = Path(root)
        schema_33_refusal = _assert_schema_33_refusal(root_path)
        database = root_path / "frozen-evidence.sqlite"
        engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
        try:
            _seed(engine, source_body)
            frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
            ordinary = engine.search("slice10wheelevidenceneedle")
            expanded = engine.search_expand_frozen(
                "slice10wheelevidenceneedle", frozen, depth=0
            )
            plain_frozen = engine.search_frozen(
                "slice10wheelevidenceneedle", frozen, explain=False
            )
            explained_frozen = engine.search_frozen(
                "slice10wheelevidenceneedle", frozen, explain=True
            )
            plain_evidence = engine.search_with_evidence(
                fathomdb.EvidenceSearchRequestV1(
                    query="slice10wheelevidenceneedle",
                    context=frozen,
                    include_explanation=False,
                )
            )
            explained_evidence = engine.search_with_evidence(
                fathomdb.EvidenceSearchRequestV1(
                    query="slice10wheelevidenceneedle",
                    context=frozen,
                    include_explanation=True,
                )
            )

            assert ordinary.results[0].body == "slice10wheelevidenceneedle"
            assert expanded.search_hits[0].body == "slice10wheelevidenceneedle"
            assert plain_frozen.explanation is None
            assert plain_evidence.search_result.explanation is None
            assert explained_frozen.explanation is not None
            assert explained_frozen.explanation.correlation_id.startswith("x")
            assert explained_evidence.search_result.explanation is not None
            assert (
                explained_evidence.search_result.explanation.correlation_id.startswith(
                    "x"
                )
            )
            assert plain_frozen.results == explained_frozen.results
            assert (
                plain_evidence.search_result.results
                == explained_evidence.search_result.results
            )
            assert [
                entry.artifact_revision_id for entry in plain_evidence.evidence
            ] == [entry.artifact_revision_id for entry in explained_evidence.evidence]

            retained_reference = explained_evidence.evidence[0].evidence_ref
            resolved = engine.resolve_evidence(
                fathomdb.EvidenceResolveRequestV1(
                    evidence_ref=retained_reference,
                    context=frozen,
                )
            )
            assert resolved.canonical_source_body == source_body
            assert resolved.dependency is not None
            assert resolved.dependency.dependency_id == "slice10-wheel-dependency"

            graph_result = fathomdb.graph.expand(
                engine,
                fathomdb.GraphExpandRequestV1(
                    schema_version=1,
                    seed=fathomdb.GraphExplicitSeedV1(
                        schema_version=1,
                        type="explicit",
                        logical_ids=(
                            fathomdb.IdSpace(space="logical", value="slice20-wheel-root"),
                        ),
                    ),
                    direction="outgoing",
                    edge_kinds=("supports",),
                    target_kinds=("claim",),
                    context=fathomdb.FrozenGraphReadContextV1(
                        schema_version=1,
                        type="frozen",
                        context=frozen,
                    ),
                    max_depth=1,
                    result_limit=1,
                    max_work_units="10",
                    include_explanation=False,
                    include_evidence=True,
                ),
            )
            assert graph_result.evidence is not None
            graph_entry = graph_result.evidence.entries[0]
            graph_target = engine.resolve_graph_evidence(
                fathomdb.GraphEvidenceResolveRequestV1(
                    evidence_ref=graph_entry.target_evidence_ref,
                    context=frozen,
                )
            )
            graph_edge = engine.resolve_graph_evidence(
                fathomdb.GraphEvidenceResolveRequestV1(
                    evidence_ref=graph_entry.terminal_edge_evidence_ref,
                    context=frozen,
                )
            )
            assert graph_target.artifact.logical_id == "slice10-wheel-claim"
            assert graph_target.canonical_source_body == source_body
            assert graph_edge.artifact.artifact_class == "edge"
            assert graph_edge.artifact.from_id == "slice20-wheel-root"
            assert graph_edge.artifact.to_id == "slice10-wheel-claim"
            matrix_rows = _graph_evidence_matrix(engine, frozen)
        finally:
            engine.close()

        reopened = fathomdb.Engine.open(str(database), use_default_embedder=False)
        try:
            restarted = reopened.search_frozen(
                "slice10wheelevidenceneedle", frozen, explain=True
            )
            assert restarted.explanation is not None
            assert restarted.explanation.correlation_id.startswith("x")
            resolved_after_restart = reopened.resolve_evidence(
                fathomdb.EvidenceResolveRequestV1(
                    evidence_ref=retained_reference,
                    context=frozen,
                )
            )
            assert resolved_after_restart.canonical_source_body == source_body
            dependency = reopened.dependency_for_derived(
                {
                    "schema_version": 1,
                    "derived_revision_id": "slice10-wheel-claim-r1",
                }
            )
            assert dependency is not None
            assert dependency.dependency_id == "slice10-wheel-dependency"
        finally:
            reopened.close()

    matrix_report = os.environ.get("FATHOMDB_SLICE50_EVIDENCE_REPORT")
    if matrix_report is not None:
        Path(matrix_report).write_text(
            json.dumps(
                {
                    "schema_version": "fathomdb.slice50-graph-evidence/v1",
                    "rows": matrix_rows,
                    "schema_33_refusal": schema_33_refusal,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )

    Path(os.environ["FATHOMDB_VERIFY_REPORT"]).write_text(
        f"{module_path}\n{native_path}\n{str(editable).lower()}\n"
        "frozen-evidence-profile-v1\n",
        encoding="utf-8",
    )
    print("frozen evidence wheel profile: ok")


if __name__ == "__main__":
    _run_profile()
