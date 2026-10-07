#!/usr/bin/env python3
"""Run Slice 135 S01 pilot queries through an installed Python wheel."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
from time import perf_counter_ns
import zipfile


QUERIES = {
    "text": "brightblue",
    "vector": "boat schedule across water",
    "hybrid": "harbor ferry",
}
SOURCE_ID = "slice135-s01-python"


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def make_corpus(size: int) -> list[dict[str, str]]:
    """Build a fixed two-anchor corpus with stable neutral filler rows."""
    if size not in (32, 256):
        raise ValueError("S01 corpus size must be 32 or 256")
    summaries = [
        ("A", "harbor lantern ferry timetable brightblue"),
        ("B", "orchard apple harvest calendar redgreen"),
    ]
    summaries.extend(
        (f"F{index:04}", f"neutral archive ledger inventory entry {index} papers")
        for index in range(size - 2)
    )
    return [
        {
            "kind": "doc",
            "body": json.dumps({"summary": summary}),
            "logical_id": logical_id,
            "source_id": SOURCE_ID,
        }
        for logical_id, summary in summaries
    ]


def assert_result(kind: str, result: object, eligible_ids: set[str]) -> None:
    """Check basic result validity without making a gold relevance claim."""
    hits = result.results  # type: ignore[attr-defined]
    _require(bool(hits), "empty result")
    _require(
        all(hit.id.space == "logical" and hit.id.value in eligible_ids for hit in hits),
        "unknown id or id space",
    )
    if kind == "text":
        _require(
            len(hits) == 1 and hits[0].id.value == "A" and hits[0].branch == "text",
            "text result is not the seeded exact match",
        )
    elif kind == "vector":
        _require(any(hit.branch == "vector" for hit in hits), "vector branch absent")
    elif kind == "hybrid":
        _require(
            any(hit.id.value == "A" for hit in hits),
            "hybrid result lost lexical anchor",
        )
        _require(
            all(hit.branch in ("text", "vector") for hit in hits),
            "hybrid result has an unexpected branch",
        )
    else:
        raise ValueError(f"unknown query kind {kind}")


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _installed_wheel_identity(wheel: Path, expected_sha256: str) -> dict[str, str]:
    import fathomdb
    import fathomdb._fathomdb as native

    wheel_bytes = wheel.read_bytes()
    actual_sha256 = _sha256(wheel_bytes)
    if actual_sha256 != expected_sha256:
        raise ValueError("wheel SHA-256 mismatch")
    if fathomdb.__file__ is None or native.__file__ is None:
        raise ValueError("installed package or native module has no file path")
    package_path = Path(fathomdb.__file__).resolve()
    native_path = Path(native.__file__).resolve()
    prefix = Path(sys.prefix).resolve()
    if not package_path.is_relative_to(prefix) or not native_path.is_relative_to(
        prefix
    ):
        raise ValueError("fathomdb import did not resolve inside the selected venv")
    with zipfile.ZipFile(wheel) as archive:
        for archive_name, installed_path in (
            ("fathomdb/__init__.py", package_path),
            ("fathomdb/_fathomdb.abi3.so", native_path),
        ):
            if _sha256(archive.read(archive_name)) != _sha256(
                installed_path.read_bytes()
            ):
                raise ValueError(f"installed file differs from wheel: {archive_name}")
    return {
        "wheel_sha256": actual_sha256,
        "module_path": str(package_path),
        "native_path": str(native_path),
        "native_sha256": _sha256(native_path.read_bytes()),
        "python_executable": str(Path(sys.executable).resolve()),
    }


def _query(engine: object, kind: str) -> object:
    if kind == "text":
        return engine.search_text_only(QUERIES[kind])  # type: ignore[attr-defined]
    return engine.search(QUERIES[kind])  # type: ignore[attr-defined]


def _timed_query(
    engine: object, kind: str, eligible_ids: set[str]
) -> dict[str, object]:
    start = perf_counter_ns()
    result = _query(engine, kind)
    elapsed = perf_counter_ns() - start
    attempt: dict[str, object] = {
        "latency_ns": elapsed,
        "ids": [hit.id.value for hit in result.results],  # type: ignore[attr-defined]
        "branches": [hit.branch for hit in result.results],  # type: ignore[attr-defined]
    }
    try:
        assert_result(kind, result, eligible_ids)
    except AssertionError as error:
        attempt["semantic_ok"] = False
        attempt["reason"] = str(error)
    else:
        attempt["semantic_ok"] = True
    return attempt


def run_pilot(
    *, size: int, samples: int, wheel: Path, wheel_sha256: str, source_sha: str
) -> dict[str, object]:
    """Emit functional and raw session-cold/warm query observations."""
    if samples < 10:
        raise ValueError("pilot requires at least ten warm samples per cell")
    if len(source_sha) != 40 or any(
        char not in "0123456789abcdef" for char in source_sha
    ):
        raise ValueError("source SHA must be forty lowercase hex digits")
    if len(wheel_sha256) != 64 or any(
        char not in "0123456789abcdef" for char in wheel_sha256
    ):
        raise ValueError("wheel SHA-256 must be sixty-four lowercase hex digits")

    import fathomdb

    identity = _installed_wheel_identity(wheel, wheel_sha256)
    corpus = make_corpus(size)
    eligible_ids = {row["logical_id"] for row in corpus}
    corpus_bytes = json.dumps(corpus, sort_keys=True, separators=(",", ":")).encode()
    output: dict[str, object] = {
        "schema_version": 1,
        "status": "BASELINE_PILOT_OR_CANDIDATE_EXPLORATION_NOT_FROZEN_COMPARISON",
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "source_sha": source_sha,
        "artifact": identity,
        "corpus_size": size,
        "corpus_sha256": _sha256(corpus_bytes),
        "model": "fathomdb-bge-small-en-v1.5 default embedder",
        "queries": QUERIES,
        "warm_samples_per_cell": samples,
        "cells": {},
    }
    with TemporaryDirectory(prefix="slice135-python-s01-") as temporary:
        database = str(Path(temporary) / "s01.sqlite")
        engine = fathomdb.Engine.open(database, use_default_embedder=True)
        try:
            report = engine.open_report()
            _require(
                report.default_embedder.name == "fathomdb-bge-small-en-v1.5",
                "default embedder identity changed",
            )
            engine.write(corpus)
            engine.configure_projections(
                [
                    fathomdb.ProjectionSpec(
                        name="summary",
                        roles=frozenset({fathomdb.ProjectionRole.SEARCHABLE}),
                        vector=True,
                    )
                ]
            )
            engine.drain(timeout_s=120)
            readiness = next(
                spec.vector_dense_readiness
                for spec in fathomdb.read.projections(engine)
                if spec.name == "summary"
            )
            _require(readiness == "ready", "vector projection is not ready")
            _require(
                fathomdb.read.get(engine, "A").body == corpus[0]["body"],
                "anchor A changed before query",
            )
            _require(
                fathomdb.read.get(engine, "B").body == corpus[1]["body"],
                "anchor B changed before query",
            )
        finally:
            engine.close()

        for kind in QUERIES:
            engine = fathomdb.Engine.open(database, use_default_embedder=True)
            try:
                if kind == "vector":
                    _require(
                        not engine.search_text_only(QUERIES[kind]).results,
                        "vector query has lexical matches",
                    )
                if kind == "hybrid":
                    _require(
                        bool(engine.search_text_only(QUERIES[kind]).results),
                        "hybrid query has no lexical arm",
                    )
                cold = _timed_query(engine, kind, eligible_ids)
                warmup = _timed_query(engine, kind, eligible_ids)
                warm = [
                    _timed_query(engine, kind, eligible_ids) for _ in range(samples)
                ]
                output["cells"][kind] = {  # type: ignore[index]
                    "session_cold": cold,
                    "warmup": warmup,
                    "warm": warm,
                }
            finally:
                engine.close()
    output["finished_utc"] = datetime.now(timezone.utc).isoformat()
    output["semantic_failures"] = sum(
        not attempt["semantic_ok"]
        for cell in output["cells"].values()  # type: ignore[union-attr]
        for attempt in [cell["session_cold"], cell["warmup"], *cell["warm"]]
    )
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rows", type=int, required=True, choices=(32, 256))
    parser.add_argument("--samples", type=int, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    result = run_pilot(
        size=args.rows,
        samples=args.samples,
        wheel=args.wheel,
        wheel_sha256=args.wheel_sha256,
        source_sha=args.source_sha,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(
        f"S01 installed Python pilot: {args.rows} rows, {args.samples} warm samples/cell, "
        f"{result['semantic_failures']} semantic failures"
    )
    return 0 if result["semantic_failures"] == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
