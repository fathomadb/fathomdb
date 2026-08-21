#!/usr/bin/env python3
"""Verify that a candidate consumer received the exact hosted-route receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any, NoReturn


COMMIT_SHA = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")
WORKFLOW_REF = "fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main"
UNMERGED_SCHEMA = "fathomdb.cuda-unmerged-route-receipt/v1"
MAIN_SCHEMA = "fathomdb.cuda-main-route-receipt/v2"
MAIN_OWNED_MANIFEST = Path("control-plane/dev/release/cuda-unmerged-candidates.json")


def fail(message: str) -> NoReturn:
    raise SystemExit(f"cuda-unmerged-receipt: {message}")


def canonical_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"


def require_sha(value: object, label: str, pattern: re.Pattern[str]) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        fail(f"{label} must be lowercase hexadecimal")
    return value


def require_positive_int(value: object, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 1:
        fail(f"{label} must be a positive integer")
    return value


def load_main_owned_manifest(path: Path) -> bytes:
    if path != MAIN_OWNED_MANIFEST:
        fail("manifest must use the exact main-owned control-plane path")
    current = Path(".")
    for component in path.parts:
        current /= component
        if current.is_symlink():
            fail("manifest path must not traverse a symlink")
    try:
        raw = path.read_bytes()
        manifest: Any = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read manifest: {error}")
    if not isinstance(manifest, dict) or raw != canonical_json(manifest):
        fail("manifest must be a canonical JSON object")
    return raw


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument("--workflow-ref", required=True)
    parser.add_argument("--workflow-sha", required=True)
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--run-attempt", type=int, required=True)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    raw_manifest = load_main_owned_manifest(args.manifest)
    if args.receipt.is_symlink():
        fail("receipt must not be a symlink")
    try:
        raw = args.receipt.read_bytes()
        receipt: Any = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read receipt: {error}")
    if not isinstance(receipt, dict) or raw != canonical_json(receipt):
        fail("receipt must be a canonical JSON object")
    expected_fields = {
        "schema_version", "workflow_ref", "workflow_sha", "run_id", "run_attempt",
        "manifest_sha256", "candidate_sha", "candidate_pr", "approval_review_ids", "provenance_pr",
        "provenance_approval_review_ids", "api_response_sha256",
    }
    if receipt.get("schema_version") == MAIN_SCHEMA:
        expected_main_fields = {
            "schema_version", "workflow_ref", "workflow_sha", "run_id", "run_attempt", "candidate_sha",
        }
        if set(receipt) != expected_main_fields:
            fail("main-route receipt schema is unsupported")
        if args.workflow_ref != WORKFLOW_REF or receipt["workflow_ref"] != args.workflow_ref:
            fail("receipt workflow ref differs from the exact trusted route")
        if require_sha(args.candidate_sha, "requested candidate SHA", COMMIT_SHA) != receipt["candidate_sha"]:
            fail("receipt candidate SHA does not match this consumer")
        if require_sha(args.workflow_sha, "requested workflow SHA", COMMIT_SHA) != receipt["workflow_sha"]:
            fail("receipt workflow SHA does not match this consumer")
        if require_positive_int(args.run_id, "requested run ID") != receipt["run_id"]:
            fail("receipt run ID does not match this consumer")
        if require_positive_int(args.run_attempt, "requested run attempt") != receipt["run_attempt"]:
            fail("receipt run attempt does not match this consumer")
        print("cuda-unmerged-receipt: pass")
        return
    if set(receipt) != expected_fields or receipt["schema_version"] != UNMERGED_SCHEMA:
        fail("receipt schema is unsupported")
    if args.workflow_ref != WORKFLOW_REF or receipt["workflow_ref"] != args.workflow_ref:
        fail("receipt workflow ref differs from the exact trusted route")
    if require_sha(args.candidate_sha, "requested candidate SHA", COMMIT_SHA) != receipt["candidate_sha"]:
        fail("receipt candidate SHA does not match this consumer")
    if require_sha(args.workflow_sha, "requested workflow SHA", COMMIT_SHA) != receipt["workflow_sha"]:
        fail("receipt workflow SHA does not match this consumer")
    if require_positive_int(args.run_id, "requested run ID") != receipt["run_id"]:
        fail("receipt run ID does not match this consumer")
    if require_positive_int(args.run_attempt, "requested run attempt") != receipt["run_attempt"]:
        fail("receipt run attempt does not match this consumer")
    require_sha(receipt["workflow_sha"], "receipt workflow SHA", COMMIT_SHA)
    if require_sha(receipt["manifest_sha256"], "receipt manifest SHA", SHA256) != hashlib.sha256(raw_manifest).hexdigest():
        fail("receipt manifest SHA does not match the main-owned manifest")
    require_sha(receipt["candidate_sha"], "receipt candidate SHA", COMMIT_SHA)
    require_positive_int(receipt["candidate_pr"], "receipt candidate PR")
    approval_ids = receipt["approval_review_ids"]
    if not isinstance(approval_ids, list) or len(set(approval_ids)) != len(approval_ids):
        fail("receipt approval review IDs are invalid")
    for review_id in approval_ids:
        require_positive_int(review_id, "receipt approval review ID")
    require_positive_int(receipt["provenance_pr"], "receipt provenance PR")
    provenance_approval_ids = receipt["provenance_approval_review_ids"]
    if (
        not isinstance(provenance_approval_ids, list)
        or len(set(provenance_approval_ids)) != len(provenance_approval_ids)
    ):
        fail("receipt provenance approval review IDs are invalid")
    for review_id in provenance_approval_ids:
        require_positive_int(review_id, "receipt provenance approval review ID")
    response_digests = receipt["api_response_sha256"]
    if not isinstance(response_digests, dict) or set(response_digests) != {
        "protection_baseline", "pull_request", "reviews", "provenance_pull_request", "provenance_reviews"
    }:
        fail("receipt API response digest inventory is invalid")
    for name, value in response_digests.items():
        require_sha(value, f"receipt API response digest {name}", SHA256)
    print("cuda-unmerged-receipt: pass")


if __name__ == "__main__":
    main()
