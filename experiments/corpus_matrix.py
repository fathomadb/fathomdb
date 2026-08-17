"""Fail-closed CORPUS-01 portfolio and external-only human-gold contracts.

This module validates committed metadata only.  It never opens a corpus payload,
downloads a source, creates gold answers, or writes an external artifact.
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from typing import Mapping


MATRIX_SCHEMA_VERSION = "corpus-01-matrix.v1"
PROTOCOL_SCHEMA_VERSION = "corpus-01-human-gold-protocol.v1"
GOLD_MANIFEST_SCHEMA_VERSION = "corpus-01-human-gold-manifest.v1"
PROGRAM_TRACK = "CORPUS-01"

_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_IDENTIFIER = re.compile(r"^[a-z][a-z0-9_-]{1,63}$")
_SAFE_LABEL = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:+/-]{0,127}$")
_PAYLOAD_STATES = {"external_payload_not_verified", "not_acquired"}
_JUDGMENTS = {"supported", "contradicted", "insufficient_evidence"}
_ADJUDICATIONS = {"not_required", "adjudicated"}
_REQUIRED_CATEGORIES = (
    "knowledge_update",
    "supersession",
    "source_erasure",
    "time_scoped_validity",
)
_EXPECTED_CORPORA = (
    "locomo",
    "longmemeval",
    "timelineqa",
    "timeqa",
    "test_of_time",
    "ir_c",
    "beir",
    "musique",
    "summhay",
    "apnews_benchmarkqed",
)
_MATRIX_KEYS = {"schema_version", "program_track", "matrix_id", "corpora", "human_gold_categories"}
_ENTRY_KEYS = {
    "corpus_id", "portfolio_family", "payload_state", "payload_rule", "license", "license_basis",
    "redistribution_rule", "factual_prerequisites", "supported_categories", "unsupported_categories",
    "metrics", "power", "supported_claims", "unsupported_claims",
}
_POWER_KEYS = {"rule", "class_counts"}
_CLASS_COUNT_KEYS = {"class", "count", "count_status"}
_PROTOCOL_KEYS = {
    "schema_version", "program_track", "protocol_id", "matrix_sha256", "gold_manifest_schema",
    "required_categories", "required_reviewer_count", "allowed_judgments", "allowed_adjudications",
    "prohibited_fields", "external_artifact_rule", "workflow",
}
_MANIFEST_KEYS = {
    "schema_version", "artifact_id", "scope", "matrix_sha256", "protocol_sha256", "payload_root_sha256", "records",
}
_RECORD_KEYS = {
    "annotation_id", "corpus_id", "category", "source_locator_sha256", "evidence_locator_sha256",
    "judgment", "reviewer_hashes", "adjudication",
}


class CorpusMatrixError(ValueError):
    """Raised when CORPUS-01 metadata could misrepresent gold or payload facts."""


@dataclass(frozen=True)
class CorpusEntry:
    """One corpus's bounded claims, payload posture, and evaluable categories."""

    corpus_id: str
    payload_state: str
    supported_categories: tuple[str, ...]
    unsupported_categories: tuple[str, ...]
    metrics: tuple[str, ...]


@dataclass(frozen=True)
class CorpusMatrix:
    """Validated, versioned CORPUS-01 portfolio metadata."""

    matrix_id: str
    corpus_ids: tuple[str, ...]
    human_gold_categories: tuple[str, ...]
    entries: Mapping[str, CorpusEntry]

    def entry(self, corpus_id: str) -> CorpusEntry:
        """Return a validated corpus entry by its stable identifier."""
        return self.entries[corpus_id]


@dataclass(frozen=True)
class HumanGoldProtocol:
    """Validated external-only, human-review protocol metadata."""

    protocol_id: str
    matrix_sha256: str
    protocol_sha256: str
    required_reviewer_count: int
    prohibited_fields: tuple[str, ...]
    corpus_ids: tuple[str, ...]


@dataclass(frozen=True)
class ExternalGoldManifest:
    """A content-free external human-gold manifest suitable for later audit."""

    artifact_id: str
    record_count: int
    categories: tuple[str, ...]
    corpus_ids: tuple[str, ...]


def canonical_sha256(value: object) -> str:
    """Hash canonical JSON so all bindings are reproducible and content-free."""
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    ).hexdigest()


def _require_sha(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise CorpusMatrixError(f"{label} must be a lowercase sha256")
    return value


def _require_identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or _IDENTIFIER.fullmatch(value) is None:
        raise CorpusMatrixError(f"{label} must be a stable identifier")
    return value


def _require_label(value: object, label: str) -> str:
    if not isinstance(value, str) or _SAFE_LABEL.fullmatch(value) is None:
        raise CorpusMatrixError(f"{label} must be a safe label")
    return value


def _require_unique_identifiers(value: object, label: str, *, allow_empty: bool = False) -> tuple[str, ...]:
    if not isinstance(value, list) or (not value and not allow_empty):
        raise CorpusMatrixError(f"{label} must be a {'list' if allow_empty else 'non-empty list'}")
    values = tuple(_require_identifier(item, label) for item in value)
    if len(set(values)) != len(values):
        raise CorpusMatrixError(f"{label} must be unique")
    return values


def _require_text_list(value: object, label: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not value:
        raise CorpusMatrixError(f"{label} must be a non-empty list")
    values = tuple(_require_label(item, label) for item in value)
    if len(set(values)) != len(values):
        raise CorpusMatrixError(f"{label} must be unique")
    return values


def _validate_power(value: object) -> None:
    if not isinstance(value, dict) or set(value) != _POWER_KEYS:
        raise CorpusMatrixError("power keys do not match corpus-01-matrix.v1")
    _require_label(value["rule"], "power rule")
    counts = value["class_counts"]
    if not isinstance(counts, list) or not counts:
        raise CorpusMatrixError("power class_counts must be a non-empty list")
    seen: set[str] = set()
    for row in counts:
        if not isinstance(row, dict) or set(row) != _CLASS_COUNT_KEYS:
            raise CorpusMatrixError("power class count keys do not match corpus-01-matrix.v1")
        name = _require_identifier(row["class"], "power class")
        if name in seen:
            raise CorpusMatrixError("power class_counts must be unique")
        seen.add(name)
        count = row["count"]
        if count is not None and (not isinstance(count, int) or isinstance(count, bool) or count < 0):
            raise CorpusMatrixError("power class count must be a non-negative integer or null")
        if row["count_status"] not in {"registered", "historical", "not_verified", "not_acquired"}:
            raise CorpusMatrixError("power class count_status is invalid")
        if count is None and row["count_status"] not in {"not_verified", "not_acquired"}:
            raise CorpusMatrixError("unknown power class count must be explicitly unverified")


def validate_matrix(document: object) -> CorpusMatrix:
    """Validate the committed portfolio without assuming an external payload exists."""
    if not isinstance(document, dict) or set(document) != _MATRIX_KEYS:
        raise CorpusMatrixError("matrix keys do not match corpus-01-matrix.v1")
    if document["schema_version"] != MATRIX_SCHEMA_VERSION:
        raise CorpusMatrixError("matrix schema_version must be corpus-01-matrix.v1")
    if document["program_track"] != PROGRAM_TRACK:
        raise CorpusMatrixError("matrix program_track must be CORPUS-01")
    matrix_id = _require_identifier(document["matrix_id"], "matrix_id")
    categories = _require_unique_identifiers(document["human_gold_categories"], "human_gold_categories")
    if categories != _REQUIRED_CATEGORIES:
        raise CorpusMatrixError("human_gold_categories must use the complete canonical lifecycle set")
    rows = document["corpora"]
    if not isinstance(rows, list):
        raise CorpusMatrixError("corpora must be a list")
    entries: dict[str, CorpusEntry] = {}
    for row in rows:
        if not isinstance(row, dict) or set(row) != _ENTRY_KEYS:
            raise CorpusMatrixError("corpus entry keys do not match corpus-01-matrix.v1")
        corpus_id = _require_identifier(row["corpus_id"], "corpus_id")
        if corpus_id in entries:
            raise CorpusMatrixError("corpus_id must be unique")
        _require_label(row["portfolio_family"], "portfolio_family")
        payload_state = row["payload_state"]
        if payload_state not in _PAYLOAD_STATES:
            raise CorpusMatrixError("payload_state is invalid")
        _require_label(row["payload_rule"], "payload_rule")
        _require_label(row["license"], "license")
        if row["license_basis"] not in {"manifest_json", "acquire_script", "research_note", "historical_register"}:
            raise CorpusMatrixError("license_basis is not a verified local factual basis")
        _require_label(row["redistribution_rule"], "redistribution_rule")
        _require_text_list(row["factual_prerequisites"], "factual_prerequisites")
        supported = _require_unique_identifiers(
            row["supported_categories"], "supported_categories", allow_empty=True
        )
        unsupported = _require_unique_identifiers(row["unsupported_categories"], "unsupported_categories")
        if set(supported) | set(unsupported) != set(_REQUIRED_CATEGORIES) or set(supported) & set(unsupported):
            raise CorpusMatrixError("each lifecycle category must be explicitly supported or unsupported")
        metrics = _require_unique_identifiers(row["metrics"], "metrics")
        _validate_power(row["power"])
        _require_text_list(row["supported_claims"], "supported_claims")
        _require_text_list(row["unsupported_claims"], "unsupported_claims")
        entries[corpus_id] = CorpusEntry(corpus_id, payload_state, supported, unsupported, metrics)
    corpus_ids = tuple(entries)
    if corpus_ids != _EXPECTED_CORPORA:
        raise CorpusMatrixError("corpora must use the complete canonical CORPUS-01 portfolio order")
    return CorpusMatrix(matrix_id, corpus_ids, categories, entries)


def validate_human_gold_protocol(document: object, matrix_document: object) -> HumanGoldProtocol:
    """Validate the no-answer, external-only protocol against one exact matrix."""
    matrix = validate_matrix(matrix_document)
    if not isinstance(document, dict) or set(document) != _PROTOCOL_KEYS:
        raise CorpusMatrixError("protocol keys do not match corpus-01-human-gold-protocol.v1")
    if document["schema_version"] != PROTOCOL_SCHEMA_VERSION:
        raise CorpusMatrixError("protocol schema_version must be corpus-01-human-gold-protocol.v1")
    if document["program_track"] != PROGRAM_TRACK:
        raise CorpusMatrixError("protocol program_track must be CORPUS-01")
    protocol_id = _require_identifier(document["protocol_id"], "protocol_id")
    matrix_sha = _require_sha(document["matrix_sha256"], "matrix_sha256")
    if matrix_sha != canonical_sha256(matrix_document):
        raise CorpusMatrixError("protocol matrix_sha256 does not bind the supplied matrix")
    if document["gold_manifest_schema"] != GOLD_MANIFEST_SCHEMA_VERSION:
        raise CorpusMatrixError("protocol gold_manifest_schema is invalid")
    categories = _require_unique_identifiers(document["required_categories"], "required_categories")
    if categories != _REQUIRED_CATEGORIES:
        raise CorpusMatrixError("protocol required_categories must use the complete canonical lifecycle set")
    if document["required_reviewer_count"] != 2:
        raise CorpusMatrixError("protocol required_reviewer_count must be two")
    if tuple(document["allowed_judgments"]) != tuple(sorted(_JUDGMENTS)):
        raise CorpusMatrixError("protocol allowed_judgments must be canonical")
    if tuple(document["allowed_adjudications"]) != tuple(sorted(_ADJUDICATIONS)):
        raise CorpusMatrixError("protocol allowed_adjudications must be canonical")
    prohibited = _require_unique_identifiers(document["prohibited_fields"], "prohibited_fields")
    expected_prohibited = ("answer", "answer_text", "evidence_text", "question", "raw_payload", "verbatim_quote")
    if prohibited != expected_prohibited:
        raise CorpusMatrixError("protocol prohibited_fields must use the complete canonical no-payload set")
    if document["external_artifact_rule"] != "external_only_content_free_manifest_in_repo":
        raise CorpusMatrixError("protocol external_artifact_rule is invalid")
    workflow = _require_unique_identifiers(document["workflow"], "workflow")
    if workflow != ("sample_external_records", "blind_double_review", "adjudicate_disagreement", "export_opaque_manifest"):
        raise CorpusMatrixError("protocol workflow is incomplete or non-canonical")
    return HumanGoldProtocol(
        protocol_id,
        matrix_sha,
        canonical_sha256(document),
        document["required_reviewer_count"],
        prohibited,
        matrix.corpus_ids,
    )


def validate_external_gold_manifest(document: object, protocol: HumanGoldProtocol) -> ExternalGoldManifest:
    """Validate a later external manifest without admitting raw Q/A or payload text."""
    if not isinstance(document, dict) or set(document) != _MANIFEST_KEYS:
        raise CorpusMatrixError("gold manifest keys do not match corpus-01-human-gold-manifest.v1")
    if document["schema_version"] != GOLD_MANIFEST_SCHEMA_VERSION:
        raise CorpusMatrixError("gold manifest schema_version is invalid")
    artifact_id = _require_identifier(document["artifact_id"], "artifact_id")
    if document["scope"] not in {"pilot", "portfolio_complete"}:
        raise CorpusMatrixError("gold manifest scope is invalid")
    if _require_sha(document["matrix_sha256"], "gold manifest matrix_sha256") != protocol.matrix_sha256:
        raise CorpusMatrixError("gold manifest matrix_sha256 does not match protocol")
    if _require_sha(document["protocol_sha256"], "gold manifest protocol_sha256") != protocol.protocol_sha256:
        raise CorpusMatrixError("gold manifest protocol_sha256 does not match protocol")
    _require_sha(document["payload_root_sha256"], "gold manifest payload_root_sha256")
    rows = document["records"]
    if not isinstance(rows, list) or not rows:
        raise CorpusMatrixError("gold manifest records must be a non-empty list")
    categories: list[str] = []
    corpus_ids: list[str] = []
    seen_ids: set[str] = set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != _RECORD_KEYS:
            raise CorpusMatrixError("gold record keys do not match corpus-01-human-gold-manifest.v1")
        annotation_id = _require_identifier(row["annotation_id"], "annotation_id")
        if annotation_id in seen_ids:
            raise CorpusMatrixError("gold annotation_id must be unique")
        seen_ids.add(annotation_id)
        corpus_id = _require_identifier(row["corpus_id"], "gold corpus_id")
        if corpus_id not in protocol.corpus_ids:
            raise CorpusMatrixError("gold corpus_id must be a known matrix corpus")
        corpus_ids.append(corpus_id)
        category = _require_identifier(row["category"], "gold category")
        if category not in _REQUIRED_CATEGORIES:
            raise CorpusMatrixError("gold category is not a CORPUS-01 lifecycle category")
        categories.append(category)
        _require_sha(row["source_locator_sha256"], "gold source_locator_sha256")
        _require_sha(row["evidence_locator_sha256"], "gold evidence_locator_sha256")
        if row["judgment"] not in _JUDGMENTS:
            raise CorpusMatrixError("gold judgment is invalid")
        reviewers = row["reviewer_hashes"]
        if not isinstance(reviewers, list) or len(reviewers) != protocol.required_reviewer_count:
            raise CorpusMatrixError("gold reviewer_hashes must contain the required independent reviewers")
        if len(set(_require_sha(item, "gold reviewer_hash") for item in reviewers)) != len(reviewers):
            raise CorpusMatrixError("gold reviewer_hashes must be unique")
        if row["adjudication"] not in _ADJUDICATIONS:
            raise CorpusMatrixError("gold adjudication is invalid")
    found_categories = tuple(sorted(set(categories)))
    if document["scope"] == "portfolio_complete" and set(found_categories) != set(_REQUIRED_CATEGORIES):
        raise CorpusMatrixError("complete gold manifest must cover every lifecycle category")
    return ExternalGoldManifest(artifact_id, len(rows), found_categories, tuple(sorted(set(corpus_ids))))
