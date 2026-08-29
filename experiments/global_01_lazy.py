"""GLOBAL-01 lazy-coverage preparation and experiment controls."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import statistics
import subprocess
import time
import urllib.error
import urllib.request
import zipfile
from datetime import datetime, timezone
from html.parser import HTMLParser
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib
from experiments.fathomdb_test_setup import prepare_test_database


SCHEMA = "global-01.lazy-coverage.v1"
RECOVERY_SCHEMA = "global-01.lazy-coverage.v2"
OUTPUT_RECOVERY_SCHEMA = "global-01.lazy-coverage.v3"
COMPACT_RECOVERY_SCHEMA = "global-01.lazy-coverage.v4"
SCORER_OUTPUT_SCHEMA = "global-01.lazy-coverage.v5"
SCORER_ADAPTER_SCHEMA = "global-01.lazy-coverage.v6"
TARGETED_RETRY_SCHEMA = "global-01.lazy-coverage.v7"
PROGRAM_TRACK = "GLOBAL-01"
PROFILE = "global_lazy_coverage_v1"
DISPOSITIONS = {
    "included",
    "redundant",
    "irrelevant",
    "conflicting-or-uncertain",
    "omitted-for-budget",
}
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_TOKEN = re.compile(r"[a-z0-9]+")


class Global01LazyError(ValueError):
    """Raised when the registered GLOBAL-01 lazy contract is violated."""


class _TextExtractor(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []

    def handle_data(self, data: str) -> None:
        value = data.strip()
        if value:
            self.parts.append(value)


def canonical_sha256(value: object) -> str:
    """Hash canonical JSON for a configuration or private manifest."""
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def file_sha256(path: Path) -> str:
    """Hash a file without loading it all into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _require_keys(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise Global01LazyError(f"{label} config keys do not match {SCHEMA}")
    return value


def _require_sha(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise Global01LazyError(f"{label} must be a lowercase SHA-256")
    return value


def validate_config(value: object) -> dict[str, Any]:
    """Strictly validate the registered lazy-coverage configuration."""
    root = _require_keys(
        value,
        "root",
        {
            "schema_version",
            "program_track",
            "run_label",
            "approval",
            "corpus",
            "questions",
            "profiles",
            "models",
            "evaluation",
            "execution",
            "pricing",
        },
    )
    schema = root["schema_version"]
    expected_label = {
        SCHEMA: "apnews-global-lazy-coverage",
        RECOVERY_SCHEMA: "apnews-global-lazy-coverage-v2",
        OUTPUT_RECOVERY_SCHEMA: "apnews-global-lazy-coverage-v3",
        COMPACT_RECOVERY_SCHEMA: "apnews-global-lazy-coverage-v4",
        SCORER_OUTPUT_SCHEMA: "apnews-global-lazy-coverage-v5",
        SCORER_ADAPTER_SCHEMA: "apnews-global-lazy-coverage-v6",
        TARGETED_RETRY_SCHEMA: "apnews-global-lazy-coverage-v7",
    }.get(schema)
    if (
        expected_label is None
        or root["program_track"] != PROGRAM_TRACK
        or root["run_label"] != expected_label
    ):
        raise Global01LazyError("GLOBAL-01 lazy schema, track, or label drifted")

    approval = _require_keys(
        root["approval"],
        "approval",
        {"state", "approved_by", "approved_at", "cost_cap_usd"},
    )
    if approval["state"] not in {"pending_hitl", "approved"}:
        raise Global01LazyError("approval state must be pending_hitl or approved")

    corpus = _require_keys(
        root["corpus"],
        "corpus",
        {
            "root",
            "archive_sha256",
            "article_count",
            "document_granularity",
            "license_class",
        },
    )
    _require_sha(corpus["archive_sha256"], "corpus archive")
    if (
        corpus["article_count"] != 1397
        or corpus["document_granularity"] != "canonical_article"
        or corpus["license_class"]
        != "noncommercial_nonredistributable_evaluation_only"
    ):
        raise Global01LazyError("AP News corpus contract drifted")

    questions = _require_keys(
        root["questions"],
        "questions",
        {
            "assertions_path",
            "assertions_file_sha256",
            "qualification",
            "qualified_count",
            "qualified_assertion_count",
            "qualified_manifest_sha256",
            "split",
        },
    )
    for key in (
        "assertions_file_sha256",
        "qualified_manifest_sha256",
    ):
        _require_sha(questions[key], key)
    if questions["qualified_count"] != 49 or questions["qualified_assertion_count"] != 637:
        raise Global01LazyError("qualified question or assertion count drifted")
    qualification = _require_keys(
        questions["qualification"],
        "qualification",
        {"is_valid", "grounding_min", "relevance_min", "verifiability_min"},
    )
    if qualification != {
        "is_valid": True,
        "grounding_min": 4,
        "relevance_min": 4,
        "verifiability_min": 4,
    }:
        raise Global01LazyError("assertion qualification drifted")
    split = _require_keys(
        questions["split"],
        "split",
        {
            "seed",
            "exclude_first_run_from_development",
            "first_run_private_manifest",
            "development_count",
            "development_selection_sha256",
            "heldout_count",
            "heldout_selection_sha256",
            "first_run_overlap_heldout_count",
            "witness_count",
            "witness_selection_sha256",
        },
    )
    for key in (
        "development_selection_sha256",
        "heldout_selection_sha256",
        "witness_selection_sha256",
    ):
        _require_sha(split[key], key)
    if (
        split["seed"] != "global-01-split-20260829-v1"
        or split["exclude_first_run_from_development"] is not True
        or split["development_count"] != 10
        or split["heldout_count"] != 39
        or split["witness_count"] != 3
        or split["first_run_overlap_heldout_count"] != 2
    ):
        raise Global01LazyError("question split contract drifted")

    profiles = _require_keys(root["profiles"], "profiles", {"control", "treatment"})
    control = profiles["control"]
    reduce_max_tokens = (
        4096
        if schema
        in {
            OUTPUT_RECOVERY_SCHEMA,
            COMPACT_RECOVERY_SCHEMA,
            SCORER_OUTPUT_SCHEMA,
            SCORER_ADAPTER_SCHEMA,
            TARGETED_RETRY_SCHEMA,
        }
        else 1500
    )
    if not isinstance(control, dict) or control != {
        "name": "source_mapreduce_c_v1_fts50",
        "retrieval": "fts_only",
        "candidate_depth": 50,
        "map_batch_documents": 5,
        "map_max_tokens": 300,
        "reduce_max_tokens": reduce_max_tokens,
    }:
        raise Global01LazyError("scaled control contract drifted")
    treatment = _require_keys(
        profiles["treatment"],
        "treatment",
        {
            "name",
            "caller_selected",
            "subquery_count",
            "retrieval",
            "candidate_depth_per_query",
            "rrf_k",
            "grouping",
            "relevance_weight",
            "novelty_weight",
            "max_documents_per_group",
            "max_documents_total",
            "context_max_tokens",
            "map_max_tokens",
            "reduce_max_tokens",
            "derived_persistence",
        },
    )
    if treatment["name"] != PROFILE or treatment["caller_selected"] is not True:
        raise Global01LazyError("lazy treatment must remain explicitly caller-selected")
    if (
        treatment["subquery_count"] != 4
        or treatment["retrieval"] != "fts_only"
        or treatment["candidate_depth_per_query"] != 50
        or treatment["rrf_k"] != 60
        or treatment["grouping"] != "subquery_intent_source_linked_v1"
        or treatment["relevance_weight"] + treatment["novelty_weight"] != 1.0
        or treatment["max_documents_per_group"] != 6
        or treatment["max_documents_total"] != 24
        or treatment["context_max_tokens"] != 24000
        or treatment["map_max_tokens"] != 600
        or treatment["reduce_max_tokens"] != reduce_max_tokens
        or treatment["derived_persistence"] != "none"
    ):
        raise Global01LazyError("lazy treatment parameters drifted")

    models = _require_keys(
        root["models"], "models", {"generator", "pairwise_judge", "assertion_scorer"}
    )
    if models["generator"].get("model") != "deepseek-v4-pro":
        raise Global01LazyError("generator model drifted")
    if models["pairwise_judge"].get("model") != "claude-haiku":
        raise Global01LazyError("pairwise judge model drifted")
    if models["assertion_scorer"].get("model") != "claude-haiku":
        raise Global01LazyError("assertion scorer model drifted")
    if models["assertion_scorer"].get("source_excerpt_max_chars") != 1600:
        raise Global01LazyError("assertion scorer excerpt boundary drifted")
    if schema in {
        SCORER_OUTPUT_SCHEMA,
        SCORER_ADAPTER_SCHEMA,
        TARGETED_RETRY_SCHEMA,
    } and models["assertion_scorer"].get("max_tokens") != 2048:
        raise Global01LazyError("v5 assertion scorer output ceiling drifted")

    execution = root["execution"]
    if (
        not isinstance(execution, dict)
        or execution.get("fathomdb_version") != "0.8.23"
        or execution.get("read_view") != "strict_current"
        or execution.get("checkpoint_every_cell") is not True
        or execution.get("resume_only_missing") is not True
        or execution.get("concurrency") != 1
        or execution.get("honor_retry_after") is not True
        or execution.get("completeness_required") is not True
    ):
        raise Global01LazyError("execution resilience contract drifted")
    if schema in {
        RECOVERY_SCHEMA,
        OUTPUT_RECOVERY_SCHEMA,
        COMPACT_RECOVERY_SCHEMA,
        SCORER_OUTPUT_SCHEMA,
        SCORER_ADAPTER_SCHEMA,
        TARGETED_RETRY_SCHEMA,
    } and execution.get(
        "map_output_adapter"
    ) != (
        "compact_refs_or_canonical_v2"
    ):
        raise Global01LazyError("v2 map output adapter drifted")
    if schema in {
        OUTPUT_RECOVERY_SCHEMA,
        COMPACT_RECOVERY_SCHEMA,
        SCORER_OUTPUT_SCHEMA,
        SCORER_ADAPTER_SCHEMA,
        TARGETED_RETRY_SCHEMA,
    } and (
        execution.get("generator_max_output_tokens") != 393216
        or execution.get("reduction_max_tokens_basis")
        != "DeepSeek docs, live route, and observed truncation (v1)"
    ):
        raise Global01LazyError("v3 output-limit basis drifted")
    if schema in {
        COMPACT_RECOVERY_SCHEMA,
        SCORER_OUTPUT_SCHEMA,
        SCORER_ADAPTER_SCHEMA,
        TARGETED_RETRY_SCHEMA,
    } and execution.get("reduce_output_adapter") != (
        "compact_refs_with_canonical_restore_v1"
    ):
        raise Global01LazyError("v4 reduction output adapter drifted")
    if schema in {
        SCORER_OUTPUT_SCHEMA,
        SCORER_ADAPTER_SCHEMA,
        TARGETED_RETRY_SCHEMA,
    } and (
        execution.get("scorer_max_output_tokens") != 65536
        or execution.get("scorer_max_tokens_basis")
        != "Anthropic model docs and observed truncation (v1)"
    ):
        raise Global01LazyError("v5 scorer output-limit basis drifted")
    if schema in {SCORER_ADAPTER_SCHEMA, TARGETED_RETRY_SCHEMA} and execution.get(
        "scorer_output_adapter"
    ) != "required_fields_projection_v1":
        raise Global01LazyError("v6 scorer output adapter drifted")
    if schema == TARGETED_RETRY_SCHEMA and execution.get(
        "semantic_retry_adapter"
    ) != "targeted_content_free_v1":
        raise Global01LazyError("v7 semantic retry adapter drifted")

    pricing = root["pricing"]
    if (
        not isinstance(pricing, dict)
        or pricing.get("currency") != "USD"
        or pricing.get("planned_model_cells") != 1376
        or pricing.get("maximum_semantic_submissions") != 4128
        or pricing.get("projected_total_usd", 0) <= 0
        or pricing.get("recommended_cap_usd", 0) < pricing["projected_total_usd"]
    ):
        raise Global01LazyError("pricing contract drifted")
    return json.loads(json.dumps(root))


def assert_execution_authorized(config: Mapping[str, Any]) -> None:
    """Refuse any model-backed call before explicit HITL authorization."""
    approval = config["approval"]
    if (
        approval["state"] != "approved"
        or not approval["approved_by"]
        or not approval["approved_at"]
        or not isinstance(approval["cost_cap_usd"], (int, float))
        or approval["cost_cap_usd"] <= 0
    ):
        raise Global01LazyError("paid execution requires explicit HITL approval")


def _qualified_assertions(row: Mapping[str, Any]) -> list[dict[str, Any]]:
    qualified = []
    for assertion in row.get("assertions", []):
        validation = assertion.get("validation", {})
        scores = validation.get("scores", {})
        if validation.get("is_valid") is True and all(
            scores.get(key, 0) >= 4
            for key in ("grounding", "relevance", "verifiability")
        ):
            qualified.append(assertion)
    return qualified


def qualify_and_split_questions(
    rows: Sequence[Mapping[str, Any]],
    *,
    first_run_question_hashes: set[str],
    split_seed: str,
    development_count: int,
    witness_count: int,
) -> dict[str, list[dict[str, Any]]]:
    """Apply the registered assertion threshold and deterministic split."""
    qualified = []
    for row in rows:
        assertions = _qualified_assertions(row)
        if not assertions:
            continue
        question_id = row.get("question_id")
        text = row.get("question_text")
        if not isinstance(question_id, str) or not isinstance(text, str):
            raise Global01LazyError("qualified question lacks stable identity or text")
        qualified.append(
            {
                "question_id": question_id,
                "question_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
                "qualified_assertion_count": len(assertions),
                "qualified_assertions_sha256": canonical_sha256(
                    [assertion["statement"] for assertion in assertions]
                ),
            }
        )
    candidates = [
        row for row in qualified if row["question_sha256"] not in first_run_question_hashes
    ]
    candidates.sort(
        key=lambda row: hashlib.sha256(
            f"{split_seed}:{row['question_id']}".encode("utf-8")
        ).hexdigest()
    )
    if len(candidates) < development_count or witness_count > development_count:
        raise Global01LazyError("question pool cannot satisfy the registered split")
    development = candidates[:development_count]
    development_ids = {row["question_id"] for row in development}
    heldout = [row for row in qualified if row["question_id"] not in development_ids]
    return {
        "qualified": qualified,
        "development": development,
        "heldout": heldout,
        "witness": development[:witness_count],
    }


def retrieve_candidates(
    engine: object,
    queries: Sequence[str],
    *,
    candidate_depth: int,
    source_hashes: Mapping[str, str],
    read_view: object,
) -> list[dict[str, Any]]:
    """Retrieve through one preconstructed ReadView and merge by canonical source."""
    merged: dict[str, dict[str, Any]] = {}
    for query_ordinal, query in enumerate(queries):
        result = engine.search(query, limit=candidate_depth, view=read_view)  # type: ignore[attr-defined]
        for rank, hit in enumerate(result.results, start=1):
            source_id = getattr(hit, "source_id", None)
            body = getattr(hit, "body", None)
            if not isinstance(source_id, str) or source_id not in source_hashes:
                raise Global01LazyError("retrieval hit lacks registered canonical source")
            if not isinstance(body, str) or not body:
                raise Global01LazyError("retrieval hit lacks canonical body")
            row = merged.setdefault(
                source_id,
                {
                    "source_id": source_id,
                    "content_sha256": source_hashes[source_id],
                    "body": body,
                    "ranks": {},
                },
            )
            if row["body"] != body:
                raise Global01LazyError("one canonical source returned conflicting bodies")
            row["ranks"][str(query_ordinal)] = rank
    return list(merged.values())


def _terms(text: str) -> set[str]:
    return {term for term in _TOKEN.findall(text.lower()) if len(term) >= 3}


def _jaccard(left: set[str], right: set[str]) -> float:
    union = left | right
    return len(left & right) / len(union) if union else 0.0


def select_candidates(
    candidates: Sequence[Mapping[str, Any]],
    *,
    rrf_k: int,
    relevance_weight: float,
    novelty_weight: float,
    max_documents_per_group: int,
    max_documents_total: int,
    context_max_tokens: int,
) -> list[dict[str, Any]]:
    """Select a bounded source-linked set with deterministic MMR-like scoring."""
    prepared = []
    for candidate in candidates:
        raw_ranks = candidate.get("ranks")
        if not isinstance(raw_ranks, dict) or not raw_ranks:
            raise Global01LazyError("candidate lacks query-rank membership")
        ranks = {str(key): int(value) for key, value in raw_ranks.items()}
        group = min(ranks, key=lambda key: (int(ranks[key]), int(key)))
        relevance = sum(1.0 / (rrf_k + int(rank)) for rank in ranks.values())
        body = str(candidate["body"])
        prepared.append(
            {
                **dict(candidate),
                "group_ordinal": int(group),
                "rrf_relevance": relevance,
                "terms": _terms(body),
                "estimated_tokens": max(1, math.ceil(len(body) / 4)),
            }
        )
    maximum = max((row["rrf_relevance"] for row in prepared), default=1.0)
    selected: list[dict[str, Any]] = []
    group_counts: dict[int, int] = {}
    used_tokens = 0
    remaining = prepared[:]
    while remaining and len(selected) < max_documents_total:
        scored = []
        for row in remaining:
            if group_counts.get(row["group_ordinal"], 0) >= max_documents_per_group:
                continue
            if used_tokens + row["estimated_tokens"] > context_max_tokens:
                continue
            novelty = 1.0 - max(
                (_jaccard(row["terms"], prior["terms"]) for prior in selected),
                default=0.0,
            )
            score = relevance_weight * (row["rrf_relevance"] / maximum) + novelty_weight * novelty
            scored.append((score, row["rrf_relevance"], str(row["source_id"]), row))
        if not scored:
            break
        _, _, _, chosen = max(scored, key=lambda item: (item[0], item[1], item[2]))
        remaining.remove(chosen)
        selected.append(chosen)
        used_tokens += chosen["estimated_tokens"]
        group = chosen["group_ordinal"]
        group_counts[group] = group_counts.get(group, 0) + 1
    result = []
    for ordinal, row in enumerate(selected, start=1):
        result.append(
            {
                key: row[key]
                for key in (
                    "source_id",
                    "content_sha256",
                    "body",
                    "ranks",
                    "group_ordinal",
                    "rrf_relevance",
                    "estimated_tokens",
                )
            }
            | {"selection_ordinal": ordinal}
        )
    return result


def validate_structured_answer(
    value: object,
    *,
    known_sources: Mapping[str, str],
    mapped_claim_ids: set[str],
) -> dict[str, Any]:
    """Require complete claim attribution and one ledger row per mapped claim."""
    if not isinstance(value, dict) or set(value) != {
        "answer",
        "claims",
        "coverage_ledger",
    }:
        raise Global01LazyError("structured answer keys are incomplete")
    if not isinstance(value["answer"], str) or not value["answer"].strip():
        raise Global01LazyError("structured answer text is empty")
    claims = value["claims"]
    if not isinstance(claims, list) or not claims:
        raise Global01LazyError("structured answer has no claims")
    final_ids: set[str] = set()
    for claim in claims:
        if not isinstance(claim, dict) or set(claim) != {
            "claim_id",
            "text",
            "sources",
        }:
            raise Global01LazyError("final claim keys are incomplete")
        claim_id = claim["claim_id"]
        if not isinstance(claim_id, str) or not claim_id or claim_id in final_ids:
            raise Global01LazyError("final claim identity is missing or duplicated")
        final_ids.add(claim_id)
        if not isinstance(claim["text"], str) or not claim["text"].strip():
            raise Global01LazyError("final claim text is empty")
        sources = claim["sources"]
        if not isinstance(sources, list) or not sources:
            raise Global01LazyError("final claim lacks a canonical source")
        for source in sources:
            if not isinstance(source, dict) or set(source) != {
                "source_id",
                "content_sha256",
            }:
                raise Global01LazyError("canonical source keys are incomplete")
            if known_sources.get(source["source_id"]) != source["content_sha256"]:
                raise Global01LazyError("canonical source identity or hash drifted")
    ledger = value["coverage_ledger"]
    if not isinstance(ledger, list):
        raise Global01LazyError("coverage ledger is missing")
    seen: set[str] = set()
    included_final_ids: set[str] = set()
    for row in ledger:
        if not isinstance(row, dict) or set(row) != {
            "mapped_claim_id",
            "disposition",
            "final_claim_ids",
        }:
            raise Global01LazyError("coverage ledger row is incomplete")
        mapped_id = row["mapped_claim_id"]
        if not isinstance(mapped_id, str) or mapped_id in seen:
            raise Global01LazyError("coverage ledger duplicates a mapped claim")
        seen.add(mapped_id)
        if row["disposition"] not in DISPOSITIONS:
            raise Global01LazyError("coverage ledger disposition is invalid")
        links = row["final_claim_ids"]
        if not isinstance(links, list) or any(link not in final_ids for link in links):
            raise Global01LazyError("coverage ledger references an unknown final claim")
        if row["disposition"] == "included" and not links:
            raise Global01LazyError("included ledger row lacks a final claim")
        included_final_ids.update(links)
    if seen != mapped_claim_ids:
        raise Global01LazyError("coverage ledger does not cover every mapped claim exactly once")
    if included_final_ids != final_ids:
        raise Global01LazyError("final claims are not fully linked from the coverage ledger")
    return json.loads(json.dumps(value))


def required_cells(
    *, question_ids: Sequence[str], pairwise_repetitions: int, scorer_trials: int
) -> list[str]:
    """Return the deterministic paid-cell plan used for completeness checks."""
    cells = []
    for question_id in question_ids:
        cells.extend(
            [
                f"answers/control/{question_id}",
                f"decomposition/treatment/{question_id}",
                f"answers/treatment/{question_id}",
            ]
        )
        for arm in ("control", "treatment"):
            for trial in range(scorer_trials):
                cells.append(f"scorer/{arm}/{question_id}/{trial}")
        for repetition in range(pairwise_repetitions):
            for order in ("ct", "tc"):
                cells.append(f"judge/{question_id}/{repetition}/{order}")
    return cells


def missing_cells(cells: Sequence[str], completed: set[str]) -> list[str]:
    """Preserve cell order while selecting only incomplete work."""
    return [cell for cell in cells if cell not in completed]


def _article_text(row: Mapping[str, Any]) -> tuple[str, str]:
    altids = row.get("altids")
    source_id = altids.get("itemid") if isinstance(altids, dict) else None
    headline = row.get("headline")
    body = row.get("body_nitf")
    if (
        not isinstance(source_id, str)
        or not source_id
        or not isinstance(headline, str)
        or not headline
        or not isinstance(body, str)
        or not body
    ):
        raise Global01LazyError("AP News article lacks source identity or text")
    parser = _TextExtractor()
    parser.feed(body)
    return source_id, f"{headline}\n\n{' '.join(parser.parts)}".strip()


def load_documents(
    config: Mapping[str, Any], repository_root: Path
) -> tuple[list[dict[str, Any]], str]:
    """Load and bind every canonical AP News document."""
    corpus_root = repository_root / config["corpus"]["root"]
    archive_path = corpus_root / "raw_data.zip"
    manifest_path = corpus_root / "MANIFEST.json"
    if not archive_path.is_file() or not manifest_path.is_file():
        raise Global01LazyError("AP News corpus or manifest is unavailable")
    archive_hash = file_sha256(archive_path)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if (
        archive_hash != config["corpus"]["archive_sha256"]
        or manifest.get("raw_data_zip_sha256") != archive_hash
        or manifest.get("n_articles") != config["corpus"]["article_count"]
    ):
        raise Global01LazyError("AP News corpus identity drifted")
    documents = []
    with zipfile.ZipFile(archive_path) as archive:
        names = sorted(name for name in archive.namelist() if name.endswith(".json"))
        if len(names) != config["corpus"]["article_count"]:
            raise Global01LazyError("AP News archive article count drifted")
        for ordinal, name in enumerate(names):
            source_id, text = _article_text(json.loads(archive.read(name)))
            documents.append(
                {
                    "ordinal": ordinal,
                    "source_id": source_id,
                    "text": text,
                    "content_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
                    "bytes": len(text.encode("utf-8")),
                }
            )
    if len({row["source_id"] for row in documents}) != len(documents):
        raise Global01LazyError("AP News canonical source IDs are not unique")
    return documents, file_sha256(manifest_path)


def inspect_question_inputs(
    config: Mapping[str, Any], repository_root: Path
) -> tuple[dict[str, Any], dict[str, Any]]:
    """Regenerate the qualified split and retain payload only in a private manifest."""
    corpus_root = repository_root / config["corpus"]["root"]
    question_path = corpus_root / config["questions"]["assertions_path"]
    if not question_path.is_file() or file_sha256(question_path) != config["questions"][
        "assertions_file_sha256"
    ]:
        raise Global01LazyError("assertion-backed question input drifted")
    rows = json.loads(question_path.read_text(encoding="utf-8"))
    if not isinstance(rows, list):
        raise Global01LazyError("assertion-backed question input is not a list")
    split_config = config["questions"]["split"]
    first_run_path = repository_root / split_config["first_run_private_manifest"]
    first_run = json.loads(first_run_path.read_text(encoding="utf-8"))
    first_run_hashes = {
        row["sha256"]
        for row in first_run.get("questions", [])
        if isinstance(row, dict) and isinstance(row.get("sha256"), str)
    }
    split = qualify_and_split_questions(
        rows,
        first_run_question_hashes=first_run_hashes,
        split_seed=split_config["seed"],
        development_count=split_config["development_count"],
        witness_count=split_config["witness_count"],
    )
    if (
        len(split["qualified"]) != config["questions"]["qualified_count"]
        or sum(row["qualified_assertion_count"] for row in split["qualified"])
        != config["questions"]["qualified_assertion_count"]
        or canonical_sha256(split["qualified"])
        != config["questions"]["qualified_manifest_sha256"]
        or canonical_sha256(split["development"])
        != split_config["development_selection_sha256"]
        or canonical_sha256(split["heldout"])
        != split_config["heldout_selection_sha256"]
        or canonical_sha256(split["witness"])
        != split_config["witness_selection_sha256"]
    ):
        raise Global01LazyError("qualified question manifest or split drifted")
    by_id = {row["question_id"]: row for row in rows}
    private_questions = []
    for split_name in ("development", "heldout"):
        witness_ids = {row["question_id"] for row in split["witness"]}
        for safe in split[split_name]:
            source = by_id[safe["question_id"]]
            assertions = _qualified_assertions(source)
            private_questions.append(
                {
                    **safe,
                    "split": split_name,
                    "witness": safe["question_id"] in witness_ids,
                    "text": source["question_text"],
                    "qualified_assertions": [row["statement"] for row in assertions],
                }
            )
    overlap = sum(
        row["question_sha256"] in first_run_hashes for row in split["heldout"]
    )
    if overlap != split_config["first_run_overlap_heldout_count"]:
        raise Global01LazyError("first-run overlap placement drifted")
    report = {
        "qualified_count": len(split["qualified"]),
        "qualified_assertion_count": sum(
            row["qualified_assertion_count"] for row in split["qualified"]
        ),
        "qualified_manifest_sha256": canonical_sha256(split["qualified"]),
        "development_count": len(split["development"]),
        "development_selection_sha256": canonical_sha256(split["development"]),
        "heldout_count": len(split["heldout"]),
        "heldout_selection_sha256": canonical_sha256(split["heldout"]),
        "witness_count": len(split["witness"]),
        "witness_selection_sha256": canonical_sha256(split["witness"]),
        "first_run_overlap_heldout_count": overlap,
    }
    return report, {
        "schema_version": "global-01.lazy-private-input.v1",
        "questions": private_questions,
    }


def inspect_airlock(base_url: str, key: str, config: Mapping[str, Any]) -> dict[str, Any]:
    """Perform an authenticated zero-spend model-discovery request."""
    request = urllib.request.Request(
        f"{base_url.rstrip('/')}/v1/models",
        headers={"Authorization": f"Bearer {key}"},
    )
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            payload = json.load(response)
    except (urllib.error.HTTPError, urllib.error.URLError) as exc:
        raise Global01LazyError("Airlock model discovery failed") from exc
    models = {
        row.get("id"): row for row in payload.get("data", []) if isinstance(row, dict)
    }
    aliases = [
        config["models"]["generator"]["model"],
        config["models"]["pairwise_judge"]["model"],
    ]
    if any(alias not in models for alias in aliases):
        raise Global01LazyError("Airlock lacks a required GLOBAL-01 model alias")
    return {
        "key_source": "AIRLOCK_VIRTUAL_KEY",
        "required_aliases": aliases,
        "model_count": len(models),
    }


def _percentile(values: Sequence[float], fraction: float) -> float:
    ordered = sorted(values)
    if not ordered:
        raise Global01LazyError("latency sample is empty")
    index = min(len(ordered) - 1, math.ceil(fraction * len(ordered)) - 1)
    return ordered[max(0, index)]


def run_lifecycle_canaries(engine: object, read_view_type: type) -> dict[str, Any]:
    """Prove strict-current, temporal, supersession, and erasure behavior."""
    engine.write(  # type: ignore[attr-defined]
        [
            {
                "kind": "document",
                "logical_id": "global-01-supersession-canary",
                "source_id": "global-01-canary-prior",
                "body": "globalcanarytoken priorvalue",
            }
        ]
    )
    engine.drain(timeout_s=60)  # type: ignore[attr-defined]
    engine.write(  # type: ignore[attr-defined]
        [
            {
                "kind": "document",
                "logical_id": "global-01-supersession-canary",
                "source_id": "global-01-canary-current",
                "body": "globalcanarytoken currentvalue",
            },
            {
                "kind": "document",
                "logical_id": "global-01-temporal-prior",
                "source_id": "global-01-temporal-prior-source",
                "body": "globaltemporaltoken priorwindow",
                "valid_from": 1000,
                "valid_until": 2000,
            },
            {
                "kind": "document",
                "logical_id": "global-01-temporal-current",
                "source_id": "global-01-temporal-current-source",
                "body": "globaltemporaltoken currentwindow",
                "valid_from": 2000,
            },
            {
                "kind": "document",
                "logical_id": "global-01-erasure-canary",
                "source_id": "global-01-erasure-source",
                "body": "globalerasetoken removable",
            },
        ]
    )
    engine.drain(timeout_s=60)  # type: ignore[attr-defined]
    strict = engine.search(  # type: ignore[attr-defined]
        "globalcanarytoken", limit=10, view=read_view_type()
    ).results
    strict_bodies = {hit.body for hit in strict}
    if strict_bodies != {"globalcanarytoken currentvalue"}:
        raise Global01LazyError("strict ReadView leaked a superseded canary")
    at_1500 = engine.search(  # type: ignore[attr-defined]
        "globaltemporaltoken", limit=10, view=read_view_type(valid_as_of=1500)
    ).results
    at_2500 = engine.search(  # type: ignore[attr-defined]
        "globaltemporaltoken", limit=10, view=read_view_type(valid_as_of=2500)
    ).results
    if {hit.body for hit in at_1500} != {"globaltemporaltoken priorwindow"}:
        raise Global01LazyError("ReadView leaked the wrong temporal state at 1500")
    if {hit.body for hit in at_2500} != {"globaltemporaltoken currentwindow"}:
        raise Global01LazyError("ReadView leaked the wrong temporal state at 2500")
    before = engine.search(  # type: ignore[attr-defined]
        "globalerasetoken", limit=10, view=read_view_type()
    ).results
    engine.erase_source("global-01-erasure-source")  # type: ignore[attr-defined]
    after = engine.search(  # type: ignore[attr-defined]
        "globalerasetoken", limit=10, view=read_view_type()
    ).results
    if not before or after:
        raise Global01LazyError("source erasure canary failed")
    return {
        "strict_current_supersession": "pass",
        "temporal_boundaries": 2,
        "temporal_failures": 0,
        "erasure": "pass",
        "derived_rows_written": 0,
    }


def run_preflight(
    config: Mapping[str, Any],
    *,
    repository_root: Path,
    artifact_root: Path,
    airlock_url: str,
) -> dict[str, Any]:
    """Execute all registered zero-spend input, runtime, and lifecycle checks."""
    if artifact_root.exists():
        raise Global01LazyError("preflight artifact root already exists")
    artifact_root.mkdir(parents=True, mode=0o700)
    documents, corpus_manifest_hash = load_documents(config, repository_root)
    question_report, private_manifest = inspect_question_inputs(config, repository_root)
    private_manifest["documents"] = [
        {
            key: row[key]
            for key in ("ordinal", "source_id", "content_sha256", "bytes")
        }
        for row in documents
    ]
    private_path = artifact_root / "private-input-manifest.json"
    private_path.write_text(json.dumps(private_manifest, indent=2) + "\n", encoding="utf-8")
    private_path.chmod(0o600)

    key = os.environ.get("AIRLOCK_VIRTUAL_KEY")
    if not key:
        raise Global01LazyError("AIRLOCK_VIRTUAL_KEY is required for authenticated preflight")
    authentication = inspect_airlock(airlock_url, key, config)

    cli_path = (repository_root / config["execution"]["fathomdb_cli"]).resolve()
    if not cli_path.is_file():
        raise Global01LazyError("registered FathomDB CLI is unavailable")
    version = subprocess.run(
        [str(cli_path), "--version"], check=True, text=True, capture_output=True
    ).stdout.strip()
    if version != "fathomdb 0.8.23":
        raise Global01LazyError("registered FathomDB CLI version drifted")
    prepared = prepare_test_database(
        artifact_root / "database",
        test_id="global-01-lazy-preflight",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        warm_cache=False,
        check_reranker=False,
        fathomdb_bin=str(cli_path),
    )
    from fathomdb import Engine, __version__ as fathomdb_version  # type: ignore[import-not-found]
    from fathomdb.types import ReadView  # type: ignore[import-not-found]

    if fathomdb_version != config["execution"]["fathomdb_version"]:
        raise Global01LazyError("FathomDB Python runtime version drifted")
    engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
    try:
        for start in range(0, len(documents), 100):
            engine.write(
                [
                    {
                        "kind": "document",
                        "logical_id": f"global-01-ap-{row['ordinal']:04d}",
                        "source_id": row["source_id"],
                        "body": row["text"],
                    }
                    for row in documents[start : start + 100]
                ]
            )
        engine.drain(timeout_s=180)
    finally:
        engine.close()

    source_hashes = {row["source_id"]: row["content_sha256"] for row in documents}
    questions = private_manifest["questions"]
    cold_ms = []
    steady_ms = []
    hit_counts = []
    control_context_tokens = 0
    engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
    try:
        view = ReadView()
        for question in questions:
            started = time.perf_counter()
            rows = retrieve_candidates(
                engine,
                [question["text"]],
                candidate_depth=config["profiles"]["control"]["candidate_depth"],
                source_hashes=source_hashes,
                read_view=view,
            )
            cold_ms.append((time.perf_counter() - started) * 1000)
            hit_counts.append(len(rows))
            control_context_tokens += sum(math.ceil(len(row["body"]) / 4) for row in rows)
            for _ in range(config["evaluation"]["steady_repetitions"]):
                started = time.perf_counter()
                retrieve_candidates(
                    engine,
                    [question["text"]],
                    candidate_depth=config["profiles"]["control"]["candidate_depth"],
                    source_hashes=source_hashes,
                    read_view=view,
                )
                steady_ms.append((time.perf_counter() - started) * 1000)
        lifecycle = run_lifecycle_canaries(engine, ReadView)
    finally:
        engine.close()
    if min(hit_counts, default=0) == 0:
        raise Global01LazyError("scaled control retrieved no candidates for a question")

    safe = {
        "schema_version": "global-01.lazy-preflight.v1",
        "state": "ready_for_hitl",
        "program_track": PROGRAM_TRACK,
        "config_sha256": canonical_sha256(config),
        "cost_usd": 0.0,
        "corpus": {
            "archive_sha256": config["corpus"]["archive_sha256"],
            "manifest_sha256": corpus_manifest_hash,
            "article_count": len(documents),
            "document_binding_sha256": canonical_sha256(private_manifest["documents"]),
        },
        "questions": question_report,
        "runtime": {
            "fathomdb_python_version": fathomdb_version,
            "fathomdb_cli_version": version,
            "fathomdb_cli_sha256": file_sha256(cli_path),
            "doctor_sha256": file_sha256(prepared.doctor_path),
            "database_config_sha256": file_sha256(prepared.config_path),
            "embedder": "none",
            "reranker": "disabled",
            "gpu_required": False,
        },
        "authentication": authentication,
        "retrieval": {
            "questions": len(questions),
            "candidate_depth": config["profiles"]["control"]["candidate_depth"],
            "minimum_hit_count": min(hit_counts),
            "maximum_hit_count": max(hit_counts),
            "control_context_tokens_estimate": control_context_tokens,
            "cold_p50_ms": statistics.median(cold_ms),
            "cold_p95_ms": _percentile(cold_ms, 0.95),
            "steady_p50_ms": statistics.median(steady_ms),
            "steady_p95_ms": _percentile(steady_ms, 0.95),
        },
        "lifecycle": lifecycle,
        "resilience": {
            "checkpoint_every_cell": True,
            "resume_only_missing": True,
            "retry_attempts": config["execution"]["retry_attempts"],
            "honor_retry_after": True,
            "completeness_required": True,
        },
        "pricing": {
            "planned_model_cells": config["pricing"]["planned_model_cells"],
            "maximum_semantic_submissions": config["pricing"][
                "maximum_semantic_submissions"
            ],
            "projected_total_usd": config["pricing"]["projected_total_usd"],
            "recommended_cap_usd": config["pricing"]["recommended_cap_usd"],
        },
    }
    safe_path = artifact_root / "safe-preflight.json"
    safe_path.write_text(json.dumps(safe, indent=2) + "\n", encoding="utf-8")
    safe_path.chmod(0o600)
    return safe


def register_preflight_receipt(
    config: Mapping[str, Any],
    *,
    report_path: Path,
    config_path: Path,
    base_dir: Path,
    ts: datetime,
) -> tuple[str, Path]:
    """Register one safe zero-spend preflight receipt and index row."""
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if (
        report.get("schema_version") != "global-01.lazy-preflight.v1"
        or report.get("state") != "ready_for_hitl"
        or report.get("config_sha256") != canonical_sha256(config)
        or report.get("cost_usd") != 0.0
    ):
        raise Global01LazyError("safe preflight report identity or state drifted")
    authorized = config["approval"]["state"] == "approved"
    verdict = "authorized_ready" if authorized else "awaiting_hitl"
    run_id, run_dir = _lib.write_record(
        "global-01-lazy-preflight",
        ts=ts,
        config_obj=config,
        metrics=report,
        verdict=verdict,
        read=(
            "GLOBAL-01 lazy-coverage input, isolated 0.8.23 runtime, retrieval, "
            "lifecycle, Airlock alias, and resilience preflight passed at zero spend; "
            + (
                "the registered paid A/A gate is authorized."
                if authorized
                else "paid A/A and witness remain unauthorized."
            )
        ),
        code=_lib.git_info(),
        corpus={
            "source": "AP News BenchmarkQED",
            "manifest_sha256": config["corpus"]["archive_sha256"],
            "datasets": [
                {
                    "name": "AP News BenchmarkQED",
                    "documents": report["corpus"]["article_count"],
                    "questions": report["questions"]["qualified_count"],
                    "selection_sha256": config["questions"][
                        "qualified_manifest_sha256"
                    ],
                }
            ],
        },
        seeds={"split": config["questions"]["split"]["seed"]},
        env=_lib.env_info(
            key_deps={
                "fathomdb": report["runtime"]["fathomdb_python_version"],
                "generator": config["models"]["generator"]["model"],
                "judge": config["models"]["pairwise_judge"]["model"],
            }
        ),
        cost_usd=0.0,
        headline={
            "program_track": PROGRAM_TRACK,
            "status": "authorized_ready" if authorized else "ready_for_hitl",
        },
        n=report["questions"]["qualified_count"],
        config_path=str(config_path),
        tdd_evidence={
            "red": "paid CLI and lifecycle/resume contracts failed before implementation",
            "green": "GLOBAL-01 lazy preparation and execution-control tests pass",
        },
        tests=[
            "tests/experiments/test_global_01_lazy.py",
            "tests/experiments/test_global_01_lazy_live.py",
        ],
        artifacts=[
            {
                "path": str(report_path),
                "sha256": file_sha256(report_path),
            }
        ],
        open_questions=(
            []
            if authorized
            else [
                "HITL must authorize the paid A/A check and a hard USD cap before execution."
            ]
        ),
        base_dir=base_dir,
    )
    _lib.regen_index_md(
        index_path=base_dir / "index.jsonl", md_path=base_dir / "INDEX.md"
    )
    return run_id, run_dir


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=("validate", "preflight", "register-preflight")
    )
    parser.add_argument("config", type=Path)
    parser.add_argument("--repository-root", type=Path, default=Path.cwd())
    parser.add_argument("--artifact-root", type=Path)
    parser.add_argument("--report-path", type=Path)
    parser.add_argument("--base-dir", type=Path, default=Path("experiments"))
    parser.add_argument("--airlock-url", default="http://127.0.0.1:4000")
    args = parser.parse_args()
    config = validate_config(json.loads(args.config.read_text(encoding="utf-8")))
    if args.command == "preflight":
        if args.artifact_root is None:
            parser.error("preflight requires --artifact-root")
        report = run_preflight(
            config,
            repository_root=args.repository_root.resolve(),
            artifact_root=args.artifact_root.resolve(),
            airlock_url=args.airlock_url,
        )
        print(json.dumps(report, sort_keys=True))
        return 0
    if args.command == "register-preflight":
        if args.report_path is None:
            parser.error("register-preflight requires --report-path")
        run_id, run_dir = register_preflight_receipt(
            config,
            report_path=args.report_path,
            config_path=args.config,
            base_dir=args.base_dir,
            ts=datetime.now(timezone.utc).replace(second=0, microsecond=0),
        )
        print(json.dumps({"run_id": run_id, "run_dir": str(run_dir)}))
        return 0
    print(
        json.dumps(
            {
                "state": "valid",
                "schema_version": config["schema_version"],
                "config_sha256": canonical_sha256(config),
                "approval": config["approval"]["state"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
