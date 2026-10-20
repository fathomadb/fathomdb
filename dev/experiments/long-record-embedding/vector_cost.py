"""Vectors per record, per option, from the measured wordpiece lengths.

Writes results/vector_cost.json. Options (content wordpieces, cut = 510):
  today            1 vector, first 510 wordpieces
  overflow_510     head vector + one per further 510-window of the overflow
  chunk_256_192    chunk-only, 256-wordpiece windows, stride 192
  record_plus_256  record vector + chunk_256_192 for records over the cut
  late_8192        long-context late chunking, 256 windows inside 8192 macro chunks
  long_ctx_8192    1 vector, first 8190 wordpieces
"""

from __future__ import annotations

import json
import math
from pathlib import Path

from tokenizers import Tokenizer

import size_corpora as sc

CUT = 510


def windows(n: int, size: int, stride: int) -> int:
    return 1 if n <= size else 1 + math.ceil((n - size) / stride)


OPTIONS = {
    "today": lambda n: 1,
    "overflow_510": lambda n: max(1, math.ceil(n / CUT)),
    "chunk_256_192": lambda n: windows(n, 256, 192),
    "record_plus_256": lambda n: 1 if n <= CUT else 1 + windows(n, 256, 192),
    "late_8192": lambda n: windows(n, 256, 256),
    "long_ctx_8192": lambda n: 1,
}


def main() -> None:
    data = sc.DEFAULT_DATA
    tok = Tokenizer.from_file(str(sc.DEFAULT_TOKENIZER))
    tok.no_truncation()
    qs = sc._lme(data)
    corpora = [sc.lme_session(data, qs), sc.locomo_session(data), sc.apnews(data), sc.irc(data)]
    rows = []
    for c in corpora:
        lens = [len(e.ids) for e in tok.encode_batch(list(c.bodies.values()), add_special_tokens=False)]
        total_wp = sum(lens)
        row = {"corpus": c.name, "records": len(lens), "wordpieces": total_wp}
        for name, f in OPTIONS.items():
            row[name] = sum(f(n) for n in lens) / len(lens)
        # Wordpieces the embedder runs over, relative to today's (sum of min(n, 510)).
        today_wp = sum(min(n, CUT) for n in lens)
        row["embed_wp_ratio_full_text"] = total_wp / today_wp
        row["embed_wp_ratio_8192_cap"] = sum(min(n, 8190) for n in lens) / today_wp
        row["share_wp_past_8190"] = sum(max(0, n - 8190) for n in lens) / total_wp
        rows.append(row)
    out = Path(__file__).parent / "results/vector_cost.json"
    out.write_text(json.dumps(rows, indent=2) + "\n")
    for r in rows:
        print(json.dumps({k: (round(v, 2) if isinstance(v, float) else v) for k, v in r.items()}))


if __name__ == "__main__":
    main()
