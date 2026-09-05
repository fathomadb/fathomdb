"""Measure Slice 35 legacy-search overhead against its pinned parent commit."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import random
import statistics
import subprocess
import sys
import venv
from dataclasses import replace
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib, measurement_classification, scale_02


SCHEMA = "scale-02-slice35.v1"
PROGRAM_TRACK = "SCALE-02"
REPO_ROOT = Path(__file__).resolve().parent.parent
_TOP_KEYS = {
    "schema_version",
    "program_track",
    "release",
    "approval",
    "baseline",
    "candidate",
    "inputs",
    "workload",
    "policy",
    "artifact_root",
    "claim_boundary",
}
_TOP_KEYS_V2 = _TOP_KEYS | {"campaign_order", "prior_receipt"}
_TOP_KEYS_V3 = _TOP_KEYS | {"measurement_plan"}
_RUNTIME_KEYS = {
    "source_commit",
    "wheel",
    "wheel_sha256",
    "python_extension_sha256",
    "fathomdb_bin",
    "fathomdb_bin_sha256",
}


class Slice35ScaleError(ValueError):
    """Raised when the Slice 35 measurement contract or evidence drifts."""


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise Slice35ScaleError(
            f"{label} keys drifted: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _digest(value: object, label: str) -> str:
    if (
        not isinstance(value, str)
        or len(value) != 64
        or any(character not in "0123456789abcdef" for character in value)
    ):
        raise Slice35ScaleError(f"{label} must be a lowercase sha256")
    return value


def _canonical_sha256(value: object) -> str:
    payload = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _path(value: object, label: str) -> Path:
    if not isinstance(value, str) or not value:
        raise Slice35ScaleError(f"{label} must be a non-empty path")
    path = Path(value)
    return (path if path.is_absolute() else REPO_ROOT / path).resolve()


def resolve_config(
    document: object, *, validate_files: bool = True
) -> dict[str, Any]:
    """Strictly validate and resolve the registered comparison contract."""
    version = document.get("schema_version") if isinstance(document, dict) else None
    if version == "scale-02-slice35.v2":
        expected_keys = _TOP_KEYS_V2
    elif version == "scale-02-slice35.v3":
        expected_keys = _TOP_KEYS_V3
    else:
        expected_keys = _TOP_KEYS
    root = _exact(document, "config", expected_keys)
    if (
        root["schema_version"]
        not in {SCHEMA, "scale-02-slice35.v2", "scale-02-slice35.v3"}
        or root["program_track"] != PROGRAM_TRACK
        or root["release"] != "0.8.25"
        or root["claim_boundary"] != "legacy_search_non_regression_only"
    ):
        raise Slice35ScaleError("configuration identity drifted")
    approval = _exact(
        root["approval"], "approval", {"state", "approved_by", "approved_at"}
    )
    if approval["state"] != "approved" or approval["approved_by"] != "HITL":
        raise Slice35ScaleError("execution is not HITL-approved")
    workload = _exact(
        root["workload"],
        "workload",
        {
            "records",
            "repetitions",
            "warmups",
            "steady_queries",
            "query_order_seed",
            "bootstrap_seed",
            "bootstrap_resamples",
        },
    )
    expected_workload = {
        "records": 10_000,
        "repetitions": 5,
        "warmups": 100,
        "steady_queries": 1_000,
        "query_order_seed": "0x5CA1E025350001",
        "bootstrap_seed": "0x5CA1E02535B007",
        "bootstrap_resamples": 2_000,
    }
    if workload != expected_workload:
        raise Slice35ScaleError("workload drifted from the registered matrix")
    policy = _exact(
        root["policy"],
        "policy",
        {"confidence", "max_relative_regression", "metrics"},
    )
    if policy != {
        "confidence": 0.95,
        "max_relative_regression": 0.03,
        "metrics": ["p50", "p95"],
    }:
        raise Slice35ScaleError("policy drifted from the registered boundary")
    inputs = _exact(
        root["inputs"], "inputs", {"base_config", "base_config_sha256"}
    )
    _digest(inputs["base_config_sha256"], "inputs.base_config_sha256")
    for name in ("baseline", "candidate"):
        runtime = _exact(root[name], name, _RUNTIME_KEYS)
        commit = runtime["source_commit"]
        if not isinstance(commit, str) or len(commit) != 40:
            raise Slice35ScaleError(f"{name}.source_commit must be a full commit")
        for key in (
            "wheel_sha256",
            "python_extension_sha256",
            "fathomdb_bin_sha256",
        ):
            _digest(runtime[key], f"{name}.{key}")
        if validate_files:
            for path_key, digest_key in (
                ("wheel", "wheel_sha256"),
                ("fathomdb_bin", "fathomdb_bin_sha256"),
            ):
                path = _path(runtime[path_key], f"{name}.{path_key}")
                if not path.is_file() or _sha256(path) != runtime[digest_key]:
                    raise Slice35ScaleError(f"{name}.{path_key} drifted")
    if root["schema_version"] == "scale-02-slice35.v2":
        if root["campaign_order"] != ["candidate", "baseline"]:
            raise Slice35ScaleError("v2 must reverse the v1 campaign order")
        prior = _exact(
            root["prior_receipt"],
            "prior_receipt",
            {"path", "sha256", "verdict"},
        )
        _digest(prior["sha256"], "prior_receipt.sha256")
        if prior["verdict"] != "advisory_limit_observed":
            raise Slice35ScaleError("v2 must preserve the measured v1 failure")
        if validate_files:
            prior_path = _path(prior["path"], "prior_receipt.path")
            if not prior_path.is_file() or _sha256(prior_path) != prior["sha256"]:
                raise Slice35ScaleError("prior receipt drifted")
    if root["schema_version"] == "scale-02-slice35.v3":
        plan = _exact(
            root["measurement_plan"],
            "measurement_plan",
            {"path", "sha256", "plan_id"},
        )
        _digest(plan["sha256"], "measurement_plan.sha256")
        if not isinstance(plan["plan_id"], str) or not plan["plan_id"]:
            raise Slice35ScaleError("measurement_plan.plan_id must be non-empty")
        if validate_files:
            plan_path = _path(plan["path"], "measurement_plan.path")
            try:
                plan_document = json.loads(plan_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as exc:
                raise Slice35ScaleError("measurement plan is unavailable") from exc
            if (
                _canonical_sha256(plan_document) != plan["sha256"]
                or plan_document.get("plan_id") != plan["plan_id"]
            ):
                raise Slice35ScaleError("measurement plan reference drifted")
    if validate_files:
        base = _path(inputs["base_config"], "inputs.base_config")
        if not base.is_file() or _sha256(base) != inputs["base_config_sha256"]:
            raise Slice35ScaleError("base SCALE-02 configuration drifted")
        artifact_root = _path(root["artifact_root"], "artifact_root")
        if artifact_root.is_relative_to(REPO_ROOT):
            raise Slice35ScaleError("artifact root must remain outside the repository")
    return root


def load_config(path: str | Path, *, validate_files: bool = True) -> dict[str, Any]:
    """Load a Slice 35 comparison configuration."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise Slice35ScaleError("configuration is unavailable") from exc
    return resolve_config(document, validate_files=validate_files)


def paired_relative_upper(
    baseline: Sequence[float],
    candidate: Sequence[float],
    *,
    seed: int,
    resamples: int,
) -> float:
    """Return the one-sided 95% upper bound of paired relative regression."""
    if len(baseline) != len(candidate) or not baseline:
        raise Slice35ScaleError("paired samples must be non-empty and equal length")
    ratios = [(new / old) - 1.0 for old, new in zip(baseline, candidate, strict=True)]
    randomizer = random.Random(seed)
    estimates = []
    for _ in range(resamples):
        sample = [ratios[randomizer.randrange(len(ratios))] for _ in ratios]
        estimates.append(statistics.fmean(sample))
    return scale_02._percentile(estimates, 0.95)


def regression_verdict(upper: Mapping[str, float], maximum: float) -> str:
    """Apply the registered all-metrics non-regression decision."""
    return "pass" if all(value <= maximum for value in upper.values()) else "fail"


def classification_document(
    *,
    run_id: str,
    authority: Mapping[str, Any],
    source_artifacts: list[dict[str, Any]],
    measurement_plan_sha256: str,
    search_call_counts: Mapping[str, int],
) -> dict[str, Any]:
    """Build the native data-plane classification for one completed campaign."""
    witnesses = [
        {
            "id": f"{arm}-calls",
            "call_path_id": f"{arm}-search",
            "component_id": f"{arm}-engine",
            "arm_id": arm,
            "engine_search_state": "executed",
            "call_count": search_call_counts[arm],
            "count_semantics": "exact",
            "evidence_kind": "source_result",
            "source_artifact_ids": ["metrics", "runner", "invocation"],
        }
        for arm in ("baseline", "candidate")
    ]
    document = {
        "schema_version": measurement_classification.SCHEMA_VERSION,
        "classifier_version": measurement_classification.CLASSIFIER_VERSION,
        "classification_id": "",
        "run_id": run_id,
        "outcome": "complete",
        "blocked_reason": None,
        "measurement_plan_id": authority["plan_id"],
        "source_artifacts": source_artifacts,
        "components": authority["components"],
        "call_paths": authority["call_paths"],
        "execution_witnesses": witnesses,
        "metrics": authority["metric_bindings"],
        "metric_exclusions": authority["metric_exclusions"],
        "comparisons": authority["comparisons"],
        "claims": authority["claims"],
        "migration": {
            "kind": "native",
            "manifest_entry_sha256": None,
            "manifest_path": None,
            "measurement_plan_id": authority["plan_id"],
            "measurement_plan_sha256": measurement_plan_sha256,
        },
    }
    document["classification_id"] = measurement_classification.classification_id(
        document
    )
    return document


def _write_run_classification(
    *,
    run_id: str,
    run_dir: Path,
    config_path: Path,
    config: Mapping[str, Any],
    code_git_sha: str,
) -> None:
    plan_ref = config["measurement_plan"]
    plan_path = _path(plan_ref["path"], "measurement_plan.path")
    authority = json.loads(plan_path.read_text(encoding="utf-8"))
    implementation_artifacts = []
    for artifact_id, path in (
        ("runner", "experiments/scale_02_slice35.py"),
        ("invocation", "experiments/scale_02.py"),
    ):
        locator = f"{code_git_sha}:{path}"
        payload = subprocess.run(
            ["git", "show", locator],
            cwd=REPO_ROOT,
            check=True,
            stdout=subprocess.PIPE,
        ).stdout
        implementation_artifacts.append(
            {
                "id": artifact_id,
                "role": "implementation",
                "locator_kind": "git_blob",
                "locator": locator,
                "sha256": hashlib.sha256(payload).hexdigest(),
                "measurement_root_json_pointers": [],
            }
        )
    artifacts = [
        {
            "id": "metrics",
            "role": "metrics_payload",
            "locator_kind": "repository_path",
            "locator": str((run_dir / "metrics.json").relative_to(REPO_ROOT)),
            "sha256": _sha256(run_dir / "metrics.json"),
            "measurement_root_json_pointers": [
                "/upper_95_relative_regression",
                "/verdict",
            ],
        },
        {
            "id": "record",
            "role": "record",
            "locator_kind": "repository_path",
            "locator": str((run_dir / "record.json").relative_to(REPO_ROOT)),
            "sha256": _sha256(run_dir / "record.json"),
            "measurement_root_json_pointers": [],
        },
        {
            "id": "configuration",
            "role": "configuration",
            "locator_kind": "repository_path",
            "locator": str(config_path.relative_to(REPO_ROOT)),
            "sha256": _sha256(config_path),
            "measurement_root_json_pointers": [],
        },
        {
            "id": "plan",
            "role": "configuration",
            "locator_kind": "repository_path",
            "locator": str(plan_path.relative_to(REPO_ROOT)),
            "sha256": _sha256(plan_path),
            "measurement_root_json_pointers": [],
        },
        *implementation_artifacts,
    ]
    metrics_payload = json.loads((run_dir / "metrics.json").read_text(encoding="utf-8"))
    search_call_counts = {
        arm: metrics_payload["treatments"][arm]["measured_search_call_count"]
        for arm in ("baseline", "candidate")
    }
    if any(
        not isinstance(count, int) or isinstance(count, bool) or count <= 0
        for count in search_call_counts.values()
    ):
        raise Slice35ScaleError("observed search call count is invalid")
    document = classification_document(
        run_id=run_id,
        authority=authority,
        source_artifacts=artifacts,
        measurement_plan_sha256=plan_ref["sha256"],
        search_call_counts=search_call_counts,
    )
    measurement_classification.write_classification(
        run_dir,
        document,
        repository_root=REPO_ROOT,
        authority=authority,
    )


def independent_relative_upper(
    baseline: Sequence[float],
    candidate: Sequence[float],
    *,
    seed: int,
    resamples: int,
) -> float:
    """Bootstrap the ratio of treatment means for balanced independent runs."""
    if not baseline or not candidate:
        raise Slice35ScaleError("independent samples must be non-empty")
    randomizer = random.Random(seed)
    estimates = []
    for _ in range(resamples):
        old = statistics.fmean(
            baseline[randomizer.randrange(len(baseline))] for _ in baseline
        )
        new = statistics.fmean(
            candidate[randomizer.randrange(len(candidate))] for _ in candidate
        )
        estimates.append((new / old) - 1.0)
    return scale_02._percentile(estimates, 0.95)


def _runtime_scale_config(config: Mapping[str, Any], treatment: str) -> scale_02.Scale02Config:
    base_path = _path(config["inputs"]["base_config"], "inputs.base_config")
    base = scale_02.load_config(base_path)
    runtime = config[treatment]
    module = importlib.import_module("fathomdb._fathomdb")
    if module.__file__ is None:
        raise Slice35ScaleError(f"{treatment} loaded extension has no file")
    extension = Path(module.__file__).resolve()
    if _sha256(extension) != runtime["python_extension_sha256"]:
        raise Slice35ScaleError(f"{treatment} loaded extension drifted")
    workload = config["workload"]
    return replace(
        base,
        repetitions=workload["repetitions"],
        cold_query_count=0,
        warmup_query_count=workload["warmups"],
        steady_query_count=workload["steady_queries"],
        query_order_seed=workload["query_order_seed"],
        python=Path(sys.executable),
        python_extension=extension,
        python_extension_sha256=runtime["python_extension_sha256"],
        fathomdb_bin=_path(runtime["fathomdb_bin"], f"{treatment}.fathomdb_bin"),
        fathomdb_bin_sha256=runtime["fathomdb_bin_sha256"],
    )


def _worker(config_path: Path, treatment: str, output_root: Path) -> None:
    config = load_config(config_path)
    scale_config = _runtime_scale_config(config, treatment)
    fixture = scale_02.load_fixture(scale_config)
    rows = scale_02.build_rows(
        fixture.documents, config["workload"]["records"], seed=scale_config.growth_seed
    )
    treatment_root = output_root / treatment
    treatment_root.mkdir(parents=True, exist_ok=False)
    repetitions = [
        scale_02._execute_repetition(
            scale_config,
            fixture,
            rows,
            point_root=treatment_root,
            point=config["workload"]["records"],
            repetition=index,
            observe_measured_search_calls=True,
        )
        for index in range(1, config["workload"]["repetitions"] + 1)
    ]
    summary = {
        "schema_version": "scale-02-slice35-treatment.v1",
        "treatment": treatment,
        "source_commit": config[treatment]["source_commit"],
        "repetitions": len(repetitions),
        "errors": sum(item["errors"] for item in repetitions),
        "timeouts": sum(item["timeouts"] for item in repetitions),
        "measured_search_call_count": sum(
            item["measured_search_call_count"] for item in repetitions
        ),
        "p50_by_repetition": [
            scale_02._percentile(item["steady_query_ms"], 0.50)
            for item in repetitions
        ],
        "p95_by_repetition": [
            scale_02._percentile(item["steady_query_ms"], 0.95)
            for item in repetitions
        ],
    }
    (treatment_root / "summary.json").write_text(
        json.dumps(summary, sort_keys=True) + "\n", encoding="utf-8"
    )


def _install_runtime(run_root: Path, treatment: str, wheel: Path) -> Path:
    runtime = run_root / f"{treatment}-runtime"
    venv.EnvBuilder(with_pip=True, clear=False).create(runtime)
    python = runtime / "bin" / "python"
    subprocess.run(
        [str(python), "-m", "pip", "install", "--no-deps", str(wheel)],
        check=True,
    )
    return python


def run(config_path: str | Path) -> dict[str, Any]:
    """Run both exact-source treatments and register a safe aggregate receipt."""
    config_path = Path(config_path).resolve()
    config = load_config(config_path)
    artifact_root = _path(config["artifact_root"], "artifact_root")
    run_stamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    run_root = artifact_root / f"slice35-{run_stamp}-{os.getpid()}"
    run_root.mkdir(parents=True, mode=0o700)
    for treatment in config.get("campaign_order", ["baseline", "candidate"]):
        python = _install_runtime(
            run_root, treatment, _path(config[treatment]["wheel"], f"{treatment}.wheel")
        )
        environment = dict(os.environ)
        environment["PYTHONPATH"] = str(REPO_ROOT)
        subprocess.run(
            [
                str(python),
                "-m",
                "experiments.scale_02_slice35",
                "worker",
                str(config_path),
                treatment,
                str(run_root),
            ],
            cwd=run_root,
            env=environment,
            check=True,
        )
    summaries = {
        name: json.loads((run_root / name / "summary.json").read_text(encoding="utf-8"))
        for name in ("baseline", "candidate")
    }
    analysis_summaries = summaries
    prior_run_id = None
    if config["schema_version"] == "scale-02-slice35.v2":
        prior_record = json.loads(
            _path(config["prior_receipt"]["path"], "prior_receipt.path").read_text(
                encoding="utf-8"
            )
        )
        prior_run_id = prior_record["run_id"]
        prior_treatments = prior_record["metrics"]["treatments"]
        analysis_summaries = {}
        for treatment in ("baseline", "candidate"):
            analysis_summaries[treatment] = {
                **summaries[treatment],
                "repetitions": prior_treatments[treatment]["repetitions"]
                + summaries[treatment]["repetitions"],
                "errors": prior_treatments[treatment]["errors"]
                + summaries[treatment]["errors"],
                "timeouts": prior_treatments[treatment]["timeouts"]
                + summaries[treatment]["timeouts"],
                "p50_by_repetition": prior_treatments[treatment]["p50_by_repetition"]
                + summaries[treatment]["p50_by_repetition"],
                "p95_by_repetition": prior_treatments[treatment]["p95_by_repetition"]
                + summaries[treatment]["p95_by_repetition"],
            }
    seed = int(config["workload"]["bootstrap_seed"], 16)
    upper_fn = (
        independent_relative_upper
        if config["schema_version"] == "scale-02-slice35.v2"
        else paired_relative_upper
    )
    upper = {
        metric: upper_fn(
            analysis_summaries["baseline"][f"{metric}_by_repetition"],
            analysis_summaries["candidate"][f"{metric}_by_repetition"],
            seed=seed + offset,
            resamples=config["workload"]["bootstrap_resamples"],
        )
        for offset, metric in enumerate(("p50", "p95"))
    }
    verdict = regression_verdict(upper, config["policy"]["max_relative_regression"])
    result = {
        "schema_version": "scale-02-slice35-result.v1",
        "program_track": PROGRAM_TRACK,
        "verdict": verdict,
        "upper_95_relative_regression": upper,
        "treatments": analysis_summaries,
        "current_campaign": summaries,
        "prior_run_id": prior_run_id,
        "claim_boundary": config["claim_boundary"],
    }
    result_path = run_root / "result.json"
    result_path.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    code_info = _lib.git_info()
    run_id, record_dir = _lib.write_record(
        "scale-02-slice35",
        ts=datetime.now(UTC),
        config_obj=config,
        metrics=result,
        verdict="complete" if verdict == "pass" else "advisory_limit_observed",
        read=f"Slice 35 legacy search non-regression: {verdict}",
        code={**code_info, "baseline_commit": config["baseline"]["source_commit"]},
        corpus={"source": "SCALE-02 frozen 10k input pack", "datasets": ["tc5-qualified-real-v2"]},
        seeds={"query_order": config["workload"]["query_order_seed"], "bootstrap": config["workload"]["bootstrap_seed"]},
        env=_lib.env_info(key_deps={"fathomdb_baseline": config["baseline"]["source_commit"], "fathomdb_candidate": config["candidate"]["source_commit"]}),
        cost_usd=0.0,
        headline={"verdict": verdict, **upper},
        n=config["workload"]["records"],
        config_path=str(config_path),
        tests=["tests/experiments/test_scale_02_slice35.py"],
        files_changed=[],
        artifacts=[
            {
                "kind": "external_safe_summary",
                "path": "result.json",
                "sha256": _sha256(result_path),
            }
        ],
        review=None,
        open_questions=[] if verdict == "pass" else ["Slice 35 exceeds the registered legacy-search regression bound"],
    )
    if config["schema_version"] == "scale-02-slice35.v3":
        _write_run_classification(
            run_id=run_id,
            run_dir=record_dir,
            config_path=config_path,
            config=config,
            code_git_sha=code_info["git_sha"],
        )
    _lib.regen_index_md()
    return {"run_id": run_id, "result": result, "external_result": str(result_path)}


def main(argv: list[str] | None = None) -> int:
    """Validate, execute, or run one isolated treatment worker."""
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate_parser = subparsers.add_parser("validate")
    validate_parser.add_argument("config", type=Path)
    run_parser = subparsers.add_parser("run")
    run_parser.add_argument("config", type=Path)
    worker_parser = subparsers.add_parser("worker")
    worker_parser.add_argument("config", type=Path)
    worker_parser.add_argument("treatment", choices=("baseline", "candidate"))
    worker_parser.add_argument("output_root", type=Path)
    args = parser.parse_args(argv)
    if args.command == "validate":
        load_config(args.config)
        print("ok")
    elif args.command == "run":
        print(json.dumps(run(args.config), sort_keys=True))
    else:
        _worker(args.config, args.treatment, args.output_root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
