"""Type-boundary regression for the Slice 65 installed-wheel control.

The control invokes a disposable ``test-hooks`` wheel, whose diagnostic
methods deliberately do not belong in the shipped ``_fathomdb`` stub.  This
test makes both sides of that boundary load-bearing: Pyright must accept the
control, and the public stub must continue to omit the private hooks.
"""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import pytest


_PYTHON_ROOT = Path(__file__).resolve().parents[1]
_REPO_ROOT = _PYTHON_ROOT.parents[1]
_CONTROL = _PYTHON_ROOT / "tests" / "test_slice65_wal_attribution_installed.py"
_PUBLIC_STUB = _PYTHON_ROOT / "fathomdb" / "_fathomdb.pyi"
_TEST_HOOKS = (
    "_arm_actual_checkpoint_observation_for_test",
    "_arm_binding_native_state_observation_for_test",
    "_arm_next_reader_completion_pause_for_test",
    "_arm_next_reader_snapshot_pause_for_test",
    "_checkpoint_at_rest_for_test",
    "_drain_actual_checkpoint_observations_for_test",
    "_drain_binding_native_state_observations_for_test",
    "_native_raw_wal_checkpoint_for_test",
    "_wal_attribution_binding_inventory_for_test",
    "_wal_attribution_binding_native_state_inventory_for_test",
    "_wal_attribution_checkpoint_records_for_test",
    "_wal_attribution_snapshot_for_test",
)


def _control_pyright_diagnostics() -> list[dict[str, object]]:
    pyright = shutil.which("pyright")
    if pyright is None:
        pytest.skip("pyright not installed; install via `pip install pyright`")

    result = subprocess.run(
        [
            pyright,
            "--project",
            str(_PYTHON_ROOT),
            "--venvpath",
            str(_REPO_ROOT),
            "--outputjson",
            str(_CONTROL),
        ],
        capture_output=True,
        check=False,
        text=True,
    )
    if not result.stdout.strip():
        pytest.fail(f"pyright produced no JSON output (exit {result.returncode}):\n{result.stderr}")
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        pytest.fail(f"could not parse pyright JSON output: {exc}\n{result.stdout}\n{result.stderr}")
        raise  # unreachable; satisfies type checkers
    diagnostics = report.get("generalDiagnostics")
    assert isinstance(diagnostics, list), f"pyright did not return diagnostics: {report!r}"
    return diagnostics


def test_slice65_installed_control_type_checks_without_shipping_test_hooks() -> None:
    """The control's local test-hook boundary must satisfy normal Pyright."""
    diagnostics = _control_pyright_diagnostics()
    errors = [
        diagnostic
        for diagnostic in diagnostics
        if diagnostic.get("severity") == "error"
        and diagnostic.get("file") == str(_CONTROL)
    ]
    assert not errors, "\n".join(str(error.get("message", error)) for error in errors)


def test_slice65_test_hooks_remain_absent_from_the_public_stub() -> None:
    """The typing boundary must not advertise test-only symbols to SDK users."""
    stub = _PUBLIC_STUB.read_text(encoding="utf-8")
    leaked = [hook for hook in _TEST_HOOKS if hook in stub]
    assert not leaked, f"test-only Slice 65 hooks leaked into the public stub: {leaked}"
