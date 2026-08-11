"""Developer CLI for an independent performance artifact from an EARP run."""

from __future__ import annotations

from pathlib import Path

import pytest

from eval.performance.cli import main
from performance_quality_fixture import diagnostic_quality_workload

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_cli_consumes_quality_run_without_retyping_workload(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    _, _, root, workload = diagnostic_quality_workload(tmp_path)

    code = main(
        [
            "diagnostic",
            "--experiments-root",
            str(root),
            "--quality-run",
            workload.parent_run_id,
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
    from eval.performance.earp_adapter import load_earp_workload  # noqa: PLC0415

    quality = run_characterization(**_bed(tmp_path))
    assert quality.run_id is not None
    assert quality.run_dir is not None
    assert (quality.run_dir / "earp.workload-manifest.v1.json").is_file()
    root = tmp_path / "experiments"
    workload = load_earp_workload(root, quality.run_id)

    code = main(
        [
            "characterization",
            "--experiments-root",
            str(root),
            "--quality-run",
            workload.parent_run_id,
            "--repetitions",
            "1",
            "--treatments",
            "fresh_store,fresh_store_warm_query",
        ]
    )

    assert code == 0
    assert "performance run" in capsys.readouterr().out
    assert len(list(root.glob("runs/*/performance.earp.v1.json"))) == 1
