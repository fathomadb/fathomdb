"""Corpus-scale EARP characterization carries one-run observed cost."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from eval.performance.earp_adapter import load_earp_workload

pytest.importorskip("fathomdb._fathomdb", reason="native binding not built")


def test_characterization_writes_observed_cost_and_workload_knobs(tmp_path: Path) -> None:
    from test_characterization import _bed  # noqa: PLC0415
    from eval.earp.characterize import run_characterization  # noqa: PLC0415

    result = run_characterization(**_bed(tmp_path))
    assert result.run_id is not None
    assert result.run_dir is not None
    observed = json.loads(
        (result.run_dir / "earp.observed-cost.v1.json").read_text(encoding="utf-8")
    )
    assert observed["evidence_family_id"] == result.run_id
    assert {"open", "write", "query"} <= set(observed["phases_ms"])
    workload = load_earp_workload(tmp_path / "experiments", result.run_id)
    assert workload.effective_knobs == {"limit": result.fanout_used}
