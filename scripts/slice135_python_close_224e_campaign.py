#!/usr/bin/env python3
"""Run the frozen installed-Python S02-L paired lifecycle campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
BLOCK_RUNNER = ROOT / "scripts/slice135_python_close_block.py"
LIFECYCLE_RUNNER = ROOT / "scripts/slice135_python_close_memory.py"
PILOT_HELPER = ROOT / "scripts/slice135_pilot.py"
AUDITOR = ROOT / "scripts/slice135_python_close_audit.py"
WHEEL = {
    "baseline": PLAN
    / "results/2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl",
    "candidate": Path(
        "/tmp/slice135-current-wheel-224e44/"
        "fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl"
    ),
}
ORDER = [
    ["baseline", "candidate"],
    ["candidate", "baseline"],
    ["baseline", "candidate"],
    ["candidate", "baseline"],
    ["baseline", "candidate"],
]


def sha(path: Path) -> str:
    """Hash exact bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_protocol(protocol: dict, root: Path) -> None:
    """Reject drift from the source-bound subset frozen before paired timing."""
    plan = root / "dev/plans/0.8.27/features/slice-135"
    if (
        protocol.get("schema_version") != 1
        or protocol.get("status") != "FROZEN_S02_L_PYTHON_PAIRED"
    ):
        raise ValueError("S02-L protocol not frozen")
    if (
        protocol.get("samples_per_block") != 20
        or protocol.get("pairs") != 5
        or protocol.get("total_valid_samples_per_version") != 100
        or protocol.get("warmup_per_block") != 1
    ):
        raise ValueError("S02-L sample count changed")
    if protocol.get("pair_order") != ORDER:
        raise ValueError("S02-L pair order changed")
    if protocol.get("minimum_idle_seconds_between_blocks", 0) < 5:
        raise ValueError("S02-L idle interval changed")
    for key, path in (
        ("block_runner_sha256", root / "scripts/slice135_python_close_block.py"),
        ("lifecycle_runner_sha256", root / "scripts/slice135_python_close_memory.py"),
        ("pilot_helper_sha256", root / "scripts/slice135_pilot.py"),
        ("audit_sha256", root / "scripts/slice135_python_close_audit.py"),
        ("baseline_noise_audit_sha256", plan / protocol["baseline_noise_evidence"]),
    ):
        if sha(path) != protocol.get(key):
            raise ValueError(f"{key} changed")
    for role, path in WHEEL.items():
        artifact = protocol.get(role, {})
        if len(artifact.get("source_sha", "")) != 40 or sha(path) != artifact.get(
            "wheel_sha256"
        ):
            raise ValueError(f"{role} source or wheel changed")
    noise = json.loads((plan / protocol["baseline_noise_evidence"]).read_text())
    if (
        noise.get("status") != "VALID_BASELINE_S02_L_NOISE_PILOT"
        or noise.get("valid_cycles") != 100
    ):
        raise ValueError("baseline pilot not qualified")
    for key in ("source_sha", "wheel_sha256"):
        if noise[key] != protocol["baseline"][key]:
            raise ValueError(f"baseline pilot {key} differs")


def run_campaign(
    protocol_path: Path,
    *,
    baseline_checkout: Path,
    candidate_checkout: Path,
    baseline_python: Path,
    candidate_python: Path,
    output: Path,
) -> None:
    """Execute ten ordered blocks and preserve every failed or valid attempt."""
    protocol = json.loads(protocol_path.read_text())
    validate_protocol(protocol, ROOT)
    for path in (baseline_python, candidate_python):
        if not path.is_file():
            raise ValueError(f"installed Python missing: {path}")
    output.mkdir(parents=True, exist_ok=False)
    (output / "protocol.json").write_bytes(protocol_path.read_bytes())
    campaign_manifest = {
        "schema_version": 1,
        "status": "RUNNING_S02_L_PAIRED",
        "protocol_sha256": sha(protocol_path),
        "campaign_runner_sha256": sha(Path(__file__)),
        "baseline_checkout": str(baseline_checkout),
        "candidate_checkout": str(candidate_checkout),
        "baseline_python": str(baseline_python),
        "candidate_python": str(candidate_python),
    }
    (output / "campaign-manifest.json").write_text(
        json.dumps(campaign_manifest, indent=2) + "\n"
    )
    order_path = output / "run-order.jsonl"
    for pair_index, pair in enumerate(protocol["pair_order"], 1):
        for position, role in enumerate(pair, 1):
            if order_path.exists():
                time.sleep(protocol["minimum_idle_seconds_between_blocks"])
            label = f"{pair_index:02d}-{position:02d}-{role}"
            checkout = baseline_checkout if role == "baseline" else candidate_checkout
            python = baseline_python if role == "baseline" else candidate_python
            command = [
                sys.executable,
                str(BLOCK_RUNNER),
                "--checkout",
                str(checkout),
                "--source-sha",
                protocol[role]["source_sha"],
                "--wheel",
                str(WHEEL[role]),
                "--wheel-sha256",
                protocol[role]["wheel_sha256"],
                "--venv-python",
                str(python),
                "--samples",
                str(protocol["samples_per_block"]),
                "--output-dir",
                str(output / label),
                "--role",
                role,
            ]
            started = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            result = subprocess.run(
                command, cwd=ROOT, capture_output=True, text=True, check=False
            )
            ended = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            (output / f"{label}.driver.stdout").write_text(result.stdout)
            (output / f"{label}.driver.stderr").write_text(result.stderr)
            entry = {
                "pair": pair_index,
                "position": position,
                "role": role,
                "label": label,
                "started_utc": started,
                "ended_utc": ended,
                "exit_code": result.returncode,
                "summary_sha256": sha(output / label / "summary.json")
                if (output / label / "summary.json").is_file()
                else None,
            }
            with order_path.open("a") as stream:
                stream.write(json.dumps(entry) + "\n")
            print(
                f"{label}: {result.stdout.strip()} exit={result.returncode}", flush=True
            )
            if result.returncode:
                raise RuntimeError(f"{label} invalid; retained in {output}")
    campaign_manifest["status"] = "PRODUCER_COMPLETE_S02_L_PAIRED"
    (output / "campaign-manifest.json").write_text(
        json.dumps(campaign_manifest, indent=2) + "\n"
    )


def main() -> None:
    """Run all pre-registered lifecycle pairs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--baseline-checkout", required=True, type=Path)
    parser.add_argument("--candidate-checkout", required=True, type=Path)
    parser.add_argument("--baseline-python", required=True, type=Path)
    parser.add_argument("--candidate-python", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    run_campaign(
        args.protocol.resolve(),
        baseline_checkout=args.baseline_checkout.resolve(),
        candidate_checkout=args.candidate_checkout.resolve(),
        baseline_python=args.baseline_python.absolute(),
        candidate_python=args.candidate_python.absolute(),
        output=args.output_dir.absolute(),
    )


if __name__ == "__main__":
    main()
