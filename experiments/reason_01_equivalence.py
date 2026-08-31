"""Development-only REASON-01 selection-equivalence checks."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import re
import socket
import subprocess
from contextlib import contextmanager
from pathlib import Path
from typing import Mapping, Sequence

from experiments import locomo_external_adapter, reason_01_profile
from experiments.fathomdb_test_setup import prepare_test_database


class EquivalenceError(RuntimeError):
    """A selected-ID or aggregate development-evidence drift."""


def verify_equivalence(
    actual: Sequence[Mapping[str, object]],
    expected: Sequence[Mapping[str, object]],
) -> Mapping[str, object]:
    """Verify exact question and selected-ID equivalence."""
    if len(actual) != len(expected):
        raise EquivalenceError(
            f"question count drifted: actual={len(actual)} expected={len(expected)}"
        )
    for index, (actual_row, expected_row) in enumerate(zip(actual, expected, strict=True)):
        actual_id = actual_row.get("question_id")
        expected_id = expected_row.get("question_id")
        if actual_id != expected_id:
            raise EquivalenceError(
                f"question order drifted at row {index}: actual={actual_id} expected={expected_id}"
            )
        actual_ids = actual_row.get("arm_turn_ids")
        expected_ids = expected_row.get("arm_turn_ids")
        if actual_ids != expected_ids:
            raise EquivalenceError(f"selected IDs drifted for question {expected_id}")
    return {"question_count": len(actual), "selected_id_rows_equal": True}


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _load_object(path: Path, label: str) -> Mapping[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise EquivalenceError(f"{label} is unavailable: {path}") from exc
    if not isinstance(value, Mapping):
        raise EquivalenceError(f"{label} must be a JSON object")
    return value


def _load_list(path: Path, label: str) -> list[object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise EquivalenceError(f"{label} is unavailable: {path}") from exc
    if not isinstance(value, list):
        raise EquivalenceError(f"{label} must be a JSON array")
    return value


def _accepted_rows(document: Mapping[str, object]) -> list[Mapping[str, object]]:
    if set(document) != {"schema_version", "observations", "traces"}:
        raise EquivalenceError("accepted equivalence receipt schema drifted")
    traces = document["traces"]
    observations = document["observations"]
    if not isinstance(traces, list) or not isinstance(observations, list) or len(traces) != 282:
        raise EquivalenceError("accepted equivalence receipt must contain 282 rows")
    observation_ids = [row.get("question_id") for row in observations if isinstance(row, Mapping)]
    rows: list[Mapping[str, object]] = []
    for item in traces:
        if not isinstance(item, Mapping) or set(item) != {"question_id", "trace"}:
            raise EquivalenceError("accepted equivalence trace is invalid")
        trace = item["trace"]
        if not isinstance(trace, Mapping) or not isinstance(trace.get("selected_ids"), list):
            raise EquivalenceError("accepted selected IDs are invalid")
        rows.append(
            {
                "question_id": item["question_id"],
                "arm_turn_ids": trace["selected_ids"],
            }
        )
    if [row["question_id"] for row in rows] != observation_ids:
        raise EquivalenceError("accepted observation and trace order drifted")
    return rows


def _external_question_id(adapter_id: object) -> str:
    if not isinstance(adapter_id, str):
        raise EquivalenceError("adapter question ID is invalid")
    match = re.fullmatch(r"locomo-(\d+)-q-(\d+)", adapter_id)
    if match is None:
        raise EquivalenceError(f"adapter question ID is invalid: {adapter_id}")
    return f"conv{match.group(1)}_q{match.group(2)}"


def _locomo_turn_rows(corpus: Sequence[object]) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for conversation_index, item in enumerate(corpus):
        if not isinstance(item, Mapping) or not isinstance(item.get("conversation"), Mapping):
            raise EquivalenceError("LOCOMO conversation is invalid")
        conversation = item["conversation"]
        speaker_a = conversation.get("speaker_a")
        if not isinstance(speaker_a, str):
            raise EquivalenceError("LOCOMO speaker identity is invalid")
        session_ids = sorted(
            (key for key in conversation if re.fullmatch(r"session_\d+", str(key))),
            key=lambda value: int(str(value).split("_")[1]),
        )
        for session_id in session_ids:
            turns = conversation[session_id]
            if not isinstance(turns, list) or not turns:
                raise EquivalenceError("LOCOMO session is invalid")
            for turn in turns:
                if not isinstance(turn, Mapping):
                    raise EquivalenceError("LOCOMO turn is invalid")
                message, raw_turn_id = locomo_external_adapter._message_text(turn, speaker_a)
                rows.append(
                    {
                        "kind": "locomo_message_chunk",
                        "body": f"{message['role']}: {message['content']}",
                        "source_id": f"locomo-{conversation_index}",
                        "logical_id": locomo_external_adapter.canonical_turn_id(
                            f"locomo-{conversation_index}", str(session_id), raw_turn_id
                        ),
                    }
                )
    if not rows:
        raise EquivalenceError("LOCOMO corpus contains no turns")
    return rows


@contextmanager
def _devices():
    keys = ("FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE", "CUDA_VISIBLE_DEVICES")
    prior = {key: os.environ.get(key) for key in keys}
    os.environ.update(
        {
            "FATHOMDB_EMBED_DEVICE": "cuda:0",
            "FATHOMDB_RERANK_DEVICE": "cuda:0",
            "CUDA_VISIBLE_DEVICES": "0",
        }
    )
    try:
        yield
    finally:
        for key, value in prior.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


def _command_output(command: Sequence[str], label: str) -> str:
    completed = subprocess.run(command, text=True, capture_output=True, check=False, timeout=30)
    if completed.returncode != 0:
        raise EquivalenceError(f"{label} failed: {completed.stderr.strip()}")
    return completed.stdout.strip()


def runtime_attestation(
    *, registry: Mapping[str, object], fathomdb_bin: Path, repo_root: Path
) -> Mapping[str, object]:
    """Attest the exact importable CUDA runtime before data generation."""
    profile = registry["profiles"][0]  # type: ignore[index]
    runtime = profile["runtime"]
    source_commit = str(runtime["fathomdb_source_commit"])
    unchanged = subprocess.run(
        [
            "git",
            "diff",
            "--quiet",
            source_commit,
            "--",
            "Cargo.toml",
            "src/python",
            "src/rust",
        ],
        cwd=repo_root,
        check=False,
    )
    if unchanged.returncode != 0:
        raise EquivalenceError("FathomDB runtime source drifted from the registry pin")
    fathomdb = importlib.import_module("fathomdb")
    native = importlib.import_module("fathomdb._fathomdb")
    package_file = getattr(fathomdb, "__file__", None)
    native_file = getattr(native, "__file__", None)
    version = getattr(fathomdb, "__version__", None)
    if not isinstance(package_file, str) or not isinstance(native_file, str):
        raise EquivalenceError("FathomDB package paths are unavailable")
    native_path = Path(native_file).resolve()
    if version != runtime["fathomdb_version"]:
        raise EquivalenceError("FathomDB Python version drifted")
    cli_version = _command_output([str(fathomdb_bin), "--version"], "FathomDB CLI version")
    if cli_version != f"fathomdb {runtime['fathomdb_version']}":
        raise EquivalenceError("FathomDB CLI version drifted")
    gpu_rows = _command_output(
        [
            "nvidia-smi",
            "--query-gpu=index,uuid,name,driver_version",
            "--format=csv,noheader",
        ],
        "CUDA attestation",
    ).splitlines()
    if not gpu_rows or not gpu_rows[0].startswith("0, GPU-"):
        raise EquivalenceError("physical GPU 0 is unavailable")
    gpu_fields = [value.strip() for value in gpu_rows[0].split(",", 3)]
    if len(gpu_fields) != 4:
        raise EquivalenceError("physical GPU 0 attestation is invalid")
    cache_root = Path.home() / ".cache" / "fathomdb"
    cache_paths = {
        "embedder_config_sha256": cache_root
        / "embedders"
        / str(runtime["embedder_cache_key"])
        / "config.json",
        "embedder_tokenizer_sha256": cache_root
        / "embedders"
        / str(runtime["embedder_cache_key"])
        / "tokenizer.json",
        "embedder_model_sha256": cache_root
        / "embedders"
        / str(runtime["embedder_cache_key"])
        / "model.safetensors",
        "reranker_config_sha256": cache_root
        / "reranker"
        / str(runtime["reranker_cache_key"])
        / "config.json",
        "reranker_tokenizer_sha256": cache_root
        / "reranker"
        / str(runtime["reranker_cache_key"])
        / "tokenizer.json",
        "reranker_model_sha256": cache_root
        / "reranker"
        / str(runtime["reranker_cache_key"])
        / "model.safetensors",
    }
    try:
        cache_hashes = {key: _sha256(path) for key, path in cache_paths.items()}
    except OSError as exc:
        raise EquivalenceError("registered model cache is unavailable") from exc
    actual = {
        "fathomdb_version": version,
        "fathomdb_source_commit": source_commit,
        "native_module_sha256": _sha256(native_path),
        "cli_sha256": _sha256(fathomdb_bin),
        "embed_device": "cuda:0",
        "rerank_device": "cuda:0",
        "embedder_id": runtime["embedder_id"],
        "reranker_id": runtime["reranker_id"],
        "host": socket.gethostname(),
        "gpu_uuid": gpu_fields[1],
        "gpu_driver": gpu_fields[3],
        "embedder_cache_key": runtime["embedder_cache_key"],
        "reranker_cache_key": runtime["reranker_cache_key"],
        **cache_hashes,
    }
    if any(actual.get(key) != runtime[key] for key in actual):
        raise EquivalenceError("registered runtime identity drifted")
    return {
        **actual,
        "python_package_path": str(Path(package_file).resolve()),
        "native_module_path": str(native_path),
        "cli_path": str(fathomdb_bin.resolve()),
        "cuda_visible_devices": "0",
        "doctor_ok": True,
        "gpu": gpu_rows[0],
    }


def run_live_equivalence(
    *,
    registry_path: Path,
    corpus_path: Path,
    accepted_path: Path,
    artifact_root: Path,
    fathomdb_bin: Path,
    repo_root: Path,
) -> Mapping[str, object]:
    """Rebuild ten LOCOMO databases and verify all 282 protected selections."""
    registry = reason_01_profile.load_registry(registry_path)
    attestation = runtime_attestation(
        registry=registry, fathomdb_bin=fathomdb_bin, repo_root=repo_root
    )
    corpus = _load_list(corpus_path, "LOCOMO corpus")
    accepted = _accepted_rows(_load_object(accepted_path, "accepted equivalence receipt"))
    accepted_ids = [str(row["question_id"]) for row in accepted]

    rows = _locomo_turn_rows(corpus)
    rows_by_conversation: dict[str, list[dict[str, str]]] = {}
    for row in rows:
        rows_by_conversation.setdefault(row["source_id"], []).append(row)
    questions = {
        _external_question_id(question["id"]): question
        for question in locomo_external_adapter._questions(corpus)
    }
    if any(question_id not in questions for question_id in accepted_ids):
        raise EquivalenceError("accepted questions do not bind to the pinned corpus")

    Engine = getattr(importlib.import_module("fathomdb"), "Engine")

    actual_by_id: dict[str, Mapping[str, object]] = {}
    doctor_rows: list[Mapping[str, str]] = []
    artifact_root.mkdir(parents=True, exist_ok=False)
    for conversation_index in range(10):
        source_id = f"locomo-{conversation_index}"
        selected_ids = [value for value in accepted_ids if value.startswith(f"conv{conversation_index}_")]
        prepared = prepare_test_database(
            artifact_root / "databases",
            test_id=f"conversation-{conversation_index}",
            embed_device="cuda:0",
            rerank_device="cuda:0",
            embedder="default",
            warm_cache=True,
            check_reranker=True,
            fathomdb_bin=str(fathomdb_bin),
        )
        doctor_rows.append(
            {"path": str(prepared.doctor_path), "sha256": _sha256(prepared.doctor_path)}
        )
        with _devices():
            engine = Engine.open(str(prepared.database_path), use_default_embedder=True)
            try:
                engine.write(rows_by_conversation[source_id])
                engine.drain(timeout_s=300)
                for question_id in selected_ids:
                    question = questions[question_id]
                    execution = reason_01_profile.execute_profile(
                        engine,
                        str(question["query"]),
                        registry,
                        intent="relationship",
                        runtime_attestation=attestation,
                    )
                    actual_by_id[question_id] = {
                        "question_id": question_id,
                        "arm_turn_ids": [hit.id.value for hit in execution.hits],
                        "trace": execution.trace,
                    }
            finally:
                engine.close()
    actual = [actual_by_id[question_id] for question_id in accepted_ids]
    summary = verify_equivalence(actual, accepted)
    output = {
        "schema_version": "reason01.equivalence.v1",
        "summary": summary,
        "runtime": attestation,
        "source": {"path": str(corpus_path), "sha256": _sha256(corpus_path)},
        "accepted": {"path": str(accepted_path), "sha256": _sha256(accepted_path)},
        "registry": {"path": str(registry_path), "sha256": _sha256(registry_path)},
        "doctor": doctor_rows,
        "rows": actual,
    }
    output_path = artifact_root / "equivalence-observations.v1.json"
    output_path.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    output_path.chmod(0o600)
    return output


def main(argv: Sequence[str] | None = None) -> int:
    """Run the live development-only REASON-01 selection equivalence."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--accepted", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--fathomdb-bin", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    output = run_live_equivalence(
        registry_path=args.registry,
        corpus_path=args.corpus,
        accepted_path=args.accepted,
        artifact_root=args.artifact_root,
        fathomdb_bin=args.fathomdb_bin,
        repo_root=args.repo_root,
    )
    print(json.dumps(output["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
