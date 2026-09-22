#!/usr/bin/env python3
"""Contract for the canonical hermetic production NAPI build wrapper."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
TS_ROOT = ROOT / "src" / "ts"
WRAPPER = TS_ROOT / "scripts" / "build-native.mjs"


def main() -> None:
    package = json.loads((TS_ROOT / "package.json").read_text())
    assert package["scripts"]["build:native"] == "node scripts/build-native.mjs"
    assert WRAPPER.is_file()

    requested_tmp = Path("/tmp/fathomdb-napi-contract-owned")
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
    assert plan["temporary_directory"] == str(requested_tmp)
    assert plan["clean"][:2] == ["cargo", "clean"]
    assert "fathomdb-napi" in plan["clean"] and "--release" in plan["clean"]
    assert plan["build"] == [
        "npm",
        "exec",
        "--",
        "napi",
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
    for name in ("TMPDIR", "TMP", "TEMP"):
        assert plan["environment"][name] == str(requested_tmp)
    print("ok    napi-build-hermetic")


if __name__ == "__main__":
    main()
