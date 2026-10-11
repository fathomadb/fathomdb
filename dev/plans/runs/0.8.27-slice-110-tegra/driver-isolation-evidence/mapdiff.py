#!/usr/bin/env python3
"""mapdiff.py A B : print mappings below 1 TiB that are only in A (-) or only in B (+)."""

import sys


def load(p):
    out = {}
    for line in open(p):
        f = line.split()
        a, b = (int(x, 16) for x in f[0].split("-"))
        if a < (1 << 40):
            out[f[0]] = (a, b, f[1], " ".join(f[5:]))
    return out


A, B = load(sys.argv[1]), load(sys.argv[2])
for k in sorted(set(A) | set(B), key=lambda k: int(k.split("-")[0], 16)):
    if (k in A) != (k in B):
        a, b, p, n = (A if k in A else B)[k]
        print(
            f"{'-' if k in A else '+'} {a:#014x}-{b:#014x} {p} {(b - a) / 2**20:12.2f}MiB {n}"
        )
