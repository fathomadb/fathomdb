#!/usr/bin/env python3
"""Run one installed-TypeScript S02 contention pilot or frozen timing block."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import subprocess

import slice135_pilot as pilot
import slice135_ts_s01_block as s01_block
import slice135_ts_s02_contention_audit as audit


ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/slice135_ts_s02_contention.mjs"
S01_HELPER = ROOT / "scripts/slice135_ts_s01.mjs"
S02_HELPER = ROOT / "scripts/slice135_ts_s02.mjs"
AUDITOR = ROOT / "scripts/slice135_ts_s02_contention_audit.py"
PRIOR = ROOT / "dev/plans/0.8.27/features/slice-135/s02-ts-vector-repaired-comparison-protocol.json"
MIN_DISK_FREE_BYTES = 1_073_741_824


def sha(path: Path) -> str:
    """Hash exact source, package or retained receipt bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nearest_rank(values: list[int], fraction: float) -> int:
    """Select the untrimmed nearest-rank statistic."""
    return sorted(values)[math.ceil(len(values) * fraction) - 1]


def validate_identity(protocol: dict, role: str, source: str, main: str, platform: str) -> None:
    """Bind a role to the existing installed TypeScript S02 package pin."""
    if protocol.get("status") != "FROZEN_TS_S02_PAIRED" or role not in protocol:
        raise ValueError("comparison protocol or role invalid")
    if protocol[role] != {
        "source_sha": source,
        "main_archive_sha256": main,
        "platform_archive_sha256": platform,
    }:
        raise ValueError("source or installed artifact identity changed")


def validate_frozen(protocol: dict, *, role: str, samples: int,
                    main_archive: Path, platform_archive: Path,
                    reference_manifest: Path) -> None:
    """Reject changed workload code, archives or paired sample rule."""
    if (
        protocol.get("status") != "FROZEN_S02_TS_CONTENTION_PAIRED"
        or role not in {"baseline", "candidate"}
        or samples != protocol.get("samples_per_block")
    ):
        raise ValueError("contention protocol, role or sample rule invalid")
    expected = {
        "runner": RUNNER, "auditor": AUDITOR, "block_runner": Path(__file__),
        "s01_helper": S01_HELPER, "s02_helper": S02_HELPER,
        "prior_protocol": PRIOR,
        f"{role}_main_archive": main_archive,
        f"{role}_platform_archive": platform_archive,
        f"{role}_reference_manifest": reference_manifest,
    }
    if any(protocol["frozen_sha256"].get(name) != sha(path) for name, path in expected.items()):
        raise ValueError("contention frozen source or archive changed")


def run_block(
    *, role: str, checkout: Path, install_root: Path, node: Path,
    main_archive: Path, platform_archive: Path, reference_manifest: Path,
    samples: int, output: Path, contention_protocol: Path | None = None,
) -> dict:
    """Keep every fresh process, database, independent audit and host record."""
    if samples < 3:
        raise ValueError("contention block needs at least three measured samples")
    prior = json.loads(PRIOR.read_text())
    source = prior[role]["source_sha"]
    main_hash = prior[role]["main_archive_sha256"]
    platform_hash = prior[role]["platform_archive_sha256"]
    validate_identity(prior, role, source, main_hash, platform_hash)
    s01_block.verify_source(checkout, source)
    archive = s01_block.verify_archives(
        main_archive, platform_archive, install_root, main_hash, platform_hash,
    )
    reference = json.loads(reference_manifest.read_text())
    if reference.get("role") != role or reference.get("source_sha") != source:
        raise ValueError("reference artifact role or source changed")
    if subprocess.run([str(node), "--version"], capture_output=True,
                      text=True, check=True).stdout.strip() != prior["node_version"]:
        raise ValueError("Node version changed")
    frozen_sha = None
    if contention_protocol is not None:
        frozen = json.loads(contention_protocol.read_text())
        validate_frozen(frozen, role=role, samples=samples,
                        main_archive=main_archive, platform_archive=platform_archive,
                        reference_manifest=reference_manifest)
        frozen_sha = sha(contention_protocol)
    elif role != "baseline":
        raise ValueError("candidate requires frozen contention protocol")
    output.mkdir(parents=True, exist_ok=False)
    bundle = output / "bundle"
    bundle.mkdir()
    for script in (RUNNER, S01_HELPER, S02_HELPER):
        shutil.copy2(script, bundle / script.name)
    identity = {
        "role": role, "source_sha": source,
        "prior_protocol_sha256": sha(PRIOR),
        "contention_protocol_sha256": frozen_sha,
        "main_archive_sha256": main_hash,
        "platform_archive_sha256": platform_hash,
        "installed_module_sha256": archive["module_sha256"],
        "installed_native_sha256": archive["native_sha256"],
        "runner_sha256": sha(bundle / RUNNER.name),
        "s01_helper_sha256": sha(bundle / S01_HELPER.name),
        "s02_helper_sha256": sha(bundle / S02_HELPER.name),
        "auditor_sha256": sha(AUDITOR),
        "block_runner_sha256": sha(Path(__file__)),
        "reference_manifest_sha256": sha(reference_manifest),
        "node_version": prior["node_version"],
    }
    (output / "identity.json").write_text(json.dumps(identity, indent=2) + "\n")
    environment = {**os.environ, "FATHOMDB_EMBED_DEVICE": "cpu", "HF_HUB_OFFLINE": "1"}
    start = pilot.inventory(output)
    start["node_version"] = prior["node_version"]
    attempts = []
    for number in range(samples + 1):
        label = "warmup" if number == 0 else f"sample-{number:03d}"
        stem = output / label
        raw = Path(str(stem) + ".json")
        database = Path(str(stem) + ".sqlite")
        resource = Path(str(stem) + ".resource")
        stdout = Path(str(stem) + ".stdout")
        stderr = Path(str(stem) + ".stderr")
        checked = Path(str(stem) + ".audit.json")
        command = [
            str(pilot.GNU_TIME), "-o", str(resource), "-f", pilot.RESOURCE_FORMAT,
            str(node), str(bundle / RUNNER.name),
            "--database", str(database), "--install-root", str(install_root),
            "--source-sha", source,
            "--expected-native-sha256", archive["native_sha256"],
            "--output", str(raw),
        ]
        (output / f"{label}.command.json").write_text(json.dumps(command) + "\n")
        try:
            result = subprocess.run(
                command, cwd=ROOT, capture_output=True, text=True,
                env=environment, check=False, timeout=120,
            )
            stdout.write_text(result.stdout)
            stderr.write_text(result.stderr)
            exit_code = result.returncode
        except subprocess.TimeoutExpired as error:
            stdout.write_bytes(error.stdout or b"")
            stderr.write_bytes(error.stderr or b"")
            exit_code = 124
        child = pilot.read_resource_report(resource)
        attempt: dict = {
            "label": label, "exit_code": exit_code, "resource": child,
        }
        try:
            if exit_code:
                raise ValueError(f"process exited {exit_code}")
            if stdout.read_text().splitlines() != ["S02_TS_CONTENTION_FUNCTIONAL_OK"] or stderr.read_text():
                raise ValueError("process output differs from expected status")
            if not pilot.complete_child_resources(child) or child["swap_events"]:
                raise ValueError("child resource report incomplete or swapped")
            accepted = audit.audit_run(
                raw_path=raw, database=database, runner=bundle / RUNNER.name,
                s01_helper=bundle / S01_HELPER.name,
                s02_helper=bundle / S02_HELPER.name,
                comparison_protocol=PRIOR, reference_manifest=reference_manifest,
                install_root=install_root, main_archive=main_archive,
                platform_archive=platform_archive,
            )
            checked.write_text(json.dumps(accepted, indent=2) + "\n")
            attempt.update({
                "valid": True, "elapsed_ns": accepted["elapsed_ns"],
                "raw_sha256": sha(raw), "database_sha256": sha(database),
                "resource_sha256": sha(resource), "audit_sha256": sha(checked),
            })
        except (OSError, ValueError, KeyError, IndexError, TypeError, json.JSONDecodeError) as error:
            attempt.update({"valid": False, "reason": str(error)})
        attempts.append(attempt)
        if not attempt["valid"]:
            break
    end = pilot.inventory(output)
    end["node_version"] = prior["node_version"]
    invalidators = sorted(set(
        reason for attempt in attempts for reason in pilot.environment_invalidators(
            start, end, MIN_DISK_FREE_BYTES, attempt["resource"],
        )
    ))
    warnings = sorted(set(
        reason for attempt in attempts for reason in pilot.environment_warnings(
            start, end, attempt["resource"],
        )
    ))
    (output / "environment.json").write_text(json.dumps({
        "start": start, "end": end,
        "invalidators": invalidators, "warnings": warnings,
    }, indent=2) + "\n")
    (output / "attempts.json").write_text(json.dumps(attempts, indent=2) + "\n")
    values = [attempt["elapsed_ns"] for attempt in attempts[1:] if attempt["valid"]]
    valid = len(attempts) == samples + 1 and all(attempt["valid"] for attempt in attempts)
    valid = valid and not invalidators
    summary = {
        "status": "VALID_S02_TS_CONTENTION_BLOCK" if valid else "INVALID_S02_TS_CONTENTION_BLOCK",
        "role": role, "samples": samples, "warmups": 1,
        "valid_samples": len(values),
        "p50_ns": nearest_rank(values, .5) if values else None,
        "p95_ns": nearest_rank(values, .95) if len(values) >= 20 else None,
        "min_ns": min(values) if values else None,
        "max_ns": max(values) if values else None,
        "invalidators": invalidators, "warnings": warnings,
        "identity_sha256": sha(output / "identity.json"),
        "environment_sha256": sha(output / "environment.json"),
        "attempts_sha256": sha(output / "attempts.json"),
    }
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in (
        "role", "checkout", "install-root", "node", "main-archive",
        "platform-archive", "reference-manifest", "samples", "output",
    ):
        parser.add_argument("--" + name, required=True,
                            type=int if name == "samples" else Path if name != "role" else str)
    parser.add_argument("--contention-protocol", type=Path)
    args = parser.parse_args()
    result = run_block(
        role=args.role, checkout=args.checkout.resolve(),
        install_root=args.install_root.resolve(), node=args.node.resolve(),
        main_archive=args.main_archive.resolve(),
        platform_archive=args.platform_archive.resolve(),
        reference_manifest=args.reference_manifest.resolve(),
        samples=args.samples, output=args.output.absolute(),
        contention_protocol=args.contention_protocol.resolve() if args.contention_protocol else None,
    )
    print(result["status"])
    if result["status"] != "VALID_S02_TS_CONTENTION_BLOCK":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
