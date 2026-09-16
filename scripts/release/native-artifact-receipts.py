#!/usr/bin/env python3
"""Write and collect exact-SHA native artifact validation receipts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


RECEIPT_SCHEMA = "fathomdb.native-artifact-receipt/v1"
MATRIX_SCHEMA = "fathomdb.native-artifact-matrix/v1"
WAL_SCHEMA = "fathomdb.windows-wal-receipt/v1"
EXPECTED_TARGET_LABELS = {
    "x86_64-unknown-linux-gnu": "linux-x64-gnu",
    "aarch64-unknown-linux-gnu": "linux-arm64-gnu",
    "x86_64-apple-darwin": "darwin-x64",
    "aarch64-apple-darwin": "darwin-arm64",
    "x86_64-pc-windows-msvc": "win32-x64-msvc",
}


def _candidate(value: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("candidate SHA must be lowercase 40-hex")
    return value


def _artifact(kind: str, path: Path) -> dict[str, object]:
    if not path.is_file():
        raise ValueError(f"missing {kind} artifact: {path}")
    return {
        "kind": kind,
        "name": path.name,
        "size": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }


def validate_receipt(payload: object) -> dict[str, object]:
    if not isinstance(payload, dict):
        raise ValueError("invalid receipt object")
    expected_keys = {
        "schema_version",
        "candidate_sha",
        "target",
        "label",
        "runner",
        "toolchains",
        "command",
        "outcome",
        "artifacts",
    }
    if set(payload) != expected_keys:
        raise ValueError("invalid receipt keys")
    if payload["schema_version"] != RECEIPT_SCHEMA:
        raise ValueError("invalid receipt schema")
    _candidate(str(payload["candidate_sha"]))
    target = payload["target"]
    label = payload["label"]
    if not isinstance(target, str) or target not in EXPECTED_TARGET_LABELS:
        raise ValueError(f"unexpected target: {target}")
    if label != EXPECTED_TARGET_LABELS[target]:
        raise ValueError(f"target-label mismatch: {target} {label}")
    for key in ("runner", "command"):
        if not isinstance(payload[key], str) or not payload[key]:
            raise ValueError(f"invalid receipt {key}")
    toolchains = payload["toolchains"]
    if not isinstance(toolchains, dict) or set(toolchains) != {"rust", "python", "node"}:
        raise ValueError("invalid receipt toolchains")
    if any(not isinstance(value, str) or not value for value in toolchains.values()):
        raise ValueError("invalid receipt toolchain value")
    if payload["outcome"] != "pass":
        raise ValueError(f"non-passing target: {target}")
    artifacts = payload["artifacts"]
    if not isinstance(artifacts, list) or len(artifacts) != 2:
        raise ValueError("invalid receipt artifacts")
    kinds = set()
    for artifact in artifacts:
        if not isinstance(artifact, dict) or set(artifact) != {"kind", "name", "size", "sha256"}:
            raise ValueError("invalid artifact keys")
        kind = artifact["kind"]
        kinds.add(kind)
        if not isinstance(artifact["name"], str) or not artifact["name"]:
            raise ValueError("invalid artifact name")
        if not isinstance(artifact["size"], int) or artifact["size"] <= 0:
            raise ValueError("invalid artifact size")
        if not isinstance(artifact["sha256"], str) or not re.fullmatch(
            r"[0-9a-f]{64}", artifact["sha256"]
        ):
            raise ValueError("invalid artifact digest")
    if kinds != {"wheel", "napi"}:
        raise ValueError("invalid artifact kinds")
    return payload


def write_receipt(args: argparse.Namespace) -> None:
    payload = {
        "schema_version": RECEIPT_SCHEMA,
        "candidate_sha": _candidate(args.candidate_sha),
        "target": args.target,
        "label": args.label,
        "runner": args.runner,
        "toolchains": {"rust": args.rust, "python": args.python, "node": args.node},
        "command": args.validation_command,
        "outcome": "pass",
        "artifacts": [_artifact("wheel", args.wheel), _artifact("napi", args.napi)],
    }
    validate_receipt(payload)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def collect_receipts(args: argparse.Namespace) -> None:
    candidate = _candidate(args.candidate_sha)
    by_target: dict[str, dict[str, object]] = {}
    for path in sorted(args.receipt_dir.glob("*.json")):
        payload = json.loads(path.read_text(encoding="utf-8"))
        payload = validate_receipt(payload)
        if payload.get("candidate_sha") != candidate:
            raise ValueError(f"candidate drift in receipt: {path.name}")
        target = payload.get("target")
        if target in by_target:
            raise ValueError(f"duplicate target: {target}")
        by_target[target] = payload
    missing = sorted(EXPECTED_TARGET_LABELS.keys() - by_target.keys())
    if missing:
        raise ValueError(f"missing target: {','.join(missing)}")
    payload = {
        "schema_version": MATRIX_SCHEMA,
        "candidate_sha": candidate,
        "platforms": [by_target[target] for target in sorted(by_target)],
    }
    validate_matrix(payload, candidate)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def validate_wal_receipt(payload: object) -> dict[str, object]:
    """Validate the distinct exact-candidate Windows WAL receipt."""

    if not isinstance(payload, dict) or set(payload) != {
        "schema_version",
        "candidate_sha",
        "platform",
        "command",
        "outcome",
        "artifacts",
    }:
        raise ValueError("invalid Windows WAL receipt keys")
    if payload["schema_version"] != WAL_SCHEMA:
        raise ValueError("invalid Windows WAL receipt schema")
    _candidate(payload["candidate_sha"])
    if payload["platform"] != "windows-x64":
        raise ValueError("invalid Windows WAL platform")
    if not isinstance(payload["command"], str) or not payload["command"]:
        raise ValueError("invalid Windows WAL command")
    if payload["outcome"] != "pass":
        raise ValueError("non-passing Windows WAL receipt")
    artifacts = payload["artifacts"]
    if not isinstance(artifacts, list) or len(artifacts) != 2:
        raise ValueError("invalid Windows WAL artifacts")
    kinds = set()
    for artifact in artifacts:
        if not isinstance(artifact, dict) or set(artifact) != {"kind", "name", "size", "sha256"}:
            raise ValueError("invalid Windows WAL artifact keys")
        kinds.add(artifact["kind"])
        if not isinstance(artifact["name"], str) or not artifact["name"]:
            raise ValueError("invalid Windows WAL artifact name")
        if not isinstance(artifact["size"], int) or artifact["size"] <= 0:
            raise ValueError("invalid Windows WAL artifact size")
        if not isinstance(artifact["sha256"], str) or not re.fullmatch(
            r"[0-9a-f]{64}", artifact["sha256"]
        ):
            raise ValueError("invalid Windows WAL artifact digest")
    if kinds != {"test-hooks-wheel", "hook-contract"}:
        raise ValueError("invalid Windows WAL artifact kinds")
    return payload


def validate_matrix(payload: object, candidate: str | None = None) -> dict[str, object]:
    """Validate a complete five-target native receipt matrix."""

    if not isinstance(payload, dict) or set(payload) != {
        "schema_version",
        "candidate_sha",
        "platforms",
    }:
        raise ValueError("invalid native matrix keys")
    if payload["schema_version"] != MATRIX_SCHEMA:
        raise ValueError("invalid native matrix schema")
    matrix_candidate = _candidate(payload["candidate_sha"])
    if candidate is not None and matrix_candidate != candidate:
        raise ValueError("native matrix candidate drift")
    platforms = payload["platforms"]
    if not isinstance(platforms, list):
        raise ValueError("invalid native matrix platforms")
    rows = [validate_receipt(row) for row in platforms]
    targets = [row["target"] for row in rows]
    if len(targets) != len(set(targets)):
        raise ValueError("duplicate native matrix target")
    missing = EXPECTED_TARGET_LABELS.keys() - set(targets)
    unexpected = set(targets) - EXPECTED_TARGET_LABELS.keys()
    if missing or unexpected:
        raise ValueError(f"native matrix target mismatch missing={sorted(missing)} unexpected={sorted(unexpected)}")
    if any(row["candidate_sha"] != matrix_candidate for row in rows):
        raise ValueError("native matrix row candidate drift")
    return payload


def write_wal_receipt(args: argparse.Namespace) -> None:
    payload = {
        "schema_version": WAL_SCHEMA,
        "candidate_sha": _candidate(args.candidate_sha),
        "platform": "windows-x64",
        "command": args.validation_command,
        "outcome": "pass",
        "artifacts": [
            _artifact("test-hooks-wheel", args.wheel),
            _artifact("hook-contract", args.hook_contract),
        ],
    }
    validate_wal_receipt(payload)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="action", required=True)
    write = subparsers.add_parser("write")
    for name in ("candidate-sha", "target", "label", "runner", "rust", "python", "node"):
        write.add_argument(f"--{name}", required=True)
    write.add_argument("--command", dest="validation_command", required=True)
    write.add_argument("--wheel", type=Path, required=True)
    write.add_argument("--napi", type=Path, required=True)
    write.add_argument("--output", type=Path, required=True)
    write_wal = subparsers.add_parser("write-wal")
    write_wal.add_argument("--candidate-sha", required=True)
    write_wal.add_argument("--wheel", type=Path, required=True)
    write_wal.add_argument("--hook-contract", type=Path, required=True)
    write_wal.add_argument("--command", dest="validation_command", required=True)
    write_wal.add_argument("--output", type=Path, required=True)
    collect = subparsers.add_parser("collect")
    collect.add_argument("--candidate-sha", required=True)
    collect.add_argument("--receipt-dir", type=Path, required=True)
    collect.add_argument("--output", type=Path, required=True)
    return parser


def main() -> int:
    args = _parser().parse_args()
    try:
        if args.action == "write":
            write_receipt(args)
        elif args.action == "write-wal":
            write_wal_receipt(args)
        else:
            collect_receipts(args)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"native_artifact_receipt_failed reason={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
