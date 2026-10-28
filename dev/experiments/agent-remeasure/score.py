"""Audit and judge agent-remeasure runs.

``score.py --out DIR RUN_DIR [RUN_DIR ...]`` reads each run's checkpoint and
stream-json transcripts, audits tool use per question, judges every answer
twice with gpt-4o-mini (the harness's evidence-aware LOCOMO judge = lenient,
plus a strict rubric), caches verdicts in ``<run>/judged.jsonl``, and writes
``DIR/summary.json`` and ``DIR/questions.jsonl``. Secrets come from the eval
env file and are never printed.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import random
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO))

FATHOM = "mcp__fathomdb__"
HARNESS = Path("/home/coreyt/.local/share/fathomdb-experiments/locomo-capability-20260814/memory-benchmarks")
ENV_FILE = Path("/home/coreyt/projects/fathomdb/dev/.env.eval")
DATASET = Path("/home/coreyt/projects/fathomdb/data/corpus-data/raw/locomo10.json")
SEED = 20261019
RESAMPLES = 10_000
_REF_LINE = re.compile(r"^(l:\S+) ")

STRICT_PROMPT = """Label the generated answer as CORRECT or WRONG under a STRICT rubric.

Question: {question}
Gold answer: {gold}
Evidence (conversation turns that contain the answer):
{evidence}

Generated answer: {response}

Rules:
1. CORRECT only if the generated answer states the gold answer's fact (paraphrase allowed). If the gold answer lists several items, every item must be present.
2. WRONG if the answer is partial, hedged between alternatives, says it does not know, or adds any fact that contradicts the gold answer or the evidence.
3. Dates must match the gold answer at the gold answer's granularity (a relative time correctly resolved to the same date is fine; a different day, month or year is WRONG).
4. Do not give credit because the evidence could support a different answer: judge against the gold answer.

Return JSON only: {{"reasoning": "<one sentence>", "label": "CORRECT" or "WRONG"}}"""


# --- transcript audit (pure; unit-tested) ------------------------------------


def _text_of(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "".join(part.get("text", "") for part in content if isinstance(part, dict))
    return ""


def cut_refs_in_result(text: str) -> tuple[list[str], list[str]]:
    """``(refs shown, refs shown with snippet_cut)`` of one search result, JSON or compact text."""
    try:
        data = json.loads(text)
    except (json.JSONDecodeError, TypeError):
        data = None
    if isinstance(data, dict) and isinstance(data.get("results"), list):
        refs = [hit.get("ref") for hit in data["results"]]
        return refs, [hit.get("ref") for hit in data["results"] if hit.get("snippet_cut")]
    refs, cut = [], []
    for line in text.splitlines():
        match = _REF_LINE.match(line)
        if not match:
            continue
        meta = line.split(" | ", 1)[0]
        refs.append(match.group(1))
        if " [cut]" in meta:
            cut.append(match.group(1))
    return refs, cut


def is_busy(text: str) -> bool:
    """Whether a tool result is a ``busy`` error (``error.class``); substring fallback when not JSON."""
    try:
        data = json.loads(text)
    except (json.JSONDecodeError, TypeError):
        return '"busy"' in text or "busy/" in text or "busy:" in text
    error = data.get("error") if isinstance(data, dict) else None
    return isinstance(error, dict) and error.get("class") == "busy"


def final_result_text(lines) -> str:
    """The ``result`` text of the transcript's final result record ("" when absent)."""
    text = ""
    for raw in lines:
        try:
            rec = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if isinstance(rec, dict) and rec.get("type") == "result":
            text = rec.get("result") or ""
    return text


def graded_answer(row: dict, result_text: str) -> str:
    """The answer to judge: the parsed JSON answer, else the final reply minus the INSTRUCTIONS_SEEN line."""
    if row.get("answer_json") is not None:
        return row.get("answer") or ""
    kept = [line for line in result_text.splitlines()
            if not line.strip().strip("`").upper().startswith("INSTRUCTIONS_SEEN:")]
    return "\n".join(kept).strip()


def answer_key(qid: str, answer: str) -> str:
    return f"{qid}:{hashlib.sha1(answer.encode()).hexdigest()}"


def audit_transcript(lines) -> dict:
    """Per-question tool and token audit of one stream-json transcript."""
    uses: dict[str, dict] = {}
    calls = collections.Counter()
    off_policy: list[str] = []
    result_bytes = collections.Counter()
    result_counts = collections.Counter()
    errors = busy = 0
    shown_cut: list[str] = []
    fetched: list[str] = []
    seen_msgs: dict[str, dict] = {}
    final: dict = {}
    init: dict = {}
    for raw in lines:
        try:
            rec = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if rec.get("type") == "system" and rec.get("subtype") == "init":
            init = rec
        if rec.get("type") == "result":
            final = rec
        message = rec.get("message") or {}
        content = message.get("content")
        if rec.get("type") == "assistant" and message.get("id") and message.get("usage"):
            seen_msgs[message["id"]] = message["usage"]
        if not isinstance(content, list):
            continue
        for part in content:
            if not isinstance(part, dict):
                continue
            if part.get("type") == "tool_use":
                name = part.get("name", "")
                uses[part.get("id")] = part
                if name.startswith(FATHOM):
                    short = name[len(FATHOM):]
                    calls[short] += 1
                    if short == "fetch":
                        fetched.extend((part.get("input") or {}).get("refs") or [])
                else:
                    off_policy.append(name)
            elif part.get("type") == "tool_result":
                use = uses.get(part.get("tool_use_id")) or {}
                short = use.get("name", "?").removeprefix(FATHOM)
                text = _text_of(part.get("content"))
                result_bytes[short] += len(text.encode())
                result_counts[short] += 1
                if part.get("is_error"):
                    errors += 1
                if is_busy(text):
                    busy += 1
                if short == "search" and not part.get("is_error"):
                    shown_cut.extend(cut_refs_in_result(text)[1])
    cut_set = set(shown_cut)
    usage = final.get("usage") or {}
    total_bytes = sum(result_bytes.values())
    return {
        "tool_calls": dict(calls), "searches": calls["search"], "fetches": calls["fetch"],
        "off_policy": off_policy, "invalid": bool(off_policy),
        "cut_refs_shown": len(cut_set), "cut_refs_fetched": len(cut_set & set(fetched)),
        "result_bytes": dict(result_bytes), "result_bytes_total": total_bytes,
        "result_tokens_est": total_bytes / 4,
        "search_bytes_per_call": result_bytes["search"] / result_counts["search"] if result_counts["search"] else None,
        "tool_errors": errors, "busy_results": busy, "api_calls": len(seen_msgs),
        "input_tokens": usage.get("input_tokens"), "output_tokens": usage.get("output_tokens"),
        "cache_read_tokens": usage.get("cache_read_input_tokens"),
        "cache_create_tokens": usage.get("cache_creation_input_tokens"),
        "cost_usd": final.get("total_cost_usd"), "num_turns": final.get("num_turns"),
        "result_subtype": final.get("subtype"), "init_tools": init.get("tools"),
        "mcp_status": [server.get("status") for server in init.get("mcp_servers") or []],
    }


# --- statistics ---------------------------------------------------------------


def bootstrap(values: list[float]) -> dict:
    if not values:
        return {"n": 0, "mean": None, "ci95": None}
    rng = random.Random(SEED)
    n = len(values)
    means = sorted(sum(rng.choices(values, k=n)) / n for _ in range(RESAMPLES))
    return {"n": n, "mean": sum(values) / n, "ci95": [means[int(0.025 * RESAMPLES)], means[int(0.975 * RESAMPLES) - 1]]}


# --- judging (live) -----------------------------------------------------------


class Judges:
    def __init__(self, ledger_dir: Path) -> None:
        from experiments import mcp_agent_loop as mal

        self.mal = mal
        self.prompts = mal.load_harness_prompts(HARNESS)
        secrets = mal.read_env_file(ENV_FILE)
        ledger = mal.CostLedger(ledger_dir, cap_usd=20.0)
        self.client = mal.ChatClient(secrets["R2_JUDGE_BASE_URL"], secrets["R2_JUDGE_API_KEY"], ledger, arm="judge")
        self.conv: dict[int, tuple] = {}

    def _conv(self, index: int):
        if index not in self.conv:
            questions, conversation = self.mal.load_questions(DATASET, index)
            self.conv[index] = ({q.qid: q for q in questions}, self.mal.Grader(self.prompts, conversation, self.client))
        return self.conv[index]

    def judge(self, conv: int, qid: str, answer: str) -> dict:
        questions, grader = self._conv(conv)
        question = questions[f"conv{conv}_{qid}"]
        lenient = grader.judge(question, answer, [])
        evidence = "\n".join(grader.lookup[ref] for ref in question.evidence if ref in grader.lookup)
        prompt = STRICT_PROMPT.format(question=question.question, gold=question.answer, evidence=evidence or "(none)",
                                      response=answer)
        parsed: dict = {}
        for _ in range(3):
            turn = self.client.complete(qid=qid, role="strict-judge", model=self.mal.JUDGE_MODEL, json_object=True,
                                        messages=[{"role": "system", "content": self.prompts.JUDGE_SYSTEM_PROMPT},
                                                  {"role": "user", "content": prompt}])
            try:
                parsed = self.mal.unwrap_judge_json(turn.content or "")
                break
            except (json.JSONDecodeError, ValueError):
                continue
        return {"lenient": lenient["score"], "strict": self.mal.judge_score(parsed),
                "strict_reasoning": parsed.get("reasoning"), "category": question.category,
                "gold_refs": question.gold_refs}


def score_run(run_dir: Path, judges: Judges | None) -> list[dict]:
    meta = json.loads((run_dir / "run.json").read_text())
    rows = [json.loads(line) for line in (run_dir / "checkpoint.jsonl").read_text().splitlines() if line.strip()]
    cache_path = run_dir / "judged.jsonl"
    cache = {}
    if cache_path.exists():
        for line in cache_path.read_text().splitlines():
            entry = json.loads(line) if line.strip() else {}
            if "key" in entry:
                cache[entry["key"]] = entry
    out = []
    for row in rows:
        transcript = run_dir / "q" / f"{row['qid']}.stream.jsonl"
        lines = transcript.read_text().splitlines()
        audit = audit_transcript(lines)
        answer = graded_answer(row, final_result_text(lines))
        key = answer_key(row["qid"], answer)
        if key not in cache and judges is not None:
            verdict = {"key": key, "qid": row["qid"], **judges.judge(meta["conv"], row["qid"], answer)}
            cache[key] = verdict
            with cache_path.open("a") as handle:
                handle.write(json.dumps(verdict) + "\n")
        verdict = cache.get(key, {})
        refs_used = set((row.get("answer_json") or {}).get("refs_used") or [])
        out.append({"run": run_dir.name, "item": meta["item"], "arm": meta["arm"], "model": meta["model"],
                    "qid": row["qid"], "wall_s": row["wall_s"], "exit": row["exit"],
                    "instructions_seen": row.get("instructions_seen"), "answer_parsed": row.get("answer_json") is not None,
                    "answer_unparsed": row.get("answer_json") is None,
                    "lenient": verdict.get("lenient"), "strict": verdict.get("strict"), "category": verdict.get("category"),
                    "gold_ref_used": bool(refs_used & set(verdict.get("gold_refs") or [])), **audit})
    return out


def summarize(rows: list[dict]) -> dict:
    by_run: dict[str, list[dict]] = collections.defaultdict(list)
    for row in rows:
        by_run[row["run"]].append(row)
    runs = {}
    for name, items in by_run.items():
        valid = [r for r in items if not r["invalid"] and r["lenient"] is not None]

        def mean(key, rows=items):
            values = [r[key] for r in rows if r.get(key) is not None]
            return sum(values) / len(values) if values else None

        cut_shown = sum(r["cut_refs_shown"] for r in items)
        runs[name] = {
            "item": items[0]["item"], "arm": items[0]["arm"], "model": items[0]["model"], "n": len(items),
            "invalid": sum(r["invalid"] for r in items),
            "answer_unparsed": sum(r["answer_unparsed"] for r in items), "exit_nonzero": sum(r["exit"] != 0 for r in items),
            "lenient": bootstrap([r["lenient"] for r in valid]), "strict": bootstrap([r["strict"] for r in valid]),
            "instructions_seen": dict(collections.Counter(r["instructions_seen"] for r in items)),
            "mean_searches": mean("searches"), "mean_fetches": mean("fetches"),
            "cut_refs_shown": cut_shown, "cut_refs_fetched": sum(r["cut_refs_fetched"] for r in items),
            "cut_fetch_rate": sum(r["cut_refs_fetched"] for r in items) / cut_shown if cut_shown else None,
            "mean_search_bytes_per_call": mean("search_bytes_per_call"),
            "mean_search_tokens_est_per_call": (mean("search_bytes_per_call") or 0) / 4,
            "mean_result_tokens_est_per_q": mean("result_tokens_est"),
            "mean_input_tokens": mean("input_tokens"), "mean_output_tokens": mean("output_tokens"),
            "mean_cache_read": mean("cache_read_tokens"), "mean_cache_create": mean("cache_create_tokens"),
            "mean_api_calls": mean("api_calls"),
            "cost_usd_total": sum(r["cost_usd"] or 0 for r in items), "wall_s_total": sum(r["wall_s"] for r in items),
            "tool_errors": sum(r["tool_errors"] for r in items), "busy_results": sum(r["busy_results"] for r in items),
            "gold_ref_used_rate": mean("gold_ref_used"),
        }
    pairs = []
    names = sorted(by_run)
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            if runs[a]["item"] != runs[b]["item"]:
                continue
            sa = {r["qid"]: r for r in by_run[a] if not r["invalid"] and r["lenient"] is not None}
            sb = {r["qid"]: r for r in by_run[b] if not r["invalid"] and r["lenient"] is not None}
            common = sorted(set(sa) & set(sb))
            if not common:
                continue
            pair = {"a": a, "b": b, "n": len(common)}
            for key in ("lenient", "strict"):
                diffs = [sa[q][key] - sb[q][key] for q in common]
                pair[key] = {**bootstrap(diffs), "a_wins": sum(d > 0 for d in diffs), "b_wins": sum(d < 0 for d in diffs)}
            pairs.append(pair)
    return {"runs": runs, "paired": pairs}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--no-judge", action="store_true")
    parser.add_argument("runs", nargs="+")
    args = parser.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    judges = None if args.no_judge else Judges(out / "judge-ledger")
    rows = [row for run in args.runs for row in score_run(Path(run), judges)]
    with (out / "questions.jsonl").open("w") as handle:
        for row in rows:
            handle.write(json.dumps(row) + "\n")
    summary = summarize(rows)
    (out / "summary.json").write_text(json.dumps(summary, indent=1))
    for name, run in summary["runs"].items():
        print(f"{name}: n={run['n']} invalid={run['invalid']} lenient={run['lenient']['mean']} "
              f"strict={run['strict']['mean']} cost=${run['cost_usd_total']:.4f} wall={run['wall_s_total']:.0f}s "
              f"search_tok/call={run['mean_search_tokens_est_per_call']:.0f} cut_fetch={run['cut_fetch_rate']}")
    if judges is not None:
        print(f"judge spend ${judges.client.ledger.total_usd():.4f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
