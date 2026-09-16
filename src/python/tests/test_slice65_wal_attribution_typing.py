"""Type-boundary regression for the Slice 65 installed-wheel control.

The control invokes a disposable ``test-hooks`` wheel, whose diagnostic
methods deliberately do not belong in the shipped ``_fathomdb`` stub.  This
test makes both sides of that boundary load-bearing: Pyright must accept the
control, and the public stub must continue to omit the private hooks.
"""

from __future__ import annotations

import ast
import json
import shutil
import subprocess
from pathlib import Path

import pytest

from _test_hooks_gate import load_test_hook_symbols


_PYTHON_ROOT = Path(__file__).resolve().parents[1]
_REPO_ROOT = _PYTHON_ROOT.parents[1]
_CONTROL = _PYTHON_ROOT / "tests" / "test_slice65_wal_attribution_installed.py"
_PUBLIC_STUB = _PYTHON_ROOT / "fathomdb" / "_fathomdb.pyi"


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
    module = ast.parse(_PUBLIC_STUB.read_text(encoding="utf-8"))
    module_functions = {
        node.name for node in module.body if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
    }
    class_functions = {
        node.name: {
            member.name
            for member in node.body
            if isinstance(member, (ast.FunctionDef, ast.AsyncFunctionDef))
        }
        for node in module.body
        if isinstance(node, ast.ClassDef)
    }
    leaked = [
        attribute if owner is None else f"{owner}.{attribute}"
        for owner, attribute in load_test_hook_symbols()
        if (
            attribute in module_functions
            if owner is None
            else attribute in class_functions.get(owner, set())
        )
    ]
    assert not leaked, f"test-only Slice 65 hooks leaked into the public stub: {leaked}"
