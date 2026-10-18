"""Truncation probe P1 (see PREREG.md): recall for gold before vs past the 512 cut.

Usage:
  FATHOMDB_EMBED_DEVICE=cuda:0 python probe.py build --db PATH
  FATHOMDB_EMBED_DEVICE=cuda:0 python probe.py run --db PATH
"""

from __future__ import annotations

import argparse
import json
import random
import time
from pathlib import Path

from tokenizers import Tokenizer

import size_corpora as sc
from cut import classify_span, cut_info

RESULTS = Path(__file__).parent / "results"
SEED = 20261018
BOOT = 10_000


def _strata(data: Path) -> tuple[list[dict], dict[str, str]]:
    qs = sc._lme(data)
    corpus = sc.lme_session(data, qs)
    tok = Tokenizer.from_file(str(sc.DEFAULT_TOKENIZER))
    tok.no_truncation()
    ids = list(corpus.bodies)
    encs = tok.encode_batch([corpus.bodies[i] for i in ids], add_special_tokens=False)
    cuts = {i: cut_info(corpus.bodies[i], e.offsets).cut_char for i, e in zip(ids, encs)}
    rows = []
    for q in qs:
        spans = corpus.gold.get(q["question_id"], [])
        kinds = {classify_span(s, cuts[r]) for r, s in spans}
        stratum = "B" if kinds == {"before"} else "P" if kinds == {"after"} else ("none" if not kinds else "mixed")
        rows.append({
            "qid": q["question_id"], "type": q["question_type"], "question": q["question"],
            "gold": list(q["answer_session_ids"]), "stratum": stratum,
        })
    return rows, corpus.bodies


def build(db: Path, data: Path) -> None:
    from fathomdb.engine import Engine

    from eval.p0a_base_retrieval import _drain_until_embedded

    _, bodies = _strata(data)
    if db.exists():
        raise SystemExit(f"{db} exists; probe builds a fresh store")
    db.parent.mkdir(parents=True, exist_ok=True)
    engine = Engine.open(str(db), use_default_embedder=True)
    engine._native._configure_vector_kind_for_test("doc")
    t0 = time.time()
    items = list(bodies.items())
    for start in range(0, len(items), 500):
        engine.write([{"kind": "doc", "body": b, "source_id": sid} for sid, b in items[start:start + 500]])
    t_write = time.time() - t0
    rep = _drain_until_embedded(db, engine, expected_docs=len(items))
    meta = {
        "docs": len(items), "write_s": round(t_write, 1), "embed_wall_s": round(time.time() - t0, 1),
        "coverage": rep.coverage, "open_report": repr(engine.open_report()),
    }
    RESULTS.mkdir(exist_ok=True)
    (RESULTS / "probe_build.json").write_text(json.dumps(meta, indent=2) + "\n")
    print(json.dumps(meta, indent=2))


def _recall(ranked: list[str], gold: list[str], k: int) -> int:
    return int(any(g in ranked[:k] for g in gold))


def run(db: Path, data: Path) -> None:
    from fathomdb.engine import Engine

    rows, _ = _strata(data)
    engine = Engine.open(str(db), use_default_embedder=True)
    out = []
    for r in rows:
        if r["stratum"] not in ("B", "P"):
            out.append({**r, "measured": False})
            continue
        t0 = time.perf_counter()
        text = [h.source_id for h in engine.search_text_only(r["question"], limit=10).results]
        t1 = time.perf_counter()
        res = engine.search(r["question"], limit=100, explain=True)
        t2 = time.perf_counter()
        hits = [h.source_id for h in res.results]
        per_hit = res.explanation.per_hit
        vec = sorted((p.vector_rank, hits[i]) for i, p in enumerate(per_hit) if p.vector_rank is not None)
        vector = [sid for _, sid in vec]
        rec = {**r, "measured": len(vector) >= 10, "text_ms": (t1 - t0) * 1e3, "hybrid100_ms": (t2 - t1) * 1e3}
        for name, ranked in (("text", text), ("hybrid", hits), ("vector", vector)):
            for k in (5, 10):
                rec[f"{name}@{k}"] = _recall(ranked, r["gold"], k)
        out.append(rec)
    RESULTS.mkdir(exist_ok=True)
    with (RESULTS / "probe_per_question.jsonl").open("w") as f:
        for rec in out:
            f.write(json.dumps({k: v for k, v in rec.items() if k != "question"}) + "\n")
    summary = summarize(out)
    (RESULTS / "probe_summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


def _mean(xs: list[int]) -> float:
    return sum(xs) / len(xs) if xs else float("nan")


def summarize(out: list[dict]) -> dict:
    meas = [r for r in out if r.get("measured")]
    strata = {s: [r for r in meas if r["stratum"] == s] for s in ("B", "P")}
    summary: dict = {
        "n": {s: len(v) for s, v in strata.items()},
        "excluded": {s: sum(r["stratum"] == s for r in out) for s in ("mixed", "none")},
        "unmeasured": sum(1 for r in out if r["stratum"] in ("B", "P") and not r.get("measured")),
        "recall": {},
    }
    for arm in ("text", "hybrid", "vector"):
        for k in (5, 10):
            key = f"{arm}@{k}"
            summary["recall"][key] = {s: _mean([r[key] for r in v]) for s, v in strata.items()}
    rng = random.Random(SEED)

    def effect(sample: dict[str, list[dict]], arm: str) -> float:
        def gap(a: str) -> float:
            return _mean([r[f"{a}@10"] for r in sample["B"]]) - _mean([r[f"{a}@10"] for r in sample["P"]])

        return gap(arm) - gap("text")

    for arm in ("vector", "hybrid"):
        point = effect(strata, arm)
        boots = sorted(
            effect({s: [rng.choice(v) for _ in v] for s, v in strata.items()}, arm) for _ in range(BOOT)
        )
        summary[f"E({arm})"] = {"point": point, "ci95": [boots[int(0.025 * BOOT)], boots[int(0.975 * BOOT)]]}
    for arm in ("text", "vector", "hybrid"):
        summary[f"gap({arm})"] = summary["recall"][f"{arm}@10"]["B"] - summary["recall"][f"{arm}@10"]["P"]
    ev, eh = summary["E(vector)"], summary["E(hybrid)"]
    summary["rule"] = {
        "vector_failure_shown": ev["point"] >= 0.10 and ev["ci95"][0] > 0,
        "product_failure_shown": eh["point"] >= 0.03 and eh["ci95"][0] > 0,
    }
    lat = [r["hybrid100_ms"] for r in meas]
    summary["hybrid100_ms_p50_p95"] = [sorted(lat)[len(lat) // 2], sorted(lat)[int(0.95 * len(lat))]]
    return summary


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["build", "run"])
    ap.add_argument("--db", type=Path, required=True)
    ap.add_argument("--data", type=Path, default=sc.DEFAULT_DATA)
    a = ap.parse_args()
    (build if a.cmd == "build" else run)(a.db, a.data)


if __name__ == "__main__":
    main()
