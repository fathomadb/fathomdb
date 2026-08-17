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
QUALIFIED_PROTOCOL_SCHEMA_VERSION = "corpus-01-human-gold-protocol.v2"
QUALIFIED_GOLD_MANIFEST_SCHEMA_VERSION = "corpus-01-human-gold-manifest.v2"
HUMAN_GOLD_AMENDMENT_SCHEMA_VERSION = "corpus-01-human-gold-amendment.v1"
PROGRAM_TRACK = "CORPUS-01"

_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_IDENTIFIER = re.compile(r"^[a-z][a-z0-9_-]{1,63}$")
_SAFE_LABEL = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:+/-]{0,127}$")
_LEDGER_SEQUENCE = re.compile(r"^seq-[1-9][0-9]*$")
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
_QUALIFIED_PROTOCOL_KEYS = {
    "schema_version", "program_track", "protocol_id", "matrix_sha256", "qualified_manifest_schema",
    "amendment_schema", "required_categories", "required_reviewer_count", "allowed_judgments",
    "allowed_adjudications", "prohibited_fields", "external_artifact_rule", "workflow",
}
_QUALIFIED_MANIFEST_KEYS = {
    "schema_version", "artifact_id", "scope", "matrix_sha256", "protocol_sha256", "preflights", "records",
}
_PREFLIGHT_KEYS = {
    "corpus_id", "category", "source_payload_sha256", "license_copy_sha256", "source_revision",
    "selected_class_counts", "exclusions_sha256", "metric", "paired_power_sha256", "claim_id", "claim_sha256",
}
_SELECTED_CLASS_KEYS = {"class", "count"}
_AMENDMENT_KEYS = {
    "schema_version", "amendment_id", "matrix_sha256", "qualified_manifest_sha256", "corpus_id",
    "category", "approval_ref", "eligibility",
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
    power_rows: tuple[Mapping[str, object], ...]


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


@dataclass(frozen=True)
class QualifiedHumanGoldManifest:
    """A preflight-bound, content-free external human-gold manifest."""

    manifest_sha256: str
    corpus_categories: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class HumanGoldAmendment:
    """A versioned approval binding a qualified human-gold pair to eligibility."""

    corpus_id: str
    category: str
    qualified_manifest_sha256: str


@dataclass(frozen=True)
class EvaluationEligibility:
    """The only allowed evidence route for a requested corpus/category pair."""

    corpus_id: str
    category: str
    evidence_mode: str


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


def _require_approval_ref(value: object) -> str:
    if not isinstance(value, str) or _LEDGER_SEQUENCE.fullmatch(value) is None:
        raise CorpusMatrixError("human-gold approval_ref must name a steward ledger sequence")
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
        power_rows = tuple(dict(item) for item in row["power"]["class_counts"])
        entries[corpus_id] = CorpusEntry(
            corpus_id, payload_state, supported, unsupported, metrics, power_rows
        )
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


def _matrix_entry(matrix: CorpusMatrix, corpus_id: object, category: object) -> tuple[CorpusEntry, str]:
    """Resolve one exact corpus/category pair or reject it before any claim binds."""
    resolved_corpus = _require_identifier(corpus_id, "corpus_id")
    resolved_category = _require_identifier(category, "category")
    if resolved_corpus not in matrix.entries:
        raise CorpusMatrixError("corpus_id must be a known matrix corpus")
    if resolved_category not in _REQUIRED_CATEGORIES:
        raise CorpusMatrixError("category must be a CORPUS-01 lifecycle category")
    return matrix.entry(resolved_corpus), resolved_category


def _validate_qualified_protocol(document: object, matrix_document: object) -> str:
    """Validate the versioned protocol that gates qualification, not just planning."""
    validate_matrix(matrix_document)
    if not isinstance(document, dict) or set(document) != _QUALIFIED_PROTOCOL_KEYS:
        raise CorpusMatrixError("qualified protocol keys do not match corpus-01-human-gold-protocol.v2")
    if document["schema_version"] != QUALIFIED_PROTOCOL_SCHEMA_VERSION:
        raise CorpusMatrixError("qualified protocol schema_version is invalid")
    if document["program_track"] != PROGRAM_TRACK:
        raise CorpusMatrixError("qualified protocol program_track must be CORPUS-01")
    _require_identifier(document["protocol_id"], "qualified protocol_id")
    if _require_sha(document["matrix_sha256"], "qualified protocol matrix_sha256") != canonical_sha256(matrix_document):
        raise CorpusMatrixError("qualified protocol matrix_sha256 does not bind the supplied matrix")
    if document["qualified_manifest_schema"] != QUALIFIED_GOLD_MANIFEST_SCHEMA_VERSION:
        raise CorpusMatrixError("qualified protocol manifest schema is invalid")
    if document["amendment_schema"] != HUMAN_GOLD_AMENDMENT_SCHEMA_VERSION:
        raise CorpusMatrixError("qualified protocol amendment schema is invalid")
    if _require_unique_identifiers(document["required_categories"], "qualified required_categories") != _REQUIRED_CATEGORIES:
        raise CorpusMatrixError("qualified protocol categories are incomplete")
    if document["required_reviewer_count"] != 2:
        raise CorpusMatrixError("qualified protocol requires two reviewers")
    if tuple(document["allowed_judgments"]) != tuple(sorted(_JUDGMENTS)):
        raise CorpusMatrixError("qualified protocol judgments are invalid")
    if tuple(document["allowed_adjudications"]) != tuple(sorted(_ADJUDICATIONS)):
        raise CorpusMatrixError("qualified protocol adjudications are invalid")
    if _require_unique_identifiers(document["prohibited_fields"], "qualified prohibited_fields") != (
        "answer", "answer_text", "evidence_text", "question", "raw_payload", "verbatim_quote"
    ):
        raise CorpusMatrixError("qualified protocol prohibited_fields are incomplete")
    if document["external_artifact_rule"] != "external_only_content_free_manifest_in_repo":
        raise CorpusMatrixError("qualified protocol external_artifact_rule is invalid")
    if _require_unique_identifiers(document["workflow"], "qualified workflow") != (
        "factual_preflight", "sample_external_records", "blind_double_review", "adjudicate_disagreement",
        "export_opaque_manifest",
    ):
        raise CorpusMatrixError("qualified protocol workflow is incomplete")
    return canonical_sha256(document)


def _validate_preflight(row: object, matrix: CorpusMatrix) -> tuple[str, str]:
    """Bind one selected corpus/category to all non-content factual prerequisites."""
    if not isinstance(row, dict) or set(row) != _PREFLIGHT_KEYS:
        raise CorpusMatrixError("preflight keys do not match corpus-01-human-gold-manifest.v2")
    entry, category = _matrix_entry(matrix, row["corpus_id"], row["category"])
    for field in ("source_payload_sha256", "license_copy_sha256", "exclusions_sha256", "paired_power_sha256", "claim_sha256"):
        _require_sha(row[field], f"preflight {field}")
    _require_label(row["source_revision"], "preflight source_revision")
    if row["metric"] not in entry.metrics:
        raise CorpusMatrixError("preflight metric is not supported by the selected corpus")
    _require_identifier(row["claim_id"], "preflight claim_id")
    selected = row["selected_class_counts"]
    if not isinstance(selected, list) or not selected:
        raise CorpusMatrixError("preflight selected_class_counts must be a non-empty list")
    declared_counts = {
        item["class"]: item["count"]
        for item in matrix_document_power_rows(matrix, entry.corpus_id)
    }
    seen: set[str] = set()
    for item in selected:
        if not isinstance(item, dict) or set(item) != _SELECTED_CLASS_KEYS:
            raise CorpusMatrixError("preflight selected class keys are invalid")
        class_name = _require_identifier(item["class"], "preflight selected class")
        if class_name in seen or class_name not in declared_counts:
            raise CorpusMatrixError("preflight selected class must be declared exactly once")
        seen.add(class_name)
        count = item["count"]
        if not isinstance(count, int) or isinstance(count, bool) or count <= 0:
            raise CorpusMatrixError("preflight selected class count must be positive")
        declared = declared_counts[class_name]
        if declared is not None and count > declared:
            raise CorpusMatrixError("preflight selected class count exceeds the matrix count")
    return entry.corpus_id, category


def matrix_document_power_rows(matrix: CorpusMatrix, corpus_id: str) -> tuple[Mapping[str, object], ...]:
    """Return retained power rows for a validated corpus without reopening the JSON document."""
    entry = matrix.entry(corpus_id)
    return entry.power_rows


def validate_qualified_human_gold_manifest(
    document: object, matrix_document: object, protocol_document: object | None = None
) -> QualifiedHumanGoldManifest:
    """Validate preflight-bound human evidence; reject every missing factual binding."""
    matrix = validate_matrix(matrix_document)
    if not isinstance(document, dict) or set(document) != _QUALIFIED_MANIFEST_KEYS:
        raise CorpusMatrixError("qualified manifest keys do not match corpus-01-human-gold-manifest.v2")
    if document["schema_version"] != QUALIFIED_GOLD_MANIFEST_SCHEMA_VERSION:
        raise CorpusMatrixError("qualified manifest schema_version is invalid")
    _require_identifier(document["artifact_id"], "qualified artifact_id")
    if document["scope"] not in {"pilot", "portfolio_complete"}:
        raise CorpusMatrixError("qualified manifest scope is invalid")
    if _require_sha(document["matrix_sha256"], "qualified matrix_sha256") != canonical_sha256(matrix_document):
        raise CorpusMatrixError("qualified manifest matrix_sha256 does not match matrix")
    preflights = document["preflights"]
    if not isinstance(preflights, list) or not preflights:
        raise CorpusMatrixError("preflights must be a non-empty list")
    pairs = tuple(_validate_preflight(row, matrix) for row in preflights)
    if len(set(pairs)) != len(pairs):
        raise CorpusMatrixError("preflights must contain each corpus/category pair once")
    if protocol_document is None:
        raise CorpusMatrixError("qualified manifest requires a bound qualified protocol")
    protocol_sha = _validate_qualified_protocol(protocol_document, matrix_document)
    if _require_sha(document["protocol_sha256"], "qualified protocol_sha256") != protocol_sha:
        raise CorpusMatrixError("qualified manifest protocol_sha256 does not match protocol")
    records = document["records"]
    if not isinstance(records, list) or not records:
        raise CorpusMatrixError("qualified manifest records must be a non-empty list")
    seen_annotations: set[str] = set()
    covered_pairs: set[tuple[str, str]] = set()
    for row in records:
        if not isinstance(row, dict) or set(row) != _RECORD_KEYS:
            raise CorpusMatrixError("qualified record keys do not match corpus-01-human-gold-manifest.v2")
        annotation_id = _require_identifier(row["annotation_id"], "qualified annotation_id")
        if annotation_id in seen_annotations:
            raise CorpusMatrixError("qualified annotation_id must be unique")
        seen_annotations.add(annotation_id)
        entry, category = _matrix_entry(matrix, row["corpus_id"], row["category"])
        pair = (entry.corpus_id, category)
        if pair not in pairs:
            raise CorpusMatrixError("qualified record has no matching factual preflight")
        covered_pairs.add(pair)
        _require_sha(row["source_locator_sha256"], "qualified source_locator_sha256")
        _require_sha(row["evidence_locator_sha256"], "qualified evidence_locator_sha256")
        if row["judgment"] not in _JUDGMENTS:
            raise CorpusMatrixError("qualified judgment is invalid")
        reviewers = row["reviewer_hashes"]
        if not isinstance(reviewers, list) or len(reviewers) != 2:
            raise CorpusMatrixError("qualified reviewer_hashes must contain two independent reviewers")
        if len(set(_require_sha(item, "qualified reviewer_hash") for item in reviewers)) != len(reviewers):
            raise CorpusMatrixError("qualified reviewer_hashes must be unique")
        if row["adjudication"] not in _ADJUDICATIONS:
            raise CorpusMatrixError("qualified adjudication is invalid")
    if set(pairs) != covered_pairs:
        raise CorpusMatrixError("each factual preflight must have qualified evidence")
    categories = {category for _, category in pairs}
    if document["scope"] == "portfolio_complete" and categories != set(_REQUIRED_CATEGORIES):
        raise CorpusMatrixError("complete qualified manifest must cover every lifecycle category")
    return QualifiedHumanGoldManifest(canonical_sha256(document), tuple(sorted(pairs)))


def validate_human_gold_amendment(
    document: object, matrix_document: object, qualified_manifest: QualifiedHumanGoldManifest
) -> HumanGoldAmendment:
    """Validate the required versioned approval before human gold changes eligibility."""
    matrix = validate_matrix(matrix_document)
    if not isinstance(document, dict) or set(document) != _AMENDMENT_KEYS:
        raise CorpusMatrixError("human-gold amendment keys are invalid")
    if document["schema_version"] != HUMAN_GOLD_AMENDMENT_SCHEMA_VERSION:
        raise CorpusMatrixError("human-gold amendment schema_version is invalid")
    _require_identifier(document["amendment_id"], "human-gold amendment_id")
    if _require_sha(document["matrix_sha256"], "human-gold amendment matrix_sha256") != canonical_sha256(matrix_document):
        raise CorpusMatrixError("human-gold amendment matrix_sha256 does not match matrix")
    if _require_sha(document["qualified_manifest_sha256"], "human-gold amendment manifest_sha256") != qualified_manifest.manifest_sha256:
        raise CorpusMatrixError("human-gold amendment manifest_sha256 does not match qualified evidence")
    entry, category = _matrix_entry(matrix, document["corpus_id"], document["category"])
    if (entry.corpus_id, category) not in qualified_manifest.corpus_categories:
        raise CorpusMatrixError("human-gold amendment pair lacks qualified evidence")
    _require_approval_ref(document["approval_ref"])
    if document["eligibility"] != "approved_human_gold":
        raise CorpusMatrixError("human-gold amendment eligibility must be approved_human_gold")
    return HumanGoldAmendment(entry.corpus_id, category, qualified_manifest.manifest_sha256)


def validate_evaluation_eligibility(
    matrix_document: object, *, corpus_id: str, category: str, amendment: HumanGoldAmendment | None = None
) -> EvaluationEligibility:
    """Select native evidence or a separately approved qualified-human-gold route."""
    matrix = validate_matrix(matrix_document)
    entry, resolved_category = _matrix_entry(matrix, corpus_id, category)
    if resolved_category in entry.supported_categories:
        return EvaluationEligibility(entry.corpus_id, resolved_category, "native_corpus")
    if amendment is None or (amendment.corpus_id, amendment.category) != (entry.corpus_id, resolved_category):
        raise CorpusMatrixError("corpus/category is unsupported without approved human-gold amendment")
    return EvaluationEligibility(entry.corpus_id, resolved_category, "qualified_human_gold")


def validate_portfolio_coverage(
    matrix_document: object, *, qualified_evidence: tuple[QualifiedHumanGoldManifest, ...]
) -> tuple[str, ...]:
    """Require factual, qualified evidence for every lifecycle category before a broad claim."""
    validate_matrix(matrix_document)
    categories = {
        category
        for manifest in qualified_evidence
        for _, category in manifest.corpus_categories
    }
    if categories != set(_REQUIRED_CATEGORIES):
        raise CorpusMatrixError("insufficient portfolio/category evidence for a broad agent-memory claim")
    return tuple(sorted(categories))
