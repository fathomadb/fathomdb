"""Fail-closed contracts for authorized external CORPUS-01 LLM reviews.

The module deliberately keeps LLM judgments separate from human gold.  It
validates only opaque, content-free evidence and never opens payloads or
transcripts.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from experiments import corpus_matrix


LLM_REVIEW_PROTOCOL_V1 = "corpus-01-llm-review-protocol.v1"
LLM_REVIEW_MANIFEST_V1 = "corpus-01-llm-review-manifest.v1"
PROGRAM_TRACK = "CORPUS-01"
_JUDGMENTS = ("contradicted", "insufficient_evidence", "supported")
_ADJUDICATIONS = ("adjudicated", "not_required")
_REQUIRED_CATEGORIES = (
    "knowledge_update",
    "supersession",
    "source_erasure",
    "time_scoped_validity",
)
_PROHIBITED_FIELDS = (
    "answer",
    "answer_text",
    "evidence_text",
    "question",
    "raw_payload",
    "verbatim_quote",
)
_PROTOCOL_KEYS = {
    "schema_version",
    "program_track",
    "protocol_id",
    "matrix_sha256",
    "manifest_schema",
    "required_categories",
    "required_reviewer_count",
    "reviewer_kind",
    "allowed_judgments",
    "allowed_adjudications",
    "prohibited_fields",
    "external_artifact_rule",
    "max_total_cost_usd",
    "workflow",
}
_MANIFEST_KEYS = {
    "schema_version",
    "artifact_id",
    "scope",
    "matrix_sha256",
    "protocol_sha256",
    "source_payload_sha256",
    "license_copy_sha256",
    "source_revision",
    "workset_sha256",
    "reviewer_runs",
    "records",
    "total_cost_usd",
    "conclusion",
}
_REVIEWER_RUN_KEYS = {
    "reviewer_id_sha256",
    "provider",
    "model",
    "prompt_sha256",
    "transcript_sha256",
    "run_receipt_sha256",
    "blinded_assignment_sha256",
}
_RECORD_KEYS = {
    "annotation_id",
    "corpus_id",
    "category",
    "source_locator_sha256",
    "evidence_locator_sha256",
    "reviewer_judgments",
    "judgment",
    "adjudication",
}


class LlmReviewError(ValueError):
    """Raised when an LLM-review artifact could overstate its evidence."""


@dataclass(frozen=True)
class LlmReviewProtocol:
    """Validated policy binding for one independent external LLM review."""

    protocol_id: str
    matrix_sha256: str
    protocol_sha256: str
    reviewer_count: int
    max_total_cost_usd: float


@dataclass(frozen=True)
class LlmReviewManifest:
    """Validated, content-free summary of an independent LLM-review run."""

    manifest_sha256: str
    categories: tuple[str, ...]
    record_count: int
    evidence_mode: str = "qualified_llm_review"


def _identifier(value: object, label: str) -> str:
    try:
        return corpus_matrix._require_identifier(value, label)
    except corpus_matrix.CorpusMatrixError as exc:
        raise LlmReviewError(str(exc)) from exc


def _label(value: object, label: str) -> str:
    try:
        return corpus_matrix._require_label(value, label)
    except corpus_matrix.CorpusMatrixError as exc:
        raise LlmReviewError(str(exc)) from exc


def _sha(value: object, label: str) -> str:
    try:
        return corpus_matrix._require_sha(value, label)
    except corpus_matrix.CorpusMatrixError as exc:
        raise LlmReviewError(str(exc)) from exc


def _unique_identifiers(value: object, label: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not value:
        raise LlmReviewError(f"{label} must be a non-empty list")
    values = tuple(_identifier(item, label) for item in value)
    if len(set(values)) != len(values):
        raise LlmReviewError(f"{label} must be unique")
    return values


def _cost(value: object, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (float, int)):
        raise LlmReviewError(f"{label} must be a finite non-negative number")
    resolved = float(value)
    if not math.isfinite(resolved) or resolved < 0:
        raise LlmReviewError(f"{label} must be a finite non-negative number")
    return resolved


def validate_protocol(document: object, matrix_document: object) -> LlmReviewProtocol:
    """Validate a protocol without treating LLM review as human gold."""
    corpus_matrix.validate_matrix(matrix_document)
    if not isinstance(document, dict) or set(document) != _PROTOCOL_KEYS:
        raise LlmReviewError("protocol keys do not match corpus-01-llm-review-protocol.v1")
    if document["schema_version"] != LLM_REVIEW_PROTOCOL_V1:
        raise LlmReviewError("protocol schema_version is invalid")
    if document["program_track"] != PROGRAM_TRACK:
        raise LlmReviewError("protocol program_track is invalid")
    protocol_id = _identifier(document["protocol_id"], "protocol_id")
    matrix_sha256 = _sha(document["matrix_sha256"], "matrix_sha256")
    if matrix_sha256 != corpus_matrix.canonical_sha256(matrix_document):
        raise LlmReviewError("protocol matrix_sha256 does not bind the supplied matrix")
    if document["manifest_schema"] != LLM_REVIEW_MANIFEST_V1:
        raise LlmReviewError("protocol manifest_schema is invalid")
    if _unique_identifiers(document["required_categories"], "required_categories") != _REQUIRED_CATEGORIES:
        raise LlmReviewError("protocol required_categories are invalid")
    if document["required_reviewer_count"] not in {2, 3}:
        raise LlmReviewError("protocol required_reviewer_count must be two or three")
    if document["reviewer_kind"] != "independent_llm":
        raise LlmReviewError("protocol reviewer_kind must be independent_llm")
    if tuple(document["allowed_judgments"]) != _JUDGMENTS:
        raise LlmReviewError("protocol allowed_judgments are invalid")
    if tuple(document["allowed_adjudications"]) != _ADJUDICATIONS:
        raise LlmReviewError("protocol allowed_adjudications are invalid")
    if tuple(document["prohibited_fields"]) != _PROHIBITED_FIELDS:
        raise LlmReviewError("protocol prohibited_fields are invalid")
    if document["external_artifact_rule"] != "external_only_content_free_manifest_in_repo":
        raise LlmReviewError("protocol external_artifact_rule is invalid")
    max_cost = _cost(document["max_total_cost_usd"], "max_total_cost_usd")
    if max_cost != 20.0:
        raise LlmReviewError("protocol max_total_cost_usd must equal the HITL cap")
    if _unique_identifiers(document["workflow"], "workflow") != (
        "factual_preflight",
        "sample_external_records",
        "blind_independent_llm_review",
        "adjudicate_disagreement",
        "export_opaque_manifest",
    ):
        raise LlmReviewError("protocol workflow is invalid")
    return LlmReviewProtocol(
        protocol_id=protocol_id,
        matrix_sha256=matrix_sha256,
        protocol_sha256=corpus_matrix.canonical_sha256(document),
        reviewer_count=document["required_reviewer_count"],
        max_total_cost_usd=max_cost,
    )


def validate_manifest(
    document: object, matrix_document: object, protocol_document: object
) -> LlmReviewManifest:
    """Validate opaque independent LLM judgments and their cost/audit bindings."""
    matrix = corpus_matrix.validate_matrix(matrix_document)
    protocol = validate_protocol(protocol_document, matrix_document)
    if not isinstance(document, dict) or set(document) != _MANIFEST_KEYS:
        raise LlmReviewError("manifest keys do not match corpus-01-llm-review-manifest.v1")
    if document["schema_version"] != LLM_REVIEW_MANIFEST_V1:
        raise LlmReviewError("manifest schema_version is invalid")
    _identifier(document["artifact_id"], "artifact_id")
    if document["scope"] not in {"partial_portfolio", "portfolio_complete"}:
        raise LlmReviewError("manifest scope is invalid")
    if _sha(document["matrix_sha256"], "matrix_sha256") != protocol.matrix_sha256:
        raise LlmReviewError("manifest matrix_sha256 does not match protocol")
    if _sha(document["protocol_sha256"], "protocol_sha256") != protocol.protocol_sha256:
        raise LlmReviewError("manifest protocol_sha256 does not match protocol")
    for field in ("source_payload_sha256", "license_copy_sha256", "workset_sha256"):
        _sha(document[field], field)
    _label(document["source_revision"], "source_revision")
    if _cost(document["total_cost_usd"], "total_cost_usd") > protocol.max_total_cost_usd:
        raise LlmReviewError("manifest cost exceeds the HITL cap")
    if document["conclusion"] not in {"evidence_limited", "portfolio_qualified"}:
        raise LlmReviewError("manifest conclusion is invalid")

    reviewer_runs = document["reviewer_runs"]
    if not isinstance(reviewer_runs, list) or len(reviewer_runs) != protocol.reviewer_count:
        raise LlmReviewError("manifest reviewer_runs count does not match protocol")
    reviewer_ids: set[str] = set()
    model_identities: set[tuple[str, str]] = set()
    for run in reviewer_runs:
        if not isinstance(run, dict) or set(run) != _REVIEWER_RUN_KEYS:
            raise LlmReviewError("reviewer run keys are invalid")
        reviewer_id = _sha(run["reviewer_id_sha256"], "reviewer_id_sha256")
        reviewer_ids.add(reviewer_id)
        model_identities.add((_label(run["provider"], "provider"), _label(run["model"], "model")))
        for field in (
            "prompt_sha256",
            "transcript_sha256",
            "run_receipt_sha256",
            "blinded_assignment_sha256",
        ):
            _sha(run[field], field)
    if len(reviewer_ids) != protocol.reviewer_count:
        raise LlmReviewError("reviewer identities must be unique")
    if len(model_identities) != protocol.reviewer_count:
        raise LlmReviewError("reviewers must have independent model identities")

    rows = document["records"]
    if not isinstance(rows, list) or not rows:
        raise LlmReviewError("manifest records must be a non-empty list")
    annotations: set[str] = set()
    categories: set[str] = set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != _RECORD_KEYS:
            raise LlmReviewError("record keys are invalid")
        annotation_id = _identifier(row["annotation_id"], "annotation_id")
        if annotation_id in annotations:
            raise LlmReviewError("annotation_id must be unique")
        annotations.add(annotation_id)
        corpus_id = _identifier(row["corpus_id"], "corpus_id")
        if corpus_id not in matrix.entries:
            raise LlmReviewError("record corpus_id is not in the matrix")
        category = _identifier(row["category"], "category")
        if category not in _REQUIRED_CATEGORIES:
            raise LlmReviewError("record category is invalid")
        categories.add(category)
        _sha(row["source_locator_sha256"], "source_locator_sha256")
        _sha(row["evidence_locator_sha256"], "evidence_locator_sha256")
        judgments = row["reviewer_judgments"]
        if not isinstance(judgments, dict) or set(judgments) != reviewer_ids:
            raise LlmReviewError("record reviewer_judgments must bind every reviewer")
        if any(value not in _JUDGMENTS for value in judgments.values()):
            raise LlmReviewError("record reviewer judgment is invalid")
        if row["judgment"] not in _JUDGMENTS:
            raise LlmReviewError("record judgment is invalid")
        if row["adjudication"] not in _ADJUDICATIONS:
            raise LlmReviewError("record adjudication is invalid")
    if document["scope"] == "portfolio_complete" and categories != set(_REQUIRED_CATEGORIES):
        raise LlmReviewError("complete manifest must cover every lifecycle category")
    if document["conclusion"] == "portfolio_qualified" and categories != set(_REQUIRED_CATEGORIES):
        raise LlmReviewError("portfolio-qualified conclusion requires complete category coverage")
    return LlmReviewManifest(corpus_matrix.canonical_sha256(document), tuple(sorted(categories)), len(rows))
