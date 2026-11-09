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
    assert (REPO / "scripts/release/cuda-rerank-preflight-witness.schema.json").is_file()
    with tempfile.TemporaryDirectory() as directory:
        tmp = Path(directory)
        valid = tmp / "valid"
        # Frozen v1 envelope bytes remain historical inputs and must now refuse.
        historical_result = run(FIXTURES / "valid")
        assert historical_result.returncode != 0, "accepted a historical v1 allocation witness"
        make_valid(valid, REPO)
        current = json.loads(
            (REPO / "scripts/tests/fixtures/tegra-gpu-witness/v2/global-free.json").read_bytes()
        )
        assert set(current) == {
            "schema", "process_id", "device_integrated", "device_ordinal_requested",
            "device_ordinal_actual", "device_uuid", "device_name", "compute_capability",
            "free_before_bytes", "free_after_bytes", "total_bytes", "delta_bytes",
            "delta_floor_bytes", "embedded_vector_dim", "measurement",
        }
        measurement = current["measurement"]
        assert set(measurement) == {
            "kind", "control_allocation_request_bytes", "control_block_count",
            "control_free_before_bytes", "control_free_after_bytes", "control_delta_bytes",
            "sole_gpu_consumer_precondition",
        }
        assert current["schema"] == "fathomdb.tegra-gpu-allocation-witness/v2"
        assert current["device_integrated"] is False and current["process_id"] == 4242
        assert current["delta_bytes"] == current["free_before_bytes"] - current["free_after_bytes"] == 200_000_000
        assert measurement["control_delta_bytes"] == measurement["control_free_before_bytes"] - measurement["control_free_after_bytes"] == 300_000_000
        assert measurement["kind"] == "global_free" and measurement["control_block_count"] == 1
        assert measurement["control_allocation_request_bytes"] == 268_435_456
        for consumer in ("python", "napi"):
            observed = json.loads((valid / f"gpu-{consumer}-cuda-witness.json").read_bytes())
            assert observed["allocation_witness"] == current

        legacy = tmp / "legacy-no-allocation-witness"
        shutil.copytree(valid, legacy)
        for consumer in ("python", "napi"):
            record_name = f"gpu-{consumer}-cuda-witness.json"
            rewrite(legacy / record_name, lambda value: value.pop("allocation_witness"))
            reseal(legacy, record_name)
        legacy_result = run(legacy)
        assert legacy_result.returncode != 0, (
            "accepted GPU observations with no in-process allocation witness"
        )
        assert "allocation witness" in legacy_result.stderr, legacy_result.stderr

        result = run(valid)
        assert result.returncode == 0, result.stderr

        mutations = (
            ("zero model delta", lambda w: w.update(free_after_bytes=w["free_before_bytes"], delta_bytes=0)),
            ("negative model delta", lambda w: w.update(free_after_bytes=w["free_before_bytes"] + 1, delta_bytes=-1)),
            ("below floor", lambda w: w.update(free_after_bytes=w["free_before_bytes"] - 1024, delta_bytes=1024)),
            ("weak floor", lambda w: w.update(delta_floor_bytes=1024)),
            ("global arithmetic", lambda w: w.update(delta_bytes=w["delta_bytes"] + 1)),
            ("missing control", lambda w: w["measurement"].pop("control_delta_bytes")),
            ("unobserved control", lambda w: w["measurement"].update(control_free_after_bytes=41_999_999_999, control_delta_bytes=1)),
            ("false control delta", lambda w: w["measurement"].update(control_delta_bytes=1)),
            ("zero blocks", lambda w: w["measurement"].update(control_block_count=0)),
            ("too many blocks", lambda w: w["measurement"].update(control_block_count=17)),
            ("zero request", lambda w: w["measurement"].update(control_allocation_request_bytes=0)),
            ("wrong precondition", lambda w: w["measurement"].update(sole_gpu_consumer_precondition="fine")),
            ("wrong PID", lambda w: w.update(process_id=4243)),
            ("wrong forward", lambda w: w.update(embedded_vector_dim=383)),
            ("wrong hardware", lambda w: w.update(device_integrated=True)),
            ("mixed fields", lambda w: w["measurement"].update(used_before_bytes=0)),
            ("wrong method", lambda w: w["measurement"].update(kind="private_pool_used")),
        )
        for consumer in ("python", "napi"):
            for index, (label, mutation) in enumerate(mutations):
                broken = tmp / f"allocation-{consumer}-{index}"
                shutil.copytree(valid, broken)
                record_name = f"gpu-{consumer}-cuda-witness.json"
                rewrite(broken / record_name, lambda observation: mutation(observation["allocation_witness"]))
                reseal(broken, record_name)
                reject(broken, f"{consumer} {label}")

        # Tegra's NVIDIA-SMI omits `GPU-`, though it identifies the same
        # driver UUID as the in-process and selected-device records.
        tegra_uuid = tmp / "tegra-bare-nvidia-smi-uuid"
        shutil.copytree(valid, tegra_uuid)
        for consumer in ("python", "napi"):
            record_name = f"gpu-{consumer}-cuda-witness.json"
            rewrite(
                tegra_uuid / record_name,
                lambda value: value.__setitem__(
                    "nvidia_smi_uuid", value["nvidia_smi_uuid"].removeprefix("GPU-")
                ),
            )
            reseal(tegra_uuid, record_name)
        result = run(tegra_uuid)
        assert result.returncode == 0, result.stderr

        rerank_v3 = tmp / "rerank-v3"
        shutil.copytree(valid, rerank_v3)
        rewrite(
            rerank_v3 / "build-input.json",
            lambda value: value.update(
                {
                    "schema_version": "fathomdb.cuda-preflight-build-input/v3",
                    "python_features": ["embed-cuda", "rerank-cuda", "pyo3/extension-module"],
                    "napi_features": ["default-embedder", "embed-cuda", "rerank-cuda"],
                    "rerank_cuda": True,
                }
            ),
        )
        reseal(rerank_v3, "build-input.json")
        rewrite(
            rerank_v3 / "cuda-preflight-witness.json",
            lambda value: value.__setitem__("schema_version", "fathomdb.cuda-preflight-witness/v3"),
        )
        assert run(rerank_v3).returncode != 0, "v3 witness without reranker evidence must be rejected"
        reranker_manifest = {
            "schema_version": "fathomdb.reranker-cache/v1",
            "repository": "cross-encoder/ms-marco-TinyBERT-L2-v2",
            "revision": "81d1926f67cb8eee2c2be17ca9f793c7c3bd20cc",
            "snapshot_relpath": "fathomdb/reranker/0290849b0459",
            "files": {"config.json": "2144195e107cd7ea61556478e7add12986ebfbc3085f924fc0b90c2410604879", "tokenizer.json": "d241a60d5e8f04cc" + "1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66", "model.safetensors": "a0e7364ddf91ff7028f1102e1b91ac7a72e3db4061241bd84efe45c72c9af03a"},
        }
        (rerank_v3 / "reranker-cache-manifest.json").write_bytes(canonical(reranker_manifest))
        reranker_digest = hashlib.sha256((rerank_v3 / "reranker-cache-manifest.json").read_bytes()).hexdigest()
        for consumer in ("python", "napi"):
            (rerank_v3 / f"reranker-{consumer}-cpu-smoke.json").write_bytes(canonical({
                "schema_version": "fathomdb.cuda-reranker-cpu-smoke/v1", "consumer": consumer,
                "requested_policy": "auto", "effective_device": "cpu", "reason": "no_visible_cuda_device",
                "network": "none", "source_imported": False, "rerank_performed": True,
                "reranker_cache_manifest_sha256": reranker_digest, "reranker_cache_read_only": True,
                "reranker_device_environment": "unset",
            }))
            stdout = rerank_v3 / f"forced-reranker-{consumer}-stdout.txt"
            stderr = rerank_v3 / f"forced-reranker-{consumer}-stderr.txt"
            message = "cuda:0 requested for reranking but unavailable: NoVisibleCudaDevice"
            argv = (["/opt/python/cp311-cp311/bin/python", "/fathomdb-harness/forced-reranker-python.py"] if consumer == "python" else ["node", "/fathomdb-harness/forced-reranker-napi.mjs"])
            stdout.write_bytes(canonical({
                "schema_version": "fathomdb.cuda-forced-reranker-capture/v1", "consumer": consumer,
                "argv": argv, "requested_policy": "cuda:0", "status": "cuda_unavailable",
                "effective_device": None, "reason": "no_visible_cuda_device",
                "error": {"type": "RerankerDevicePolicyError", "kind": "no_visible_cuda_device", "ordinal": 0, "message": message},
            }))
            stderr.write_text(f"{message}\n", encoding="utf-8")
            (rerank_v3 / f"forced-reranker-{consumer}.json").write_bytes(canonical({
                "schema_version": "fathomdb.cuda-forced-reranker-failure/v1", "consumer": consumer,
                "requested_policy": "cuda:0", "cuda_compiled": True, "visible_devices": [],
                "status": "cuda_unavailable", "effective_device": None, "reason": "no_visible_cuda_device",
                "provenance": "installed_candidate", "command": f"installed_{consumer}_engine_open_without_default_embedder", "exit_code": 1,
                "stdout_filename": stdout.name, "stdout_sha256": hashlib.sha256(stdout.read_bytes()).hexdigest(),
                "stderr_filename": stderr.name, "stderr_sha256": hashlib.sha256(stderr.read_bytes()).hexdigest(),
            }))
        for name in ("forced-reranker-python.py", "forced-reranker-napi.mjs"):
            (rerank_v3 / name).write_bytes((REPO / "scripts/release" / name).read_bytes())
        witness_path = rerank_v3 / "cuda-preflight-witness.json"
        witness = json.loads(witness_path.read_bytes())
        witness["evidence_sha256"] = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in rerank_v3.iterdir() if path.name != witness_path.name}
        witness_path.write_bytes(canonical(witness))
        result = run(rerank_v3)
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

            mutated_record, mutated_stdout, mutated_stderr = generated
            rewrite_forced_message(
                regenerated_incompatible,
                mutated_record.name,
                mutated_stdout.name,
                mutated_stderr.name,
                "fabricated incompatible failure",
                reseal_witness=False,
            )
            result = run(
                valid,
                CANDIDATE,
                "--fixture-forced-record",
                str(mutated_record),
                str(mutated_stdout),
                str(mutated_stderr),
            )
            assert result.returncode != 0, f"accepted resealed arbitrary incompatible {consumer} message"

    print("CUDA preflight witness v2 behavioral tests passed")


if __name__ == "__main__":
    main()
