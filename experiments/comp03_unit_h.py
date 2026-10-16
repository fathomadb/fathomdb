"""COMP-03 Unit H selection rules (Slice 40): the H1 query stop list and the H2 hybrid vector pool.

Both rules are fixed in their configs before anything is measured; the
commands measure on the final-head engine and write a verdict file that the
engine change then pins.

    python -m experiments.comp03_unit_h stoplist CONFIG --dataset FILE --nfcorpus-inputs DIR
        --attempt-root DIR --engine-commit SHA --runner TEXT [--out FILE]
    python -m experiments.comp03_unit_h pool CONFIG --dataset FILE --attempt-root DIR
        --engine-commit SHA --runner TEXT [--fathomdb-bin PATH] [--out FILE]

``stoplist`` simulates a longer stop list by removing the words from the
question before ``search_text_only``; that equals compiling with them as
stopwords whenever the query keeps a content token, and the rule prunes
until every query does. ``pool`` calls the sweep's own hybrid retrieval
(``comp03_answer.retrieve``) at ``top_k = P``; pool ``P`` at ``k <= P`` is the
first ``k`` of that list. Both resume per question.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import statistics
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping, Sequence

from experiments import _lib
from experiments import comp03_answer
from experiments import comp03_retrieval
from experiments import comp_fts
from experiments import comp_record

CONFIG_DIR = _lib.EXPERIMENTS_DIR / "configs" / "comp-03"
REPO_ROOT = _lib.EXPERIMENTS_DIR.parent
DEFAULT_STOPLIST_CONFIG = CONFIG_DIR / "stoplist-selection.v1.json"
DEFAULT_POOL_CONFIG = CONFIG_DIR / "vector-pool-selection.v1.json"
STOPLIST_SCHEMA = "comp-03.stoplist-selection.v1"
STOPLIST_VERDICT_SCHEMA = "comp-03.stoplist-verdict.v1"
POOL_SCHEMA = "comp-03.vector-pool-selection.v1"
POOL_VERDICT_SCHEMA = "comp-03.vector-pool-verdict.v1"
STOPLIST_KEYS = {
    "schema_version", "campaign", "corpus", "nfcorpus", "answer_set", "selection", "arm", "engine_commit",
    "engine_stopwords", "candidates", "removal_transform", "rule", "verdict_path", "claim_boundary",
}
STOPLIST_RULE_KEYS = {
    "text", "keep_min_delta_all", "nfcorpus_floor", "per_type_metric", "per_type_tolerance_questions",
    "combined_metric", "combined_point_min_questions", "combined_ci_lower_min_questions", "bootstrap", "tie_break",
}
POOL_KEYS = {
    "schema_version", "campaign", "corpus", "answer_set", "selection", "arm", "engine_commit", "candidates",
    "reference", "ks", "constraints", "rule", "device", "verdict_path", "claim_boundary",
}
SCORE_KS = (5, 10)
# Floating sums of per-word nDCG deltas land a hair off the stated boundaries.
EPS = 1e-12

Measurement = dict[str, Any]
Measure = Callable[[Sequence[frozenset[str]]], list[Measurement]]


class Comp03UnitHError(RuntimeError):
    pass


# --- configs ------------------------------------------------------------------------


def _load(path: Path, keys: set[str], schema: str) -> dict[str, Any]:
    config = json.loads(Path(path).read_text(encoding="utf-8"))
    if set(config) != keys:
        raise Comp03UnitHError(f"config keys mismatch: {sorted(set(config) ^ keys)}")
    if config["schema_version"] != schema:
        raise Comp03UnitHError(f"config schema_version must be {schema}")
    return config


def load_stoplist_config(path: Path) -> dict[str, Any]:
    config = _load(path, STOPLIST_KEYS, STOPLIST_SCHEMA)
    if set(config["rule"]) != STOPLIST_RULE_KEYS:
        raise Comp03UnitHError(f"rule keys mismatch: {sorted(set(config['rule']) ^ STOPLIST_RULE_KEYS)}")
    candidates, stop = config["candidates"], config["engine_stopwords"]
    if len(set(candidates)) != len(candidates) or set(candidates) & set(stop):
        raise Comp03UnitHError("candidates must be distinct and not already stopwords")
    if any(content_tokens(w, []) != [w] for w in candidates):
        raise Comp03UnitHError("every candidate must be one lowercase alphanumeric token")
    return config


def load_pool_config(path: Path) -> dict[str, Any]:
    config = _load(path, POOL_KEYS, POOL_SCHEMA)
    if config["reference"] != min(config["candidates"]) or config["candidates"] != sorted(config["candidates"]):
        raise Comp03UnitHError("candidates ascend and the reference is the smallest")
    if config["arm"]["product_arm"] != "fathomdb-hybrid":
        raise Comp03UnitHError("the pool is measured on the fathomdb-hybrid product arm")
    return config


def answer_ids(config: Mapping[str, Any]) -> list[str]:
    path = REPO_ROOT / config["answer_set"]["config"]
    ids = json.loads(path.read_text(encoding="utf-8"))["selection"]["question_ids"]
    if len(ids) != config["answer_set"]["question_ids"]:
        raise Comp03UnitHError(f"{path}: {len(ids)} answer-set IDs, expected {config['answer_set']['question_ids']}")
    return ids


# --- H1 simulation ------------------------------------------------------------------


def _split(raw: str) -> list[str]:
    tokens, current = [], []
    for char in raw.lower():
        if char.isalnum():
            current.append(char)
        else:
            tokens.append("".join(current))
            current = []
    tokens.append("".join(current))
    return tokens


def content_tokens(raw: str, stopwords: Iterable[str]) -> list[str]:
    """``fathomdb-query``'s ``content_tokens``: lowercased, split on non-alphanumeric, stopwords removed, deduped."""
    stop = set(stopwords)
    out: list[str] = []
    for token in _split(raw):
        if token and token not in stop and token not in out:
            out.append(token)
    return out


def strip_words(raw: str, words: Iterable[str]) -> str:
    """The removal transform: lowercase, split on non-alphanumeric, drop ``words``, join with spaces."""
    drop = set(words)
    return " ".join(token for token in _split(raw) if token and token not in drop)


def empties(raw: str, words: Iterable[str], engine_stopwords: Sequence[str]) -> bool:
    """True when ``raw`` has a content token under the engine list but none once ``words`` join it.

    A query with no content token under the engine list already takes the raw
    fallback, and no extra word changes that.
    """
    return bool(content_tokens(raw, engine_stopwords)) and not content_tokens(raw, [*engine_stopwords, *words])


def simulated_query(raw: str, words: Iterable[str], engine_stopwords: Sequence[str]) -> str:
    """The text sent to the unchanged engine to reproduce compiling with ``words`` as stopwords."""
    words = set(words)
    if not words or not content_tokens(raw, engine_stopwords):
        return raw
    return strip_words(raw, words)


# --- H1 rule ------------------------------------------------------------------------


def _selection(measurement: Measurement) -> dict[str, Mapping[str, Any]]:
    return {qid: row for qid, row in measurement["questions"].items() if row["subset"] == "selection"}


def _count(rows: Iterable[Mapping[str, Any]], metric: str) -> int:
    return int(round(sum(row[metric] for row in rows if row.get(metric) is not None)))


def _per_type_counts(measurement: Measurement, metric: str) -> dict[str, int]:
    rows = _selection(measurement).values()
    return {qtype: _count([r for r in rows if r["question_type"] == qtype], metric)
            for qtype in sorted({r["question_type"] for r in rows})}


def _diffs(new: Measurement, base: Measurement, metric: str) -> list[float]:
    base_rows, new_rows = _selection(base), _selection(new)
    return [new_rows[qid][metric] - base_rows[qid][metric] for qid in sorted(base_rows)
            if base_rows[qid].get(metric) is not None and new_rows[qid].get(metric) is not None]


def clause_a_holds(nfcorpus: Mapping[str, float], rule: Mapping[str, Any]) -> bool:
    """NFCorpus nDCG@10 and MRR@10 at or above the final-head baseline minus its tolerance."""
    return all(nfcorpus[metric] >= floor["baseline"] - floor["tolerance"] - EPS
               for metric, floor in rule["nfcorpus_floor"].items())


def clause_b_holds(counts: Mapping[str, int], base_counts: Mapping[str, int], rule: Mapping[str, Any]) -> bool:
    """Every type's selection-set count at least the D3 count minus the tolerance."""
    tol = rule["per_type_tolerance_questions"]
    return all(counts.get(qtype, 0) >= base - tol for qtype, base in base_counts.items())


def clause_c_holds(point_questions: float, ci_lower_questions: float, rule: Mapping[str, Any]) -> bool:
    """Owner ruling N1: point delta >= +1 question and CI lower bound >= -1 question."""
    return (point_questions >= rule["combined_point_min_questions"] - EPS
            and ci_lower_questions >= rule["combined_ci_lower_min_questions"] - EPS)


def combined_clause_c(diffs: Sequence[float], rule: Mapping[str, Any]) -> dict[str, Any]:
    """Paired delta and seeded bootstrap CI, both in questions (the CI of the mean times n)."""
    boot = rule["bootstrap"]
    n = len(diffs)
    ci = comp03_retrieval.bootstrap_ci(list(diffs), seed=boot["seed"], resamples=boot["resamples"])
    point = int(round(sum(diffs)))
    ci_questions = [ci[0] * n, ci[1] * n]
    return {"n": n, "point_delta_questions": point, "ci_mean": ci, "ci_questions": ci_questions,
            "holds": clause_c_holds(point, ci_questions[0], rule)}


def _summary(measurement: Measurement, rule: Mapping[str, Any]) -> dict[str, Any]:
    rows = list(measurement["questions"].values())
    out: dict[str, Any] = {"nfcorpus": dict(measurement["nfcorpus"])}
    for subset in ("selection", "answer_set"):
        mine = [r for r in rows if r["subset"] == subset]
        metrics = sorted({k for r in mine for k in r if k.startswith("session_")})
        out[subset] = {
            "n": len(mine),
            "overall": {m: _count(mine, m) for m in metrics},
            "per_type": {t: {m: _count([r for r in mine if r["question_type"] == t], m) for m in metrics}
                         for t in sorted({r["question_type"] for r in mine})},
        }
    return out


def select_stoplist(config: Mapping[str, Any], measure: Measure, queries: Mapping[str, str]) -> dict[str, Any]:
    """Evaluate the H1 rule. ``measure`` scores lists of extra words; ``queries`` are every query text by id."""
    rule = config["rule"]
    stop = config["engine_stopwords"]
    metric_all, metric_type = rule["combined_metric"], rule["per_type_metric"]
    candidates = list(config["candidates"])

    def emptied(words: Iterable[str]) -> list[str]:
        words = list(words)
        return sorted(qid for qid, text in queries.items() if empties(text, words, stop))

    per_word: dict[str, dict[str, Any]] = {}
    measurable = []
    for word in candidates:
        lost = emptied([word])
        per_word[word] = {"excluded_empty": lost}
        if not lost:
            measurable.append(word)
    measured = measure([frozenset(), *[frozenset({w}) for w in measurable]])
    base = measured[0]
    for word, result in zip(measurable, measured[1:]):
        diffs = _diffs(result, base, metric_all)
        per_word[word].update({
            "gained": sum(d > 0 for d in diffs), "lost": sum(d < 0 for d in diffs),
            "delta_all": int(round(sum(diffs))),
            "delta_ndcg": result["nfcorpus"]["ndcg_at_10"] - base["nfcorpus"]["ndcg_at_10"],
            "delta_mrr": result["nfcorpus"]["mrr_at_10"] - base["nfcorpus"]["mrr_at_10"],
        })
    for word in candidates:
        entry = per_word[word]
        entry["kept"] = "delta_all" in entry and entry["delta_all"] >= rule["keep_min_delta_all"]
    keep = [w for w in candidates if per_word[w]["kept"]]

    def drop_key(word: str) -> tuple[float, int, str]:
        return (per_word[word]["delta_ndcg"], per_word[word]["delta_all"], word)

    pruning: list[dict[str, Any]] = []
    out: dict[str, Any] = {"per_word": per_word, "pruning": pruning, "baseline": _summary(base, rule),
                           "base_measurement": base}
    if not keep:
        return {**out, "verdict": "not_adopted", "reason": "no word kept", "list": [], "clauses": {}}
    base_counts = _per_type_counts(base, metric_type)
    final: Measurement | None = None
    clauses: dict[str, Any] = {}
    while True:
        while keep and (lost := emptied(keep)):
            in_lost = {w for qid in lost for w in content_tokens(queries[qid], stop) if w in keep}
            word = min(in_lost, key=drop_key)
            keep.remove(word)
            pruning.append({"word": word, "reason": "empties", "queries": lost})
        if not keep:
            break
        final = measure([frozenset(keep)])[0]
        clauses = {"a": clause_a_holds(final["nfcorpus"], rule),
                   "b": clause_b_holds(_per_type_counts(final, metric_type), base_counts, rule)}
        if clauses["a"] and clauses["b"]:
            break
        word = min(keep, key=drop_key)
        keep.remove(word)
        pruning.append({"word": word, "reason": "clause a or b",
                        "failed": [name for name, ok in clauses.items() if not ok]})
        final = None
    if not keep or final is None:
        return {**out, "verdict": "not_adopted", "reason": "K emptied by pruning", "list": [], "clauses": clauses}
    combined = combined_clause_c(_diffs(final, base, metric_all), rule)
    clauses["c"] = combined["holds"]
    return {
        **out,
        "verdict": "adopted" if combined["holds"] else "not_adopted",
        "reason": "all clauses hold" if combined["holds"] else "clause c fails",
        "list": sorted(keep),
        "clauses": clauses,
        "combined": combined,
        "final": _summary(final, rule),
        "final_measurement": final,
    }


def stoplist_verdict(config: Mapping[str, Any], result: Mapping[str, Any], inputs: Mapping[str, Any]) -> dict[str, Any]:
    """The verdict file: the rule's outcome, its per-word table, combined metrics, and its inputs."""
    combined_metrics: dict[str, Any] = {"baseline": result["baseline"]}
    if "final" in result:
        combined_metrics["final"] = result["final"]
    verdict = {
        "schema_version": STOPLIST_VERDICT_SCHEMA,
        "verdict": result["verdict"],
        "reason": result["reason"],
        "words": list(result["list"]),
        "engine_stopwords_count": len(config["engine_stopwords"]),
        "rule": config["rule"]["text"],
        "per_word": result["per_word"],
        "pruning": result["pruning"],
        "clauses": result["clauses"],
        "combined": result.get("combined"),
        "combined_metrics": combined_metrics,
        "inputs": dict(inputs),
    }
    final = result.get("final_measurement")
    if final is not None and "ids" in final:
        verdict["simulated_fts_ids"] = final["ids"]
    return verdict


# --- H1 measurement (real stores) -------------------------------------------------------


def score_questions(ids_by_question: Mapping[str, Sequence[str]], gold: Mapping[str, Mapping[str, Any]],
                    types: Mapping[str, str], *, answer_ids: Iterable[str]) -> dict[str, dict[str, Any]]:
    """Scored (non-abstention) questions with their subset and session metrics at k = 5 and 10."""
    answer = set(answer_ids)
    out = {}
    for qid, ids in ids_by_question.items():
        if gold[qid]["abstention"]:
            continue
        scores = comp03_retrieval.question_scores(ids, gold[qid], SCORE_KS)
        out[qid] = {"question_type": types[qid], "subset": "answer_set" if qid in answer else "selection",
                    **{k: v for k, v in scores.items() if k.startswith("session_")}}
    return out


def _label(words: frozenset[str]) -> str:
    return "+".join(sorted(words)) or "<d3>"


def _write_json(path: Path, value: Any) -> None:
    comp03_answer._write_json(path, value)


def _read_json(path: Path) -> Any:
    return json.loads(Path(path).read_text(encoding="utf-8"))


class StoreMeasure:
    """Runs word lists over persistent per-question ``fathomdb-fts`` stores and one NFCorpus store."""

    def __init__(self, config: Mapping[str, Any], *, items: Sequence[Mapping[str, Any]], nfcorpus: Path,
                 attempt_root: Path, answer: Sequence[str]):
        self.config, self.root = config, attempt_root
        self.stop = config["engine_stopwords"]
        self.arm = comp_fts.ARMS[config["arm"]["product_arm"]]
        self.top_k = config["arm"]["top_k"]
        self.cases = [comp03_answer.convert_question(item) for item in items]
        self.gold = {item["question_id"]: comp03_answer.gold_for(item) for item in items}
        self.types = {case["question_id"]: case["question_type"] for case in self.cases}
        self.answer = list(answer)
        self.nfc_docs = _read_json(nfcorpus / "corpus.json")
        self.nfc_queries = _read_json(nfcorpus / "queries.json")
        self.nfc_gold = _read_json(nfcorpus / "gold.json")

    def queries(self) -> dict[str, str]:
        out = {f"lme:{case['question_id']}": case["question"] for case in self.cases}
        out.update({f"nfc:{q['id']}": q["text"] for q in self.nfc_queries})
        return out

    def _store(self, name: str, docs: list[dict[str, str]]) -> Path:
        root = self.root / "stores" / name
        if not (root / "built.json").exists():
            shutil.rmtree(root, ignore_errors=True)
            root.mkdir(parents=True)
            build = self.arm.build(root, docs)
            _write_json(root / "built.json", {"documents": len(docs), **build})
        return root

    def _run(self, store: Path, texts: Mapping[str, str], lists: Sequence[frozenset[str]],
             cache_path: Path) -> dict[str, dict[str, list[str]]]:
        """``{label: {query id: ids}}``, resumed from ``cache_path``."""
        cache = _read_json(cache_path) if cache_path.exists() else {}
        todo = [words for words in lists if _label(words) not in cache]
        if todo:
            handle = self.arm.open(store)
            try:
                for words in todo:
                    cache[_label(words)] = {
                        qid: self.arm.query(handle, simulated_query(text, words, self.stop), limit=self.top_k)
                        for qid, text in texts.items()}
            finally:
                self.arm.close(handle)
            _write_json(cache_path, cache)
        return {_label(words): cache[_label(words)] for words in lists}

    def __call__(self, lists: Sequence[frozenset[str]]) -> list[Measurement]:
        ids: dict[str, dict[str, list[str]]] = {_label(w): {} for w in lists}
        for index, case in enumerate(self.cases):
            qid = case["question_id"]
            docs = [{"id": d["id"], "body": d["body"]} for d in case["docs"] if d["body"].strip()]
            store = self._store(qid, docs)
            got = self._run(store, {qid: case["question"]}, lists, self.root / "questions" / f"{qid}.json")
            for label, by_q in got.items():
                ids[label][qid] = by_q[qid]
            if (index + 1) % 100 == 0:
                print(f"  [{index + 1}/{len(self.cases)}] {len(lists)} list(s)", flush=True)
        nfc_store = self._store("nfcorpus", self.nfc_docs)
        nfc = self._run(nfc_store, {q["id"]: q["text"] for q in self.nfc_queries}, lists,
                        self.root / "nfcorpus.json")
        out = []
        for words in lists:
            label = _label(words)
            scored = [comp_fts.score_query(nfc[label][q["id"]], self.nfc_gold[q["id"]]) for q in self.nfc_queries]
            out.append({
                "words": sorted(words),
                "ids": ids[label],
                "questions": score_questions(ids[label], self.gold, self.types, answer_ids=self.answer),
                "nfcorpus": {m: statistics.fmean(s[m] for s in scored) for m in ("ndcg_at_10", "mrr_at_10")},
            })
        print(f"measured {', '.join(_label(w) for w in lists[:3])}{' …' if len(lists) > 3 else ''}", flush=True)
        return out


def _sha_files(directory: Path, names: Iterable[str]) -> dict[str, str]:
    return {name: comp03_answer.sha256_file(directory / name) for name in sorted(names)}


def _reference_agreement(base_ids: Mapping[str, Sequence[str]], reference_sweep: Path, arm: str) -> dict[str, Any]:
    """How many D3-baseline top-k lists equal the final-head sweep's for the same arm (a store-build check)."""
    equal, differ = 0, []
    for qid, ids in sorted(base_ids.items()):
        path = reference_sweep / "questions" / qid / arm / "retrieval.json"
        if _read_json(path)["ids"] == list(ids):
            equal += 1
        else:
            differ.append(qid)
    return {"reference_sweep": str(reference_sweep), "questions": len(base_ids), "equal": equal, "differ": differ}


def run_stoplist(config: dict[str, Any], *, dataset_path: Path, nfcorpus: Path, attempt_root: Path,
                 engine: Mapping[str, str], runner: str, reference_sweep: Path | None = None) -> dict[str, Any]:
    if comp03_answer.sha256_file(dataset_path) != config["corpus"]["sha256"]:
        raise Comp03UnitHError("dataset digest does not match the config")
    nfc_files = _sha_files(nfcorpus, config["nfcorpus"]["files"])
    if nfc_files != config["nfcorpus"]["files"]:
        raise Comp03UnitHError("NFCorpus input digests do not match the config")
    items = sorted(_read_json(dataset_path), key=lambda item: item["question_id"])
    answer = answer_ids(config)
    attempt_root.mkdir(parents=True, exist_ok=True)
    _write_json(attempt_root / "config.resolved.json", config)
    measure = StoreMeasure(config, items=items, nfcorpus=nfcorpus, attempt_root=attempt_root, answer=answer)
    result = select_stoplist(config, measure, measure.queries())
    base = result["base_measurement"]
    counts = {
        "scored": len(base["questions"]),
        "selection": sum(r["subset"] == "selection" for r in base["questions"].values()),
        "answer_set_scored": sum(r["subset"] == "answer_set" for r in base["questions"].values()),
        "nfcorpus_queries": len(measure.nfc_queries),
    }
    if counts != config["selection"]["expected"]:
        raise Comp03UnitHError(f"question counts {counts} differ from the config's {config['selection']['expected']}")
    summary = {
        "schema_version": "comp-03.stoplist-selection.summary.v1",
        "verdict": result["verdict"],
        "list": result["list"],
        "baseline_ids": base["ids"],
        "final_ids": (result.get("final_measurement") or {}).get("ids"),
        "baseline_nfcorpus": base["nfcorpus"],
    }
    _write_json(attempt_root / "summary.json", summary)
    inputs = {
        "dataset_sha256": config["corpus"]["sha256"],
        "nfcorpus_files": nfc_files,
        "answer_config": config["answer_set"]["config"],
        "answer_config_sha256": comp03_answer.sha256_file(REPO_ROOT / config["answer_set"]["config"]),
        "engine": dict(engine),
        "questions": counts,
        "runner": runner,
        "attempt_root": str(attempt_root),
        "summary_sha256": comp03_answer.sha256_file(attempt_root / "summary.json"),
        "self_checks": {
            "baseline_nfcorpus": base["nfcorpus"],
            "final_head_nfcorpus": {m: f["baseline"] for m, f in config["rule"]["nfcorpus_floor"].items()},
            "final_head_sweep_ids": (None if reference_sweep is None
                                     else _reference_agreement(base["ids"], reference_sweep, config["arm"]["id"])),
        },
    }
    return stoplist_verdict(config, result, inputs)


# --- H2 pool ----------------------------------------------------------------------


def run_pool(config: Mapping[str, Any], *, dataset_path: Path, attempt_root: Path,
             fathomdb_bin: str | None = None) -> dict[str, dict[str, Any]]:
    """Every question through ``comp03_answer.retrieve`` at ``top_k = P`` for every candidate P (resumable)."""
    if comp03_answer.sha256_file(dataset_path) != config["corpus"]["sha256"]:
        raise Comp03UnitHError("dataset digest does not match the config")
    items = sorted(_read_json(dataset_path), key=lambda item: item["question_id"])
    answer = set(answer_ids(config))
    product = config["arm"]["product_arm"]
    attempt_root.mkdir(parents=True, exist_ok=True)
    _write_json(attempt_root / "config.resolved.json", config)
    rows: dict[str, dict[str, Any]] = {}
    for index, item in enumerate(items):
        case = comp03_answer.convert_question(item)
        qid = case["question_id"]
        ids: dict[str, list[str]] = {}
        for pool in config["candidates"]:
            path = attempt_root / "questions" / qid / f"{product}-p{pool}" / "retrieval.json"
            if not path.exists():
                result = comp03_answer.retrieve(product, case, attempt_root / "work", top_k=pool,
                                                fathomdb_bin=fathomdb_bin, store_label=f"{product}-p{pool}")
                if result.get("vector_rows") != result["documents"]:
                    raise Comp03UnitHError(f"{qid}/p{pool}: incomplete vector coverage")
                comp03_retrieval._record_device(attempt_root, getattr(comp_fts.ARMS[product], "device", None))
                _write_json(path, result)
            ids[str(pool)] = _read_json(path)["ids"]
        rows[qid] = {"question_type": case["question_type"], "subset": "answer_set" if qid in answer else "selection",
                     "ids": ids}
        print(f"[{index + 1}/{len(items)}] {qid}", flush=True)
    return rows


def _pool_metrics(ks: Sequence[int]) -> list[str]:
    return [f"session_recall_{kind}_at_{k}" for k in ks for kind in ("any", "all")]


def pool_table(rows: Mapping[str, Mapping[str, Any]], gold: Mapping[str, Mapping[str, Any]], *,
               candidates: Sequence[int], ks: Sequence[int], subset: str = "selection") -> dict[str, Any]:
    """Counts per P of the first k of the limit-P list, overall and per type, over one subset's scored questions."""
    metrics = _pool_metrics(ks)
    mine = {qid: row for qid, row in rows.items() if row["subset"] == subset and not gold[qid]["abstention"]}
    table = {}
    for pool in candidates:
        scored = [{"question_type": row["question_type"],
                   **comp03_retrieval.question_scores(row["ids"][str(pool)], gold[qid], ks)}
                  for qid, row in sorted(mine.items())]
        table[str(pool)] = {
            "overall": {"n": len(scored), **{m: _count(scored, m) for m in metrics}},
            "per_type": {t: {"n": sum(r["question_type"] == t for r in scored),
                             **{m: _count([r for r in scored if r["question_type"] == t], m) for m in metrics}}
                         for t in sorted({r["question_type"] for r in scored})},
        }
    return table


def select_pool(config: Mapping[str, Any], table: Mapping[str, Any]) -> dict[str, Any]:
    """The largest candidate whose constrained counts are each at least the reference's minus the tolerance."""
    constraints = config["constraints"]
    tol = constraints["tolerance_questions"]
    ref = table[str(config["reference"])]
    excluded: dict[str, list[str]] = {}
    for pool in config["candidates"]:
        row = table[str(pool)]
        failed = [f"per_type {t} {m}" for t in sorted(ref["per_type"]) for m in constraints["per_type"]
                  if row["per_type"].get(t, {}).get(m, 0) < ref["per_type"][t][m] - tol]
        failed += [f"overall {m}" for m in constraints["overall"] if row["overall"][m] < ref["overall"][m] - tol]
        if failed:
            excluded[str(pool)] = failed
    passing = [pool for pool in config["candidates"] if str(pool) not in excluded]
    return {"pool": max(passing), "reference": config["reference"], "excluded": excluded}


def pool_verdict(config: Mapping[str, Any], rows: Mapping[str, Mapping[str, Any]],
                 gold: Mapping[str, Mapping[str, Any]], inputs: Mapping[str, Any]) -> dict[str, Any]:
    candidates, ks = config["candidates"], config["ks"]
    tables = {subset: pool_table(rows, gold, candidates=candidates, ks=ks, subset=subset)
              for subset in ("selection", "answer_set")}
    choice = select_pool(config, tables["selection"])
    depth = max(ks)
    return {
        "schema_version": POOL_VERDICT_SCHEMA,
        **choice,
        "rule": config["rule"],
        "tables": tables,
        "ids_depth": depth,
        "ids": {qid: {p: ids[:depth] for p, ids in row["ids"].items()} for qid, row in sorted(rows.items())},
        "inputs": dict(inputs),
    }


# --- CLI --------------------------------------------------------------------------


def _write_verdict(path: Path, verdict: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(verdict, indent=1, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("stoplist", "pool"):
        cmd = sub.add_parser(name)
        cmd.add_argument("config", type=Path)
        cmd.add_argument("--dataset", type=Path, required=True)
        cmd.add_argument("--attempt-root", type=Path, required=True)
        cmd.add_argument("--engine-commit", required=True, help="40-hex source commit of the installed fathomdb build")
        cmd.add_argument("--runner", required=True, help="who ran it and where, e.g. 'implementer, CPU'")
        cmd.add_argument("--out", type=Path, help="verdict path; defaults to the config's verdict_path")
        if name == "stoplist":
            cmd.add_argument("--nfcorpus-inputs", type=Path, required=True)
            cmd.add_argument("--reference-sweep", type=Path,
                             help="final-head sweep attempt root; its fathomdb-fts IDs check the D3 baseline stores")
        else:
            cmd.add_argument("--fathomdb-bin")
    args = parser.parse_args(argv)
    engine = comp_record.engine_identity(args.engine_commit)
    if args.command == "stoplist":
        config = load_stoplist_config(args.config)
        verdict = run_stoplist(config, dataset_path=args.dataset, nfcorpus=args.nfcorpus_inputs.expanduser(),
                               attempt_root=args.attempt_root.expanduser(), engine=engine, runner=args.runner,
                               reference_sweep=args.reference_sweep.expanduser() if args.reference_sweep else None)
        brief = {"verdict": verdict["verdict"], "words": verdict["words"], "reason": verdict["reason"]}
    else:
        config = load_pool_config(args.config)
        os.environ.setdefault("FATHOMDB_EMBED_DEVICE", "auto")
        attempt_root = args.attempt_root.expanduser()
        rows = run_pool(config, dataset_path=args.dataset, attempt_root=attempt_root, fathomdb_bin=args.fathomdb_bin)
        items = _read_json(args.dataset)
        gold = {item["question_id"]: comp03_answer.gold_for(item) for item in items}
        device_path = attempt_root / "device.json"
        inputs = {"dataset_sha256": config["corpus"]["sha256"], "engine": engine, "runner": args.runner,
                  "answer_config": config["answer_set"]["config"],
                  "device": _read_json(device_path) if device_path.exists() else None,
                  "attempt_root": str(attempt_root),
                  "questions": {"total": len(rows), "selection_scored": sum(
                      r["subset"] == "selection" and not gold[q]["abstention"] for q, r in rows.items()),
                      "answer_set_scored": sum(
                      r["subset"] == "answer_set" and not gold[q]["abstention"] for q, r in rows.items())}}
        _write_json(attempt_root / "summary.json", {"schema_version": "comp-03.vector-pool.summary.v1",
                                                    "ids": {q: r["ids"] for q, r in rows.items()}})
        inputs["summary_sha256"] = comp03_answer.sha256_file(attempt_root / "summary.json")
        verdict = pool_verdict(config, rows, gold, inputs)
        brief = {"pool": verdict["pool"], "excluded": verdict["excluded"]}
    out = args.out or (REPO_ROOT / config["verdict_path"])
    _write_verdict(out, verdict)
    print(json.dumps({**brief, "verdict_file": str(out)}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
