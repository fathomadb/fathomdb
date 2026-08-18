#!/usr/bin/env python3
"""Behavioral RED/GREEN coverage for the CUDA preflight-v2 evidence contract."""

from __future__ import annotations

import json
import hashlib
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from cuda_preflight_v2_fixture import CANDIDATE, OTHER_CANDIDATE, canonical, make_incompatible_fixture, make_valid


REPO = Path(__file__).resolve().parents[2]
VERIFIER = REPO / "scripts/release/verify-cuda-preflight-witness.py"


def run(root: Path, candidate: str = CANDIDATE, *extra: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(VERIFIER), "--witness-dir", str(root), "--candidate-sha", candidate, *extra],
        capture_output=True,
        check=False,
        text=True,
    )


def reject(root: Path, label: str, candidate: str = CANDIDATE) -> None:
    result = run(root, candidate)
    assert result.returncode != 0, f"accepted mutation: {label}"


def rewrite(path: Path, mutate) -> None:
    value = json.loads(path.read_bytes())
    mutate(value)
    path.write_bytes(canonical(value))


def reseal(root: Path, name: str) -> None:
    witness_path = root / "cuda-preflight-witness.json"
    witness = json.loads(witness_path.read_bytes())
    digest = hashlib.sha256((root / name).read_bytes()).hexdigest()
    witness["evidence_sha256"][name] = digest
    if name == "build-input.json":
        witness["build_input_sha256"] = digest
    elif name == "model-cache-manifest.json":
        witness["model_cache_manifest_sha256"] = digest
    witness_path.write_bytes(canonical(witness))


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        tmp = Path(directory)
        valid = tmp / "valid"
        make_valid(valid, REPO)
        result = run(valid)
        assert result.returncode == 0, result.stderr

        reject(valid, "candidate substitution", OTHER_CANDIDATE)

        for name, mutation in (
            ("v1", lambda value: value.__setitem__("schema_version", "fathomdb.cuda-preflight-witness/v1")),
            ("unknown", lambda value: value.__setitem__("unknown", True)),
            ("cpu-effective", lambda value: value.__setitem__("effective_device", "cpu")),
        ):
            target = tmp / name
            shutil.copytree(valid, target)
            path = target / ("cuda-preflight-witness.json" if name != "cpu-effective" else "forced-cuda-unavailable-python.json")
            rewrite(path, mutation)
            if name == "cpu-effective":
                reseal(target, "forced-cuda-unavailable-python.json")
            reject(target, name)

        noncanonical = tmp / "noncanonical"
        shutil.copytree(valid, noncanonical)
        path = noncanonical / "cuda-preflight-witness.json"
        path.write_bytes(json.dumps(json.loads(path.read_bytes()), indent=2).encode() + b"\n")
        reject(noncanonical, "noncanonical JSON")

        root_link = tmp / "root-link"
        shutil.copytree(valid, root_link)
        witness = root_link / "cuda-preflight-witness.json"
        witness.rename(root_link / "real.json")
        witness.symlink_to("real.json")
        reject(root_link, "root symlink")

        evidence_link = tmp / "evidence-link"
        shutil.copytree(valid, evidence_link)
        evidence = evidence_link / "environment.txt"
        evidence.rename(evidence_link / "environment-real.txt")
        evidence.symlink_to("environment-real.txt")
        reject(evidence_link, "evidence symlink")

        for name, file_name, mutate in (
            ("capture-mismatch", "forced-cuda-unavailable-python-stdout.txt", lambda path: path.write_bytes(path.read_bytes().replace(b'"reason":"no_visible_cuda_device"', b'"reason":"cuda_probe_failed"'))),
            ("uuid-mismatch", "gpu-python-cuda-witness.json", lambda path: rewrite(path, lambda value: value.__setitem__("nvidia_smi_uuid", "GPU-other"))),
            ("cache-prefix", "smoke-cache-topology.json", lambda path: rewrite(path, lambda value: next(iter(value["smokes"].values()))["product_cache_files"].update({"fathomdb/embedders/deadbeefdead/config.json": next(iter(next(iter(value["smokes"].values()))["product_cache_files"].values()))}))),
            ("substitution", "environment.txt", lambda path: path.write_text("substituted\n", encoding="utf-8")),
        ):
            target = tmp / name
            shutil.copytree(valid, target)
            mutate(target / file_name)
            if name != "substitution":
                reseal(target, file_name)
            reject(target, name)

        incomplete = tmp / "incomplete"
        shutil.copytree(valid, incomplete)
        (incomplete / "gpu-napi-cuda-smoke.txt").unlink()
        reject(incomplete, "missing evidence")

        fixtures = tmp / "incompatible"
        for consumer in ("python", "napi"):
            record, stdout, stderr = make_incompatible_fixture(fixtures, consumer)
            result = run(valid, CANDIDATE, "--fixture-forced-record", str(record), str(stdout), str(stderr))
            assert result.returncode == 0, result.stderr

    print("CUDA preflight witness v2 behavioral tests passed")


if __name__ == "__main__":
    main()
