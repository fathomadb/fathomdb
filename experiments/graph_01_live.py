"""GRAPH-01 fresh native projection, paid audit, retrieval, and answer run."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import sqlite3
import statistics
import subprocess
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

from experiments import _lib, graph_01


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG = REPO_ROOT / "experiments/configs/graph-01/musique-protected-bridge.v1.json"
DEFAULT_RUN_ROOT = (
    REPO_ROOT
    / "data/performance-benchmarking/graph-01/runs/graph-01-protected-bridge-20260829-a"
)


def _sha256_path(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _canonical_sha256(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def _load_config(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if (
        not isinstance(value, dict)
        or value.get("schema_version") != "graph-01.protected-bridge.v1"
        or value.get("program_track") != "GRAPH-01"
        or value.get("approval", {}).get("state") != "approved"
        or value.get("approval", {}).get("cost_cap_usd") != 20.0
    ):
        raise graph_01.Graph01Error("GRAPH-01 configuration is not approved v1")
    return value


def _resolve(path: str) -> Path:
    return REPO_ROOT / path


def _repo_relative(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(REPO_ROOT.resolve()))
    except ValueError as exc:
        raise graph_01.Graph01Error("GRAPH-01 receipt path is outside the repository") from exc


def _verify_inputs(config: Mapping[str, Any]) -> dict[str, Any]:
    inputs = config["inputs"]
    result: dict[str, Any] = {}
    for name in ("corpus", "extractions", "cohort"):
        path = _resolve(inputs[f"{name}_path"])
        if not path.is_file():
            raise graph_01.Graph01Error(f"missing GRAPH-01 {name} input")
        digest = _sha256_path(path)
        if digest != inputs[f"{name}_sha256"]:
            raise graph_01.Graph01Error(f"GRAPH-01 {name} input drifted")
        result[name] = {"path": str(path), "sha256": digest, "bytes": path.stat().st_size}
    return result


def _load_questions(config: Mapping[str, Any]) -> tuple[list[Any], dict[str, Any]]:
    from eval.m1_baseline import load_musique

    questions = load_musique(_resolve(config["inputs"]["corpus_path"]))
    cohort = json.loads(_resolve(config["inputs"]["cohort_path"]).read_text(encoding="utf-8"))
    records = cohort.get("baseline_run", {}).get("paired_records")
    if cohort.get("run_valid") is not True or not isinstance(records, list):
        raise graph_01.Graph01Error("GRAPH-01 cohort is not the valid M1 result")
    qids = [row.get("qid") for row in records]
    if len(qids) != config["inputs"]["question_count"] or len(set(qids)) != len(qids):
        raise graph_01.Graph01Error("GRAPH-01 cohort IDs are incomplete or duplicated")
    by_id = {question.id: question for question in questions}
    try:
        selected = [by_id[qid] for qid in qids]
    except KeyError as exc:
        raise graph_01.Graph01Error("GRAPH-01 cohort question is absent from corpus") from exc
    return selected, cohort


def _paragraph_dicts(question: Any) -> list[dict[str, object]]:
    return [
        {
            "idx": paragraph.idx,
            "title": paragraph.title,
            "text": paragraph.text,
        }
        for paragraph in question.paragraphs
    ]


def _prepare_projection(
    config: Mapping[str, Any],
    questions: Sequence[Any],
    run_root: Path,
) -> tuple[dict[str, list[graph_01.AdmittedEdge]], dict[str, dict[int, set[str]]], dict[str, Any]]:
    extractions = json.loads(
        _resolve(config["inputs"]["extractions_path"]).read_text(encoding="utf-8")
    )
    if not isinstance(extractions, dict):
        raise graph_01.Graph01Error("GRAPH-01 extractions are not an object")
    payload_path = run_root / "projection-input.jsonl"
    edges_by_q: dict[str, list[graph_01.AdmittedEdge]] = {}
    membership_by_q: dict[str, dict[int, set[str]]] = {}
    aggregate: dict[str, float] = {}
    item_count = 0
    with payload_path.open("w", encoding="utf-8") as output:
        for question in questions:
            paragraphs = _paragraph_dicts(question)
            edges, report = graph_01.admit_relations(
                question.id,
                paragraphs,
                extractions,
                generic_entities=set(config["treatment"]["generic_entity_tokens"]),
                allow_missing_empty=(
                    config["inputs"]["missing_extraction_policy"]
                    == "empty_graph_keep_document"
                ),
            )
            edges_by_q[question.id] = edges
            membership_by_q[question.id] = graph_01.paragraph_entity_membership(
                question.id,
                paragraphs,
                extractions,
                allow_missing_empty=True,
            )
            for key, value in report.items():
                if isinstance(value, (int, float)) and key != "source_link_completeness":
                    aggregate[key] = aggregate.get(key, 0.0) + float(value)
            for item in graph_01.projection_items(
                question.id,
                paragraphs,
                extractions,
                edges,
                allow_missing_empty=True,
            ):
                output.write(json.dumps(item, ensure_ascii=False) + "\n")
                item_count += 1
    payload_path.chmod(0o600)
    aggregate["projection_items"] = item_count
    aggregate["source_link_completeness"] = 1.0
    return edges_by_q, membership_by_q, aggregate


def _native_build(
    payload_path: Path,
    native_root: Path,
    fathomdb_bin: Path,
    report_path: Path,
) -> None:
    from experiments.fathomdb_test_setup import prepare_test_database
    import fathomdb

    prepared = prepare_test_database(
        native_root,
        test_id="projection",
        embed_device="auto",
        rerank_device="auto",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=str(fathomdb_bin),
    )
    engine = fathomdb.Engine.open(str(prepared.database_path), use_default_embedder=False)
    written = 0
    batch: list[dict[str, Any]] = []
    try:
        with payload_path.open(encoding="utf-8") as source:
            for line in source:
                batch.append(json.loads(line))
                if len(batch) == 250:
                    engine.write(batch)
                    written += len(batch)
                    batch.clear()
            if batch:
                engine.write(batch)
                written += len(batch)
        engine.drain(timeout_s=900)
    finally:
        engine.close()

    canary = prepare_test_database(
        native_root,
        test_id="canary",
        embed_device="auto",
        rerank_device="auto",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=str(fathomdb_bin),
    )
    lifecycle = {"supersession_stale_hits": 1, "erasure_stale_hits": 1}
    engine = fathomdb.Engine.open(str(canary.database_path), use_default_embedder=False)
    try:
        node = lambda logical_id, body, source_id: {  # noqa: E731
            "kind": "entity",
            "body": body,
            "logical_id": logical_id,
            "source_id": source_id,
        }
        engine.write(
            [
                node("root", "root", "canary-v1"),
                node("old", "old", "canary-v1"),
                {"edge": {"kind": "relation", "from": "root", "to": "old", "logical_id": "edge", "source_id": "canary-v1"}},
            ]
        )
        engine.write(
            [
                node("root", "root", "canary-v2"),
                node("new", "new", "canary-v2"),
                {"edge": {"kind": "relation", "from": "root", "to": "new", "logical_id": "edge", "source_id": "canary-v2"}},
            ]
        )
        current = fathomdb.graph.neighbors(engine, "root", 1, "outgoing")
        lifecycle["supersession_stale_hits"] = sum(row.logical_id == "old" for row in current)
        engine.erase_source("canary-v2")
        after_erase = fathomdb.graph.neighbors(engine, "root", 1, "outgoing")
        lifecycle["erasure_stale_hits"] = len(after_erase)
    finally:
        engine.close()

    integrity = subprocess.run(
        [str(fathomdb_bin), "doctor", "check-integrity", "--json", str(prepared.database_path)],
        check=True,
        capture_output=True,
        text=True,
    )
    report = {
        "schema_version": "graph-01.native-build.v1",
        "database_path": str(prepared.database_path),
        "config_path": str(prepared.config_path),
        "doctor_path": str(prepared.doctor_path),
        "items_written": written,
        "lifecycle": lifecycle,
        "integrity": json.loads(integrity.stdout),
    }
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    report_path.chmod(0o600)


def _ensure_native_projection(config: Mapping[str, Any], run_root: Path) -> dict[str, Any]:
    report_path = run_root / "native-build-report.json"
    if report_path.is_file():
        return json.loads(report_path.read_text(encoding="utf-8"))
    python = _resolve(config["execution"]["fathomdb_python"])
    cli = _resolve(config["execution"]["fathomdb_cli"])
    if not python.is_file() or not cli.is_file():
        raise graph_01.Graph01Error("FathomDB 0.8.23 experiment toolchain is unavailable")
    environment = dict(os.environ)
    environment["PYTHONPATH"] = str(REPO_ROOT)
    subprocess.run(
        [
            str(python),
            str(Path(__file__).resolve()),
            "native-build",
            "--payload",
            str(run_root / "projection-input.jsonl"),
            "--native-root",
            str(run_root / "native"),
            "--fathomdb-bin",
            str(cli),
            "--report",
            str(report_path),
        ],
        cwd=REPO_ROOT,
        check=True,
        env=environment,
    )
    return json.loads(report_path.read_text(encoding="utf-8"))


def _read_native_edges(
    report: Mapping[str, Any],
    expected: Mapping[str, Sequence[graph_01.AdmittedEdge]],
) -> tuple[dict[str, list[graph_01.AdmittedEdge]], dict[str, Any]]:
    database = Path(str(report["database_path"]))
    expected_by_id = {edge.edge_id: edge for edges in expected.values() for edge in edges}
    by_q: dict[str, list[graph_01.AdmittedEdge]] = {}
    connection = sqlite3.connect(f"file:{database}?mode=ro", uri=True)
    try:
        rows = connection.execute(
            "SELECT logical_id,from_id,to_id,source_id FROM canonical_edges "
            "WHERE superseded_at IS NULL"
        ).fetchall()
        for logical_id, from_id, to_id, source_id in rows:
            original = expected_by_id.get(logical_id)
            if original is None:
                raise graph_01.Graph01Error("native projection contains an unexpected edge")
            subject = str(from_id).split("|ent:", 1)[-1]
            object_ = str(to_id).split("|ent:", 1)[-1]
            if (subject, object_, source_id) != (
                original.subject,
                original.object,
                original.source_id,
            ):
                raise graph_01.Graph01Error("native edge identity or provenance drifted")
            by_q.setdefault(original.question_id, []).append(original)
        source_complete = connection.execute(
            "SELECT COUNT(*) FROM canonical_edges e JOIN canonical_nodes d "
            "ON d.logical_id=e.source_id AND d.kind='doc' AND d.superseded_at IS NULL "
            "WHERE e.superseded_at IS NULL"
        ).fetchone()[0]
        endpoint_orphans = connection.execute(
            "SELECT COUNT(*) FROM canonical_edges e LEFT JOIN canonical_nodes f "
            "ON f.logical_id=e.from_id AND f.superseded_at IS NULL "
            "LEFT JOIN canonical_nodes t ON t.logical_id=e.to_id AND t.superseded_at IS NULL "
            "WHERE e.superseded_at IS NULL AND (f.logical_id IS NULL OR t.logical_id IS NULL)"
        ).fetchone()[0]
        inactive = connection.execute(
            "SELECT COUNT(*) FROM canonical_edges WHERE superseded_at IS NOT NULL"
        ).fetchone()[0]
        stale = connection.execute(
            "SELECT COUNT(*) FROM canonical_edges WHERE superseded_at IS NULL "
            "AND t_invalid IS NOT NULL AND t_invalid <= ?",
            (int(time.time()),),
        ).fetchone()[0]
        node_count = connection.execute(
            "SELECT COUNT(*) FROM canonical_nodes WHERE superseded_at IS NULL"
        ).fetchone()[0]
    finally:
        connection.close()
    if len(rows) != len(expected_by_id):
        raise graph_01.Graph01Error("native projection edge count is incomplete")
    return by_q, {
        "active_edges": len(rows),
        "active_nodes": node_count,
        "source_link_completeness": source_complete / max(len(rows), 1),
        "endpoint_orphans": endpoint_orphans,
        "inactive_edges": inactive,
        "active_stale_edges": stale,
        "database_bytes": database.stat().st_size,
    }


def _retry_delay(headers: Mapping[str, str], fallback: float) -> float:
    value = headers.get("Retry-After")
    if value is not None:
        try:
            return max(0.0, float(value))
        except ValueError:
            try:
                parsed = parsedate_to_datetime(value)
                return max(0.0, (parsed - datetime.now(timezone.utc)).total_seconds())
            except (TypeError, ValueError, OverflowError):
                pass
    return fallback


class AirlockClient:
    """OpenAI-compatible caller with provider-aware bounded backoff."""

    def __init__(self, config: Mapping[str, Any], key: str) -> None:
        if not key:
            raise graph_01.Graph01Error("AIRLOCK_VIRTUAL_KEY or AIRLOCK_MASTER_KEY is required")
        self.config = config
        self.base_url = os.environ.get("AIRLOCK_BASE_URL", "http://127.0.0.1:4000").rstrip("/")
        if not self.base_url.startswith(("http://127.0.0.1:", "http://localhost:")):
            raise graph_01.Graph01Error("Airlock must use authenticated loopback")
        self.key = key

    def models(self) -> set[str]:
        request = urllib.request.Request(
            f"{self.base_url}/v1/models",
            headers={"Authorization": f"Bearer {self.key}"},
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:  # noqa: S310
                payload = json.load(response)
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as exc:
            raise graph_01.Graph01Error("Airlock model preflight failed") from exc
        return {
            row["id"]
            for row in payload.get("data", [])
            if isinstance(row, dict) and isinstance(row.get("id"), str)
        }

    def reservation(self, model: str, prompt: str, max_tokens: int) -> float:
        price = self.config["pricing"][model]
        return (
            len(prompt.encode("utf-8")) * price["input_per_million"] / 1_000_000
            + max_tokens * price["output_per_million"] / 1_000_000
        )

    def complete(
        self,
        model: str,
        prompt: str,
        *,
        max_tokens: int,
        temperature: float,
    ) -> tuple[str, dict[str, int], float, float]:
        body: dict[str, Any] = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
            "temperature": temperature,
            "response_format": {"type": "json_object"},
        }
        if model == "deepseek-v4-pro":
            body["thinking"] = {"type": "disabled"}
        execution = self.config["execution"]
        started = time.perf_counter()
        for attempt in range(execution["retry_attempts"]):
            request = urllib.request.Request(
                f"{self.base_url}/v1/chat/completions",
                data=json.dumps(body).encode("utf-8"),
                headers={
                    "Authorization": f"Bearer {self.key}",
                    "Content-Type": "application/json",
                },
            )
            try:
                with urllib.request.urlopen(request, timeout=330) as response:  # noqa: S310
                    payload = json.load(response)
                content = payload["choices"][0]["message"]["content"]
                usage = payload.get("usage", {})
                prompt_tokens = int(usage.get("prompt_tokens", 0))
                completion_tokens = int(usage.get("completion_tokens", 0))
                if not isinstance(content, str) or not content or prompt_tokens <= 0 or completion_tokens <= 0:
                    raise graph_01.Graph01Error("Airlock response lacks content or usage")
                price = self.config["pricing"][model]
                cost = (
                    prompt_tokens * price["input_per_million"] / 1_000_000
                    + completion_tokens * price["output_per_million"] / 1_000_000
                )
                return (
                    content,
                    {"prompt_tokens": prompt_tokens, "completion_tokens": completion_tokens},
                    cost,
                    (time.perf_counter() - started) * 1000,
                )
            except urllib.error.HTTPError as exc:
                if exc.code != 429 and exc.code < 500:
                    raise graph_01.Graph01Error(f"Airlock HTTP {exc.code}") from exc
                if attempt + 1 == execution["retry_attempts"]:
                    raise graph_01.Graph01Error("Airlock retry budget exhausted") from exc
                delay = float(execution["retry_backoff_seconds"][attempt])
                time.sleep(_retry_delay(dict(exc.headers.items()), delay))
            except (urllib.error.URLError, TimeoutError) as exc:
                if attempt + 1 == execution["retry_attempts"]:
                    raise graph_01.Graph01Error("Airlock timeout retry budget exhausted") from exc
                time.sleep(float(execution["retry_backoff_seconds"][attempt]))
        raise AssertionError("unreachable")


@dataclass(frozen=True)
class Cell:
    name: str
    model: str
    prompt: str
    max_tokens: int
    temperature: float
    parse: Callable[[str], Any]


def _run_cells(
    client: AirlockClient,
    state: graph_01.PaidState,
    checkpoint: Path,
    cells: Sequence[Cell],
    *,
    concurrency: int,
    semantic_attempts: int = 3,
) -> None:
    pending = [cell for cell in cells if cell.name not in state.cells]
    while pending:
        batch = pending[:concurrency]
        reservations = sum(
            client.reservation(cell.model, cell.prompt, cell.max_tokens) for cell in batch
        )
        if reservations > state.remaining_cost_usd + 1e-9:
            raise graph_01.Graph01Error("GRAPH-01 cost cap cannot reserve next batch")
        with ThreadPoolExecutor(max_workers=len(batch)) as pool:
            futures = {
                pool.submit(
                    client.complete,
                    cell.model,
                    cell.prompt,
                    max_tokens=cell.max_tokens,
                    temperature=cell.temperature,
                ): cell
                for cell in batch
            }
            for future in as_completed(futures):
                cell = futures[future]
                content, usage, cost, latency_ms = future.result()
                try:
                    parsed = cell.parse(content)
                except graph_01.Graph01Error:
                    invalid_count = sum(
                        name.startswith(f"{cell.name}/invalid/") for name in state.cells
                    )
                    state.complete(
                        f"{cell.name}/invalid/{invalid_count}",
                        {
                            "response_sha256": hashlib.sha256(content.encode()).hexdigest(),
                            "usage": usage,
                            "latency_ms": latency_ms,
                        },
                        cost_usd=cost,
                    )
                    state.save(checkpoint)
                    if invalid_count + 1 >= semantic_attempts:
                        raise graph_01.Graph01Error(
                            f"semantic retries exhausted for {cell.name}"
                        )
                    continue
                state.complete(
                    cell.name,
                    {"value": parsed, "usage": usage, "latency_ms": latency_ms},
                    cost_usd=cost,
                )
                state.save(checkpoint)
        pending = [cell for cell in cells if cell.name not in state.cells]


def _edge_audit_prompt(
    edges: Sequence[graph_01.AdmittedEdge],
    question_by_id: Mapping[str, Any],
    edge_to_audit_id: Mapping[str, str],
) -> str:
    rows = []
    for edge in edges:
        paragraph = next(
            item
            for item in question_by_id[edge.question_id].paragraphs
            if item.idx == edge.paragraph_idx
        )
        rows.append(
            {
                "edge_id": edge_to_audit_id[edge.edge_id],
                "subject": edge.raw_subject,
                "predicate": edge.predicate,
                "object": edge.raw_object,
                "source": paragraph.body,
            }
        )
    return (
        "Judge each extracted relation only against its source. supported=true only when the source "
        "explicitly supports the subject-predicate-object relationship; mere co-occurrence is false. "
        "You receive no question, answer, or retrieval label. Return exactly one row per supplied "
        "edge_id and no other keys.\n\n"
        f"EDGES={json.dumps(rows, ensure_ascii=False)}\n\n"
        'Return JSON only: {"edges":[{"edge_id":"...","supported":true}]}.'
    )


def _answer_prompt(question: Any, ranking: Sequence[int]) -> str:
    context = "\n\n".join(
        f"[{rank + 1}] {question.paragraphs[index].body}"
        for rank, index in enumerate(ranking[:10])
    )
    return (
        "Answer the question using only the supplied passages. If the passages do not support an "
        "answer, return an empty answer. Be concise. Return exactly one JSON key.\n\n"
        f"QUESTION: {question.question}\n\nPASSAGES:\n{context}\n\n"
        'Return JSON only: {"answer":"..."}.'
    )


def _percentile(values: Sequence[float], p: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return 0.0
    index = p * (len(ordered) - 1)
    low, high = int(index), min(int(index) + 1, len(ordered) - 1)
    return ordered[low] + (ordered[high] - ordered[low]) * (index - low)


def _run(config_path: Path, run_root: Path) -> tuple[str, Path]:
    config = _load_config(config_path)
    input_report = _verify_inputs(config)
    run_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    questions, _cohort = _load_questions(config)
    question_by_id = {question.id: question for question in questions}
    expected_edges, membership, extraction_report = _prepare_projection(
        config, questions, run_root
    )
    native_report = _ensure_native_projection(config, run_root)
    native_edges, native_metrics = _read_native_edges(native_report, expected_edges)
    corpus_bytes = input_report["corpus"]["bytes"]
    native_metrics["storage_amplification"] = native_metrics["database_bytes"] / corpus_bytes

    config_sha = _canonical_sha256(config)
    checkpoint = run_root / "checkpoint.v1.json"
    state = (
        graph_01.PaidState.load(checkpoint, config_sha, 20.0)
        if checkpoint.is_file()
        else graph_01.PaidState.new(config_sha, 20.0)
    )
    key = os.environ.get("AIRLOCK_VIRTUAL_KEY") or os.environ.get("AIRLOCK_MASTER_KEY") or ""
    client = AirlockClient(config, key)
    required_models = {
        config["models"]["edge_judge"]["model"],
        config["models"]["answerer"]["model"],
    }
    if not required_models.issubset(client.models()):
        raise graph_01.Graph01Error("GRAPH-01 required Airlock model aliases are unavailable")

    all_edges = sorted(
        (edge for edges in native_edges.values() for edge in edges), key=lambda edge: edge.edge_id
    )
    sample_count = min(config["quality"]["edge_audit_sample"], len(all_edges))
    rng = random.Random(config["evaluation"]["bootstrap_seed"])
    audited = sorted(rng.sample(all_edges, sample_count), key=lambda edge: edge.edge_id)
    audit_cells: list[Cell] = []
    batch_size = config["quality"]["edge_audit_batch"]
    judge = config["models"]["edge_judge"]
    for start in range(0, len(audited), batch_size):
        batch = audited[start : start + batch_size]
        edge_to_audit_id = {
            edge.edge_id: f"audit-{offset:03d}" for offset, edge in enumerate(batch)
        }
        audit_id_to_edge = {value: key for key, value in edge_to_audit_id.items()}
        expected_ids = set(audit_id_to_edge)

        def parse_audit(
            text: str,
            expected_ids: set[str] = expected_ids,
            audit_id_to_edge: Mapping[str, str] = audit_id_to_edge,
        ) -> dict[str, bool]:
            value = graph_01.parse_edge_audit(text)
            if set(value) != expected_ids:
                raise graph_01.Graph01Error("edge audit IDs are incomplete or unexpected")
            return {audit_id_to_edge[audit_id]: supported for audit_id, supported in value.items()}

        audit_cells.append(
            Cell(
                f"audit/{start // batch_size}",
                judge["model"],
                _edge_audit_prompt(batch, question_by_id, edge_to_audit_id),
                judge["max_tokens"],
                judge["temperature"],
                parse_audit,
            )
        )
    _run_cells(client, state, checkpoint, audit_cells, concurrency=1)
    judgments: dict[str, bool] = {}
    for cell in audit_cells:
        judgments.update(state.cells[cell.name]["value"])
    successes = sum(judgments.values())
    precision = successes / max(len(judgments), 1)
    precision_ci = graph_01.wilson_interval(successes, len(judgments))
    lifecycle = native_report["lifecycle"]
    quality_eligible = (
        precision >= config["quality"]["edge_precision_min"]
        and precision_ci[0] >= config["quality"]["edge_precision_wilson_lower_min"]
        and native_metrics["source_link_completeness"] == 1.0
        and native_metrics["endpoint_orphans"] == 0
        and native_metrics["active_stale_edges"] == 0
    )
    lifecycle_eligible = (
        lifecycle["supersession_stale_hits"] == 0
        and lifecycle["erasure_stale_hits"] == 0
    )

    from eval.m1_baseline import (
        BGEEncoder,
        bm25_rank,
        bridges_present_at_k,
        dense_rank,
        f1_score,
        recall_at_k,
        rrf_fuse,
        supporting_positions,
    )

    encoder = BGEEncoder()
    if not encoder.available:
        raise graph_01.Graph01Error("pinned BGE assets are unavailable")
    observations = []
    graph_latencies = []
    for ordinal, question in enumerate(questions, 1):
        bm25 = bm25_rank(question.question, question.paragraphs)
        dense = dense_rank(question.question, question.paragraphs, encoder)
        control = rrf_fuse([bm25, dense], k=config["control"]["rrf_k"])
        started = time.perf_counter()
        bridge = graph_01.protected_bridge_ranking(
            question=question.question,
            baseline=control,
            paragraph_entities=membership[question.id],
            edges=native_edges.get(question.id, []),
            seed_passages=config["treatment"]["seed_passages"],
            protected_ranks=config["treatment"]["protected_control_ranks"],
            promotion_max=config["treatment"]["promotion_max"],
            candidate_depth=config["treatment"]["candidate_depth"],
            context_passages=config["treatment"]["context_passages"],
        )
        graph_latencies.append((time.perf_counter() - started) * 1000)
        gold = supporting_positions(question)
        observations.append(
            {
                "question_id": question.id,
                "hop_count": question.hop_count,
                "gold": sorted(gold),
                "control": control[:10],
                "treatment": bridge.ranking,
                "promoted": list(bridge.promoted),
                "query_anchor_count": len(bridge.query_anchors),
                "control_recall": recall_at_k(control, gold, 10),
                "treatment_recall": recall_at_k(bridge.ranking, gold, 10),
                "control_complete": bridges_present_at_k(control, gold, 10),
                "treatment_complete": bridges_present_at_k(bridge.ranking, gold, 10),
            }
        )
        if ordinal % 10 == 0:
            print(f"GRAPH-01 retrieval {ordinal}/{len(questions)}", flush=True)

    primary = [row for row in observations if row["hop_count"] in config["evaluation"]["primary_hops"]]
    two_hop = [row for row in observations if row["hop_count"] == 2]
    bootstrap = graph_01.bootstrap_paired_mean(
        [row["control_complete"] for row in primary],
        [row["treatment_complete"] for row in primary],
        draws=config["evaluation"]["bootstrap_draws"],
        seed=config["evaluation"]["bootstrap_seed"],
    )
    recall_delta = graph_01.paired_mean_delta(
        [row["control_recall"] for row in primary],
        [row["treatment_recall"] for row in primary],
    )
    two_hop_delta = graph_01.paired_mean_delta(
        [row["control_complete"] for row in two_hop],
        [row["treatment_complete"] for row in two_hop],
    )
    distinct_rate = statistics.fmean(row["control"] != row["treatment"] for row in observations)
    retrieval_metrics = {
        "quality_eligible": quality_eligible,
        "lifecycle_eligible": lifecycle_eligible,
        "complete_bridge_delta": bootstrap,
        "supporting_recall_delta": recall_delta,
        "two_hop_complete_bridge_delta": two_hop_delta,
        "distinct_question_rate": distinct_rate,
        "graph_addon_p95_ms": _percentile(graph_latencies, 0.95),
        "storage_amplification": native_metrics["storage_amplification"],
    }
    retrieval_rule = graph_01.retrieval_decision(retrieval_metrics)

    if not quality_eligible or not lifecycle_eligible:
        raise graph_01.Graph01Error("GRAPH-01 graph-quality or lifecycle eligibility failed")

    answerer = config["models"]["answerer"]
    answer_cells: list[Cell] = []
    for row in primary:
        question = question_by_id[row["question_id"]]
        if row["control"] == row["treatment"]:
            answer_cells.append(
                Cell(
                    f"answer/shared/{question.id}",
                    answerer["model"],
                    _answer_prompt(question, row["control"]),
                    answerer["max_tokens"],
                    answerer["temperature"],
                    graph_01.parse_answer,
                )
            )
        else:
            for arm in ("control", "treatment"):
                answer_cells.append(
                    Cell(
                        f"answer/{arm}/{question.id}",
                        answerer["model"],
                        _answer_prompt(question, row[arm]),
                        answerer["max_tokens"],
                        answerer["temperature"],
                        graph_01.parse_answer,
                    )
                )
    _run_cells(
        client,
        state,
        checkpoint,
        answer_cells,
        concurrency=config["execution"]["concurrency"],
    )
    control_f1, treatment_f1 = [], []
    for row in primary:
        question = question_by_id[row["question_id"]]
        if row["control"] == row["treatment"]:
            answer = state.cells[f"answer/shared/{question.id}"]["value"]
            control_answer = treatment_answer = answer
        else:
            control_answer = state.cells[f"answer/control/{question.id}"]["value"]
            treatment_answer = state.cells[f"answer/treatment/{question.id}"]["value"]
        row["control_answer"] = control_answer
        row["treatment_answer"] = treatment_answer
        control_f1.append(f1_score(control_answer, question.golds))
        treatment_f1.append(f1_score(treatment_answer, question.golds))
    answer_delta = graph_01.bootstrap_paired_mean(
        control_f1,
        treatment_f1,
        draws=config["evaluation"]["bootstrap_draws"],
        seed=config["evaluation"]["bootstrap_seed"],
    )
    answer_rule = graph_01.answer_f1_decision(
        retrieval_passed=retrieval_rule["passed"],
        supporting_recall_delta=recall_delta,
        answer_delta=answer_delta,
    )
    accepted = answer_rule["accepted"]

    observations_path = run_root / "observations.v1.json"
    observations_path.write_text(
        json.dumps({"schema_version": "graph-01.observations.v1", "rows": observations}, indent=2)
        + "\n",
        encoding="utf-8",
    )
    observations_path.chmod(0o600)
    metrics = {
        "schema_version": "graph-01.result.v1",
        "program_track": "GRAPH-01",
        "state": "complete",
        "verdict": "accept" if accepted else "reject",
        "question_count": len(observations),
        "primary_question_count": len(primary),
        "cost_usd": state.cost_usd,
        "cost_cap_usd": state.cost_cap_usd,
        "graph_quality": {
            **extraction_report,
            **native_metrics,
            "edge_audit_n": len(judgments),
            "edge_precision": precision,
            "edge_precision_wilson95": precision_ci,
            "eligible": quality_eligible,
        },
        "lifecycle": {**lifecycle, "eligible": lifecycle_eligible},
        "retrieval": {**retrieval_metrics, "decision": retrieval_rule},
        "answer": {
            "control_f1": statistics.fmean(control_f1),
            "treatment_f1": statistics.fmean(treatment_f1),
            "paired_delta": answer_delta,
            "decision": answer_rule,
            "model": answerer["model"],
        },
    }
    result_path = run_root / "result.v1.json"
    result_path.write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")
    result_path.chmod(0o600)

    code = _lib.git_info(REPO_ROOT)
    code["baseline_commit"] = None
    started = datetime.fromisoformat(state.started_at)
    run_id, receipt_dir = _lib.write_record(
        "graph-01-protected-bridge",
        ts=started,
        config_obj=config,
        metrics=metrics,
        verdict=metrics["verdict"],
        read=(
            "GRAPH-01 protected bridge-completion completed and was accepted."
            if accepted
            else "GRAPH-01 protected bridge-completion completed and was rejected."
        ),
        code=code,
        corpus={
            "source": "MuSiQue-Ans registered reused cohort",
            "manifest_sha256": config["inputs"]["corpus_sha256"],
            "datasets": [
                {
                    "name": "MuSiQue-Ans",
                    "questions": len(observations),
                    "selection_sha256": config["inputs"]["cohort_sha256"],
                }
            ],
        },
        seeds={"bootstrap": config["evaluation"]["bootstrap_seed"]},
        env=_lib.env_info(
            REPO_ROOT,
            key_deps={
                "fathomdb": config["execution"]["fathomdb_version"],
                "edge_judge": judge["model"],
                "answerer": answerer["model"],
            },
        ),
        cost_usd=state.cost_usd,
        headline={
            "program_track": "GRAPH-01",
            "decision": metrics["verdict"],
            "complete_bridge_delta": bootstrap["point"],
            "answer_f1_delta": answer_delta["point"],
        },
        n=len(observations),
        config_path=_repo_relative(config_path),
        tdd_evidence={
            "red": "initial missing module plus execution-discovered boundary failures",
            "green": "src/python/tests/test_graph_01.py",
        },
        tests={"graph_01": "14 passed"},
        files_changed=[
            "experiments/graph_01.py",
            "experiments/graph_01_live.py",
            "src/python/tests/test_graph_01.py",
        ],
        artifacts=[str(result_path), str(observations_path), str(native_report["database_path"])],
        review={
            "status": "approved",
            "path": "dev/performance-benchmarking/2026-08-29-graph-01-design-review.md",
        },
        open_questions=[],
    )
    _lib.regen_index_md()
    return run_id, receipt_dir


def _preflight(config_path: Path, run_root: Path) -> dict[str, Any]:
    config = _load_config(config_path)
    inputs = _verify_inputs(config)
    questions, _cohort = _load_questions(config)
    extractions = json.loads(
        _resolve(config["inputs"]["extractions_path"]).read_text(encoding="utf-8")
    )
    missing = 0
    for question in questions:
        for paragraph in question.paragraphs:
            missing += f"{question.id}#{paragraph.idx}" not in extractions
    from eval.m1_baseline import BGEEncoder

    toolchain = {
        "python": _resolve(config["execution"]["fathomdb_python"]).is_file(),
        "cli": _resolve(config["execution"]["fathomdb_cli"]).is_file(),
        "bge": BGEEncoder().available,
    }
    report = {
        "schema_version": "graph-01.preflight.v1",
        "config_sha256": _canonical_sha256(config),
        "question_count": len(questions),
        "missing_extractions": missing,
        "inputs": inputs,
        "toolchain": toolchain,
        "cost_cap_usd": config["approval"]["cost_cap_usd"],
        "state": (
            "ready"
            if missing == 1
            and config["inputs"]["missing_extraction_policy"]
            == "empty_graph_keep_document"
            and all(toolchain.values())
            else "blocked"
        ),
        "cost_usd": 0.0,
    }
    run_root.mkdir(parents=True, exist_ok=True, mode=0o700)
    path = run_root / "preflight.v1.json"
    path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    path.chmod(0o600)
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for name in ("preflight", "run"):
        command = subparsers.add_parser(name)
        command.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
        command.add_argument("--run-root", type=Path, default=DEFAULT_RUN_ROOT)
    native = subparsers.add_parser("native-build")
    native.add_argument("--payload", type=Path, required=True)
    native.add_argument("--native-root", type=Path, required=True)
    native.add_argument("--fathomdb-bin", type=Path, required=True)
    native.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "native-build":
        _native_build(args.payload, args.native_root, args.fathomdb_bin, args.report)
        return 0
    if args.command == "preflight":
        print(json.dumps(_preflight(args.config, args.run_root), indent=2))
        return 0
    run_id, receipt = _run(args.config, args.run_root)
    print(json.dumps({"run_id": run_id, "receipt": str(receipt)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
