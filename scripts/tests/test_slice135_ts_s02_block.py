"""Reject false installed-TypeScript S02 timing receipts before a block is accepted."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s02_block as block  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
FEASIBILITY = (
    ROOT
    / "dev/plans/0.8.27/features/slice-135/results/2026-10-07-ts-s02-feasibility/candidate.json"
)


def sample() -> tuple[dict, dict]:
    raw = json.loads(FEASIBILITY.read_text())
    for name in ("after_erasure", "after_reopen"):
        del raw["observed"][name]["canonical_counts"]
    raw["status"] = "UNFROZEN_TS_S02_PRODUCT_TIMING_FEASIBILITY"
    raw["whole_product_ns"] = raw.pop("whole_sequence_including_checks_ns")
    raw["verification_ns"] = 1_000_000
    raw["final_canonical_counts"] = {
        "graph_nodes": 0,
        "graph_edges": 0,
        "corpus_nodes": 32,
    }
    manifest = {
        "source_sha": raw["source_sha"],
        "runner_sha256": raw["runner_sha256"],
        "s01_helper_sha256": raw["s01_helper_sha256"],
        "native_sha256": raw["artifact"]["native_sha256"],
        "module_sha256": raw["artifact"]["module_sha256"],
        "package_sha256": raw["artifact"]["package_sha256"],
        "node_version": raw["artifact"]["node_version"],
        "corpus_sha256": raw["corpus_sha256"],
        "graph_records_sha256": raw["graph_records_sha256"],
    }
    return raw, manifest


def test_product_timing_receipt_and_state_guard() -> None:
    raw, manifest = sample()
    assert block.validate_sample(raw, manifest) == raw["whole_product_ns"]
    bad = deepcopy(raw)
    bad["observed"]["after_reopen"]["source_absent"] = False
    with pytest.raises(ValueError, match="reopened source"):
        block.validate_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["final_canonical_counts"]["graph_edges"] = 1
    with pytest.raises(ValueError, match="canonical"):
        block.validate_sample(bad, manifest)


def test_product_timer_excludes_interleaved_sqlite_and_rejects_artifact_drift() -> None:
    raw, manifest = sample()
    bad = deepcopy(raw)
    bad["whole_product_ns"] = 1
    with pytest.raises(ValueError, match="stage sum"):
        block.validate_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["observed"]["after_erasure"]["canonical_counts"] = raw[
        "final_canonical_counts"
    ]
    with pytest.raises(ValueError, match="interleaved SQLite"):
        block.validate_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["artifact"]["native_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="native"):
        block.validate_sample(bad, manifest)


def test_nearest_rank_median_even_samples_is_lower_middle() -> None:
    assert block.nearest_rank_median([40, 10, 30, 20]) == 20


def test_independent_guard_rejects_changed_evidence_and_runtime() -> None:
    raw, manifest = sample()
    bad = deepcopy(raw)
    bad["observed"]["evidence"]["source_body"] = "wrong"
    with pytest.raises(ValueError, match="evidence"):
        block.validate_sample(bad, manifest)
    bad = deepcopy(raw)
    bad["artifact"]["node_version"] = "v0.0.0"
    with pytest.raises(ValueError, match="Node"):
        block.validate_sample(bad, manifest)


def test_frozen_s02_subset_binds_both_installed_artifact_roles() -> None:
    path = (
        ROOT / "dev/plans/0.8.27/features/slice-135/s02-ts-comparison-protocol.json"
    )
    protocol = json.loads(path.read_text())
    for role in ("baseline", "candidate"):
        artifact = protocol[role]
        block.validate_comparison_protocol(
            protocol,
            role=role,
            source_sha=artifact["source_sha"],
            main_sha256=artifact["main_archive_sha256"],
            platform_sha256=artifact["platform_archive_sha256"],
            samples=20,
        )
    changed = deepcopy(protocol)
    changed["candidate"]["platform_archive_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="artifact"):
        block.validate_comparison_protocol(
            changed,
            role="candidate",
            source_sha=artifact["source_sha"],
            main_sha256=artifact["main_archive_sha256"],
            platform_sha256=artifact["platform_archive_sha256"],
            samples=20,
        )
