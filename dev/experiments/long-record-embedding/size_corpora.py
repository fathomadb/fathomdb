"""Size the long-record problem: wordpiece lengths and gold position vs the 512 cut.

Rebuilds each corpus's record bodies exactly as the named harness does, tokenizes
them with the engine's pinned bge-small tokenizer (no truncation), and reports
the share of records over the cut, the share of bytes past it, and where gold
evidence falls. Writes results/sizing.json and prints a Markdown table.

Usage: python size_corpora.py [--data DIR] [--tokenizer PATH]
"""

from __future__ import annotations

import argparse
import json
import re
import statistics
import zipfile
from collections import defaultdict
from pathlib import Path

from tokenizers import Tokenizer

from cut import cut_info, classify_span

DEFAULT_DATA = Path("/home/coreyt/projects/fathomdb/data/corpus-data")
DEFAULT_TOKENIZER = Path.home() / ".cache/fathomdb/embedders/0b2926f8a9b1/tokenizer.json"
IRC_SOURCES = [
    "bahmutov_dailylogs", "chain_connectives", "cnn_dailymail", "enron", "enronqa",
    "landes_todos", "qaconv", "qasper", "qmsum", "synthetic_notes",
]


class Corpus:
    def __init__(self, name: str, harness: str):
        self.name = name
        self.harness = harness
        self.bodies: dict[str, str] = {}
        # gold item -> list of (record id, char span)
        self.gold: dict[str, list[tuple[str, tuple[int, int]]]] = defaultdict(list)
        self.gold_note = ""

    def add_gold(self, item: str, rec: str, span: tuple[int, int]) -> None:
        self.gold[item].append((rec, span))


def _lines_with_spans(lines: list[str]) -> tuple[str, list[tuple[int, int]]]:
    spans, pos = [], 0
    for line in lines:
        spans.append((pos, pos + len(line)))
        pos += len(line) + 1
    return "\n".join(lines), spans


def memex(data: Path) -> Corpus:
    c = Corpus("memex-personal (doc)", "exp_cov_census.py:48 (body verbatim)")
    for line in (data / "external/memex-elps/personal.gold.jsonl").open():
        d = json.loads(line)
        c.bodies[d["doc_id"]] = d["body"]
        for i, e in enumerate(d.get("edges") or []):
            if e.get("char_span"):
                c.add_gold(f"{d['doc_id']}#{i}", d["doc_id"], tuple(e["char_span"]))
    c.gold_note = "edge evidence char_span"
    return c


def _lme(data: Path):
    return json.load((data / "raw/longmemeval-cleaned/longmemeval_s_cleaned.json").open())


def lme_session(data: Path, qs) -> Corpus:
    c = Corpus("LongMemEval-S (session)", "r2_parity_eval.py:980 _format_lme_session")
    for q in qs:
        for sid, turns in zip(q["haystack_session_ids"], q["haystack_sessions"]):
            lines = [f"[{str(t.get('role', 'unknown')).capitalize()}]: {str(t.get('content', '')).strip()}" for t in turns]
            body, spans = _lines_with_spans(lines)
            c.bodies[sid] = body
            if sid in q["answer_session_ids"]:
                for t, sp in zip(turns, spans):
                    if t.get("has_answer"):
                        c.add_gold(q["question_id"], sid, sp)
    c.gold_note = "has_answer turn line span"
    return c


def lme_turn(data: Path, qs) -> Corpus:
    c = Corpus("LongMemEval-S (turn)", "reason_01_preflight.py:190 _case_rows")
    for q in qs:
        for sid, turns in zip(q["haystack_session_ids"], q["haystack_sessions"]):
            for i, t in enumerate(turns):
                if not t.get("content"):
                    continue
                rid = f"{sid}-{i}"
                body = f"{t['role']}: {t['content']}"
                c.bodies[rid] = body
                if t.get("has_answer") and sid in q["answer_session_ids"]:
                    c.add_gold(q["question_id"], rid, (0, len(body)))
    c.gold_note = "has_answer turn = whole record"
    return c


def _locomo_sessions(data: Path):
    for conv_i, sample in enumerate(json.load((data / "raw/locomo10.json").open())):
        conv = sample["conversation"]
        nums = sorted(int(k.split("_")[1]) for k in conv if re.fullmatch(r"session_\d+", k))
        yield conv_i, sample, conv, nums


def locomo_session(data: Path) -> Corpus:
    c = Corpus("LoCoMo (session)", "locomo_loader.py:73 _session_body")
    dia_pos: dict[str, tuple[str, tuple[int, int]]] = {}
    for conv_i, sample, conv, nums in _locomo_sessions(data):
        for n in nums:
            lines, ids = [], []
            for t in conv[f"session_{n}"]:
                sp, tx = str(t.get("speaker", "")).strip(), str(t.get("text", "")).strip()
                if tx:
                    lines.append(f"{sp}: {tx}" if sp else tx)
                    ids.append(t["dia_id"])
            body, spans = _lines_with_spans(lines)
            rid = f"{sample['sample_id']}:session_{n}"
            c.bodies[rid] = body
            for d, s in zip(ids, spans):
                dia_pos[f"{conv_i}:{d}"] = (rid, s)
        for qi, qa in enumerate(sample["qa"]):
            for ev in qa.get("evidence") or []:
                for part in re.split(r"[;,\s]+", str(ev)):
                    m = re.fullmatch(r"D(\d+):0*(\d+)", part.strip())
                    if m and f"{conv_i}:D{m[1]}:{m[2]}" in dia_pos:
                        c.add_gold(f"{conv_i}-{qi}", *dia_pos[f"{conv_i}:D{m[1]}:{m[2]}"])
    c.gold_note = "evidence dia_id line span"
    return c


def locomo_turn(data: Path) -> Corpus:
    c = Corpus("LoCoMo (turn)", "fathomdb-mcp/examples/locomo_load.rs:34")
    for conv_i, sample, conv, nums in _locomo_sessions(data):
        for n in nums:
            date = conv.get(f"session_{n}_date_time") or ""
            for t in conv[f"session_{n}"]:
                body = f"[{date}] {t['speaker']}: {t['text']}" if date else f"{t['speaker']}: {t['text']}"
                if t.get("blip_caption"):
                    body += f" [shared image: {t['blip_caption']}]"
                c.bodies[f"{conv_i}:{t['dia_id']}"] = body
    c.gold_note = "turn records; gold = whole record (not positioned)"
    return c


_TAG_RE = re.compile(r"<[^>]+>")
_WS_RE = re.compile(r"\s+")


def _strip_html(text: str) -> str:
    return _WS_RE.sub(" ", _TAG_RE.sub(" ", text)).strip()


def apnews(data: Path) -> Corpus:
    c = Corpus("AP-News (article)", "apnews_corpus.py:128 _article_from_record")
    root = data / "raw/apnews_benchmarkqed"
    by_title: dict[str, str] = {}
    with zipfile.ZipFile(root / "raw_data.zip") as z:
        for name in z.namelist():
            if not name.endswith(".json"):
                continue
            rec = json.loads(z.read(name))
            doc_id = str((rec.get("altids") or {}).get("itemid") or Path(name).stem)
            title = str(rec.get("headline") or rec.get("title") or "")
            text = _strip_html(str(rec.get("body_nitf") or rec.get("body") or ""))
            c.bodies[doc_id] = f"{title}\n\n{text}".strip() if title else text
            by_title[_WS_RE.sub(" ", title).strip()] = doc_id
    found = missed = 0
    for kind in ("local", "linked"):
        for q in json.load((root / f"generated_questions_v2/data_{kind}_questions_assertions.json").open()):
            for claim in q.get("claims") or []:
                for src in claim.get("sources") or []:
                    raw = src["text"]
                    head = re.match(r"headline: (.*)\n", raw)
                    doc_id = by_title.get(_WS_RE.sub(" ", head[1]).strip()) if head else None
                    excerpt = _strip_html(re.sub(r"^(headline|firstcreated): .*\n", "", raw, flags=re.M))
                    pos = -1
                    if doc_id:
                        body = c.bodies[doc_id]
                        # Excerpts start mid-word; probe from the first space.
                        probe = excerpt[excerpt.find(" ") + 1:][:80]
                        pos = body.find(probe) if len(probe) >= 20 else -1
                    if pos < 0:
                        missed += 1
                        continue
                    found += 1
                    c.add_gold(f"{kind}:{q['question_id']}", doc_id, (pos, pos + len(excerpt)))
    c.gold_note = f"v2 local+linked claim excerpts located by substring ({found} located, {missed} not)"
    return c


def irc(data: Path) -> Corpus:
    c = Corpus("IR-C frozen corpus (doc)", "ir_c_fusion_experiment.rs:275 (body verbatim)")
    for src in IRC_SOURCES:
        for line in (data / f"raw/{src}.jsonl").open():
            d = json.loads(line)
            c.bodies[d["doc_id"]] = d["body"]
    found = 0
    for line in (data / "eval/qaconv_qa.jsonl").open():
        q = json.loads(line)
        for did in q["evidence_doc_ids"]:
            body = c.bodies.get(did, "")
            # First occurrence: the position most favourable to the truncated vector.
            hits = [body.find(a) for a in q["answers"] if a and body.find(a) >= 0]
            if hits:
                p = min(hits)
                a = next(a for a in q["answers"] if body.find(a) == p)
                c.add_gold(q["qa_id"], did, (p, p + len(a)))
                found += 1
    c.gold_note = f"qaconv exact answer substring, first occurrence ({found} located); enronqa/qmsum not positionable"
    return c


def _pct(xs: list[int], p: float) -> int:
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))]


def measure(c: Corpus, tok: Tokenizer) -> dict:
    ids = list(c.bodies)
    encs = tok.encode_batch([c.bodies[i] for i in ids], add_special_tokens=False)
    info = {i: cut_info(c.bodies[i], e.offsets) for i, e in zip(ids, encs)}
    toks = [v.tokens for v in info.values()]
    over = [v for v in info.values() if v.cut_char is not None]
    bytes_total = sum(v.bytes_total for v in info.values())
    out = {
        "corpus": c.name, "harness": c.harness, "records": len(ids),
        "tokens_p50": int(statistics.median(toks)), "tokens_p90": _pct(toks, 0.9),
        "tokens_p99": _pct(toks, 0.99), "tokens_max": max(toks),
        "share_records_over_cut": len(over) / len(ids),
        "share_bytes_past_cut": sum(v.bytes_past for v in info.values()) / bytes_total,
        "gold_note": c.gold_note,
    }
    if c.gold:
        spans = [(classify_span(s, info[r].cut_char), r) for g in c.gold.values() for r, s in g]
        items_all_after = sum(
            all(classify_span(s, info[r].cut_char) == "after" for r, s in g) for g in c.gold.values()
        )
        items_any_after = sum(
            any(classify_span(s, info[r].cut_char) != "before" for r, s in g) for g in c.gold.values()
        )
        out.update({
            "gold_items": len(c.gold), "gold_spans": len(spans),
            "share_gold_spans_after_cut": sum(k == "after" for k, _ in spans) / len(spans),
            "share_gold_spans_straddling": sum(k == "straddles" for k, _ in spans) / len(spans),
            "share_items_all_gold_after_cut": items_all_after / len(c.gold),
            "share_items_some_gold_not_before": items_any_after / len(c.gold),
        })
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", type=Path, default=DEFAULT_DATA)
    ap.add_argument("--tokenizer", type=Path, default=DEFAULT_TOKENIZER)
    args = ap.parse_args()
    tok = Tokenizer.from_file(str(args.tokenizer))
    tok.no_truncation()
    tok.no_padding()
    qs = _lme(args.data)
    corpora = [
        memex(args.data), lme_session(args.data, qs), lme_turn(args.data, qs),
        locomo_session(args.data), locomo_turn(args.data), apnews(args.data), irc(args.data),
    ]
    rows = [measure(c, tok) for c in corpora]
    out = Path(__file__).parent / "results"
    out.mkdir(exist_ok=True)
    (out / "sizing.json").write_text(json.dumps(rows, indent=2) + "\n")
    print("| corpus | records | p50 / p90 / p99 / max wp | >510 | bytes past | gold spans after / straddle | items all-after |")
    print("|---|---|---|---|---|---|---|")
    for r in rows:
        g = (f"{r['share_gold_spans_after_cut']:.1%} / {r['share_gold_spans_straddling']:.1%}"
             if "gold_spans" in r else "—")
        ia = f"{r['share_items_all_gold_after_cut']:.1%}" if "gold_items" in r else "—"
        print(f"| {r['corpus']} | {r['records']} | {r['tokens_p50']} / {r['tokens_p90']} / {r['tokens_p99']} / "
              f"{r['tokens_max']} | {r['share_records_over_cut']:.1%} | {r['share_bytes_past_cut']:.1%} | {g} | {ia} |")


if __name__ == "__main__":
    main()
