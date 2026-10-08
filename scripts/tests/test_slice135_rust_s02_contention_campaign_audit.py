"""Independent Rust contention auditor reads GNU Time bytes strictly."""

from __future__ import annotations

from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_rust_s02_contention_campaign_audit as audit  # noqa: E402


def test_resource_report_requires_complete_child_fields(tmp_path: Path) -> None:
    resource = tmp_path / "sample.resource"
    resource.write_text(
        "user_s=31.50\nsystem_s=9.25\npeak_rss_kib=300000\n"
        "fs_inputs=0\nfs_outputs=101\nmajor_faults=0\nswap_events=0\n"
    )
    report = audit.resource_report(resource)
    assert report["user_cpu_s"] == 31.5
    assert report["peak_rss_kib"] == 300000
    resource.write_text(resource.read_text().replace("swap_events=0\n", ""))
    with pytest.raises(ValueError, match="resource"):
        audit.resource_report(resource)


def test_command_audit_accepts_relocated_raw_archive() -> None:
    command = [
        "/usr/bin/time", "-o", "/tmp/original/sample-001.resource",
        "-f", audit.GNU_TIME_FORMAT,
        "/tmp/original/slice135-rust-sdk-s02-contention",
        "/tmp/original/sample-001.sqlite",
    ]
    audit.validate_command(command, "sample-001")
    with pytest.raises(ValueError, match="command"):
        audit.validate_command([*command[:-1], "/tmp/original/other.sqlite"], "sample-001")
