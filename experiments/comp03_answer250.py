"""The paired 250-question COMP-03 answer run (owner approval 2026-10-16).

FathomDB FTS through ``pack_context`` (T5/A0/k50, the REQ41-Q8 query stop
words; the M3 arm ``fathomdb-fts-pack``) against LanceDB FTS on the Slice 40
mix renderer. The 250 questions are the 60 pinned answer-set IDs plus IDs
drawn per type so the 250 follow the full 500's type mix:

    python -m experiments.comp03_answer250 build --dataset FILE --m3-config FILE [--out FILE]

then run with ``python -m experiments.comp03_answer run`` (schema
``comp-03.answer.v5``: answer reuse for the pinned 60 and a cost ceiling).
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import random
from collections import Counter
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import comp03_answer as c3

CONFIG_DIR = c3.DEFAULT_CONFIG.parent
DEFAULT_OUT = CONFIG_DIR / "hermes-answer.250.json"
TOTAL = 250
SEED = 250
CEILING_USD = 11.5
QUESTION_USD_ESTIMATE = 0.06
ROUNDING = ("quota_t = floor(250 * n_t / 500), then the leftover questions go one each to the largest "
            "fractional parts, ties broken by type name ascending (largest-remainder rounding)")
RULE = ("the 60 pinned answer-set IDs first, in their order; then, for each type in name order, "
        "sorted(random.Random(f'{seed}:{type}').sample(sorted IDs of the type not already pinned, "
        "quota - pinned of the type)). Abstention questions are members of their type, as in the 60-set rule.")


def type_quotas(counts: Mapping[str, int], total: int) -> dict[str, int]:
    """Largest-remainder quotas proportional to ``counts``; ties go to the type name sorting first."""
    size = sum(counts.values())
    exact = {t: total * n / size for t, n in counts.items()}
    quotas = {t: math.floor(v) for t, v in exact.items()}
    left = total - sum(quotas.values())
    for t in sorted(exact, key=lambda t: (-(exact[t] - quotas[t]), t))[:left]:
        quotas[t] += 1
    return dict(sorted(quotas.items()))


def select_superset(dataset: Sequence[Mapping[str, Any]], base: Sequence[str], quotas: Mapping[str, int], *,
                    seed: int) -> list[str]:
    """``base`` first, then each type's seeded draw up to its quota (``RULE``)."""
    types = {item["question_id"]: item["question_type"] for item in dataset}
    if any(q not in types for q in base) or len(set(base)) != len(base):
        raise ValueError("every pinned ID is a distinct dataset question")
    if {types[q] for q in base} - set(quotas):
        raise ValueError("a pinned ID has a type without a quota")
    picked = list(base)
    for qtype in sorted(quotas):
        pinned = [q for q in base if types[q] == qtype]
        need = quotas[qtype] - len(pinned)
        if need < 0:
            raise ValueError(f"{qtype}: {len(pinned)} pinned IDs exceed the quota {quotas[qtype]}")
        pool = sorted(q for q, t in types.items() if t == qtype and q not in set(base))
        picked.extend(sorted(random.Random(f"{seed}:{qtype}").sample(pool, need)))
    return picked


def ids_sha256(ids: Sequence[str]) -> str:
    return hashlib.sha256("\n".join(ids).encode("utf-8")).hexdigest()


def build_config(mix: Mapping[str, Any], m3: Mapping[str, Any], dataset: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """The v5 config: the M3 FathomDB FTS pack arm, the mix LanceDB arm, the 250 pinned IDs, reuse and the ceiling."""
    base = list(mix["selection"]["question_ids"])
    quotas = type_quotas(Counter(item["question_type"] for item in dataset), TOTAL)
    ids = select_superset(dataset, base, quotas, seed=SEED)
    fathom = copy.deepcopy(next(a for a in m3["arms"] if a["id"] == "fathomdb-fts-pack"))
    lance = copy.deepcopy(next(a for a in mix["arms"] if a["id"] == "lancedb-fts"))
    lance.update({"top_k": mix["retrieval"]["top_k"], "renderer": "comp03_answer.render_evidence Slice 40 mix",
                  "evidence": {"renderer": "comp03_answer.render_evidence",
                               "policy": copy.deepcopy(mix["evidence"]["policy"]),
                               "chronological": bool(mix["evidence"].get("chronological"))}})
    config = copy.deepcopy(dict(m3))
    config["schema_version"] = "comp-03.answer.v5"
    config["arms"] = [fathom, lance]
    config["live"] = {**copy.deepcopy(mix["live"]), "ceiling_usd": CEILING_USD,
                      "question_usd_estimate": QUESTION_USD_ESTIMATE,
                      "ceiling_rule": "before a question that needs any model call, stop if this run's spend "
                                      "(reused calls excluded) plus max(question_usd_estimate, the largest "
                                      "per-question spend so far) exceeds ceiling_usd; rerun to resume"}
    config["evidence"] = {"renderer": "per arm (arms[].evidence)",
                          "recency_instruction": bool(mix["evidence"].get("recency_instruction"))}
    config["selection"] = {
        "rule": RULE, "rounding": ROUNDING, "seed": SEED, "total": TOTAL, "quotas": quotas,
        "base": {"config": "experiments/configs/comp-03/hermes-answer.mix.json",
                 "question_ids_sha256": ids_sha256(base)},
        "base_count": len(base), "question_ids": ids, "question_ids_sha256": ids_sha256(ids),
    }
    config["reuse"] = [
        {"arm": "fathomdb-fts-pack", "source_attempt_root": "~/.cache/fathomdb-comp/pa-m3",
         "source_arm": "fathomdb-fts-pack"},
        {"arm": "lancedb-fts", "source_attempt_root": "~/.cache/fathomdb-comp/comp03-run-mix",
         "source_arm": "lancedb-fts"},
    ]
    config["claim_boundary"] = {
        "statement": "paired answer accuracy on 250 LongMemEval-S questions in the full-500 type mix: FathomDB FTS "
                     "(pack_context T5/A0/k50, query stop words had/long/suggest) minus LanceDB FTS (Slice 40 mix "
                     "renderer); each arm keeps its own renderer, so this compares shipped paths, not stores",
        "reuse": "the pinned 60 reuse the M3 (pa-m3) and mix (comp03-run-mix) answers when reader, judge, seeds, "
                 "prompts and evidence policy match; reused answers are marked in the receipt",
    }
    return c3.load_config_obj(config)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    build = sub.add_parser("build")
    build.add_argument("--dataset", type=Path, required=True)
    build.add_argument("--m3-config", type=Path, required=True)
    build.add_argument("--mix-config", type=Path, default=CONFIG_DIR / "hermes-answer.mix.json")
    build.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args(argv)
    mix = c3.load_config(args.mix_config)
    if c3.sha256_file(args.dataset) != mix["corpus"]["sha256"]:
        raise c3.Comp03Error("dataset digest does not match the config")
    dataset = json.loads(args.dataset.read_text(encoding="utf-8"))
    config = build_config(mix, c3.load_config(args.m3_config), dataset)
    args.out.write_text(json.dumps(config, indent=1) + "\n", encoding="utf-8")
    print(json.dumps({"out": str(args.out), "quotas": config["selection"]["quotas"],
                      "question_ids_sha256": config["selection"]["question_ids_sha256"]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
