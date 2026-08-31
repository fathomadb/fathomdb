"""Run TEMPORAL-01's deterministic synthetic TRACE validity cell."""

from __future__ import annotations

import argparse
import json
import statistics
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

from experiments import _lib
from experiments.fathomdb_test_setup import PreparedDatabase, prepare_test_database
from fathomdb.types import ReadView


SCHEMA_VERSION = "temporal-01-trace-validity.v1"
PROGRAM_TRACK = "TEMPORAL-01"
CLAIM_BOUNDARY = "synthetic_world_time_validity_only_no_corpus_or_history_claim"
_TOP_LEVEL = {"schema_version", "program_track", "cell_id", "profile", "fixture", "claim_boundary"}
_PROFILE = {"id", "retrieval", "top_k", "embedder", "embed_device", "reranker", "rerank_device"}
_FIXTURE = {"query", "records", "probes"}
_RECORD = {"logical_id", "source_id", "body", "valid_from", "valid_until"}
_UNBOUNDED_RECORD = {"logical_id", "source_id", "body"}
_PROBE = {"valid_as_of", "expected_ids"}


class TemporalTraceValidityError(ValueError):
    """Raised when the synthetic validity contract is malformed or fails."""


@dataclass(frozen=True)
class TraceRecord:
    """One payload-safe source record with an explicit validity window."""

    logical_id: str
    source_id: str
    body: str
    valid_from: int | None
    valid_until: int | None


@dataclass(frozen=True)
class Probe:
    """One pinned validity instant and its exact expected FTS hit set."""

    valid_as_of: int
    expected_ids: tuple[str, ...]


@dataclass(frozen=True)
class Config:
    """The compact TEMPORAL-01 synthetic validity execution contract."""

    cell_id: str
    profile_id: str
    top_k: int
    embedder: str
    embed_device: str
    reranker: str
    rerank_device: str
    query: str
    records: tuple[TraceRecord, ...]
    probes: tuple[Probe, ...]
    resolved: dict[str, object]

    @property
    def program_track(self) -> str:
        """Return the governing PROGRAM track identifier."""
        return PROGRAM_TRACK


def _exact(value: object, label: str, keys: set[str]) -> Mapping[str, object]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise TemporalTraceValidityError(
            f"{label} keys drifted: missing={sorted(keys - actual)}, unknown={sorted(actual - keys)}"
        )
    return value


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise TemporalTraceValidityError(f"{label} must be non-empty text")
    return value


def _integer(value: object, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise TemporalTraceValidityError(f"{label} must be an integer")
    return value


def _sorted_unique_ids(value: object, label: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not value:
        raise TemporalTraceValidityError(f"{label} must be a non-empty list")
    ids = tuple(_text(item, label) for item in value)
    if len(ids) != len(set(ids)) or tuple(sorted(ids)) != ids:
        raise TemporalTraceValidityError(f"{label} must be sorted and unique")
    return ids


def resolve_config(document: object) -> Config:
    """Strictly parse the checked-in synthetic validity configuration."""
    root = _exact(document, "config", _TOP_LEVEL)
    if root["schema_version"] != SCHEMA_VERSION or root["program_track"] != PROGRAM_TRACK:
        raise TemporalTraceValidityError("config identity is invalid")
    if root["claim_boundary"] != CLAIM_BOUNDARY:
        raise TemporalTraceValidityError("config claim boundary is invalid")
    cell_id = _text(root["cell_id"], "cell_id")
    if cell_id != "temporal-01-trace-validity":
        raise TemporalTraceValidityError("cell_id is not the registered TRACE validity cell")
    profile = _exact(root["profile"], "profile", _PROFILE)
    if (
        profile["id"] != "a0_turn_fts"
        or profile["retrieval"] != "fts"
        or profile["top_k"] != 10
        or profile["embedder"] != "none"
        or profile["embed_device"] != "cpu"
        or profile["reranker"] != "none"
        or profile["rerank_device"] != "cpu"
    ):
        raise TemporalTraceValidityError("synthetic validity cell must remain A0 FTS-only CPU")
    fixture = _exact(root["fixture"], "fixture", _FIXTURE)
    query = _text(fixture["query"], "fixture.query")
    raw_records = fixture["records"]
    if not isinstance(raw_records, list) or len(raw_records) != 4:
        raise TemporalTraceValidityError("fixture.records must contain exactly four records")
    records: list[TraceRecord] = []
    ids: set[str] = set()
    bounded = 0
    for position, raw in enumerate(raw_records):
        record_keys = (
            _UNBOUNDED_RECORD
            if isinstance(raw, dict) and set(raw) == _UNBOUNDED_RECORD
            else _RECORD
        )
        item = _exact(raw, f"fixture.records[{position}]", record_keys)
        # The unbounded anchor deliberately omits both bounds, rather than using
        # null placeholders that could conceal an accidental write behaviour.
        if set(item) == _UNBOUNDED_RECORD:
            valid_from = valid_until = None
        else:
            valid_from = _integer(item["valid_from"], "valid_from")
            valid_until = _integer(item["valid_until"], "valid_until")
            if valid_from >= valid_until:
                raise TemporalTraceValidityError("validity window must be half-open and non-empty")
            bounded += 1
        logical_id = _text(item["logical_id"], "logical_id")
        if logical_id in ids:
            raise TemporalTraceValidityError("fixture logical IDs must be unique")
        ids.add(logical_id)
        records.append(
            TraceRecord(logical_id, _text(item["source_id"], "source_id"), _text(item["body"], "body"), valid_from, valid_until)
        )
    if bounded != 3:
        raise TemporalTraceValidityError("fixture must contain three bounded TRACE windows")
    raw_probes = fixture["probes"]
    if not isinstance(raw_probes, list) or len(raw_probes) != 8:
        raise TemporalTraceValidityError("fixture.probes must contain the eight boundary probes")
    probes: list[Probe] = []
    prior_instant: int | None = None
    for position, raw in enumerate(raw_probes):
        item = _exact(raw, f"fixture.probes[{position}]", _PROBE)
        instant = _integer(item["valid_as_of"], "valid_as_of")
        if prior_instant is not None and instant <= prior_instant:
            raise TemporalTraceValidityError("fixture probes must be strictly ordered")
        expected_ids = _sorted_unique_ids(item["expected_ids"], "expected_ids")
        if not set(expected_ids).issubset(ids):
            raise TemporalTraceValidityError("probe expects an unknown fixture record")
        probes.append(Probe(instant, expected_ids))
        prior_instant = instant
    return Config(
        cell_id=cell_id,
        profile_id="a0_turn_fts",
        top_k=10,
        embedder="none",
        embed_device="cpu",
        reranker="none",
        rerank_device="cpu",
        query=query,
        records=tuple(records),
        probes=tuple(probes),
        resolved=dict(root),
    )


def load_config(path: str | Path) -> Config:
    """Load and validate one JSON execution contract."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise TemporalTraceValidityError("trace validity config is unavailable or invalid") from exc
    return resolve_config(document)


def dry_run(path: str | Path, *, artifact_root: Path) -> dict[str, object]:
    """Validate the cell without creating a database or artifact directory."""
    config = load_config(path)
    if artifact_root.exists():
        raise TemporalTraceValidityError("trace validity artifact root must be new")
    return {
        "schema_version": "temporal-01-trace-validity-dry-run.v1",
        "program_track": PROGRAM_TRACK,
        "state": "ready",
        "record_count": len(config.records),
        "probe_count": len(config.probes),
        "new_database_count": 1,
        "device": {"embed": "cpu", "rerank": "cpu", "gpu": None},
    }


def _logical_hit_ids(result: object) -> list[str]:
    hits = getattr(result, "results", None)
    if not isinstance(hits, list):
        raise TemporalTraceValidityError("FTS search returned an invalid result")
    identifiers: list[str] = []
    for hit in hits:
        identifier = getattr(getattr(hit, "id", None), "value", None)
        if identifier is None and isinstance(getattr(hit, "id", None), str):
            identifier = getattr(hit, "id")
        if not isinstance(identifier, str) or not identifier:
            raise TemporalTraceValidityError("FTS hit lacks a logical identifier")
        identifiers.append(identifier)
    if len(identifiers) != len(set(identifiers)):
        raise TemporalTraceValidityError("FTS search returned duplicate logical IDs")
    return identifiers


def evaluate_probes(
    config: Config,
    *,
    search: Callable[[str, ReadView, int], Sequence[str]],
) -> dict[str, object]:
    """Execute exact-set boundary checks through an injected FTS search seam."""
    latencies: list[float] = []
    unexpected = 0
    missing = 0
    for probe in config.probes:
        started = time.perf_counter()
        actual = tuple(sorted(search(config.query, ReadView(valid_as_of=probe.valid_as_of), config.top_k)))
        latencies.append((time.perf_counter() - started) * 1000.0)
        expected = probe.expected_ids
        unexpected += len(set(actual) - set(expected))
        missing += len(set(expected) - set(actual))
        if actual != expected:
            raise TemporalTraceValidityError(
                f"validity mismatch at {probe.valid_as_of}: expected={expected!r}, actual={actual!r}"
            )
    ordered = sorted(latencies)
    return {
        "schema_version": "temporal-01-trace-validity-result.v1",
        "program_track": PROGRAM_TRACK,
        "record_count": len(config.records),
        "probe_count": len(config.probes),
        "exact_probe_count": len(config.probes),
        "unexpected_hit_count": unexpected,
        "missing_expected_hit_count": missing,
        "query_latency_ms": {
            "p50": statistics.median(ordered),
            "p95": ordered[max(0, min(len(ordered) - 1, int(len(ordered) * 0.95) - 1))],
            "max": ordered[-1],
        },
        "claim_boundary": CLAIM_BOUNDARY,
    }


def _rows(config: Config) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for record in config.records:
        row: dict[str, object] = {
            "kind": "temporal_trace",
            "logical_id": record.logical_id,
            "source_id": record.source_id,
            "body": record.body,
        }
        if record.valid_from is not None:
            row["valid_from"] = record.valid_from
            row["valid_until"] = record.valid_until
        rows.append(row)
    return rows


def _doctor_artifacts(prepared: PreparedDatabase) -> list[dict[str, str | None]]:
    return [
        {"path": "external-fathomdb-config.v1", "sha256": _lib._sha256_file(prepared.config_path)},
        {"path": "external-fathomdb-doctor.v1", "sha256": _lib._sha256_file(prepared.doctor_path)},
    ]


def run(
    path: str | Path,
    *,
    artifact_root: Path,
    base_dir: Path = _lib.EXPERIMENTS_DIR,
    fathomdb_bin: str = "fathomdb",
    prepare: Callable[..., PreparedDatabase] = prepare_test_database,
    open_engine: Callable[[str], Any] | None = None,
) -> tuple[str, Path, dict[str, object]]:
    """Create one fresh FTS database, check all validity boundaries, and receipt it."""
    config = load_config(path)
    if artifact_root.resolve().is_relative_to(_lib.REPO_ROOT.resolve()):
        raise TemporalTraceValidityError("trace validity artifacts must remain outside the repository")
    if artifact_root.exists():
        raise TemporalTraceValidityError("trace validity artifact root must be new")
    prepared = prepare(
        artifact_root,
        test_id=config.cell_id,
        embed_device=config.embed_device,
        rerank_device=config.rerank_device,
        embedder=config.embedder,
        warm_cache=False,
        check_reranker=False,
        fathomdb_bin=fathomdb_bin,
    )
    if open_engine is None:
        from fathomdb import Engine

        def default_open_engine(database_path: str) -> Any:
            """Open the FTS-only database without loading an embedder."""
            return Engine.open(database_path, use_default_embedder=False)

        open_engine = default_open_engine
    engine = open_engine(str(prepared.database_path))
    try:
        engine.write(_rows(config))
        engine.drain(timeout_s=30)
        metrics = evaluate_probes(
            config,
            search=lambda query, view, limit: _logical_hit_ids(
                engine.search_text_only(query, view=view, limit=limit)
            ),
        )
    finally:
        engine.close()
    timestamp = datetime.now(UTC).replace(second=0, microsecond=0)
    run_id, run_dir = _lib.write_record(
        "temporal-01-trace-validity",
        ts=timestamp,
        config_obj=config.resolved,
        metrics=metrics,
        verdict="complete",
        read="Synthetic TRACE half-open validity boundaries passed; no corpus or historical-state claim.",
        code=_lib.git_info(),
        corpus={"source": "synthetic_trace", "manifest_sha256": None, "datasets": []},
        seeds={},
        env=_lib.env_info(key_deps={"fathomdb": "0.8.23"}),
        cost_usd=0.0,
        headline={"program_track": PROGRAM_TRACK, "status": "validity_contract_passed"},
        n=len(config.probes),
        config_path="experiments/configs/temporal-01/trace-validity.v1.json",
        artifacts=_doctor_artifacts(prepared),
        base_dir=base_dir,
    )
    _lib.regen_index_md(index_path=base_dir / "index.jsonl", md_path=base_dir / "INDEX.md")
    return run_id, run_dir, metrics


def main(argv: list[str] | None = None) -> int:
    """Provide `validate`, `dry-run`, and live `run` CLI commands."""
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    validate = commands.add_parser("validate")
    validate.add_argument("config", type=Path)
    dry = commands.add_parser("dry-run")
    dry.add_argument("config", type=Path)
    dry.add_argument("artifact_root", type=Path)
    live = commands.add_parser("run")
    live.add_argument("config", type=Path)
    live.add_argument("artifact_root", type=Path)
    live.add_argument("--fathomdb-bin", default="fathomdb")
    live.add_argument("--base-dir", type=Path, default=_lib.EXPERIMENTS_DIR)
    args = parser.parse_args(argv)
    if args.command == "validate":
        print(json.dumps({"state": "valid", "cell_id": load_config(args.config).cell_id}, sort_keys=True))
    elif args.command == "dry-run":
        print(json.dumps(dry_run(args.config, artifact_root=args.artifact_root), sort_keys=True))
    else:
        run_id, _run_dir, metrics = run(
            args.config,
            artifact_root=args.artifact_root,
            base_dir=args.base_dir,
            fathomdb_bin=args.fathomdb_bin,
        )
        print(json.dumps({"run_id": run_id, "metrics": metrics}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
