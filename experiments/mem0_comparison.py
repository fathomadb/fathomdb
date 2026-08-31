"""Close a safe, typed comparison receipt after both LOCOMO arms complete."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from experiments import _lib


EXPERIMENT = "fathomdb-vs-mem0-locomo-comparison"
SCHEMA_VERSION = "mem0-comparison.v1"
RESULT_SCHEMA_VERSION = "mem0-comparison.result.v1"
BOOTSTRAP_SEED = 20260814
BOOTSTRAP_RESAMPLES = 10_000


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _validate_arm(
    record: dict[str, Any], *, expected_experiment: str, expected_program_track: str
) -> dict[str, Any]:
    normalized = _lib.record_from_dict(record)
    arm = _lib.asdict(normalized)
    if arm["schema_version"] != _lib.RECORD_V1:
        raise ValueError("comparison arms must use experiments.record.v1")
    if arm["experiment"] != expected_experiment:
        raise ValueError(f"expected {expected_experiment} arm receipt")
    if arm["config"]["resolved"].get("program_track") != expected_program_track:
        raise ValueError(f"{expected_experiment} arm must declare program_track={expected_program_track}")
    if arm["verdict"] != "complete" or not arm["metrics"].get("completion", {}).get("complete"):
        raise ValueError(f"{expected_experiment} arm must be complete")
    return arm


def _matched_workload(native: dict[str, Any], fathom: dict[str, Any]) -> dict[str, Any]:
    native_config = native["config"]["resolved"]
    fathom_config = fathom["config"]["resolved"]
    native_benchmark = native_config.get("benchmark", {})
    fathom_benchmark = fathom_config.get("benchmark", {})
    for key in (
        "top_k", "top_k_cutoffs", "conversations", "categories", "predict_only",
        "resume",
    ):
        if native_benchmark.get(key) != fathom_benchmark.get(key):
            raise ValueError(f"arm workload mismatch for benchmark.{key}")
    native_corpus = native_config.get("corpus", {})
    fathom_corpus = fathom_config.get("corpus", {})
    for key in ("raw_sha256", "normalized_sha256", "sessions", "eligible_questions"):
        if native_corpus.get(key) != fathom_corpus.get(key):
            raise ValueError(f"arm workload mismatch for corpus.{key}")
    if native_config.get("harness", {}).get("git_sha") != fathom_config.get("harness", {}).get("git_sha"):
        raise ValueError("arm workload mismatch for harness.git_sha")
    return {
        "top_k": native_benchmark["top_k"], "top_k_cutoffs": native_benchmark["top_k_cutoffs"],
        "conversations": native_benchmark["conversations"], "categories": native_benchmark["categories"],
        "resume": native_benchmark["resume"],
        "raw_sha256": native_corpus["raw_sha256"], "normalized_sha256": native_corpus["normalized_sha256"],
        "sessions": native_corpus["sessions"], "eligible_questions": native_corpus["eligible_questions"],
        "harness_git_sha": native_config.get("harness", {}).get("git_sha"),
    }


def _load_scores(
    output_dir: Path, *, expected_count: int, expected_categories: set[int], cutoff: int
) -> tuple[dict[str, tuple[int, float]], dict[str, Any]]:
    if not output_dir.is_dir():
        raise ValueError(f"scored output directory does not exist: {output_dir}")
    scores: dict[str, tuple[int, float]] = {}
    digest_rows: list[list[Any]] = []
    label = f"top_{cutoff}"
    for path in sorted(output_dir.glob("conv*_q*.json")):
        try:
            item = json.loads(path.read_text(encoding="utf-8"))
            question_id = item["question_id"]
            category = item["category"]
            score = item["cutoff_results"][label]["score"]
        except (OSError, KeyError, TypeError, json.JSONDecodeError) as exc:
            raise ValueError(f"invalid scored output: {path.name}") from exc
        if not isinstance(question_id, str) or path.stem != question_id:
            raise ValueError(f"invalid question identity: {path.name}")
        if question_id in scores:
            raise ValueError(f"duplicate scored question: {question_id}")
        if (
            not isinstance(category, int)
            or isinstance(category, bool)
            or category not in expected_categories
        ):
            raise ValueError(f"invalid category for {question_id}")
        if isinstance(score, bool) or not isinstance(score, (int, float)) or float(score) not in {0.0, 1.0}:
            raise ValueError(f"invalid binary score for {question_id}")
        scores[question_id] = (category, float(score))
        digest_rows.append([question_id, category, float(score)])
    if len(scores) != expected_count:
        raise ValueError(f"expected {expected_count} scored questions, found {len(scores)}")
    digest = hashlib.sha256(
        json.dumps(digest_rows, separators=(",", ":"), ensure_ascii=True).encode()
    ).hexdigest()
    return scores, {"scored_files": len(scores), "sha256": digest}


def _accuracy(scores: list[float]) -> float:
    return sum(scores) / len(scores)


def _paired_metrics(
    native: dict[str, tuple[int, float]],
    fathom: dict[str, tuple[int, float]],
    *,
    seed: int,
    resamples: int,
) -> dict[str, Any]:
    if set(native) != set(fathom):
        raise ValueError("scored question identities do not match")
    if resamples < 2:
        raise ValueError("bootstrap resamples must be at least 2")
    native_values: list[float] = []
    fathom_values: list[float] = []
    deltas: list[float] = []
    categories: dict[int, tuple[list[float], list[float]]] = {}
    for question_id in sorted(native):
        native_category, native_score = native[question_id]
        fathom_category, fathom_score = fathom[question_id]
        if native_category != fathom_category:
            raise ValueError(f"category mismatch for {question_id}")
        native_values.append(native_score)
        fathom_values.append(fathom_score)
        deltas.append(fathom_score - native_score)
        native_category_values, fathom_category_values = categories.setdefault(
            native_category, ([], [])
        )
        native_category_values.append(native_score)
        fathom_category_values.append(fathom_score)
    rng = random.Random(seed)
    n = len(deltas)
    bootstrap_means = sorted(
        sum(deltas[rng.randrange(n)] for _ in range(n)) / n
        for _ in range(resamples)
    )
    lower_index = max(0, math.ceil(0.05 * resamples) - 1)

    def summary(left: list[float], right: list[float]) -> dict[str, Any]:
        left_accuracy = _accuracy(left)
        right_accuracy = _accuracy(right)
        return {
            "questions": len(left),
            "mem0_oss_accuracy": left_accuracy,
            "fathomdb_accuracy": right_accuracy,
            "fathomdb_minus_mem0": right_accuracy - left_accuracy,
        }

    return {
        "overall": summary(native_values, fathom_values),
        "by_category": {
            str(category): summary(*categories[category]) for category in sorted(categories)
        },
        "one_sided_95_lower_bound": bootstrap_means[lower_index],
        "bootstrap": {"resamples": resamples, "seed": seed},
    }


def write_receipt(native_record: dict[str, Any], fathom_record: dict[str, Any], *, ts: datetime,
                  base_dir: str | Path) -> tuple[str, Path]:
    """Validate two completed arm records and write a generic comparison receipt."""
    native = _validate_arm(
        native_record,
        expected_experiment="mem0-oss-locomo-native",
        expected_program_track="MEMORY-01",
    )
    fathom = _validate_arm(
        fathom_record,
        expected_experiment="fathomdb-locomo-official-seam",
        expected_program_track="MEMORY-01",
    )
    workload = _matched_workload(native, fathom)
    execution_controls = {
        "mem0_oss": {
            "max_workers": native["config"]["resolved"]["benchmark"]["max_workers"],
            "rpm": native["config"]["resolved"]["benchmark"]["rpm"],
        },
        "fathomdb": {
            "max_workers": fathom["config"]["resolved"]["benchmark"]["max_workers"],
            "rpm": fathom["config"]["resolved"]["benchmark"]["rpm"],
        },
    }
    config = {
        "schema_version": SCHEMA_VERSION, "campaign": "locomo_predict_only",
        "program_track": "MEMORY-01",
        "workload": workload,
        "arms": {"mem0_oss": native["run_id"], "fathomdb": fathom["run_id"]},
        "execution_controls": execution_controls,
    }
    run_id = _lib.make_run_id(EXPERIMENT, ts, _lib.config_sha256(config))
    run_dir = Path(base_dir) / "runs" / run_id
    result = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "run_id": run_id,
        "verdict": "complete",
        "arms": config["arms"],
        "workload": workload,
        "execution_controls": execution_controls,
    }
    result_path = run_dir / "mem0-comparison.result.v1.json"
    run_dir.mkdir(parents=True, exist_ok=True)
    result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    _lib.write_record(
        EXPERIMENT, ts=ts, config_obj=config, metrics={"phase": "retrieval_only", "arms": config["arms"]},
        verdict="complete", read="matched LOCOMO predict-only arm receipts are complete", code=_lib.git_info(),
        corpus={"source": "LOCOMO", "manifest_sha256": workload["raw_sha256"], "datasets": []}, seeds={},
        env=_lib.env_info(), cost_usd=None, headline={"phase": "retrieval_only"}, artifacts=[{
            "path": str(result_path.relative_to(Path(base_dir))), "sha256": _sha256(result_path),
        }], base_dir=base_dir,
    )
    _lib.regen_index_md(index_path=Path(base_dir) / "index.jsonl", md_path=Path(base_dir) / "INDEX.md")
    return run_id, run_dir


def write_scored_receipt(
    native_record: dict[str, Any],
    fathom_record: dict[str, Any],
    *,
    native_output_dir: str | Path,
    fathom_output_dir: str | Path,
    ts: datetime,
    base_dir: str | Path,
    bootstrap_seed: int = BOOTSTRAP_SEED,
    bootstrap_resamples: int = BOOTSTRAP_RESAMPLES,
) -> tuple[str, Path]:
    """Write a content-free paired answer-quality decision receipt."""
    native = _validate_arm(
        native_record,
        expected_experiment="mem0-oss-locomo-native",
        expected_program_track="MEMORY-01",
    )
    fathom = _validate_arm(
        fathom_record,
        expected_experiment="fathomdb-locomo-official-seam",
        expected_program_track="MEMORY-01",
    )
    workload = _matched_workload(native, fathom)
    cutoffs = workload["top_k_cutoffs"]
    if cutoffs != [10]:
        raise ValueError("scored comparison requires the registered top-10 cutoff")
    expected_count = workload["eligible_questions"]
    try:
        expected_categories = {int(item) for item in workload["categories"].split(",")}
    except (AttributeError, ValueError) as exc:
        raise ValueError("registered categories must be comma-separated integers") from exc
    if not expected_categories or not expected_categories <= {1, 2, 3, 4}:
        raise ValueError("registered categories must be a non-empty subset of 1-4")
    native_scores, native_manifest = _load_scores(
        Path(native_output_dir),
        expected_count=expected_count,
        expected_categories=expected_categories,
        cutoff=10,
    )
    fathom_scores, fathom_manifest = _load_scores(
        Path(fathom_output_dir),
        expected_count=expected_count,
        expected_categories=expected_categories,
        cutoff=10,
    )
    paired = _paired_metrics(
        native_scores,
        fathom_scores,
        seed=bootstrap_seed,
        resamples=bootstrap_resamples,
    )
    verdict = "pass" if paired["one_sided_95_lower_bound"] >= 0.0 else "fail"
    arms = {"mem0_oss": native["run_id"], "fathomdb": fathom["run_id"]}
    config = {
        "schema_version": SCHEMA_VERSION,
        "campaign": "locomo_paired_answer_scoring",
        "program_track": "MEMORY-01",
        "workload": workload,
        "arms": arms,
        "scoring": {
            "answerer_model": "gpt-4o-mini",
            "judge_model": "gpt-4o-mini",
            "provider": "openai_compatible_airlock",
            "provider_route": "openrouter_exact_models",
            "evidence_aware": True,
            "cutoff": 10,
            "max_workers": 2,
            "rpm_per_role": 20,
            "bootstrap_seed": bootstrap_seed,
            "bootstrap_resamples": bootstrap_resamples,
            "decision_rule": "one_sided_95_lower_bound_fathomdb_minus_mem0_gte_zero",
        },
    }
    run_id = _lib.make_run_id(EXPERIMENT, ts, _lib.config_sha256(config))
    run_dir = Path(base_dir) / "runs" / run_id
    result = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "run_id": run_id,
        "verdict": verdict,
        "arms": arms,
        "workload": workload,
        "scoring": config["scoring"],
        "paired": paired,
        "score_manifests": {"mem0_oss": native_manifest, "fathomdb": fathom_manifest},
    }
    run_dir.mkdir(parents=True, exist_ok=True)
    result_path = run_dir / "mem0-comparison.result.v1.json"
    result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    _lib.write_record(
        EXPERIMENT,
        ts=ts,
        config_obj=config,
        metrics={
            "phase": "paired_answer_scoring",
            "arms": arms,
            "paired": paired,
            "score_manifests": result["score_manifests"],
        },
        verdict=verdict,
        read=(
            "FathomDB meets the registered near-parity rule"
            if verdict == "pass"
            else "FathomDB does not meet the registered near-parity rule"
        ),
        code=_lib.git_info(),
        corpus={"source": "LOCOMO", "manifest_sha256": workload["raw_sha256"], "datasets": []},
        seeds={"paired_bootstrap": str(bootstrap_seed)},
        env=_lib.env_info(),
        cost_usd=None,
        headline={
            "fathomdb_minus_mem0": paired["overall"]["fathomdb_minus_mem0"],
            "one_sided_95_lower_bound": paired["one_sided_95_lower_bound"],
        },
        artifacts=[{
            "path": str(result_path.relative_to(Path(base_dir))),
            "sha256": _sha256(result_path),
        }],
        base_dir=base_dir,
    )
    _lib.regen_index_md(index_path=Path(base_dir) / "index.jsonl", md_path=Path(base_dir) / "INDEX.md")
    return run_id, run_dir


def main(argv: list[str] | None = None) -> int:
    """Close a comparator receipt from two existing generic arm receipts."""
    parser = argparse.ArgumentParser(description="Mem0/FathomDB LOCOMO comparison receipt")
    parser.add_argument("native_record", type=Path)
    parser.add_argument("fathom_record", type=Path)
    parser.add_argument("--native-output-dir", type=Path)
    parser.add_argument("--fathom-output-dir", type=Path)
    args = parser.parse_args(argv)
    try:
        native_record = json.loads(args.native_record.read_text(encoding="utf-8"))
        fathom_record = json.loads(args.fathom_record.read_text(encoding="utf-8"))
        common = {
            "ts": datetime.now(timezone.utc).replace(second=0, microsecond=0),
            "base_dir": _lib.EXPERIMENTS_DIR,
        }
        if (args.native_output_dir is None) != (args.fathom_output_dir is None):
            raise ValueError("both scored output directories are required together")
        if args.native_output_dir is not None:
            run_id, run_dir = write_scored_receipt(
                native_record,
                fathom_record,
                native_output_dir=args.native_output_dir,
                fathom_output_dir=args.fathom_output_dir,
                **common,
            )
        else:
            run_id, run_dir = write_receipt(native_record, fathom_record, **common)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"mem0-comparison: {exc}", file=__import__("sys").stderr)
        return 2
    print(f"mem0 comparison receipt {run_id}: {run_dir}")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
