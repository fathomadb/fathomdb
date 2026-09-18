#!/usr/bin/env python3
"""Contract tests for the Slice 65 candidate-manifest wrapper."""

from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BASE_SCRIPT = ROOT / "scripts/release/slice50-candidate-manifest.py"
WRAPPER_SCRIPT = ROOT / "scripts/release/slice65-candidate-manifest.py"
BASE_MANIFEST = ROOT / "dev/plans/0.8.26/features/slice-50/candidate-manifest.json"
QUALIFICATION = {
    "design_lifecycle": "pass",
    "sdk_surface_parity": "pass",
    "slice60_owner_probes": "pass",
}


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def rejected(module, payload: object) -> None:
    try:
        module.validate_manifest(payload)
    except ValueError:
        return
    raise AssertionError("malformed Slice 65 manifest was accepted")


def main() -> None:
    assert BASE_SCRIPT.is_file()
    assert BASE_MANIFEST.is_file()
    assert WRAPPER_SCRIPT.is_file(), f"missing Slice 65 wrapper: {WRAPPER_SCRIPT}"

    base_bytes = BASE_MANIFEST.read_bytes()
    base = json.loads(base_bytes)
    slice50 = load_module(BASE_SCRIPT, "slice50_candidate_manifest_test")
    wrapper = load_module(WRAPPER_SCRIPT, "slice65_candidate_manifest_test")
    slice50.validate_manifest(base)

    payload = {
        "schema_version": "fathomdb.slice65-candidate/v1",
        "candidate_sha": base["candidate_sha"],
        "base_candidate": base,
        "qualification": QUALIFICATION,
    }
    assert wrapper.validate_manifest(copy.deepcopy(payload)) == payload

    for key in payload:
        malformed = copy.deepcopy(payload)
        del malformed[key]
        rejected(wrapper, malformed)

    malformed = copy.deepcopy(payload)
    malformed["unexpected"] = True
    rejected(wrapper, malformed)

    malformed = copy.deepcopy(payload)
    malformed["base_candidate"]["schema_version"] = "fathomdb.slice65-candidate/v1"
    rejected(wrapper, malformed)

    malformed = copy.deepcopy(payload)
    malformed["candidate_sha"] = "f" * 40
    rejected(wrapper, malformed)

    for key in QUALIFICATION:
        malformed = copy.deepcopy(payload)
        del malformed["qualification"][key]
        rejected(wrapper, malformed)

    malformed = copy.deepcopy(payload)
    malformed["qualification"]["unexpected"] = "pass"
    rejected(wrapper, malformed)

    malformed = copy.deepcopy(payload)
    malformed["qualification"]["design_lifecycle"] = "fail"
    rejected(wrapper, malformed)

    with tempfile.TemporaryDirectory() as temporary:
        manifest = Path(temporary) / "slice65.json"
        manifest.write_text(json.dumps(payload), encoding="utf-8")
        result = subprocess.run(
            [
                sys.executable,
                str(WRAPPER_SCRIPT),
                "validate",
                "--manifest",
                str(manifest),
            ],
            text=True,
            capture_output=True,
            check=False,
        )
        assert result.returncode == 0, result.stderr

    assert BASE_MANIFEST.read_bytes() == base_bytes
    print("All Slice 65 candidate-manifest tests passed")


if __name__ == "__main__":
    main()
