"""Contract tests for the compact SCALE-02 FTS follow-up."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from experiments import scale_02_followup as followup


def test_configured_executable_path_preserves_venv_symlink(tmp_path):
    executable = tmp_path / "venv" / "bin" / "python"
    executable.parent.mkdir(parents=True)
    executable.symlink_to(sys.executable)

    resolved = followup._configured_executable_path(executable, "runtime.python")

    assert resolved == executable
    assert resolved.is_symlink()


def test_freeze_inputs_writes_digest_manifest_and_refuses_drift(tmp_path):
    tc5 = tmp_path / "tc5-source"
    (tc5 / "documents").mkdir(parents=True)
    (tc5 / "documents" / "00000.txt").write_text("alpha", encoding="utf-8")
    (tc5 / "queries").mkdir()
    (tc5 / "queries" / "000.txt").write_text("alpha", encoding="utf-8")
    (tc5 / "tc5-corpus-input.v1.json").write_text(
        json.dumps({"schema_version": "tc5-corpus-input.v1"}), encoding="utf-8"
    )
    qualified = tmp_path / "qualified.json"
    qualified.write_text('{"schema_version":"tc5-manifest.v1"}', encoding="utf-8")
    locomo = tmp_path / "locomo.json"
    locomo.write_text("[]", encoding="utf-8")
    subset = tmp_path / "subset.json"
    subset.write_text(
        json.dumps(
            {
                "schema_version": "locomo-fixed-subset.v1",
                "question_ids": [f"q-{index}" for index in range(32)],
            }
        ),
        encoding="utf-8",
    )
    output = tmp_path / "persistent" / "input-pack-v1"

    result = followup.freeze_inputs(
        tc5_root=tc5,
        tc5_qualified_manifest=qualified,
        locomo_corpus=locomo,
        locomo_subset=subset,
        output_root=output,
    )

    manifest = json.loads((output / "input-manifest.v1.json").read_text())
    assert manifest["schema_version"] == "scale-02-input-manifest.v1"
    assert manifest["file_count"] == 6
    assert result["manifest_sha256"] == followup.sha_file(
        output / "input-manifest.v1.json"
    )
    assert all(not Path(item["path"]).is_absolute() for item in manifest["files"])

    (tc5 / "documents" / "00000.txt").write_text("drift", encoding="utf-8")
    with pytest.raises(followup.Scale02FollowupError, match="already exists"):
        followup.freeze_inputs(
            tc5_root=tc5,
            tc5_qualified_manifest=qualified,
            locomo_corpus=locomo,
            locomo_subset=subset,
            output_root=output,
        )


def test_reader_witness_requires_every_observation_to_match():
    expected = {"cache_size": -65536, "mmap_size": 268435456, "temp_store": 0}
    rows = [
        {
            "schema_version": "scale-02-reader-settings.v1",
            "requested": "cache_size=-65536,mmap_size=268435456",
            "cache_size": -65536,
            "mmap_size": 268435456,
            "temp_store": 0,
            "sqlite_version": "3.50.4",
        }
        for _ in range(8)
    ]

    observed = followup.validate_reader_observations(rows, expected)
    assert observed["connection_count"] == 8
    assert observed["sqlite_version"] == "3.50.4"

    rows[-1]["mmap_size"] = 0
    with pytest.raises(followup.Scale02FollowupError, match="drifted"):
        followup.validate_reader_observations(rows, expected)


def test_selection_uses_lowest_footprint_eligible_candidate():
    cells = [
        {
            "id": "rank_default",
            "query_path": "rank_fast",
            "footprint_order": 0,
            "errors": 0,
            "timeouts": 0,
            "complete_repetitions": True,
            "upper_95": {"steady_p50_ms": 18.0, "steady_p99_ms": 70.0, "rss_fraction": 0.01},
        },
        {
            "id": "rank_mmap256",
            "query_path": "rank_fast",
            "footprint_order": 2,
            "errors": 0,
            "timeouts": 0,
            "complete_repetitions": True,
            "upper_95": {"steady_p50_ms": 8.0, "steady_p99_ms": 20.0, "rss_fraction": 0.02},
        },
    ]
    equivalence = {
        "tc5": {"query_count": 100, "mismatch_count": 0},
        "answer_01": {"query_count": 32, "mismatch_count": 0},
    }

    selection = followup.select_candidate(
        cells,
        equivalence,
        policy={"steady_p50_ms": 20, "steady_p99_ms": 150, "max_rss_fraction": 0.8},
    )

    assert selection["recommended_cell"] == "rank_default"
    assert selection["state"] == "recommendation_pending_hitl"
    assert selection["quality_applicability"] == "unchanged"


def test_selection_rejects_a_quality_mismatch():
    cell = {
        "id": "rank_default",
        "query_path": "rank_fast",
        "footprint_order": 0,
        "errors": 0,
        "timeouts": 0,
        "complete_repetitions": True,
        "upper_95": {"steady_p50_ms": 10.0, "steady_p99_ms": 20.0, "rss_fraction": 0.01},
    }
    equivalence = {
        "tc5": {"query_count": 100, "mismatch_count": 1},
        "answer_01": {"query_count": 32, "mismatch_count": 0},
    }

    selection = followup.select_candidate(
        [cell],
        equivalence,
        policy={"steady_p50_ms": 20, "steady_p99_ms": 150, "max_rss_fraction": 0.8},
    )

    assert selection["recommended_cell"] is None
    assert selection["quality_applicability"] == "changed"
