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
SCHEMA_V2 = "comp-03.retrieval-sweep.v2"
ARM_IDS = ["fathomdb-fts", "fathomdb-hybrid", "lancedb-fts"]
CONFIG_KEYS = {
    "schema_version", "program_track", "campaign", "corpus", "selection", "document_unit",
    "arms", "retrieval", "metrics", "device", "decision_rule", "claim_boundary",
}
RULE_KEYS = {"metric", "treatment", "control", "focus_type"}
MQ1_KEYS = {"arm", "variant", "words", "engine_stopwords", "simulated_fts_ids", "pass", "informative"}


class Comp03RetrievalError(RuntimeError):
    pass


def load_config(path: Path) -> dict[str, Any]:
    config = json.loads(Path(path).read_text(encoding="utf-8"))
    v2 = config.get("schema_version") == SCHEMA_V2
    keys = CONFIG_KEYS | ({"mq1"} if v2 else set())
    if set(config) != keys:
        raise Comp03RetrievalError(f"config keys mismatch: {sorted(set(config) ^ keys)}")
    if config["schema_version"] not in {SCHEMA, SCHEMA_V2}:
        raise Comp03RetrievalError("unsupported config schema_version")
    if v2:
        _check_v2(config)
    elif any("variants" in arm or "engine_config" in arm for arm in config["arms"]):
        raise Comp03RetrievalError("v1 arms take no variants or engine settings")
    if [arm["id"] for arm in config["arms"]] != ARM_IDS:
        raise Comp03RetrievalError(f"sweep arms are {', '.join(ARM_IDS)}")
    if any(arm["product_arm"] not in comp_fts.ARMS for arm in config["arms"]):
        raise Comp03RetrievalError("every arm names a comp_fts product arm")
    if set(config["decision_rule"]) != RULE_KEYS:
        raise Comp03RetrievalError("decision_rule needs metric, treatment, control, focus_type")
    return config


def _check_v2(config: Mapping[str, Any]) -> None:
    """Slice 41: stop-word query variants of the FathomDB arms, extra limits, and MQ1."""
    variant_ids = [v["id"] for arm in config["arms"] for v in arm.get("variants", [])]
    arm_ids = [arm["id"] for arm in config["arms"]]
    if len(set(variant_ids + arm_ids)) != len(variant_ids + arm_ids):
        raise Comp03RetrievalError("arm and variant IDs must be distinct")
    for arm in config["arms"]:
        for variant in arm.get("variants", []):
            comp_fts.configured_arm(arm["product_arm"], variant["engine_config"])
    limits = config["retrieval"].get("extra_limits", [])
    if any(limit <= config["retrieval"]["top_k"] for limit in limits) or limits != sorted(set(limits)):
        raise Comp03RetrievalError("extra_limits ascend and exceed top_k")
    mq1 = config["mq1"]
    if set(mq1) != MQ1_KEYS:
        raise Comp03RetrievalError(f"mq1 keys mismatch: {sorted(set(mq1) ^ MQ1_KEYS)}")
    owner = next((arm for arm in config["arms"] if arm["id"] == mq1["arm"]), None)
    variant = next((v for v in (owner or {}).get("variants", []) if v["id"] == mq1["variant"]), None)
    if owner is None or owner["product_arm"] != "fathomdb" or variant is None:
        raise Comp03RetrievalError("mq1 names a variant of the fathomdb FTS arm")
    stop = variant["engine_config"]["query_stop_words"]
    if stop.get("base", "fathomdb") != "fathomdb" or stop.get("keep") or list(stop["extra"]) != list(mq1["words"]):
        raise Comp03RetrievalError("the mq1 variant adds exactly mq1.words to the built-in list")


def _repo_path(value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else _lib.EXPERIMENTS_DIR.parent / path


def check_engine_stopwords(words: Sequence[str]) -> None:
    """The transform's engine list must be the installed engine's built-in list."""
    import hashlib

    import fathomdb

    digest = hashlib.sha256("\n".join(sorted(set(words))).encode("utf-8")).hexdigest()
    if digest != fathomdb.DEFAULT_STOP_WORDS_SHA256:
        raise Comp03RetrievalError(
            f"engine_stopwords hash {digest} is not the installed DEFAULT_STOP_WORDS_SHA256 "
            f"{fathomdb.DEFAULT_STOP_WORDS_SHA256}")


def derived_result(result: Mapping[str, Any], variant_id: str) -> dict[str, Any]:
    """A variant's retrieval record: the parent store's build fields with the variant's IDs and timings."""
    variant = result["variants"][variant_id]
    shared = {k: result[k] for k in ("arm", "version", "documents", "blank_rounds_skipped", "disk_bytes", "setup",
                                     "ingest_ack_s", "build_return_s", "drain_s", "vector_rows") if k in result}
    return {**shared, **{k: variant[k] for k in ("settings", "engine_config", "stop_words_sha256", "ids", "ids_at",
                                                 "latency_ms", "repeat_latency_ms")},
            "variant_of": result.get("store_arm"), "store": "the parent arm's store, reopened with engine_config"}


def mq1_table(results: Mapping[str, Mapping[str, Any]], *, variant: str,
              simulated: Mapping[str, Sequence[str]]) -> dict[str, Any]:
    """MQ1: the setting's top-k against the harness transform's, per question; ``simulated`` is informative."""
    differing = sorted(qid for qid, r in results.items() if r["variants"][variant]["ids"] != r["transform"]["ids"])
    sim = [qid for qid in results if qid in simulated]
    return {
        "questions": len(results),
        "equal_transform": len(results) - len(differing),
        "differing_transform": differing,
        "pass": bool(results) and not differing,
        "simulated": {"questions": len(sim),
                      "equal": sum(results[q]["variants"][variant]["ids"] == list(simulated[q]) for q in sim),
                      "informative": True},
    }


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
    by_id = {arm["id"]: arm for arm in config["arms"]}
    top_k, ks = config["retrieval"]["top_k"], config["metrics"]["ks"]
    extra_limits = tuple(config["retrieval"].get("extra_limits", ()))
    mq1 = config.get("mq1")
    transform = None
    if mq1 is not None and mq1["arm"] in by_id:
        engine_stopwords = _read_json(_repo_path(mq1["engine_stopwords"]))["engine_stopwords"]
        check_engine_stopwords(engine_stopwords)
        transform = {"words": list(mq1["words"]), "engine_stopwords": engine_stopwords}
    attempt_root.mkdir(parents=True, exist_ok=True)
    _write_json(attempt_root / "config.resolved.json", config)
    rows: list[dict[str, Any]] = []
    results: dict[str, list[dict[str, Any]]] = {}
    for arm in config["arms"]:
        results[arm["id"]] = []
        for variant in arm.get("variants", []):
            results[variant["id"]] = []
    mq1_results: dict[str, dict[str, Any]] = {}
    for index, item in enumerate(items):
        case, gold = convert_question(item), gold_for(item)
        qid = case["question_id"]
        for arm_id, product in comp03_answer.arm_order(index, arms):
            path = attempt_root / "questions" / qid / arm_id / "retrieval.json"
            variants = by_id[arm_id].get("variants", [])
            if not path.exists():
                kwargs: dict[str, Any] = {}
                if extra_limits:
                    kwargs["extra_limits"] = extra_limits
                if variants:
                    kwargs["variants"] = variants
                if transform is not None and arm_id == mq1["arm"]:
                    kwargs["transform"] = transform
                result = comp03_answer.retrieve(product, case, attempt_root / "work", top_k=top_k,
                                                fathomdb_bin=fathomdb_bin, store_label=arm_id, **kwargs)
                if "vector_rows" in result and result["vector_rows"] != result["documents"]:
                    raise Comp03RetrievalError(f"{qid}/{arm_id}: incomplete vector coverage")
                _record_device(attempt_root, getattr(comp_fts.ARMS[product], "device", None))
                result["store_arm"] = arm_id
                _write_json(path, result)
            result = _read_json(path)
            scored = [(arm_id, result)]
            for variant in variants:
                vpath = attempt_root / "questions" / qid / variant["id"] / "retrieval.json"
                if not vpath.exists():
                    _write_json(vpath, derived_result(result, variant["id"]))
                scored.append((variant["id"], _read_json(vpath)))
            if transform is not None and arm_id == mq1["arm"]:
                mq1_results[qid] = result
            for scored_id, scored_result in scored:
                results[scored_id].append(scored_result)
                if not gold["abstention"]:
                    rows.append({"question_id": qid, "question_type": case["question_type"], "arm": scored_id,
                                 **question_scores(scored_result["ids"], gold, ks)})
        print(f"[{index + 1}/{len(items)}] {qid}", flush=True)
    summary = summarize(config, rows, results, attempt_root)
    if transform is not None:
        verdict = _read_json(_repo_path(mq1["simulated_fts_ids"]))
        summary["mq1"] = {**mq1_table(mq1_results, variant=mq1["variant"],
                                      simulated=verdict.get("simulated_fts_ids", {})),
                          "variant": mq1["variant"], "words": list(mq1["words"])}
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
            "stop_words_sha256": sorted({r["stop_words_sha256"] for r in arm_results if r.get("stop_words_sha256")}),
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
        "schema_version": ("comp-03.retrieval-sweep.summary.v2" if config["schema_version"] == SCHEMA_V2
                           else "comp-03.retrieval-sweep.summary.v1"),
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


# --- Unit G: evidence policies, evaluated offline on the stored sweep records --------------

EVIDENCE_BASELINE = {"per_round_chars": 4000, "total_chars": 20000, "session_expansion": False}
POOLED_TOLERANCE = 3
FTS_TOLERANCE = 1
EVIDENCE_RULE = (
    "maximise has_answer_in_prompt subject to, per type, has_answer_untruncated_in_prompt not lower than "
    "the current policy's (4000/20000/off) by more than 3 pooled counts or 1 count for fathomdb-fts alone; "
    "ties broken by fewer chars"
)
COUNT_METRICS = ("gold_session_any_in_prompt", "gold_session_all_in_prompt", "has_answer_in_prompt",
                 "has_answer_untruncated_in_prompt", "rounds_in_prompt", "chars")


def evidence_policy_grid() -> list[dict[str, Any]]:
    return [{"per_round_chars": per, "total_chars": total, "session_expansion": expansion}
            for per in (4000, 2000, 1000) for total in (20000, 30000) for expansion in (False, True)]


def policy_key(policy: Mapping[str, Any]) -> str:
    return f"{policy['per_round_chars']}/{policy['total_chars']}/{'on' if policy['session_expansion'] else 'off'}"


def parse_policy_key(key: str) -> dict[str, Any]:
    per, total, expansion = key.split("/")
    if expansion not in {"on", "off"}:
        raise ValueError(f"bad policy key {key}")
    return {"per_round_chars": int(per), "total_chars": int(total), "session_expansion": expansion == "on"}


def evidence_rounds(ids: Sequence[str], docs: Sequence[Mapping[str, Any]],
                    policy: Mapping[str, Any]) -> list[tuple[str, int]]:
    """The rounds a prompt shows, in order, with the characters each contributes.

    The top-k IDs come first in rank order; with session expansion, the remaining
    rounds of each hit session follow (sessions by their best hit's rank, rounds
    by position). Each round is capped at ``per_round_chars`` and then cut to what
    is left of ``total_chars``; whitespace-only rounds contribute 0 characters.
    """
    by_id = {doc["id"]: doc for doc in docs}
    order = list(ids)
    if policy["session_expansion"]:
        listed = set(order)
        for session in dict.fromkeys(_session(round_id) for round_id in ids):
            for doc in docs:
                if _session(doc["id"]) == session and doc["id"] not in listed:
                    order.append(doc["id"])
                    listed.add(doc["id"])
    remaining = policy["total_chars"]
    shown: list[tuple[str, int]] = []
    for round_id in order:
        if remaining <= 0:
            break
        body = by_id[round_id]["body"]
        if not body.strip():
            shown.append((round_id, 0))
            continue
        chars = min(len(body), policy["per_round_chars"], remaining)
        shown.append((round_id, chars))
        remaining -= chars
    return shown


def _question_evidence(shown: Sequence[tuple[str, int]], gold: Mapping[str, Any],
                       docs: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    lengths = {doc["id"]: len(doc["body"]) for doc in docs}
    chars = dict(shown)
    in_prompt = {round_id for round_id, count in shown if count > 0}
    session_relevant = list(gold["session_relevant"])
    answer_sessions = {_session(r) for r in session_relevant}
    found = {_session(r) for r in session_relevant if r in in_prompt}
    turn = list(gold["turn_relevant"])
    gold_shown = sum(chars.get(r, 0) for r in session_relevant)
    gold_total = sum(lengths[r] for r in session_relevant)
    return {
        "gold_session_any_in_prompt": int(bool(found)),
        "gold_session_all_in_prompt": int(bool(answer_sessions) and found == answer_sessions),
        "has_answer_in_prompt": int(any(r in in_prompt for r in turn)),
        "has_answer_untruncated_in_prompt": int(any(r in in_prompt and chars[r] == lengths[r] for r in turn)),
        "gold_chars_shown_fraction": gold_shown / gold_total if gold_total else 0.0,
        "rounds_in_prompt": len(in_prompt),
        "chars": sum(count for _, count in shown),
        "_gold_chars_shown": gold_shown,
        "_gold_chars_total": gold_total,
    }


def evidence_metrics(shown: Sequence[tuple[str, int]], gold: Mapping[str, Any],
                     docs: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """Per-question prompt metrics for one policy's shown rounds."""
    return {k: v for k, v in _question_evidence(shown, gold, docs).items() if not k.startswith("_")}


def _aggregate_evidence(rows: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    def block(members: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
        out: dict[str, Any] = {name: sum(r[name] for r in members) for name in COUNT_METRICS}
        shown, total = sum(r["_gold_chars_shown"] for r in members), sum(r["_gold_chars_total"] for r in members)
        out["gold_chars_shown_fraction"] = shown / total if total else 0.0
        out["n"] = len(members)
        return out

    return {"overall": block(rows),
            "per_type": {t: block([r for r in rows if r["question_type"] == t])
                         for t in sorted({r["question_type"] for r in rows})}}


def select_evidence_policy(pooled: Mapping[str, Any], *, baseline: str,
                           fts: Mapping[str, Any] | None = None) -> dict[str, Any]:
    """Apply the pre-stated rule (REQ40-G2) to pooled and, optionally, fathomdb-fts tables."""
    def failures(table: Mapping[str, Any], key: str, tolerance: int, label: str) -> list[str]:
        base = table[baseline]["per_type"]
        row = table[key]["per_type"]
        return [f"{label}{qtype}" for qtype in base
                if row[qtype]["has_answer_untruncated_in_prompt"]
                < base[qtype]["has_answer_untruncated_in_prompt"] - tolerance]

    excluded: dict[str, list[str]] = {}
    for key in pooled:
        failed = failures(pooled, key, POOLED_TOLERANCE, "")
        if fts is not None:
            failed += failures(fts, key, FTS_TOLERANCE, "fathomdb-fts:")
        if failed:
            excluded[key] = failed
    candidates = [key for key in pooled if key not in excluded]
    best = max(candidates, key=lambda key: (pooled[key]["overall"]["has_answer_in_prompt"],
                                            -pooled[key]["overall"]["chars"]))
    return {"policy": best, "baseline": baseline, "excluded": excluded, "rule": EVIDENCE_RULE}


def evaluate_evidence_policies(attempt_root: Path, dataset_path: Path, answer_ids: Sequence[str],
                               arms: Sequence[str]) -> dict[str, Any]:
    """Score every policy on every scored question, for the selection and answer sets."""
    items = json.loads(Path(dataset_path).read_text(encoding="utf-8"))
    answer = set(answer_ids)
    grid = evidence_policy_grid()
    rows: dict[str, dict[str, dict[str, list[dict[str, Any]]]]] = {
        subset: {policy_key(policy): {arm: [] for arm in arms} for policy in grid}
        for subset in ("selection", "answer_set")}
    counts = {"scored": 0, "selection": 0, "answer_set_ids": len(answer), "answer_set_scored": 0}
    for item in sorted(items, key=lambda i: i["question_id"]):
        case, gold = convert_question(item), gold_for(item)
        if gold["abstention"]:
            continue
        counts["scored"] += 1
        subset = "answer_set" if case["question_id"] in answer else "selection"
        counts["selection" if subset == "selection" else "answer_set_scored"] += 1
        for arm in arms:
            ids = _read_json(attempt_root / "questions" / case["question_id"] / arm / "retrieval.json")["ids"]
            for policy in grid:
                metrics = _question_evidence(evidence_rounds(ids, case["docs"], policy), gold, case["docs"])
                rows[subset][policy_key(policy)][arm].append({**metrics, "question_type": case["question_type"]})
    tables = {}
    for subset, by_policy in rows.items():
        tables[subset] = {
            "pooled": {key: _aggregate_evidence([r for arm in arms for r in by_arm[arm]])
                       for key, by_arm in by_policy.items()},
            "per_arm": {arm: {key: _aggregate_evidence(by_arm[arm]) for key, by_arm in by_policy.items()}
                        for arm in arms},
        }
    return {"tables": tables, "questions": counts}


def _overall_delta(table: Mapping[str, Any], policy: str, baseline: str) -> dict[str, Any]:
    new, base = table[policy]["overall"], table[baseline]["overall"]
    return {name: new[name] - base[name] for name in (*COUNT_METRICS, "gold_chars_shown_fraction")}


def evidence_policy_verdict(attempt_root: Path, dataset_path: Path, answer_ids: Sequence[str],
                            arms: Sequence[str]) -> dict[str, Any]:
    """Select the policy on the selection set and record the answer-set expectation."""
    evaluated = evaluate_evidence_policies(attempt_root, dataset_path, answer_ids, arms)
    tables = evaluated["tables"]
    baseline = policy_key(EVIDENCE_BASELINE)
    selection = tables["selection"]
    choice = select_evidence_policy(selection["pooled"], baseline=baseline,
                                    fts=selection["per_arm"].get("fathomdb-fts"))
    answer_set = tables["answer_set"]
    return {
        "schema_version": "comp-03.evidence-policy-verdict.v1",
        **choice,
        "selection_set": "scored questions not among the answer-run question IDs",
        "questions": evaluated["questions"],
        "arms": list(arms),
        "dataset_sha256": sha256_file(dataset_path),
        "sweep_summary_sha256": sha256_file(attempt_root / "summary.json"),
        "sweep_attempt_root": str(attempt_root),
        "tables": tables,
        "answer_set_expectation": {
            "policy": choice["policy"], "baseline": baseline,
            "pooled": _overall_delta(answer_set["pooled"], choice["policy"], baseline),
            "per_arm": {arm: _overall_delta(answer_set["per_arm"][arm], choice["policy"], baseline) for arm in arms},
        },
    }


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
    evidence = sub.add_parser("evidence", help="evaluate evidence policies offline and write the verdict")
    evidence.add_argument("--attempt-root", type=Path, required=True)
    evidence.add_argument("--dataset", type=Path, required=True)
    evidence.add_argument("--answer-config", type=Path, required=True,
                          help="answer config whose question IDs form the answer set")
    evidence.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command == "evidence":
        answer_config = comp03_answer.load_config(args.answer_config)
        arms = [arm["id"] for arm in answer_config["arms"]]
        verdict = evidence_policy_verdict(args.attempt_root, args.dataset,
                                          answer_config["selection"]["question_ids"], arms)
        args.out.write_text(json.dumps(verdict, indent=1, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps({"policy": verdict["policy"], "questions": verdict["questions"],
                          "excluded": verdict["excluded"]}))
        return 0
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
