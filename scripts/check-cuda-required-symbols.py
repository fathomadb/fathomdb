#!/usr/bin/env python3
"""Required CUDA symbol drift guard (0.8.28 Slice 31, owner ruling Q26).

Before building a CUDA device, the embedder refuses a CUDA 12 cuBLAS or
cuRAND that lacks a symbol FathomDB's CUDA path will call
(`cuda_libraries.rs`, REQUIRED_CUBLAS_SYMBOLS and REQUIRED_CURAND_SYMBOLS).
Every such call goes through Candle, and Candle reaches these libraries only
through cudarc's `cublas` and `curand` result layers. This guard fails when:

  (a) the constants differ from the `sys::` functions called in cudarc's
      `src/cublas/result.rs` and `src/curand/result.rs`;
  (b) Candle's sources call a `cublas` or `curand` sys function directly, or
      a cuBLAS safe-layer method outside the result layer
      (`set_pointer_mode`, `get_pointer_mode`, grouped GEMM);
  (c) a FathomDB crate calls a `cublas` or `curand` sys function.

Inputs: --root DIR (the resolved cudarc-fathomdb and candle-*-fathomdb
sources from `cargo metadata --offline --locked`, FathomDB's crates under
src/rust/crates, and the embedder's src/cuda_libraries.rs), or explicit
--cudarc-src, --candle-src, --fathomdb-src and --constants for fixtures.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

CONSTANTS_FILE = "src/rust/crates/fathomdb-embedder/src/cuda_libraries.rs"
CALL = re.compile(r"\bsys::((?:cublas|curand)[A-Z][A-Za-z0-9_]*)\s*\(")
DIRECT_CALL = re.compile(r"\b(?:cublas|curand)::sys::((?:cublas|curand)[A-Z][A-Za-z0-9_]*)\s*\(")
SAFE_OUTSIDE_RESULT = re.compile(r"\b(set_pointer_mode|get_pointer_mode|grouped_gemm|GroupedGemm)\b")


class Precondition(Exception):
    """The inputs cannot support a verdict."""


def constant(text: str, name: str) -> list[str]:
    match = re.search(rf"\b{name}\s*:\s*\[&(?:'static\s+)?str;\s*\d+\]\s*=\s*\[(.*?)\];", text, re.DOTALL)
    if match is None:
        raise Precondition(f"no {name} constant")
    return re.findall(r'"([^"]+)"', match.group(1))


def called(result_rs: Path) -> list[str]:
    try:
        return sorted(set(CALL.findall(result_rs.read_text(encoding="utf-8"))))
    except OSError as exc:
        raise Precondition(f"cannot read {result_rs}: {exc}") from exc


def rust_files(directory: Path) -> list[Path]:
    return sorted(path for path in directory.rglob("*.rs") if "target" not in path.parts)


def direct_calls(directory: Path, label: str, safe_layer: bool) -> list[str]:
    failures = []
    for path in rust_files(directory):
        text = path.read_text(encoding="utf-8")
        for regex in (CALL, DIRECT_CALL):
            for name in sorted(set(regex.findall(text))):
                failures.append(f"{label} calls {name} directly ({path})")
        if safe_layer:
            for name in sorted(set(SAFE_OUTSIDE_RESULT.findall(text))):
                failures.append(f"{label} uses the cuBLAS safe-layer {name} outside the result layer ({path})")
    return sorted(set(failures))


def resolved_sources(root: Path) -> tuple[Path, list[Path]]:
    result = subprocess.run(
        ["cargo", "metadata", "--offline", "--locked", "--format-version", "1"],
        cwd=root, capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise Precondition(f"cargo metadata failed: {result.stderr.strip()}")
    packages = json.loads(result.stdout)["packages"]
    cudarc = [Path(p["manifest_path"]).parent for p in packages if p["name"] == "cudarc-fathomdb"]
    candle = [
        Path(p["manifest_path"]).parent for p in packages
        if p["name"].startswith("candle-") and p["name"].endswith("-fathomdb")
    ]
    if len(cudarc) != 1:
        raise Precondition(f"expected one resolved cudarc-fathomdb source, found {len(cudarc)}")
    if not candle:
        raise Precondition("no resolved candle-*-fathomdb source")
    return cudarc[0], candle


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path)
    parser.add_argument("--cudarc-src", type=Path)
    parser.add_argument("--candle-src", type=Path, action="append", default=[])
    parser.add_argument("--fathomdb-src", type=Path)
    parser.add_argument("--constants", type=Path)
    args = parser.parse_args()
    try:
        if args.root is not None:
            cudarc_src, candle_srcs = resolved_sources(args.root)
            fathomdb_src = args.root / "src/rust/crates"
            constants_path = args.root / CONSTANTS_FILE
        else:
            if not (args.cudarc_src and args.candle_src and args.fathomdb_src and args.constants):
                raise Precondition("without --root, give --cudarc-src, --candle-src, --fathomdb-src and --constants")
            cudarc_src, candle_srcs = args.cudarc_src, args.candle_src
            fathomdb_src, constants_path = args.fathomdb_src, args.constants
        try:
            constants_text = constants_path.read_text(encoding="utf-8")
        except OSError as exc:
            raise Precondition(f"cannot read {constants_path}: {exc}") from exc
        required = {
            "cuBLAS": ("REQUIRED_CUBLAS_SYMBOLS", constant(constants_text, "REQUIRED_CUBLAS_SYMBOLS"),
                       called(cudarc_src / "src/cublas/result.rs")),
            "cuRAND": ("REQUIRED_CURAND_SYMBOLS", constant(constants_text, "REQUIRED_CURAND_SYMBOLS"),
                       called(cudarc_src / "src/curand/result.rs")),
        }
    except Precondition as exc:
        print(f"UNVERIFIED cuda-required-symbols: {exc}", file=sys.stderr)
        return 2

    failures: list[str] = []
    for library, (name, declared, used) in required.items():
        missing = sorted(set(used) - set(declared))
        extra = sorted(set(declared) - set(used))
        if missing:
            failures.append(f"{library}: called by the result layer but not in {name}: {', '.join(missing)}")
        if extra:
            failures.append(f"{library}: in {name} but not called by the result layer: {', '.join(extra)}")
        if len(declared) != len(set(declared)):
            failures.append(f"{library}: {name} names a symbol twice")
    for candle_src in candle_srcs:
        failures.extend(direct_calls(candle_src / "src", "Candle", safe_layer=True))
    failures.extend(direct_calls(fathomdb_src, "FathomDB", safe_layer=False))
    if failures:
        for failure in failures:
            print(f"FAIL  cuda-required-symbols: {failure}", file=sys.stderr)
        return 1
    print(
        f"ok cuda-required-symbols: {len(required['cuBLAS'][1])} cuBLAS and "
        f"{len(required['cuRAND'][1])} cuRAND names match cudarc's result layers; "
        "no direct sys call from Candle or FathomDB"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
