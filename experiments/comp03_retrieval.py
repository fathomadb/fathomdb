"""COMP-03 retrieval sweep: FathomDB FTS vs FathomDB hybrid on every LongMemEval-S question.

Retrieval only, no models. Each question is its own fresh store per arm, built
from the same rounds through ``comp03_answer``'s conversion, so the gold-leak
discipline is the answer run's. The sweep reports session and turn recall,
session MRR, and paired bootstrap CIs for every hybrid − FTS delta, then
evaluates the decision rule stated in the config and writes its verdict into
the summary; the COMP-03 answer run reads that verdict to decide whether its
hybrid arm runs.

    python -m experiments.comp03_retrieval run CONFIG --dataset FILE --attempt-root DIR
        [--fathomdb-bin PATH] [--max-questions N] [--base-dir DIR]

The embedding device is ``FATHOMDB_EMBED_DEVICE`` (``auto``, ``cpu``,
``cuda:N``); a CUDA request that resolves to CPU stops the sweep.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import statistics
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib
from experiments import comp03_answer
from experiments import comp_fts
from experiments import comp_record
from experiments.comp03_answer import convert_question, gold_for, sha256_file

__all__ = ["convert_question", "gold_for"]

DEFAULT_CONFIG = _lib.EXPERIMENTS_DIR / "configs" / "comp-03" / "retrieval-sweep.v1.json"
EXPERIMENT = "comp-03-retrieval-sweep"
SCHEMA = "comp-03.retrieval-sweep.v1"
ARM_IDS = ["fathomdb-fts", "fathomdb-hybrid", "lancedb-fts"]
CONFIG_KEYS = {
    "schema_version", "program_track", "campaign", "corpus", "selection", "document_unit",
    "arms", "retrieval", "metrics", "device", "decision_rule", "claim_boundary",
}
RULE_KEYS = {"metric", "treatment", "control", "focus_type"}


class Comp03RetrievalError(RuntimeError):
    pass


def load_config(path: Path) -> dict[str, Any]:
    config = json.loads(Path(path).read_text(encoding="utf-8"))
    if set(config) != CONFIG_KEYS:
        raise Comp03RetrievalError(f"config keys mismatch: {sorted(set(config) ^ CONFIG_KEYS)}")
    if config["schema_version"] != SCHEMA:
        raise Comp03RetrievalError("unsupported config schema_version")
    if [arm["id"] for arm in config["arms"]] != ARM_IDS:
        raise Comp03RetrievalError(f"sweep arms are {', '.join(ARM_IDS)}")
    if any(arm["product_arm"] not in comp_fts.ARMS for arm in config["arms"]):
        raise Comp03RetrievalError("every arm names a comp_fts product arm")
    if set(config["decision_rule"]) != RULE_KEYS:
        raise Comp03RetrievalError("decision_rule needs metric, treatment, control, focus_type")
    return config


# --- metrics -----------------------------------------------------------------


def _session(round_id: str) -> str:
    return round_id.split("-", 1)[0]


def question_scores(ids: Sequence[str], gold: Mapping[str, Any], ks: Sequence[int] = (5, 10)) -> dict[str, float | None]:
    """Per-question retrieval metrics over the ranked round IDs.

    Session metrics use every round of every answer session; Recall-all@k
    needs at least one round of each answer session in the top k. Turn
    recall uses the ``has_answer`` rounds. A metric is ``None`` when the
    question has no relevant round of that kind.
    """
    session_relevant = set(gold["session_relevant"])
    answer_sessions = {_session(r) for r in session_relevant}
    turn_relevant = set(gold["turn_relevant"])
    out: dict[str, float | None] = {}
    for k in ks:
        top = list(ids)[:k]
        if session_relevant:
            found = {_session(r) for r in top if r in session_relevant}
            out[f"session_recall_any_at_{k}"] = 1.0 if found else 0.0
            out[f"session_recall_all_at_{k}"] = 1.0 if found == answer_sessions else 0.0
        else:
            out[f"session_recall_any_at_{k}"] = out[f"session_recall_all_at_{k}"] = None
        out[f"turn_recall_any_at_{k}"] = (
            (1.0 if any(r in turn_relevant for r in top) else 0.0) if turn_relevant else None
        )
    if session_relevant:
        rank = next((i for i, r in enumerate(list(ids)[:10], 1) if r in session_relevant), None)
        out["session_mrr_at_10"] = 0.0 if rank is None else 1.0 / rank
    else:
        out["session_mrr_at_10"] = None
    return out


def bootstrap_ci(diffs: Sequence[float], *, seed: int, resamples: int) -> list[float]:
    """Percentile 95% CI of the mean of ``diffs`` (paired resampling)."""
    if not diffs:
        return [0.0, 0.0]
    rng = random.Random(seed)
    n = len(diffs)
    boots = sorted(statistics.fmean(rng.choices(diffs, k=n)) for _ in range(resamples))
    return [boots[round(0.025 * resamples) - 1], boots[round(0.975 * resamples) - 1]]


def paired_deltas(rows: Sequence[Mapping[str, Any]], metric: str, *, treatment: str, control: str,
                  seed: int, resamples: int) -> dict[str, Any]:
    """Mean ``treatment − control`` over questions both arms scored, overall and per type."""
    by_question: dict[str, dict[str, Any]] = {}
    for row in rows:
        if row["arm"] in (treatment, control) and row.get(metric) is not None:
            entry = by_question.setdefault(row["question_id"], {"question_type": row["question_type"]})
            entry[row["arm"]] = row[metric]
    pairs = [(e["question_type"], e[treatment] - e[control])
             for _, e in sorted(by_question.items()) if treatment in e and control in e]

    def summary(diffs: list[float]) -> dict[str, Any]:
        return {"delta": statistics.fmean(diffs) if diffs else 0.0,
                "ci": bootstrap_ci(diffs, seed=seed, resamples=resamples), "n": len(diffs)}

    return {
        "overall": summary([d for _, d in pairs]),
        "per_type": {qtype: summary([d for t, d in pairs if t == qtype]) for qtype in sorted({t for t, _ in pairs})},
    }


def evaluate_decision_rule(rule: Mapping[str, Any], deltas: Mapping[str, Any]) -> dict[str, Any]:
    """Adopt iff (a) focus-type CI lower > 0, (b) no other type's CI upper < 0, (c) overall delta >= 0."""
    focus = rule["focus_type"]
    per_type = deltas["per_type"]
    clauses = {
        "a": focus in per_type and per_type[focus]["ci"][0] > 0,
        "b": all(value["ci"][1] >= 0 for qtype, value in per_type.items() if qtype != focus),
        "c": deltas["overall"]["delta"] >= 0,
    }
    return {"verdict": "adopt" if all(clauses.values()) else "reject", "clauses": clauses}


def require_vector_coverage(database_path: Path, documents: int) -> int:
    """Embedded ``doc`` cursors on a closed store; raises unless every document has one."""
    count = comp_fts.vector_row_count(Path(database_path))
    if count != documents:
        raise Comp03RetrievalError(f"{database_path}: {count} of {documents} documents embedded")
    return count


# --- sweep ----------------------------------------------------------------------


def _write_json(path: Path, value: Any) -> None:
    comp03_answer._write_json(path, value)


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _record_device(attempt_root: Path, device: dict[str, Any] | None) -> None:
    if device is None:
        return
    path = attempt_root / "device.json"
    if path.exists():
        if _read_json(path) != device:
            raise Comp03RetrievalError(f"device changed within one attempt: {_read_json(path)} vs {device}")
        return
    _write_json(path, device)


def _mean_block(rows: Sequence[Mapping[str, Any]], metrics: Sequence[str]) -> dict[str, Any]:
    block: dict[str, Any] = {"n": len(rows)}
    for metric in metrics:
        values = [r[metric] for r in rows if r.get(metric) is not None]
        block[metric] = statistics.fmean(values) if values else None
        block[f"{metric}_n"] = len(values)
    return block


def _timing(results: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    if not results:
        return {}
    first = [r["latency_ms"] for r in results]
    warm = [statistics.median(r["repeat_latency_ms"]) for r in results]
    out = {
        "version": results[0]["version"],
        "query_first_p50_ms": statistics.median(first),
        "query_warm_p50_ms": statistics.median(warm),
        "query_warm_p95_ms": comp_fts._pct(warm, 0.95),
        "build_p50_s": statistics.median(r["build_return_s"] for r in results),
    }
    if all("drain_s" in r for r in results):
        out["drain_p50_s"] = statistics.median(r["drain_s"] for r in results)
    return out


def sweep(config: dict[str, Any], *, dataset_path: Path, attempt_root: Path, fathomdb_bin: str | None = None,
          max_questions: int | None = None) -> dict[str, Any]:
    """Retrieve every question through every arm (resumable), then score and decide."""
    if sha256_file(dataset_path) != config["corpus"]["sha256"]:
        raise Comp03RetrievalError("dataset digest does not match the config")
    items = sorted(json.loads(dataset_path.read_text(encoding="utf-8")), key=lambda item: item["question_id"])
    if max_questions is not None:
        items = items[:max_questions]
    arms = [(arm["id"], arm["product_arm"]) for arm in config["arms"]]
    top_k, ks = config["retrieval"]["top_k"], config["metrics"]["ks"]
    attempt_root.mkdir(parents=True, exist_ok=True)
    _write_json(attempt_root / "config.resolved.json", config)
    rows: list[dict[str, Any]] = []
    results: dict[str, list[dict[str, Any]]] = {arm_id: [] for arm_id, _ in arms}
    for index, item in enumerate(items):
        case, gold = convert_question(item), gold_for(item)
        qid = case["question_id"]
        for arm_id, product in comp03_answer.arm_order(index, arms):
            path = attempt_root / "questions" / qid / arm_id / "retrieval.json"
            if not path.exists():
                result = comp03_answer.retrieve(product, case, attempt_root / "work", top_k=top_k,
                                                fathomdb_bin=fathomdb_bin, store_label=arm_id)
                if "vector_rows" in result and result["vector_rows"] != result["documents"]:
                    raise Comp03RetrievalError(f"{qid}/{arm_id}: incomplete vector coverage")
                _record_device(attempt_root, getattr(comp_fts.ARMS[product], "device", None))
                _write_json(path, result)
            result = _read_json(path)
            results[arm_id].append(result)
            if not gold["abstention"]:
                rows.append({"question_id": qid, "question_type": case["question_type"], "arm": arm_id,
                             **question_scores(result["ids"], gold, ks)})
        print(f"[{index + 1}/{len(items)}] {qid}", flush=True)
    summary = summarize(config, rows, results, attempt_root)
    _write_json(attempt_root / "summary.json", summary)
    return summary


def metric_names(ks: Sequence[int]) -> list[str]:
    names = []
    for k in ks:
        names += [f"session_recall_any_at_{k}", f"session_recall_all_at_{k}", f"turn_recall_any_at_{k}"]
    return names + ["session_mrr_at_10"]


def summarize(config: Mapping[str, Any], rows: Sequence[Mapping[str, Any]],
              results: Mapping[str, Sequence[Mapping[str, Any]]], attempt_root: Path) -> dict[str, Any]:
    metrics = metric_names(config["metrics"]["ks"])
    boot = config["metrics"]["bootstrap"]
    rule = config["decision_rule"]
    arms_out = {}
    for arm_id, arm_results in results.items():
        mine = [r for r in rows if r["arm"] == arm_id]
        arms_out[arm_id] = {
            "overall": _mean_block(mine, metrics),
            "per_type": {t: _mean_block([r for r in mine if r["question_type"] == t], metrics)
                         for t in sorted({r["question_type"] for r in mine})},
            "timing": _timing(arm_results),
            "questions": len(arm_results),
        }
    treatment, control = rule["treatment"], rule["control"]
    deltas: dict[str, Any] = {}
    if treatment in results and control in results:
        deltas = {metric: paired_deltas(rows, metric, treatment=treatment, control=control,
                                        seed=boot["seed"], resamples=boot["resamples"]) for metric in metrics}
        decision = {**evaluate_decision_rule(rule, deltas[rule["metric"]]), "rule": dict(rule)}
    else:
        decision = {"verdict": "not_evaluated", "clauses": {}, "rule": dict(rule),
                    "reason": f"sweep lacks {treatment} or {control}"}
    device_path = attempt_root / "device.json"
    return {
        "schema_version": "comp-03.retrieval-sweep.summary.v1",
        "questions": max((len(v) for v in results.values()), default=0),
        "arms": arms_out,
        "deltas": {"treatment": treatment, "control": control, "by_metric": deltas},
        "decision": decision,
        "device": _read_json(device_path) if device_path.exists() else None,
        "blank_rounds_skipped": {arm_id: sum(r.get("blank_rounds_skipped", 0) for r in arm_results)
                                 for arm_id, arm_results in results.items()},
    }


def write_receipt(config: dict[str, Any], summary: dict[str, Any], *, attempt_root: Path, dataset_path: Path,
                  base_dir: Path | None, config_path: str | None,
                  engine: dict[str, str] | None = None) -> tuple[str, Path]:
    base = base_dir if base_dir is not None else _lib.EXPERIMENTS_DIR
    git = _lib.git_info()
    decision = summary["decision"]
    metric = config["decision_rule"]["metric"]
    overall = summary["deltas"]["by_metric"].get(metric, {}).get("overall")
    read = f"decision {decision['verdict']} on {metric}"
    if overall:
        read += f": hybrid − fts {overall['delta']:+.3f} (95% {overall['ci'][0]:+.3f}..{overall['ci'][1]:+.3f}, n={overall['n']})"
    run_id, run_dir = _lib.write_record(
        EXPERIMENT,
        ts=datetime.now(UTC),
        config_obj=config,
        metrics=summary,
        verdict="complete",
        read=read,
        title="COMP-03 LongMemEval-S retrieval sweep, FathomDB FTS vs hybrid",
        headline={"decision": decision["verdict"], "clauses": decision["clauses"], "questions": summary["questions"]},
        n=summary["questions"],
        code={"git_sha": git.get("git_sha"), "dirty": git.get("dirty"), "branch": git.get("branch"),
              "baseline_commit": None},
        corpus={"source": config["corpus"]["identifier"], "manifest_sha256": None,
                "datasets": [{"name": dataset_path.name, "sha256": config["corpus"]["sha256"]}]},
        seeds={"bootstrap": config["metrics"]["bootstrap"]["seed"]},
        env=_lib.env_info(key_deps={arm: block["timing"].get("version") for arm, block in summary["arms"].items()}
                          | comp_record.engine_key_deps(engine)),
        cost_usd=0.0,
        config_path=config_path,
        artifacts=[{"kind": "external_attempt", "summary_sha256": sha256_file(attempt_root / "summary.json")}],
        base_dir=base,
    )
    _lib.regen_index_md(index_path=base / "index.jsonl", md_path=base / "INDEX.md")
    return run_id, run_dir


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("config", type=Path)
    run.add_argument("--dataset", type=Path, required=True)
    run.add_argument("--attempt-root", type=Path, required=True)
    run.add_argument("--fathomdb-bin")
    run.add_argument("--max-questions", type=int)
    run.add_argument("--base-dir", type=Path, help="receipt root; defaults to experiments/")
    run.add_argument("--engine-commit", required=True, help="40-hex source commit of the installed fathomdb build")
    args = parser.parse_args(argv)
    engine = comp_record.engine_identity(args.engine_commit)
    config = load_config(args.config)
    os.environ.setdefault("FATHOMDB_EMBED_DEVICE", "auto")
    summary = sweep(config, dataset_path=args.dataset, attempt_root=args.attempt_root,
                    fathomdb_bin=args.fathomdb_bin, max_questions=args.max_questions)
    run_id, run_dir = write_receipt(config, summary, attempt_root=args.attempt_root, dataset_path=args.dataset,
                                    base_dir=args.base_dir, config_path=str(args.config), engine=engine)
    print(json.dumps({"run_id": run_id, "receipt": str(run_dir / "record.json"), "decision": summary["decision"]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
