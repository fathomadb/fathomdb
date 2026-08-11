"""Developer CLI for an independent performance artifact from an EARP run."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from eval.earp.config import resolve_config
from eval.earp.runner import run_diagnostic
from eval.performance.cli import main

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_cli_consumes_quality_run_without_retyping_workload(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    fixture = tmp_path / "fixture.jsonl"
    fixture.write_text(
        json.dumps(
            {
                "kind": "doc",
                "body": "deal sheet",
                "source_id": "source",
                "logical_id": "doc-a",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    config = {
        "schema_version": "earp.v1",
        "campaign": "diagnostic",
        "scenario": {
            "fixture": str(fixture),
            "query": {"call": "Engine.search_text_only", "text": "deal", "limit": 10},
        },
    }
    resolution = resolve_config(config)
    assert resolution.scenario is not None, resolution.blockers
    root = tmp_path / "experiments"
    quality = run_diagnostic(
        scenario=resolution.scenario,
        config_doc=config,
        experiments_root=root,
        experiment="earp-diagnostic",
        ts=datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc),
    )
    assert quality.run_id is not None

    code = main(
        [
            "diagnostic",
            "--experiments-root",
            str(root),
            "--quality-run",
            quality.run_id,
            "--repetitions",
            "1",
            "--treatments",
            "fresh_store,fresh_store_warm_query",
        ]
    )

    assert code == 0
    output = capsys.readouterr().out
    assert "performance run" in output
    assert len(list((root / "runs").glob("*/performance.earp.v1.json"))) == 1


def test_cli_repeats_a_characterization_quality_run(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    from test_characterization import _bed  # noqa: PLC0415
    from eval.earp.characterize import run_characterization  # noqa: PLC0415

    quality = run_characterization(**_bed(tmp_path))
    assert quality.run_id is not None
    root = tmp_path / "experiments"

    code = main(
        [
            "characterization",
            "--experiments-root",
            str(root),
            "--quality-run",
            quality.run_id,
            "--repetitions",
            "1",
            "--treatments",
            "fresh_store,fresh_store_warm_query",
        ]
    )

    assert code == 0
    assert "performance run" in capsys.readouterr().out
    assert len(list(root.glob("runs/*/performance.earp.v1.json"))) == 1
