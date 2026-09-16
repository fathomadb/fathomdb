#!/usr/bin/env python3
"""Slice 50 P26-09 single-source private-hook inventory contract."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "scripts/release/smoke/python-test-hooks-v1.json"
GATE = ROOT / "src/python/tests/_test_hooks_gate.py"
TYPING = ROOT / "src/python/tests/test_slice65_wal_attribution_typing.py"
WINDOWS_GUARD = ROOT / "scripts/tests/test_windows_wal_attribution_ci_job.sh"
WORKFLOW = ROOT / ".github/workflows/ci.yml"
PY_BINDING = ROOT / "src/rust/crates/fathomdb-py/src/lib.rs"
INSTALLED = ROOT / "src/python/tests/test_slice65_wal_attribution_installed.py"


def load_gate():
    spec = importlib.util.spec_from_file_location("_test_hooks_gate_slice50", GATE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> None:
    assert CONTRACT.is_file(), f"missing hook contract: {CONTRACT}"
    payload = json.loads(CONTRACT.read_text(encoding="utf-8"))
    assert payload["schema_version"] == "fathomdb.python-test-hooks/v1"
    assert set(payload) == {"schema_version", "symbols"}
    rows = payload["symbols"]
    assert rows and len(rows) == len({(row["owner"], row["attribute"]) for row in rows})
    assert all(set(row) == {"owner", "attribute"} for row in rows)

    gate = load_gate()
    expected = tuple((row["owner"], row["attribute"]) for row in rows)
    assert gate.TEST_HOOK_SYMBOLS == expected
    assert gate.load_test_hook_symbols(CONTRACT) == expected
    assert gate.hook_surface_drift(PY_BINDING.read_text(encoding="utf-8")) == ((), ())

    typing = TYPING.read_text(encoding="utf-8")
    assert "_TEST_HOOKS = (" not in typing
    assert "load_test_hook_symbols" in typing
    guard = WINDOWS_GUARD.read_text(encoding="utf-8")
    assert "HOOK_CONTRACT=" in guard
    assert "python-test-hooks-v1.json" in guard
    assert "installed-wheel hook contract is complete" in guard
    workflow = WORKFLOW.read_text(encoding="utf-8")
    assert "python-test-hooks-v1.json" in workflow
    assert workflow.count("--test-hooks-contract") >= 3
    installed = INSTALLED.read_text(encoding="utf-8")
    assert "--test-hooks-contract" in installed
    assert "load_test_hook_symbols" in installed
    print("PASS test-slice50-hook-inventory")


if __name__ == "__main__":
    main()
