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
