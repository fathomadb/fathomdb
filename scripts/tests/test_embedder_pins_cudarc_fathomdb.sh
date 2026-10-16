#!/usr/bin/env bash
# fathomdb-embedder's CUDA features pin cudarc-fathomdb exactly to the locked
# version (0.8.28 Slice 31, R-CODE-4 note 1). candle-core-fathomdb 0.11.0
# requires cudarc-fathomdb `0.19.10` as a caret requirement, so a crates.io
# consumer without a lock file would otherwise resolve any later 0.19.x. The
# pin is a direct optional dependency that enables no features of its own.
#
# Any host: reads `cargo metadata --no-deps` and Cargo.lock.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

metadata="$(mktemp)"
trap 'rm -f "$metadata"' EXIT
cargo metadata --no-deps --format-version 1 --locked >"$metadata"

python3 -I - "$ROOT" "$metadata" <<'PY'
import json
import pathlib
import sys

try:
    import tomllib
except ImportError:  # Python 3.10 (Ubuntu 22.04, Jetson); tomli is in [dev].
    import tomli as tomllib

root = pathlib.Path(sys.argv[1])
metadata = json.loads(pathlib.Path(sys.argv[2]).read_text())
lock = tomllib.loads((root / "Cargo.lock").read_text())
locked = [p["version"] for p in lock["package"] if p["name"] == "cudarc-fathomdb"]
failures = []
if len(locked) != 1:
    failures.append(f"Cargo.lock holds {len(locked)} cudarc-fathomdb entries, expected 1")

embedder = next(p for p in metadata["packages"] if p["name"] == "fathomdb-embedder")
deps = [d for d in embedder["dependencies"] if d["name"] == "cudarc-fathomdb" and d["kind"] is None]
if len(deps) != 1:
    failures.append(f"fathomdb-embedder has {len(deps)} normal cudarc-fathomdb dependencies, expected 1")
else:
    dep = deps[0]
    key = dep.get("rename") or dep["name"]
    if locked and dep["req"] != f"={locked[0]}":
        failures.append(f"cudarc-fathomdb requirement is {dep['req']!r}, expected '={locked[0]}' (the locked version)")
    if not dep["optional"]:
        failures.append("cudarc-fathomdb is not optional: the default build must not reach it")
    if dep["uses_default_features"] or dep["features"]:
        failures.append("cudarc-fathomdb pin enables features; it must add none (Candle selects them)")
    for feature in ("embed-cuda", "rerank-cuda"):
        if f"dep:{key}" not in embedder["features"].get(feature, []):
            failures.append(f"feature {feature} does not enable dep:{key}")

if failures:
    for failure in failures:
        print(f"FAIL: {failure}")
    sys.exit(1)
print(f"PASS: fathomdb-embedder pins cudarc-fathomdb ={locked[0]} under embed-cuda and rerank-cuda")
PY
