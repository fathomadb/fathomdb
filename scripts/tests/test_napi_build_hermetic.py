#!/usr/bin/env python3
"""Contract for the canonical hermetic production NAPI build wrapper."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
TS_ROOT = ROOT / "src" / "ts"
WRAPPER = TS_ROOT / "scripts" / "build-native.mjs"


def main() -> None:
    package = json.loads((TS_ROOT / "package.json").read_text())
    assert package["scripts"]["build:native"] == "node scripts/build-native.mjs"
    assert WRAPPER.is_file()

    temporary_root = Path(tempfile.gettempdir())
    before = set(temporary_root.glob("fathomdb-napi-production-*"))
    secret = "must-not-appear-in-build-plan"
    no_requested_tmp = dict(os.environ)
    no_requested_tmp["FATHOMDB_PRINT_PLAN_SENTINEL"] = secret
    sanitized = subprocess.run(
        ["node", str(WRAPPER), "--print-plan"],
        cwd=TS_ROOT,
        env=no_requested_tmp,
        check=True,
        capture_output=True,
        text=True,
    )
    assert secret not in sanitized.stdout
    assert set(temporary_root.glob("fathomdb-napi-production-*")) == before

    with tempfile.TemporaryDirectory(prefix="fathomdb-napi-contract-") as directory:
        requested_tmp = Path(directory)
        environment = dict(os.environ)
        environment["FATHOMDB_NAPI_BUILD_TMPDIR"] = str(requested_tmp)
        completed = subprocess.run(
            ["node", str(WRAPPER), "--print-plan"],
            cwd=TS_ROOT,
            env=environment,
            check=True,
            capture_output=True,
            text=True,
        )
    plan = json.loads(completed.stdout)
    assert plan["temporary_directory"] == str(requested_tmp.resolve())
    assert plan["clean"][:2] == ["cargo", "clean"]
    assert "fathomdb-napi" in plan["clean"] and "--release" in plan["clean"]
    expected_napi_args = [
        "build",
        "--platform",
        "--release",
        "--cargo-cwd",
        "../rust/crates/fathomdb-napi",
        "--features",
        "default-embedder",
        "--js",
        "false",
    ]
    if sys.platform == "win32":
        assert Path(plan["build"][0]).name == "node.exe"
        assert plan["build"][2:] == expected_napi_args
    else:
        assert plan["build"] == ["npm", "exec", "--", "napi", *expected_napi_args]
    windows_plan_environment = dict(environment)
    windows_plan_environment["FATHOMDB_NAPI_BUILD_PRINT_PLATFORM"] = "win32"
    windows_plan = subprocess.run(
        ["node", str(WRAPPER), "--print-plan"],
        cwd=TS_ROOT,
        env=windows_plan_environment,
        check=True,
        capture_output=True,
        text=True,
    )
    windows_build = json.loads(windows_plan.stdout)["build"]
    assert Path(windows_build[0]).name in {"node", "node.exe"}
    assert windows_build[1].endswith("@napi-rs/cli/scripts/index.js") or windows_build[1].endswith(
        "@napi-rs\\cli\\scripts\\index.js"
    )
    assert windows_build[2:] == expected_napi_args
    for name in ("TMPDIR", "TMP", "TEMP"):
        assert plan["environment"][name] == str(requested_tmp.resolve())
    assert set(plan["environment"]) == {"TMPDIR", "TMP", "TEMP"}

    if "--execute" in sys.argv[1:]:
        subprocess.run(["npm", "run", "build:native:debug"], cwd=TS_ROOT, check=True)
        debug_declarations = (TS_ROOT / "index.d.ts").read_text()
        for hook in ("forcePanicForTest", "forcePanicInAccessorForTest"):
            assert hook in debug_declarations
        subprocess.run(["npm", "run", "build:native"], cwd=TS_ROOT, check=True)
        production_declarations = (TS_ROOT / "index.d.ts").read_text()
        for hook in ("forcePanicForTest", "forcePanicInAccessorForTest"):
            assert hook not in production_declarations
    print("ok    napi-build-hermetic")


if __name__ == "__main__":
    main()
