#!/usr/bin/env python3
"""Independently reopen every Rust SDK S02 contention process receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

import slice135_rust_s02_contention_audit as run_audit


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "scripts/slice135_rust_s02_contention.rs"
AUDITOR = ROOT / "scripts/slice135_rust_s02_contention_audit.py"
CAMPAIGN = ROOT / "scripts/slice135_rust_s02_contention_campaign.py"
STABLE = ("host", "kernel", "cpu", "storage", "governor", "toolchain", "profiler",
          "resource_tool")
GNU_TIME_FORMAT = (
    "user_s=%U\nsystem_s=%S\npeak_rss_kib=%M\nfs_inputs=%I\nfs_outputs=%O\n"
    "major_faults=%F\nswap_events=%W\n"
)


def sha(path: Path) -> str:
    """Hash retained bytes without trusting producer digests."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def resource_report(path: Path) -> dict:
    """Parse GNU Time's child-only resource lines independently."""
    lines = [line.split("=", 1) for line in path.read_text().splitlines() if "=" in line]
    fields = dict(lines)
    expected = {"user_s", "system_s", "peak_rss_kib", "fs_inputs", "fs_outputs",
                "major_faults", "swap_events"}
    if len(lines) != len(expected) or set(fields) != expected:
        raise ValueError("Rust contention resource fields incomplete")
    try:
        result = {
            "scope": "measured-workload-child-process",
            "method": "gnu-time",
            "user_cpu_s": float(fields["user_s"]),
            "system_cpu_s": float(fields["system_s"]),
            "peak_rss_kib": int(fields["peak_rss_kib"]),
            "fs_inputs": int(fields["fs_inputs"]),
            "fs_outputs": int(fields["fs_outputs"]),
            "major_faults": int(fields["major_faults"]),
            "swap_events": int(fields["swap_events"]),
            "unsupported": [],
        }
    except ValueError as error:
        raise ValueError("Rust contention resource field malformed") from error
    if (
        any(result[key] < 0 for key in ("user_cpu_s", "system_cpu_s", "peak_rss_kib",
                                           "fs_inputs", "fs_outputs", "major_faults",
                                           "swap_events"))
        or result["peak_rss_kib"] < 1
        or result["swap_events"]
    ):
        raise ValueError("Rust contention child resources invalid")
    return result


def environment_warnings(environment: dict, resource: dict) -> list[str]:
    """Recompute host-only paging context and reject timing invalidators."""
    start, end = environment.get("start", {}), environment.get("end", {})
    if any(not start.get(key) or start[key] != end.get(key) for key in STABLE):
        raise ValueError("Rust contention environment identity drift")
    if start.get("governor") == "unavailable" or start.get("competing_jobs") or end.get("competing_jobs"):
        raise ValueError("Rust contention environment invalid")
    if min(start.get("disk_free_bytes", 0), end.get("disk_free_bytes", 0)) < 1_073_741_824:
        raise ValueError("Rust contention disk pressure")
    before, after = start.get("swap_pages"), end.get("swap_pages")
    if type(before) is not int or type(after) is not int or after < before:
        raise ValueError("Rust contention swap counter invalid")
    if environment.get("invalidators") != []:
        raise ValueError("Rust contention declared invalidator")
    warnings = []
    if after > before:
        warnings.append(f"host swap drift: {after - before} pages; child swap events: 0")
    if resource["major_faults"]:
        warnings.append(f"child major faults: {resource['major_faults']}")
    warnings = sorted(warnings)
    if environment.get("warnings") != warnings:
        raise ValueError("Rust contention host warning changed")
    return warnings


def parse_stdout(path: Path) -> dict:
    """Require one unambiguous structured product receipt."""
    lines = path.read_text().splitlines()
    prefix = "SLICE135_RUST_S02_CONTENTION "
    if len(lines) != 1 or not lines[0].startswith(prefix):
        raise ValueError("Rust contention stdout receipt missing")
    return json.loads(lines[0][len(prefix):])


def validate_command(command: list, label: str) -> None:
    """Bind command semantics while allowing the retained archive to move."""
    if (
        len(command) != 7
        or command[0] != "/usr/bin/time"
        or command[1] != "-o"
        or Path(command[2]).name != f"{label}.resource"
        or command[3] != "-f"
        or command[4] != GNU_TIME_FORMAT
        or Path(command[5]).name != "slice135-rust-sdk-s02-contention"
        or Path(command[6]).name != f"{label}.sqlite"
    ):
        raise ValueError("Rust contention process command changed")


def verify_product(checkout: Path, protocol: dict) -> None:
    """Bind the external consumer to the exact clean candidate source."""
    def git(*arguments: str) -> str:
        return subprocess.run(["git", *arguments], cwd=checkout, capture_output=True,
                              text=True, check=True).stdout.strip()

    if (
        git("rev-parse", "HEAD") != protocol["product_source_sha"]
        or git("rev-parse", "HEAD:src/rust/crates") != protocol["product_rust_tree"]
        or git("status", "--porcelain", "--untracked-files=all")
    ):
        raise ValueError("Rust contention clean product source changed")


def audit_campaign(
    directory: Path, *, protocol_path: Path, checkout: Path, manifest: Path,
    lock: Path, binary: Path,
) -> dict:
    """Recompute all process, environment, timing and reopened-state claims."""
    protocol = json.loads(protocol_path.read_text())
    assets = {
        "consumer_source": SOURCE, "auditor": AUDITOR, "campaign_runner": CAMPAIGN,
        "external_manifest": manifest, "external_lock": lock,
        "compiled_binary": binary,
    }
    if protocol.get("status") != "FROZEN_RUST_S02_CONTENTION" or any(
        protocol["frozen_sha256"].get(name) != sha(path) for name, path in assets.items()
    ):
        raise ValueError("Rust contention frozen inputs changed")
    verify_product(checkout, protocol)
    if sha(manifest.parent / "src/main.rs") != sha(SOURCE):
        raise ValueError("Rust contention external source changed")
    attempts_path = directory / "attempts.json"
    attempts = json.loads(attempts_path.read_text())
    summary = json.loads((directory / "summary.json").read_text())
    if (
        summary.get("status") != "VALID_RUST_S02_CONTENTION_CAMPAIGN"
        or summary.get("protocol_sha256") != sha(protocol_path)
        or summary.get("attempts_sha256") != sha(attempts_path)
        or len(attempts) != protocol["measured_processes"]
        or [row.get("label") for row in attempts] != [
            f"sample-{number:03d}" for number in range(1, protocol["measured_processes"] + 1)
        ]
    ):
        raise ValueError("Rust contention campaign summary or attempts changed")
    whole = []
    overlap = []
    warnings = []
    peak_rss = []
    for row in attempts:
        label = row["label"]
        stem = directory / label
        stdout = Path(str(stem) + ".stdout")
        stderr = Path(str(stem) + ".stderr")
        database = Path(str(stem) + ".sqlite")
        resource = Path(str(stem) + ".resource")
        audit_path = Path(str(stem) + ".audit.json")
        command = json.loads(Path(str(stem) + ".command.json").read_text())
        validate_command(command, label)
        if (
            row.get("exit_code") != 0 or row.get("valid") is not True
            or stderr.read_text()
        ):
            raise ValueError(f"Rust contention {label} process exit or stderr changed")
        raw_resources = resource_report(resource)
        if row.get("resource") != raw_resources:
            raise ValueError(f"Rust contention {label} resource claim changed")
        run_warnings = environment_warnings(row["environment"], raw_resources)
        receipt = parse_stdout(stdout)
        checked = run_audit.validate_receipt(receipt, database)
        if (
            json.loads(audit_path.read_text()) != checked
            or row.get("whole_product_ns") != checked["whole_product_ns"]
            or row.get("actual_overlap_cycles") != checked["actual_overlap_cycles"]
            or row.get("stdout_sha256") != sha(stdout)
            or row.get("database_sha256") != sha(database)
            or row.get("resource_sha256") != sha(resource)
            or row.get("audit_sha256") != sha(audit_path)
        ):
            raise ValueError(f"Rust contention {label} raw or state claim changed")
        whole.append(checked["whole_product_ns"])
        overlap.append(checked["actual_overlap_cycles"])
        peak_rss.append(raw_resources["peak_rss_kib"])
        if run_warnings:
            warnings.append(label)
    recomputed = {
        "valid_samples": len(whole), "total_attempts": len(attempts),
        "whole_median_ns": int(statistics.median(whole)),
        "whole_min_ns": min(whole), "whole_max_ns": max(whole),
        "overlap_min": min(overlap), "overlap_max": max(overlap),
        "warning_samples": warnings, "source_sha": protocol["product_source_sha"],
    }
    if any(summary.get(key) != value for key, value in recomputed.items()):
        raise ValueError("Rust contention campaign statistic changed")
    return {
        "status": "VALID_RUST_S02_CONTENTION_INDEPENDENT_AUDIT",
        "protocol_sha256": sha(protocol_path),
        "summary_sha256": sha(directory / "summary.json"),
        "attempts_sha256": sha(attempts_path),
        **recomputed,
        "peak_rss_kib_min": min(peak_rss),
        "peak_rss_kib_max": max(peak_rss),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("campaign", "protocol", "checkout", "manifest", "lock", "binary", "output"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    result = audit_campaign(
        args.campaign.resolve(), protocol_path=args.protocol.resolve(),
        checkout=args.checkout.resolve(), manifest=args.manifest.resolve(),
        lock=args.lock.resolve(), binary=args.binary.resolve(),
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"])


if __name__ == "__main__":
    main()
