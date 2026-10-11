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


def test_frozen_reference_manifest_is_bound(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    files = {}
    for name, attribute in (
        ("runner", "RUNNER"), ("auditor", "AUDITOR"),
        ("block_runner", "__file__"), ("s01_helper", "S01_HELPER"),
        ("s02_helper", "S02_HELPER"), ("prior_protocol", "PRIOR"),
    ):
        path = tmp_path / name
        path.write_text(name)
        files[name] = path
        monkeypatch.setattr(block, attribute, str(path) if attribute == "__file__" else path)
    for name in ("baseline_main_archive", "baseline_platform_archive", "baseline_reference_manifest"):
        path = tmp_path / name
        path.write_text(name)
        files[name] = path
    protocol = {
        "status": "FROZEN_S02_TS_CONTENTION_PAIRED",
        "samples_per_block": 20,
        "frozen_sha256": {name: block.sha(path) for name, path in files.items()},
    }
    arguments = {
        "role": "baseline", "samples": 20,
        "main_archive": files["baseline_main_archive"],
        "platform_archive": files["baseline_platform_archive"],
        "reference_manifest": files["baseline_reference_manifest"],
    }
    block.validate_frozen(protocol, **arguments)
    files["baseline_reference_manifest"].write_text("changed")
    with pytest.raises(ValueError, match="frozen"):
        block.validate_frozen(protocol, **arguments)
