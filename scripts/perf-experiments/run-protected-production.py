#!/usr/bin/env python3
"""Run the six directional production-only protected-write cells."""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SEALED_RUNNER = ROOT / "scripts" / "perf-experiments" / "run_slice71b.py"
SEALED_MANIFEST = (
    ROOT
    / "experiments"
    / "configs"
    / "release-0825-slice71b-attribution-manifest.v1.json"
)


def _exact_keys(value: dict[str, object], expected: set[str], label: str) -> None:
    unknown = set(value) - expected
    missing = expected - set(value)
    if unknown or missing:
        raise ValueError(
            f"{label} shape drifted; missing={sorted(missing)} unknown={sorted(unknown)}"
        )


def _load_manifest(path: Path, runner: object) -> dict[str, object]:
    """Validate the separate directional contract and its sealed base identity."""
    manifest = json.loads(path.read_text(encoding="utf-8"))
    _exact_keys(
        manifest,
        {
            "schema_version",
            "base_manifest",
            "fixtures",
            "treatment",
            "repetitions_per_fixture",
            "environment_policy",
            "timeouts_s",
        },
        "directional manifest",
    )
    if (
        manifest["schema_version"]
        != "fathomdb.performance-gauntlet.protected-production-manifest/v1"
        or manifest["fixtures"] != ["scale02", "ac013"]
        or manifest["treatment"] != "production"
        or manifest["repetitions_per_fixture"] != 3
    ):
        raise ValueError("directional protected-write workload identity drifted")
    base = manifest["base_manifest"]
    _exact_keys(base, {"path", "sha256"}, "base_manifest")
    expected_base = (ROOT / str(base["path"])).resolve()
    if (
        expected_base != SEALED_MANIFEST.resolve()
        or runner.sha256(expected_base) != base["sha256"]
    ):
        raise ValueError("sealed protected-write manifest identity drifted")
    sealed = json.loads(expected_base.read_text(encoding="utf-8"))
    if manifest["timeouts_s"] != sealed["timeouts_s"]:
        raise ValueError("directional protected-write timeouts drifted")
    expected_policy = dict(sealed["environment_policy"])
    expected_policy["max_swap_io_delta"] = 1024
    if manifest["environment_policy"] != expected_policy:
        raise ValueError("directional protected-write environment policy drifted")
    return manifest


def _validate_receipt(receipt: dict[str, object]) -> None:
    """Reject a nominal completion without six exact successful cells."""
    cells = receipt.get("cells")
    if not isinstance(cells, list):
        raise ValueError("protected-write receipt cells are unavailable")
    expected = {
        (fixture, "production", ordinal)
        for fixture in ("scale02", "ac013")
        for ordinal in range(1, 4)
    }
    actual = {
        (cell.get("fixture"), cell.get("treatment"), cell.get("ordinal"))
        for cell in cells
        if isinstance(cell, dict)
    }
    if actual != expected or any(
        not cell.get("environment_valid") for cell in cells if isinstance(cell, dict)
    ):
        raise ValueError("protected-write receipt does not contain six valid cells")


def _write_json_atomic(path: Path, document: object) -> None:
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    os.replace(temporary, path)


def _load_runner():
    spec = importlib.util.spec_from_file_location("gauntlet_slice71b", SEALED_RUNNER)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load protected-write implementation")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> int:
    """Build the existing probe once and execute production three times per fixture."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args()
    runner = _load_runner()
    source_root = args.source_root.resolve()
    output_root = args.output_root.resolve()
    if output_root.exists():
        parser.error("protected-write output root must be new")
    source_ref, dirty = runner._source_identity(source_root)
    if dirty:
        parser.error("protected-write target source must be clean")
    directional = _load_manifest(args.manifest.resolve(), runner)
    document = json.loads(SEALED_MANIFEST.read_text(encoding="utf-8"))
    document["product_sources"]["unchanged_current"] = source_ref
    document["environment_policy"] = directional["environment_policy"]
    document["timeouts_s"] = directional["timeouts_s"]
    document["runner"] = {
        "path": str(Path(__file__).resolve()),
        "sha256": runner.sha256(Path(__file__).resolve()),
    }
    output_root.mkdir(parents=True)
    build_root = output_root / "build"
    build_root.mkdir()
    build_log = build_root / "build.log"
    target_root = Path(tempfile.mkdtemp(prefix="fathomdb-gauntlet-protected-"))
    lock_root = Path(tempfile.mkdtemp(prefix="fathomdb-gauntlet-protected-lock-"))
    cells = []
    failures = []
    try:
        manifest = runner._write_probe_manifest(lock_root, source_root)
        subprocess.run(
            [
                "cargo",
                "generate-lockfile",
                "--offline",
                "--manifest-path",
                str(manifest),
            ],
            cwd=source_root,
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        lock_path = lock_root / "Cargo.lock"
        executable, build_identity = runner._build_probe(
            source_root,
            target_root,
            lock_path,
            build_log,
            int(document["timeouts_s"]["build"]),
        )
        libsqlite = runner._libsqlite3_sys_version(lock_path)
        for fixture in directional["fixtures"]:
            for ordinal in range(1, int(directional["repetitions_per_fixture"]) + 1):
                try:
                    cells.append(
                        runner._run_cell(
                            document=document,
                            source_ref=source_ref,
                            executable=executable,
                            build_identity=build_identity,
                            libsqlite3_sys=libsqlite,
                            fixture=fixture,
                            treatment=str(directional["treatment"]),
                            ordinal=ordinal,
                            cell_root=output_root
                            / str(fixture)
                            / f"production-{ordinal}",
                        )
                    )
                except runner.CampaignAbort as error:
                    failures.append(error.failure)
                    break
            if failures:
                break
    finally:
        shutil.rmtree(target_root, ignore_errors=True)
        shutil.rmtree(lock_root, ignore_errors=True)
    receipt = {
        "schema_version": "fathomdb.performance-gauntlet.protected-production/v1",
        "source_commit": source_ref,
        "sealed_runner": str(SEALED_RUNNER),
        "sealed_manifest": str(SEALED_MANIFEST),
        "directional_manifest": str(args.manifest.resolve()),
        "directional_manifest_sha256": runner.sha256(args.manifest.resolve()),
        "cells": cells,
        "failures": failures,
        "state": "complete" if len(cells) == 6 and not failures else "incomplete",
    }
    if receipt["state"] == "complete":
        _validate_receipt(receipt)
    receipt_path = output_root / "receipt.json"
    _write_json_atomic(receipt_path, receipt)
    print(receipt_path)
    return 0 if receipt["state"] == "complete" else 1


if __name__ == "__main__":
    raise SystemExit(main())
