"""S4 — the durable writer.

Makes a partial run impossible to mistake for a complete one, by owning the
ordering (stage and validate sidecars, materialize the shared record, append
the index last) and the run identity that ordering depends on.

`_lib.write_record` materializes and appends in ONE call with no hook between
them, so the ordering is only achievable by pre-deriving the identity, staging
into the derived directory, then calling `write_record` with a byte-identical
config document and the same `ts` so it recomputes the same id.

Design of record: `dev/design/earp-slice-4-design.md`.
"""

from __future__ import annotations

import copy
import hashlib
import json
import shutil
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Mapping, Sequence

from eval.earp._experiments import lib as _lib
from eval.earp.observed_cost import OBSERVED_COST_NAME, Observation, SCHEMA_VERSION as OBSERVED_COST_SCHEMA_VERSION
from eval.earp.schema import (
    PER_QUERY_SCHEMA_PATH,
    RESULT_SCHEMA_PATH,
    WORKLOAD_MANIFEST_SCHEMA_PATH,
)
from eval.earp.schema.models import (
    SCHEMA_VERSION_PER_QUERY,
    SCHEMA_VERSION_RESULT,
    Blocker,
    BlockerCode,
    RunVerdict,
)
from eval.earp.schema.validate import validate

_RESULT_SCHEMA: dict[str, Any] = json.loads(RESULT_SCHEMA_PATH.read_text(encoding="utf-8"))
_PER_QUERY_SCHEMA: dict[str, Any] = json.loads(
    PER_QUERY_SCHEMA_PATH.read_text(encoding="utf-8")
)
_WORKLOAD_MANIFEST_SCHEMA: dict[str, Any] = json.loads(
    WORKLOAD_MANIFEST_SCHEMA_PATH.read_text(encoding="utf-8")
)

SIDECAR_NAME = "earp.result.v1.json"
PER_QUERY_NAME = "earp.per-query.v1.jsonl"
WORKLOAD_MANIFEST_NAME = "earp.workload-manifest.v1.json"


@dataclass(frozen=True)
class WriteOutcome:
    run_id: str
    run_dir: Path | None = None
    blocker: Blocker | None = None


def _canonical(document: Mapping[str, Any]) -> str:
    return json.dumps(document, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def write_run(
    *,
    experiment: str,
    ts: datetime,
    config_doc: Mapping[str, Any],
    experiments_root: Path,
    verdict: RunVerdict,
    read: str,
    metrics: Mapping[str, Any],
    code: Mapping[str, Any],
    env: Mapping[str, Any],
    corpus: Mapping[str, Any],
    seeds: Mapping[str, Any],
    cost_usd: float,
    per_query: Sequence[Mapping[str, Any]] = (),
    sidecar: Mapping[str, Any] | None = None,
    sidecar_blockers: Sequence[Mapping[str, Any]] = (),
    observed_cost: Mapping[str, Any] | None = None,
    tdd_evidence: Mapping[str, Any] | None = None,
    n: int | None = None,
    headline: Mapping[str, Any] | None = None,
) -> WriteOutcome:
    """Stage, validate, claim, write, materialize, append.

    `config_doc` must be the RAW config document -- the same object S3 hashed.
    Passing a `ResolvedScenario` would hash differently and then fail
    destructively: `record.json` writes, and `yaml.safe_dump` raises on the
    enums afterwards, leaving a half-materialized directory.
    """
    if not isinstance(verdict, RunVerdict):
        raise ValueError(
            f"verdict must be a pinned RunVerdict token, got {verdict!r}; "
            f"Record.verdict is an untyped str, so the vocabulary is enforced here"
        )
    if not isinstance(config_doc, Mapping):
        raise TypeError("config_doc must be the raw config document (a mapping)")
    if ts.tzinfo is None or ts.utcoffset() != timezone.utc.utcoffset(None):
        raise ValueError(
            "ts must be UTC: _lib stamps a literal 'Z' without converting, so a "
            "naive or local timestamp yields a run_id that lies about its timezone"
        )

    # `_resolved_dict` raises on a Mapping that is not a dict, and returns the
    # SAME object rather than a copy -- so copy here, or a later mutation of the
    # caller's document would silently change the directory.
    document = copy.deepcopy(dict(config_doc))
    sha = _lib.config_sha256(document)
    run_id = _lib.make_run_id(experiment, ts, sha)
    root = Path(experiments_root)
    run_dir = root / "runs" / run_id

    # 1-2. Serialize, then validate the PARSED TEXT -- validating the object
    # graph would not tell us the bytes on disk are trustworthy.
    result_doc = (
        dict(sidecar)
        if sidecar is not None
        else _default_sidecar(run_id, sha, verdict, metrics)
    )
    result_doc.setdefault("blockers", list(sidecar_blockers))
    if sidecar_blockers:
        result_doc["blockers"] = list(sidecar_blockers)
    sidecar_text = _canonical(result_doc)
    findings = validate(json.loads(sidecar_text), _RESULT_SCHEMA)
    if findings:
        raise ValueError(f"sidecar does not conform: {[f.message for f in findings][:4]}")

    per_query_lines = [json.dumps(row, sort_keys=True, ensure_ascii=False) for row in per_query]
    for index, line in enumerate(per_query_lines):
        row_findings = validate(json.loads(line), _PER_QUERY_SCHEMA)
        if row_findings:
            raise ValueError(
                f"per-query line {index} does not conform: "
                f"{[f.message for f in row_findings][:4]}"
            )
    per_query_text = "".join(line + "\n" for line in per_query_lines)
    observed_text = (
        _canonical(_normalize_observed_cost(observed_cost, run_id, sha))
        if observed_cost is not None
        else None
    )

    # 3. Claim the directory with one atomic syscall rather than exists-then-write.
    try:
        run_dir.mkdir(parents=True, exist_ok=False)
    except FileExistsError:
        existing = run_dir / SIDECAR_NAME
        prior = existing.read_text(encoding="utf-8") if existing.is_file() else None
        existing_record = run_dir / "record.json"
        prior_record: Mapping[str, Any] = {}
        if existing_record.is_file():
            try:
                decoded = json.loads(existing_record.read_text(encoding="utf-8"))
                if isinstance(decoded, Mapping):
                    prior_record = decoded
            except json.JSONDecodeError:
                pass
        prior_code = prior_record.get("code")
        same_code = isinstance(prior_code, Mapping) and dict(prior_code) == {
            "git_sha": code.get("git_sha"),
            "dirty": code.get("dirty"),
            "branch": code.get("branch"),
            "baseline_commit": code.get("baseline_commit"),
        }
        same_per_query = (run_dir / PER_QUERY_NAME).read_text(encoding="utf-8") == per_query_text if (run_dir / PER_QUERY_NAME).is_file() else not per_query_text
        same_observed = (
            (run_dir / OBSERVED_COST_NAME).read_text(encoding="utf-8") == observed_text
            if observed_text is not None and (run_dir / OBSERVED_COST_NAME).is_file()
            else observed_text is None and not (run_dir / OBSERVED_COST_NAME).exists()
        )
        if prior == sidecar_text and same_code and same_per_query and same_observed and _existing_artifact_graph_is_valid(
            run_dir, prior_record
        ):
            return WriteOutcome(run_id=run_id, run_dir=run_dir)
        # The hashes are EQUAL by construction in a real collision -- the
        # measurements are what differ -- so the comparison is on bytes.
        return WriteOutcome(
            run_id=run_id,
            blocker=Blocker(
                code=BlockerCode.RUN_ID_COLLISION,
                message=(
                    f"run {run_id} already exists with different evidence; run_id is "
                    f"minute-resolution, so advance ts by a minute or change the config. "
                    f"Nothing was written."
                ),
                stage="writer.stage",
                detail={"run_id": run_id, "run_dir": str(run_dir)},
            ),
        )

    # 4. Stage the complete directed artifact graph before record/index
    # materialization.  ``write_record`` will write the same config bytes
    # again, but staging it here lets the manifest bind its exact digest first.
    config_text = _lib._dump_yaml(document)
    (run_dir / SIDECAR_NAME).write_text(sidecar_text, encoding="utf-8")
    (run_dir / PER_QUERY_NAME).write_text(per_query_text, encoding="utf-8")
    (run_dir / "config.resolved.yaml").write_text(config_text, encoding="utf-8")
    if observed_text is not None:
        (run_dir / OBSERVED_COST_NAME).write_text(observed_text, encoding="utf-8")
    manifest = _workload_manifest(
        run_id=run_id,
        config_sha256=sha,
        result_doc=result_doc,
        code=code,
        config_doc=document,
        result_sha256=_sha256_text(sidecar_text),
        config_sha256_bytes=_sha256_text(config_text),
    )
    manifest_text = _canonical(manifest)
    manifest_findings = validate(json.loads(manifest_text), _WORKLOAD_MANIFEST_SCHEMA)
    if manifest_findings:
        shutil.rmtree(run_dir, ignore_errors=True)
        raise ValueError(f"workload manifest does not conform: {[f.message for f in manifest_findings][:4]}")
    (run_dir / WORKLOAD_MANIFEST_NAME).write_text(manifest_text, encoding="utf-8")

    artifacts = [
        _artifact(run_id, SIDECAR_NAME, sidecar_text),
        _artifact(run_id, PER_QUERY_NAME, per_query_text),
        _artifact(run_id, "config.resolved.yaml", config_text),
        _artifact(run_id, WORKLOAD_MANIFEST_NAME, manifest_text),
    ]
    if observed_text is not None:
        artifacts.append(_artifact(run_id, OBSERVED_COST_NAME, observed_text))

    # 5. Materialize + append (one call, index last inside it).  Any failure
    # removes staged files so there is no complete-looking orphan.
    index_path = root / "index.jsonl"
    try:
        _lib.write_record(
            experiment,
            ts=ts,
            config_obj=document,
            metrics=dict(metrics),
            verdict=verdict.value,
            read=read,
            code=dict(code),
            corpus=dict(corpus),
            seeds=dict(seeds),
            env=dict(env),
            cost_usd=cost_usd,
            n=n,
            headline=dict(headline) if headline else None,
            tdd_evidence=dict(tdd_evidence) if tdd_evidence else None,
            artifacts=artifacts,
            base_dir=root,
            index_path=index_path,
        )
    except Exception:
        shutil.rmtree(run_dir, ignore_errors=True)
        raise

    # 6. INDEX.md is GENERATED from index.jsonl, so it regenerates after the
    # append -- and both paths are passed, or the default would overwrite the
    # repo's committed INDEX.md from this tmp index.
    _lib.regen_index_md(index_path=index_path, md_path=root / "INDEX.md")
    return WriteOutcome(run_id=run_id, run_dir=run_dir)


def _normalize_observed_cost(
    observed_cost: Mapping[str, Any], run_id: str, config_sha256: str
) -> dict[str, Any]:
    """Validate caller observations while binding their durable identity here.

    The writer is the only component that knows the collision-safe run ID. A
    caller therefore supplies measurements, while this boundary supplies the
    evidence-family and resolved-config linkage rather than trusting a stale
    or hand-typed value.
    """
    if not isinstance(observed_cost, Mapping):
        raise TypeError("observed_cost must be a mapping")
    if observed_cost.get("schema_version") != OBSERVED_COST_SCHEMA_VERSION:
        raise ValueError("observed-cost.v2 is the only complete observed-cost schema")
    if observed_cost.get("scope") != "one_run_observation":
        raise ValueError("observed_cost must be one_run_observation evidence")
    document = dict(observed_cost)
    arms = document.get("arms")
    if isinstance(arms, Mapping):
        normalized_arms: dict[str, Any] = {}
        for name, arm in arms.items():
            if not isinstance(name, str) or not isinstance(arm, Mapping):
                raise ValueError("observed_cost arms must be named mappings")
            normalized_arms[name] = _normalize_observed_cost(arm, run_id, config_sha256)
        return {
            "schema_version": OBSERVED_COST_SCHEMA_VERSION,
            "scope": "one_run_observation",
            "evidence_family_id": run_id,
            "config_sha256": config_sha256,
            "arms": normalized_arms,
        }
    observation = Observation(
        evidence_family_id=run_id,
        config_sha256=config_sha256,
        phases_ms=_mapping(document, "phases_ms"),
        counts=_mapping(document, "counts"),
        storage=_mapping(document, "storage"),
        query_samples=tuple(document.get("query_samples", ())),
        unavailable=_mapping_or_empty(document, "unavailable"),
        provenance=_mapping_or_empty(document, "provenance"),
    )
    return observation.as_document()


def _mapping(document: Mapping[str, Any], key: str) -> Mapping[str, Any]:
    value = document.get(key)
    if not isinstance(value, Mapping):
        raise ValueError(f"observed_cost {key} must be a mapping")
    return value


def _mapping_or_empty(document: Mapping[str, Any], key: str) -> Mapping[str, Any]:
    value = document.get(key, {})
    if not isinstance(value, Mapping):
        raise ValueError(f"observed_cost {key} must be a mapping")
    return value


def _sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _artifact(run_id: str, name: str, text: str) -> dict[str, str]:
    return {"path": f"runs/{run_id}/{name}", "sha256": _sha256_text(text)}


def _existing_artifact_graph_is_valid(run_dir: Path, record: Mapping[str, Any]) -> bool:
    """Accept idempotence only for a complete, digest-verified prior graph."""
    artifacts = record.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        return False
    prefix = f"runs/{run_dir.name}/"
    for artifact in artifacts:
        if not isinstance(artifact, Mapping):
            return False
        path = artifact.get("path")
        digest = artifact.get("sha256")
        if not isinstance(path, str) or not path.startswith(prefix):
            return False
        name = path[len(prefix):]
        if not name or "/" in name or not isinstance(digest, str):
            return False
        candidate = run_dir / name
        if not candidate.is_file() or hashlib.sha256(candidate.read_bytes()).hexdigest() != digest:
            return False
    return True


def _workload_manifest(
    *,
    run_id: str,
    config_sha256: str,
    result_doc: Mapping[str, Any],
    code: Mapping[str, Any],
    config_doc: Mapping[str, Any],
    result_sha256: str,
    config_sha256_bytes: str,
) -> dict[str, Any]:
    scenario = result_doc.get("scenario")
    if not isinstance(scenario, Mapping):
        raise ValueError("quality result lacks a resolved scenario for workload manifest")
    query_call = scenario.get("query_call")
    if not isinstance(query_call, str) or not query_call:
        raise ValueError("quality result lacks query_call for workload manifest")
    knobs = scenario.get("effective_knobs", {})
    if not isinstance(knobs, Mapping):
        raise ValueError("quality result effective_knobs must be a mapping")
    return {
        "schema_version": "earp.workload-manifest.v1",
        "quality_parent": {
            "run_id": run_id,
            "evidence_family_id": run_id,
            "result_path": SIDECAR_NAME,
            "result_sha256": result_sha256,
            "candidate_sha": str(code.get("git_sha") or ""),
            "clean": not bool(code.get("dirty")),
        },
        "resolved_config": {
            "path": "config.resolved.yaml",
            "sha256": config_sha256_bytes,
            "canonical_json": _lib.canonical_json(dict(config_doc)),
        },
        "workload": {
            "config_sha256": config_sha256,
            "query_call": query_call,
            "effective_knobs": dict(knobs),
            "retrieval_mode": str(scenario.get("retrieval_mode", "fts_only")),
            "max_measurable_k": int(scenario.get("fanout_used", knobs.get("limit", 10))),
            "use_default_embedder": bool(
                isinstance(config_doc.get("scenario"), Mapping)
                and isinstance(config_doc["scenario"].get("engine"), Mapping)
                and config_doc["scenario"]["engine"].get("use_default_embedder")
            ),
            **({"corpus": dict(config_doc["corpus"])} if isinstance(config_doc.get("corpus"), Mapping) else {}),
            **({"gold": dict(config_doc["gold"])} if isinstance(config_doc.get("gold"), Mapping) else {}),
            **({"projections": config_doc["projections"]} if isinstance(config_doc.get("projections"), Mapping) else {}),
            **({"embedder": config_doc["embedder"]} if isinstance(config_doc.get("embedder"), Mapping) else {}),
            "device": {"kind": "cpu"},
        },
        "performance_plan": {
            "treatments": ["fresh_store", "fresh_store_warm_query"],
            "repetitions": 1,
            "warmup_rule": "fresh_store_warm_query performs one unmeasured query after fresh ingest",
            "aggregation_rule": "descriptive_empirical_order_statistics",
            "invalid_result_policy": "typed_cell",
            "command": "fathomdb-performance",
        },
    }


def _default_sidecar(
    run_id: str, sha: str, verdict: RunVerdict, metrics: Mapping[str, Any]
) -> dict[str, Any]:
    """The metrics ride in the sidecar, which is what makes a re-measurement
    byte-different from the prior run and therefore a detectable collision --
    the config hash alone is equal by construction."""
    return {
        "schema_version": SCHEMA_VERSION_RESULT,
        "run_id": run_id,
        "campaign": "diagnostic",
        "verdict": verdict.value,
        "scenario": {"config_sha256": sha, "query_call": "Engine.search_text_only"},
        "metrics": dict(metrics),
        "witnesses": [],
        "blockers": [],
    }


__all__ = ["PER_QUERY_NAME", "SCHEMA_VERSION_PER_QUERY", "SIDECAR_NAME", "WORKLOAD_MANIFEST_NAME", "WriteOutcome", "write_run"]
