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
import math
import os
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
    decoder = json.JSONDecoder()
    for line in text.splitlines():
        if line.startswith('"'):
            try:
                ref, end = decoder.raw_decode(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(ref, str) or not ref.startswith("l:"):
                continue
            tokens = re.findall(r'(?:[^\s"]|"(?:\\.|[^"\\])*")+', line[end:])
            meta = tokens[:tokens.index("|")] if "|" in tokens else []
            if not meta or not any(token.startswith("arm=") for token in meta):
                continue
            refs.append(ref)
            if "cut" in meta:
                cut.append(ref)
        else:
            match = _REF_LINE.match(line)
            if match:
                refs.append(match.group(1))
                if " [cut]" in line.split(" | ", 1)[0]:
                    cut.append(match.group(1))
    return refs, cut


def is_busy(text: str) -> bool:
    """Whether a tool result is a ``busy`` error (``error.class``); substring fallback when not JSON."""
    try:
        data = json.loads(text)
    except (json.JSONDecodeError, TypeError):
        return bool(re.match(r"^(?:error )?busy[/:]", text.lstrip()))
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
    hits_per_page: list[int] = []
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
                    refs, cut = cut_refs_in_result(text)
                    shown_cut.extend(cut)
                    hits_per_page.append(len(refs))
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
        "search_hits_per_page": hits_per_page,
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
    def __init__(self, ledger_dir: Path, *, cap_usd: float, dataset: Path, harness: Path,
                 env_file: Path, model: str) -> None:
        from experiments import mcp_agent_loop as mal

        self.mal = mal
        self.model = model
        self.dataset = dataset
        mal.cost_usd(model, mal.Usage(0, 0, 0, 0))
        self.prompts = mal.load_harness_prompts(harness)
        secrets = mal.read_env_file(env_file)
        ledger = mal.CostLedger(ledger_dir, cap_usd=cap_usd)
        self.client = mal.ChatClient(secrets["R2_JUDGE_BASE_URL"], secrets["R2_JUDGE_API_KEY"], ledger,
                                     arm="judge", retry_unknown=False)
        self.conv: dict[int, tuple] = {}

    def _conv(self, index: int):
        if index not in self.conv:
            questions, conversation = self.mal.load_questions(self.dataset, index)
            self.conv[index] = ({q.qid: q for q in questions},
                                self.mal.Grader(self.prompts, conversation, self.client,
                                                parse_attempts=1, judge_model=self.model))
        return self.conv[index]

    def judge(self, conv: int, qid: str, answer: str) -> dict:
        questions, grader = self._conv(conv)
        question = questions[f"conv{conv}_{qid}"]
        lenient = grader.judge(question, answer, [])
        evidence = "\n".join(grader.lookup[ref] for ref in question.evidence if ref in grader.lookup)
        prompt = STRICT_PROMPT.format(question=question.question, gold=question.answer, evidence=evidence or "(none)",
                                      response=answer)
        turn = self.client.complete(qid=qid, role="strict-judge", model=self.model, json_object=True,
                                    messages=[{"role": "system", "content": self.prompts.JUDGE_SYSTEM_PROMPT},
                                              {"role": "user", "content": prompt}])
        parsed = self.mal.unwrap_judge_json(turn.content or "")
        if not isinstance(parsed, dict) or str(parsed.get("label", "")).upper() not in {"CORRECT", "WRONG"}:
            raise ValueError("malformed billed strict judge response; no automatic retry")
        return {"lenient": lenient["score"], "strict": self.mal.judge_score(parsed),
                "strict_reasoning": parsed.get("reasoning"), "category": question.category,
                "gold_refs": question.gold_refs}


def validate_rows(rows: list[dict], expected: list[str] | None = None, *, require_grades: bool = True) -> None:
    qids = [r["qid"] for r in rows]
    if not rows or len(set(qids)) != len(qids) or (expected is not None and set(qids) != set(expected)):
        raise ValueError("missing, duplicate or unexpected questions; phase invalid")
    for row in rows:
        invalid = (row.get("exit") != 0 or row.get("invalid") or not row.get("answer_parsed")
                   or row.get("tool_errors") or row.get("busy_results") or row.get("result_subtype") != "success"
                   or not 1 <= row.get("searches", 0) <= 3 or sum(row.get("tool_calls", {}).values()) > 4)
        if require_grades:
            invalid = invalid or row.get("lenient") not in (0.0, 1.0) or row.get("strict") not in (0.0, 1.0)
        if invalid:
            raise ValueError(f"invalid episode/grade {row['qid']}; no measurement verdict")


def _append_grade(path: Path, entry: dict) -> None:
    with path.open("a") as handle:
        handle.write(json.dumps(entry) + "\n")
        handle.flush()
        os.fsync(handle.fileno())


def score_run(run_dir: Path, judges: Judges | None) -> list[dict]:
    meta = json.loads((run_dir / "run.json").read_text())
    latest = {}
    for line in (run_dir / "checkpoint.jsonl").read_text().splitlines():
        if line.strip():
            row = json.loads(line)
            latest[row["qid"]] = row
    if any(r.get("status") != "complete" or r.get("valid") is not True for r in latest.values()):
        raise ValueError("incomplete/invalid checkpoint; no grading or retry")
    expected = [q.split("_", 1)[-1] for q in meta["questions"]]
    if str(meta["item"]) == "4":
        from run import CONV0_SAMPLE

        if meta["conv"] != 0 or len(expected) != 30 or set(expected) != set(CONV0_SAMPLE):
            raise ValueError("item4 requires the unchanged fixed30 conv0 sample")
    cache_path = run_dir / "judged.jsonl"
    cache = {}
    if cache_path.exists():
        for line in cache_path.read_text().splitlines():
            if line.strip():
                entry = json.loads(line)
                cache[entry["key"]] = entry
    out = []
    answers = {}
    for row in latest.values():
        lines = (run_dir / "q" / f"{row['qid']}.stream.jsonl").read_text().splitlines()
        audit = audit_transcript(lines)
        answer = graded_answer(row, final_result_text(lines))
        key = answer_key(row["qid"], answer)
        verdict = cache.get(key, {})
        if verdict and (verdict.get("status") == "started" or verdict.get("lenient") not in (0.0, 1.0)
                        or verdict.get("strict") not in (0.0, 1.0)):
            raise ValueError("incomplete judge reservation; unknown spend cannot be retried")
        if verdict and judges is not None and verdict.get("judge_model") != judges.model:
            raise ValueError("cached judge model differs; do not reuse or rejudge this run automatically")
        answers[row["qid"]] = (answer, key)
        out.append({"run": run_dir.name, "item": meta["item"], "arm": meta["arm"], "model": meta["model"],
                    "qid": row["qid"], "wall_s": row["wall_s"], "exit": row["exit"],
                    "instructions_seen": row.get("instructions_seen"),
                    "answer_parsed": row.get("answer_json") is not None and bool(answer.strip()),
                    "answer_unparsed": row.get("answer_json") is None,
                    "lenient": verdict.get("lenient"), "strict": verdict.get("strict"), "category": verdict.get("category"),
                    "gold_ref_used": bool(set((row.get("answer_json") or {}).get("refs_used") or [])
                                          & set(verdict.get("gold_refs") or [])), **audit})
    validate_rows(out, expected, require_grades=False)
    for row in out:
        answer, key = answers[row["qid"]]
        if key not in cache and judges is not None:
            judges.client.ledger.check()
            _append_grade(cache_path, {"key": key, "qid": row["qid"], "status": "started"})
            verdict = {"key": key, "qid": row["qid"], "status": "complete", "judge_model": judges.model,
                       **judges.judge(meta["conv"], row["qid"], answer)}
            _append_grade(cache_path, verdict)
            cache[key] = verdict
            row.update({k: verdict[k] for k in ("lenient", "strict", "category")})
            source = latest[row["qid"]]
            row["gold_ref_used"] = bool(set((source.get("answer_json") or {}).get("refs_used") or [])
                                        & set(verdict["gold_refs"]))
    return out


def summarize(rows: list[dict], expected_by_run: dict | None = None, *, require_grades: bool = True) -> dict:
    by_run: dict[str, list[dict]] = collections.defaultdict(list)
    for row in rows:
        by_run[row["run"]].append(row)
    grades_complete = bool(rows) and all(r.get("lenient") in (0.0, 1.0) and r.get("strict") in (0.0, 1.0) for r in rows)
    runs = {}
    for name, items in by_run.items():
        validate_rows(items, expected_by_run[name] if expected_by_run else None, require_grades=require_grades)
        valid = items if grades_complete else []

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
            "mean_hits_per_page": mean("search_hits_mean", [{"search_hits_mean": sum(r["search_hits_per_page"]) / len(r["search_hits_per_page"])}
                                                        for r in items if r.get("search_hits_per_page")]),
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
            "gold_ref_used_rate": mean("gold_ref_used") if grades_complete else None,
        }
    pairs = []
    names = sorted(by_run)
    for i, a in enumerate(names):
        for b in names[i + 1:]:
            if runs[a]["item"] != runs[b]["item"]:
                continue
            if {r["qid"] for r in by_run[a]} != {r["qid"] for r in by_run[b]}:
                raise ValueError("paired runs have different question sets; no intersection verdict")
            if not grades_complete:
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
    return {"runs": runs, "paired": pairs, "accuracy_complete": grades_complete}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--no-judge", action="store_true", help="audit only; no accuracy claim without cached grades")
    parser.add_argument("--cap-usd", type=float, help="judge reported-cost stop; may overshoot by the in-flight call")
    parser.add_argument("--dataset", type=Path, default=DATASET)
    parser.add_argument("--harness", type=Path, default=HARNESS)
    parser.add_argument("--env-file", type=Path, default=ENV_FILE)
    parser.add_argument("--judge-model", default="gpt-4o-mini")
    parser.add_argument("runs", nargs="+")
    args = parser.parse_args()
    if not args.no_judge and (args.cap_usd is None or not math.isfinite(args.cap_usd) or args.cap_usd <= 0):
        parser.error("judging requires a finite positive --cap-usd")
    paths = [Path(p) for p in args.runs]
    if len({p.name for p in paths}) != len(paths):
        parser.error("run directory basenames must be unique")
    expected = {p.name: [q.split("_", 1)[-1] for q in json.loads((p / "run.json").read_text())["questions"]]
                for p in paths}
    # Validate every episode and pair before constructing a client or spending.
    rows = [row for path in paths for row in score_run(path, None)]
    summarize(rows, expected, require_grades=False)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    judges = None if args.no_judge else Judges(out / "judge-ledger", cap_usd=args.cap_usd, dataset=args.dataset,
                                             harness=args.harness, env_file=args.env_file, model=args.judge_model)
    if judges is not None:
        rows = [row for path in paths for row in score_run(path, judges)]
    summary = summarize(rows, expected, require_grades=not args.no_judge)
    with (out / "questions.jsonl").open("w") as handle:
        for row in rows:
            handle.write(json.dumps(row) + "\n")
    (out / "summary.json").write_text(json.dumps(summary, indent=1))
    for name, run in summary["runs"].items():
        print(f"{name}: n={run['n']} invalid={run['invalid']} lenient={run['lenient']['mean']} "
              f"strict={run['strict']['mean']} cost=${run['cost_usd_total']:.4f} "
              f"search_tokens_est_bytes_div_4/call={run['mean_search_tokens_est_per_call']:.0f}")
    if judges is not None:
        print(f"judge spend ${judges.client.ledger.total_usd():.4f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
