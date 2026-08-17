#!/usr/bin/env python3
"""External, hash-pinnable LOCOMO/PARENT cell-adapter ABI.

This program is copied to an access-controlled external runtime before a
coordinator release names it as ``cell_adapter``.  Its stdin/stdout boundary is
strict and content-free: corpus text, questions, retrieved passages, and
predictions remain transient in that external process.  It never writes a
repository receipt or index row.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Callable, Mapping

from experiments.locomo_provenance import (
    canonical_session_id,
    canonical_turn_id,
    phase_b_question_eligible,
    payload_fingerprint,
)


REQUEST_SCHEMA = "locomo-live-executor.request.v1"
RESULT_SCHEMA = "locomo-live-executor.cell-result.v1"
METRICS_SCHEMA = "locomo-external-adapter-metrics.v1"
ADAPTER_CONFIG_SCHEMA = "locomo-external-adapter.v1"
SAFE_METRICS_REF = "locomo-external-adapter-metrics.v1"
_SAFE_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
_SHA = re.compile(r"^[0-9a-f]{64}$")
_REQUEST_KEYS = {
    "schema_version",
    "release_id",
    "action",
    "mode",
    "cell",
    "external_inputs",
    "output_root",
}
_CELL_KEYS = {
    "cell_id",
    "program_track",
    "ingest_unit",
    "treatment",
    "retrieval",
    "runtime",
    "parent_child",
}
_RUNTIME_KEYS = {"device", "cache_state"}
_INPUT_KEYS = {
    "corpus",
    "turn_provenance",
    "session_provenance",
    "dry_run_subset",
    "trace_projection",
    "parent_relation_proof",
}
PARENT_CHILD_FROZEN: dict[str, object] = {
    "version": "parent_child_turn_session_v1",
    "parent_relation": "exact_enclosing_session",
    "child_top_k": 10,
    "parent_bundle_limit": 5,
    "parent_rank": "best_child_original_rank_then_parent_session_id",
    "fusion": "none",
    "neighbors_each_side": 1,
    "max_turns_per_bundle": 3,
    "max_turns_total": 15,
    "safe_attribution": [
        "parent_session_id",
        "seed_child_id",
        "ordered_neighbor_ids",
        "trace_source_id",
    ],
}
_REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
_FROZEN_TREATMENTS = {
    "fts_only": "fts",
    "hybrid": "hybrid",
    "hybrid_ce_alpha_03_pool_10": "hybrid",
    "hybrid_ce_alpha_10_pool_10": "hybrid",
    "hybrid_ce_alpha_10_pool_20": "hybrid",
    "fts_bounded_neighbor": "fts",
}
_FROZEN_CELL_IDS: dict[str, dict[str, object]] = {}
for _ingest_unit in ("turn", "session"):
    for _treatment, _retrieval in _FROZEN_TREATMENTS.items():
        for _device in ("cpu", "gpu"):
            for _cache_state in ("cold", "steady"):
                _cell_id = f"{_ingest_unit}--{_treatment}--{_device}--{_cache_state}"
                _FROZEN_CELL_IDS[_cell_id] = {
                    "cell_id": _cell_id,
                    "program_track": "LOCOMO-01",
                    "ingest_unit": _ingest_unit,
                    "treatment": _treatment,
                    "retrieval": _retrieval,
                    "runtime": {"device": _device, "cache_state": _cache_state},
                    "parent_child": None,
                }
for _device in ("cpu", "gpu"):
    for _cache_state in ("cold", "steady"):
        _cell_id = f"turn--parent_child_turn_session_v1--{_device}--{_cache_state}"
        _FROZEN_CELL_IDS[_cell_id] = {
            "cell_id": _cell_id,
            "program_track": "PARENT-01",
            "ingest_unit": "turn",
            "treatment": "parent_child_turn_session_v1",
            "retrieval": "hybrid",
            "runtime": {"device": _device, "cache_state": _cache_state},
            "parent_child": PARENT_CHILD_FROZEN,
        }
_FROZEN_ACTION_CELLS = {
    "fixed_subset_dry_run": (
        "turn--fts_only--cpu--cold",
        "turn--hybrid--cpu--steady",
        "session--fts_only--cpu--cold",
        "session--hybrid--cpu--steady",
        "turn--parent_child_turn_session_v1--cpu--cold",
    ),
    "cpu_grid": tuple(
        cell_id
        for cell_id, cell in _FROZEN_CELL_IDS.items()
        if cell["runtime"]["device"] == "cpu"
    ),
    "gpu_ce_grid": tuple(
        cell_id
        for cell_id, cell in _FROZEN_CELL_IDS.items()
        if cell["runtime"]["device"] == "gpu"
    ),
}


class AdapterError(ValueError):
    """Raised before an unsafe request can execute or emit an unsafe result."""


def _exact(value: object, label: str, expected: set[str]) -> dict[str, object]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        raise AdapterError(
            f"{label} keys mismatch: missing={sorted(expected - actual)}, unknown={sorted(actual - expected)}"
        )
    return value


def _identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_ID.fullmatch(value) is None:
        raise AdapterError(f"{label} must be a safe identifier")
    return value


def _sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA.fullmatch(value) is None:
        raise AdapterError(f"{label} must be a lowercase sha256")
    return value


def _load_json(path: Path, label: str) -> object:
    try:
        return json.loads(
            path.read_text(encoding="utf-8"), object_pairs_hook=_no_duplicate_keys
        )
    except (OSError, json.JSONDecodeError, AdapterError) as exc:
        raise AdapterError(f"{label} is unavailable or invalid") from exc


def _no_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise AdapterError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_request_json(value: str) -> dict[str, object]:
    """Parse one stdin request with duplicate-key rejection."""
    try:
        request = json.loads(value, object_pairs_hook=_no_duplicate_keys)
    except AdapterError:
        raise
    except json.JSONDecodeError as exc:
        raise AdapterError("request is not strict JSON") from exc
    return _validate_request(request)


def _external_path(value: object, label: str, *, directory: bool = False) -> Path:
    if not isinstance(value, str) or not Path(value).is_absolute():
        raise AdapterError(f"{label} must be an absolute external path")
    path = Path(value)
    if path.is_symlink():
        raise AdapterError(f"{label} must not be a symlink")
    resolved = path.resolve()
    if directory:
        return resolved
    if not resolved.is_file() or resolved.is_symlink():
        raise AdapterError(f"{label} is unavailable")
    return resolved


def _validate_request(value: object) -> dict[str, object]:
    request = _exact(value, "request", _REQUEST_KEYS)
    if request["schema_version"] != REQUEST_SCHEMA:
        raise AdapterError("request schema mismatch")
    _identifier(request["release_id"], "release_id")
    if request["action"] not in {"fixed_subset_dry_run", "cpu_grid", "gpu_ce_grid"}:
        raise AdapterError("request action is not released")
    if request["mode"] not in {"dry_run", "full_grid"}:
        raise AdapterError("request mode is unsafe")
    if (request["action"] == "fixed_subset_dry_run") != (request["mode"] == "dry_run"):
        raise AdapterError("request action and mode drifted")
    cell = _exact(request["cell"], "cell", _CELL_KEYS)
    _identifier(cell["cell_id"], "cell_id")
    if cell["program_track"] not in {"LOCOMO-01", "PARENT-01"}:
        raise AdapterError("cell program track is unsafe")
    if cell["ingest_unit"] not in {"turn", "session"} or cell["retrieval"] not in {
        "fts",
        "hybrid",
    }:
        raise AdapterError("cell retrieval semantics are unsafe")
    if not isinstance(cell["treatment"], str) or not cell["treatment"]:
        raise AdapterError("cell treatment is unsafe")
    runtime = _exact(cell["runtime"], "cell runtime", _RUNTIME_KEYS)
    if runtime["device"] not in {"cpu", "gpu"} or runtime["cache_state"] not in {
        "cold",
        "steady",
    }:
        raise AdapterError("cell runtime is unsafe")
    expected_cell = _FROZEN_CELL_IDS.get(str(cell["cell_id"]))
    if expected_cell is None or cell != expected_cell:
        raise AdapterError("cell does not match one exact frozen Phase-B treatment")
    if str(cell["cell_id"]) not in _FROZEN_ACTION_CELLS[str(request["action"])]:
        raise AdapterError("cell is outside the released action partition")
    inputs = _exact(request["external_inputs"], "external inputs", _INPUT_KEYS)
    for key in _INPUT_KEYS:
        _external_path(inputs[key], key)
    output_root = _external_path(request["output_root"], "output root", directory=True)
    if output_root.is_relative_to(_REPOSITORY_ROOT):
        raise AdapterError(
            "output root must remain outside the repository and historical outputs"
        )
    return request


def validate_config(value: object) -> dict[str, object]:
    """Validate the checked-in, content-free adapter deployment declaration."""
    config = _exact(
        value,
        "adapter config",
        {
            "schema_version",
            "program_tracks",
            "request_schema",
            "result_schema",
            "metrics_schema",
            "repository_writes",
        },
    )
    if config != {
        "schema_version": ADAPTER_CONFIG_SCHEMA,
        "program_tracks": ["LOCOMO-01", "PARENT-01"],
        "request_schema": REQUEST_SCHEMA,
        "result_schema": RESULT_SCHEMA,
        "metrics_schema": METRICS_SCHEMA,
        "repository_writes": "forbidden",
    }:
        raise AdapterError("adapter config semantics drifted")
    return config


def _message_text(turn: Mapping[str, object], speaker_a: str) -> tuple[dict[str, str], str]:
    speaker, text, turn_id = turn.get("speaker"), turn.get("text"), turn.get("dia_id")
    if (
        not isinstance(speaker, str)
        or not isinstance(text, str)
        or not isinstance(turn_id, str)
    ):
        raise AdapterError("LOCOMO turn is incomplete")
    _identifier(turn_id, "LOCOMO turn id")
    query, caption = turn.get("query", ""), turn.get("blip_caption", "")
    if not isinstance(query, str) or not isinstance(caption, str):
        raise AdapterError("LOCOMO image annotation is unsafe")
    suffix = (
        f" [Sharing image - query: {query}. The image shows: {caption}]"
        if query and caption
        else (
            f" [Sharing image - query for: {query}]"
            if query
            else (f" [Sharing image that shows: {caption}]" if caption else "")
        )
    )
    return ({"role": "user" if speaker == speaker_a else "assistant", "content": f"{speaker}: {text}{suffix}"}, turn_id)


def _payload_fingerprint(user_id: str, messages: list[dict[str, str]]) -> str:
    """Return the canonical preflight identity for one adapter ingestion payload."""
    return payload_fingerprint({"user_id": user_id, "messages": messages})


def _sessions(corpus: object) -> list[dict[str, object]]:
    if not isinstance(corpus, list):
        raise AdapterError("LOCOMO corpus must be a list")
    output: list[dict[str, object]] = []
    for conversation_index, item in enumerate(corpus):
        if not isinstance(item, Mapping) or not isinstance(
            item.get("conversation"), Mapping
        ):
            raise AdapterError("LOCOMO corpus conversation is missing")
        conversation = item["conversation"]
        speaker_a = conversation.get("speaker_a")
        if not isinstance(speaker_a, str) or not speaker_a:
            raise AdapterError("LOCOMO speaker_a is missing")
        for session_id in sorted(
            key for key in conversation if re.fullmatch(r"session_\d+", str(key))
        ):
            turns = conversation[session_id]
            if not isinstance(turns, list) or not turns:
                raise AdapterError("LOCOMO session is empty")
            rendered = [
                _message_text(turn, speaker_a)
                for turn in turns
                if isinstance(turn, Mapping)
            ]
            if len(rendered) != len(turns):
                raise AdapterError("LOCOMO session turn is invalid")
            output.append(
                {
                    "conversation_id": f"locomo-{conversation_index}",
                    "session_id": session_id,
                    "user_id": f"locomo_{conversation_index}_adapter",
                    "messages": rendered,
                }
            )
    if not output:
        raise AdapterError("LOCOMO corpus has no sessions")
    return output


def provenance_documents_for_corpus(
    corpus: object,
) -> tuple[dict[str, object], dict[str, object]]:
    """Create synthetic-only compatible manifests; real manifests remain external."""
    turn_entries: list[dict[str, object]] = []
    session_entries: list[dict[str, object]] = []
    for session in _sessions(corpus):
        messages = session["messages"]
        assert isinstance(messages, list)
        turn_ids = [turn_id for _, turn_id in messages]
        session_entries.append(
            {
                "fingerprint": _payload_fingerprint(
                    str(session["user_id"]), [message for message, _ in messages]
                ),
                "conversation_id": session["conversation_id"],
                "session_id": session["session_id"],
                "turn_ids": turn_ids,
            }
        )
        for message, turn_id in messages:
            turn_entries.append(
                {
                    "fingerprint": _payload_fingerprint(
                        str(session["user_id"]), [message]
                    ),
                    "conversation_id": session["conversation_id"],
                    "session_id": session["session_id"],
                    "turn_ids": [turn_id],
                }
            )
    return (
        {"schema_version": "locomo-provenance.v1", "entries": turn_entries},
        {"schema_version": "locomo-provenance.v1", "entries": session_entries},
    )


def synthetic_trace_sidecar(source_id: str) -> dict[str, object]:
    """Return a minimal safe TRACE fixture for adapter tests only."""
    _identifier(source_id, "TRACE source")
    source_sha = "a" * 64
    return {
        "schema_version": "trace-projection.v1",
        "sources": [
            {"source_id": source_id, "source_sha256": source_sha, "lifecycle": "active"}
        ],
        "projections": [
            {
                "projection_id": "projection-1",
                "source_id": source_id,
                "source_sha256": source_sha,
                "kind": "text",
                "lifecycle": "active",
                "searchable": True,
            }
        ],
        "supersessions": [],
        "outcomes": {
            "source_count": 1,
            "active_source_count": 1,
            "superseded_source_count": 0,
            "erased_source_count": 0,
            "projection_count": 1,
            "searchable_projection_count": 1,
            "unattributed_projection_count": 0,
            "stale_searchable_projection_count": 0,
        },
        "diagnostics": [],
    }


def relation_proof_document(
    turns: Mapping[str, object],
    sessions: Mapping[str, object],
    *,
    conversation_id: str,
    child_id: str,
    session_id: str,
    source_id: str,
) -> dict[str, object]:
    """Create one synthetic parent proof from compatible manifest documents."""
    turn_entries = [
        entry
        for entry in turns["entries"]
        if entry["conversation_id"] == conversation_id
        and entry["session_id"] == session_id
        and child_id in entry["turn_ids"]
    ]
    session_entries = [
        entry
        for entry in sessions["entries"]
        if entry["conversation_id"] == conversation_id and entry["session_id"] == session_id
    ]
    if len(turn_entries) != 1 or len(session_entries) != 1:
        raise AdapterError("synthetic parent proof requires an exact scoped provenance entry")
    turn_entry, session_entry = turn_entries[0], session_entries[0]
    members = [
        {
            "id": canonical_turn_id(
                session_entry["conversation_id"], session_id, item
            ),
            "ordinal": index,
            "trace_source_id": source_id,
        }
        for index, item in enumerate(session_entry["turn_ids"])
    ]
    ordinal = next(index for index, item in enumerate(session_entry["turn_ids"]) if item == child_id)
    canonical_child_id = canonical_turn_id(
        turn_entry["conversation_id"], session_id, child_id
    )
    return {
        "schema_version": "locomo-parent-relation-proof.v2",
        "turn_provenance_sha256": hashlib.sha256(
            json.dumps(turns).encode()
        ).hexdigest(),
        "session_provenance_sha256": hashlib.sha256(
            json.dumps(sessions).encode()
        ).hexdigest(),
        "entries": [
            {
                "child_id": canonical_child_id,
                "parent_session_id": canonical_session_id(
                    session_entry["conversation_id"], session_id
                ),
                "ordinal": ordinal,
                "trace_source_id": source_id,
                "turn_provenance_fingerprint": turn_entry["fingerprint"],
                "session_provenance_fingerprint": session_entry["fingerprint"],
                "session_members": members,
            }
        ],
    }


def _manifest(value: object, label: str) -> dict[str, dict[str, object]]:
    document = _exact(value, label, {"schema_version", "entries"})
    if document["schema_version"] != "locomo-provenance.v1" or not isinstance(
        document["entries"], list
    ):
        raise AdapterError(f"{label} schema is unsafe")
    entries: dict[str, dict[str, object]] = {}
    for raw in document["entries"]:
        entry = _exact(
            raw,
            f"{label} entry",
            {"fingerprint", "conversation_id", "session_id", "turn_ids"},
        )
        _sha256(entry["fingerprint"], f"{label} fingerprint")
        _identifier(entry["conversation_id"], f"{label} conversation")
        _identifier(entry["session_id"], f"{label} session")
        if not isinstance(entry["turn_ids"], list) or not entry["turn_ids"]:
            raise AdapterError(f"{label} turn ids are unsafe")
        for turn_id in entry["turn_ids"]:
            _identifier(turn_id, f"{label} turn id")
        if entry["fingerprint"] in entries:
            raise AdapterError(f"{label} fingerprint is ambiguous")
        entries[entry["fingerprint"]] = entry
    return entries


def _active_trace_sources(value: object) -> set[str]:
    trace = _exact(
        value,
        "TRACE sidecar",
        {
            "schema_version",
            "sources",
            "projections",
            "supersessions",
            "outcomes",
            "diagnostics",
        },
    )
    if (
        trace["schema_version"] != "trace-projection.v1"
        or not isinstance(trace["sources"], list)
        or not isinstance(trace["outcomes"], Mapping)
    ):
        raise AdapterError("TRACE sidecar is unsafe")
    outcomes = trace["outcomes"]
    if (
        outcomes.get("unattributed_projection_count") != 0
        or outcomes.get("stale_searchable_projection_count") != 0
    ):
        raise AdapterError("TRACE sidecar lifecycle proof is incomplete")
    active: set[str] = set()
    for source in trace["sources"]:
        row = _exact(
            source, "TRACE source", {"source_id", "source_sha256", "lifecycle"}
        )
        _identifier(row["source_id"], "TRACE source id")
        _sha256(row["source_sha256"], "TRACE source sha")
        if row["lifecycle"] == "active":
            active.add(str(row["source_id"]))
    if not active:
        raise AdapterError("TRACE sidecar has no active source")
    return active


def _relations(
    value: object,
    active_sources: set[str],
    *,
    turn_manifest: Mapping[str, Mapping[str, object]],
    session_manifest: Mapping[str, Mapping[str, object]],
    turn_manifest_sha256: str,
    session_manifest_sha256: str,
) -> dict[str, dict[str, object]]:
    document = _exact(
        value,
        "parent relation proof",
        {
            "schema_version",
            "turn_provenance_sha256",
            "session_provenance_sha256",
            "entries",
        },
    )
    if document[
        "schema_version"
    ] != "locomo-parent-relation-proof.v2" or not isinstance(document["entries"], list):
        raise AdapterError("parent relation proof schema is unsafe")
    if (
        document["turn_provenance_sha256"] != turn_manifest_sha256
        or document["session_provenance_sha256"] != session_manifest_sha256
    ):
        raise AdapterError("parent relation proof provenance binding drifted")
    result: dict[str, dict[str, object]] = {}
    expected = {
        "child_id",
        "parent_session_id",
        "ordinal",
        "trace_source_id",
        "turn_provenance_fingerprint",
        "session_provenance_fingerprint",
        "session_members",
    }
    for raw in document["entries"]:
        entry = _exact(raw, "parent relation entry", expected)
        for key in ("child_id", "parent_session_id", "trace_source_id"):
            _identifier(entry[key], f"parent relation {key}")
        if (
            entry["trace_source_id"] not in active_sources
            or not isinstance(entry["ordinal"], int)
            or entry["ordinal"] < 0
        ):
            raise AdapterError("parent relation proof is not TRACE-qualified")
        _sha256(
            entry["turn_provenance_fingerprint"], "parent relation turn fingerprint"
        )
        _sha256(
            entry["session_provenance_fingerprint"],
            "parent relation session fingerprint",
        )
        if (
            not isinstance(entry["session_members"], list)
            or not entry["session_members"]
        ):
            raise AdapterError("parent relation members are unsafe")
        ordinals: set[int] = set()
        for expected_ordinal, member in enumerate(entry["session_members"]):
            row = _exact(
                member, "parent session member", {"id", "ordinal", "trace_source_id"}
            )
            _identifier(row["id"], "parent member id")
            if (
                not isinstance(row["ordinal"], int)
                or row["ordinal"] != expected_ordinal
                or row["ordinal"] in ordinals
                or row["trace_source_id"] != entry["trace_source_id"]
            ):
                raise AdapterError(
                    "parent relation proof does not bind canonical provenance manifests"
                )
            ordinals.add(row["ordinal"])
        if entry["child_id"] in result:
            raise AdapterError("parent relation child is ambiguous")
        turn_entry = turn_manifest.get(str(entry["turn_provenance_fingerprint"]))
        session_entry = session_manifest.get(
            str(entry["session_provenance_fingerprint"])
        )
        members = entry["session_members"]
        assert isinstance(members, list)
        if turn_entry is None or session_entry is None:
            raise AdapterError(
                "parent relation proof does not bind canonical provenance manifests"
            )
        raw_turn_ids = turn_entry["turn_ids"]
        raw_session_members = session_entry["turn_ids"]
        assert isinstance(raw_turn_ids, list) and isinstance(raw_session_members, list)
        if len(raw_turn_ids) != 1:
            raise AdapterError(
                "parent relation proof does not bind canonical provenance manifests"
            )
        expected_child_id = canonical_turn_id(
            turn_entry["conversation_id"], turn_entry["session_id"], raw_turn_ids[0]
        )
        expected_members = [
            canonical_turn_id(
                session_entry["conversation_id"], session_entry["session_id"], member
            )
            for member in raw_session_members
        ]
        child_ordinal = next(
            (ordinal for ordinal, member in enumerate(expected_members) if member == entry["child_id"]),
            None,
        )
        if (
            entry["child_id"] != expected_child_id
            or turn_entry["conversation_id"] != session_entry["conversation_id"]
            or canonical_session_id(turn_entry["conversation_id"], turn_entry["session_id"])
            != entry["parent_session_id"]
            or canonical_session_id(session_entry["conversation_id"], session_entry["session_id"])
            != entry["parent_session_id"]
            or expected_members != [member["id"] for member in members]
            or entry["ordinal"] != child_ordinal
        ):
            raise AdapterError(
                "parent relation proof does not bind canonical provenance manifests"
            )
        resolved = dict(entry)
        resolved["_conversation_id"] = turn_entry["conversation_id"]
        resolved["_raw_turn_id"] = raw_turn_ids[0]
        result[str(entry["child_id"])] = resolved
    return result


def _questions(corpus: object) -> list[dict[str, object]]:
    if not isinstance(corpus, list):
        raise AdapterError("LOCOMO corpus must be a list")
    result: list[dict[str, object]] = []
    for conversation_index, item in enumerate(corpus):
        if not isinstance(item, Mapping):
            raise AdapterError("LOCOMO question group is unsafe")
        qa = item.get("qa", item.get("questions"))
        if not isinstance(qa, list):
            raise AdapterError("LOCOMO qa list is missing")
        for question_index, raw in enumerate(qa):
            if (
                not isinstance(raw, Mapping)
                or not isinstance(raw.get("question"), str)
                or not raw["question"]
            ):
                raise AdapterError("LOCOMO question is unsafe")
            if not phase_b_question_eligible(dict(raw)):
                continue
            evidence, category = raw.get("evidence"), raw.get("category")
            if (
                not isinstance(evidence, list)
                or not evidence
                or not isinstance(category, int)
                or isinstance(category, bool)
            ):
                raise AdapterError("LOCOMO evidence-backed question is unsafe")
            if not all(isinstance(value, str) and value for value in evidence):
                raise AdapterError("LOCOMO evidence identifier is unsafe")
            ids = set(evidence)
            result.append(
                {
                    "id": f"locomo-{conversation_index}-q-{question_index}",
                    "conversation_id": f"locomo-{conversation_index}",
                    "user_id": f"locomo_{conversation_index}_adapter",
                    "query": raw["question"],
                    "evidence": ids,
                    "category": category,
                }
            )
    return result


def _select_questions(
    request: Mapping[str, object], corpus: object
) -> list[dict[str, object]]:
    questions = _questions(corpus)
    if request["mode"] == "dry_run":
        subset = _exact(
            _load_json(
                _external_path(
                    request["external_inputs"]["dry_run_subset"], "dry-run subset"
                ),
                "dry-run subset",
            ),
            "dry-run subset",
            {"schema_version", "question_ids"},
        )
        if subset["schema_version"] != "locomo-fixed-subset.v1" or not isinstance(
            subset["question_ids"], list
        ):
            raise AdapterError("dry-run subset schema is unsafe")
        selected_ids = subset["question_ids"]
        if (
            len(selected_ids) != 32
            or len(set(selected_ids)) != 32
            or not all(isinstance(item, str) for item in selected_ids)
        ):
            raise AdapterError(
                "dry-run subset must contain exactly 32 unique safe questions"
            )
        by_id = {str(item["id"]): item for item in questions}
        if any(item not in by_id for item in selected_ids):
            raise AdapterError("dry-run subset does not match external corpus")
        return [by_id[item] for item in selected_ids]
    if len(questions) != 1536:
        raise AdapterError("full grid requires exactly 1536 evidence-backed questions")
    return questions


def _ingest_rows(
    corpus: object, *, ingest_unit: str, manifest: Mapping[str, Mapping[str, object]]
) -> tuple[list[dict[str, str]], dict[str, tuple[str, ...]]]:
    rows: list[dict[str, str]] = []
    evidence_by_logical: dict[str, tuple[str, ...]] = {}
    for session in _sessions(corpus):
        messages = session["messages"]
        assert isinstance(messages, list)
        groups = (
            [messages] if ingest_unit == "session" else [[item] for item in messages]
        )
        for group in groups:
            messages = [item[0] for item in group]
            turn_ids = [item[1] for item in group]
            fingerprint = _payload_fingerprint(str(session["user_id"]), messages)
            entry = manifest.get(fingerprint)
            if (
                entry is None
                or entry["conversation_id"] != session["conversation_id"]
                or entry["session_id"] != session["session_id"]
                or entry["turn_ids"] != turn_ids
            ):
                raise AdapterError(
                    "external provenance manifest does not qualify corpus ingestion"
                )
            logical_id = (
                canonical_turn_id(
                    session["conversation_id"], session["session_id"], turn_ids[0]
                )
                if ingest_unit == "turn"
                else f"{session['conversation_id']}:{session['session_id']}"
            )
            _identifier(logical_id, "logical id")
            if logical_id in evidence_by_logical:
                raise AdapterError("external corpus logical identity is ambiguous")
            evidence_by_logical[logical_id] = tuple(turn_ids)
            rows.append(
                {
                    "kind": "locomo_message_chunk",
                    "body": "\n".join(message["content"] for message in messages),
                    "source_id": str(session["conversation_id"]),
                    "logical_id": logical_id,
                }
            )
    return rows, evidence_by_logical


def _p95(values: list[float]) -> float:
    if not values:
        raise AdapterError("runtime timing is empty")
    return sorted(values)[max(0, math.ceil(len(values) * 0.95) - 1)]


def _class(category: int) -> str:
    return (
        "factoid"
        if category == 1
        else ("temporal" if category == 2 else "multi_session")
    )


def _require_single_visible_cuda() -> str:
    visible = os.environ.get("CUDA_VISIBLE_DEVICES")
    if visible not in {None, "0"}:
        raise AdapterError("CUDA runtime must expose exactly selected device cuda:0")
    try:
        completed = subprocess.run(
            ["nvidia-smi", "--query-gpu=index", "--format=csv,noheader"],
            text=True,
            capture_output=True,
            check=False,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise AdapterError("CUDA device is unavailable") from exc
    if completed.returncode != 0 or completed.stdout.strip().splitlines() != ["0"]:
        raise AdapterError("CUDA device is unavailable")
    return "cuda:0"


def _default_engine_factory(path: str, dense: bool):
    from fathomdb import Engine

    return Engine.open(path, use_default_embedder=dense)


def _engine_search(engine: object, query: str, cell: Mapping[str, object]) -> list[str]:
    treatment = str(cell["treatment"])
    if cell["retrieval"] == "fts":
        result = engine.search_text_only(query, limit=10)
    else:
        kwargs: dict[str, object] = {"limit": 10}
        if treatment == "hybrid_ce_alpha_03_pool_10":
            kwargs.update(rerank_depth=10, alpha=0.3, pool_n=10)
        elif treatment == "hybrid_ce_alpha_10_pool_10":
            kwargs.update(rerank_depth=10, alpha=1.0, pool_n=10)
        elif treatment == "hybrid_ce_alpha_10_pool_20":
            kwargs.update(rerank_depth=20, alpha=1.0, pool_n=20)
        result = engine.search(query, **kwargs)
        if getattr(engine, "dense_disabled", lambda: False)():
            raise AdapterError(
                "hybrid retrieval rejected: dense runtime is unavailable"
            )
    hits = getattr(result, "results", None)
    if not isinstance(hits, list):
        raise AdapterError("FathomDB retrieval result is unsafe")
    logical_ids: list[str] = []
    for hit in hits[:10]:
        identity = getattr(hit, "id", None)
        value = getattr(identity, "value", None)
        _identifier(value, "FathomDB hit logical id")
        logical_ids.append(value)
    return logical_ids


def _parent_hits(
    logical_ids: list[str], relations: Mapping[str, Mapping[str, object]]
) -> list[dict[str, object]]:
    hits: list[dict[str, object]] = []
    seen_children: set[str] = set()
    for rank, child_id in enumerate(logical_ids[:10], start=1):
        if child_id in seen_children:
            raise AdapterError(
                "PARENT child ranking contains a duplicate logical identity"
            )
        seen_children.add(child_id)
        relation = relations.get(child_id)
        if relation is None:
            raise AdapterError("PARENT child hit lacks canonical relation proof")
        members = relation["session_members"]
        assert isinstance(members, list)
        ordinal = relation["ordinal"]
        neighbors = [
            {
                "id": row["id"],
                "parent_session_id": relation["parent_session_id"],
                "ordinal": row["ordinal"],
                "trace_source_id": relation["trace_source_id"],
            }
            for row in members
            if abs(int(row["ordinal"]) - int(ordinal)) == 1
        ]
        hits.append(
            {
                "child_id": child_id,
                "rank": rank,
                "child_provenance": {
                    "parent_session_ids": [relation["parent_session_id"]],
                    "ordinal": ordinal,
                    "trace_source_id": relation["trace_source_id"],
                },
                "neighbors": neighbors,
            }
        )
    return hits


def _parent_bundles(hits: list[dict[str, object]]) -> list[dict[str, object]]:
    """Deduplicate ranked child hits into the frozen bounded parent contexts."""
    selected: dict[str, dict[str, object]] = {}
    for hit in hits:
        provenance = hit["child_provenance"]
        assert isinstance(provenance, Mapping)
        parent_session_id = provenance["parent_session_ids"][0]
        _identifier(parent_session_id, "PARENT parent session id")
        existing = selected.get(parent_session_id)
        if existing is None or hit["rank"] < existing["rank"]:
            selected[parent_session_id] = hit
    bundles: list[dict[str, object]] = []
    for hit in sorted(
        selected.values(),
        key=lambda item: (
            item["rank"],
            item["child_provenance"]["parent_session_ids"][0],
        ),
    )[:5]:
        provenance = hit["child_provenance"]
        assert isinstance(provenance, Mapping)
        neighbors = hit["neighbors"]
        assert isinstance(neighbors, list)
        bundles.append(
            {
                "parent_session_id": provenance["parent_session_ids"][0],
                "seed_child_id": hit["child_id"],
                "ordered_neighbor_ids": [neighbor["id"] for neighbor in neighbors],
                "trace_source_id": provenance["trace_source_id"],
                "rank": hit["rank"],
                "child_hit_count": len(hits),
            }
        )
    return bundles


def _ordered_evidence(
    logical_ids: list[str], evidence_by_logical: Mapping[str, tuple[str, ...]]
) -> list[str]:
    """Expand hits in rank order while retaining the first appearance of every turn."""
    ordered: list[str] = []
    seen: set[str] = set()
    for logical_id in logical_ids:
        for evidence_id in evidence_by_logical.get(logical_id, ()):
            if evidence_id not in seen:
                seen.add(evidence_id)
                ordered.append(evidence_id)
    return ordered


def _metrics(
    questions: list[dict[str, object]],
    retrieved: list[list[str]],
    *,
    ingest_ack_ms: float,
    ready_ms: float,
    query_ms: list[float],
    parent: bool,
    relations: Mapping[str, Mapping[str, object]],
    parent_bundles: list[list[dict[str, object]]] | None,
) -> dict[str, object]:
    values: dict[str, list[float]] = {
        "r10": [],
        "r1": [],
        "mrr": [],
        "ndcg": [],
        "temporal": [],
    }
    classes: dict[str, list[float]] = {
        "factoid": [],
        "temporal": [],
        "multi_session": [],
    }
    parent_session: list[float] = []
    parent_duplicate_rates: list[float] = []
    parent_context_expansions: list[float] = []
    bundles_by_question = parent_bundles or [[] for _ in questions]
    for question, hits, bundles in zip(
        questions, retrieved, bundles_by_question, strict=True
    ):
        evidence = question["evidence"]
        assert isinstance(evidence, set)
        relevance = [hit in evidence for hit in hits[:10]]
        first = next(
            (index for index, value in enumerate(relevance, start=1) if value), None
        )
        r10, r1, mrr = (
            float(any(relevance)),
            float(bool(relevance and relevance[0])),
            (1.0 / first if first else 0.0),
        )
        ideal = sum(
            1 / math.log2(rank + 1) for rank in range(1, min(10, len(evidence)) + 1)
        )
        ndcg = (
            sum(
                1 / math.log2(rank + 1)
                for rank, value in enumerate(relevance, start=1)
                if value
            )
            / ideal
        )
        values["r10"].append(r10)
        values["r1"].append(r1)
        values["mrr"].append(mrr)
        values["ndcg"].append(ndcg)
        bucket = _class(int(question["category"]))
        classes[bucket].append(r10)
        if bucket == "temporal":
            values["temporal"].append(r10)
        if parent:
            expected = {
                str(relation["parent_session_id"])
                for relation in relations.values()
                if relation.get("_conversation_id") == question.get("conversation_id")
                and relation.get("_raw_turn_id") in evidence
            }
            observed = {str(bundle["parent_session_id"]) for bundle in bundles}
            parent_session.append(float(bool(expected & observed)) if expected else 0.0)
            child_hit_count = bundles[0]["child_hit_count"] if bundles else 0
            parent_duplicate_rates.append(
                (child_hit_count - len(bundles)) / child_hit_count
                if child_hit_count
                else 0.0
            )
            parent_context_expansions.append(
                sum(len(bundle["ordered_neighbor_ids"]) for bundle in bundles)
            )
    if any(not rows for rows in classes.values()) or not values["temporal"]:
        raise AdapterError("selected questions do not cover required LOCOMO classes")

    def mean(rows: list[float]) -> float:
        return sum(rows) / len(rows)

    summary: dict[str, object] = {
        "m1": {"r_at_10": mean(values["r10"])},
        "m2": {
            "mrr": mean(values["mrr"]),
            "r_at_1": mean(values["r1"]),
            "ndcg_at_10": mean(values["ndcg"]),
        },
        "m4_proxy": {"temporal_evidence_recall": mean(values["temporal"])},
        "m6": {"facade_query_ms": _p95(query_ms), "engine_query_ms": _p95(query_ms)},
        "m7": {"ingest_ack_ms": ingest_ack_ms, "ready_to_search_ms": ready_ms},
        "class_metrics": {
            name: {"r_at_10": mean(rows)} for name, rows in classes.items()
        },
    }
    if parent:
        summary["parent_metrics"] = {
            "child_evidence_recall": mean(values["r10"]),
            "parent_session_recall": mean(parent_session),
            "duplicate_rate": mean(parent_duplicate_rates),
            "context_expansion_count": mean(parent_context_expansions),
            "class_latency_ms": {name: _p95(query_ms) for name in classes},
        }
    return summary


def execute_request(
    value: object, *, engine_factory: Callable[[str, bool], object] | None = None
) -> dict[str, object]:
    """Run one qualified cell externally and return only the safe result ABI."""
    request = _validate_request(value)
    inputs = request["external_inputs"]
    assert isinstance(inputs, Mapping)
    corpus = _load_json(_external_path(inputs["corpus"], "corpus"), "corpus")
    turn_path = _external_path(inputs["turn_provenance"], "turn provenance")
    session_path = _external_path(inputs["session_provenance"], "session provenance")
    manifests = (
        _manifest(
            _load_json(turn_path, "turn provenance"),
            "turn provenance",
        ),
        _manifest(
            _load_json(session_path, "session provenance"),
            "session provenance",
        ),
    )
    active_sources = _active_trace_sources(
        _load_json(
            _external_path(inputs["trace_projection"], "TRACE sidecar"), "TRACE sidecar"
        )
    )
    relations = _relations(
        _load_json(
            _external_path(inputs["parent_relation_proof"], "parent relation proof"),
            "parent relation proof",
        ),
        active_sources,
        turn_manifest=manifests[0],
        session_manifest=manifests[1],
        turn_manifest_sha256=hashlib.sha256(turn_path.read_bytes()).hexdigest(),
        session_manifest_sha256=hashlib.sha256(session_path.read_bytes()).hexdigest(),
    )
    cell = request["cell"]
    assert isinstance(cell, Mapping)
    selected_cuda = (
        _require_single_visible_cuda() if cell["runtime"]["device"] == "gpu" else None
    )
    selected_questions = _select_questions(request, corpus)
    rows, evidence_by_logical = _ingest_rows(
        corpus,
        ingest_unit=str(cell["ingest_unit"]),
        manifest=manifests[0 if cell["ingest_unit"] == "turn" else 1],
    )
    output_root = _external_path(request["output_root"], "output root", directory=True)
    if output_root.exists():
        if any(output_root.iterdir()):
            raise AdapterError("output root must be empty for one cell")
    else:
        output_root.mkdir(parents=True, exist_ok=False)
    factory = engine_factory or _default_engine_factory
    engine = factory(
        str(output_root / "fathomdb.sqlite"), cell["retrieval"] == "hybrid"
    )
    try:
        started = time.monotonic()
        engine.write(rows)
        acknowledged = time.monotonic()
        engine.drain(timeout_s=120)
        ready = time.monotonic()
        if cell["runtime"]["cache_state"] == "steady" and selected_questions:
            _engine_search(engine, str(selected_questions[0]["query"]), cell)
        retrieved: list[list[str]] = []
        parent_bundles: list[list[dict[str, object]]] = []
        proof_parent_hits: list[dict[str, object]] | None = None
        elapsed: list[float] = []
        for question in selected_questions:
            began = time.monotonic()
            logical_ids = _engine_search(engine, str(question["query"]), cell)
            elapsed.append((time.monotonic() - began) * 1000)
            if any(logical_id not in evidence_by_logical for logical_id in logical_ids):
                raise AdapterError("FathomDB hit lacks canonical external provenance")
            if cell["program_track"] == "PARENT-01":
                child_hits = _parent_hits(logical_ids, relations)
                if proof_parent_hits is None:
                    proof_parent_hits = child_hits
                parent_bundles.append(_parent_bundles(child_hits))
            retrieved.append(_ordered_evidence(logical_ids, evidence_by_logical)[:10])
        summary = _metrics(
            selected_questions,
            retrieved,
            ingest_ack_ms=(acknowledged - started) * 1000,
            ready_ms=(ready - started) * 1000,
            query_ms=elapsed,
            parent=cell["program_track"] == "PARENT-01",
            relations=relations,
            parent_bundles=parent_bundles
            if cell["program_track"] == "PARENT-01"
            else None,
        )
    finally:
        close = getattr(engine, "close", None)
        if callable(close):
            close()
    metrics_path = output_root / "locomo-external-adapter-metrics.v1.json"
    metrics_path.write_text(
        json.dumps(
            {
                "schema_version": METRICS_SCHEMA,
                "cell_id": cell["cell_id"],
                "mode": request["mode"],
                "metric_summary": summary,
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    result: dict[str, object] = {
        "schema_version": RESULT_SCHEMA,
        "cell_id": cell["cell_id"],
        "mode": request["mode"],
        "external_metrics_ref": SAFE_METRICS_REF,
        "external_metrics_sha256": hashlib.sha256(
            metrics_path.read_bytes()
        ).hexdigest(),
        "metric_summary": summary,
    }
    if cell["program_track"] == "PARENT-01":
        if proof_parent_hits is None:
            raise AdapterError("PARENT cell did not produce a top-10 child proof")
        result["parent_hits"] = proof_parent_hits
    if selected_cuda is not None:
        result["device_attestation"] = {"device": selected_cuda, "cuda_available": True}
    return result


def main() -> int:
    """Read exactly one request from stdin and write exactly one safe JSON result."""
    try:
        result = execute_request(parse_request_json(sys.stdin.read()))
        print(json.dumps(result, sort_keys=True))
        return 0
    except (AdapterError, OSError, RuntimeError) as exc:
        print(f"locomo-external-adapter: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
