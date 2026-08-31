"""Zero-spend REASON-01 held-out cohort and environment preflight."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import re
import sys
from contextlib import contextmanager
from pathlib import Path
from typing import Mapping, Sequence

from experiments.fathomdb_test_setup import prepare_test_database


class PreflightError(RuntimeError):
    """A held-out selection, evidence, or environment refusal."""


def select_heldout_cases(
    rows: Sequence[Mapping[str, object]],
    prior_case_ids: Sequence[str],
    *,
    expected_count: int,
) -> tuple[Mapping[str, object], ...]:
    """Select all untouched eligible multi-session cases in source order."""
    if not isinstance(expected_count, int) or isinstance(expected_count, bool) or expected_count <= 0:
        raise PreflightError("expected_count must be a positive integer")
    if len(set(prior_case_ids)) != len(prior_case_ids):
        raise PreflightError("prior case IDs contain duplicates")
    prior = set(prior_case_ids)
    eligible: list[Mapping[str, object]] = []
    eligible_ids: set[str] = set()
    for row in rows:
        if row.get("question_type") != "multi-session":
            continue
        question_id = row.get("question_id")
        gold = row.get("answer_session_ids")
        if not isinstance(question_id, str) or not question_id:
            raise PreflightError("eligible row has an invalid question ID")
        if question_id in eligible_ids:
            raise PreflightError(f"eligible question ID is duplicated: {question_id}")
        eligible_ids.add(question_id)
        if (
            not isinstance(gold, list)
            or not gold
            or any(not isinstance(value, str) or not value for value in gold)
        ):
            raise PreflightError(f"eligible row lacks complete gold sessions: {question_id}")
        if question_id not in prior:
            eligible.append(row)
    missing_prior = prior - eligible_ids
    if missing_prior:
        raise PreflightError("prior case IDs are not eligible multi-session rows")
    if len(eligible) != expected_count:
        raise PreflightError(
            f"held-out cohort count drifted: got {len(eligible)}, expected {expected_count}"
        )
    selected_ids = {str(row["question_id"]) for row in eligible}
    if selected_ids & prior:
        raise PreflightError("held-out cohort overlaps prior cases")
    return tuple(eligible)


_DIGEST = re.compile(r"[0-9a-f]{64}")


def _digest(value: str, label: str) -> str:
    if not isinstance(value, str) or _DIGEST.fullmatch(value) is None:
        raise PreflightError(f"{label} must be a lowercase sha256")
    return value


def build_content_free_manifest(
    cases: Sequence[Mapping[str, object]],
    *,
    source_sha256: str,
    exclusions_sha256: str,
    selection_code_sha256: str,
    config_sha256: str,
) -> Mapping[str, object]:
    """Build a content-free manifest for the frozen held-out cohort."""
    question_ids: list[str] = []
    for case in cases:
        value = case.get("question_id")
        if not isinstance(value, str) or not value:
            raise PreflightError("held-out case has an invalid question ID")
        question_ids.append(value)
    if len(set(question_ids)) != len(question_ids):
        raise PreflightError("held-out question IDs contain duplicates")
    encoded_ids = "\n".join(question_ids).encode()
    return {
        "schema_version": "reason01.heldout-manifest.v1",
        "question_count": len(question_ids),
        "question_ids": question_ids,
        "question_ids_sha256": hashlib.sha256(encoded_ids).hexdigest(),
        "source_sha256": _digest(source_sha256, "source_sha256"),
        "exclusions_sha256": _digest(exclusions_sha256, "exclusions_sha256"),
        "selection_code_sha256": _digest(selection_code_sha256, "selection_code_sha256"),
        "config_sha256": _digest(config_sha256, "config_sha256"),
    }


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _load_list(path: Path, label: str) -> list[Mapping[str, object]]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PreflightError(f"{label} is unavailable: {path}") from exc
    if not isinstance(value, list) or any(not isinstance(row, Mapping) for row in value):
        raise PreflightError(f"{label} must be a JSON array of objects")
    return value


def _prior_ids(path: Path) -> list[str]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PreflightError(f"prior receipt is unavailable: {path}") from exc
    if not isinstance(document, Mapping) or set(document) != {"schema_version", "rows"}:
        raise PreflightError("prior receipt schema drifted")
    rows = document["rows"]
    if not isinstance(rows, list) or len(rows) != 24:
        raise PreflightError("prior receipt must contain exactly 24 rows")
    ids: list[str] = []
    for row in rows:
        if not isinstance(row, Mapping) or not isinstance(row.get("question_id"), str):
            raise PreflightError("prior receipt contains an invalid question ID")
        ids.append(str(row["question_id"]))
    if len(set(ids)) != 24:
        raise PreflightError("prior receipt question IDs are not unique")
    return ids


def write_frozen_manifest(
    *,
    source_path: Path,
    prior_receipt_path: Path,
    registry_path: Path,
    output_path: Path,
    expected_count: int = 109,
) -> Mapping[str, object]:
    """Freeze all untouched cases and atomically write their content-free manifest."""
    rows = _load_list(source_path, "LongMemEval source")
    cases = select_heldout_cases(
        rows, _prior_ids(prior_receipt_path), expected_count=expected_count
    )
    manifest = build_content_free_manifest(
        cases,
        source_sha256=_sha256(source_path),
        exclusions_sha256=_sha256(prior_receipt_path),
        selection_code_sha256=_sha256(Path(__file__)),
        config_sha256=_sha256(registry_path),
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    temporary = output_path.with_suffix(output_path.suffix + ".tmp")
    temporary.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.chmod(0o600)
    os.replace(temporary, output_path)
    return manifest


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


def _case_rows(case: Mapping[str, object]) -> list[dict[str, str]]:
    question_id = case.get("question_id")
    session_ids = case.get("haystack_session_ids")
    sessions = case.get("haystack_sessions")
    if (
        not isinstance(question_id, str)
        or not isinstance(session_ids, list)
        or not isinstance(sessions, list)
        or len(session_ids) != len(sessions)
    ):
        raise PreflightError("held-out case haystack is invalid")
    rows: list[dict[str, str]] = []
    for session_id, messages in zip(session_ids, sessions, strict=True):
        if not isinstance(session_id, str) or not isinstance(messages, list) or not messages:
            raise PreflightError(f"held-out session is invalid: {question_id}")
        for message_index, message in enumerate(messages):
            if not isinstance(message, Mapping):
                raise PreflightError(f"held-out message is invalid: {question_id}")
            role, content = message.get("role"), message.get("content")
            if not isinstance(role, str) or not role or not isinstance(content, str):
                raise PreflightError(f"held-out message is invalid: {question_id}")
            if not content:
                continue
            rows.append(
                {
                    "kind": "longmemeval_message_chunk",
                    "body": f"{role}: {content}",
                    "source_id": f"longmemeval-{question_id}-{session_id}",
                    "logical_id": f"longmemeval-{question_id}-{session_id}-{message_index}",
                }
            )
    if not rows:
        raise PreflightError(f"held-out case contains no messages: {question_id}")
    return rows


def prepare_heldout_databases(
    cases: Sequence[Mapping[str, object]],
    *,
    artifact_root: Path,
    fathomdb_bin: Path,
    runtime: Mapping[str, object],
) -> Mapping[str, object]:
    """Create and attest one fresh CUDA FathomDB database per held-out case."""
    if runtime.get("embed_device") != "cuda:0" or runtime.get("rerank_device") != "cuda:0":
        raise PreflightError("runtime does not attest both CUDA devices")
    Engine = getattr(importlib.import_module("fathomdb"), "Engine")
    artifact_root.mkdir(parents=True, exist_ok=False)
    databases: list[Mapping[str, object]] = []
    for index, case in enumerate(cases):
        question_id = str(case["question_id"])
        print(
            f"reason01-preflight: database={index + 1}/{len(cases)}",
            file=sys.stderr,
            flush=True,
        )
        prepared = prepare_test_database(
            artifact_root / "databases",
            test_id=f"heldout-{index:03d}",
            embed_device="cuda:0",
            rerank_device="cuda:0",
            embedder="default",
            warm_cache=True,
            check_reranker=True,
            fathomdb_bin=str(fathomdb_bin),
        )
        with _devices():
            engine = Engine.open(str(prepared.database_path), use_default_embedder=True)
            try:
                rows = _case_rows(case)
                engine.write(rows)
                engine.drain(timeout_s=300)
            finally:
                engine.close()
        databases.append(
            {
                "question_id_sha256": hashlib.sha256(
                    f"reason01:question:{question_id}".encode()
                ).hexdigest(),
                "row_count": len(rows),
                "database_path": str(prepared.database_path),
                "config_sha256": _sha256(prepared.config_path),
                "doctor_sha256": _sha256(prepared.doctor_path),
            }
        )
    receipt = {
        "schema_version": "reason01.environment-preflight.v1",
        "database_count": len(databases),
        "runtime": dict(runtime),
        "databases": databases,
    }
    receipt_path = artifact_root / "environment-preflight.v1.json"
    receipt_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    receipt_path.chmod(0o600)
    return receipt


def main(argv: Sequence[str] | None = None) -> int:
    """Freeze the zero-spend REASON-01 held-out input manifest."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--prior-receipt", type=Path, required=True)
    parser.add_argument("--registry", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected-count", type=int, default=109)
    parser.add_argument("--prepare-databases", action="store_true")
    parser.add_argument("--artifact-root", type=Path)
    parser.add_argument("--fathomdb-bin", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    manifest = write_frozen_manifest(
        source_path=args.source,
        prior_receipt_path=args.prior_receipt,
        registry_path=args.registry,
        output_path=args.output,
        expected_count=args.expected_count,
    )
    output: dict[str, object] = {"manifest": manifest}
    if args.prepare_databases:
        if args.artifact_root is None or args.fathomdb_bin is None:
            raise PreflightError("database preparation requires artifact root and FathomDB CLI")
        from experiments.reason_01_equivalence import runtime_attestation
        from experiments.reason_01_profile import load_registry

        rows = _load_list(args.source, "LongMemEval source")
        cases = select_heldout_cases(
            rows, _prior_ids(args.prior_receipt), expected_count=args.expected_count
        )
        runtime = runtime_attestation(
            registry=load_registry(args.registry),
            fathomdb_bin=args.fathomdb_bin,
            repo_root=args.repo_root,
        )
        output["environment"] = prepare_heldout_databases(
            cases,
            artifact_root=args.artifact_root,
            fathomdb_bin=args.fathomdb_bin,
            runtime=runtime,
        )
    print(json.dumps(output, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
