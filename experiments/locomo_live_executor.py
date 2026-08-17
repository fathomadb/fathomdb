"""Fail-closed external runner for authorized LOCOMO-01/PARENT-01 cells.

The runner is intentionally separate from :mod:`experiments.locomo_phase_b`.
That module remains the safe plan/receipt adapter; this module is the future
external-process seam and remains unusable without a coordinator release.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, Sequence

from eval.locomo_loader import corpus_hash, load_locomo

from experiments import locomo_phase_b, locomo_provenance, trace_projection


SCHEMA_VERSION = "locomo-live-executor.v1"
RELEASE_SCHEMA_VERSION = "locomo-live-executor.release.v1"
CELL_RESULT_SCHEMA_VERSION = "locomo-live-executor.cell-result.v1"
PROJECTION_SCHEMA_VERSION = "locomo-live-execution-projection.v1"
_REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
_SAFE_IDENTIFIER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_GIT_SHA = re.compile(r"^[0-9a-f]{40}$")
_CONFIG_KEYS = {"schema_version", "campaign", "program_tracks", "phase_b", "runner", "actions", "output"}
_PHASE_B_KEYS = {"path", "config_sha256"}
_RUNNER_KEYS = {"module", "sha256"}
_ACTION_KEYS = {"id", "mode", "selector", "expected_question_count", "cell_ids_sha256"}
_OUTPUT_KEYS = {"schema_version", "receipt_projection_schema", "repository_writes"}
_RELEASE_KEYS = {
    "schema_version", "release_id", "issued_by", "release_sha256", "integrated_git_sha",
    "phase_b_config_sha256", "executor_config_sha256", "runner_sha256", "independent_review_git_sha",
    "authorizations", "approved_actions", "gpu_policy", "external_roots", "cell_adapter", "review_evidence",
}
_ROOT_KEYS = {
    "artifact_root", "corpus", "turn_provenance", "session_provenance", "dry_run_subset", "trace_projection",
    "parent_relation_proof",
}
_ARTIFACT_ROOT_KEYS = {"path", "binding_sha256"}
_INPUT_ROOT_KEYS = {"path", "sha256"}
_ADAPTER_KEYS = {"path", "sha256"}
_GPU_POLICY_KEYS = {"cuda_required", "allow_cpu_fallback", "selected_device"}
_REVIEW_EVIDENCE_KEYS = {"path", "sha256"}


class LiveExecutorError(ValueError):
    """Raised when a release, external input, or adapter result is unsafe."""


@dataclass(frozen=True)
class LiveAction:
    """One immutable action gate and the exact cells it may execute."""

    action_id: str
    mode: str
    cell_ids: tuple[str, ...]
    expected_question_count: int


@dataclass(frozen=True)
class LiveExecutorPlan:
    """The frozen local-runner configuration resolved against Phase-B."""

    config_sha256: str
    phase_b_config_sha256: str
    runner_sha256: str
    program_tracks: tuple[str, ...]
    cells: tuple[locomo_phase_b.GridCell, ...]
    actions: tuple[LiveAction, ...]
    external_input_sha256: dict[str, str]

    def action(self, action_id: str) -> LiveAction:
        """Return one configured action or fail before any process starts."""
        for action in self.actions:
            if action.action_id == action_id:
                return action
        raise LiveExecutorError("unexpected execution action")


@dataclass(frozen=True)
class CellProjection:
    """Content-free result projection emitted by one external cell adapter."""

    cell_id: str
    mode: str
    external_metrics_ref: str
    external_metrics_sha256: str
    metric_summary: Mapping[str, object]
    parent_context: tuple[dict[str, object], ...]


def _exact(value: object, label: str, expected: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        raise LiveExecutorError(
            f"{label} keys mismatch: missing={sorted(expected - actual)}, unknown={sorted(actual - expected)}"
        )
    return value


def _identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_IDENTIFIER.fullmatch(value) is None:
        raise LiveExecutorError(f"{label} must be a safe identifier")
    return value


def _sha256(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise LiveExecutorError(f"{label} must be a lowercase sha256")
    return value


def _git_sha(value: object, label: str) -> str:
    if not isinstance(value, str) or _GIT_SHA.fullmatch(value) is None:
        raise LiveExecutorError(f"{label} must be a full lowercase git SHA")
    return value


def _canonical_json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")


def _canonical_sha256(value: object) -> str:
    return hashlib.sha256(_canonical_json(value)).hexdigest()


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _normalized_corpus_sha256(path: Path) -> str:
    """Return the frozen LOCOMO corpus identity for an externally held raw file."""
    try:
        documents, _ = load_locomo(path)
    except (OSError, ValueError, TypeError, AttributeError) as exc:
        raise LiveExecutorError("corpus cannot be normalized") from exc
    return corpus_hash(documents)


def _external_input_sha256(name: str, path: Path) -> str:
    """Return the release-pinned identity for one external Phase-B input."""
    return _normalized_corpus_sha256(path) if name == "corpus" else _file_sha256(path)


def _module_sha256() -> str:
    return _file_sha256(Path(__file__))


def _cell_ids_sha256(cell_ids: Sequence[str]) -> str:
    return _canonical_sha256(list(cell_ids))


def parse_adapter_json(value: str) -> dict[str, object]:
    """Parse one adapter result with duplicate-key rejection before validation."""
    def reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
        document: dict[str, object] = {}
        for key, value in pairs:
            if key in document:
                raise LiveExecutorError("JSON input contains duplicate keys")
            document[key] = value
        return document

    try:
        document = json.loads(value, object_pairs_hook=reject_duplicate_keys)
    except json.JSONDecodeError as exc:
        raise LiveExecutorError("JSON input is invalid") from exc
    if not isinstance(document, dict):
        raise LiveExecutorError("JSON input must be an object")
    return document


def _load_json(path: Path) -> dict[str, object]:
    try:
        return parse_adapter_json(path.read_text(encoding="utf-8"))
    except (OSError, LiveExecutorError) as exc:
        raise LiveExecutorError("required JSON document is unavailable or invalid") from exc


def _load_phase_b(reference: object) -> locomo_phase_b.LocomoPhaseBPlan:
    item = _exact(reference, "phase_b", _PHASE_B_KEYS)
    if item["path"] != "experiments/configs/locomo-01/phase-b-execution.v1.json":
        raise LiveExecutorError("live executor must consume the frozen Phase-B configuration")
    expected = _sha256(item["config_sha256"], "phase_b.config_sha256")
    path = _REPOSITORY_ROOT / item["path"]
    plan = locomo_phase_b.load_config(_load_json(path))
    if plan.config_sha256 != expected:
        raise LiveExecutorError("frozen Phase-B config digest drifted")
    return plan


def _selected_cells(plan: locomo_phase_b.LocomoPhaseBPlan, selector: object) -> tuple[str, ...]:
    if selector == "frozen_dry_run":
        return plan.dry_run_cell_ids
    if selector == "cpu_grid":
        return tuple(cell.cell_id for cell in plan.cells if cell.runtime["device"] == "cpu")
    if selector == "gpu_ce_grid":
        # This lane contains all remaining GPU cells, including every CE cell.
        # Keeping it whole makes CPU+GPU partitions exactly cover the frozen grid.
        return tuple(cell.cell_id for cell in plan.cells if cell.runtime["device"] == "gpu")
    raise LiveExecutorError("execution action selector is not authorized")


def load_config(document: object) -> LiveExecutorPlan:
    """Validate the separate local runner without contacting external systems."""
    root = _exact(document, "config", _CONFIG_KEYS)
    if root["schema_version"] != SCHEMA_VERSION or root["campaign"] != "authorized_locomo_parent_execution":
        raise LiveExecutorError("config must declare the authorized LOCOMO/PARENT live executor")
    if root["program_tracks"] != ["LOCOMO-01", "PARENT-01"]:
        raise LiveExecutorError("live executor must retain LOCOMO-01 and PARENT-01 tracks")
    runner = _exact(root["runner"], "runner", _RUNNER_KEYS)
    if runner["module"] != "experiments.locomo_live_executor":
        raise LiveExecutorError("runner module drifted")
    runner_sha256 = _sha256(runner["sha256"], "runner.sha256")
    if runner_sha256 != _module_sha256():
        raise LiveExecutorError("runner digest drifted")
    phase_b = _load_phase_b(root["phase_b"])
    actions = root["actions"]
    if not isinstance(actions, list) or len(actions) != 3:
        raise LiveExecutorError("config must contain exactly three separately gated actions")
    expected_actions = (
        ("fixed_subset_dry_run", "dry_run", "frozen_dry_run", 32),
        ("cpu_grid", "full_grid", "cpu_grid", 1536),
        ("gpu_ce_grid", "full_grid", "gpu_ce_grid", 1536),
    )
    resolved_actions: list[LiveAction] = []
    for raw, expected in zip(actions, expected_actions, strict=True):
        action = _exact(raw, "action", _ACTION_KEYS)
        action_id, mode, selector, question_count = expected
        if (
            action["id"] != action_id or action["mode"] != mode or action["selector"] != selector
            or action["expected_question_count"] != question_count
        ):
            raise LiveExecutorError("execution action semantics drifted")
        cell_ids = _selected_cells(phase_b, selector)
        if _sha256(action["cell_ids_sha256"], "action.cell_ids_sha256") != _cell_ids_sha256(cell_ids):
            raise LiveExecutorError("execution action cell membership drifted")
        resolved_actions.append(LiveAction(action_id, mode, cell_ids, question_count))
    cpu = resolved_actions[1].cell_ids
    gpu = resolved_actions[2].cell_ids
    if len(cpu) != 26 or len(gpu) != 26 or set(cpu) & set(gpu) or set(cpu) | set(gpu) != {cell.cell_id for cell in phase_b.cells}:
        raise LiveExecutorError("CPU and GPU/CE action partitions must exactly cover the frozen grid")
    output = _exact(root["output"], "output", _OUTPUT_KEYS)
    if output != {
        "schema_version": PROJECTION_SCHEMA_VERSION,
        "receipt_projection_schema": "locomo-phase-b.safe-receipt.v1",
        "repository_writes": "forbidden",
    }:
        raise LiveExecutorError("live executor output must remain external-only and receipt-compatible")
    inputs = phase_b.config["external_inputs"]
    assert isinstance(inputs, dict)  # validated by Phase-B before this boundary
    return LiveExecutorPlan(
        config_sha256=_canonical_sha256(root), phase_b_config_sha256=phase_b.config_sha256,
        runner_sha256=runner_sha256, program_tracks=("LOCOMO-01", "PARENT-01"), cells=phase_b.cells,
        actions=tuple(resolved_actions),
        external_input_sha256={key: str(inputs[key]["sha256"]) for key in (
            "corpus", "turn_provenance", "session_provenance", "dry_run_subset"
        )},
    )


def release_sha256(release: object) -> str:
    """Hash a release record while excluding only its declared self-hash field."""
    if not isinstance(release, Mapping):
        raise LiveExecutorError("release must be an object")
    unsigned = {key: value for key, value in release.items() if key != "release_sha256"}
    return _canonical_sha256(unsigned)


def release_binding_sha256(release: object) -> str:
    """Hash the reviewed release binding without self-hash/evidence path cycles."""
    if not isinstance(release, Mapping):
        raise LiveExecutorError("release must be an object")
    binding = {
        key: value for key, value in release.items()
        if key not in {"release_sha256", "review_evidence"}
    }
    return _canonical_sha256(binding)


def validate_release_shape(plan: LiveExecutorPlan, release: object, *, action: str) -> dict[str, Any]:
    """Validate coordinator authority and immutable bindings before filesystem access."""
    token = _exact(release, "release", _RELEASE_KEYS)
    if token["schema_version"] != RELEASE_SCHEMA_VERSION:
        raise LiveExecutorError("release schema mismatch")
    _identifier(token["release_id"], "release.release_id")
    if token["issued_by"] != "track-runner-coordinator":
        raise LiveExecutorError("release must be issued by the Track Runner coordinator")
    _sha256(token["release_sha256"], "release.release_sha256")
    if release_sha256(token) != token["release_sha256"]:
        raise LiveExecutorError("release self hash does not match its content")
    _git_sha(token["integrated_git_sha"], "release.integrated_git_sha")
    if _sha256(token["phase_b_config_sha256"], "release.phase_b_config_sha256") != plan.phase_b_config_sha256:
        raise LiveExecutorError("release Phase-B config digest drifted")
    if _sha256(token["executor_config_sha256"], "release.executor_config_sha256") != plan.config_sha256:
        raise LiveExecutorError("release executor config digest drifted")
    if _sha256(token["runner_sha256"], "release.runner_sha256") != plan.runner_sha256:
        raise LiveExecutorError("release runner digest drifted")
    _git_sha(token["independent_review_git_sha"], "release.independent_review_git_sha")
    if token["authorizations"] != ["seq-249", "seq-250"]:
        raise LiveExecutorError("release must retain both LOCOMO/PARENT authorizations")
    expected_actions = ["fixed_subset_dry_run"] if action == "fixed_subset_dry_run" else ["cpu_grid", "gpu_ce_grid"]
    if token["approved_actions"] != expected_actions:
        raise LiveExecutorError("release must approve the exact separately gated action set")
    expected_gpu_policy = {
        "cuda_required": action != "fixed_subset_dry_run", "allow_cpu_fallback": False,
        "selected_device": "cuda:0" if action != "fixed_subset_dry_run" else "cpu",
    }
    if _exact(token["gpu_policy"], "release.gpu_policy", _GPU_POLICY_KEYS) != expected_gpu_policy:
        raise LiveExecutorError("GPU policy drifted from the frozen action gate")
    roots = _exact(token["external_roots"], "release.external_roots", _ROOT_KEYS)
    artifact = _exact(roots["artifact_root"], "release.external_roots.artifact_root", _ARTIFACT_ROOT_KEYS)
    if not isinstance(artifact["path"], str) or not Path(artifact["path"]).is_absolute():
        raise LiveExecutorError("artifact root must be an absolute external path")
    _sha256(artifact["binding_sha256"], "artifact_root.binding_sha256")
    for key in (
        "corpus", "turn_provenance", "session_provenance", "dry_run_subset", "trace_projection",
        "parent_relation_proof",
    ):
        item = _exact(roots[key], f"release.external_roots.{key}", _INPUT_ROOT_KEYS)
        if not isinstance(item["path"], str) or not Path(item["path"]).is_absolute():
            raise LiveExecutorError(f"{key} must be an absolute external path")
        _sha256(item["sha256"], f"{key}.sha256")
    adapter = _exact(token["cell_adapter"], "release.cell_adapter", _ADAPTER_KEYS)
    if not isinstance(adapter["path"], str) or not Path(adapter["path"]).is_absolute():
        raise LiveExecutorError("cell adapter must be an absolute external path")
    _sha256(adapter["sha256"], "cell_adapter.sha256")
    review_evidence = _exact(token["review_evidence"], "release.review_evidence", _REVIEW_EVIDENCE_KEYS)
    if not isinstance(review_evidence["path"], str) or not Path(review_evidence["path"]).is_absolute():
        raise LiveExecutorError("review evidence must be an absolute external path")
    _sha256(review_evidence["sha256"], "review_evidence.sha256")
    return token


def _external_path(value: object, label: str, *, directory: bool) -> Path:
    if not isinstance(value, str):
        raise LiveExecutorError(f"{label} must be a path")
    path = Path(value).resolve()
    if path.is_relative_to(_REPOSITORY_ROOT) or "experiments/runs" in path.as_posix():
        raise LiveExecutorError(f"{label} must remain outside the repository and historical outputs")
    if (directory and not path.is_dir()) or (not directory and not path.is_file()):
        raise LiveExecutorError(f"{label} is unavailable")
    return path


def load_trace_lifecycle_proof(path: Path, expected_sha256: str) -> set[str]:
    """Validate the complete TRACE-01 schema before accepting active sources."""
    if _file_sha256(path) != expected_sha256:
        raise LiveExecutorError("TRACE lifecycle proof digest drifted")
    sidecar = _load_json(path)
    try:
        trace_projection._validate_sidecar(sidecar)
    except trace_projection.TraceProjectionError as exc:
        raise LiveExecutorError(f"TRACE lifecycle proof is not a complete trace-projection.v1 sidecar: {exc}") from exc
    sources = sidecar["sources"]
    projections = sidecar["projections"]
    assert isinstance(sources, list) and isinstance(projections, list)
    active: set[str] = set()
    for source in sources:
        assert isinstance(source, Mapping)
        source_id = str(source["source_id"])
        if source["lifecycle"] == "active":
            active.add(source_id)
    searchable_sources = {
        str(projection["source_id"])
        for projection in projections if isinstance(projection, Mapping) and projection["searchable"] is True
    }
    active &= searchable_sources
    if not active:
        raise LiveExecutorError("TRACE lifecycle proof has no active searchable sources")
    return active


def _canonical_provenance_entries(path: Path, expected_sha256: str, *, label: str) -> dict[str, dict[str, object]]:
    if _file_sha256(path) != expected_sha256:
        raise LiveExecutorError(f"{label} manifest digest drifted")
    document = _load_json(path)
    try:
        locomo_provenance.ProvenanceMap.from_document(document)
    except ValueError as exc:
        raise LiveExecutorError(f"{label} manifest is not canonical provenance") from exc
    entries = document["entries"]
    assert isinstance(entries, list)
    resolved: dict[str, dict[str, object]] = {}
    for raw in entries:
        assert isinstance(raw, Mapping)
        fingerprint = _sha256(raw["fingerprint"], f"{label} fingerprint")
        if fingerprint in resolved:
            raise LiveExecutorError(f"{label} manifest has duplicate fingerprints")
        resolved[fingerprint] = {
            "conversation_id": _identifier(raw["conversation_id"], f"{label} conversation_id"),
            "session_id": _identifier(raw["session_id"], f"{label} session_id"),
            "turn_ids": tuple(_identifier(item, f"{label} turn_id") for item in raw["turn_ids"]),
        }
    return resolved


def load_parent_relation_proof(
    path: Path, expected_sha256: str, turn_manifest_path: Path, turn_manifest_sha256: str,
    session_manifest_path: Path, session_manifest_sha256: str, *, active_trace_source_ids: set[str],
) -> dict[str, dict[str, object]]:
    """Load one hash-pinned, content-free parent/session membership inventory."""
    if _file_sha256(path) != expected_sha256:
        raise LiveExecutorError("parent membership/ordinal proof digest drifted")
    document = _load_json(path)
    if set(document) != {"schema_version", "turn_provenance_sha256", "session_provenance_sha256", "entries"}:
        raise LiveExecutorError("parent membership/ordinal proof fields are unsafe")
    if (
        document["schema_version"] != "locomo-parent-relation-proof.v2" or not isinstance(document["entries"], list)
        or document["turn_provenance_sha256"] != turn_manifest_sha256
        or document["session_provenance_sha256"] != session_manifest_sha256
    ):
        raise LiveExecutorError("parent membership/ordinal proof schema is unsafe")
    turn_entries = _canonical_provenance_entries(turn_manifest_path, turn_manifest_sha256, label="turn provenance")
    session_entries = _canonical_provenance_entries(session_manifest_path, session_manifest_sha256, label="session provenance")
    relations: dict[str, dict[str, object]] = {}
    for raw in document["entries"]:
        entry = _exact(
            raw, "parent relation entry",
            {
                "child_id", "parent_session_id", "ordinal", "trace_source_id", "turn_provenance_fingerprint",
                "session_provenance_fingerprint", "session_members",
            },
        )
        child_id = _identifier(entry["child_id"], "parent relation child_id")
        parent_session_id = _identifier(entry["parent_session_id"], "parent relation parent_session_id")
        ordinal = entry["ordinal"]
        trace_source_id = _identifier(entry["trace_source_id"], "parent relation trace_source_id")
        turn_fingerprint = _sha256(entry["turn_provenance_fingerprint"], "parent relation turn provenance fingerprint")
        session_fingerprint = _sha256(entry["session_provenance_fingerprint"], "parent relation session provenance fingerprint")
        if not isinstance(ordinal, int) or isinstance(ordinal, bool) or ordinal < 0:
            raise LiveExecutorError("parent relation ordinal is unsafe")
        if trace_source_id not in active_trace_source_ids:
            raise LiveExecutorError("parent relation TRACE attribution is not active")
        members = entry["session_members"]
        if not isinstance(members, list) or not members:
            raise LiveExecutorError("parent relation must enumerate the enclosing session")
        member_ordinals: dict[int, str] = {}
        for raw_member in members:
            member = _exact(raw_member, "parent relation session member", {"id", "ordinal", "trace_source_id"})
            member_id = _identifier(member["id"], "parent relation member id")
            member_ordinal = member["ordinal"]
            member_trace = _identifier(member["trace_source_id"], "parent relation member trace_source_id")
            if (
                not isinstance(member_ordinal, int) or isinstance(member_ordinal, bool) or member_ordinal < 0
                or member_trace != trace_source_id
            ):
                raise LiveExecutorError("parent relation session membership is unsafe")
            if member_ordinal in member_ordinals or member_id in member_ordinals.values():
                raise LiveExecutorError("parent relation session members must be unique")
            member_ordinals[member_ordinal] = member_id
        turn_entry = turn_entries.get(turn_fingerprint)
        session_entry = session_entries.get(session_fingerprint)
        if turn_entry is None or session_entry is None:
            raise LiveExecutorError("parent relation does not match canonical provenance membership/ordinal mapping")
        raw_turn_ids = turn_entry["turn_ids"]
        raw_session_members = session_entry["turn_ids"]
        assert isinstance(raw_turn_ids, tuple) and isinstance(raw_session_members, tuple)
        if len(raw_turn_ids) != 1:
            raise LiveExecutorError("parent relation does not match canonical provenance membership/ordinal mapping")
        expected_child_id = locomo_provenance.canonical_turn_id(
            turn_entry["conversation_id"], turn_entry["session_id"], raw_turn_ids[0]
        )
        expected_members = tuple(
            locomo_provenance.canonical_turn_id(
                session_entry["conversation_id"], session_entry["session_id"], member
            )
            for member in raw_session_members
        )
        if member_ordinals.get(ordinal) != child_id or child_id in relations:
            raise LiveExecutorError("parent relation must prove each child membership and ordinal exactly once")
        if (
            child_id != expected_child_id
            or turn_entry["conversation_id"] != session_entry["conversation_id"]
            or locomo_provenance.canonical_session_id(
                turn_entry["conversation_id"], turn_entry["session_id"]
            ) != parent_session_id
            or locomo_provenance.canonical_session_id(
                session_entry["conversation_id"], session_entry["session_id"]
            ) != parent_session_id
            or expected_members != tuple(member_ordinals[index] for index in sorted(member_ordinals))
        ):
            raise LiveExecutorError("parent relation does not match canonical provenance membership/ordinal mapping")
        relations[child_id] = {
            "parent_session_id": parent_session_id, "ordinal": ordinal, "trace_source_id": trace_source_id,
            "member_ordinals": member_ordinals,
        }
    if not relations:
        raise LiveExecutorError("parent membership/ordinal proof is empty")
    return relations


def validate_review_evidence(
    path: Path, expected_sha256: str, release: Mapping[str, object], *, release_binding_sha256: str,
) -> None:
    """Require an accepted review record bound to this exact release binding."""
    if _file_sha256(path) != expected_sha256:
        raise LiveExecutorError("review evidence digest drifted")
    record = _load_json(path)
    expected = {
        "schema_version", "verdict", "review_git_sha", "release_id", "release_binding_sha256",
        "integrated_git_sha", "phase_b_config_sha256", "executor_config_sha256", "runner_sha256",
    }
    if set(record) != expected or record["schema_version"] != "locomo-live-executor.review-evidence.v1":
        raise LiveExecutorError("review evidence schema is unsafe")
    if record["verdict"] != "accepted":
        raise LiveExecutorError("review evidence verdict is not accepted")
    for key in ("review_git_sha", "integrated_git_sha"):
        _git_sha(record[key], f"review evidence {key}")
    for key in ("release_binding_sha256", "phase_b_config_sha256", "executor_config_sha256", "runner_sha256"):
        _sha256(record[key], f"review evidence {key}")
    release_keys = {
        "review_git_sha": "independent_review_git_sha", "release_id": "release_id",
        "integrated_git_sha": "integrated_git_sha", "phase_b_config_sha256": "phase_b_config_sha256",
        "executor_config_sha256": "executor_config_sha256", "runner_sha256": "runner_sha256",
    }
    for record_key, release_key in release_keys.items():
        if record[record_key] != release[release_key]:
            raise LiveExecutorError("review evidence does not bind the released review/configuration")
    if record["release_binding_sha256"] != release_binding_sha256:
        raise LiveExecutorError("review evidence does not bind the exact release")


def require_cuda_device(selected_device: str) -> None:
    """Check that the released CUDA device is actually enumerated on this host."""
    if not isinstance(selected_device, str) or re.fullmatch(r"cuda:[0-9]+", selected_device) is None:
        raise LiveExecutorError("selected CUDA device is unsafe")
    try:
        completed = subprocess.run(
            ["nvidia-smi", "--query-gpu=index", "--format=csv,noheader"], check=False, text=True, capture_output=True,
        )
    except OSError as exc:
        raise LiveExecutorError("selected CUDA device is unavailable") from exc
    selected_index = selected_device.split(":", 1)[1]
    devices = {line.strip() for line in completed.stdout.splitlines() if re.fullmatch(r"[0-9]+", line.strip())}
    if completed.returncode != 0 or selected_index not in devices:
        raise LiveExecutorError("selected CUDA device is unavailable")


def validate_gpu_attestation(value: object, *, selected_device: str) -> None:
    """Reject an adapter result that does not attest to the released CUDA device."""
    attestation = _exact(value, "adapter GPU attestation", {"device", "cuda_available"})
    if attestation != {"device": selected_device, "cuda_available": True}:
        raise LiveExecutorError("adapter GPU attestation rejects CPU fallback or device drift")


def validate_release(
    plan: LiveExecutorPlan, release: object, *, action: str
) -> tuple[dict[str, Any], set[str], dict[str, dict[str, object]]]:
    """Check release shape, current integration, files, digests, and lifecycle proof."""
    token = validate_release_shape(plan, release, action=action)
    git_env = dict(os.environ)
    git_env.pop("GIT_DIR", None)
    git_env.pop("GIT_WORK_TREE", None)
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=_REPOSITORY_ROOT, check=False, text=True,
        capture_output=True, env=git_env,
    )
    if completed.returncode != 0 or completed.stdout.strip() != token["integrated_git_sha"]:
        raise LiveExecutorError("release does not bind the current integrated campaign SHA")
    roots = token["external_roots"]
    assert isinstance(roots, Mapping)
    artifact_root = _external_path(roots["artifact_root"]["path"], "artifact root", directory=True)
    if not artifact_root.is_dir():  # keeps the type checker honest about the release path
        raise LiveExecutorError("artifact root is unavailable")
    for key, expected_sha in plan.external_input_sha256.items():
        path = _external_path(roots[key]["path"], key, directory=False)
        if roots[key]["sha256"] != expected_sha or _external_input_sha256(key, path) != expected_sha:
            raise LiveExecutorError(f"{key} provenance or corpus digest drifted")
    trace_path = _external_path(roots["trace_projection"]["path"], "TRACE lifecycle proof", directory=False)
    active_trace_sources = load_trace_lifecycle_proof(trace_path, roots["trace_projection"]["sha256"])
    relation_path = _external_path(roots["parent_relation_proof"]["path"], "parent membership/ordinal proof", directory=False)
    parent_relations = load_parent_relation_proof(
        relation_path, roots["parent_relation_proof"]["sha256"],
        _external_path(roots["turn_provenance"]["path"], "turn provenance", directory=False),
        roots["turn_provenance"]["sha256"],
        _external_path(roots["session_provenance"]["path"], "session provenance", directory=False),
        roots["session_provenance"]["sha256"], active_trace_source_ids=active_trace_sources,
    )
    adapter = _external_path(token["cell_adapter"]["path"], "cell adapter", directory=False)
    if not os.access(adapter, os.X_OK) or _file_sha256(adapter) != token["cell_adapter"]["sha256"]:
        raise LiveExecutorError("cell adapter is unavailable or digest drifted")
    review = token["review_evidence"]
    assert isinstance(review, Mapping)
    validate_review_evidence(
        _external_path(review["path"], "review evidence", directory=False), review["sha256"], token,
        release_binding_sha256=release_binding_sha256(token),
    )
    if action == "gpu_ce_grid":
        require_cuda_device(str(token["gpu_policy"]["selected_device"]))
    return token, active_trace_sources, parent_relations


def synthetic_metric_summary(*, parent: bool) -> dict[str, object]:
    """Return safe fixture-only complete metrics for lifecycle contract tests."""
    metrics: dict[str, object] = {
        "m1": {"r_at_10": 1.0}, "m2": {"mrr": 1.0, "r_at_1": 1.0, "ndcg_at_10": 1.0},
        "m4_proxy": {"temporal_evidence_recall": 1.0},
        "m6": {"facade_query_ms": 1.0, "engine_query_ms": 1.0},
        "m7": {"ingest_ack_ms": 1.0, "ready_to_search_ms": 1.0},
        "class_metrics": {
            "factoid": {"r_at_10": 1.0}, "temporal": {"r_at_10": 1.0}, "multi_session": {"r_at_10": 1.0},
        },
    }
    if parent:
        metrics["parent_metrics"] = {
            "child_evidence_recall": 1.0, "parent_session_recall": 1.0, "duplicate_rate": 0.0,
            "context_expansion_count": 0, "class_latency_ms": {"factoid": 1.0, "temporal": 1.0, "multi_session": 1.0},
        }
    return metrics


def _trace_ids_from_hits(hits: Sequence[object]) -> set[str]:
    trace_ids: set[str] = set()
    for hit in hits:
        if not isinstance(hit, Mapping):
            continue
        provenance = hit.get("child_provenance")
        if isinstance(provenance, Mapping) and isinstance(provenance.get("trace_source_id"), str):
            trace_ids.add(provenance["trace_source_id"])
        neighbors = hit.get("neighbors")
        if isinstance(neighbors, list):
            for neighbor in neighbors:
                if isinstance(neighbor, Mapping) and isinstance(neighbor.get("trace_source_id"), str):
                    trace_ids.add(neighbor["trace_source_id"])
    return trace_ids


def validate_cell_result(
    plan: LiveExecutorPlan, cell: locomo_phase_b.GridCell, result: object, *, active_trace_source_ids: set[str],
    parent_relations: Mapping[str, Mapping[str, object]], selected_cuda_device: str | None = None,
) -> CellProjection:
    """Validate one adapter response and project only safe LOCOMO/PARENT evidence."""
    expected = {
        "schema_version", "cell_id", "mode", "external_metrics_ref", "external_metrics_sha256", "metric_summary",
    }
    if cell.runtime["device"] == "gpu":
        expected.add("device_attestation")
    if cell.program_track == "PARENT-01":
        expected.add("parent_hits")
    item = _exact(result, "cell result", expected)
    if item["schema_version"] != CELL_RESULT_SCHEMA_VERSION or item["cell_id"] != cell.cell_id:
        raise LiveExecutorError("adapter result does not bind the dispatched frozen cell")
    if item["mode"] not in {"dry_run", "full_grid"}:
        raise LiveExecutorError("adapter result mode is unsafe")
    _identifier(item["external_metrics_ref"], "external_metrics_ref")
    _sha256(item["external_metrics_sha256"], "external_metrics_sha256")
    if cell.runtime["device"] == "gpu":
        if selected_cuda_device is None:
            raise LiveExecutorError("GPU result lacks a released selected CUDA device")
        validate_gpu_attestation(item["device_attestation"], selected_device=selected_cuda_device)
    phase_result = locomo_phase_b.CellExecutionResult(
        cell_id=item["cell_id"], mode=item["mode"], external_metrics_ref=item["external_metrics_ref"],
        external_metrics_sha256=item["external_metrics_sha256"], metric_summary=item["metric_summary"],
    )
    try:
        locomo_phase_b._validate_result(phase_result)
        locomo_phase_b._validate_complete_metric_summary(phase_result, cell)
    except locomo_phase_b.LocomoPhaseBError as exc:
        raise LiveExecutorError(f"adapter result metric contract failed: {exc}") from exc
    parent_context: tuple[dict[str, object], ...] = ()
    if cell.program_track == "PARENT-01":
        hits = item["parent_hits"]
        if not isinstance(hits, list):
            raise LiveExecutorError("parent result must provide exact membership/ordinal proof")
        trace_ids = _trace_ids_from_hits(hits)
        if not trace_ids or not trace_ids <= active_trace_source_ids:
            raise LiveExecutorError("parent result TRACE attribution is not active lifecycle proof")
        for raw_hit in hits:
            if not isinstance(raw_hit, Mapping) or not isinstance(raw_hit.get("child_id"), str):
                raise LiveExecutorError("parent result membership/ordinal proof is unsafe")
            relation = parent_relations.get(raw_hit["child_id"])
            provenance = raw_hit.get("child_provenance")
            if relation is None or not isinstance(provenance, Mapping):
                raise LiveExecutorError("parent result child is absent from the membership/ordinal proof")
            if (
                provenance.get("parent_session_ids") != [relation["parent_session_id"]]
                or provenance.get("ordinal") != relation["ordinal"]
                or provenance.get("trace_source_id") != relation["trace_source_id"]
            ):
                raise LiveExecutorError("parent result membership/ordinal proof does not match the child")
            member_ordinals = relation["member_ordinals"]
            assert isinstance(member_ordinals, Mapping)
            for neighbor in raw_hit.get("neighbors", []):
                if not isinstance(neighbor, Mapping) or member_ordinals.get(neighbor.get("ordinal")) != neighbor.get("id"):
                    raise LiveExecutorError("parent result neighbor is absent from the exact session membership proof")
                if neighbor.get("trace_source_id") != relation["trace_source_id"]:
                    raise LiveExecutorError("parent result neighbor TRACE attribution drifted")
        try:
            bundles = locomo_phase_b.parent_child_bundles(hits)
        except locomo_phase_b.LocomoPhaseBError as exc:
            raise LiveExecutorError(str(exc)) from exc
        parent_context = tuple(bundles)
    return CellProjection(
        cell_id=phase_result.cell_id, mode=phase_result.mode, external_metrics_ref=phase_result.external_metrics_ref,
        external_metrics_sha256=phase_result.external_metrics_sha256, metric_summary=dict(phase_result.metric_summary),
        parent_context=parent_context,
    )


def _projection_path(artifact_root: Path, release_id: str, action: LiveAction) -> Path:
    return artifact_root / release_id / action.action_id / "locomo-live-execution-projection.v1.json"


def action_projection_document(
    plan: LiveExecutorPlan, action: LiveAction, release: Mapping[str, object], results: Sequence[CellProjection],
) -> dict[str, object]:
    """Build one complete, content-free action document before any external write."""
    if len(results) != len(action.cell_ids):
        raise LiveExecutorError("action is not complete; it is not receipt/index eligible")
    if [result.cell_id for result in results] != list(action.cell_ids):
        raise LiveExecutorError("action results must be complete, ordered, and duplicate-free")
    if any(result.mode != action.mode for result in results):
        raise LiveExecutorError("action result modes drifted")
    return {
        "schema_version": PROJECTION_SCHEMA_VERSION,
        "release_id": release["release_id"],
        "release_sha256": release["release_sha256"],
        "phase_b_config_sha256": plan.phase_b_config_sha256,
        "executor_config_sha256": plan.config_sha256,
        "runner_sha256": plan.runner_sha256,
        "program_tracks": list(plan.program_tracks),
        "action": action.action_id,
        "mode": action.mode,
        "expected_question_count": action.expected_question_count,
        "cell_ids": list(action.cell_ids),
        "result_count": len(results),
        "receipt_status": "dry_run_proof" if action.mode == "dry_run" else "not_eligible_until_cpu_and_gpu_actions_complete",
        "index_eligible": action.mode == "dry_run",
        "results": [
            {
                "cell_id": result.cell_id, "mode": result.mode, "external_metrics_ref": result.external_metrics_ref,
                "external_metrics_sha256": result.external_metrics_sha256, "metric_summary": result.metric_summary,
                "parent_context": list(result.parent_context),
            }
            for result in results
        ],
    }


def write_action_projection(
    artifact_root: Path, *, release_id: str, release_sha: str, plan: LiveExecutorPlan, action: LiveAction,
    results: Sequence[CellProjection],
) -> Path:
    """Write one external-only, content-free action projection after complete coverage."""
    payload = action_projection_document(plan, action, {"release_id": release_id, "release_sha256": release_sha}, results)
    output_path = _projection_path(artifact_root, release_id, action)
    if output_path.exists():
        raise LiveExecutorError("duplicate action projection is forbidden")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(payload, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    return output_path


def _validate_action_projection(
    plan: LiveExecutorPlan, action: LiveAction, release: Mapping[str, object], document: object,
) -> list[dict[str, object]]:
    expected = {
        "schema_version", "release_id", "release_sha256", "phase_b_config_sha256", "executor_config_sha256",
        "runner_sha256", "program_tracks", "action", "mode", "expected_question_count", "cell_ids", "result_count",
        "receipt_status", "index_eligible", "results",
    }
    item = _exact(document, "action projection", expected)
    if (
        item["schema_version"] != PROJECTION_SCHEMA_VERSION or item["release_id"] != release["release_id"]
        or item["release_sha256"] != release["release_sha256"] or item["phase_b_config_sha256"] != plan.phase_b_config_sha256
        or item["executor_config_sha256"] != plan.config_sha256 or item["runner_sha256"] != plan.runner_sha256
        or item["program_tracks"] != list(plan.program_tracks) or item["action"] != action.action_id or item["mode"] != action.mode
        or item["expected_question_count"] != action.expected_question_count or item["cell_ids"] != list(action.cell_ids)
        or item["result_count"] != len(action.cell_ids) or item["receipt_status"] != "not_eligible_until_cpu_and_gpu_actions_complete"
        or item["index_eligible"] is not False or not isinstance(item["results"], list)
    ):
        raise LiveExecutorError("action projection is not a complete same-release/config full-grid component")
    if [row.get("cell_id") if isinstance(row, Mapping) else None for row in item["results"]] != list(action.cell_ids):
        raise LiveExecutorError("action projection has partial, duplicate, or reordered cells")
    return [dict(row) for row in item["results"] if isinstance(row, Mapping)]


def combine_full_grid_projections(
    plan: LiveExecutorPlan, release: Mapping[str, object], cpu_projection: object, gpu_projection: object,
) -> dict[str, object]:
    """Close only one same-release, exact-52-cell full-grid evidence set."""
    expected_actions = ["cpu_grid", "gpu_ce_grid"]
    if release.get("approved_actions", expected_actions) != expected_actions:
        raise LiveExecutorError("full-grid closure requires one release with both separate CPU/GPU gates")
    cpu = _validate_action_projection(plan, plan.action("cpu_grid"), release, cpu_projection)
    gpu = _validate_action_projection(plan, plan.action("gpu_ce_grid"), release, gpu_projection)
    by_id = {row["cell_id"]: row for row in [*cpu, *gpu]}
    if len(by_id) != 52 or set(by_id) != {cell.cell_id for cell in plan.cells}:
        raise LiveExecutorError("full-grid closure requires exactly 52 unique complete result cells")
    return {
        "schema_version": "locomo-live-full-grid-projection.v1",
        "release_id": release["release_id"], "release_sha256": release["release_sha256"],
        "phase_b_config_sha256": plan.phase_b_config_sha256, "executor_config_sha256": plan.config_sha256,
        "runner_sha256": plan.runner_sha256, "result_count": 52, "receipt_status": "complete",
        "index_eligible": True, "results": [by_id[cell.cell_id] for cell in plan.cells],
    }


def finalize_full_grid(plan: LiveExecutorPlan, release: object) -> Path:
    """Combine only the two same-release full-grid action projections externally."""
    token, _, _ = validate_release(plan, release, action="gpu_ce_grid")
    roots = token["external_roots"]
    assert isinstance(roots, Mapping)
    artifact_root = Path(roots["artifact_root"]["path"]).resolve()
    cpu_path = _projection_path(artifact_root, str(token["release_id"]), plan.action("cpu_grid"))
    gpu_path = _projection_path(artifact_root, str(token["release_id"]), plan.action("gpu_ce_grid"))
    if not cpu_path.is_file() or not gpu_path.is_file():
        raise LiveExecutorError("full-grid closure requires both complete same-release action projections")
    output_path = artifact_root / str(token["release_id"]) / "full-grid" / "locomo-live-full-grid-projection.v1.json"
    if output_path.exists():
        raise LiveExecutorError("duplicate full-grid closure projection is forbidden")
    combined = combine_full_grid_projections(plan, token, _load_json(cpu_path), _load_json(gpu_path))
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(combined, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    return output_path


def run_action(plan: LiveExecutorPlan, release: object, *, action: str) -> Path:
    """Invoke the released external adapter once per exact frozen cell.

    This function is intentionally not used by tests with a valid release.  A
    future coordinator invocation is the first permitted external execution.
    """
    token, active_trace_sources, parent_relations = validate_release(plan, release, action=action)
    selected = plan.action(action)
    roots = token["external_roots"]
    assert isinstance(roots, Mapping)
    artifact_root = Path(roots["artifact_root"]["path"]).resolve()
    output_path = _projection_path(artifact_root, token["release_id"], selected)
    if output_path.exists():
        raise LiveExecutorError("duplicate action projection is forbidden")
    adapter = str(token["cell_adapter"]["path"])
    cells = {cell.cell_id: cell for cell in plan.cells}
    results: list[CellProjection] = []
    for cell_id in selected.cell_ids:
        cell = cells[cell_id]
        request = {
            "schema_version": "locomo-live-executor.request.v1", "release_id": token["release_id"],
            "action": action, "mode": selected.mode,
            "cell": {
                "cell_id": cell.cell_id, "program_track": cell.program_track, "ingest_unit": cell.ingest_unit,
                "treatment": cell.treatment, "retrieval": cell.retrieval, "runtime": cell.runtime,
                "parent_child": cell.parent_child,
            },
            "external_inputs": {key: roots[key]["path"] for key in (
                "corpus", "turn_provenance", "session_provenance", "dry_run_subset", "trace_projection",
                "parent_relation_proof",
            )},
            "output_root": str(artifact_root / token["release_id"] / action / "raw" / cell.cell_id),
        }
        completed = subprocess.run(
            [adapter], input=json.dumps(request, sort_keys=True), text=True, capture_output=True, check=False,
        )
        if completed.returncode != 0:
            raise LiveExecutorError("external cell adapter failed; no partial projection was written")
        result = parse_adapter_json(completed.stdout)
        projection = validate_cell_result(
            plan, cell, result, active_trace_source_ids=active_trace_sources, parent_relations=parent_relations,
            selected_cuda_device=str(token["gpu_policy"]["selected_device"]) if action == "gpu_ce_grid" else None,
        )
        if projection.mode != selected.mode:
            raise LiveExecutorError("adapter result mode does not match its released action")
        results.append(projection)
    return write_action_projection(
        artifact_root, release_id=token["release_id"], release_sha=token["release_sha256"], plan=plan,
        action=selected, results=results,
    )


def main(argv: list[str] | None = None) -> int:
    """Validate, preview, or later execute one coordinator-released action."""
    parser = argparse.ArgumentParser(description="LOCOMO/PARENT release-bound external executor")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("validate", "preview"):
        command = sub.add_parser(name)
        command.add_argument("config", type=Path)
    execute = sub.add_parser("execute")
    execute.add_argument("config", type=Path)
    execute.add_argument("release", type=Path)
    execute.add_argument("--action", required=True)
    finalize = sub.add_parser("finalize-full-grid")
    finalize.add_argument("config", type=Path)
    finalize.add_argument("release", type=Path)
    args = parser.parse_args(argv)
    try:
        plan = load_config(_load_json(args.config))
        if args.command == "validate":
            print("LOCOMO/PARENT live executor config resolves")
            return 0
        if args.command == "preview":
            print(json.dumps({
                "schema_version": "locomo-live-executor.preview.v1", "program_tracks": list(plan.program_tracks),
                "actions": [{"id": item.action_id, "mode": item.mode, "cell_count": len(item.cell_ids)} for item in plan.actions],
            }, sort_keys=True))
            return 0
        if args.command == "finalize-full-grid":
            finalize_full_grid(plan, _load_json(args.release))
            return 0
        run_action(plan, _load_json(args.release), action=args.action)
        return 0
    except (LiveExecutorError, OSError) as exc:
        print(f"locomo-live-executor: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
