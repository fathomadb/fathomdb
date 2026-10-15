#!/usr/bin/env python3
"""PTX driver-floor check (0.8.28 Slice 31, T-CUDA-COMPAT C1, AC31-15).

A CUDA driver JIT-compiles a PTX module only if it supports the module's PTX
ISA version. The 12.6 artifact's driver floor (560.28.03) supports PTX ISA
8.5, so every module FathomDB ships must declare `.version` <= 8.5, and its
`.target` must be exactly the compute capability the release contract builds
for (`sm_75` on x86_64, `sm_87` for the Jetson Orin): a later target would
not load on the floor GPU at all.

Inputs: --ptx-dir DIR (every *.ptx file below it, e.g. Candle's kernel
OUT_DIR) and/or --artifact FILE (PTX text embedded in a binary), and
--target sm_NN. Exit 0 when every module passes and at least one was found.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

PTX_ISA_FLOOR = (8, 5)
VERSION = re.compile(rb"^\s*\.version\s+(\d+)\.(\d+)\s*$", re.MULTILINE)
TARGET = re.compile(rb"^\s*\.target\s+([A-Za-z0-9_]+)", re.MULTILINE)


def modules_in(name: str, data: bytes) -> list[tuple[str, bytes]]:
    """Split text that may hold several PTX modules at each `.version` line."""
    starts = [match.start() for match in VERSION.finditer(data)]
    if not starts:
        return []
    bounds = [*starts, len(data)]
    if len(starts) == 1:
        return [(name, data[starts[0]:])]
    return [(f"{name}[{index}]", data[bounds[index]:bounds[index + 1]]) for index in range(len(starts))]


def check_module(label: str, text: bytes, target: str) -> list[str]:
    failures = []
    version = VERSION.search(text)
    targets = TARGET.findall(text)
    if version is None:
        return [f"{label}: no .version line"]
    major, minor = int(version.group(1)), int(version.group(2))
    if (major, minor) > PTX_ISA_FLOOR:
        failures.append(
            f"{label}: .version {major}.{minor} is above the driver floor "
            f"{PTX_ISA_FLOOR[0]}.{PTX_ISA_FLOOR[1]}"
        )
    if not targets:
        failures.append(f"{label}: no .target line")
    for found in targets:
        found_text = found.decode("ascii")
        if found_text != target:
            failures.append(f"{label}: .target {found_text} is not {target}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--ptx-dir", type=Path, action="append", default=[])
    parser.add_argument("--artifact", type=Path, action="append", default=[])
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"sm_\d{2,3}", args.target):
        print(f"FAIL  ptx-floor: --target must look like sm_75, got {args.target!r}", file=sys.stderr)
        return 2
    modules: list[tuple[str, bytes]] = []
    for directory in args.ptx_dir:
        for path in sorted(directory.rglob("*.ptx")):
            modules.extend(modules_in(str(path.relative_to(directory)), path.read_bytes()))
    for artifact in args.artifact:
        modules.extend(modules_in(artifact.name, artifact.read_bytes()))
    if not modules:
        print("FAIL  ptx-floor: no PTX module found", file=sys.stderr)
        return 1
    failures = [failure for label, text in modules for failure in check_module(label, text, args.target)]
    if failures:
        for failure in failures:
            print(f"FAIL  ptx-floor: {failure}", file=sys.stderr)
        return 1
    print(
        f"ok ptx-floor: {len(modules)} PTX module(s), each .version <= "
        f"{PTX_ISA_FLOOR[0]}.{PTX_ISA_FLOOR[1]} and .target {args.target}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
