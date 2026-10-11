#!/usr/bin/env python3
"""replay_layout.py MAPS_FILE: print the start addresses (comma-separated, hex) of every
mapping that overlaps [8 GiB, 128 GiB), for replay as pool_va_repro --at blockers."""

import sys

G = 1 << 30
starts = []
for line in open(sys.argv[1]):
    a, b = (int(x, 16) for x in line.split()[0].split("-"))
    if b > 8 * G and a < 128 * G:
        starts.append(max(a, 8 * G))
print(",".join(hex(s) for s in starts))
