"""The standalone SDK driver may repeat only repository-approved Cargo patches."""

from __future__ import annotations

import importlib.util
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "pin_guard", ROOT / "scripts/check-pinned-override-rot.py"
)
assert SPEC and SPEC.loader
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)


def test_driver_duplicate_patch_must_resolve_to_approved_root_source():
    manifest = "experiments/sdk_drivers/rust/Cargo.toml"
    valid = {
        "manifest": manifest,
        "mechanism": "patch.crates-io",
        "package": "libsqlite3-sys",
        "path": "../../../third_party/libsqlite3-sys-0.38.1",
        "git": None,
        "rev": None,
    }
    assert guard.sdk_driver_duplicate_patch(ROOT, valid)
    assert not guard.sdk_driver_duplicate_patch(
        ROOT, {**valid, "path": "../../../other/libsqlite3-sys"}
    )
    assert not guard.sdk_driver_duplicate_patch(
        ROOT, {**valid, "manifest": "other/Cargo.toml"}
    )


def test_driver_may_not_repeat_retired_cudarc_or_candle_patches():
    manifest = "experiments/sdk_drivers/rust/Cargo.toml"
    kernels = {
        "manifest": manifest,
        "mechanism": "patch.crates-io",
        "package": "candle-kernels",
        "path": None,
        "git": "https://github.com/coreyt/candle-fathomdb.git",
        "rev": "182ceab0547348993b6dbec4b1aed610dcc8b23f",
    }
    assert guard.sdk_driver_duplicate_patch(ROOT, kernels)
    assert not guard.sdk_driver_duplicate_patch(
        ROOT, {**kernels, "rev": "1aefdd008ad1c994635b688b8e6f2ae5a5a920ae"}
    )
    assert not guard.sdk_driver_duplicate_patch(
        ROOT, {**kernels, "package": "candle-core-fathomdb"}
    )
    assert not guard.sdk_driver_duplicate_patch(
        ROOT,
        {
            **kernels,
            "package": "cudarc",
            "path": "../../../third_party/cudarc-0.19.7",
            "git": None,
            "rev": None,
        },
    )
