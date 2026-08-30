"""Pure GRAPH-01 protected bridge-completion mechanics.

The live runner owns I/O, FathomDB setup, model calls, and receipt writing. This
module keeps the projection transformation, bounded path selection, statistics,
decision rules, and paid-state semantics deterministic and testable offline.
"""

from __future__ import annotations

import json
import math
import os
import random
import unicodedata
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Mapping, Sequence


class Graph01Error(RuntimeError):
    """Raised when GRAPH-01 cannot preserve its registered contract."""


_GENERIC = {"entity", "person", "place", "thing", "unknown"}


def _tokens(value: str) -> list[str]:
    tokens: list[str] = []
    current: list[str] = []
    current_kind: str | None = None
    for character in value.casefold():
        category = unicodedata.category(character)
        kind = "word" if category[0] in {"L", "M", "N"} else "symbol" if category[0] == "S" else None
        if kind != current_kind and current:
            tokens.append("".join(current))
            current = []
        if kind is None:
            current_kind = None
            continue
        current.append(character)
        current_kind = kind
    if current:
        tokens.append("".join(current))
    return tokens


def normalize_entity(value: str) -> str:
    """Return the stable Unicode-token identity used only within a question."""
    return " ".join(_tokens(value))


def _phrase_in(entity: str, text: str) -> bool:
    entity_tokens = entity.split()
    text_tokens = _tokens(text)
    if not entity_tokens:
        return False
    width = len(entity_tokens)
    return any(text_tokens[i : i + width] == entity_tokens for i in range(len(text_tokens) - width + 1))


@dataclass(frozen=True)
class AdmittedEdge:
    """One deterministic, exactly attributed relation eligible for traversal."""

    edge_id: str
    question_id: str
    paragraph_idx: int
    source_id: str
    subject: str
    predicate: str
    object: str
    raw_subject: str
    raw_object: str


def _entities(value: object) -> list[Mapping[str, object]]:
    if not isinstance(value, list):
        raise Graph01Error("extraction entities must be a list")
    if not all(isinstance(item, dict) for item in value):
        raise Graph01Error("extraction entity must be an object")
    return value


def _relations(value: object) -> list[Mapping[str, object]]:
    if not isinstance(value, list):
        raise Graph01Error("extraction relations must be a list")
    if not all(isinstance(item, dict) for item in value):
        raise Graph01Error("extraction relation must be an object")
    return value


def _paragraph_index(paragraphs: Sequence[Mapping[str, object]]) -> dict[int, Mapping[str, object]]:
    result: dict[int, Mapping[str, object]] = {}
    for paragraph in paragraphs:
        idx = paragraph.get("idx")
        if not isinstance(idx, int) or isinstance(idx, bool) or idx in result:
            raise Graph01Error("paragraph indices must be unique integers")
        if not isinstance(paragraph.get("title"), str) or not isinstance(paragraph.get("text"), str):
            raise Graph01Error("paragraph title and text must be strings")
        result[idx] = paragraph
    return result


def admit_relations(
    question_id: str,
    paragraphs: Sequence[Mapping[str, object]],
    extractions: Mapping[str, Mapping[str, object]],
    *,
    generic_entities: set[str] | None = None,
    allow_missing_empty: bool = False,
) -> tuple[list[AdmittedEdge], dict[str, Any]]:
    """Apply the frozen question-blind support filter to extracted relations."""
    by_idx = _paragraph_index(paragraphs)
    generic = generic_entities or _GENERIC
    types: dict[str, set[str]] = {}
    entity_sources: dict[str, set[int]] = {}
    duplicate_entities = 0
    missing_extractions = 0
    for idx in sorted(by_idx):
        extraction = extractions.get(f"{question_id}#{idx}")
        if not isinstance(extraction, Mapping):
            if not allow_missing_empty:
                raise Graph01Error(f"missing extraction for {question_id}#{idx}")
            extraction = {"entities": [], "relations": []}
            missing_extractions += 1
        seen: set[str] = set()
        for entity in _entities(extraction.get("entities")):
            name, type_name = entity.get("name"), entity.get("type")
            if not isinstance(name, str) or not isinstance(type_name, str):
                raise Graph01Error("entity name and type must be strings")
            norm = normalize_entity(name)
            if not norm:
                raise Graph01Error("entity name normalizes empty")
            if norm in seen:
                duplicate_entities += 1
            seen.add(norm)
            types.setdefault(norm, set()).add(normalize_entity(type_name))
            entity_sources.setdefault(norm, set()).add(idx)
    conflicts = {name for name, values in types.items() if len(values) > 1}
    report: dict[str, Any] = {
        "relations_total": 0,
        "relations_admitted": 0,
        "rejected_missing_endpoint": 0,
        "rejected_nonverbatim_endpoint": 0,
        "rejected_generic_endpoint": 0,
        "rejected_type_conflict": 0,
        "rejected_malformed_relation": 0,
        "duplicate_entities": duplicate_entities,
        "type_conflict_entities": len(conflicts),
        "missing_extractions": missing_extractions,
        "endpoint_orphans": 0,
        "source_link_completeness": 1.0,
    }
    admitted: list[AdmittedEdge] = []
    for idx in sorted(by_idx):
        source_id = f"{question_id}#{idx}"
        extraction = extractions.get(source_id)
        if not isinstance(extraction, Mapping):
            if not allow_missing_empty:
                raise Graph01Error(f"missing extraction for {source_id}")
            extraction = {"entities": [], "relations": []}
        paragraph_entities = {
            normalize_entity(str(entity["name"]))
            for entity in _entities(extraction.get("entities"))
        }
        paragraph = by_idx[idx]
        body = f"{paragraph['title']}\n{paragraph['text']}"
        for ordinal, relation in enumerate(_relations(extraction.get("relations"))):
            report["relations_total"] += 1
            raw_subject = relation.get("subject")
            raw_predicate = relation.get("predicate")
            raw_object = relation.get("object")
            if not all(isinstance(value, str) and value.strip() for value in (raw_subject, raw_predicate, raw_object)):
                report["rejected_malformed_relation"] += 1
                continue
            subject = normalize_entity(raw_subject)
            object_ = normalize_entity(raw_object)
            rejected = False
            if subject not in paragraph_entities or object_ not in paragraph_entities:
                report["rejected_missing_endpoint"] += 1
                rejected = True
            if not _phrase_in(subject, body) or not _phrase_in(object_, body):
                report["rejected_nonverbatim_endpoint"] += 1
                rejected = True
            if subject in generic or object_ in generic:
                report["rejected_generic_endpoint"] += 1
                rejected = True
            if subject in conflicts or object_ in conflicts:
                report["rejected_type_conflict"] += 1
                rejected = True
            if rejected:
                continue
            if subject not in entity_sources or object_ not in entity_sources:
                report["endpoint_orphans"] += 1
                continue
            admitted.append(
                AdmittedEdge(
                    edge_id=f"{source_id}|edge:{ordinal}",
                    question_id=question_id,
                    paragraph_idx=idx,
                    source_id=source_id,
                    subject=subject,
                    predicate=" ".join(raw_predicate.split()),
                    object=object_,
                    raw_subject=" ".join(raw_subject.split()),
                    raw_object=" ".join(raw_object.split()),
                )
            )
    report["relations_admitted"] = len(admitted)
    if admitted and any(not edge.source_id for edge in admitted):
        report["source_link_completeness"] = 0.0
    return admitted, report


def paragraph_entity_membership(
    question_id: str,
    paragraphs: Sequence[Mapping[str, object]],
    extractions: Mapping[str, Mapping[str, object]],
    *,
    allow_missing_empty: bool = False,
) -> dict[int, set[str]]:
    """Return extracted entity membership by paragraph index."""
    by_idx = _paragraph_index(paragraphs)
    result: dict[int, set[str]] = {}
    for idx in sorted(by_idx):
        extraction = extractions.get(f"{question_id}#{idx}")
        if not isinstance(extraction, Mapping):
            if not allow_missing_empty:
                raise Graph01Error(f"missing extraction for {question_id}#{idx}")
            extraction = {"entities": [], "relations": []}
        result[idx] = {
            normalize_entity(str(entity["name"]))
            for entity in _entities(extraction.get("entities"))
        }
    return result


def projection_items(
    question_id: str,
    paragraphs: Sequence[Mapping[str, object]],
    extractions: Mapping[str, Mapping[str, object]],
    edges: Sequence[AdmittedEdge],
    *,
    allow_missing_empty: bool = False,
) -> list[dict[str, Any]]:
    """Create deterministic canonical writes with exact edge provenance."""
    by_idx = _paragraph_index(paragraphs)
    items: list[dict[str, Any]] = []
    for idx, paragraph in sorted(by_idx.items()):
        source_id = f"{question_id}#{idx}"
        items.append(
            {
                "kind": "doc",
                "body": f"{paragraph['title']}\n{paragraph['text']}",
                "logical_id": source_id,
                "source_id": source_id,
            }
        )
    membership = paragraph_entity_membership(
        question_id,
        paragraphs,
        extractions,
        allow_missing_empty=allow_missing_empty,
    )
    first_source: dict[str, int] = {}
    display: dict[str, str] = {}
    for idx in sorted(membership):
        extraction = extractions.get(f"{question_id}#{idx}")
        if not isinstance(extraction, Mapping):
            if not allow_missing_empty:
                raise Graph01Error(f"missing extraction for {question_id}#{idx}")
            extraction = {"entities": [], "relations": []}
        for entity in _entities(extraction.get("entities")):
            norm = normalize_entity(str(entity["name"]))
            first_source.setdefault(norm, idx)
            display.setdefault(norm, " ".join(str(entity["name"]).split()))
    for norm in sorted(first_source):
        source_id = f"{question_id}#{first_source[norm]}"
        items.append(
            {
                "kind": "entity",
                "body": display[norm],
                "logical_id": f"{question_id}|ent:{norm}",
                "source_id": source_id,
            }
        )
    for edge in sorted(edges, key=lambda item: item.edge_id):
        items.append(
            {
                "edge": {
                    "kind": "relation",
                    "from": f"{question_id}|ent:{edge.subject}",
                    "to": f"{question_id}|ent:{edge.object}",
                    "logical_id": edge.edge_id,
                    "source_id": edge.source_id,
                }
            }
        )
    return items


@dataclass(frozen=True)
class BridgeRanking:
    """One bounded treatment ranking and its auditable promotions."""

    ranking: list[int]
    promoted: tuple[int, ...]
    path_depths: tuple[int, ...]
    query_anchors: tuple[str, ...]


def protected_bridge_ranking(
    *,
    question: str,
    baseline: Sequence[int],
    paragraph_entities: Mapping[int, set[str]],
    edges: Sequence[AdmittedEdge],
    seed_passages: int,
    protected_ranks: int,
    promotion_max: int,
    candidate_depth: int,
    context_passages: int,
) -> BridgeRanking:
    """Promote path-source passages without displacing protected control ranks."""
    if not 0 <= protected_ranks <= context_passages <= candidate_depth <= len(baseline):
        raise Graph01Error("bridge ranking boundaries are inconsistent")
    all_entities = set().union(*paragraph_entities.values()) if paragraph_entities else set()
    anchors = tuple(sorted(entity for entity in all_entities if _phrase_in(entity, question)))
    if not anchors:
        return BridgeRanking(list(baseline[:context_passages]), (), (), ())
    seed_entity_sources: dict[str, set[int]] = {}
    for paragraph_idx in baseline[:seed_passages]:
        for entity in paragraph_entities.get(paragraph_idx, set()):
            seed_entity_sources.setdefault(entity, set()).add(paragraph_idx)
    adjacency: dict[str, list[tuple[str, AdmittedEdge]]] = {}
    for edge in edges:
        adjacency.setdefault(edge.subject, []).append((edge.object, edge))
        adjacency.setdefault(edge.object, []).append((edge.subject, edge))
    for value in adjacency.values():
        value.sort(key=lambda row: (row[0], row[1].edge_id))
    baseline_top = set(baseline[:context_passages])
    candidates = set(baseline[:candidate_depth]) - baseline_top
    evidence: dict[int, dict[str, Any]] = {}
    for anchor in anchors:
        frontier: list[tuple[str, tuple[AdmittedEdge, ...], frozenset[str]]] = [
            (anchor, (), frozenset({anchor}))
        ]
        for depth in (1, 2):
            next_frontier: list[tuple[str, tuple[AdmittedEdge, ...], frozenset[str]]] = []
            for node, path, visited in frontier:
                for neighbor, edge in adjacency.get(node, []):
                    if neighbor in visited:
                        continue
                    new_path = (*path, edge)
                    new_visited = visited | {neighbor}
                    seed_sources = seed_entity_sources.get(neighbor, set())
                    if seed_sources:
                        for path_edge in new_path:
                            candidate = path_edge.paragraph_idx
                            if candidate not in candidates:
                                continue
                            item = evidence.setdefault(
                                candidate,
                                {"anchors": set(), "seeds": set(), "min_depth": depth},
                            )
                            item["anchors"].add(anchor)
                            item["seeds"].update(seed_sources)
                            item["min_depth"] = min(item["min_depth"], depth)
                    if depth < 2:
                        next_frontier.append((neighbor, new_path, new_visited))
            frontier = next_frontier
    positions = {paragraph: rank for rank, paragraph in enumerate(baseline)}
    ordered = sorted(
        evidence,
        key=lambda paragraph: (
            -len(evidence[paragraph]["anchors"]),
            -len(evidence[paragraph]["seeds"]),
            evidence[paragraph]["min_depth"],
            positions[paragraph],
            paragraph,
        ),
    )
    promoted = tuple(ordered[:promotion_max])
    ranking = list(baseline[:protected_ranks]) + list(promoted)
    for paragraph in baseline:
        if paragraph not in ranking:
            ranking.append(paragraph)
        if len(ranking) == context_passages:
            break
    return BridgeRanking(
        ranking,
        promoted,
        tuple(int(evidence[item]["min_depth"]) for item in promoted),
        anchors,
    )


def _quantile(values: Sequence[float], probability: float) -> float:
    ordered = sorted(values)
    if not ordered:
        raise Graph01Error("quantile requires observations")
    position = probability * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    weight = position - lower
    return ordered[lower] * (1 - weight) + ordered[upper] * weight


def _defined_deltas(
    control: Sequence[float | None], treatment: Sequence[float | None]
) -> tuple[list[float], int]:
    if len(control) != len(treatment) or not control:
        raise Graph01Error("paired inputs are invalid")
    deltas: list[float] = []
    excluded = 0
    for control_value, treatment_value in zip(control, treatment, strict=True):
        if control_value is None and treatment_value is None:
            excluded += 1
            continue
        if control_value is None or treatment_value is None:
            raise Graph01Error("paired inputs have asymmetric undefined values")
        deltas.append(float(treatment_value) - float(control_value))
    if not deltas:
        raise Graph01Error("paired inputs have no defined observations")
    return deltas, excluded


def paired_mean_delta(
    control: Sequence[float | None], treatment: Sequence[float | None]
) -> float:
    """Return a paired mean after symmetrically excluding undefined gold cells."""
    deltas, _excluded = _defined_deltas(control, treatment)
    return sum(deltas) / len(deltas)


def bootstrap_paired_mean(
    control: Sequence[float | None],
    treatment: Sequence[float | None],
    *,
    draws: int,
    seed: int,
) -> dict[str, Any]:
    """Return point and percentile interval for paired question-level deltas."""
    if draws <= 0:
        raise Graph01Error("paired bootstrap inputs are invalid")
    deltas, excluded = _defined_deltas(control, treatment)
    rng = random.Random(seed)
    samples = [
        sum(deltas[rng.randrange(len(deltas))] for _ in deltas) / len(deltas)
        for _ in range(draws)
    ]
    return {
        "point": sum(deltas) / len(deltas),
        "ci95": [_quantile(samples, 0.025), _quantile(samples, 0.975)],
        "n": len(deltas),
        "excluded_undefined": excluded,
        "draws": draws,
        "seed": seed,
    }


def retrieval_decision(metrics: Mapping[str, Any]) -> dict[str, Any]:
    """Apply the frozen all-boundary retrieval rule."""
    delta = metrics["complete_bridge_delta"]
    boundaries = {
        "quality": metrics["quality_eligible"] is True,
        "lifecycle": metrics["lifecycle_eligible"] is True,
        "complete_bridge_point": float(delta["point"]) >= 0.04,
        "complete_bridge_ci": float(delta["ci95"][0]) > 0.0,
        "supporting_recall": float(metrics["supporting_recall_delta"]) >= 0.0,
        "two_hop": float(metrics["two_hop_complete_bridge_delta"]) > -0.02,
        "distinctness": float(metrics["distinct_question_rate"]) >= 0.10,
        "latency": float(metrics["graph_addon_p95_ms"]) <= 25.0,
        "storage": float(metrics["storage_amplification"]) <= 1.5,
    }
    return {"passed": all(boundaries.values()), "boundaries": boundaries}


def answer_f1_decision(
    *,
    retrieval_passed: bool,
    supporting_recall_delta: float,
    answer_delta: Mapping[str, Any],
) -> dict[str, Any]:
    """Apply the frozen retrieval-confirmation or material-answer routes."""
    point = float(answer_delta["point"])
    lower = float(answer_delta["ci95"][0])
    retrieval_route = retrieval_passed and lower > -0.02
    answer_route = point >= 0.04 and lower > 0.0 and supporting_recall_delta >= 0.0
    return {
        "accepted": retrieval_route or answer_route,
        "retrieval_route": retrieval_route,
        "material_answer_route": answer_route,
    }


def wilson_interval(successes: int, total: int, *, z: float = 1.959963984540054) -> list[float]:
    """Wilson score interval for independent edge-audit precision."""
    if total <= 0 or not 0 <= successes <= total:
        raise Graph01Error("Wilson inputs are invalid")
    p = successes / total
    denominator = 1 + z * z / total
    center = (p + z * z / (2 * total)) / denominator
    radius = z * math.sqrt(p * (1 - p) / total + z * z / (4 * total * total)) / denominator
    return [max(0.0, center - radius), min(1.0, center + radius)]


@dataclass
class PaidState:
    """Atomically persisted paid cells with a hard cumulative cost cap."""

    schema_version: str
    config_sha256: str
    cost_cap_usd: float
    started_at: str
    cost_usd: float = 0.0
    cells: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def new(cls, config_sha256: str, cost_cap_usd: float) -> "PaidState":
        started_at = datetime.now(timezone.utc).replace(second=0, microsecond=0).isoformat()
        return cls("graph-01.paid-state.v1", config_sha256, cost_cap_usd, started_at)

    @classmethod
    def load(cls, path: Path, config_sha256: str, cost_cap_usd: float) -> "PaidState":
        value = json.loads(path.read_text(encoding="utf-8"))
        if (
            value.get("schema_version") != "graph-01.paid-state.v1"
            or value.get("config_sha256") != config_sha256
            or value.get("cost_cap_usd") != cost_cap_usd
        ):
            raise Graph01Error("checkpoint configuration or cost cap drifted")
        return cls(**value)

    @property
    def remaining_cost_usd(self) -> float:
        return self.cost_cap_usd - self.cost_usd

    def complete(self, cell: str, value: Any, *, cost_usd: float) -> None:
        if cell in self.cells:
            raise Graph01Error(f"cell already complete: {cell}")
        if self.cost_usd + cost_usd > self.cost_cap_usd + 1e-9:
            raise Graph01Error("GRAPH-01 cost cap would be exceeded")
        self.cost_usd += cost_usd
        self.cells[cell] = value

    def missing(self, required: Sequence[str]) -> list[str]:
        return [cell for cell in required if cell not in self.cells]

    def save(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(path.suffix + ".tmp")
        temporary.write_text(json.dumps(asdict(self), indent=2) + "\n", encoding="utf-8")
        temporary.chmod(0o600)
        os.replace(temporary, path)


def _json_exact(text: str) -> Mapping[str, Any]:
    candidate = text.strip()
    lines = candidate.splitlines()
    if len(lines) >= 3 and lines[0] in {"```", "```json"} and lines[-1] == "```":
        candidate = "\n".join(lines[1:-1]).strip()
    try:
        value = json.loads(candidate)
    except json.JSONDecodeError as exc:
        raise Graph01Error("model response is not JSON") from exc
    if not isinstance(value, dict):
        raise Graph01Error("model response is not an object")
    return value


def parse_edge_audit(text: str) -> dict[str, bool]:
    """Parse an exact batched independent edge-precision judgment."""
    value = _json_exact(text)
    if set(value) != {"edges"} or not isinstance(value["edges"], list):
        raise Graph01Error("edge audit must contain only an edges list")
    result: dict[str, bool] = {}
    for row in value["edges"]:
        if not isinstance(row, dict) or set(row) != {"edge_id", "supported"}:
            raise Graph01Error("edge audit row shape is invalid")
        edge_id, supported = row["edge_id"], row["supported"]
        if not isinstance(edge_id, str) or not edge_id or not isinstance(supported, bool):
            raise Graph01Error("edge audit row values are invalid")
        if edge_id in result:
            raise Graph01Error("edge audit contains duplicate edge IDs")
        result[edge_id] = supported
    return result


def parse_answer(text: str) -> str:
    """Parse an exact answer-only response."""
    value = _json_exact(text)
    if set(value) != {"answer"} or not isinstance(value["answer"], str):
        raise Graph01Error("answer response must contain only a string answer")
    return " ".join(value["answer"].split())
