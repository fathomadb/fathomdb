"""TypeScript contention blocks reject changed source and sample rules."""

from __future__ import annotations

from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s02_contention_block as block  # noqa: E402


def test_prior_installed_artifact_pin_is_exact() -> None:
    protocol = {
        "status": "FROZEN_TS_S02_PAIRED",
        "baseline": {
            "source_sha": "a" * 40,
            "main_archive_sha256": "b" * 64,
            "platform_archive_sha256": "c" * 64,
        },
    }
    block.validate_identity(protocol, "baseline", "a" * 40, "b" * 64, "c" * 64)
    with pytest.raises(ValueError, match="artifact"):
        block.validate_identity(protocol, "baseline", "a" * 40, "d" * 64,
                                "c" * 64)
    with pytest.raises(ValueError, match="role"):
        block.validate_identity(protocol, "candidate", "a" * 40, "b" * 64,
                                "c" * 64)


def test_block_statistics_do_not_trim_slow_samples() -> None:
    assert block.nearest_rank([1, 2, 3, 4, 100], .5) == 3
    assert block.nearest_rank([1, 2, 3, 4, 100], .95) == 100
