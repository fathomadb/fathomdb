#!/usr/bin/env python3
"""Collect installed-Python filter, page and typed-error observations without scoring."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import platform
import sqlite3
import sys
from typing import Any
import zipfile


def sha256(path: Path) -> str:
    """Hash one on-disk input."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def installed_identity(wheel: Path, wheel_hash: str, native_hash: str) -> dict[str, str]:
    """Bind imported SDK bytes to the pinned wheel and native member."""
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


def _serialize_hits(hits: list[Any]) -> list[dict[str, str]]:
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


def _serialize_page_items(items: list[Any]) -> list[dict[str, str]]:
    return [
        {"logical_id": item.logical_id, "kind": item.kind, "body": item.body}
        for item in items
    ]


def _observe_one(engine: Any, request: dict[str, Any]) -> dict[str, Any]:
    import fathomdb
    from fathomdb import read

    try:
        if request["op"] == "search":
            query_filter = fathomdb.SearchFilter(**request["filter"])
            result = engine.search(request["query"], query_filter)
            return {"kind": "success", "hits": _serialize_hits(list(result.results))}
        if request["op"] == "canonical_page":
            frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
            cursor = None
            pages = []
            for _ in range(10):
                page = read.canonical_page(
                    engine,
                    request["kind"],
                    frozen,
                    fathomdb.PageRequestV1(limit=request["limit"], cursor=cursor),
                )
                next_cursor = page.next_cursor
                pages.append({"items": _serialize_page_items(list(page.items)), "has_more": next_cursor is not None})
                if next_cursor is None:
                    return {"kind": "success", "pages": pages}
                if next_cursor == cursor:
                    raise RuntimeError("pagination cursor did not advance")
                cursor = next_cursor
            raise RuntimeError("pagination exceeded ten pages")
        raise ValueError("unsupported matrix operation")
    except Exception as error:
        return {
            "kind": "error",
            "error_type": type(error).__name__,
            "reason": getattr(error, "reason", None),
            "field_path": getattr(error, "field_path", None),
        }


def observe_matrix(engine: Any, requests: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Record every requested product outcome, including unexpected errors."""
    return [{"id": request["id"], "outcome": _observe_one(engine, request)} for request in requests]


def run(
    *, protocol_path: Path, fixture_path: Path, wheel: Path, version: str, output: Path,
) -> Path:
    """Collect before/after-reopen outcomes from one fresh installed-wheel database."""
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
    source_id = protocol["source_id"]
    prefix = protocol["logical_id_prefix"]
    rows = [
        {"kind": row["kind"], "body": row["body"], "logical_id": f"{prefix}{index}", "source_id": source_id}
        for index, row in enumerate(fixture["corpus"])
    ]
    requests = protocol["requests"]
    output.mkdir(parents=True, exist_ok=False)
    database = output / "matrix.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        for row in rows:
            engine.write([row])
        engine.drain(timeout_s=30)
        before = observe_matrix(engine, requests)
    finally:
        engine.close()
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        after = observe_matrix(engine, requests)
    finally:
        engine.close()
    cases = [
        {"id": first["id"], "before": first["outcome"], "after": second["outcome"]}
        for first, second in zip(before, after, strict=True)
    ]
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
        "cases": cases,
    }
    path = output / "raw.json"
    path.write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
    return path


def main() -> None:
    """Parse a pinned installed-wheel collection request."""
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
