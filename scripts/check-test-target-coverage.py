#!/usr/bin/env python3
"""Fail when a workspace test target is run by no gate (0.8.27 RH-15).

Reads every crate's test targets and requirements (Cargo `required-features`
plus file-level `#![cfg(...)]`), the features `cargo test --workspace` unifies
on this host (`cargo metadata --filter-platform`), the committed
`scripts/test-feature-matrix.toml`, and `scripts/test-skip-allowlist.toml`. It
fails, naming the target or entry, when a target is covered by neither the
workspace gate nor the matrix, when the source-derived requirement sets drift
from the matrix, or when an allowlist entry is stale. No build.
"""

from __future__ import annotations

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
import test_targets  # noqa: E402


def main() -> int:
    root = test_targets.REPO_ROOT
    try:
        host = test_targets.host_triple()
        crates = test_targets.read_workspace(root)
        failures, gate_only = test_targets.check_coverage(
            crates,
            test_targets.workspace_features(root, host),
            test_targets.load_matrix(test_targets.MATRIX_PATH),
            test_targets.load_allowlist(test_targets.ALLOWLIST_PATH),
            host,
        )
    except test_targets.TestTargetsError as exc:
        print(f"FAIL check-test-target-coverage: {exc}", file=sys.stderr)
        return 1
    for failure in failures:
        print(f"FAIL check-test-target-coverage: {failure}", file=sys.stderr)
    if failures:
        return 1
    targets = sum(len(crate.targets) for crate in crates)
    print(
        f"ok    check-test-target-coverage: {targets} targets; "
        f"{len(gate_only)} run only by scripts/test-feature-complete.sh"
    )
    for target in gate_only:
        print(f"      feature-complete only: {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
