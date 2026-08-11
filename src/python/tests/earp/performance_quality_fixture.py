"""Writer-originated quality fixtures for performance-evidence contracts."""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from eval.earp.config import ResolvedScenario, resolve_config
from eval.earp.runner import run_diagnostic
from eval.performance.earp_adapter import WorkloadRef, load_earp_workload

TS = datetime(2026, 8, 11, 12, 0, tzinfo=timezone.utc)


def diagnostic_quality_workload(
    tmp_path: Path,
) -> tuple[ResolvedScenario, dict[str, Any], Path, WorkloadRef]:
    """Write a real quality run, then load only its advertised verified manifest.

    The helper intentionally has no `WorkloadRef` constructor path. A bridge or
    publication success test obtains its workload only through the adapter's
    quality-artifact verification boundary.
    """
    fixture = tmp_path / "fixture.jsonl"
    fixture.write_text(
        json.dumps(
            {
                "kind": "doc",
                "body": "the deal sheet is missing for March",
                "source_id": "source",
                "logical_id": "doc-a",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    config: dict[str, Any] = {
        "schema_version": "earp.v1",
        "campaign": "diagnostic",
        "scenario": {
            "fixture": str(fixture),
            "query": {
                "call": "Engine.search_text_only",
                "text": "deal sheet",
                "limit": 10,
            },
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
        ts=TS,
    )
    assert quality.run_id is not None
    assert quality.run_dir is not None

    manifest = quality.run_dir / "earp.workload-manifest.v1.json"
    assert manifest.is_file(), "quality writer must stage the workload manifest"
    record = json.loads((quality.run_dir / "record.json").read_text(encoding="utf-8"))
    expected_path = f"runs/{quality.run_id}/{manifest.name}"
    assert {
        "path": expected_path,
        "sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
    } in record["artifacts"]

    return resolution.scenario, config, root, load_earp_workload(root, quality.run_id)
