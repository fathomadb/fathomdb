"""COMP-03 evidence through ``fathomdb.pack_context`` (0.8.28 Slice 41).

Items are built from the dataset in ``caller`` mode: one part per turn, in
original order, so the parts joined by ``"\\n"`` equal the stored round body
and retrieval is untouched. The Slice 40 mix is the policy
``full_items = 10**6`` (AC41-27 checks that it reproduces every recorded mix
``evidence_sha256``). Coverage is offline and deterministic.

    python -m experiments.comp03_pack ac41-27 --mix-attempt-root DIR --dataset FILE --out FILE
    python -m experiments.comp03_pack m1 [CONFIG] --attempt-root DIR --dataset FILE --out FILE
    python -m experiments.comp03_pack mf1 [CONFIG] --dataset FILE --out FILE
        (--ids-jsonl FILE [--ids-key ids100] | --attempt-root DIR --arm ID)
    python -m experiments.comp03_pack answer-config --mix-config FILE --m1 FILE --out FILE
        [--control-reuse] [--stop-words]

The MF1 rules (``rank1_session``, ``assistant_reference``) are harness
heuristics (REQ41-F6), applied through ``full_ids`` and ``full_items``.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import statistics
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib
from experiments import comp03_answer as c3

DEFAULT_CONFIG = _lib.EXPERIMENTS_DIR / "configs" / "comp-03" / "pack-coverage.v1.json"
SCHEMA = "comp-03.pack-coverage.v1"
CONFIG_KEYS = {"schema_version", "campaign", "corpus", "answer_set", "arms", "base_policy", "control", "grid",
               "weights", "guard", "rule", "mf1", "ac41_27", "claim_boundary"}
WEIGHTS = {"multi-session": 0.266, "temporal-reasoning": 0.266, "knowledge-update": 0.156,
           "single-session-user": 0.14, "single-session-assistant": 0.112, "single-session-preference": 0.06}
SSA = "single-session-assistant"
METRICS = ("gold_all", "hasA_full", "hasA_allfull")
MIX_POLICY: dict[str, Any] = {"budget_chars": 30000, "item_chars": 4000, "full_items": 10**6, "compact_chars": 300,
                              "keep_roles": ["user"], "expand_groups": True, "order": "chronological"}
STOP_WORDS = {"query_stop_words": {"base": "fathomdb", "extra": ["had", "long", "suggest"]}}
EPS = 1e-9
# The assistant-reference detector of the 2026-10-16 Q&A (variants.py ``DET``), verbatim.
ASSISTANT_REFERENCE = re.compile(
    r"\b(you (told|said|mentioned|recommended|suggested|gave|provided|listed|created|wrote|made|shared|proposed"
    r"|described|explained|outlined|advised|generated|came up with|helped me|offered)|did you (say|suggest|recommend"
    r"|mention|tell|give|provide|list|create|write|propose)|your (suggestion|recommendation|answer|response|list"
    r"|advice|reply)|remind me|you had (recommended|suggested|mentioned|said)|that you (recommended|suggested"
    r"|mentioned|wrote|gave|listed|created))\b", re.I)


class Comp03PackError(RuntimeError):
    pass


def load_config(path: Path) -> dict[str, Any]:
    config = json.loads(Path(path).read_text(encoding="utf-8"))
    if set(config) != CONFIG_KEYS:
        raise Comp03PackError(f"config keys mismatch: {sorted(set(config) ^ CONFIG_KEYS)}")
    if config["schema_version"] != SCHEMA:
        raise Comp03PackError(f"config schema_version must be {SCHEMA}")
    if set(config["arms"]) != {"fts", "hybrid"}:
        raise Comp03PackError("arms maps fts and hybrid to sweep arm IDs")
    return config


# --- items and packing ------------------------------------------------------------------------


def caller_items(item: Mapping[str, Any]) -> list[Any]:
    """Every round of a dataset item as a ``ContextItem``: one part per turn (design § 4)."""
    from fathomdb import ContextItem, ContextPart

    out = []
    for s_index, (date, session) in enumerate(zip(item["haystack_dates"], item["haystack_sessions"], strict=True)):
        for r_index, turns in enumerate(c3._rounds(session)):
            out.append(ContextItem(
                id=f"s{s_index:03d}-r{r_index:03d}",
                parts=tuple(ContextPart(role=turn["role"], text=turn["content"]) for turn in turns),
                group=f"s{s_index:03d}", position=r_index, order_key=date, header=f"[Session date: {date}]"))
    return out


def _policy(policy: Mapping[str, Any], full_ids: Sequence[str]) -> Any:
    from fathomdb import ContextPolicy

    fields = {k: v for k, v in policy.items() if k != "k"}
    fields["keep_roles"] = tuple(fields.get("keep_roles", ("user",)))
    return ContextPolicy(**fields, full_ids=tuple(full_ids))


def pack_question(item: Mapping[str, Any], ids: Sequence[str], policy: Mapping[str, Any], *,
                  full_ids: Sequence[str] = ()) -> Any:
    """``pack_context`` over the ranked ``ids`` as hits and every other round as context."""
    from fathomdb import pack_context

    items = caller_items(item)
    by_id = {i.id: i for i in items}
    listed = set(ids)
    return pack_context([by_id[i] for i in ids], _policy(policy, full_ids),
                        context=[i for i in items if i.id not in listed])


def rank1_session_full_ids(ids: Sequence[str], *, full_items: int, cap: int) -> list[str]:
    """MF1 (c): hits of the rank-1 hit's session past the first ``full_items``, up to ``cap - full_items``.

    So at most ``cap`` items render in full (variants.py ``promote``).
    """
    if not ids:
        return []
    session = ids[0][:4]
    return [r for i, r in enumerate(ids) if i >= full_items and r[:4] == session][:max(cap - full_items, 0)]


def apply_rule(rule: Mapping[str, Any] | None, policy: Mapping[str, Any], ids: Sequence[str],
               question: str) -> tuple[Mapping[str, Any], list[str]]:
    """The policy and ``full_ids`` a harness rule gives for one question."""
    if rule is None:
        return policy, []
    if rule["name"] == "rank1_session":
        return policy, rank1_session_full_ids(ids, full_items=policy["full_items"], cap=rule["cap"])
    if rule["name"] == "assistant_reference":
        if ASSISTANT_REFERENCE.search(question):
            return {**policy, "full_items": rule["full_items"]}, []
        return policy, []
    raise Comp03PackError(f"unknown harness rule {rule['name']}")


def evidence_record(item: Mapping[str, Any], ids: Sequence[str], evidence: Mapping[str, Any]) -> dict[str, Any]:
    """The text an answer run sends for a ``pack_context`` evidence config, with its digest and counts."""
    if evidence.get("item_source", "caller") != "caller":
        raise Comp03PackError("only caller items are implemented")
    policy, full_ids = apply_rule(evidence.get("harness_rule"), evidence["pack"], ids, item["question"])
    packed = pack_question(item, ids, policy, full_ids=full_ids)
    rendered = [e for e in packed.items if e.tier != "omitted"]
    return {"text": packed.text, "evidence_sha256": hashlib.sha256(packed.text.encode("utf-8")).hexdigest(),
            "rounds_in_prompt": len(rendered), "chars": packed.chars_used,
            "forced": sum(1 for e in rendered if e.forced)}


# --- coverage --------------------------------------------------------------------------------


def answer_turns(item: Mapping[str, Any]) -> dict[str, list[int]]:
    """Round ID -> indices of its ``has_answer`` turns (grading only)."""
    out = {}
    for s_index, session in enumerate(item["haystack_sessions"]):
        for r_index, turns in enumerate(c3._rounds(session)):
            idx = [k for k, turn in enumerate(turns) if turn.get("has_answer")]
            if idx:
                out[f"s{s_index:03d}-r{r_index:03d}"] = idx
    return out


def question_coverage(packed: Any, item: Mapping[str, Any]) -> dict[str, Any]:
    """gold_all (every answer session shown), hasA_full / hasA_allfull (any / every answer round with
    each answer turn in full) and rounds shown."""
    rendered = {e.id: e for e in packed.items if e.tier != "omitted"}
    gold = set(item["answer_session_ids"])
    gold_groups = {f"s{i:03d}" for i, sid in enumerate(item["haystack_session_ids"]) if sid in gold}
    answers = answer_turns(item)

    def full(rid: str) -> bool:
        entry = rendered.get(rid)
        return entry is not None and all(entry.parts[k].chars == entry.parts[k].of_chars for k in answers[rid])

    return {"gold_all": gold_groups <= {rid[:4] for rid in rendered},
            "hasA_full": any(full(r) for r in answers), "hasA_allfull": bool(answers) and all(full(r) for r in answers),
            "rounds": len(rendered)}


def coverage_table(rows: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """Per-type rates, report-weighted rates (``WEIGHTS``) and mean rounds shown."""
    per_type = {}
    for qtype in sorted({r["question_type"] for r in rows}):
        mine = [r for r in rows if r["question_type"] == qtype]
        per_type[qtype] = {"n": len(mine), **{m: sum(bool(r[m]) for r in mine) / len(mine) for m in METRICS},
                           "rounds": statistics.fmean(r["rounds"] for r in mine)}
    weighted = {m: sum(WEIGHTS[t] * block[m] for t, block in per_type.items()) for m in METRICS}
    return {"n": len(rows), "per_type": per_type, "weighted": weighted,
            "rounds": statistics.fmean(r["rounds"] for r in rows) if rows else 0.0}


# --- inputs ----------------------------------------------------------------------------------


def _items(dataset_path: Path) -> dict[str, Mapping[str, Any]]:
    return {item["question_id"]: item for item in json.loads(Path(dataset_path).read_text(encoding="utf-8"))}


def recorded_ids(attempt_root: Path, qid: str, arm: str, k: int) -> list[str]:
    """IDs a sweep recorded for one question and arm at limit ``k``."""
    record = json.loads((Path(attempt_root) / "questions" / qid / arm / "retrieval.json").read_text(encoding="utf-8"))
    ids_at = record.get("ids_at", {})
    if str(k) in ids_at:
        return list(ids_at[str(k)])
    if len(record["ids"]) < k and k > 10:
        raise Comp03PackError(f"{qid}/{arm}: no IDs recorded at k = {k}")
    return list(record["ids"])[:k]


def load_ids_jsonl(path: Path, key: str = "ids100") -> dict[str, list[str]]:
    """``{qid: ids}`` from a recorded top-100 file (one ``{"qid", key}`` object per line)."""
    out = {}
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        if line.strip():
            record = json.loads(line)
            out[record["qid"]] = list(record[key])
    return out


def _subset(qid: str, answer: set[str]) -> str:
    return "answer_set" if qid in answer else "selection"


def answer_ids(config: Mapping[str, Any]) -> list[str]:
    path = _lib.EXPERIMENTS_DIR.parent / config["answer_set"]["config"]
    return list(json.loads(path.read_text(encoding="utf-8"))["selection"]["question_ids"])


# --- M1 -------------------------------------------------------------------------------------


def _grid_policy(config: Mapping[str, Any], row: Mapping[str, Any]) -> dict[str, Any]:
    return {**config["base_policy"], "full_items": row["full_items"], "compact_chars": row["compact_chars"],
            "order": row.get("order", "rank")}


def evaluate_m1(config: Mapping[str, Any], attempt_root: Path, dataset_path: Path,
                answer_ids: Sequence[str]) -> dict[str, Any]:
    """Coverage of the control and every grid policy at its own k, per arm, on both subsets; then the rule."""
    items = _items(dataset_path)
    answer = set(answer_ids)
    rows_list = [config["control"], *config["grid"]]
    rows: dict[str, dict[str, dict[str, list[dict[str, Any]]]]] = {
        s: {arm: {row["name"]: [] for row in rows_list} for arm in config["arms"]} for s in ("selection", "answer_set")}
    counts = {"scored": 0, "selection": 0, "answer_set": 0, "abstention_skipped": 0}
    for qid in sorted(items):
        item = items[qid]
        if qid.endswith("_abs"):
            counts["abstention_skipped"] += 1
            continue
        counts["scored"] += 1
        subset = _subset(qid, answer)
        counts[subset] += 1
        for arm_key, arm_id in config["arms"].items():
            for row in rows_list:
                ids = recorded_ids(attempt_root, qid, arm_id, row["k"])
                packed = pack_question(item, ids, _grid_policy(config, row))
                rows[subset][arm_key][row["name"]].append(
                    {**question_coverage(packed, item), "question_type": item["question_type"]})
    tables = {s: {arm: {name: coverage_table(r) for name, r in by.items()} for arm, by in arms.items()}
              for s, arms in rows.items()}
    return {
        "schema_version": "comp-03.pack-coverage.m1.v1",
        "questions": counts,
        "tables": tables,
        "choice": select_policy(config, tables["selection"]),
        "inputs": {"dataset_sha256": c3.sha256_file(Path(dataset_path)), "attempt_root": str(attempt_root),
                   "arms": dict(config["arms"])},
        "renderer": "fathomdb.pack_context, caller items",
    }


def select_policy(config: Mapping[str, Any], tables: Mapping[str, Mapping[str, Any]]) -> dict[str, Any]:
    """Test-plan § 4.1: guard (REQ41-M2) on both arms, highest mean weighted allfull (ties: smaller k,
    larger compact_chars), adopted only with a gain of ``min_gain`` on both arms; else the fallback."""
    guard, rule = config["guard"], config["rule"]
    control = config["control"]["name"]
    grid = {row["name"]: row for row in config["grid"]}
    excluded: dict[str, list[str]] = {}
    for name in grid:
        failed = []
        for arm, table in tables.items():
            base, mine = table[control]["per_type"], table[name]["per_type"]
            for qtype, block in mine.items():
                value = block["hasA_allfull"]
                if qtype == SSA:
                    if value < guard["ssa_min"] - EPS:
                        failed.append(f"{arm}:SSA")
                elif value < base[qtype]["hasA_allfull"] - guard["other_tolerance"] - EPS:
                    failed.append(f"{arm}:{qtype}")
        if failed:
            excluded[name] = failed

    def mean(name: str) -> float:
        return statistics.fmean(t[name]["weighted"]["hasA_allfull"] for t in tables.values())

    candidates = [n for n in grid if n not in excluded]
    out = {"excluded": excluded, "rule": rule["text"],
           "grid": {n: {k: grid[n][k] for k in ("full_items", "compact_chars", "k")} for n in grid},
           "mean_weighted_allfull": {n: mean(n) for n in grid}}
    if not candidates:
        return {**out, "best": None, "policy": rule["fallback"], "adopted": False,
                "reason": "no policy passes the guard on both arms"}
    best = max(candidates, key=lambda n: (round(mean(n), 9), -grid[n]["k"], grid[n]["compact_chars"]))
    gains = {arm: t[best]["weighted"]["hasA_allfull"] - t[control]["weighted"]["hasA_allfull"]
             for arm, t in tables.items()}
    if all(g >= rule["min_gain"] - EPS for g in gains.values()):
        return {**out, "best": best, "policy": best, "adopted": True, "gains": gains, "reason": "rule met"}
    return {**out, "best": best, "policy": rule["fallback"], "adopted": False, "gains": gains,
            "reason": f"no policy gains {rule['min_gain']} weighted allfull over the control on both arms"}


# --- AC41-27 and MF1 ------------------------------------------------------------------------------


def ac41_27(mix_root: Path, dataset_path: Path, policy: Mapping[str, Any] | None = None) -> dict[str, Any]:
    """Pack each recorded mix top-k with the mix policy and compare against its ``evidence_sha256``."""
    items = _items(dataset_path)
    policy = MIX_POLICY if policy is None else policy
    arms: dict[str, dict[str, Any]] = {}
    for path in sorted(Path(mix_root).glob("questions/*/*/retrieval.json")):
        qid, arm = path.parent.parent.name, path.parent.name
        record = json.loads(path.read_text(encoding="utf-8"))
        text = pack_question(items[qid], record["ids"], policy).text
        block = arms.setdefault(arm, {"checked": 0, "matched": 0, "mismatched": []})
        block["checked"] += 1
        if hashlib.sha256(text.encode("utf-8")).hexdigest() == record["evidence_sha256"]:
            block["matched"] += 1
        else:
            block["mismatched"].append(qid)
    return {"schema_version": "comp-03.pack-coverage.ac41-27.v1", "arms": arms,
            "pass": bool(arms) and all(not b["mismatched"] for b in arms.values()),
            "policy": dict(policy), "dataset_sha256": c3.sha256_file(Path(dataset_path))}


def evaluate_mf1(config: Mapping[str, Any], ids_by_qid: Mapping[str, Sequence[str]], dataset_path: Path,
                 answer_ids: Sequence[str]) -> dict[str, Any]:
    """MF1: the variant rows through ``pack_context`` on recorded rankings, per subset."""
    items = _items(dataset_path)
    answer = set(answer_ids)
    mf1 = config["mf1"]
    rows = {s: {row["name"]: [] for row in mf1["rows"]} for s in ("selection", "answer_set")}
    for qid in sorted(ids_by_qid):
        if qid.endswith("_abs") or qid not in items:
            continue
        item = items[qid]
        for row in mf1["rows"]:
            ids = list(ids_by_qid[qid])[:row["k"]]
            policy, full_ids = apply_rule(row.get("rule"), _grid_policy(config, row), ids, item["question"])
            packed = pack_question(item, ids, policy, full_ids=full_ids)
            rows[_subset(qid, answer)][row["name"]].append(
                {**question_coverage(packed, item), "question_type": item["question_type"]})
    tables = {s: {name: coverage_table(r) for name, r in by.items()} for s, by in rows.items()}
    checks = {}
    for name, expected in mf1["expected"].items():
        table = tables["selection"][name]
        observed = {"weighted": table["weighted"]["hasA_allfull"],
                    **{key: table["per_type"].get(qtype, {}).get("hasA_allfull")
                       for key, qtype in (("SSA", SSA), ("SSP", "single-session-preference"))}}
        checks[name] = {key: {"expected": value, "observed": observed[key],
                              "equal_at_3dp": observed[key] is not None and round(observed[key], 3) == value}
                        for key, value in expected.items()}
    return {"schema_version": "comp-03.pack-coverage.mf1.v1", "tables": tables, "expected": mf1["expected"],
            "checks": checks, "heuristics": "harness-side rules (REQ41-F6), not SDK behaviour",
            "renderer": "fathomdb.pack_context, caller items"}


# --- M3 answer config --------------------------------------------------------------------------------


def build_pack_answer_config(mix: Mapping[str, Any], choice: Mapping[str, Any], *, control_reuse: bool,
                             engine_config: Mapping[str, Any] | None) -> dict[str, Any]:
    """The M3 answer config: the FathomDB FTS and hybrid arms through ``pack_context`` with the M1 policy,
    plus the mix arms unless the post-Unit H mix run is reused as the control. LanceDB is not re-run."""
    name = choice["policy"]
    row = choice["grid"][name]
    order = "chronological" if mix["evidence"].get("chronological") else "rank"
    base = {k: v for k, v in MIX_POLICY.items() if k not in ("full_items", "compact_chars", "order")}
    pack_policy = {**base, "full_items": row["full_items"], "compact_chars": row["compact_chars"], "order": order}
    mix_policy = {**MIX_POLICY, "order": order}
    products = {arm["id"]: arm for arm in mix["arms"]}

    def arm(source: str, suffix: str, top_k: int, policy: Mapping[str, Any], label: str) -> dict[str, Any]:
        out = {k: v for k, v in products[source].items() if k != "requires_sweep_verdict"}
        out.update({"id": f"{source}-{suffix}", "top_k": top_k, "renderer": label,
                    "evidence": {"renderer": "pack_context", "item_source": "caller", "pack": dict(policy)}})
        if engine_config:
            out["engine_config"] = copy.deepcopy(dict(engine_config))
        return out

    arms = []
    if not control_reuse:
        arms += [arm(s, "mix", 10, mix_policy, "pack_context Slice 40 mix (caller items)")
                 for s in ("fathomdb-fts", "fathomdb-hybrid")]
    arms += [arm(s, "pack", row["k"], pack_policy, f"pack_context {name} (caller items)")
             for s in ("fathomdb-fts", "fathomdb-hybrid")]
    config = copy.deepcopy(dict(mix))
    config.pop("decision_rule", None)
    config["schema_version"] = "comp-03.answer.v4"
    config["arms"] = arms
    config["retrieval"] = {**config["retrieval"], "top_k": 10,
                           "per_arm": "each arm's top_k is its retrieval depth; retrieval metrics use top_k 10"}
    config["evidence"] = {"renderer": "per arm (arms[].evidence)", "recency_instruction":
                          bool(mix["evidence"].get("recency_instruction")),
                          "pack_choice": {"policy": name, "adopted": choice["adopted"]}}
    config["claim_boundary"] = {
        "statement": "REQ41-M3: a no-regression check of pack_context against the Slice 40 mix on the FathomDB "
                     "FTS and hybrid arms; noise floor about 3 of 60 flips per arm",
        "control": "the post-Unit H Slice 40 mix answer run" if control_reuse else "the mix arms of this run",
        "lancedb": "not re-run; it stays on the Slice 40 renderer (owner ruling 2026-10-16, R0 Q1)",
    }
    return c3.load_config_obj(config)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    check = sub.add_parser("ac41-27")
    check.add_argument("--mix-attempt-root", type=Path, required=True)
    m1 = sub.add_parser("m1")
    m1.add_argument("--attempt-root", type=Path, required=True)
    mf1 = sub.add_parser("mf1")
    source = mf1.add_mutually_exclusive_group(required=True)
    source.add_argument("--ids-jsonl", type=Path)
    source.add_argument("--attempt-root", type=Path)
    mf1.add_argument("--ids-key", default="ids100")
    mf1.add_argument("--arm", default="fathomdb-fts")
    for p in (check, m1, mf1):
        p.add_argument("--dataset", type=Path, required=True)
    for p in (m1, mf1):
        p.add_argument("config", type=Path, nargs="?", default=DEFAULT_CONFIG)
    answer = sub.add_parser("answer-config")
    answer.add_argument("--mix-config", type=Path, required=True)
    answer.add_argument("--m1", type=Path, required=True, help="the m1 output; its choice picks the policy")
    answer.add_argument("--control-reuse", action="store_true")
    answer.add_argument("--stop-words", action="store_true", help="open the FathomDB arms with REQ41-Q8's setting")
    for p in (check, m1, mf1, answer):
        p.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command == "ac41-27":
        result = ac41_27(args.mix_attempt_root, args.dataset)
    elif args.command == "m1":
        config = load_config(args.config)
        result = evaluate_m1(config, args.attempt_root, args.dataset, answer_ids(config))
    elif args.command == "mf1":
        config = load_config(args.config)
        if args.ids_jsonl is not None:
            ids = load_ids_jsonl(args.ids_jsonl, args.ids_key)
        else:
            ids = {p.parent.parent.name: recorded_ids(args.attempt_root, p.parent.parent.name, args.arm, 30)
                   for p in sorted(args.attempt_root.glob(f"questions/*/{args.arm}/retrieval.json"))}
        result = evaluate_mf1(config, ids, args.dataset, answer_ids(config))
    else:
        mix = c3.load_config(args.mix_config)
        choice = json.loads(args.m1.read_text(encoding="utf-8"))["choice"]
        result = build_pack_answer_config(mix, choice, control_reuse=args.control_reuse,
                                          engine_config=STOP_WORDS if args.stop_words else None)
    args.out.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({k: result[k] for k in ("pass", "choice", "checks", "schema_version") if k in result})[:2000])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
