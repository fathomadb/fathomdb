#!/usr/bin/env python3
"""Collect installed-Python graph, evidence, erasure and reopen observations."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import platform
import sqlite3
import sys
from typing import Any, Callable
import zipfile


def sha256(path: Path) -> str:
    """Hash one on-disk input."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def installed_identity(wheel: Path, wheel_hash: str, native_hash: str) -> dict[str, str]:
    """Bind the imported SDK to the pinned wheel and native member bytes."""
    import fathomdb
    import fathomdb._fathomdb as native

    if sha256(wheel) != wheel_hash:
        raise ValueError("wheel SHA-256 mismatch")
    if fathomdb.__file__ is None or native.__file__ is None:
        raise ValueError("installed SDK path unavailable")
    package = Path(fathomdb.__file__).resolve()
    native_path = Path(native.__file__).resolve()
    prefix = Path(sys.prefix).resolve()
    if not package.is_relative_to(prefix) or not native_path.is_relative_to(prefix):
        raise ValueError("SDK import escaped isolated Python environment")
    with zipfile.ZipFile(wheel) as archive:
        for member, installed in (
            ("fathomdb/__init__.py", package),
            ("fathomdb/_fathomdb.abi3.so", native_path),
        ):
            if hashlib.sha256(archive.read(member)).hexdigest() != sha256(installed):
                raise ValueError(f"installed SDK differs from wheel: {member}")
    if sha256(native_path) != native_hash:
        raise ValueError("native module SHA-256 mismatch")
    return {"module_path": str(package), "native_path": str(native_path)}


def _capture(call: Callable[[], Any]) -> Any:
    try:
        return call()
    except Exception as error:
        return {
            "error_type": type(error).__name__,
            "reason": getattr(error, "reason", None),
            "field_path": getattr(error, "field_path", None),
            "message": str(error),
        }


def _nodes(engine: Any, root: str, depth: int) -> list[dict[str, str]]:
    import fathomdb

    return sorted(
        (
            {"logical_id": node.logical_id, "kind": node.kind, "body": node.body}
            for node in fathomdb.graph.neighbors(engine, root, depth=depth, direction="outgoing")
        ),
        key=lambda row: row["logical_id"],
    )


def _hits(hits: list[Any]) -> list[dict[str, str]]:
    return [
        {
            "id_space": hit.id.space,
            "id_value": hit.id.value,
            "kind": hit.kind,
            "body": hit.body,
            "branch": hit.branch,
            "source_id": hit.source_id,
        }
        for hit in hits
    ]


def _evidence(engine: Any, query: str) -> dict[str, Any]:
    import fathomdb

    frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
    result = engine.search_with_evidence(fathomdb.EvidenceSearchRequestV1(
        query=query, context=frozen, include_explanation=False,
    ))
    refs = [
        {"result_index": ref.result_index, "artifact_revision_id": ref.artifact_revision_id}
        for ref in result.evidence
    ]
    resolved = None
    if result.evidence:
        first = engine.resolve_evidence(fathomdb.EvidenceResolveRequestV1(
            evidence_ref=result.evidence[0].evidence_ref, context=frozen,
        ))
        resolved = {
            "canonical_source_body": first.canonical_source_body,
            "evidence_text": first.evidence_text,
            "source_revision_id": first.source_revision_id,
            "representative_arm": first.projection_origin.representative_arm,
        }
    return {
        "evidence_hits": _hits(list(result.search_result.results)),
        "evidence_refs": refs,
        "resolved": resolved,
    }


def observe_before(engine: Any, fixture: dict[str, Any]) -> dict[str, Any]:
    """Record graph neighbors and resolved source evidence before erasure."""
    observed = {
        "graph_depth1": _capture(lambda: _nodes(engine, fixture["graph_root"], 1)),
        "graph_depth2": _capture(lambda: _nodes(engine, fixture["graph_root"], 2)),
    }
    evidence = _capture(lambda: _evidence(engine, fixture["evidence_query"]))
    if isinstance(evidence, dict) and "error_type" not in evidence:
        observed.update(evidence)
    else:
        observed.update({"evidence_hits": evidence, "evidence_refs": evidence, "resolved": evidence})
    return observed


def erase(engine: Any, fixture: dict[str, Any]) -> dict[str, Any]:
    """Record the public erasure report without inferring success from search."""
    def call() -> dict[str, Any]:
        report = engine.erase_source(fixture["erase_source"])
        return {
            "source_ref": report.source_ref,
            "nodes_excised": report.nodes_excised,
            "edges_excised": report.edges_excised,
        }

    return _capture(call)


def observe_after(engine: Any, fixture: dict[str, Any]) -> dict[str, Any]:
    """Record retained graph and removed text branch after erasure or reopen."""
    return {
        "graph_depth2": _capture(lambda: _nodes(engine, fixture["graph_root"], 2)),
        "evidence_hits": _capture(lambda: _hits(list(engine.search(fixture["evidence_query"]).results))),
    }


def run(
    *, protocol_path: Path, fixture_path: Path, wheel: Path, version: str, output: Path,
) -> Path:
    """Run the fixed sequence on a fresh database and retain raw observations."""
    import fathomdb

    protocol_bytes = protocol_path.read_bytes()
    protocol = json.loads(protocol_bytes)
    fixture = json.loads(fixture_path.read_bytes())
    if protocol.get("schema_version") != 1 or version not in protocol.get("versions", {}):
        raise ValueError("protocol schema or version mismatch")
    runner_hash = sha256(Path(__file__).resolve())
    fixture_hash = sha256(fixture_path)
    if protocol.get("runner_sha256") != runner_hash or protocol.get("fixture_sha256") != fixture_hash:
        raise ValueError("runner or fixture SHA-256 mismatch")
    specification = protocol["versions"][version]
    identity = installed_identity(wheel, specification["wheel_sha256"], specification["native_sha256"])
    output.mkdir(parents=True, exist_ok=False)
    database = output / "graph-evidence.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        engine.write(fixture["writes"])
        engine.drain(timeout_s=30)
        before = observe_before(engine, fixture)
        erasure = erase(engine, fixture)
        after_erase = observe_after(engine, fixture)
    finally:
        engine.close()
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        after_reopen = observe_after(engine, fixture)
    finally:
        engine.close()
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
    raw = {
        "schema_version": 1,
        "version": version,
        "source_sha": specification["source_sha"],
        "wheel_sha256": specification["wheel_sha256"],
        "native_sha256": specification["native_sha256"],
        "protocol_sha256": hashlib.sha256(protocol_bytes).hexdigest(),
        "fixture_sha256": fixture_hash,
        "runner_sha256": runner_hash,
        "database_sha256": sha256(database),
        "sqlite_integrity_check": integrity,
        "identity": identity,
        "environment": {
            "python_executable": str(Path(sys.executable).resolve()),
            "python_version": sys.version,
            "platform": platform.platform(),
        },
        "observations": {
            "before": before,
            "erase": erasure,
            "after_erase": after_erase,
            "after_reopen": after_reopen,
        },
    }
    path = output / "raw.json"
    path.write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
    return path


def main() -> None:
    """Parse an installed-wheel request and write one raw version receipt."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--version", choices=("baseline", "candidate"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(run(
        protocol_path=args.protocol,
        fixture_path=args.fixture,
        wheel=args.wheel,
        version=args.version,
        output=args.output,
    ))


if __name__ == "__main__":
    main()
