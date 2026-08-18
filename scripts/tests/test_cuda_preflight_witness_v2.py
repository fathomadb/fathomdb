#!/usr/bin/env python3
"""Behavioral RED/GREEN coverage for the CUDA preflight-v2 evidence contract."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from cuda_preflight_v2_fixture import CANDIDATE, OTHER_CANDIDATE, canonical, make_incompatible_fixture, make_valid


REPO = Path(__file__).resolve().parents[2]
VERIFIER = REPO / "scripts/release/verify-cuda-preflight-witness.py"
FIXTURES = REPO / "scripts/tests/fixtures/cuda-preflight-v2"


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


def rewrite_forced_message(
    root: Path,
    record_name: str,
    stdout_name: str,
    stderr_name: str,
    message: str,
    *,
    reseal_witness: bool,
) -> None:
    stdout_path = root / stdout_name
    stderr_path = root / stderr_name
    rewrite(stdout_path, lambda value: value["error"].__setitem__("message", message))
    stderr_path.write_text(f"{message}\n", encoding="utf-8")
    rewrite(
        root / record_name,
        lambda value: value.update(
            {
                "stdout_sha256": hashlib.sha256(stdout_path.read_bytes()).hexdigest(),
                "stderr_sha256": hashlib.sha256(stderr_path.read_bytes()).hexdigest(),
            }
        ),
    )
    if reseal_witness:
        for name in (record_name, stdout_name, stderr_name):
            reseal(root, name)


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        tmp = Path(directory)
        valid = tmp / "valid"
        shutil.copytree(FIXTURES / "valid", valid)
        regenerated = tmp / "regenerated"
        make_valid(regenerated, REPO)
        assert {
            path.relative_to(valid): path.read_bytes() for path in sorted(valid.rglob("*")) if path.is_file()
        } == {
            path.relative_to(regenerated): path.read_bytes()
            for path in sorted(regenerated.rglob("*")) if path.is_file()
        }, "committed valid fixture is not deterministic"
        result = run(valid)
        assert result.returncode == 0, result.stderr

        arbitrary_unavailable = tmp / "arbitrary-unavailable-message"
        shutil.copytree(valid, arbitrary_unavailable)
        rewrite_forced_message(
            arbitrary_unavailable,
            "forced-cuda-unavailable-python.json",
            "forced-cuda-unavailable-python-stdout.txt",
            "forced-cuda-unavailable-python-stderr.txt",
            "fabricated unavailable failure",
            reseal_witness=True,
        )
        reject(arbitrary_unavailable, "resealed arbitrary unavailable message")

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

        extra = tmp / "extra-root-member"
        shutil.copytree(valid, extra)
        (extra / "unexpected.txt").write_text("unexpected\n", encoding="utf-8")
        reject(extra, "unknown root member")

        for name, file_name, mutate in (
            ("capture-mismatch", "forced-cuda-unavailable-python-stdout.txt", lambda path: path.write_bytes(path.read_bytes().replace(b'"reason":"no_visible_cuda_device"', b'"reason":"cuda_probe_failed"'))),
            ("uuid-mismatch", "gpu-python-cuda-witness.json", lambda path: rewrite(path, lambda value: value.__setitem__("nvidia_smi_uuid", "GPU-other"))),
            ("pid-mismatch", "gpu-napi-cuda-witness.json", lambda path: rewrite(path, lambda value: value.__setitem__("nvidia_smi_compute_process_id", 4343))),
            ("cache-prefix", "smoke-cache-topology.json", lambda path: rewrite(path, lambda value: next(iter(value["smokes"].values()))["product_cache_files"].update({"fathomdb/embedders/deadbeefdead/config.json": next(iter(next(iter(value["smokes"].values()))["product_cache_files"].values()))}))),
            ("model-manifest", "model-cache-manifest.json", lambda path: rewrite(path, lambda value: value.__setitem__("revision", OTHER_CANDIDATE))),
            ("build-input", "build-input.json", lambda path: rewrite(path, lambda value: value.__setitem__("rerank_cuda", True))),
            ("harness-substitution", "forced-python-open.py", lambda path: path.write_text("raise SystemExit(0)\n", encoding="utf-8")),
            ("forced-unknown-field", "forced-cuda-unavailable-napi.json", lambda path: rewrite(path, lambda value: value.__setitem__("unknown", True))),
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

        fixtures = FIXTURES / "incompatible"
        for consumer in ("python", "napi"):
            regenerated_incompatible = tmp / f"regenerated-incompatible-{consumer}"
            generated = make_incompatible_fixture(regenerated_incompatible, consumer)
            names = tuple(path.name for path in generated)
            assert all(
                (fixtures / name).read_bytes() == (regenerated_incompatible / name).read_bytes()
                for name in names
            ), "committed incompatible fixture is not deterministic"
            record, stdout, stderr = (fixtures / name for name in names)
            result = run(valid, CANDIDATE, "--fixture-forced-record", str(record), str(stdout), str(stderr))
            assert result.returncode == 0, result.stderr

            rewrite_forced_message(
                regenerated_incompatible,
                record.name,
                stdout.name,
                stderr.name,
                "fabricated incompatible failure",
                reseal_witness=False,
            )
            result = run(
                valid,
                CANDIDATE,
                "--fixture-forced-record",
                str(record),
                str(stdout),
                str(stderr),
            )
            assert result.returncode != 0, f"accepted resealed arbitrary incompatible {consumer} message"

    print("CUDA preflight witness v2 behavioral tests passed")


if __name__ == "__main__":
    main()
