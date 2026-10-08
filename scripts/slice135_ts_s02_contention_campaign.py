#!/usr/bin/env python3
"""Run the frozen installed-TypeScript S02 contention paired campaign."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
BLOCK = ROOT / "scripts/slice135_ts_s02_contention_block.py"
RUNNER = ROOT / "scripts/slice135_ts_s02_contention.mjs"
AUDITOR = ROOT / "scripts/slice135_ts_s02_contention_audit.py"
S01_HELPER = ROOT / "scripts/slice135_ts_s01.mjs"
S02_HELPER = ROOT / "scripts/slice135_ts_s02.mjs"
PRIOR = ROOT / "dev/plans/0.8.27/features/slice-135/s02-ts-vector-repaired-comparison-protocol.json"


def sha(path: Path) -> str:
    """Hash exact frozen files and retained summaries."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def order_for_pairs(pairs: int) -> list[tuple[int, str]]:
    """Alternate the first version in each preassigned pair."""
    if pairs < 1:
        raise ValueError("at least one version pair required")
    return [
        (number, role)
        for number in range(1, pairs + 1)
        for role in (("baseline", "candidate") if number % 2 else ("candidate", "baseline"))
    ]


def validate_freeze(protocol: dict, artifacts: dict[str, Path]) -> None:
    """Reject altered schedule, code or installed archive identity."""
    if (
        protocol.get("status") != "FROZEN_S02_TS_CONTENTION_PAIRED"
        or protocol.get("pairs") != 5
        or protocol.get("samples_per_block") != 20
        or protocol.get("pair_order") != [list(item) for item in order_for_pairs(5)]
        or protocol.get("minimum_idle_seconds") != 20
    ):
        raise ValueError("contention campaign protocol or schedule changed")
    for key, artifact in artifacts.items():
        if protocol["frozen_sha256"].get(key) != sha(artifact):
            raise ValueError(f"contention frozen {key} changed")


def run_campaign(
    *, protocol_path: Path, output: Path, node: Path,
    baseline_checkout: Path, baseline_install: Path,
    baseline_main: Path, baseline_platform: Path, baseline_reference: Path,
    candidate_checkout: Path, candidate_install: Path,
    candidate_main: Path, candidate_platform: Path, candidate_reference: Path,
) -> dict:
    """Retain ten serial block commands, order and exact outcomes."""
    protocol = json.loads(protocol_path.read_text())
    validate_freeze(protocol, {
        "runner": RUNNER, "auditor": AUDITOR, "block_runner": BLOCK,
        "campaign_runner": Path(__file__),
        "s01_helper": S01_HELPER, "s02_helper": S02_HELPER,
        "prior_protocol": PRIOR,
        "baseline_main_archive": baseline_main,
        "baseline_platform_archive": baseline_platform,
        "baseline_reference_manifest": baseline_reference,
        "candidate_main_archive": candidate_main,
        "candidate_platform_archive": candidate_platform,
        "candidate_reference_manifest": candidate_reference,
    })
    roles = {
        "baseline": (baseline_checkout, baseline_install,
                     baseline_main, baseline_platform, baseline_reference),
        "candidate": (candidate_checkout, candidate_install,
                      candidate_main, candidate_platform, candidate_reference),
    }
    output.mkdir(parents=True, exist_ok=False)
    order_path = output / "run-order.jsonl"
    results = []
    last_end = None
    for pair, role in order_for_pairs(protocol["pairs"]):
        if last_end is not None:
            remaining = protocol["minimum_idle_seconds"] - (time.monotonic() - last_end)
            if remaining > 0:
                time.sleep(remaining)
        checkout, install, main, platform, reference = roles[role]
        block_output = output / f"pair-{pair:02d}-{role}"
        command = [
            sys.executable, str(BLOCK),
            "--role", role, "--checkout", str(checkout),
            "--install-root", str(install), "--node", str(node),
            "--main-archive", str(main),
            "--platform-archive", str(platform),
            "--reference-manifest", str(reference),
            "--samples", str(protocol["samples_per_block"]),
            "--contention-protocol", str(protocol_path),
            "--output", str(block_output),
        ]
        start_ns = time.monotonic_ns()
        start_utc = datetime.now(timezone.utc).isoformat()
        try:
            child = subprocess.run(
                command, capture_output=True, text=True, check=False,
                timeout=protocol["block_timeout_seconds"],
            )
            stdout, stderr, exit_code = child.stdout, child.stderr, child.returncode
        except subprocess.TimeoutExpired as error:
            stdout = (error.stdout or b"").decode(errors="replace")
            stderr = (error.stderr or b"").decode(errors="replace")
            exit_code = 124
        end_ns = time.monotonic_ns()
        last_end = time.monotonic()
        prefix = output / block_output.name
        (output / f"{prefix.name}.driver.stdout").write_text(stdout)
        (output / f"{prefix.name}.driver.stderr").write_text(stderr)
        (output / f"{prefix.name}.driver.command.json").write_text(json.dumps(command) + "\n")
        summary_path = block_output / "summary.json"
        row = {
            "pair": pair, "role": role, "block": block_output.name,
            "start_monotonic_ns": start_ns, "end_monotonic_ns": end_ns,
            "start_utc": start_utc, "exit_code": exit_code,
            "summary_sha256": sha(summary_path) if summary_path.is_file() else None,
        }
        with order_path.open("a") as stream:
            stream.write(json.dumps(row) + "\n")
        results.append(row)
        if (
            exit_code != 0
            or not summary_path.is_file()
            or json.loads(summary_path.read_text()).get("status") != "VALID_S02_TS_CONTENTION_BLOCK"
        ):
            break
    valid = len(results) == 10 and all(row["exit_code"] == 0 for row in results)
    summary = {
        "status": "VALID_S02_TS_CONTENTION_CAMPAIGN" if valid else "INVALID_S02_TS_CONTENTION_CAMPAIGN",
        "protocol_sha256": sha(protocol_path),
        "run_order_sha256": sha(order_path),
        "blocks": len(results), "pairs": protocol["pairs"],
        "samples_per_block": protocol["samples_per_block"],
    }
    (output / "campaign-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--node", required=True, type=Path)
    for role in ("baseline", "candidate"):
        for name in ("checkout", "install", "main", "platform", "reference"):
            parser.add_argument(f"--{role}-{name}", required=True, type=Path)
    args = parser.parse_args()
    result = run_campaign(
        protocol_path=args.protocol.resolve(), output=args.output.absolute(),
        node=args.node.absolute(),
        baseline_checkout=args.baseline_checkout.resolve(),
        baseline_install=args.baseline_install.resolve(),
        baseline_main=args.baseline_main.resolve(),
        baseline_platform=args.baseline_platform.resolve(),
        baseline_reference=args.baseline_reference.resolve(),
        candidate_checkout=args.candidate_checkout.resolve(),
        candidate_install=args.candidate_install.resolve(),
        candidate_main=args.candidate_main.resolve(),
        candidate_platform=args.candidate_platform.resolve(),
        candidate_reference=args.candidate_reference.resolve(),
    )
    print(result["status"])
    if result["status"] != "VALID_S02_TS_CONTENTION_CAMPAIGN":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
