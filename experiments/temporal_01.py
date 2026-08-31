"""Content-free factual preflight for the TEMPORAL-01 input portfolio.

This module checks registered external inputs and their structural capability.
It neither creates a FathomDB database nor invokes a model or a live
measurement. A passing input check is deliberately *not* an execution release:
the treatment remains blocked until an external validity-window manifest and
adapter are reviewed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping


POLICY_SCHEMA_VERSION = "temporal-01-factual-preflight-policy.v1"
REPORT_SCHEMA_VERSION = "temporal-01-factual-preflight-report.v1"
PROGRAM_TRACK = "TEMPORAL-01"
CLAIM_BOUNDARY = "world_time_validity_only_no_history_as_of_or_supersession_claim"
BLOCKERS = (
    "external_validity_window_manifest_missing",
    "history_as_of_not_supported",
    "temporal_adapter_not_implemented",
)

_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_IDENTIFIER = re.compile(r"^[a-z][a-z0-9_-]{1,63}$")
_SAFE_LABEL = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:+-]{0,127}$")
_POLICY_KEYS = {
    "schema_version",
    "program_track",
    "preflight_id",
    "registry_sha256",
    "inputs",
    "baseline",
    "treatment",
    "claim_boundary",
}
_REPORT_KEYS = {
    "schema_version",
    "program_track",
    "preflight_id",
    "state",
    "eligible_for_execution",
    "registry_sha256",
    "baseline",
    "treatment",
    "claim_boundary",
    "corpora",
    "blockers",
    "no_live_execution",
}


class Temporal01PreflightError(ValueError):
    """Raised when TEMPORAL-01's factual input boundary is not satisfied."""


@dataclass(frozen=True)
class Temporal01Policy:
    """The fixed, content-free source selection for factual preparation."""

    preflight_id: str
    registry_sha256: str
    timeqa_files: tuple[str, ...]
    longmemeval_file: str
    longmemeval_class: str
    timelineqa_densities: tuple[str, ...]
    baseline: str
    treatment: str
    claim_boundary: str


@dataclass(frozen=True)
class Temporal01Preflight:
    """Verified input facts with an explicit no-execution boundary."""

    policy: Temporal01Policy
    corpora: Mapping[str, Mapping[str, object]]

    @property
    def state(self) -> str:
        """Return the factual state without implying an approved measurement."""
        return "input_facts_confirmed_treatment_blocked"

    def safe_report(self) -> dict[str, object]:
        """Project only hashes, counts, capability facts, and blockers."""
        return {
            "schema_version": REPORT_SCHEMA_VERSION,
            "program_track": PROGRAM_TRACK,
            "preflight_id": self.policy.preflight_id,
            "state": self.state,
            "eligible_for_execution": False,
            "registry_sha256": self.policy.registry_sha256,
            "baseline": self.policy.baseline,
            "treatment": self.policy.treatment,
            "claim_boundary": self.policy.claim_boundary,
            "corpora": dict(self.corpora),
            "blockers": list(BLOCKERS),
            "no_live_execution": True,
        }


def canonical_sha256(value: object) -> str:
    """Hash canonical JSON for stable, content-free configuration bindings."""
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    ).hexdigest()


def _sha256_path(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _require_sha(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise Temporal01PreflightError(f"{label} must be a lowercase sha256")
    return value


def _require_identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _IDENTIFIER.fullmatch(value) is None:
        raise Temporal01PreflightError(f"{label} must be a stable identifier")
    return value


def _require_label(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_LABEL.fullmatch(value) is None:
        raise Temporal01PreflightError(f"{label} must be a safe label")
    return value


def _mapping(value: object, label: str) -> Mapping[str, object]:
    if not isinstance(value, dict):
        raise Temporal01PreflightError(f"{label} must be an object")
    return value


def _string_list(value: object, label: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not value or not all(isinstance(item, str) and item for item in value):
        raise Temporal01PreflightError(f"{label} must be a non-empty list of strings")
    values = tuple(value)
    if len(values) != len(set(values)):
        raise Temporal01PreflightError(f"{label} must not contain duplicates")
    return values


def load_policy(document: object) -> Temporal01Policy:
    """Validate the checked-in TEMPORAL-01 factual-preflight policy."""
    policy = _mapping(document, "policy")
    if set(policy) != _POLICY_KEYS:
        raise Temporal01PreflightError("policy keys do not match temporal-01 factual-preflight v1")
    if policy["schema_version"] != POLICY_SCHEMA_VERSION or policy["program_track"] != PROGRAM_TRACK:
        raise Temporal01PreflightError("policy identity is invalid")
    inputs = _mapping(policy["inputs"], "policy inputs")
    if set(inputs) != {"timeqa", "longmemeval", "timelineqa"}:
        raise Temporal01PreflightError("policy inputs must select the three registered temporal corpora")
    timeqa = _mapping(inputs["timeqa"], "TimeQA policy")
    longmemeval = _mapping(inputs["longmemeval"], "LongMemEval policy")
    timelineqa = _mapping(inputs["timelineqa"], "TimelineQA policy")
    if set(timeqa) != {"files"} or set(longmemeval) != {"file", "class"} or set(timelineqa) != {"densities"}:
        raise Temporal01PreflightError("policy input selection keys are invalid")
    files = _string_list(timeqa["files"], "TimeQA files")
    if files != ("test.easy.json", "test.hard.json"):
        raise Temporal01PreflightError("TimeQA policy must use the held-out easy and hard test files")
    lme_file = longmemeval["file"]
    lme_class = longmemeval["class"]
    if lme_file != "longmemeval_s_cleaned.json" or lme_class != "temporal-reasoning":
        raise Temporal01PreflightError("LongMemEval policy must use the S temporal slice")
    densities = _string_list(timelineqa["densities"], "TimelineQA densities")
    if densities != ("sparse", "medium", "dense"):
        raise Temporal01PreflightError("TimelineQA policy must retain the three density strata")
    if policy["baseline"] != "a0_turn_fts" or policy["treatment"] != "readview_valid_as_of":
        raise Temporal01PreflightError("policy comparison is not the approved single baseline/treatment pair")
    if policy["claim_boundary"] != CLAIM_BOUNDARY:
        raise Temporal01PreflightError("policy claim boundary is invalid")
    return Temporal01Policy(
        preflight_id=_require_identifier(policy["preflight_id"], "preflight_id"),
        registry_sha256=_require_sha(policy["registry_sha256"], "registry_sha256"),
        timeqa_files=files,
        longmemeval_file=lme_file,
        longmemeval_class=lme_class,
        timelineqa_densities=densities,
        baseline=policy["baseline"],
        treatment=policy["treatment"],
        claim_boundary=policy["claim_boundary"],
    )


def _registry_source(registry: Mapping[str, object], corpus_id: str) -> Mapping[str, object]:
    sources = _mapping(registry.get("sources"), "registry sources")
    return _mapping(sources.get(corpus_id), f"registry {corpus_id}")


def _timeqa_facts(policy: Temporal01Policy, registry: Mapping[str, object], data_root: Path) -> dict[str, object]:
    source = _registry_source(registry, "timeqa")
    if source.get("license") != "BSD-3-Clause":
        raise Temporal01PreflightError("TimeQA license is not the registered BSD-3-Clause basis")
    files = _mapping(source.get("files"), "TimeQA registry files")
    question_count = 0
    unanswerable_count = 0
    payload_hashes: list[str] = []
    required = {"idx", "context", "paragraphs", "question", "targets"}
    for name in policy.timeqa_files:
        metadata = _mapping(files.get(name), f"TimeQA registry {name}")
        path = data_root / "timeqa" / name
        if not path.is_file():
            raise Temporal01PreflightError("TimeQA payload is unavailable")
        actual_hash = _sha256_path(path)
        if actual_hash != _require_sha(metadata.get("sha256"), "TimeQA registry digest"):
            raise Temporal01PreflightError("TimeQA payload digest does not match the registry")
        count = 0
        with path.open(encoding="utf-8") as source_file:
            for line in source_file:
                if not line.strip():
                    continue
                row = _mapping(json.loads(line), "TimeQA record")
                if not required.issubset(row) or not isinstance(row["targets"], list):
                    raise Temporal01PreflightError("TimeQA record lacks the registered retrieval/answer fields")
                count += 1
                if not row["targets"]:
                    unanswerable_count += 1
        if count != metadata.get("qa_count"):
            raise Temporal01PreflightError("TimeQA question count does not match the registry")
        question_count += count
        payload_hashes.append(actual_hash)
    return {
        "license": "BSD-3-Clause",
        "payload_sha256": canonical_sha256(payload_hashes),
        "question_count": question_count,
        "unanswerable_count": unanswerable_count,
        "capability": "time_sensitive_answer_and_abstention_only",
    }


def _longmemeval_facts(policy: Temporal01Policy, registry: Mapping[str, object], data_root: Path) -> dict[str, object]:
    source = _registry_source(registry, "longmemeval")
    if source.get("license") != "MIT":
        raise Temporal01PreflightError("LongMemEval license is not the registered MIT basis")
    metadata = _mapping(_mapping(source.get("files"), "LongMemEval registry files").get(policy.longmemeval_file), "LongMemEval file")
    path = data_root / "longmemeval-cleaned" / policy.longmemeval_file
    if not path.is_file():
        raise Temporal01PreflightError("LongMemEval payload is unavailable")
    actual_hash = _sha256_path(path)
    if actual_hash != _require_sha(metadata.get("sha256"), "LongMemEval registry digest"):
        raise Temporal01PreflightError("LongMemEval payload digest does not match the registry")
    rows = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(rows, list) or len(rows) != metadata.get("instances"):
        raise Temporal01PreflightError("LongMemEval instance count does not match the registry")
    temporal_rows = [row for row in rows if isinstance(row, dict) and row.get("question_type") == policy.longmemeval_class]
    declared_counts = _mapping(metadata.get("question_type_counts"), "LongMemEval class counts")
    if len(temporal_rows) != declared_counts.get(policy.longmemeval_class):
        raise Temporal01PreflightError("LongMemEval temporal class count does not match the registry")
    required = {"question_date", "haystack_dates", "haystack_sessions", "answer_session_ids"}
    dated = 0
    for row in temporal_rows:
        if not required.issubset(row):
            raise Temporal01PreflightError("LongMemEval temporal record lacks required temporal/evidence fields")
        if isinstance(row["question_date"], str) and isinstance(row["haystack_dates"], list) and isinstance(row["haystack_sessions"], list) and len(row["haystack_dates"]) == len(row["haystack_sessions"]):
            dated += 1
    upstream = _mapping(source.get("upstream"), "LongMemEval upstream")
    return {
        "license": "MIT",
        "license_copy_sha256": _require_sha(source.get("license_sha256"), "LongMemEval license digest"),
        "source_revision": _require_label(upstream.get("revision"), "LongMemEval source revision"),
        "payload_sha256": actual_hash,
        "instance_count": len(rows),
        "temporal_question_count": len(temporal_rows),
        "dated_temporal_question_count": dated,
        "capability": "session_ordering_requires_external_validity_derivation",
    }


def _timelineqa_facts(policy: Temporal01Policy, registry: Mapping[str, object], data_root: Path) -> dict[str, object]:
    source = _registry_source(registry, "timelineqa")
    if source.get("license") != "CC-BY-NC-4.0":
        raise Temporal01PreflightError("TimelineQA license is not the registered CC-BY-NC-4.0 basis")
    index_path = data_root / "timelineqa" / "index.json"
    if not index_path.is_file():
        raise Temporal01PreflightError("TimelineQA index is unavailable")
    index = _mapping(json.loads(index_path.read_text(encoding="utf-8")), "TimelineQA index")
    upstream = _mapping(source.get("upstream"), "TimelineQA upstream")
    params = _mapping(upstream.get("params"), "TimelineQA generator parameters")
    for field in ("final_year", "seed_base", "n_personas_per_density"):
        if index.get(field) != params.get(field):
            raise Temporal01PreflightError("TimelineQA index does not match the registered generator parameters")
    registry_density = _mapping(source.get("per_density"), "TimelineQA registry density counts")
    index_density = _mapping(index.get("per_density"), "TimelineQA index density counts")
    atomic_qa_count = 0
    for density in policy.timelineqa_densities:
        registered = _mapping(registry_density.get(density), "TimelineQA registered density")
        observed = _mapping(index_density.get(density), "TimelineQA observed density")
        if observed.get("personas") != registered.get("personas") or observed.get("atomic_qa") != registered.get("atomic_qa"):
            raise Temporal01PreflightError("TimelineQA density counts do not match the registry")
        atomic_qa_count += observed["atomic_qa"]
    if atomic_qa_count != index.get("total_atomic_qa"):
        raise Temporal01PreflightError("TimelineQA selected density total does not match its index")
    return {
        "license": "CC-BY-NC-4.0",
        "source_revision": _require_label(upstream.get("revision"), "TimelineQA source revision"),
        "density_count": len(policy.timelineqa_densities),
        "atomic_qa_count": atomic_qa_count,
        "payload_hash_status": "generator_seed_pinned_external_selection_manifest_required",
        "capability": "daily_life_timeline_requires_external_validity_derivation",
    }


def preflight(policy_document: object, registry_document: object, data_root: str | Path) -> Temporal01Preflight:
    """Verify external input facts without constructing an evaluation database."""
    policy = load_policy(policy_document)
    registry = _mapping(registry_document, "corpus registry")
    if canonical_sha256(registry) != policy.registry_sha256:
        raise Temporal01PreflightError("registry_sha256 does not bind the supplied corpus registry")
    root = Path(data_root)
    if not root.is_dir():
        raise Temporal01PreflightError("external corpus data root is unavailable")
    return Temporal01Preflight(
        policy=policy,
        corpora={
            "timeqa": _timeqa_facts(policy, registry, root),
            "longmemeval": _longmemeval_facts(policy, registry, root),
            "timelineqa": _timelineqa_facts(policy, registry, root),
        },
    )


def _validate_report(report: Mapping[str, object]) -> None:
    if set(report) != _REPORT_KEYS:
        raise Temporal01PreflightError("report keys do not match temporal-01 factual-preflight v1")
    if report["schema_version"] != REPORT_SCHEMA_VERSION or report["program_track"] != PROGRAM_TRACK:
        raise Temporal01PreflightError("report identity is invalid")
    _require_identifier(report["preflight_id"], "report preflight_id")
    _require_sha(report["registry_sha256"], "report registry_sha256")
    if report["state"] != "input_facts_confirmed_treatment_blocked" or report["eligible_for_execution"] is not False:
        raise Temporal01PreflightError("report execution boundary is invalid")
    if report["baseline"] != "a0_turn_fts" or report["treatment"] != "readview_valid_as_of":
        raise Temporal01PreflightError("report comparison is invalid")
    if report["claim_boundary"] != CLAIM_BOUNDARY or report["blockers"] != list(BLOCKERS) or report["no_live_execution"] is not True:
        raise Temporal01PreflightError("report claim boundary is invalid")
    corpora = _mapping(report["corpora"], "report corpora")
    if set(corpora) != {"timeqa", "longmemeval", "timelineqa"}:
        raise Temporal01PreflightError("report corpora are incomplete")


def write_report(report: Mapping[str, object], *, output_root: str | Path, report_path: str | Path) -> Path:
    """Write one safe preflight projection outside the repository."""
    _validate_report(report)
    root = Path(output_root).resolve()
    destination = Path(report_path).resolve()
    repository = Path(__file__).resolve().parents[1]
    if root.is_relative_to(repository) or destination.is_relative_to(repository):
        raise Temporal01PreflightError("preflight output must remain outside the repository")
    if not root.is_dir() or not destination.is_relative_to(root) or destination.exists() or destination.is_symlink():
        raise Temporal01PreflightError("preflight report destination must be a new file under an external root")
    destination.write_text(json.dumps(report, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
    return destination


def main(argv: list[str] | None = None) -> int:
    """Run a no-execution TEMPORAL-01 factual preflight from explicit paths."""
    parser = argparse.ArgumentParser(description="Verify TEMPORAL-01 inputs without a live measurement")
    parser.add_argument("policy", type=Path)
    parser.add_argument("--registry", type=Path, default=Path("tests/corpus/scripts/manifest.json"))
    parser.add_argument("--data-root", type=Path, default=Path("data/corpus-data/raw"))
    parser.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args(argv)
    policy = json.loads(args.policy.read_text(encoding="utf-8"))
    registry = json.loads(args.registry.read_text(encoding="utf-8"))
    result = preflight(policy, registry, args.data_root)
    path = write_report(
        result.safe_report(),
        output_root=args.output_root,
        report_path=args.output_root / f"{result.policy.preflight_id}.json",
    )
    print(json.dumps({"state": result.state, "report_sha256": _sha256_path(path)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
