#!/usr/bin/env bash
# `embed-cuda` and `rerank-cuda` imply `tegra-pool` in every FathomDB crate
# that has a `tegra-pool` feature (0.8.28 Slice 31, owner ruling Q37,
# dev/plans/0.8.28/features/slice-31/design.md § 14.3).
#
# Cargo features do not propagate upward, so each crate must list its own
# `tegra-pool` in its CUDA features; otherwise, for example, the PyO3 import
# hook (gated on fathomdb-py's own `tegra-pool`) would not be compiled. The
# Node hook is gated only on aarch64 Linux with CUDA, never on `tegra-pool`;
# that asymmetry is what lets the Node addon initialise CUDA at registration
# without the pool feature, and this test records it.
#
# Any host: reads `cargo metadata --no-deps` and two source files.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

metadata="$(mktemp)"
trap 'rm -f "$metadata"' EXIT
cargo metadata --no-deps --format-version 1 --locked >"$metadata"

python3 -I - "$ROOT" "$metadata" <<'PY'
import json
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
metadata = json.loads(pathlib.Path(sys.argv[2]).read_text())
failures = []
checked = []
for package in metadata["packages"]:
    features = package.get("features", {})
    if "tegra-pool" not in features:
        continue
    for cuda in ("embed-cuda", "rerank-cuda"):
        if cuda not in features:
            continue
        checked.append(f"{package['name']}/{cuda}")
        if "tegra-pool" not in features[cuda]:
            failures.append(
                f"{package['name']}: feature {cuda} does not enable its own tegra-pool "
                f"({features[cuda]})"
            )

expected = {
    "fathomdb",
    "fathomdb-cli",
    "fathomdb-embedder",
    "fathomdb-engine",
    "fathomdb-napi",
    "fathomdb-py",
    "fathomdb-sdk",
}
seen = {name.split("/")[0] for name in checked}
if seen != expected:
    failures.append(f"crates with tegra-pool and a CUDA feature: {sorted(seen)}, expected {sorted(expected)}")

embedder_toml = (root / "src/rust/crates/fathomdb-embedder/Cargo.toml").read_text()
comment = re.search(r"((?:^#[^\n]*\n)+)tegra-pool = ", embedder_toml, re.M)
if not comment or "dev/plans/0.8.28/features/slice-31/design.md" not in comment.group(1) or "14.3" not in comment.group(1):
    failures.append("fathomdb-embedder's tegra-pool comment does not cite slice-31 design.md § 14.3")

napi_hook = (root / "src/rust/crates/fathomdb-napi/src/cuda_early_init.rs").read_text()
if "tegra-pool" in napi_hook:
    failures.append("fathomdb-napi/src/cuda_early_init.rs mentions tegra-pool: the Node hook must stay gated on aarch64 Linux + CUDA only")

if failures:
    for failure in failures:
        print(f"FAIL: {failure}")
    sys.exit(1)
print(f"PASS: tegra-pool implied by {len(checked)} CUDA features: {', '.join(sorted(checked))}")
PY
