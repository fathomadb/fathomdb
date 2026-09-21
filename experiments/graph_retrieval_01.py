"""GRAPH-RETRIEVAL-01 immutable seeds, candidate policy, and scorers."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Sequence

from experiments import graph_01, graph_benchmarks
from experiments.fathomdb_test_setup import prepare_test_database


SCHEMA_VERSION = "graph-retrieval-01.config.v1"
SEED_SCHEMA_VERSION = "graph-retrieval-01.seed-manifest.v1"
PROGRAM_TRACK = "GRAPH-RETRIEVAL-01"
_ROOT_KEYS = {
    "schema_version",
    "program_track",
    "cell_id",
    "claim_boundary",
    "inputs",
    "model",
    "arms",
    "policy",
    "bootstrap",
    "measurement",
    "outputs",
    "stark",
}


class GraphRetrievalBenchmarkError(ValueError):
    """Report an invalid retrieval benchmark contract or observation."""


@dataclass(frozen=True)
class Config:
    """Resolved GRAPH-RETRIEVAL-01 contract."""

    program_track: str
    cell_id: str
    inputs: dict[str, object]
    model: dict[str, object]
    arms: tuple[str, ...]
    policy: dict[str, object]
    bootstrap: dict[str, int]
    measurement: dict[str, object]
    resolved: dict[str, object]


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    try:
        return graph_benchmarks.exact_mapping(value, label, keys)
    except graph_benchmarks.GraphBenchmarkError as error:
        raise GraphRetrievalBenchmarkError(str(error)) from error


def resolve_config(document: object) -> Config:
    """Strictly validate and resolve the native-expansion retrieval contract."""
    root = _exact(document, "config", _ROOT_KEYS)
    if (
        root["schema_version"] != SCHEMA_VERSION
        or root["program_track"] != PROGRAM_TRACK
    ):
        raise GraphRetrievalBenchmarkError("configuration identity is invalid")
    if root["cell_id"] != "graph-retrieval01":
        raise GraphRetrievalBenchmarkError("cell_id is invalid")
    if root["claim_boundary"] != "historical_300_directional_native_expand_retrieval":
        raise GraphRetrievalBenchmarkError("claim boundary drifted")
    inputs = _exact(
        root["inputs"],
        "inputs",
        {
            "cohort",
            "corpus_sha256",
            "extractions_sha256",
            "cohort_sha256",
            "graph01_config_sha256",
            "historical_observations_sha256",
            "seed_manifest_sha256",
        },
    )
    if inputs["cohort"] != "historical-300":
        raise GraphRetrievalBenchmarkError("cohort identity drifted")
    for key in (
        "corpus_sha256",
        "extractions_sha256",
        "cohort_sha256",
        "graph01_config_sha256",
        "historical_observations_sha256",
    ):
        value = inputs[key]
        if not isinstance(value, str) or len(value) != 64:
            raise GraphRetrievalBenchmarkError(f"inputs.{key} is invalid")
    seed_sha = inputs["seed_manifest_sha256"]
    if seed_sha is not None and (not isinstance(seed_sha, str) or len(seed_sha) != 64):
        raise GraphRetrievalBenchmarkError("inputs.seed_manifest_sha256 is invalid")
    model = _exact(
        root["model"],
        "model",
        {
            "repository",
            "revision",
            "pooling",
            "normalization",
            "device_policy",
            "files",
        },
    )
    if (
        model["repository"] != "BAAI/bge-small-en-v1.5"
        or model["revision"] != "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"
        or model["pooling"] != "cls"
        or model["normalization"] != "l2"
        or model["device_policy"] != "cuda-rtx3090-only"
    ):
        raise GraphRetrievalBenchmarkError("model identity drifted")
    files = _exact(
        model["files"],
        "model.files",
        {"config.json", "tokenizer.json", "model.safetensors"},
    )
    if any(not isinstance(value, str) or len(value) != 64 for value in files.values()):
        raise GraphRetrievalBenchmarkError("model file identity drifted")
    arms_value = root["arms"]
    expected_arms = ["fused_rrf_k60", "native_expand_depth1", "native_expand_depth2"]
    if arms_value != expected_arms:
        raise GraphRetrievalBenchmarkError("arm identity drifted")
    policy = _exact(
        root["policy"],
        "policy",
        {
            "evaluation_depth",
            "context_depth",
            "protected_prefix",
            "promotion_limit",
            "deduplicate_by",
            "tie_break",
        },
    )
    expected_policy = {
        "evaluation_depth": 20,
        "context_depth": 10,
        "protected_prefix": 8,
        "promotion_limit": 2,
        "deduplicate_by": "paragraph_id",
        "tie_break": ["hop", "seed_rank", "passage_id"],
    }
    if policy != expected_policy:
        raise GraphRetrievalBenchmarkError("policy identity drifted")
    bootstrap = _exact(root["bootstrap"], "bootstrap", {"draws", "seed"})
    if bootstrap != {"draws": 2000, "seed": 20260921}:
        raise GraphRetrievalBenchmarkError("bootstrap identity drifted")
    measurement = _exact(
        root["measurement"], "measurement", {"repetitions", "percentile_estimator"}
    )
    if measurement != {"repetitions": 3, "percentile_estimator": "linear-p-n-minus-1"}:
        raise GraphRetrievalBenchmarkError("measurement identity drifted")
    outputs = _exact(
        root["outputs"], "outputs", {"retrieval_section", "performance_section"}
    )
    if outputs != {
        "retrieval_section": "retrieval_quality",
        "performance_section": "performance",
    }:
        raise GraphRetrievalBenchmarkError("output sections drifted")
    stark = _exact(root["stark"], "stark", {"state", "numeric_tolerance"})
    if stark != {"state": "blocked_prerequisite", "numeric_tolerance": 1e-12}:
        raise GraphRetrievalBenchmarkError("STaRK blocker identity drifted")
    return Config(
        program_track=PROGRAM_TRACK,
        cell_id="graph-retrieval01",
        inputs=dict(inputs),
        model={**model, "files": dict(files)},
        arms=tuple(expected_arms),
        policy=dict(policy),
        bootstrap={"draws": 2000, "seed": 20260921},
        measurement=dict(measurement),
        resolved=dict(root),
    )


def load_config(path: str | Path) -> Config:
    """Load and validate one checked-in retrieval configuration."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GraphRetrievalBenchmarkError(
            "configuration is unavailable or invalid"
        ) from error
    return resolve_config(document)


def validate_seed_manifest(document: object) -> dict[str, object]:
    """Validate the external GPU-qualified top-20 seed manifest."""
    root = _exact(
        document,
        "seed manifest",
        {
            "schema_version",
            "cohort",
            "question_count",
            "ranking_depth",
            "model",
            "historical_top10_parity",
            "rankings",
            "rankings_sha256",
            "materializer_sha256",
        },
    )
    if (
        root["schema_version"] != SEED_SCHEMA_VERSION
        or root["cohort"] != "historical-300"
    ):
        raise GraphRetrievalBenchmarkError("seed manifest identity drifted")
    if root["ranking_depth"] != 20:
        raise GraphRetrievalBenchmarkError("seed manifest ranking depth drifted")
    model = _exact(
        root["model"],
        "seed manifest model",
        {"repository", "revision", "pooling", "device", "gpu_uuid"},
    )
    if model["device"] != "cuda" or not isinstance(model["gpu_uuid"], str):
        raise GraphRetrievalBenchmarkError(
            "seed manifest must use CUDA with a pinned GPU UUID"
        )
    if (
        model["repository"] != "BAAI/bge-small-en-v1.5"
        or model["revision"] != "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"
        or model["pooling"] != "cls"
    ):
        raise GraphRetrievalBenchmarkError("seed manifest model identity drifted")
    parity = _exact(
        root["historical_top10_parity"], "historical parity", {"matched", "required"}
    )
    if (
        parity["matched"] != parity["required"]
        or parity["required"] != root["question_count"]
    ):
        raise GraphRetrievalBenchmarkError("historical top-10 parity is incomplete")
    rankings = root["rankings"]
    if not isinstance(rankings, list) or len(rankings) != root["question_count"]:
        raise GraphRetrievalBenchmarkError("seed manifest question count drifted")
    question_ids: set[str] = set()
    for item in rankings:
        row = _exact(item, "seed ranking", {"question_id", "passage_ids"})
        question_id = row["question_id"]
        passage_ids = row["passage_ids"]
        if (
            not isinstance(question_id, str)
            or question_id in question_ids
            or not isinstance(passage_ids, list)
            or len(passage_ids) != 20
            or len(set(passage_ids)) != 20
            or any(not isinstance(value, str) for value in passage_ids)
        ):
            raise GraphRetrievalBenchmarkError("seed ranking is invalid")
        question_ids.add(question_id)
    if graph_benchmarks.canonical_sha256(rankings) != root["rankings_sha256"]:
        raise GraphRetrievalBenchmarkError("seed manifest rankings digest drifted")
    materializer = root["materializer_sha256"]
    if not isinstance(materializer, str) or len(materializer) != 64:
        raise GraphRetrievalBenchmarkError("materializer identity is invalid")
    return dict(root)


class HistoricalCudaBgeEncoder:
    """Run the retained 0.8.2 BGE math on CUDA for ranking continuity."""

    hidden_size = 384
    attention_heads = 12
    layers = 12
    layer_norm_epsilon = 1e-12

    def __init__(self, model_cache: Path, device: str) -> None:
        try:
            import torch
            from safetensors.torch import load_file
            from tokenizers import Tokenizer
        except ImportError as error:
            raise GraphRetrievalBenchmarkError(
                "historical CUDA encoder dependencies are unavailable"
            ) from error
        if not device.startswith("cuda:") or not torch.cuda.is_available():
            raise GraphRetrievalBenchmarkError(
                "historical encoder requires CUDA; refusing CPU fallback"
            )
        torch.backends.cuda.matmul.allow_tf32 = False
        torch.backends.cudnn.allow_tf32 = False
        torch.set_float32_matmul_precision("highest")
        self._torch = torch
        self._device = torch.device(device)
        self._weights = load_file(
            str(model_cache / "model.safetensors"), device=device
        )
        self._tokenizer = Tokenizer.from_file(str(model_cache / "tokenizer.json"))

    def _layer_norm(self, value: Any, weight: Any, bias: Any) -> Any:
        mean = value.mean(dim=-1, keepdim=True)
        variance = value.var(dim=-1, correction=0, keepdim=True)
        return (
            (value - mean)
            / self._torch.sqrt(variance + self.layer_norm_epsilon)
            * weight
            + bias
        )

    def _linear(self, value: Any, name: str) -> Any:
        return (
            value @ self._weights[name + ".weight"].transpose(0, 1)
            + self._weights[name + ".bias"]
        )

    def _gelu(self, value: Any) -> Any:
        return 0.5 * value * (
            1
            + self._torch.tanh(
                math.sqrt(2 / math.pi) * (value + 0.044715 * value**3)
            )
        )

    def _encode(self, text: str) -> list[float]:
        torch = self._torch
        token_ids = self._tokenizer.encode(text).ids[:512]
        ids = torch.tensor(token_ids, dtype=torch.long, device=self._device)
        weights = self._weights
        with torch.inference_mode():
            value = weights["embeddings.word_embeddings.weight"][ids]
            value = value + weights["embeddings.position_embeddings.weight"][: len(ids)]
            value = value + weights["embeddings.token_type_embeddings.weight"][0]
            value = self._layer_norm(
                value,
                weights["embeddings.LayerNorm.weight"],
                weights["embeddings.LayerNorm.bias"],
            )
            token_count = len(ids)
            for layer in range(self.layers):
                prefix = f"encoder.layer.{layer}."
                query = (
                    self._linear(value, prefix + "attention.self.query")
                    .reshape(token_count, self.attention_heads, -1)
                    .transpose(0, 1)
                )
                key = (
                    self._linear(value, prefix + "attention.self.key")
                    .reshape(token_count, self.attention_heads, -1)
                    .transpose(0, 1)
                )
                projected = (
                    self._linear(value, prefix + "attention.self.value")
                    .reshape(token_count, self.attention_heads, -1)
                    .transpose(0, 1)
                )
                scores = query @ key.transpose(1, 2) / math.sqrt(query.shape[-1])
                scores = scores - scores.max(dim=-1, keepdim=True).values
                attention = self._torch.exp(scores)
                attention = attention / attention.sum(dim=-1, keepdim=True)
                context = (
                    (attention @ projected)
                    .transpose(0, 1)
                    .reshape(token_count, self.hidden_size)
                )
                attention_output = self._linear(
                    context, prefix + "attention.output.dense"
                )
                value = self._layer_norm(
                    value + attention_output,
                    weights[prefix + "attention.output.LayerNorm.weight"],
                    weights[prefix + "attention.output.LayerNorm.bias"],
                )
                intermediate = self._gelu(
                    self._linear(value, prefix + "intermediate.dense")
                )
                output = self._linear(intermediate, prefix + "output.dense")
                value = self._layer_norm(
                    value + output,
                    weights[prefix + "output.LayerNorm.weight"],
                    weights[prefix + "output.LayerNorm.bias"],
                )
            cls = value[0]
            normalized = cls / (torch.linalg.vector_norm(cls) + 1e-9)
        return normalized.cpu().tolist()

    def embed_batch(self, texts: list[str]) -> list[list[float]]:
        """Embed in historical one-text call order while computing on CUDA."""
        return [self._encode(text) for text in texts]


def _fused_seed_ranking(
    question: Any,
    embed_batch_cls: Callable[[list[str]], list[list[float]]],
) -> list[int]:
    """Reproduce the historical CLS-pooled BM25+dense RRF ranking."""
    import numpy as np
    from eval.m1_baseline import bm25_rank, rrf_fuse

    texts = [
        "Represent this sentence for searching relevant passages: "
        + question.question,
        *[paragraph.body for paragraph in question.paragraphs],
    ]
    vectors = np.asarray(embed_batch_cls(texts), dtype=np.float32)
    if vectors.ndim != 2 or vectors.shape[0] != len(texts):
        raise GraphRetrievalBenchmarkError("CLS batch embedding shape is invalid")
    dense = [
        int(index)
        for index in np.argsort(
            -(vectors[1:] @ vectors[0]),
            kind="stable",
        )
    ]
    return rrf_fuse([bm25_rank(question.question, question.paragraphs), dense], k=60)


def _historical_source_revision_ids(
    question_id: str, paragraph_idx: int, body: str
) -> tuple[str, str]:
    identity = hashlib.sha256(
        f"{question_id}|{paragraph_idx}|{body}".encode()
    ).hexdigest()[:24]
    return f"source-{identity}", f"version-{identity}"


def materialize_seed_manifest(
    config: Config,
    *,
    corpus_path: Path,
    cohort_path: Path,
    historical_observations_path: Path,
    model_cache: Path,
    artifact_root: Path,
    output_path: Path,
    fathomdb_bin: Path,
    cuda_uuid: str,
) -> dict[str, object]:
    """Materialize canonical top-20 seeds with the pinned CUDA embedder."""
    import fathomdb
    from eval.m1_baseline import load_musique

    paths = {
        "corpus_sha256": corpus_path,
        "cohort_sha256": cohort_path,
        "historical_observations_sha256": historical_observations_path,
    }
    for key, path in paths.items():
        if graph_benchmarks.sha256_file(path) != config.inputs[key]:
            raise GraphRetrievalBenchmarkError(f"{key} identity drifted")
    for filename, digest in config.model["files"].items():
        if graph_benchmarks.sha256_file(model_cache / filename) != digest:
            raise GraphRetrievalBenchmarkError(
                f"model file identity drifted: {filename}"
            )
    visible = subprocess.run(
        ["nvidia-smi", "--query-gpu=uuid,name", "--format=csv,noheader"],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
    )
    matching = [
        line
        for line in visible.stdout.splitlines()
        if line.startswith(cuda_uuid + ",") and "RTX 3090" in line
    ]
    if visible.returncode or len(matching) != 1:
        raise GraphRetrievalBenchmarkError(
            "pinned RTX 3090 CUDA UUID is unavailable; refusing CPU fallback"
        )
    questions = load_musique(corpus_path)
    by_id = {question.id: question for question in questions}
    cohort = json.loads(cohort_path.read_text())
    cohort_rows = cohort.get("baseline_run", {}).get("paired_records")
    if not isinstance(cohort_rows, list) or len(cohort_rows) != 300:
        raise GraphRetrievalBenchmarkError("historical cohort is invalid")
    observations = json.loads(historical_observations_path.read_text()).get("rows")
    if not isinstance(observations, list) or len(observations) != 300:
        raise GraphRetrievalBenchmarkError("historical top-10 observations are invalid")
    historical = {str(row["question_id"]): row["control"] for row in observations}
    prior_visible = os.environ.get("CUDA_VISIBLE_DEVICES")
    prior_device = os.environ.get("FATHOMDB_EMBED_DEVICE")
    os.environ["CUDA_VISIBLE_DEVICES"] = cuda_uuid
    os.environ["FATHOMDB_EMBED_DEVICE"] = "cuda:0"
    try:
        prepared = prepare_test_database(
            artifact_root,
            test_id="graph-retrieval-seeds",
            embed_device="cuda:0",
            rerank_device="cpu",
            embedder="default",
            warm_cache=True,
            check_reranker=False,
            fathomdb_bin=str(fathomdb_bin),
        )
        engine = fathomdb.Engine.open(
            str(prepared.database_path), use_default_embedder=True
        )
        try:
            report = engine.open_report()
            resolution = report.embedder_device_resolution
            if (
                resolution is None
                or resolution.effective_device.kind != "cuda"
                or resolution.selected_cuda_uuid != cuda_uuid
            ):
                raise GraphRetrievalBenchmarkError(
                    "embedder did not select the pinned CUDA UUID"
                )
            historical_encoder = HistoricalCudaBgeEncoder(model_cache, "cuda:0")
            rankings = []
            for cohort_row in cohort_rows:
                question_id = str(cohort_row["qid"])
                question = by_id.get(question_id)
                if question is None:
                    raise GraphRetrievalBenchmarkError("cohort question is absent")
                fused = _fused_seed_ranking(question, historical_encoder.embed_batch)
                if fused[:10] != historical.get(question_id):
                    raise GraphRetrievalBenchmarkError(
                        f"historical top-10 parity failed for {question_id}"
                    )
                rankings.append(
                    {
                        "question_id": question_id,
                        "passage_ids": [str(value) for value in fused[:20]],
                    }
                )
        finally:
            engine.close()
    finally:
        if prior_visible is None:
            os.environ.pop("CUDA_VISIBLE_DEVICES", None)
        else:
            os.environ["CUDA_VISIBLE_DEVICES"] = prior_visible
        if prior_device is None:
            os.environ.pop("FATHOMDB_EMBED_DEVICE", None)
        else:
            os.environ["FATHOMDB_EMBED_DEVICE"] = prior_device
    manifest = {
        "schema_version": SEED_SCHEMA_VERSION,
        "cohort": "historical-300",
        "question_count": 300,
        "ranking_depth": 20,
        "model": {
            "repository": config.model["repository"],
            "revision": config.model["revision"],
            "pooling": config.model["pooling"],
            "device": "cuda",
            "gpu_uuid": cuda_uuid,
        },
        "historical_top10_parity": {"matched": 300, "required": 300},
        "rankings": rankings,
        "rankings_sha256": graph_benchmarks.canonical_sha256(rankings),
        "materializer_sha256": graph_benchmarks.sha256_file(__file__),
    }
    validate_seed_manifest(manifest)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return manifest


def promote_candidates(
    baseline: Sequence[str],
    candidates: Sequence[dict[str, object]],
    *,
    protected_prefix: int,
    promotion_limit: int,
    evaluation_depth: int,
) -> list[str]:
    """Apply the frozen deterministic graph-candidate promotion policy."""
    if (
        len(baseline) < evaluation_depth
        or len(set(baseline[:evaluation_depth])) != evaluation_depth
    ):
        raise GraphRetrievalBenchmarkError(
            "baseline ranking is too short or contains duplicates"
        )
    if not 0 <= protected_prefix <= evaluation_depth or promotion_limit < 0:
        raise GraphRetrievalBenchmarkError("candidate policy bounds are invalid")
    best: dict[str, tuple[int, int, str]] = {}
    baseline_set = set(baseline[:evaluation_depth])
    for item in candidates:
        passage_id = item.get("passage_id")
        hop = item.get("hop")
        seed_rank = item.get("seed_rank")
        if (
            not isinstance(passage_id, str)
            or isinstance(hop, bool)
            or not isinstance(hop, int)
            or isinstance(seed_rank, bool)
            or not isinstance(seed_rank, int)
        ):
            raise GraphRetrievalBenchmarkError("candidate is invalid")
        if passage_id in baseline_set:
            continue
        key = (hop, seed_rank, passage_id)
        if passage_id not in best or key < best[passage_id]:
            best[passage_id] = key
    promoted = [item[2] for item in sorted(best.values())[:promotion_limit]]
    ranking = list(baseline[:protected_prefix]) + promoted
    for passage_id in baseline:
        if passage_id not in ranking:
            ranking.append(passage_id)
        if len(ranking) == evaluation_depth:
            break
    if len(ranking) != evaluation_depth or len(set(ranking)) != evaluation_depth:
        raise GraphRetrievalBenchmarkError(
            "candidate policy did not preserve the fixed budget"
        )
    return ranking


def stark_qualification_state() -> dict[str, object]:
    """Return the registered blocker without manufacturing a STaRK score."""
    return {
        "state": "blocked_prerequisite",
        "benchmark": "STaRK",
        "reason": (
            "complete official knowledge base, split, mappings, baseline, and "
            "evaluator-parity manifest are not qualified"
        ),
        "numeric_tolerance": 1e-12,
        "score": None,
    }


def _metrics(
    ranking: Sequence[str], gold: set[str], cutoffs: Sequence[int]
) -> dict[str, float]:
    result: dict[str, float] = {}
    for cutoff in cutoffs:
        prefix = ranking[:cutoff]
        hits = sum(item in gold for item in prefix)
        result[f"complete_support_at_{cutoff}"] = float(
            bool(gold) and gold.issubset(prefix)
        )
        result[f"support_recall_at_{cutoff}"] = hits / len(gold) if gold else 0.0
        result[f"support_precision_at_{cutoff}"] = hits / cutoff
        dcg = sum(
            (1.0 / math.log2(index + 2))
            for index, item in enumerate(prefix)
            if item in gold
        )
        ideal = sum(
            1.0 / math.log2(index + 2) for index in range(min(len(gold), cutoff))
        )
        result[f"ndcg_at_{cutoff}"] = dcg / ideal if ideal else 0.0
    first = next((index for index, item in enumerate(ranking, 1) if item in gold), None)
    result["first_support_mrr"] = 0.0 if first is None else 1.0 / first
    return result


def _mean_rows(rows: Sequence[dict[str, float]]) -> dict[str, float]:
    keys = rows[0].keys()
    return {key: sum(row[key] for row in rows) / len(rows) for key in keys}


def score_rankings(
    rows: Sequence[dict[str, object]],
    *,
    cutoffs: tuple[int, ...],
    draws: int,
    seed: int,
) -> dict[str, object]:
    """Score paired control/treatment rankings overall and by hop count."""
    if not rows or not cutoffs:
        raise GraphRetrievalBenchmarkError("retrieval observations are empty")
    scored: list[dict[str, object]] = []
    for row in rows:
        question_id = row.get("question_id")
        hop = row.get("hop_count")
        gold_value = row.get("gold")
        control = row.get("control")
        treatment = row.get("treatment")
        if (
            not isinstance(question_id, str)
            or isinstance(hop, bool)
            or not isinstance(hop, int)
            or not isinstance(gold_value, list)
            or not isinstance(control, list)
            or not isinstance(treatment, list)
        ):
            raise GraphRetrievalBenchmarkError("retrieval observation is invalid")
        gold = {str(value) for value in gold_value}
        control_metrics = _metrics(control, gold, cutoffs)
        treatment_metrics = _metrics(treatment, gold, cutoffs)
        scored.append(
            {
                "question_id": question_id,
                "hop": hop,
                "gold": gold,
                "control": control,
                "treatment": treatment,
                "control_metrics": control_metrics,
                "treatment_metrics": treatment_metrics,
            }
        )
    overall = _mean_rows([item["treatment_metrics"] for item in scored])
    overall.update(
        {
            "candidate_change_rate": sum(
                item["control"] != item["treatment"] for item in scored
            )
            / len(scored),
            "useful_expansion_rate": sum(
                bool(item["gold"] - set(item["control"]))
                and bool((item["gold"] - set(item["control"])) & set(item["treatment"]))
                for item in scored
            )
            / len(scored),
            "harmful_displacement_rate": sum(
                bool((item["gold"] & set(item["control"])) - set(item["treatment"]))
                for item in scored
            )
            / len(scored),
            "no_op_expansion_rate": sum(
                item["control"] == item["treatment"] for item in scored
            )
            / len(scored),
        }
    )
    by_hop: dict[str, object] = {}
    for hop in sorted({int(item["hop"]) for item in scored}):
        subset = [item["treatment_metrics"] for item in scored if item["hop"] == hop]
        by_hop[str(hop)] = _mean_rows(subset)
    paired: dict[str, object] = {}
    for cutoff in cutoffs:
        key = f"complete_support_at_{cutoff}"
        paired[key] = graph_01.bootstrap_paired_mean(
            [item["control_metrics"][key] for item in scored],
            [item["treatment_metrics"][key] for item in scored],
            draws=draws,
            seed=seed,
        )
    denominators = graph_benchmarks.reconcile_denominators(
        attempted=len(scored), completed=len(scored), errors=0, typed_refusals=0
    )
    return {
        "retrieval_quality": {"overall": overall, "by_hop": by_hop},
        "paired_deltas": paired,
        "denominators": denominators,
    }


def run_smoke(
    config: Config, artifact_root: Path, *, config_path: Path
) -> dict[str, object]:
    """Exercise native expansion/evidence before applying retrieval promotion."""
    import fathomdb

    fathomdb_bin = os.environ.get(
        "FATHOMDB_BIN",
        str(Path(__file__).resolve().parent.parent / "target/debug/fathomdb"),
    )
    prepared = prepare_test_database(
        artifact_root,
        test_id="graph-retrieval01",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=fathomdb_bin,
    )
    source_body = "bridge passage with the missing supporting fact"
    source_digest = hashlib.sha256(source_body.encode()).hexdigest()

    def derived(revision: str) -> dict[str, object]:
        return {
            "schema_version": 1,
            "role": "derived",
            "artifact_revision_id": revision,
            "source_version_id": "source-v1",
            "source_revision_id": "source-r1",
            "source_locator": {"kind": "whole_body"},
            "canonical_source_hash": {
                "algorithm": "sha256",
                "digest_hex": source_digest,
            },
        }

    engine = fathomdb.Engine.open(
        str(prepared.database_path), use_default_embedder=False
    )
    latencies: list[float] = []
    try:
        engine.write(
            [
                {
                    "kind": "document",
                    "body": source_body,
                    "source_id": "bridge",
                    "logical_id": "bridge",
                    "provenance": {
                        "schema_version": 1,
                        "role": "canonical",
                        "artifact_revision_id": "source-r1",
                        "source_version_id": "source-v1",
                    },
                },
                {
                    "kind": "entity",
                    "body": "seed entity",
                    "source_id": "bridge",
                    "logical_id": "seed-entity",
                    "provenance": derived("seed-r1"),
                },
                {
                    "kind": "entity",
                    "body": "bridge entity",
                    "source_id": "bridge",
                    "logical_id": "bridge-entity",
                    "provenance": derived("bridge-r1"),
                },
                {
                    "edge": {
                        "kind": "related",
                        "from": "seed-entity",
                        "to": "bridge-entity",
                        "source_id": "bridge",
                        "logical_id": "bridge-edge",
                        "provenance": derived("edge-r1"),
                    }
                },
            ]
        )
        engine.drain(timeout_s=30)
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        request = fathomdb.GraphExpandRequestV1(
            schema_version=1,
            seed=fathomdb.GraphExplicitSeedV1(
                schema_version=1,
                type="explicit",
                logical_ids=(fathomdb.IdSpace(space="logical", value="seed-entity"),),
            ),
            direction="outgoing",
            edge_kinds=("related",),
            target_kinds=("entity",),
            context=fathomdb.FrozenGraphReadContextV1(
                schema_version=1, type="frozen", context=frozen
            ),
            max_depth=1,
            result_limit=10,
            max_work_units="10",
            include_explanation=False,
            include_evidence=True,
        )
        result = None
        for _ in range(int(config.measurement["repetitions"])):
            started = time.perf_counter_ns()
            result = fathomdb.graph.expand(engine, request)
            latencies.append((time.perf_counter_ns() - started) / 1_000_000)
        assert result is not None
        if result.evidence is None or len(result.evidence.entries) != 1:
            raise GraphRetrievalBenchmarkError(
                "native expansion evidence is unavailable"
            )
        resolved = engine.resolve_graph_evidence(
            fathomdb.GraphEvidenceResolveRequestV1(
                evidence_ref=result.evidence.entries[0].target_evidence_ref,
                context=frozen,
            )
        )
        resolved_hash = resolved.canonical_source_hash
        resolved_digest = (
            resolved_hash["digest_hex"]
            if isinstance(resolved_hash, dict)
            else resolved_hash.digest_hex
        )
        if resolved_digest != source_digest:
            raise GraphRetrievalBenchmarkError(
                "native candidate source identity drifted"
            )
        candidates = [
            {
                "passage_id": resolved.source_id,
                "hop": result.targets[0].origin.hop_count,
                "seed_rank": result.targets[0].origin.seed_ordinal,
            }
        ]
    finally:
        engine.close()
    baseline = [f"p{index:02d}" for index in range(20)]
    treatment = promote_candidates(
        baseline,
        candidates,
        protected_prefix=int(config.policy["protected_prefix"]),
        promotion_limit=int(config.policy["promotion_limit"]),
        evaluation_depth=int(config.policy["evaluation_depth"]),
    )
    metrics = score_rankings(
        [
            {
                "question_id": "smoke-q1",
                "hop_count": 2,
                "gold": ["p00", "bridge"],
                "control": baseline,
                "treatment": treatment,
            }
        ],
        cutoffs=(5, 10, 20),
        draws=20,
        seed=7,
    )
    metrics["performance"] = {
        "native_expand_latency_ms": {
            "p50": graph_benchmarks.percentile(latencies, 0.50),
            "p95": graph_benchmarks.percentile(latencies, 0.95),
            "p99": graph_benchmarks.percentile(latencies, 0.99),
        },
        "operations": len(latencies),
    }
    output = artifact_root / "metrics.json"
    output.write_text(
        json.dumps(metrics, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    record = graph_benchmarks.write_benchmark_record(
        experiment=config.cell_id,
        config=config.resolved,
        metrics=metrics,
        base_dir=artifact_root,
        config_path=config_path,
        corpus_manifest_sha256=source_digest,
        datasets=["graph-retrieval-native-smoke"],
    )
    return {"state": "complete", "metrics": str(output), "record": str(record)}


def run_historical(
    config: Config,
    *,
    corpus_path: Path,
    extractions_path: Path,
    cohort_path: Path,
    seed_manifest_path: Path,
    artifact_root: Path,
    config_path: Path,
    fathomdb_bin: Path,
) -> dict[str, object]:
    """Run the registered historical cohort through native graph expansion."""
    import fathomdb
    from eval.m1_baseline import load_musique

    for key, path in (
        ("corpus_sha256", corpus_path),
        ("extractions_sha256", extractions_path),
        ("cohort_sha256", cohort_path),
    ):
        if graph_benchmarks.sha256_file(path) != config.inputs[key]:
            raise GraphRetrievalBenchmarkError(f"{key} identity drifted")
    seed_digest = graph_benchmarks.sha256_file(seed_manifest_path)
    if config.inputs["seed_manifest_sha256"] != seed_digest:
        raise GraphRetrievalBenchmarkError(
            "qualified seed manifest is absent or does not match the config"
        )
    seed_manifest = validate_seed_manifest(json.loads(seed_manifest_path.read_text()))
    questions = load_musique(corpus_path)
    by_id = {question.id: question for question in questions}
    cohort = json.loads(cohort_path.read_text())
    cohort_rows = cohort.get("baseline_run", {}).get("paired_records")
    if not isinstance(cohort_rows, list) or len(cohort_rows) != 300:
        raise GraphRetrievalBenchmarkError("historical cohort is invalid")
    selected = []
    for row in cohort_rows:
        question = by_id.get(str(row.get("qid")))
        if question is None:
            raise GraphRetrievalBenchmarkError("cohort question is absent")
        selected.append(question)
    extractions = json.loads(extractions_path.read_text())
    if not isinstance(extractions, dict):
        raise GraphRetrievalBenchmarkError("extractions must be an object")
    seed_by_question = {
        str(row["question_id"]): list(row["passage_ids"])
        for row in seed_manifest["rankings"]
    }
    if set(seed_by_question) != {question.id for question in selected}:
        raise GraphRetrievalBenchmarkError("seed manifest question set drifted")

    prepared = prepare_test_database(
        artifact_root,
        test_id="graph-retrieval-historical",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=str(fathomdb_bin),
    )
    engine = fathomdb.Engine.open(
        str(prepared.database_path), use_default_embedder=False
    )
    memberships: dict[str, dict[int, set[str]]] = {}
    try:
        batch: list[dict[str, object]] = []
        for question in selected:
            paragraphs = [
                {
                    "idx": paragraph.idx,
                    "title": paragraph.title,
                    "text": paragraph.text,
                }
                for paragraph in question.paragraphs
            ]
            edges, _ = graph_01.admit_relations(
                question.id,
                paragraphs,
                extractions,
                allow_missing_empty=True,
            )
            membership = graph_01.paragraph_entity_membership(
                question.id, paragraphs, extractions, allow_missing_empty=True
            )
            memberships[question.id] = membership
            paragraph_by_idx = {
                paragraph.idx: paragraph for paragraph in question.paragraphs
            }
            source_meta: dict[int, tuple[str, str, str]] = {}
            for idx, paragraph in paragraph_by_idx.items():
                body = f"{paragraph.title}\n{paragraph.text}"
                source_revision, source_version = _historical_source_revision_ids(
                    question.id, idx, body
                )
                source_meta[idx] = (
                    body,
                    source_revision,
                    source_version,
                )
                batch.append(
                    {
                        "kind": "doc",
                        "body": body,
                        "logical_id": f"{question.id}#{idx}",
                        "source_id": f"{question.id}#{idx}",
                        "provenance": {
                            "schema_version": 1,
                            "role": "canonical",
                            "artifact_revision_id": source_meta[idx][1],
                            "source_version_id": source_meta[idx][2],
                        },
                    }
                )
            first_source: dict[str, int] = {}
            display: dict[str, str] = {}
            for idx in sorted(membership):
                extraction = extractions.get(f"{question.id}#{idx}") or {"entities": []}
                for entity in extraction.get("entities", []):
                    normalized = graph_01.normalize_entity(str(entity["name"]))
                    first_source.setdefault(normalized, idx)
                    display.setdefault(
                        normalized, " ".join(str(entity["name"]).split())
                    )

            def provenance(idx: int, revision: str) -> dict[str, object]:
                body, source_revision, version = source_meta[idx]
                return {
                    "schema_version": 1,
                    "role": "derived",
                    "artifact_revision_id": revision,
                    "source_version_id": version,
                    "source_revision_id": source_revision,
                    "source_locator": {"kind": "whole_body"},
                    "canonical_source_hash": {
                        "algorithm": "sha256",
                        "digest_hex": hashlib.sha256(body.encode()).hexdigest(),
                    },
                }

            for entity in sorted(first_source):
                idx = first_source[entity]
                revision = (
                    "entity-"
                    + hashlib.sha256(f"{question.id}|{entity}".encode()).hexdigest()[
                        :24
                    ]
                )
                batch.append(
                    {
                        "kind": "entity",
                        "body": display[entity],
                        "logical_id": f"{question.id}|ent:{entity}",
                        "source_id": f"{question.id}#{idx}",
                        "provenance": provenance(idx, revision),
                    }
                )
            for edge in sorted(edges, key=lambda item: item.edge_id):
                revision = (
                    "edge-" + hashlib.sha256(edge.edge_id.encode()).hexdigest()[:24]
                )
                batch.append(
                    {
                        "edge": {
                            "kind": "relation",
                            "from": f"{question.id}|ent:{edge.subject}",
                            "to": f"{question.id}|ent:{edge.object}",
                            "logical_id": edge.edge_id,
                            "source_id": edge.source_id,
                            "provenance": provenance(edge.paragraph_idx, revision),
                        }
                    }
                )
            if len(batch) >= 500:
                engine.write(batch)
                batch.clear()
        if batch:
            engine.write(batch)
        engine.drain(timeout_s=900)
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        rows_by_depth: dict[int, list[dict[str, object]]] = {1: [], 2: []}
        request_digests: dict[str, dict[str, str]] = {}
        latencies: dict[int, list[float]] = {1: [], 2: []}
        for question in selected:
            baseline = seed_by_question[question.id]
            normalized_question = (
                " " + graph_01.normalize_entity(question.question) + " "
            )
            all_entities = set().union(*memberships[question.id].values())
            anchors = tuple(
                sorted(
                    entity
                    for entity in all_entities
                    if f" {entity} " in normalized_question
                )
            )
            gold = [
                str(paragraph.idx)
                for paragraph in question.paragraphs
                if paragraph.is_supporting
            ]
            for depth in (1, 2):
                candidates: list[dict[str, object]] = []
                request_identity: dict[str, object] = {
                    "question_id": question.id,
                    "anchors": list(anchors),
                    "direction": "both",
                    "depth": depth,
                }
                result_identity: dict[str, object] = {"targets": [], "work_units": "0"}
                if anchors:
                    request = fathomdb.GraphExpandRequestV1(
                        schema_version=1,
                        seed=fathomdb.GraphExplicitSeedV1(
                            schema_version=1,
                            type="explicit",
                            logical_ids=tuple(
                                fathomdb.IdSpace(
                                    space="logical",
                                    value=f"{question.id}|ent:{entity}",
                                )
                                for entity in anchors
                            ),
                        ),
                        direction="both",
                        edge_kinds=("relation",),
                        target_kinds=("entity",),
                        context=fathomdb.FrozenGraphReadContextV1(
                            schema_version=1, type="frozen", context=frozen
                        ),
                        max_depth=depth,
                        result_limit=50,
                        max_work_units="10000",
                        include_explanation=False,
                        include_evidence=True,
                    )
                    started = time.perf_counter_ns()
                    result = fathomdb.graph.expand(engine, request)
                    latencies[depth].append(
                        (time.perf_counter_ns() - started) / 1_000_000
                    )
                    if result.evidence is None:
                        raise GraphRetrievalBenchmarkError(
                            "native retrieval omitted exact evidence"
                        )
                    for entry in result.evidence.entries:
                        target = result.targets[entry.target_index]
                        resolved_edge = engine.resolve_graph_evidence(
                            fathomdb.GraphEvidenceResolveRequestV1(
                                evidence_ref=entry.terminal_edge_evidence_ref,
                                context=frozen,
                            )
                        )
                        prefix = question.id + "#"
                        if not resolved_edge.source_id.startswith(prefix):
                            raise GraphRetrievalBenchmarkError(
                                "evidence resolved outside the question corpus"
                            )
                        candidates.append(
                            {
                                "passage_id": resolved_edge.source_id[len(prefix) :],
                                "hop": target.origin.hop_count,
                                "seed_rank": target.origin.seed_ordinal,
                            }
                        )
                    result_identity = {
                        "targets": [item.logical_id for item in result.targets],
                        "work_units": result.work_units,
                    }
                treatment = promote_candidates(
                    baseline,
                    candidates,
                    protected_prefix=int(config.policy["protected_prefix"]),
                    promotion_limit=int(config.policy["promotion_limit"]),
                    evaluation_depth=int(config.policy["evaluation_depth"]),
                )
                rows_by_depth[depth].append(
                    {
                        "question_id": question.id,
                        "hop_count": question.hop_count,
                        "gold": gold,
                        "control": baseline,
                        "treatment": treatment,
                    }
                )
                request_digests[f"{question.id}:d{depth}"] = {
                    "request": graph_benchmarks.canonical_sha256(request_identity),
                    "result": graph_benchmarks.canonical_sha256(result_identity),
                }
    finally:
        engine.close()
    metrics = {
        f"native_expand_depth{depth}": score_rankings(
            rows,
            cutoffs=(5, 10, 20),
            draws=config.bootstrap["draws"],
            seed=config.bootstrap["seed"],
        )
        for depth, rows in rows_by_depth.items()
    }
    metrics["performance"] = {
        f"depth{depth}_latency_ms": {
            "p50": graph_benchmarks.percentile(values, 0.50),
            "p95": graph_benchmarks.percentile(values, 0.95),
            "p99": graph_benchmarks.percentile(values, 0.99),
        }
        for depth, values in latencies.items()
        if values
    }
    observations_path = artifact_root / "historical-observations.v1.json"
    observations_path.write_text(
        json.dumps(
            {
                "schema_version": "graph-retrieval-01.observations.v1",
                "rows_by_depth": rows_by_depth,
                "native_digests": request_digests,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n"
    )
    record = graph_benchmarks.write_benchmark_record(
        experiment=config.cell_id,
        config=config.resolved,
        metrics=metrics,
        base_dir=artifact_root,
        config_path=config_path,
        corpus_manifest_sha256=str(config.inputs["corpus_sha256"]),
        datasets=["MuSiQue-Ans historical-300 directional reuse"],
    )
    return {
        "state": "complete",
        "metrics": metrics,
        "observations": str(observations_path),
        "record": str(record),
    }


def main(argv: list[str] | None = None) -> int:
    """Validate the retrieval contract or execute its no-model native smoke."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "command", choices=("validate", "dry-run", "smoke", "stark-status")
    )
    parser.add_argument("config", type=Path)
    parser.add_argument("artifact_root", type=Path, nargs="?")
    args = parser.parse_args(argv)
    config = load_config(args.config)
    if args.command == "stark-status":
        print(json.dumps(stark_qualification_state(), sort_keys=True))
    elif args.command == "smoke":
        if args.artifact_root is None or args.artifact_root.exists():
            raise GraphRetrievalBenchmarkError("smoke artifact root must be new")
        args.artifact_root.mkdir(parents=True)
        print(
            json.dumps(
                run_smoke(config, args.artifact_root, config_path=args.config),
                sort_keys=True,
            )
        )
    else:
        if args.artifact_root is not None and args.artifact_root.exists():
            raise GraphRetrievalBenchmarkError("artifact root must be new")
        print(json.dumps({"state": "ready", "cell": config.cell_id}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
