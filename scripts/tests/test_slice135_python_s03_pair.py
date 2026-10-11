"""Frozen comparison safeguards for the Slice 135 S03 Python subset."""

from __future__ import annotations

import sys
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s03_pair_block as pair_block  # noqa: E402
import slice135_python_s03_pair_campaign as campaign  # noqa: E402
import slice135_python_s03_pair_audit as paired_audit  # noqa: E402


def test_pair_block_requires_frozen_matching_artifacts() -> None:
    spec = {
        "schema_version": 1,
        "status": "FROZEN_S03_PYTHON_PAIRED",
        "sizes": [32, 256],
        "samples_per_case": 100,
        "input_sha256": {
            str(path.relative_to(pair_block.ROOT)): pair_block.digest(path)
            for path in (
                pair_block.RUNNER,
                pair_block.audit.ROOT / "scripts/slice135_python_s01.py",
                pair_block.audit.ROOT / "scripts/slice135_python_s02.py",
                pair_block.audit.FIXTURE,
                pair_block.ROOT / "scripts/slice135_python_s01_block.py",
            )
        },
        "baseline": {"source_sha": "a" * 40, "wheel_sha256": "b" * 64},
        "candidate": {"source_sha": "c" * 40, "wheel_sha256": "d" * 64},
    }
    pair_block.validate_spec(
        spec, role="candidate", size=256, source_sha="c" * 40, wheel_sha256="d" * 64
    )
    for changed in (
        {**spec, "status": "DRAFT"},
        {**spec, "samples_per_case": 99},
        {**spec, "sizes": [32]},
        {**spec, "input_sha256": {}},
    ):
        try:
            pair_block.validate_spec(
                changed,
                role="candidate",
                size=256,
                source_sha="c" * 40,
                wheel_sha256="d" * 64,
            )
        except ValueError:
            pass
        else:
            raise AssertionError("unfrozen S03 comparison was accepted")
    try:
        pair_block.validate_spec(
            spec, role="candidate", size=256, source_sha="c" * 40, wheel_sha256="b" * 64
        )
    except ValueError:
        pass
    else:
        raise AssertionError("candidate wheel identity mismatch was accepted")


def test_pair_schedule_alternates_with_20_declared_blocks() -> None:
    blocks = campaign.planned_blocks()
    assert len(blocks) == 20
    for size in (32, 256):
        pairs = [item for item in blocks if item[0] == size]
        assert [item[2] for item in pairs] == [
            role
            for index in range(1, 6)
            for role in (
                ("baseline", "candidate") if index % 2 else ("candidate", "baseline")
            )
        ]
    records = [
        {
            "ordinal": ordinal,
            "size": size,
            "pair_index": index,
            "role": role,
            "name": name,
        }
        for ordinal, (size, index, role, name) in enumerate(blocks, 1)
    ]
    paired_audit.check_order(records)
    records[0], records[1] = records[1], records[0]
    try:
        paired_audit.check_order(records)
    except ValueError:
        pass
    else:
        raise AssertionError("swapped version order was accepted")
