from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

from experiments import graph_retrieval_01


ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / "data/corpus-data/raw/musique_dev.jsonl"
OBSERVATIONS = (
    ROOT
    / "data/performance-benchmarking/graph-01/runs/graph-01-protected-bridge-20260829-a/observations.v1.json"
)
MODEL_CACHE = (
    Path.home()
    / ".cache/huggingface/hub/models--BAAI--bge-small-en-v1.5/snapshots"
    / "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"
)


@pytest.mark.skipif(
    os.environ.get("FATHOMDB_GRAPH_GPU_TESTS") != "1",
    reason="requires the pinned RTX 3090 and local BGE assets",
)
def test_historical_cuda_encoder_reproduces_retained_top10() -> None:
    from eval.m1_baseline import load_musique

    observations = json.loads(OBSERVATIONS.read_text())["rows"][:2]
    questions = {question.id: question for question in load_musique(CORPUS)}
    encoder = graph_retrieval_01.HistoricalCudaBgeEncoder(MODEL_CACHE, "cuda:0")

    for observation in observations:
        actual = graph_retrieval_01._fused_seed_ranking(
            questions[observation["question_id"]], encoder.embed_batch
        )
        assert actual[:10] == observation["control"]
