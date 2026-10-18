"""Position-intervention probe P2 (see PREREG-P2.md).

Usage:
  python probe2.py build --db ROTATED.sqlite
  python probe2.py run --original ORIGINAL.sqlite --rotated ROTATED.sqlite
The original store is P1's (`probe.py build`); both are built in the same order.
"""

from __future__ import annotations

import argparse
import json
import random
import statistics
import time
from collections import Counter
from pathlib import Path

from tokenizers import Tokenizer

import size_corpora as sc
from cut import classify_span, cut_info
from probe import RESULTS, SEED, _drain, _recall

BOOT = 10_000
ARMS = ("text", "hybrid", "vector")


def rotate(lines: list[str], gold: set[int], to_front: bool) -> list[str]:
    g = [line for i, line in enumerate(lines) if i in gold]
    rest = [line for i, line in enumerate(lines) if i not in gold]
    return g + rest if to_front else rest + g


def _line(t: dict) -> str:
    return f"[{str(t.get('role', 'unknown')).capitalize()}]: {str(t.get('content', '')).strip()}"


def _spans(lines: list[str]) -> list[tuple[int, int]]:
    out, pos = [], 0
    for line in lines:
        out.append((pos, pos + len(line)))
        pos += len(line) + 1
    return out


def plan(data: Path, twins: bool = False) -> tuple[dict[str, str], dict[str, str], list[dict], dict]:
    """Original bodies, rotated bodies, eligible questions, exclusion counts.

    `twins` applies the P2b amendment: body-identical sessions move together and
    count as gold (`PREREG-P2b.md`).
    """
    qs = sc._lme(data)
    tok = Tokenizer.from_file(str(sc.DEFAULT_TOKENIZER))
    tok.no_truncation()

    def cut_of(body: str) -> int | None:
        return cut_info(body, tok.encode(body, add_special_tokens=False).offsets).cut_char

    original: dict[str, str] = {}
    lines_of: dict[str, list[str]] = {}
    for q in qs:
        for sid, turns in zip(q["haystack_session_ids"], q["haystack_sessions"]):
            if sid not in original:
                lines_of[sid] = [_line(t) for t in turns]
                original[sid] = "\n".join(lines_of[sid])
    group_of: dict[str, list[str]] = {}
    for sid, body in original.items():
        group_of.setdefault(body, []).append(sid)

    def key(sid: str) -> str:
        return original[sid] if twins else sid

    def members(sid: str) -> list[str]:
        return group_of[original[sid]] if twins else [sid]

    cuts = {sid: c for sid, c in zip(original, [
        cut_info(b, e.offsets).cut_char
        for b, e in zip(original.values(), tok.encode_batch(list(original.values()), add_special_tokens=False))
    ])}

    candidates = []
    for q in qs:
        gold_turns: dict[str, set[int]] = {}
        for sid, turns in zip(q["haystack_session_ids"], q["haystack_sessions"]):
            if sid in q["answer_session_ids"]:
                idx = {i for i, t in enumerate(turns) if t.get("has_answer")}
                if idx:
                    gold_turns[sid] = idx
        if not gold_turns:
            continue
        kinds = {
            classify_span(_spans(lines_of[sid])[i], cuts[sid]) for sid, idx in gold_turns.items() for i in idx
        }
        stratum = "B" if kinds == {"before"} else "P" if kinds == {"after"} else None
        if stratum:
            candidates.append({"q": q, "stratum": stratum, "gold_turns": gold_turns})

    excluded = Counter()
    rotated_lines: dict[str, list[str]] = {}
    owner: dict[str, tuple[str, frozenset[int]]] = {}
    eligible = []
    for c in candidates:
        target = "before" if c["stratum"] == "P" else "after"
        new: dict[str, list[str]] = {}
        ok = True
        for sid, idx in c["gold_turns"].items():
            prior = owner.get(key(sid))
            if prior and prior != (c["stratum"], frozenset(idx)):
                ok = False
                excluded["gold session claimed by another question"] += 1
                break
            new_lines = rotate(lines_of[sid], idx, to_front=c["stratum"] == "P")
            body = "\n".join(new_lines)
            new_idx = range(len(idx)) if c["stratum"] == "P" else range(len(new_lines) - len(idx), len(new_lines))
            spans = _spans(new_lines)
            if any(classify_span(spans[i], cut_of(body)) != target for i in new_idx):
                ok = False
                excluded["gold not moved across the cut"] += 1
                break
            new[sid] = new_lines
        if not ok:
            continue
        for sid, idx in c["gold_turns"].items():
            owner[key(sid)] = (c["stratum"], frozenset(idx))
        rotated_lines.update(new)
        eligible.append(c)

    # Drop questions whose gold sessions are also gold for the other stratum.
    strata_of: dict[str, set[str]] = {}
    for c in eligible:
        for sid in c["gold_turns"]:
            strata_of.setdefault(key(sid), set()).add(c["stratum"])
    final = []
    for c in eligible:
        if any(len(strata_of[key(sid)]) > 1 for sid in c["gold_turns"]):
            excluded["gold session shared across strata"] += 1
            continue
        final.append(c)
    # Twins share lines, so a group member takes its gold sibling's reordering.
    source = {m: sid for c in final for sid in c["gold_turns"] for m in members(sid)}
    keep = set(source)
    rotated = {sid: ("\n".join(rotated_lines[source[sid]]) if sid in keep else body) for sid, body in original.items()}
    questions = [{
        "qid": c["q"]["question_id"], "type": c["q"]["question_type"], "stratum": c["stratum"],
        "question": c["q"]["question"],
        "gold": sorted({m for g in c["q"]["answer_session_ids"] if g in original for m in members(g)}),
    } for c in final]
    counts = {
        "candidates": len(candidates), "eligible": {s: sum(q["stratum"] == s for q in questions) for s in "BP"},
        "excluded": dict(excluded), "rotated_sessions": len(keep),
    }
    return original, rotated, questions, counts


def _tag(twins: bool) -> str:
    return "probe2b" if twins else "probe2"


def build(db: Path, data: Path, twins: bool) -> None:
    from fathomdb.engine import Engine

    _, rotated, _, counts = plan(data, twins)
    if db.exists():
        raise SystemExit(f"{db} exists; probe builds a fresh store")
    db.parent.mkdir(parents=True, exist_ok=True)
    engine = Engine.open(str(db), use_default_embedder=True)
    engine._native._configure_vector_kind_for_test("doc")
    t0 = time.time()
    items = list(rotated.items())
    for start in range(0, len(items), 500):
        engine.write([{"kind": "doc", "body": b, "source_id": sid} for sid, b in items[start:start + 500]])
    embedded = _drain(db, engine, len(items))
    meta = {**counts, "docs": len(items), "embed_wall_s": round(time.time() - t0, 1),
            "coverage": embedded / len(items)}
    (RESULTS / f"{_tag(twins)}_build.json").write_text(json.dumps(meta, indent=2) + "\n")
    print(json.dumps(meta, indent=2))


def _measure(engine, q: dict) -> dict:
    text = [h.source_id for h in engine.search_text_only(q["question"], limit=10).results]
    hybrid = [h.source_id for h in engine.search(q["question"], limit=10).results]
    res = engine.search(q["question"], limit=100, explain=True)
    ids = [h.source_id for h in res.results]
    vector = [ids[i] for _, i in sorted((p.vector_rank, i) for i, p in enumerate(res.explanation.per_hit)
                                         if p.vector_rank is not None)]
    out = {f"{a}@10": _recall(r, q["gold"], 10) for a, r in (("text", text), ("hybrid", hybrid), ("vector", vector))}
    out["vector_ranked"] = len(vector)
    return out


def run(original: Path, rotated: Path, data: Path, twins: bool) -> None:
    from fathomdb.engine import Engine

    _, _, questions, _ = plan(data, twins)
    eng = {"original": Engine.open(str(original), use_default_embedder=True),
           "rotated": Engine.open(str(rotated), use_default_embedder=True)}
    rows = []
    for q in questions:
        m = {name: _measure(e, q) for name, e in eng.items()}
        vis, hid = ("rotated", "original") if q["stratum"] == "P" else ("original", "rotated")
        rows.append({
            "qid": q["qid"], "type": q["type"], "stratum": q["stratum"],
            "measured": min(m["original"]["vector_ranked"], m["rotated"]["vector_ranked"]) >= 10,
            "visible": m[vis], "hidden": m[hid],
        })
    with (RESULTS / f"{_tag(twins)}_per_question.jsonl").open("w") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    summary = paired_summary(rows)
    (RESULTS / f"{_tag(twins)}_summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


def paired_summary(rows: list[dict]) -> dict:
    meas = [r for r in rows if r.get("measured")]

    def d(r: dict, arm: str) -> int:
        return r["visible"][f"{arm}@10"] - r["hidden"][f"{arm}@10"]

    rng = random.Random(SEED)
    out: dict = {"n": len(meas), "unmeasured": len(rows) - len(meas),
                 "n_by_stratum": dict(Counter(r["stratum"] for r in meas))}
    for arm in ARMS:
        point = statistics.fmean(d(r, arm) for r in meas)
        boots = sorted(statistics.fmean(d(rng.choice(meas), arm) for _ in meas) for _ in range(BOOT))
        se = statistics.pstdev(boots)
        out[f"D({arm})"] = {"point": point, "ci95": [boots[int(0.025 * BOOT)], boots[int(0.975 * BOOT)]],
                            "se": se, "mde80": 2.8 * se}
        out[f"recall_{arm}"] = {
            "visible": statistics.fmean(r["visible"][f"{arm}@10"] for r in meas),
            "hidden": statistics.fmean(r["hidden"][f"{arm}@10"] for r in meas),
        }
    for key, group in (("by_stratum", "stratum"), ("by_type", "type")):
        out[key] = {
            g: {"n": len(sub), **{f"D({a})": statistics.fmean(d(r, a) for r in sub) for a in ARMS}}
            for g in sorted({r[group] for r in meas})
            for sub in [[r for r in meas if r[group] == g]]
        }
    agree = statistics.fmean(int(r["visible"]["text@10"] == r["hidden"]["text@10"]) for r in meas)
    ok = abs(out["D(text)"]["point"]) <= 0.01 and agree >= 0.98
    out["validity"] = {"text_agreement": agree, "ok": ok}
    if not ok:
        out["rule"] = None
        return out
    dv, dh = out["D(vector)"], out["D(hybrid)"]
    shown = dh["point"] >= 0.03 and dh["ci95"][0] > 0
    out["rule"] = {
        "vector_failure_shown": dv["point"] >= 0.10 and dv["ci95"][0] > 0,
        "product_failure_shown": shown,
        "hybrid_underpowered": (not shown) and dh["mde80"] > 0.03,
    }
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["build", "run", "plan"])
    ap.add_argument("--db", type=Path)
    ap.add_argument("--original", type=Path)
    ap.add_argument("--rotated", type=Path)
    ap.add_argument("--data", type=Path, default=sc.DEFAULT_DATA)
    ap.add_argument("--twins", action="store_true", help="P2b amendment (PREREG-P2b.md)")
    a = ap.parse_args()
    if a.cmd == "plan":
        print(json.dumps(plan(a.data, a.twins)[3], indent=2))
    elif a.cmd == "build":
        build(a.db, a.data, a.twins)
    else:
        run(a.original, a.rotated, a.data, a.twins)


if __name__ == "__main__":
    main()
