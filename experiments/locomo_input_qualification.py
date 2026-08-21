"""Content-free factual preflight for the released LOCOMO/PARENT executor.

This module never invokes FathomDB, an adapter, a model, or a device.  It reads
authorized external input bytes only to validate their hash-pinned structure and
emits a hash-bound report plus derivative manifests outside the repository.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Mapping

from experiments import locomo_provenance, trace_projection
from eval.locomo_loader import corpus_hash, load_locomo


_REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
_REPORT_SCHEMA = "locomo-input-qualification-report.v1"
_TRACE_NAME = "trace-projection.v1.json"
_RELATION_NAME = "locomo-parent-relation-proof.v2.json"
_REPORT_NAME = "locomo-input-qualification-report.v1.json"


class QualificationError(ValueError):
    """Raised for an unsafe qualification request rather than a factual blocker."""


def _reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    document: dict[str, object] = {}
    for key, value in pairs:
        if key in document:
            raise QualificationError("JSON document contains duplicate keys")
        document[key] = value
    return document


def _load_json(path: Path) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_reject_duplicate_keys)
    except (OSError, json.JSONDecodeError) as exc:
        raise QualificationError("required JSON document is unavailable or invalid") from exc


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _normalized_corpus_sha256(path: Path) -> str:
    """Return the frozen LOCOMO document-corpus identity for raw external bytes."""
    try:
        documents, _ = load_locomo(path)
    except (OSError, ValueError, TypeError, AttributeError) as exc:
        raise QualificationError("corpus cannot be normalized") from exc
    return corpus_hash(documents)


def _canonical_sha256(value: Mapping[str, object]) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    ).hexdigest()


def report_sha256(report: Mapping[str, object]) -> str:
    """Return the report self-hash while excluding only its declared hash field."""
    return _canonical_sha256({key: value for key, value in report.items() if key != "report_sha256"})


def _external_input(path: Path, label: str) -> Path | None:
    resolved = path.resolve()
    if resolved.is_relative_to(_REPOSITORY_ROOT):
        raise QualificationError(f"{label} must remain outside the repository")
    return resolved if resolved.is_file() else None


def _artifact_root(path: Path) -> Path:
    resolved = path.resolve()
    if resolved.is_relative_to(_REPOSITORY_ROOT) or "experiments/runs" in resolved.as_posix():
        raise QualificationError("artifact root must remain outside the repository and historical outputs")
    if resolved.exists() and not resolved.is_dir():
        raise QualificationError("artifact root must be a directory")
    if resolved.exists() and any(resolved.iterdir()):
        raise QualificationError("artifact root must be empty for a new qualification")
    return resolved


def _expected_inputs(phase_b: object) -> dict[str, dict[str, object]]:
    if not isinstance(phase_b, Mapping) or not isinstance(phase_b.get("external_inputs"), Mapping):
        raise QualificationError("Phase-B configuration lacks external input pins")
    inputs = phase_b["external_inputs"]
    assert isinstance(inputs, Mapping)
    expected: dict[str, dict[str, object]] = {}
    for key in ("corpus", "turn_provenance", "session_provenance", "dry_run_subset"):
        item = inputs.get(key)
        if not isinstance(item, Mapping) or not isinstance(item.get("sha256"), str):
            raise QualificationError(f"Phase-B configuration lacks {key} SHA-256 pin")
        expected[key] = dict(item)
    return expected


def _locomo_matrix_eligibility(matrix: object) -> dict[str, object]:
    if not isinstance(matrix, Mapping) or not isinstance(matrix.get("corpora"), list):
        raise QualificationError("CORPUS-01 matrix is unavailable or invalid")
    matches = [item for item in matrix["corpora"] if isinstance(item, Mapping) and item.get("corpus_id") == "locomo"]
    if len(matches) != 1:
        raise QualificationError("CORPUS-01 matrix must contain exactly one LOCOMO row")
    row = matches[0]
    fields = ("license", "payload_rule", "supported_categories", "supported_claims", "unsupported_claims")
    if any(field not in row for field in fields):
        raise QualificationError("CORPUS-01 LOCOMO row lacks claim eligibility")
    return {
        "corpus_id": "locomo",
        "license": row["license"],
        "payload_rule": row["payload_rule"],
        "supported_categories": row["supported_categories"],
        "supported_claims": row["supported_claims"],
        "unsupported_claims": row["unsupported_claims"],
    }


def _question_ids(corpus: object) -> set[str]:
    """Return the frozen Phase-B evidence-backed LOCOMO question population."""
    if not isinstance(corpus, list):
        raise QualificationError("corpus is not a LOCOMO list")
    result: set[str] = set()
    for conversation_index, item in enumerate(corpus):
        if not isinstance(item, Mapping):
            raise QualificationError("corpus question group is invalid")
        questions = item.get("qa", item.get("questions"))
        if not isinstance(questions, list):
            raise QualificationError("corpus question group lacks qa")
        for question_index, question in enumerate(questions):
            if not isinstance(question, Mapping) or not isinstance(question.get("question"), str) or not question["question"]:
                raise QualificationError("corpus contains an invalid question")
            if locomo_provenance.phase_b_question_eligible(question):
                result.add(f"locomo-{conversation_index}-q-{question_index}")
    return result


def _fixed_subset(value: object, *, expected_count: int, question_ids: set[str]) -> tuple[bool, str | None]:
    if not isinstance(value, Mapping) or set(value) != {"schema_version", "question_ids"}:
        return False, "dry_run_subset_schema_invalid"
    ids = value["question_ids"]
    if value["schema_version"] != "locomo-fixed-subset.v1" or not isinstance(ids, list):
        return False, "dry_run_subset_schema_invalid"
    if len(ids) != expected_count or len(set(ids)) != expected_count or not all(isinstance(item, str) for item in ids):
        return False, "dry_run_subset_id_count_invalid"
    if not set(ids).issubset(question_ids):
        return False, "dry_run_subset_ids_not_in_corpus"
    return True, None


def _validated_manifest_rows(value: object) -> list[Mapping[str, object]]:
    try:
        locomo_provenance.ProvenanceMap.from_document(value)
    except ValueError as exc:
        raise QualificationError("canonical provenance manifest is invalid") from exc
    assert isinstance(value, Mapping) and isinstance(value["entries"], list)
    return value["entries"]


def _trace_document(session_rows: list[Mapping[str, object]]) -> dict[str, object]:
    session_by_key: dict[tuple[str, str], Mapping[str, object]] = {}
    sources: list[trace_projection.Source] = []
    projections: list[trace_projection.Projection] = []
    for raw in session_rows:
        assert isinstance(raw, Mapping)
        key = (str(raw["conversation_id"]), str(raw["session_id"]))
        if key in session_by_key:
            raise QualificationError("session provenance has ambiguous conversation/session membership")
        session_by_key[key] = raw
        fingerprint = str(raw["fingerprint"])
        source_id = f"locomo-session-source-{fingerprint}"
        sources.append(trace_projection.Source(source_id, fingerprint))
        projections.append(trace_projection.Projection(f"locomo-session-text-{fingerprint}", source_id, fingerprint, "text"))
    trace = trace_projection.build_trace_projection(sources, projections, {"warnings": []})
    trace_projection._validate_sidecar(trace)
    return trace


def _relation_document(
    turn_rows: list[Mapping[str, object]], session_rows: list[Mapping[str, object]]
) -> dict[str, object]:
    session_by_key: dict[tuple[str, str], Mapping[str, object]] = {}
    for raw in session_rows:
        key = (str(raw["conversation_id"]), str(raw["session_id"]))
        if key in session_by_key:
            raise QualificationError("session provenance has ambiguous conversation/session membership")
        session_by_key[key] = raw
    relation_entries: list[dict[str, object]] = []
    seen_children: set[str] = set()
    for raw in turn_rows:
        assert isinstance(raw, Mapping)
        turn_ids = raw["turn_ids"]
        if not isinstance(turn_ids, list) or len(turn_ids) != 1:
            raise QualificationError("turn provenance must contain one turn per entry")
        child_id = locomo_provenance.canonical_turn_id(
            raw["conversation_id"], raw["session_id"], turn_ids[0]
        )
        if child_id in seen_children:
            raise QualificationError("turn provenance has an ambiguous child identifier")
        seen_children.add(child_id)
        key = (str(raw["conversation_id"]), str(raw["session_id"]))
        session = session_by_key.get(key)
        if session is None:
            raise QualificationError("turn provenance lacks an enclosing canonical session")
        members = session["turn_ids"]
        if not isinstance(members, list) or str(turn_ids[0]) not in members:
            raise QualificationError("turn provenance child is absent from its canonical session")
        session_fingerprint = str(session["fingerprint"])
        source_id = f"locomo-session-source-{session_fingerprint}"
        relation_entries.append(
            {
                "child_id": child_id,
                "parent_session_id": locomo_provenance.canonical_session_id(
                    session["conversation_id"], session["session_id"]
                ),
                "ordinal": members.index(str(turn_ids[0])),
                "trace_source_id": source_id,
                "turn_provenance_fingerprint": raw["fingerprint"],
                "session_provenance_fingerprint": session_fingerprint,
                "session_members": [
                    {
                        "id": locomo_provenance.canonical_turn_id(
                            session["conversation_id"], session["session_id"], member
                        ),
                        "ordinal": ordinal,
                        "trace_source_id": source_id,
                    }
                    for ordinal, member in enumerate(members)
                ],
            }
        )
    relation = {
        "schema_version": "locomo-parent-relation-proof.v2",
        "turn_provenance_sha256": "",
        "session_provenance_sha256": "",
        "entries": sorted(relation_entries, key=lambda row: str(row["child_id"])),
    }
    return relation


def _write_json(path: Path, value: Mapping[str, object]) -> None:
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n", encoding="utf-8")


def _external_json_or_blocker(path: Path, *, blocker: str, blockers: list[str]) -> object | None:
    """Read one data-plane JSON document or retain its fixed blocker code."""
    try:
        return _load_json(path)
    except QualificationError:
        blockers.append(blocker)
        return None


def _derivation_blocker(error: QualificationError) -> str:
    """Map a safe structural failure to one durable, payload-free blocker code."""
    codes = {
        "turn provenance has an ambiguous child identifier": "parent_relation_proof_ambiguous_child_identifier",
        "turn provenance must contain one turn per entry": "parent_relation_proof_turn_shape_invalid",
        "turn provenance lacks an enclosing canonical session": "parent_relation_proof_missing_enclosing_session",
        "turn provenance child is absent from its canonical session": "parent_relation_proof_child_not_in_session",
        "session provenance has ambiguous conversation/session membership": "parent_relation_proof_ambiguous_session_membership",
        "canonical provenance manifest is invalid": "canonical_provenance_manifest_invalid",
    }
    return codes.get(str(error), "parent_relation_proof_derivation_invalid")


def qualify(
    *,
    phase_b_path: Path,
    corpus_matrix_path: Path,
    corpus_path: Path,
    turn_provenance_path: Path,
    session_provenance_path: Path,
    dry_run_subset_path: Path,
    artifact_root: Path,
) -> Path:
    """Create a content-free, self-hashed factual preflight report.

    Pin mismatches are factual blockers in the report; they never make a
    release eligible. Missing or malformed control documents are also recorded
    as blockers, without inventing replacement input material.
    """
    root = _artifact_root(artifact_root)
    expected = _expected_inputs(_load_json(phase_b_path))
    eligibility = _locomo_matrix_eligibility(_load_json(corpus_matrix_path))
    files = {
        "corpus": _external_input(corpus_path, "corpus"),
        "turn_provenance": _external_input(turn_provenance_path, "turn provenance"),
        "session_provenance": _external_input(session_provenance_path, "session provenance"),
        "dry_run_subset": _external_input(dry_run_subset_path, "dry-run subset"),
    }
    blockers = [f"{name}_input_unavailable" for name, path in files.items() if path is None]
    input_status: dict[str, dict[str, object]] = {}
    for name, path in files.items():
        if path is None:
            input_status[name] = {"present": False, "expected_sha256": expected[name]["sha256"]}
            continue
        actual = _sha256(path)
        if name == "corpus":
            input_status[name] = {
                "present": True,
                "expected_sha256": expected[name]["sha256"],
                "raw_sha256": actual,
            }
            continue
        input_status[name] = {
            "present": True,
            "expected_sha256": expected[name]["sha256"],
            "actual_sha256": actual,
            "sha256_matches": actual == expected[name]["sha256"],
        }
        if actual != expected[name]["sha256"]:
            blockers.append(f"{name}_sha256_mismatch")
    artifacts: dict[str, dict[str, object]] = {}
    all_inputs_present = all(path is not None for path in files.values())
    if all_inputs_present:
        assert all(path is not None for path in files.values())
        corpus = _external_json_or_blocker(files["corpus"], blocker="corpus_json_invalid", blockers=blockers)
        subset = _external_json_or_blocker(
            files["dry_run_subset"], blocker="dry_run_subset_json_invalid", blockers=blockers
        )
        question_ids: set[str] | None = None
        if corpus is not None:
            try:
                question_ids = _question_ids(corpus)
            except QualificationError:
                blockers.append("corpus_shape_invalid")
            else:
                try:
                    normalized_sha256 = _normalized_corpus_sha256(files["corpus"])
                except QualificationError:
                    blockers.append("corpus_shape_invalid")
                else:
                    input_status["corpus"].update(
                        {
                            "actual_sha256": normalized_sha256,
                            "sha256_matches": normalized_sha256 == expected["corpus"]["sha256"],
                        }
                    )
                    if normalized_sha256 != expected["corpus"]["sha256"]:
                        blockers.append("corpus_sha256_mismatch")
                declared_full_count = expected["corpus"].get("question_count")
                if len(question_ids) != declared_full_count:
                    blockers.append("corpus_question_count_mismatch")
        if subset is not None and question_ids is not None:
            expected_subset_count = expected["dry_run_subset"].get("question_count")
            if not isinstance(expected_subset_count, int):
                raise QualificationError("Phase-B configuration lacks dry-run question count")
            valid_subset, subset_blocker = _fixed_subset(
                subset, expected_count=expected_subset_count, question_ids=question_ids
            )
            if not valid_subset:
                assert subset_blocker is not None
                blockers.append(subset_blocker)
    turn_and_session_pinned = all(
        files[name] is not None and input_status[name].get("sha256_matches") is True
        for name in ("turn_provenance", "session_provenance")
    )
    root.mkdir(parents=True, exist_ok=True)
    if turn_and_session_pinned:
        assert files["turn_provenance"] is not None and files["session_provenance"] is not None
        turns = _load_json(files["turn_provenance"])
        sessions = _load_json(files["session_provenance"])
        try:
            turn_rows = _validated_manifest_rows(turns)
            session_rows = _validated_manifest_rows(sessions)
        except QualificationError as exc:
            blockers.append(_derivation_blocker(exc))
        else:
            try:
                trace = _trace_document(session_rows)
            except QualificationError as exc:
                blockers.append(_derivation_blocker(exc))
            else:
                trace_path = root / _TRACE_NAME
                _write_json(trace_path, trace)
                artifacts["trace_projection"] = {"sha256": _sha256(trace_path), "source_count": len(session_rows)}
            try:
                relation = _relation_document(turn_rows, session_rows)
            except QualificationError as exc:
                blockers.append(_derivation_blocker(exc))
            else:
                relation["turn_provenance_sha256"] = str(expected["turn_provenance"]["sha256"])
                relation["session_provenance_sha256"] = str(expected["session_provenance"]["sha256"])
                relation_path = root / _RELATION_NAME
                _write_json(relation_path, relation)
                artifacts["parent_relation_proof"] = {
                    "sha256": _sha256(relation_path), "entry_count": len(relation["entries"])
                }
    report: dict[str, object] = {
        "schema_version": _REPORT_SCHEMA,
        "program_tracks": ["LOCOMO-01", "PARENT-01"],
        "qualification_status": "qualified" if not blockers else "blocked",
        "blockers": sorted(blockers),
        "input_status": input_status,
        "corpus_claim_eligibility": eligibility,
        "artifacts": artifacts,
        "no_live_actions": ["adapter_not_invoked", "fathomdb_not_loaded", "model_not_loaded", "device_not_selected", "measurement_not_run"],
        "report_sha256": "",
    }
    report["report_sha256"] = report_sha256(report)
    report_path = root / _REPORT_NAME
    _write_json(report_path, report)
    return report_path


def main(argv: list[str] | None = None) -> int:
    """Run a content-free preflight from explicitly declared external paths."""
    parser = argparse.ArgumentParser(description="Qualify LOCOMO/PARENT external inputs without execution")
    parser.add_argument("--phase-b", required=True, type=Path)
    parser.add_argument("--corpus-matrix", required=True, type=Path)
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--turn-provenance", required=True, type=Path)
    parser.add_argument("--session-provenance", required=True, type=Path)
    parser.add_argument("--dry-run-subset", required=True, type=Path)
    parser.add_argument("--artifact-root", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        report = qualify(
            phase_b_path=args.phase_b,
            corpus_matrix_path=args.corpus_matrix,
            corpus_path=args.corpus,
            turn_provenance_path=args.turn_provenance,
            session_provenance_path=args.session_provenance,
            dry_run_subset_path=args.dry_run_subset,
            artifact_root=args.artifact_root,
        )
    except QualificationError as exc:
        parser.error(str(exc))
    print(report)
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
