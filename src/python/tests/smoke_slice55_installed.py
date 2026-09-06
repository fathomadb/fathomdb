"""Installed-candidate Slice 55 smoke over real databases and native errors."""

from __future__ import annotations

import hashlib
import sqlite3
import tempfile
from pathlib import Path

import fathomdb
from fathomdb import _fathomdb


def _seed(engine: fathomdb.Engine, derived_count: int = 1) -> None:
    source_body = "slice55 installed source"
    digest = hashlib.sha256(source_body.encode()).hexdigest()
    writes: list[dict[str, object]] = [
        {
            "kind": "doc",
            "body": source_body,
            "source_id": "slice55-installed",
            "logical_id": "source",
            "provenance": {
                "schema_version": 1,
                "role": "canonical",
                "artifact_revision_id": "source-r1",
                "source_version_id": "v1",
            },
        }
    ]
    for index in range(derived_count):
        writes.append(
            {
                "kind": "fact",
                "body": f"slice55 installed derived {index}",
                "source_id": "slice55-installed",
                "logical_id": f"derived-{index}",
                "provenance": {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": f"derived-r{index}",
                    "source_version_id": "v1",
                    "source_revision_id": "source-r1",
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": digest,
                    },
                },
            }
        )
    engine.write(writes)
    for index in range(derived_count):
        engine.register_source_dependency(
            {
                "schema_version": 1,
                "dependency_id": f"dep-{index}",
                "source_revision_id": "source-r1",
                "derived_revision_id": f"derived-r{index}",
            }
        )


def _request(
    engine: fathomdb.Engine,
    root: str = "source-r1",
    direction: str = "to_dependents",
    *,
    max_relations: int = 100,
    max_work_units: int = 101,
) -> fathomdb.DependencyTraceRequestV1:
    return fathomdb.DependencyTraceRequestV1(
        root_revision_id=root,
        direction=direction,  # type: ignore[arg-type]
        context=engine.freeze_read_context(fathomdb.ReadContextV1()),
        max_relations=max_relations,
        max_work_units=max_work_units,
    )


def _expect_trace_error(
    engine: fathomdb.Engine,
    request: fathomdb.DependencyTraceRequestV1,
    reason: str,
    field_path: str,
) -> None:
    try:
        engine.trace_dependency(request)
    except fathomdb.DependencyTraceError as error:
        assert error.code == "FDB_DEPENDENCY_TRACE"
        assert error.reason == reason
        assert error.field_path == field_path
    else:
        raise AssertionError(f"expected native DependencyTraceError {reason}")


def main() -> None:
    assert fathomdb.DependencyTraceError is _fathomdb.DependencyTraceError
    assert not hasattr(fathomdb.Engine, "check_data_plane_integrity")
    assert not hasattr(fathomdb.Engine, "doctor")

    with tempfile.TemporaryDirectory(prefix="fathomdb-s55-installed-") as directory:
        root = Path(directory)
        success = fathomdb.Engine.open(str(root / "success.fathom"), use_default_embedder=False)
        _seed(success, 2)
        trace = success.trace_dependency(_request(success))
        assert trace.complete and len(trace.dependency_edges) == 2
        assert trace.nodes[0].artifact_revision_id == "source-r1"
        explained = success.search("installed", explain=True)
        assert explained.explanation is not None
        assert explained.explanation.correlation_id
        assert all(item.structural is not None for item in explained.explanation.per_hit)
        _expect_trace_error(
            success,
            _request(success, direction="invalid"),
            "trace_direction_invalid",
            "/direction",
        )
        _expect_trace_error(
            success,
            _request(success, root="absent-r1"),
            "trace_unavailable",
            "/rootRevisionId",
        )
        _expect_trace_error(
            success,
            _request(success, max_relations=1, max_work_units=2),
            "trace_bound_exceeded",
            "",
        )
        success.close()

        corrupt_path = root / "corrupt.fathom"
        corrupt = fathomdb.Engine.open(str(corrupt_path), use_default_embedder=False)
        _seed(corrupt)
        corrupt.close()
        with sqlite3.connect(corrupt_path) as connection:
            connection.execute("PRAGMA ignore_check_constraints=ON")
            connection.execute(
                "UPDATE _fathomdb_source_dependencies SET schema_version=2 "
                "WHERE dependency_id='dep-0'"
            )
        corrupt = fathomdb.Engine.open(str(corrupt_path), use_default_embedder=False)
        _expect_trace_error(
            corrupt,
            _request(corrupt),
            "trace_corrupt",
            "",
        )
        corrupt.close()

    print("slice55 installed native smoke: ok")


if __name__ == "__main__":
    main()
