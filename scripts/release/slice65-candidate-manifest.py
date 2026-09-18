#!/usr/bin/env python3
"""Assemble and validate the Slice 65 candidate qualification wrapper."""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path


SCHEMA = "fathomdb.slice65-candidate/v1"
QUALIFICATIONS = {
    "design_lifecycle",
    "sdk_surface_parity",
    "slice60_owner_probes",
}


def _slice50():
    path = Path(__file__).with_name("slice50-candidate-manifest.py")
    spec = importlib.util.spec_from_file_location("slice50_candidate_manifest", path)
    if spec is None or spec.loader is None:
        raise ValueError("cannot load Slice 50 candidate-manifest validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _candidate(value: object) -> str:
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("candidate SHA must be lowercase 40-hex")
    return value


def _qualification(value: str) -> tuple[str, str]:
    name, separator, outcome = value.partition("=")
    if not separator or not name or not outcome:
        raise ValueError(f"invalid qualification=outcome coordinate: {value}")
    return name, outcome


def validate_manifest(payload: object) -> dict[str, object]:
    """Strictly validate one Slice 65 wrapper and its Slice 50 base payload."""

    if not isinstance(payload, dict) or set(payload) != {
        "schema_version",
        "candidate_sha",
        "base_candidate",
        "qualification",
    }:
        raise ValueError("invalid Slice 65 candidate manifest keys")
    if payload["schema_version"] != SCHEMA:
        raise ValueError("invalid Slice 65 candidate manifest schema")
    candidate = _candidate(payload["candidate_sha"])
    base = _slice50().validate_manifest(payload["base_candidate"])
    if base["candidate_sha"] != candidate:
        raise ValueError("Slice 65 and Slice 50 candidate SHAs differ")
    qualification = payload["qualification"]
    if not isinstance(qualification, dict) or set(qualification) != QUALIFICATIONS:
        raise ValueError("invalid Slice 65 qualification set")
    if any(outcome != "pass" for outcome in qualification.values()):
        raise ValueError("invalid Slice 65 qualification outcome")
    return payload


def assemble(args: argparse.Namespace) -> dict[str, object]:
    """Delegate base assembly and add only Slice 65 qualification outcomes."""

    base = _slice50().assemble(args)
    qualification = dict(_qualification(value) for value in args.qualification)
    payload = {
        "schema_version": SCHEMA,
        "candidate_sha": args.candidate_sha,
        "base_candidate": base,
        "qualification": qualification,
    }
    return validate_manifest(payload)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="action", required=True)
    create = subparsers.add_parser("assemble")
    create.add_argument("--repo-root", type=Path, required=True)
    create.add_argument("--candidate-sha", required=True)
    create.add_argument("--rust", required=True)
    create.add_argument("--python", required=True)
    create.add_argument("--node", required=True)
    create.add_argument("--artifact", action="append", default=[])
    create.add_argument("--evidence", action="append", default=[])
    create.add_argument("--native-matrix", type=Path)
    create.add_argument("--windows-wal-receipt", type=Path)
    create.add_argument("--external-state", action="append", default=[])
    create.add_argument("--command", action="append", default=[])
    create.add_argument("--qualification", action="append", default=[])
    create.add_argument("--output", type=Path, required=True)
    validate = subparsers.add_parser("validate")
    validate.add_argument("--manifest", type=Path, required=True)
    return parser


def main() -> int:
    args = _parser().parse_args()
    try:
        if args.action == "assemble":
            payload = assemble(args)
            args.output.write_text(
                json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
            )
        else:
            validate_manifest(json.loads(args.manifest.read_text(encoding="utf-8")))
    except (
        KeyError,
        OSError,
        ValueError,
        json.JSONDecodeError,
        subprocess.CalledProcessError,
    ) as error:
        print(f"slice65_candidate_manifest_failed reason={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
