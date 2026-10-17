"""The runner-unrecoverable quarantine: an explicit list, never a blanket pass.

``comp-03-hermes-answer-20261016T1239Z-2e410a4f`` (the v1 COMP-03 answer run)
was measured from a dirty tree whose recorded commit lacks its runner, so it
cannot be classified against its code; the v2 receipts supersede it. The
classifier accepts it only under ``runner_code_unrecoverable_superseded`` and
only because it is listed.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest

from experiments import measurement_classification as mc

ROOT = Path(__file__).resolve().parents[2]
RUN = "comp-03-hermes-answer-20261016T1239Z-2e410a4f"
SUMMARY = "f129662f6055d6042b5fc6151e93a7108d88d9e2aae9ca8dc4ebf371714dba14"


def _entry(run_id: str = RUN) -> dict:
    record = ROOT / "experiments" / "runs" / RUN / "record.json"
    return {"run_id": run_id, "record_sha256": mc._sha256_bytes(record.read_bytes()),
            "reason": mc.RUNNER_UNRECOVERABLE_REASON, "rationale": "test"}


def test_the_listed_receipt_validates_only_under_the_new_reason():
    document = mc.runner_unrecoverable_quarantine(ROOT, RUN, external_summary_sha256=SUMMARY, rationale="test")
    assert document["reason"] == "runner_code_unrecoverable_superseded"
    assert document["missing_runner_paths"] == ["experiments/comp03_answer.py"]
    assert document["recorded_dirty"] is True
    mc.validate_runner_unrecoverable_quarantine(document, repository_root=ROOT, entry=_entry())
    other_reason = {**_entry(), "reason": "invalid_ancillary_locator_quarantined"}
    with pytest.raises(mc.ClassificationError):
        mc.validate_runner_unrecoverable_quarantine(document, repository_root=ROOT, entry=other_reason)


def test_an_unlisted_run_with_the_same_shape_is_rejected():
    document = mc.runner_unrecoverable_quarantine(ROOT, RUN, external_summary_sha256=SUMMARY, rationale="test")
    other = "comp-03-hermes-answer-20261016T1422Z-c807ed90"
    renamed = {**copy.deepcopy(document), "run_id": other}
    with pytest.raises(mc.ClassificationError):
        mc.validate_runner_unrecoverable_quarantine(renamed, repository_root=ROOT, entry=_entry(other))
    with pytest.raises(mc.ClassificationError):
        mc.runner_unrecoverable_quarantine(ROOT, other, external_summary_sha256=SUMMARY, rationale="test")
    with pytest.raises(mc.ClassificationError):
        mc._parse_superseded_postcutover_runs([_entry(other)])
    assert set(mc._parse_superseded_postcutover_runs([_entry()])) == {RUN}


def test_the_quarantine_needs_the_runner_to_be_absent_and_the_summary_digest_to_match():
    document = mc.runner_unrecoverable_quarantine(ROOT, RUN, external_summary_sha256=SUMMARY, rationale="test")
    present = {**copy.deepcopy(document), "missing_runner_paths": ["experiments/_lib.py"]}
    with pytest.raises(mc.ClassificationError):
        mc.validate_runner_unrecoverable_quarantine(present, repository_root=ROOT, entry=_entry())
    with pytest.raises(mc.ClassificationError):
        mc.runner_unrecoverable_quarantine(ROOT, RUN, external_summary_sha256="0" * 64, rationale="test")


def test_the_committed_tree_carries_the_quarantine_and_validates():
    sidecar = ROOT / "experiments" / "runs" / RUN / mc.LEGACY_SIDECAR_NAME
    document = json.loads(sidecar.read_text(encoding="utf-8"))
    assert document["schema_version"] == mc.QUARANTINE_SCHEMA_VERSION and document["run_id"] == RUN
    mc.validate_repository(ROOT)
