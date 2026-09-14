"""Source-independent installed-wheel witness for frozen evidence retrieval."""

from __future__ import annotations

import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory

import fathomdb
from fathomdb import _fathomdb


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


def _run_profile() -> None:
    module_path, native_path, editable = _assert_import_provenance()
    source_body = "slice10 canonical wheel evidence bytes"

    with TemporaryDirectory() as root:
        database = Path(root) / "frozen-evidence.sqlite"
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

    Path(os.environ["FATHOMDB_VERIFY_REPORT"]).write_text(
        f"{module_path}\n{native_path}\n{str(editable).lower()}\n"
        "frozen-evidence-profile-v1\n",
        encoding="utf-8",
    )
    print("frozen evidence wheel profile: ok")


if __name__ == "__main__":
    _run_profile()
