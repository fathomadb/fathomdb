#!/usr/bin/env python3
"""Run the 0.8.26 CE candidate against the recorded 0.8.25 baseline."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ARTIFACT = ROOT / "scripts" / "release" / "slice72_ce_artifact.py"
PROFILE = ROOT / "scripts" / "release" / "run-slice72-ce-profile.py"
VERIFY = ROOT / "scripts" / "release" / "verify-slice72-ce-profile.py"
BASELINE_ROOT = ROOT / "dev" / "plans" / "runs" / "0.8.25-slice-72"
MANIFEST_SHA256 = "4535895d0ae0e2bb1febd61e036c9013f7c607114b250dab3458980fa6cfaae9"
BASELINE_SHA256 = {
    "cpu": "c093a4e131515496552f90bc6e8adf7d3b864b371815c4ceeab408c3bb4c2b39",
    "cuda": "e40fbccc9c357d6ada801e234b8a9bf372418195d1cb32b6646cd8d28cc1196f",
}


def digest(path: Path) -> str:
    """Return one frozen input's content identity."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(
    command: list[str], stdout_path: Path, *, check: bool = True
) -> subprocess.CompletedProcess[str]:
    """Run one Slice 72 command and retain stdout and stderr separately."""
    result = subprocess.run(
        command, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    stdout_path.write_text(result.stdout, encoding="utf-8")
    stdout_path.with_suffix(".stderr.log").write_text(result.stderr, encoding="utf-8")
    if check and result.returncode:
        raise RuntimeError(f"command failed; see {stdout_path}")
    return result


def ratio_only_failure(stderr: str) -> str | None:
    """Return the informational ratio finding, excluding integrity failures."""
    match = re.fullmatch(
        r"FAIL Slice 72 CE profile: "
        r"(cpu|cuda) (standalone|engine) p95 regression ([0-9.]+) exceeds ([0-9.]+)\n?",
        stderr,
    )
    return match.group(0).removeprefix("FAIL Slice 72 CE profile: ").strip() if match else None


def main() -> int:
    """Build/profile both candidate devices, then use the existing verifier."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-manifest", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--cache-root", type=Path, required=True)
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument("--cuda-uuid", required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args()
    output = args.output_root.resolve()
    if output.exists():
        parser.error("CE output root must be new")
    output.mkdir(parents=True)
    if digest(args.base_manifest) != MANIFEST_SHA256:
        raise ValueError("frozen CE manifest identity drifted")
    baseline_cells = {
        "cpu": BASELINE_ROOT / "baseline-cpu" / "baseline-cpu.json",
        "cuda": BASELINE_ROOT / "baseline-cuda" / "baseline-cuda.json",
    }
    for device, path in baseline_cells.items():
        if digest(path) != BASELINE_SHA256[device]:
            raise ValueError(f"frozen CE {device} baseline identity drifted")
    manifest = json.loads(args.base_manifest.read_text(encoding="utf-8"))
    manifest["candidate_sha"] = args.candidate_sha
    manifest["cuda"]["selected_uuid"] = args.cuda_uuid
    manifest["cuda"]["visible_selector"] = args.cuda_uuid
    manifest_path = output / "manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    candidate_cells: list[Path] = []
    for device in ("cpu", "cuda"):
        artifact_dir = output / f"candidate-{device}-artifact"
        run(
            [
                sys.executable,
                str(ARTIFACT),
                "--manifest",
                str(manifest_path),
                "--source-root",
                str(args.source_root),
                "--role",
                "candidate",
                "--device",
                device,
                "--output-dir",
                str(artifact_dir),
            ],
            output / "logs" / f"build-{device}.log",
        )
        cell_dir = output / f"candidate-{device}"
        command = [
            sys.executable,
            str(PROFILE),
            "--manifest",
            str(manifest_path),
            "--artifact-receipt",
            str(artifact_dir / "artifact-receipt.json"),
            "--cache-root",
            str(args.cache_root),
            "--role",
            "candidate",
            "--device",
            device,
            "--output-dir",
            str(cell_dir),
        ]
        if device == "cuda":
            command.extend(["--cuda-visible-devices", args.cuda_uuid])
        run(command, output / "logs" / f"profile-{device}.log")
        candidate_cells.append(cell_dir / f"candidate-{device}.json")

    cells = [
        baseline_cells["cpu"],
        baseline_cells["cuda"],
        *candidate_cells,
    ]
    receipt = output / "receipt.json"
    verifier = run(
        [
            sys.executable,
            str(VERIFY),
            "--manifest",
            str(manifest_path),
            "--cells",
            *(str(path) for path in cells),
            "--output",
            str(receipt),
        ],
        output / "logs" / "verify.log",
        check=False,
    )
    verifier_status = {
        "schema_version": "fathomdb.performance-gauntlet.ce-verifier/v1",
        "returncode": verifier.returncode,
        "stdout": str(output / "logs" / "verify.log"),
        "stderr": str(output / "logs" / "verify.stderr.log"),
        "ratio_outcome": "within-1.10",
    }
    if verifier.returncode:
        ratio_failure = ratio_only_failure(verifier.stderr)
        if ratio_failure is None:
            raise RuntimeError("CE verifier integrity failure; see verifier logs")
        verifier_status["ratio_outcome"] = ratio_failure
    (output / "verifier-status.json").write_text(
        json.dumps(verifier_status, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(receipt if receipt.is_file() else output / "verifier-status.json")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, KeyError, ValueError) as error:
        print(f"FAIL CE profile: {error}", file=sys.stderr)
        raise SystemExit(1) from error
