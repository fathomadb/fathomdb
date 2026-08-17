#!/usr/bin/env python3
"""Fail-closed verifier for a main-owned unmerged CUDA candidate route.

The workflow creates the facts input from authenticated, read-only GitHub API
responses.  This program deliberately does not make network calls: it can be
tested with fixtures and it cannot silently replace a reviewed response with a
different query.  It accepts only canonical JSON and emits a canonical receipt
that downstream candidate consumers bind to their exact run.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import tempfile
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, NoReturn


COMMIT_SHA = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")
TIMESTAMP = re.compile(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z")
WORKFLOW_REF = "fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main"
MANIFEST_SCHEMA = "fathomdb.cuda-unmerged-candidates/v1"
CANDIDATE_SCHEMA = "fathomdb.cuda-unmerged-candidate/v1"
FACTS_SCHEMA = "fathomdb.cuda-unmerged-candidate-facts/v1"
BASELINE_SCHEMA = "fathomdb.cuda-protection-baseline/v1"
RECEIPT_SCHEMA = "fathomdb.cuda-unmerged-route-receipt/v1"
PURPOSE = "0.8.23 non-publishing CUDA preflight"


def fail(message: str) -> NoReturn:
    raise SystemExit(f"cuda-unmerged-candidate: {message}")


def canonical_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"


def load_canonical_object(path: Path, label: str) -> tuple[dict[str, Any], bytes]:
    if path.is_symlink():
        fail(f"{label} must not be a symlink")
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {label}: {error}")
    if not isinstance(value, dict):
        fail(f"{label} must be a JSON object")
    if raw != canonical_json(value):
        fail(f"{label} is not canonical JSON")
    return value, raw


def require_exact_keys(value: dict[str, Any], fields: set[str], label: str) -> None:
    if set(value) != fields:
        fail(f"{label} has missing or unknown fields")


def require_sha(value: object, label: str, pattern: re.Pattern[str] = COMMIT_SHA) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        fail(f"{label} must be lowercase hexadecimal")
    return value


def require_positive_int(value: object, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 1:
        fail(f"{label} must be a positive integer")
    return value


def parse_timestamp(value: object, label: str) -> datetime:
    if not isinstance(value, str) or TIMESTAMP.fullmatch(value) is None:
        fail(f"{label} must be an exact UTC timestamp")
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=UTC)
    except ValueError as error:
        fail(f"{label} is not a real timestamp: {error}")


def digest(value: object) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def validate_manifest(manifest: dict[str, Any], candidate_sha: str, now: datetime) -> dict[str, Any]:
    require_exact_keys(manifest, {"schema_version", "candidates"}, "manifest")
    if manifest["schema_version"] != MANIFEST_SCHEMA or not isinstance(manifest["candidates"], list):
        fail("manifest schema is unsupported")
    matches: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, record in enumerate(manifest["candidates"]):
        if not isinstance(record, dict):
            fail(f"manifest candidate {index} is not an object")
        require_exact_keys(
            record,
            {
                "schema_version", "candidate_sha", "candidate_pr", "candidate_pr_head_sha",
                "required_reviewers", "expires_at", "purpose", "provenance_pr", "provenance_commit",
            },
            f"manifest candidate {index}",
        )
        if record["schema_version"] != CANDIDATE_SCHEMA:
            fail(f"manifest candidate {index} schema is unsupported")
        record_sha = require_sha(record["candidate_sha"], f"manifest candidate {index} SHA")
        if record_sha in seen:
            fail("manifest has duplicate candidate SHA records")
        seen.add(record_sha)
        if require_sha(record["candidate_pr_head_sha"], f"manifest candidate {index} head SHA") != record_sha:
            fail("manifest candidate head SHA must equal candidate SHA")
        require_positive_int(record["candidate_pr"], f"manifest candidate {index} PR")
        require_positive_int(record["provenance_pr"], f"manifest candidate {index} provenance PR")
        require_sha(record["provenance_commit"], f"manifest candidate {index} provenance commit")
        reviewers = record["required_reviewers"]
        if not isinstance(reviewers, list) or not reviewers or any(not isinstance(item, str) or not item for item in reviewers):
            fail("manifest required reviewers must be a nonempty list of logins")
        if len(set(reviewers)) != len(reviewers):
            fail("manifest required reviewers must be unique")
        if record["purpose"] != PURPOSE:
            fail("manifest candidate purpose is not authorized")
        if parse_timestamp(record["expires_at"], "manifest candidate expiry") <= now:
            fail("manifest candidate authorization has expired")
        if record_sha == candidate_sha:
            matches.append(record)
    if len(matches) != 1:
        fail("manifest does not authorize exactly one requested candidate SHA")
    return matches[0]


def validate_facts(facts: dict[str, Any], record: dict[str, Any], candidate_sha: str, now: datetime) -> list[int]:
    require_exact_keys(
        facts,
        {"schema_version", "protection_baseline", "pull_request", "reviews", "reviews_pagination_complete", "api_response_sha256"},
        "facts",
    )
    if facts["schema_version"] != FACTS_SCHEMA:
        fail("facts schema is unsupported")
    baseline = facts["protection_baseline"]
    if not isinstance(baseline, dict):
        fail("facts protection baseline is not an object")
    require_exact_keys(
        baseline,
        {"schema_version", "expires_at", "main_protection_sha256", "runner_group_sha256", "environment_sha256"},
        "facts protection baseline",
    )
    if baseline["schema_version"] != BASELINE_SCHEMA:
        fail("facts protection baseline schema is unsupported")
    if parse_timestamp(baseline["expires_at"], "facts protection baseline expiry") <= now:
        fail("facts protection baseline has expired")
    for name in ("main_protection_sha256", "runner_group_sha256", "environment_sha256"):
        require_sha(baseline[name], f"facts protection baseline {name}", SHA256)

    pr = facts["pull_request"]
    if not isinstance(pr, dict):
        fail("facts pull request is not an object")
    require_exact_keys(pr, {"number", "state", "draft", "base", "head", "user"}, "facts pull request")
    if require_positive_int(pr["number"], "facts pull request number") != record["candidate_pr"]:
        fail("facts pull request number does not match the authorization")
    if pr["state"] != "open" or pr["draft"] is not False:
        fail("facts pull request must be open and non-draft")
    for side in ("base", "head"):
        value = pr[side]
        if not isinstance(value, dict):
            fail(f"facts pull request {side} is not an object")
        require_exact_keys(value, ({"ref", "repo_full_name"} if side == "base" else {"sha", "repo_full_name"}), f"facts pull request {side}")
        if value["repo_full_name"] != "fathomadb/fathomdb":
            fail(f"facts pull request {side} repository is not fathomadb/fathomdb")
    if pr["base"]["ref"] != "main" or pr["head"]["sha"] != candidate_sha:
        fail("facts pull request target or head no longer matches the authorization")
    author = pr["user"]
    if not isinstance(author, dict):
        fail("facts pull request author is not an object")
    require_exact_keys(author, {"login"}, "facts pull request author")
    if not isinstance(author["login"], str) or not author["login"]:
        fail("facts pull request author login is invalid")

    reviews = facts["reviews"]
    if not isinstance(reviews, list) or facts["reviews_pagination_complete"] is not True:
        fail("facts reviews must be complete")
    latest: dict[str, dict[str, Any]] = {}
    for index, review in enumerate(reviews):
        if not isinstance(review, dict):
            fail(f"facts review {index} is not an object")
        require_exact_keys(review, {"id", "user", "state", "commit_id", "submitted_at"}, f"facts review {index}")
        review_id = require_positive_int(review["id"], f"facts review {index} ID")
        user = review["user"]
        if not isinstance(user, dict):
            fail(f"facts review {index} user is not an object")
        require_exact_keys(user, {"login"}, f"facts review {index} user")
        login = user["login"]
        if not isinstance(login, str) or not login:
            fail(f"facts review {index} reviewer login is invalid")
        require_sha(review["commit_id"], f"facts review {index} commit")
        parse_timestamp(review["submitted_at"], f"facts review {index} time")
        previous = latest.get(login)
        if previous is None or review["submitted_at"] > previous["submitted_at"]:
            latest[login] = review
        _ = review_id
    approved_ids: list[int] = []
    for reviewer in record["required_reviewers"]:
        review = latest.get(reviewer)
        if reviewer == author["login"] or review is None:
            fail("facts lack an independent required reviewer approval")
        if review["state"] != "APPROVED" or review["commit_id"] != candidate_sha:
            fail("facts required reviewer lacks an approval for the exact candidate")
        approved_ids.append(review["id"])
    if len(set(approved_ids)) != len(approved_ids):
        fail("facts required approvals must have distinct review IDs")

    response_digests = facts["api_response_sha256"]
    if not isinstance(response_digests, dict):
        fail("facts API response digests are not an object")
    require_exact_keys(response_digests, {"protection_baseline", "pull_request", "reviews"}, "facts API response digests")
    for name, value in (("protection_baseline", baseline), ("pull_request", pr), ("reviews", reviews)):
        if require_sha(response_digests[name], f"facts API response digest {name}", SHA256) != digest(value):
            fail(f"facts API response digest does not match {name}")
    return approved_ids


def write_receipt(path: Path, receipt: dict[str, Any]) -> None:
    if path.is_symlink():
        fail("receipt output must not be a symlink")
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(canonical_json(receipt))
        os.replace(temporary_name, path)
    except OSError as error:
        fail(f"cannot write receipt: {error}")


def authorize(args: argparse.Namespace) -> None:
    candidate_sha = require_sha(args.candidate_sha, "requested candidate SHA")
    workflow_sha = require_sha(args.workflow_sha, "workflow SHA")
    if args.workflow_ref != WORKFLOW_REF:
        fail("workflow ref is not the reviewed main release workflow")
    run_id = require_positive_int(args.run_id, "run ID")
    run_attempt = require_positive_int(args.run_attempt, "run attempt")
    now = parse_timestamp(args.now, "current time")
    manifest, raw_manifest = load_canonical_object(args.manifest, "manifest")
    facts, _ = load_canonical_object(args.facts, "facts")
    record = validate_manifest(manifest, candidate_sha, now)
    review_ids = validate_facts(facts, record, candidate_sha, now)
    receipt = {
        "schema_version": RECEIPT_SCHEMA,
        "workflow_ref": args.workflow_ref,
        "workflow_sha": workflow_sha,
        "run_id": run_id,
        "run_attempt": run_attempt,
        "manifest_sha256": hashlib.sha256(raw_manifest).hexdigest(),
        "candidate_sha": candidate_sha,
        "candidate_pr": record["candidate_pr"],
        "approval_review_ids": review_ids,
        "api_response_sha256": facts["api_response_sha256"],
    }
    write_receipt(args.receipt, receipt)
    if args.github_output is not None:
        if args.github_output.is_symlink():
            fail("GitHub output must not be a symlink")
        try:
            with args.github_output.open("a", encoding="ascii") as handle:
                handle.write("authorized=true\n")
                handle.write(f"manifest_sha256={receipt['manifest_sha256']}\n")
        except OSError as error:
            fail(f"cannot write GitHub output: {error}")
    print("cuda-unmerged-candidate: authorized")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    authorize_parser = commands.add_parser("authorize")
    authorize_parser.add_argument("--manifest", type=Path, required=True)
    authorize_parser.add_argument("--facts", type=Path, required=True)
    authorize_parser.add_argument("--candidate-sha", required=True)
    authorize_parser.add_argument("--workflow-ref", required=True)
    authorize_parser.add_argument("--workflow-sha", required=True)
    authorize_parser.add_argument("--run-id", type=int, required=True)
    authorize_parser.add_argument("--run-attempt", type=int, required=True)
    authorize_parser.add_argument("--now", required=True)
    authorize_parser.add_argument("--receipt", type=Path, required=True)
    authorize_parser.add_argument("--github-output", type=Path)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.command == "authorize":
        authorize(args)
    else:  # pragma: no cover - argparse makes this unreachable
        fail("unknown command")


if __name__ == "__main__":
    main()
