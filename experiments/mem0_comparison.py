"""Close a safe, typed comparison receipt after both LOCOMO arms complete."""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from experiments import _lib


EXPERIMENT = "fathomdb-vs-mem0-locomo-comparison"
SCHEMA_VERSION = "mem0-comparison.v1"
RESULT_SCHEMA_VERSION = "mem0-comparison.result.v1"


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
        "resume", "max_workers", "rpm",
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
        "resume": native_benchmark["resume"], "max_workers": native_benchmark["max_workers"], "rpm": native_benchmark["rpm"],
        "raw_sha256": native_corpus["raw_sha256"], "normalized_sha256": native_corpus["normalized_sha256"],
        "sessions": native_corpus["sessions"], "eligible_questions": native_corpus["eligible_questions"],
        "harness_git_sha": native_config.get("harness", {}).get("git_sha"),
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
        expected_program_track="LOCOMO-01",
    )
    workload = _matched_workload(native, fathom)
    config = {
        "schema_version": SCHEMA_VERSION, "campaign": "locomo_predict_only",
        "program_track": "MEMORY-01",
        "workload": workload,
        "arms": {"mem0_oss": native["run_id"], "fathomdb": fathom["run_id"]},
    }
    run_id = _lib.make_run_id(EXPERIMENT, ts, _lib.config_sha256(config))
    run_dir = Path(base_dir) / "runs" / run_id
    result = {"schema_version": RESULT_SCHEMA_VERSION, "run_id": run_id, "verdict": "complete", "arms": config["arms"], "workload": workload}
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


def main(argv: list[str] | None = None) -> int:
    """Close a comparator receipt from two existing generic arm receipts."""
    parser = argparse.ArgumentParser(description="Mem0/FathomDB LOCOMO comparison receipt")
    parser.add_argument("native_record", type=Path)
    parser.add_argument("fathom_record", type=Path)
    args = parser.parse_args(argv)
    try:
        run_id, run_dir = write_receipt(
            json.loads(args.native_record.read_text(encoding="utf-8")),
            json.loads(args.fathom_record.read_text(encoding="utf-8")),
            ts=datetime.now(timezone.utc).replace(second=0, microsecond=0), base_dir=_lib.EXPERIMENTS_DIR,
        )
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"mem0-comparison: {exc}", file=__import__("sys").stderr)
        return 2
    print(f"mem0 comparison receipt {run_id}: {run_dir}")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
