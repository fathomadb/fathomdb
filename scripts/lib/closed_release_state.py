"""Validate the recorded scope of a release whose ladder has ended."""

from __future__ import annotations

import math
from typing import Any


def _slice_id(value: object) -> bool:
    if type(value) is int:
        return value >= 0
    return type(value) is float and math.isfinite(value) and value >= 0


def _recorded_out_of_scope(release: str, state: dict, row: dict) -> bool:
    # The retained .29 rows predate a structured terminal disposition. Read
    # their explicit scope and matching decision entries without rewriting history.
    decisions = state.get("decisions")
    if release != "0.8.29" or not isinstance(decisions, dict):
        return False

    if row["slice"] == 120 and row["status"] == "DEFERRED_BY_OWNER":
        ruled = decisions.get("ruled")
        return isinstance(ruled, list) and any(
            isinstance(item, dict)
            and item.get("id") == "artifact-selection"
            and item.get("owner_slice") == 120
            and item.get("source") == "dev/steward/steward-ledger.jsonl seq-378"
            for item in ruled
        )

    if row["slice"] == 150 and row["status"] == "NOT_STARTED":
        unruled = decisions.get("unruled")
        evidence = row.get("evidence")
        return (
            row.get("short") == "OPTIONAL-EXPERIMENTS"
            and isinstance(evidence, str)
            and "All15 inventory items remain NOT_SELECTED." in evidence
            and isinstance(unruled, list)
            and any(
                isinstance(item, dict)
                and item.get("id") == "optional-experiments"
                and item.get("owner_slice") == 150
                for item in unruled
            )
        )
    return False


def validate_closed_ladder(release: str, state: dict[str, Any]) -> bool:
    """Validate a closed ladder and return whether main integration is pending.

    Raise ValueError for malformed closure, unfinished selected work, or
    disagreement between the landed index and ladder.
    """
    ladder = state.get("ladder")
    landed = state.get("landed")
    if (
        "next_slice" not in state
        or state["next_slice"] is not None
        or state.get("remaining_ladder") != []
        or not isinstance(ladder, list)
        or not ladder
        or not isinstance(landed, list)
    ):
        raise ValueError("invalid end-of-ladder state; require explicit null next_slice, empty remaining work and a ladder and landed list")
    if any(not _slice_id(slice_no) for slice_no in landed) or len(set(landed)) != len(landed):
        raise ValueError("malformed or duplicate slice ID in landed list")

    pending = False
    if "completion" in state:
        completion = state["completion"]
        if (
            not isinstance(completion, dict)
            or set(completion) != {"ref", "main_integration"}
            or completion["ref"] != f"origin/release/{release}"
            or completion["main_integration"] not in ("PENDING", "COMPLETE")
        ):
            raise ValueError(f"invalid completion record; expected exact origin/release/{release} and main integration status")
        pending = completion["main_integration"] == "PENDING"

    seen = set()
    expected_landed = set()
    for row in ladder:
        if not isinstance(row, dict) or not _slice_id(row.get("slice")) or not isinstance(row.get("status"), str):
            raise ValueError("malformed terminal ladder row")
        slice_no = row["slice"]
        if slice_no in seen:
            raise ValueError(f"duplicate terminal row for Slice {slice_no}")
        seen.add(slice_no)
        status = row["status"]
        if status == "LANDED":
            expected_landed.add(slice_no)
        elif not (pending and status == "COMPLETE_ON_RELEASE_BRANCH") and not _recorded_out_of_scope(release, state, row):
            raise ValueError(f"Slice {slice_no} is not terminal for this release: {status}")

    if set(landed) != expected_landed:
        raise ValueError("end-of-ladder landed list disagrees with the LANDED rows")
    return pending
