"""Human-intended guardrails for the GLOBAL-01 first run."""

from __future__ import annotations

import hashlib
import json
import zipfile
from pathlib import Path

import pytest

from experiments import global_01


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _config() -> dict[str, object]:
    return {
        "schema_version": "global-01.first-run.v1",
        "program_track": "GLOBAL-01",
        "run_label": "apnews-15doc-first-run",
        "approval": {
            "state": "pending_hitl",
            "approved_by": None,
            "approved_at": None,
            "cost_cap_usd": None,
        },
        "corpus": {
            "root": "corpus",
            "archive_sha256": "a" * 64,
            "article_count": 2,
            "witness_count": 2,
            "preserved_input_root": "preserved/input",
        },
        "questions": {
            "scope": "global",
            "count": 2,
            "selection_rule": "ordered_global_stride_floor",
            "selection_sha256": "b" * 64,
        },
        "graphrag": {
            "version": "3.1.0",
            "freeze": "preserved/freeze.txt",
            "freeze_sha256": "c" * 64,
            "settings": "preserved/settings.yaml",
            "settings_sha256": "d" * 64,
            "prompts": "preserved/prompts",
            "embedding_shim": "shim.py",
            "embedding_shim_sha256": "e" * 64,
            "answer_model": "deepseek-v4-pro",
            "thinking_mode": "disabled",
            "community_level": 1,
            "dynamic_selection": False,
            "concurrency": 4,
        },
        "fathomdb": {
            "version": "0.8.23",
            "profile": "source_mapreduce_c_v1",
            "embedder": "none",
            "reranker": "disabled",
            "map_batch_documents": 5,
            "map_max_tokens": 300,
            "reduce_max_tokens": 1500,
        },
        "judge": {
            "model": "claude-haiku",
            "repetitions": 5,
            "order_swapped": True,
            "metrics": [
                "comprehensiveness",
                "diversity",
                "empowerment",
                "directness",
            ],
        },
        "pricing": {
            "currency": "USD",
            "source": "local_pinned_2026-08-25",
            "deepseek-v4-pro": {
                "input_per_million": 1.32,
                "output_per_million": 3.96,
            },
            "claude-haiku": {
                "input_per_million": 1.0,
                "output_per_million": 5.0,
            },
            "projected_total_usd": 4.22,
            "recommended_cap_usd": 6.0,
        },
    }


def _write_corpus_fixture(root: Path) -> tuple[Path, Path]:
    corpus = root / "corpus"
    corpus.mkdir()
    archive = corpus / "raw_data.zip"
    records = [
        {
            "altids": {"itemid": f"doc-{index}"},
            "headline": f"Title {index}",
            "body_nitf": f"<p>Body {index}</p>",
        }
        for index in range(2)
    ]
    with zipfile.ZipFile(archive, "w") as bundle:
        for index, record in enumerate(records):
            bundle.writestr(f"{index:02d}.json", json.dumps(record))
    questions = corpus / "generated_questions_v1"
    questions.mkdir()
    global_questions = [f"Global question {index}?" for index in range(4)]
    (questions / "activity_global_questions_text.json").write_text(
        json.dumps(global_questions), encoding="utf-8"
    )
    manifest = {
        "schema": "0.8.4-apnews-benchmarkqed-manifest-v1",
        "license": "evaluation only",
        "n_articles": 2,
        "raw_data_zip_sha256": _sha256(archive),
        "autoq_questions": {
            "generated_questions_v1": ["activity_global_questions_text.json"]
        },
    }
    (corpus / "MANIFEST.json").write_text(json.dumps(manifest), encoding="utf-8")

    preserved = root / "preserved" / "input"
    preserved.mkdir(parents=True)
    for index in range(2):
        (preserved / f"doc_{index:02d}.txt").write_text(
            f"Title {index}\n\nBody {index}", encoding="utf-8"
        )
    return corpus, preserved


def test_input_preflight_binds_each_document_and_question_without_payload(tmp_path):
    corpus, preserved = _write_corpus_fixture(tmp_path)
    config = _config()
    config["corpus"]["archive_sha256"] = _sha256(corpus / "raw_data.zip")  # type: ignore[index]
    selected = ["Global question 0?", "Global question 2?"]
    config["questions"]["selection_sha256"] = global_01.canonical_sha256(  # type: ignore[index]
        selected
    )

    report, private_manifest = global_01.inspect_inputs(
        global_01.validate_config(config), repository_root=tmp_path
    )

    assert report["state"] == "input_ready"
    assert report["corpus"]["witness_count"] == 2
    assert report["corpus"]["all_preserved_inputs_match"] is True
    assert report["questions"]["count"] == 2
    assert report["questions"]["selection_sha256"] == global_01.canonical_sha256(
        selected
    )
    assert len(private_manifest["documents"]) == 2
    serialized = json.dumps(report)
    assert "Body 0" not in serialized
    assert "Global question" not in serialized
    assert "doc-0" not in serialized
    assert str(corpus) not in serialized
    assert preserved.is_dir()


def test_input_preflight_rejects_a_preserved_document_mismatch(tmp_path):
    corpus, preserved = _write_corpus_fixture(tmp_path)
    (preserved / "doc_01.txt").write_text("changed", encoding="utf-8")
    config = _config()
    config["corpus"]["archive_sha256"] = _sha256(corpus / "raw_data.zip")  # type: ignore[index]
    config["questions"]["selection_sha256"] = global_01.canonical_sha256(  # type: ignore[index]
        ["Global question 0?", "Global question 2?"]
    )

    with pytest.raises(global_01.Global01Error, match="preserved GraphRAG input"):
        global_01.inspect_inputs(
            global_01.validate_config(config), repository_root=tmp_path
        )


def test_paid_execution_is_refused_until_hitl_approves_a_positive_cap():
    config = global_01.validate_config(_config())

    with pytest.raises(global_01.Global01Error, match="explicit HITL approval"):
        global_01.assert_execution_authorized(config)


def test_config_rejects_a_secret_field():
    config = _config()
    config["api_key"] = "must-not-be-serialized"

    with pytest.raises(global_01.Global01Error, match="config keys"):
        global_01.validate_config(config)


def test_safe_report_rejects_environment_secret_values():
    report = {
        "schema_version": "global-01.preflight.v1",
        "state": "ready_for_hitl",
        "authentication": {
            "key_source": "AIRLOCK_VIRTUAL_KEY",
            "accidental_value": "secret-token",
        },
    }

    with pytest.raises(global_01.Global01Error, match="secret value"):
        global_01.assert_safe_document(report, secret_values=["secret-token"])
