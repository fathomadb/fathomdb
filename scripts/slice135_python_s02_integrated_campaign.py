#!/usr/bin/env python3
"""Run a source-bound frozen installed-Python S02 paired campaign."""

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
PLAN = ROOT / "dev/plans/0.8.27/features/slice-135"
BLOCK = ROOT / "scripts/slice135_python_s02_block.py"
RUNNER = ROOT / "scripts/slice135_python_s02_timing.py"
S01 = ROOT / "scripts/slice135_python_s01.py"
S02 = ROOT / "scripts/slice135_python_s02.py"
PILOT = ROOT / "scripts/slice135_pilot.py"
WHEELS = {
    "baseline": PLAN
    / "results/2026-10-07-python-wheel-baseline/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl",
    "candidate": PLAN
    / "results/2026-10-07-python-integrated-candidate/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl",
}
ORDER = [
    ["baseline", "candidate"],
    ["candidate", "baseline"],
    ["baseline", "candidate"],
    ["candidate", "baseline"],
    ["baseline", "candidate"],
]


def sha(path: Path) -> str:
    """Hash exact file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_protocol(
    protocol: dict, *, wheel_paths: dict[str, Path] | None = None
) -> None:
    """Reject drift from the pre-registered S02 timing and artifact design."""
    wheels = WHEELS if wheel_paths is None else wheel_paths
    if (
        protocol.get("schema_version") != 1
        or protocol.get("status") != "FROZEN_S02_PYTHON_PAIRED"
    ):
        raise ValueError("integrated S02 protocol not frozen")
    if (
        protocol.get("samples_per_block") != 20
        or protocol.get("pairs") != 5
        or protocol.get("total_valid_samples_per_version") != 100
        or protocol.get("warmup_per_block") != 1
    ):
        raise ValueError("integrated S02 sample count changed")
    if protocol.get("pair_order") != ORDER:
        raise ValueError("integrated S02 pair order changed")
    if protocol.get("minimum_idle_seconds_between_blocks", 0) < 5:
        raise ValueError("integrated S02 idle interval changed")
    campaign_hash = protocol.get("campaign_runner_sha256")
    if campaign_hash is not None and campaign_hash != sha(Path(__file__)):
        raise ValueError("campaign runner differs from frozen S02 protocol")
    snapshot = protocol.get("candidate_product_snapshot")
    if snapshot is not None and snapshot.get("source_sha") != protocol.get(
        "candidate", {}
    ).get("source_sha"):
        raise ValueError("candidate product snapshot differs from protocol")
    for key, path in (
        ("block_runner_sha256", BLOCK),
        ("timed_runner_sha256", RUNNER),
        ("s01_helper_sha256", S01),
        ("s02_helper_sha256", S02),
        ("pilot_helper_sha256", PILOT),
    ):
        if sha(path) != protocol.get(key):
            raise ValueError(f"integrated S02 {key} changed")
    pilot = PLAN / protocol["baseline_noise_evidence"]
    if sha(pilot) != protocol.get("baseline_noise_audit_sha256"):
        raise ValueError("baseline noise audit changed")
    noise = json.loads(pilot.read_text())
    if noise.get("status") != "CONTROLLED_BASELINE_ONLY_NO_PAIR":
        raise ValueError("baseline noise pilot invalid")
    for role in ("baseline", "candidate"):
        identity = protocol.get(role, {})
        if len(identity.get("source_sha", "")) != 40 or sha(
            wheels[role]
        ) != identity.get("wheel_sha256"):
            raise ValueError(f"{role} wheel or source identity changed")


def now() -> str:
    """Return a UTC time for the retained campaign manifest."""
    return datetime.now(timezone.utc).isoformat()


def run_campaign(
    protocol_path: Path,
    *,
    baseline_checkout: Path,
    candidate_checkout: Path,
    baseline_python: Path,
    candidate_python: Path,
    output: Path,
    wheel_paths: dict[str, Path] | None = None,
    expected_protocol_sha256: str | None = None,
) -> None:
    """Run alternating blocks serially, preserving every attempted block."""
    wheels = WHEELS if wheel_paths is None else wheel_paths
    protocol_hash = sha(protocol_path)
    if expected_protocol_sha256 is not None and protocol_hash != expected_protocol_sha256:
        raise ValueError("S02 protocol bytes differ from declared freeze")
    protocol = json.loads(protocol_path.read_text())
    validate_protocol(protocol, wheel_paths=wheels)
    for path in (baseline_python, candidate_python):
        if not path.is_file():
            raise ValueError(f"installed Python missing: {path}")
    output.mkdir(parents=True, exist_ok=False)
    (output / "protocol.json").write_bytes(protocol_path.read_bytes())
    (output / "campaign.py").write_bytes(Path(__file__).read_bytes())
    run = {
        "schema_version": 1,
        "status": "RUNNING",
        "protocol_sha256": protocol_hash,
        "campaign_runner_sha256": sha(Path(__file__)),
        "started_utc": now(),
        "pair_order": protocol["pair_order"],
        "samples_per_block": protocol["samples_per_block"],
        "last_complete_pair": 0,
        "baseline_checkout": str(baseline_checkout),
        "candidate_checkout": str(candidate_checkout),
        "baseline_python": str(baseline_python),
        "candidate_python": str(candidate_python),
        "baseline_wheel": str(wheels["baseline"]),
        "candidate_wheel": str(wheels["candidate"]),
    }
    manifest_path = output / "run-manifest.json"
    manifest_path.write_text(json.dumps(run, indent=2) + "\n")
    order_path = output / "run-order.jsonl"
    for pair_index, pair in enumerate(protocol["pair_order"], 1):
        for position, role in enumerate(pair, 1):
            if sha(protocol_path) != protocol_hash:
                raise ValueError("frozen S02 protocol changed during campaign")
            if order_path.exists():
                time.sleep(protocol["minimum_idle_seconds_between_blocks"])
            label = f"pair-{pair_index:02d}-{role}"
            checkout = baseline_checkout if role == "baseline" else candidate_checkout
            python = baseline_python if role == "baseline" else candidate_python
            command = [
                sys.executable,
                str(BLOCK),
                "--checkout",
                str(checkout),
                "--source-sha",
                protocol[role]["source_sha"],
                "--wheel",
                str(wheels[role]),
                "--wheel-sha256",
                protocol[role]["wheel_sha256"],
                "--venv-python",
                str(python),
                "--samples",
                str(protocol["samples_per_block"]),
                "--output-dir",
                str(output / label),
                "--comparison-protocol",
                str(protocol_path),
                "--role",
                role,
            ]
            (output / f"{label}.driver-command.json").write_text(
                json.dumps(command, indent=2) + "\n"
            )
            started = now()
            try:
                result = subprocess.run(
                    command,
                    cwd=ROOT,
                    capture_output=True,
                    text=True,
                    check=False,
                    timeout=600,
                )
                stdout, stderr, exit_code = (
                    result.stdout,
                    result.stderr,
                    result.returncode,
                )
            except subprocess.TimeoutExpired as error:
                stdout = (error.stdout or b"").decode(errors="replace")
                stderr = (error.stderr or b"").decode(errors="replace")
                exit_code = 124
            ended = now()
            (output / f"{label}.driver.stdout").write_text(stdout)
            (output / f"{label}.driver.stderr").write_text(stderr)
            entry = {
                "pair": pair_index,
                "position": position,
                "role": role,
                "label": label,
                "started_utc": started,
                "ended_utc": ended,
                "exit_code": exit_code,
                "summary_sha256": sha(output / label / "summary.json")
                if (output / label / "summary.json").is_file()
                else None,
            }
            with order_path.open("a") as stream:
                stream.write(json.dumps(entry) + "\n")
            print(f"{label}: {stdout.strip()} exit={exit_code}", flush=True)
            if exit_code:
                run["status"] = "INVALID_BLOCK_RETAINED"
                run["updated_utc"] = now()
                manifest_path.write_text(json.dumps(run, indent=2) + "\n")
                raise RuntimeError(f"{label} invalid; preserved in {output}")
        run["last_complete_pair"] = pair_index
        run["updated_utc"] = now()
        manifest_path.write_text(json.dumps(run, indent=2) + "\n")
    run["status"] = "FULL_CAMPAIGN_COMPLETE"
    run["finished_utc"] = now()
    manifest_path.write_text(json.dumps(run, indent=2) + "\n")


def main() -> None:
    """Execute the integrated S02 subset from its frozen protocol."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--baseline-checkout", required=True, type=Path)
    parser.add_argument("--candidate-checkout", required=True, type=Path)
    parser.add_argument("--baseline-python", required=True, type=Path)
    parser.add_argument("--candidate-python", required=True, type=Path)
    parser.add_argument("--baseline-wheel", type=Path, default=WHEELS["baseline"])
    parser.add_argument("--candidate-wheel", type=Path, default=WHEELS["candidate"])
    parser.add_argument("--protocol-sha256")
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()
    run_campaign(
        args.protocol.resolve(),
        baseline_checkout=args.baseline_checkout.resolve(),
        candidate_checkout=args.candidate_checkout.resolve(),
        baseline_python=args.baseline_python.absolute(),
        candidate_python=args.candidate_python.absolute(),
        output=args.output_dir.absolute(),
        wheel_paths={
            "baseline": args.baseline_wheel.resolve(),
            "candidate": args.candidate_wheel.resolve(),
        },
        expected_protocol_sha256=args.protocol_sha256,
    )


if __name__ == "__main__":
    main()
