#!/usr/bin/env python3
"""Capture one source-bound S03 installed-Python comparison block."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess

import slice135_python_s01_block as s01block
import slice135_python_s03_audit as audit


ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/slice135_python_s03.py"
MIN_DISK_FREE_BYTES = 1_073_741_824
PILOT = s01block.PILOT


def digest(path: Path) -> str:
    """Hash the bytes used for this block."""
    return audit.digest(path)


def write_json(path: Path, value: object) -> None:
    """Retain a deterministic JSON record."""
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def validate_spec(
    spec: dict, *, role: str, size: int, source_sha: str, wheel_sha256: str
) -> None:
    """Refuse timing outside the previously frozen subset identity."""
    if (
        spec.get("schema_version") != 1
        or spec.get("status") != "FROZEN_S03_PYTHON_PAIRED"
        or spec.get("sizes") != [32, 256]
        or spec.get("samples_per_case") != 100
        or role not in ("baseline", "candidate")
        or size not in (32, 256)
    ):
        raise ValueError("S03 comparison protocol is not frozen or cell differs")
    identity = spec.get(role)
    if not isinstance(identity, dict) or (
        identity.get("source_sha"),
        identity.get("wheel_sha256"),
    ) != (source_sha, wheel_sha256):
        raise ValueError(f"{role} source or wheel identity mismatch")
    expected = {
        str(path.relative_to(ROOT)): digest(path)
        for path in (
            RUNNER,
            audit.S01,
            audit.S02,
            audit.FIXTURE,
            ROOT / "scripts/slice135_python_s01_block.py",
        )
    }
    if spec.get("input_sha256") != expected:
        raise ValueError("S03 workload or fixture bytes differ from frozen protocol")


def run_block(
    *,
    spec_path: Path,
    role: str,
    checkout: Path,
    wheel: Path,
    venv_python: Path,
    size: int,
    output: Path,
) -> dict:
    """Measure and independently audit one fresh-database version block."""
    spec = json.loads(spec_path.read_text())
    identity = spec[role]
    source_sha = identity["source_sha"]
    wheel_sha256 = identity["wheel_sha256"]
    validate_spec(
        spec,
        role=role,
        size=size,
        source_sha=source_sha,
        wheel_sha256=wheel_sha256,
    )
    if not PILOT.GNU_TIME.is_file():
        raise ValueError("GNU Time resource capture unavailable")
    source = s01block._verify_source(checkout, source_sha)
    if digest(wheel) != wheel_sha256:
        raise ValueError(f"{role} wheel bytes differ from frozen protocol")
    if not venv_python.is_file():
        raise ValueError(f"{role} installed Python missing")
    output.mkdir(parents=True, exist_ok=False)
    write_json(output / "source.json", source)
    write_json(
        output / "block-protocol.json",
        {
            "schema_version": 1,
            "status": "FROZEN_S03_PYTHON_PAIRED_BLOCK",
            "comparison_protocol_sha256": digest(spec_path),
            "role": role,
            "size": size,
            "samples_per_case": 100,
            "source_sha": source_sha,
            "wheel_sha256": wheel_sha256,
            "runner_sha256": digest(RUNNER),
            "block_runner_sha256": digest(Path(__file__)),
            "auditor_sha256": digest(Path(audit.__file__)),
            "timing_boundary": "installed Python call through materialized observation",
            "timing_mode": "unprofiled",
            "min_disk_free_bytes": MIN_DISK_FREE_BYTES,
        },
    )
    start = PILOT.inventory(output)
    resource_path = output / "resource.txt"
    command = [
        str(PILOT.GNU_TIME),
        "-o",
        str(resource_path),
        "-f",
        PILOT.RESOURCE_FORMAT,
        str(venv_python),
        str(RUNNER),
        "--size",
        str(size),
        "--repetitions",
        "100",
        "--wheel",
        str(wheel),
        "--wheel-sha256",
        wheel_sha256,
        "--source-sha",
        source_sha,
        "--output",
        str(output / "workload"),
    ]
    write_json(output / "run-command.json", command)
    environment = {
        **os.environ,
        "FATHOMDB_EMBED_DEVICE": "cpu",
        "PYTHONOPTIMIZE": "0",
        "PYTHONDONTWRITEBYTECODE": "1",
    }
    environment.pop("PYTHONPATH", None)
    environment.pop("VIRTUAL_ENV", None)
    invalid: list[str] = []
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            capture_output=True,
            text=True,
            env=environment,
            check=False,
            timeout=300,
        )
        (output / "run.stdout.log").write_text(result.stdout)
        (output / "run.stderr.log").write_text(result.stderr)
        if result.returncode:
            invalid.append(f"workload exited {result.returncode}")
    except subprocess.TimeoutExpired as error:
        (output / "run.stdout.log").write_bytes(error.stdout or b"")
        (output / "run.stderr.log").write_bytes(error.stderr or b"")
        invalid.append("workload timed out after 300 seconds")
    end = PILOT.inventory(output)
    resources = PILOT.read_resource_report(resource_path)
    invalid.extend(
        PILOT.environment_invalidators(start, end, MIN_DISK_FREE_BYTES, resources)
    )
    write_json(output / "resources.json", resources)
    summary = None
    if (output / "workload/raw.json").is_file():
        try:
            summary = audit.audit(
                output / "workload",
                size=size,
                source_sha=source_sha,
                wheel_sha256=wheel_sha256,
            )
            if any(cell["count"] != 100 for cell in summary["cases"].values()):
                raise ValueError("audited case count differs from frozen protocol")
            write_json(output / "independent-audit.json", summary)
        except (KeyError, TypeError, ValueError) as error:
            invalid.append(f"independent audit failed: {error}")
    else:
        invalid.append("raw workload output missing")
    warnings = PILOT.environment_warnings(start, end, resources)
    write_json(
        output / "environment.json",
        {"start": start, "end": end, "invalidators": invalid, "warnings": warnings},
    )
    attempt = {
        "status": "VALID_S03_PAIRED_BLOCK" if not invalid else "INVALID_BLOCK",
        "invalidators": invalid,
        "warnings": warnings,
        "comparison_protocol_sha256": digest(spec_path),
        "block_protocol_sha256": digest(output / "block-protocol.json"),
        "source_sha": source_sha,
        "wheel_sha256": wheel_sha256,
        "runner_sha256": digest(RUNNER),
        "block_runner_sha256": digest(Path(__file__)),
        "summary_sha256": digest(output / "independent-audit.json")
        if summary
        else None,
    }
    write_json(output / "attempt.json", attempt)
    return attempt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec-path", type=Path, required=True)
    parser.add_argument("--role", choices=("baseline", "candidate"), required=True)
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--venv-python", type=Path, required=True)
    parser.add_argument("--size", type=int, choices=(32, 256), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    attempt = run_block(**vars(args))
    print(json.dumps({"status": attempt["status"], "output": str(args.output)}))
    return 0 if attempt["status"] == "VALID_S03_PAIRED_BLOCK" else 1


if __name__ == "__main__":
    raise SystemExit(main())
