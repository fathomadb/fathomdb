#!/usr/bin/env python3
"""Assemble and validate the non-publishing Slice 50 candidate manifest."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path


SCHEMA = "fathomdb.slice50-candidate/v1"
ARTIFACT_KINDS = {"wheel", "npm", "cli"}
EVIDENCE_KINDS = {"graph-evidence", "gitleaks-generated", "gitleaks-tracked"}
COMMANDS = {
    "agent-verify",
    "installed-wheel-profile",
    "installed-npm-profile",
    "cli-integrity",
    "gitleaks-generated",
    "gitleaks-tracked",
}


def _candidate(value: object) -> str:
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("candidate SHA must be lowercase 40-hex")
    return value


def _coordinate(value: str) -> tuple[str, Path]:
    kind, separator, raw_path = value.partition("=")
    if not separator or not kind or not raw_path:
        raise ValueError(f"invalid kind=path coordinate: {value}")
    return kind, Path(raw_path)


def _command(value: str) -> tuple[str, str]:
    name, separator, outcome = value.partition("=")
    if not separator or not name or not outcome:
        raise ValueError(f"invalid command=outcome coordinate: {value}")
    return name, outcome


def _file_record(kind: str, path: Path) -> dict[str, object]:
    if not path.is_file():
        raise ValueError(f"missing {kind} file: {path}")
    return {
        "kind": kind,
        "name": path.name,
        "size": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }


def _payload_digest(payload: object) -> str:
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def _validate_file_records(value: object, expected_kinds: set[str], field: str) -> None:
    if not isinstance(value, list) or len(value) != len(expected_kinds):
        raise ValueError(f"invalid {field} count")
    kinds = set()
    for record in value:
        if not isinstance(record, dict) or set(record) != {"kind", "name", "size", "sha256"}:
            raise ValueError(f"invalid {field} record")
        kinds.add(record["kind"])
        if not isinstance(record["name"], str) or not record["name"]:
            raise ValueError(f"invalid {field} name")
        if not isinstance(record["size"], int) or record["size"] <= 0:
            raise ValueError(f"invalid {field} size")
        if not isinstance(record["sha256"], str) or not re.fullmatch(
            r"[0-9a-f]{64}", record["sha256"]
        ):
            raise ValueError(f"invalid {field} digest")
    if kinds != expected_kinds:
        raise ValueError(f"invalid {field} kinds")


def validate_manifest(payload: object) -> dict[str, object]:
    """Strictly validate a durable Slice 50 candidate manifest."""

    if not isinstance(payload, dict) or set(payload) != {
        "schema_version",
        "candidate_sha",
        "axes",
        "toolchains",
        "artifacts",
        "evidence",
        "commands",
        "external",
        "profiles",
    }:
        raise ValueError("invalid candidate manifest keys")
    if payload["schema_version"] != SCHEMA:
        raise ValueError("invalid candidate manifest schema")
    candidate = _candidate(payload["candidate_sha"])
    axes = payload["axes"]
    if not isinstance(axes, dict) or set(axes) != {"workspace", "embedder_api"}:
        raise ValueError("invalid release axes")
    if any(not isinstance(value, str) or not value for value in axes.values()):
        raise ValueError("invalid release axis value")
    toolchains = payload["toolchains"]
    if not isinstance(toolchains, dict) or set(toolchains) != {"rust", "python", "node"}:
        raise ValueError("invalid candidate toolchains")
    if any(not isinstance(value, str) or not value for value in toolchains.values()):
        raise ValueError("invalid candidate toolchain value")
    _validate_file_records(payload["artifacts"], ARTIFACT_KINDS, "artifact")
    _validate_file_records(payload["evidence"], EVIDENCE_KINDS, "evidence")
    commands = payload["commands"]
    if not isinstance(commands, dict) or set(commands) != COMMANDS:
        raise ValueError("invalid command set")
    if any(outcome != "pass" for outcome in commands.values()):
        raise ValueError("invalid command outcome")
    external = payload["external"]
    if not isinstance(external, dict) or set(external) != {"native_matrix", "windows_wal"}:
        raise ValueError("invalid external evidence set")
    for name, record in external.items():
        if not isinstance(record, dict) or record.get("state") not in {"passed", "unresolved"}:
            raise ValueError(f"invalid external state: {name}")
        if record["state"] == "passed":
            if set(record) != {
                "state",
                "receipt_name",
                "size",
                "sha256",
                "payload_sha256",
                "receipt",
            }:
                raise ValueError(f"invalid passed external receipt: {name}")
            if not isinstance(record["receipt_name"], str) or not record["receipt_name"]:
                raise ValueError(f"invalid external receipt name: {name}")
            if not isinstance(record["size"], int) or record["size"] <= 0:
                raise ValueError(f"invalid external receipt size: {name}")
            if not isinstance(record["sha256"], str) or not re.fullmatch(
                r"[0-9a-f]{64}", record["sha256"]
            ):
                raise ValueError(f"invalid external receipt digest: {name}")
            if record["payload_sha256"] != _payload_digest(record["receipt"]):
                raise ValueError(f"external receipt payload digest mismatch: {name}")
            validators = _receipt_validators()
            if name == "native_matrix":
                validators.validate_matrix(record["receipt"], candidate)
            else:
                validators.validate_wal_receipt(record["receipt"])
                if record["receipt"]["candidate_sha"] != candidate:
                    raise ValueError("Windows WAL receipt candidate drift")
        elif set(record) != {"state", "reason"} or not record.get("reason"):
            raise ValueError(f"invalid unresolved external receipt: {name}")
    profiles = payload["profiles"]
    if not isinstance(profiles, dict) or set(profiles) != {"graph_evidence"}:
        raise ValueError("invalid candidate profiles")
    graph_profile = profiles["graph_evidence"]
    if not isinstance(graph_profile, dict) or set(graph_profile) != {
        "payload",
        "payload_sha256",
        "source_sha256",
        "source_size",
    }:
        raise ValueError("invalid graph evidence profile binding")
    if graph_profile["payload_sha256"] != _payload_digest(graph_profile["payload"]):
        raise ValueError("graph evidence payload digest mismatch")
    graph_record = next(
        record for record in payload["evidence"] if record["kind"] == "graph-evidence"
    )
    if (
        graph_profile["source_sha256"] != graph_record["sha256"]
        or graph_profile["source_size"] != graph_record["size"]
    ):
        raise ValueError("graph evidence source binding mismatch")
    _validate_graph_evidence(graph_profile["payload"])
    return payload


def _receipt_validators():
    path = Path(__file__).with_name("native-artifact-receipts.py")
    spec = importlib.util.spec_from_file_location("slice50_native_artifact_receipts", path)
    if spec is None or spec.loader is None:
        raise ValueError("cannot load native artifact receipt validators")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _graph_validator():
    path = Path(__file__).with_name("slice50-evidence-matrix.py")
    spec = importlib.util.spec_from_file_location("slice50_evidence_matrix", path)
    if spec is None or spec.loader is None:
        raise ValueError("cannot load graph evidence validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _validate_graph_evidence(payload: object) -> dict[str, object]:
    if not isinstance(payload, dict) or set(payload) != {
        "schema_version",
        "rows",
        "schema_33_refusal",
    }:
        raise ValueError("invalid graph evidence keys")
    if payload["schema_version"] != "fathomdb.slice50-graph-evidence/v1":
        raise ValueError("invalid graph evidence schema")
    _graph_validator().validate_rows(payload["rows"])
    refusal = payload["schema_33_refusal"]
    if not isinstance(refusal, dict) or set(refusal) != {
        "database",
        "expected_schema",
        "supported_schema",
        "outcome",
        "before",
        "after",
    }:
        raise ValueError("invalid schema-33 refusal keys")
    if (
        refusal["expected_schema"] != 33
        or refusal["supported_schema"] != 34
        or refusal["outcome"] != "typed_refusal"
        or refusal["before"] != refusal["after"]
    ):
        raise ValueError("invalid schema-33 refusal evidence")
    snapshot = refusal["before"]
    if not isinstance(snapshot, list) or not snapshot:
        raise ValueError("empty schema-33 refusal snapshot")
    for record in snapshot:
        if not isinstance(record, dict) or set(record) != {"name", "size", "sha256"}:
            raise ValueError("invalid schema-33 snapshot record")
        if not isinstance(record["size"], int) or record["size"] <= 0:
            raise ValueError("invalid schema-33 snapshot size")
        if not isinstance(record["sha256"], str) or not re.fullmatch(
            r"[0-9a-f]{64}", record["sha256"]
        ):
            raise ValueError("invalid schema-33 snapshot digest")
    return payload


def _external_receipt(path: Path, schema: str, candidate: str) -> dict[str, object]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    validators = _receipt_validators()
    if schema == "fathomdb.native-artifact-matrix/v1":
        validators.validate_matrix(payload, candidate)
    else:
        validators.validate_wal_receipt(payload)
        if payload["candidate_sha"] != candidate:
            raise ValueError("Windows WAL receipt candidate drift")
    record = _file_record("receipt", path)
    return {
        "state": "passed",
        "receipt_name": record["name"],
        "size": record["size"],
        "sha256": record["sha256"],
        "payload_sha256": _payload_digest(payload),
        "receipt": payload,
    }


def assemble(args: argparse.Namespace) -> dict[str, object]:
    candidate = _candidate(args.candidate_sha)
    head = subprocess.run(
        ["git", "-C", str(args.repo_root), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if head != candidate:
        raise ValueError(f"candidate SHA does not match repo HEAD: {candidate} != {head}")
    cargo = tomllib.loads((args.repo_root / "Cargo.toml").read_text(encoding="utf-8"))
    axes = {
        "workspace": cargo["workspace"]["package"]["version"],
        "embedder_api": cargo["workspace"]["dependencies"]["fathomdb-embedder-api"][
            "version"
        ],
    }
    artifact_coordinates = dict(_coordinate(value) for value in args.artifact)
    evidence_coordinates = dict(_coordinate(value) for value in args.evidence)
    commands = dict(_command(value) for value in args.command)
    unresolved = dict(_command(value) for value in args.external_state)
    graph_payload = json.loads(evidence_coordinates["graph-evidence"].read_text(encoding="utf-8"))
    _validate_graph_evidence(graph_payload)
    evidence_records = [
        _file_record(kind, evidence_coordinates[kind]) for kind in sorted(EVIDENCE_KINDS)
    ]
    graph_record = next(record for record in evidence_records if record["kind"] == "graph-evidence")
    payload = {
        "schema_version": SCHEMA,
        "candidate_sha": candidate,
        "axes": axes,
        "toolchains": {"rust": args.rust, "python": args.python, "node": args.node},
        "artifacts": [
            _file_record(kind, artifact_coordinates[kind]) for kind in sorted(ARTIFACT_KINDS)
        ],
        "evidence": evidence_records,
        "commands": commands,
        "external": {},
        "profiles": {
            "graph_evidence": {
                "payload": graph_payload,
                "payload_sha256": _payload_digest(graph_payload),
                "source_sha256": graph_record["sha256"],
                "source_size": graph_record["size"],
            }
        },
    }
    external_inputs = {
        "native_matrix": (args.native_matrix, "fathomdb.native-artifact-matrix/v1"),
        "windows_wal": (args.windows_wal_receipt, "fathomdb.windows-wal-receipt/v1"),
    }
    if set(unresolved) - external_inputs.keys():
        raise ValueError("unknown unresolved external evidence")
    for name, (path, schema) in external_inputs.items():
        if path is not None and name in unresolved:
            raise ValueError(f"external evidence cannot be both passed and unresolved: {name}")
        if path is not None:
            payload["external"][name] = _external_receipt(path, schema, candidate)
        elif name in unresolved and unresolved[name]:
            payload["external"][name] = {"state": "unresolved", "reason": unresolved[name]}
        else:
            raise ValueError(f"missing external evidence state: {name}")
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
        tomllib.TOMLDecodeError,
    ) as error:
        print(f"candidate_manifest_failed reason={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
