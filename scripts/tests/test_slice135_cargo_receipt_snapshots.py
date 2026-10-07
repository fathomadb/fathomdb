"""Archived Slice 135 Cargo receipts must remain auditable without live pins."""

from __future__ import annotations

from pathlib import Path
import tomllib


ROOT = Path(__file__).resolve().parents[2]
RESULTS = ROOT / "dev/plans/0.8.27/features/slice-135/results"
RECEIPTS = (
    "2026-10-07-e01-e12-feasibility/baseline",
    "2026-10-07-e01-e12-feasibility/candidate",
    "2026-10-07-e12-adapter-pilot/shared/baseline-build",
    "2026-10-07-e12-adapter-pilot/shared/candidate-build",
    "2026-10-07-rust-sdk-capability",
    "2026-10-07-rust-sdk-s02-feasibility",
)


def test_archived_manifests_are_parseable_and_not_live_cargo_configs() -> None:
    for relative in RECEIPTS:
        directory = RESULTS / relative
        assert not (directory / "Cargo.toml").exists()
        snapshot = directory / "Cargo.toml.snapshot"
        manifest = tomllib.loads(snapshot.read_text())
        assert manifest["package"]["name"]
