#!/usr/bin/env python3
"""Run ten bounded, fresh-process Rust SDK shared-engine contention cases."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess

import slice135_pilot as pilot
import slice135_rust_s02_contention_audit as audit


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "scripts/slice135_rust_s02_contention.rs"
AUDITOR = ROOT / "scripts/slice135_rust_s02_contention_audit.py"
MIN_DISK_FREE_BYTES = 1_073_741_824


def sha(path: Path) -> str:
    """Hash exact source, executable and raw receipt bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_freeze(protocol: dict, assets: dict[str, Path]) -> None:
    """Reject changed executable inputs or a reduced functional sample count."""
    if (
        protocol.get("status") != "FROZEN_RUST_S02_CONTENTION"
        or protocol.get("measured_processes") != 10
    ):
        raise ValueError("Rust contention protocol or sample count changed")
    for name, path in assets.items():
        if protocol.get("frozen_sha256", {}).get(name) != sha(path):
            raise ValueError(f"Rust contention frozen {name} changed")


def verify_product(checkout: Path, protocol: dict) -> None:
    """Require the clean source and Rust tree pinned before execution."""
    def git(*arguments: str) -> str:
        return subprocess.run(
            ["git", *arguments], cwd=checkout, capture_output=True,
            text=True, check=True,
        ).stdout.strip()

    if (
        git("rev-parse", "HEAD") != protocol.get("product_source_sha")
        or git("rev-parse", "HEAD:src/rust/crates") != protocol.get("product_rust_tree")
        or git("status", "--porcelain", "--untracked-files=all")
    ):
        raise ValueError("Rust contention product source differs from clean freeze")


def parse_stdout(stdout: str) -> dict:
    """Require one structured product receipt and no extra output."""
    lines = stdout.splitlines()
    prefix = "SLICE135_RUST_S02_CONTENTION "
    if len(lines) != 1 or not lines[0].startswith(prefix):
        raise ValueError("Rust contention structured receipt missing")
    return json.loads(lines[0][len(prefix):])


def run_campaign(
    *, protocol_path: Path, checkout: Path, manifest: Path, lock: Path,
    binary: Path, output: Path,
) -> dict:
    """Retain every process, database, resource and independent state audit."""
    protocol = json.loads(protocol_path.read_text())
    validate_freeze(protocol, {
        "consumer_source": SOURCE,
        "auditor": AUDITOR,
        "campaign_runner": Path(__file__),
        "external_manifest": manifest,
        "external_lock": lock,
        "compiled_binary": binary,
    })
    verify_product(checkout, protocol)
    if sha(manifest.parent / "src/main.rs") != sha(SOURCE):
        raise ValueError("Rust contention external consumer source changed")
    if str(checkout / "src/rust/crates/fathomdb-sdk") not in manifest.read_text():
        raise ValueError("Rust contention external manifest resolves another SDK")
    output.mkdir(parents=True, exist_ok=False)
    attempts = []
    for number in range(1, protocol["measured_processes"] + 1):
        label = f"sample-{number:03d}"
        database = output / f"{label}.sqlite"
        resource = output / f"{label}.resource"
        stdout_path = output / f"{label}.stdout"
        stderr_path = output / f"{label}.stderr"
        command = [
            str(pilot.GNU_TIME), "-o", str(resource), "-f", pilot.RESOURCE_FORMAT,
            str(binary), str(database),
        ]
        (output / f"{label}.command.json").write_text(json.dumps(command) + "\n")
        start = pilot.inventory(output)
        environment = {**os.environ, "FATHOMDB_EMBED_DEVICE": "cpu", "HF_HUB_OFFLINE": "1"}
        try:
            child = subprocess.run(command, capture_output=True, text=True,
                                   env=environment, check=False, timeout=120)
            stdout_path.write_text(child.stdout)
            stderr_path.write_text(child.stderr)
            exit_code = child.returncode
        except subprocess.TimeoutExpired as error:
            stdout_path.write_bytes(error.stdout or b"")
            stderr_path.write_bytes(error.stderr or b"")
            exit_code = 124
        end = pilot.inventory(output)
        resources = pilot.read_resource_report(resource)
        invalidators = pilot.environment_invalidators(
            start, end, MIN_DISK_FREE_BYTES, resources,
        )
        warnings = pilot.environment_warnings(start, end, resources)
        attempt = {
            "label": label, "exit_code": exit_code, "resource": resources,
            "environment": {"start": start, "end": end,
                            "invalidators": invalidators, "warnings": warnings},
        }
        try:
            if exit_code:
                raise ValueError(f"process exited {exit_code}")
            if stderr_path.read_text():
                raise ValueError("Rust contention unexpected stderr")
            if invalidators:
                raise ValueError(f"environment invalidators: {invalidators}")
            receipt = parse_stdout(stdout_path.read_text())
            checked = audit.validate_receipt(receipt, database)
            audit_path = output / f"{label}.audit.json"
            audit_path.write_text(json.dumps(checked, indent=2) + "\n")
            attempt.update({
                "valid": True,
                "whole_product_ns": checked["whole_product_ns"],
                "actual_overlap_cycles": checked["actual_overlap_cycles"],
                "stdout_sha256": sha(stdout_path),
                "database_sha256": sha(database),
                "resource_sha256": sha(resource),
                "audit_sha256": sha(audit_path),
            })
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            attempt.update({"valid": False, "reason": str(error)})
        attempts.append(attempt)
        (output / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
        if not attempt["valid"]:
            break
    valid = len(attempts) == protocol["measured_processes"] and all(
        row["valid"] for row in attempts
    )
    whole = [row["whole_product_ns"] for row in attempts if row["valid"]]
    overlap = [row["actual_overlap_cycles"] for row in attempts if row["valid"]]
    summary = {
        "status": "VALID_RUST_S02_CONTENTION_CAMPAIGN" if valid
                  else "INVALID_RUST_S02_CONTENTION_CAMPAIGN",
        "protocol_sha256": sha(protocol_path),
        "attempts_sha256": sha(output / "attempts.json"),
        "valid_samples": len(whole),
        "total_attempts": len(attempts),
        "whole_median_ns": int(statistics.median(whole)) if whole else None,
        "whole_min_ns": min(whole) if whole else None,
        "whole_max_ns": max(whole) if whole else None,
        "overlap_min": min(overlap) if overlap else None,
        "overlap_max": max(overlap) if overlap else None,
        "warning_samples": [row["label"] for row in attempts if row["environment"]["warnings"]],
        "source_sha": protocol["product_source_sha"],
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("protocol", "checkout", "manifest", "lock", "binary", "output"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    result = run_campaign(
        protocol_path=args.protocol.resolve(), checkout=args.checkout.resolve(),
        manifest=args.manifest.resolve(), lock=args.lock.resolve(),
        binary=args.binary.resolve(), output=args.output.absolute(),
    )
    print(result["status"])
    if result["status"] != "VALID_RUST_S02_CONTENTION_CAMPAIGN":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
