#!/usr/bin/env bash
# Contract tests for the local-only unmerged CUDA-candidate provenance route.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
AUTHORIZE="$REPO_ROOT/scripts/release/verify-cuda-unmerged-candidate.py"
VERIFY_RECEIPT="$REPO_ROOT/scripts/release/verify-cuda-unmerged-receipt.py"
MANIFEST="$REPO_ROOT/dev/release/cuda-unmerged-candidates.json"

TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT

CANDIDATE='0123456789abcdef0123456789abcdef01234567'
OTHER_CANDIDATE='89abcdef0123456789abcdef0123456789abcdef'
WORKFLOW_SHA='fedcba9876543210fedcba9876543210fedcba98'
WORKFLOW_REF='fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/main'
NOW='2026-08-17T18:00:00Z'

write_fixture() {
  local manifest="$1" facts="$2" candidate="$3"
  python3 - "$manifest" "$facts" "$candidate" "$NOW" <<'PY'
import hashlib
import json
import sys

manifest_path, facts_path, candidate, now = sys.argv[1:]

def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode() + b"\n"

def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()

record = {
    "schema_version": "fathomdb.cuda-unmerged-candidate/v1",
    "candidate_sha": candidate,
    "candidate_pr": 228,
    "candidate_pr_head_sha": candidate,
    "required_reviewers": ["independent-reviewer"],
    "expires_at": "2026-08-18T18:00:00Z",
    "purpose": "0.8.23 non-publishing CUDA preflight",
    "provenance_pr": 229,
    "provenance_commit": "1111111111111111111111111111111111111111",
    "provenance_required_reviewers": ["independent-provenance-reviewer"],
}
manifest = {
    "schema_version": "fathomdb.cuda-unmerged-candidates/v1",
    "candidates": [record],
}
baseline = {
    "schema_version": "fathomdb.cuda-protection-baseline/v1",
    "expires_at": "2026-08-18T18:00:00Z",
    "main_protection_sha256": "a" * 64,
    "runner_group_sha256": "b" * 64,
    "environment_sha256": "c" * 64,
}
pr = {
    "number": 228,
    "state": "open",
    "draft": False,
    "base": {"ref": "main", "repo_full_name": "fathomadb/fathomdb"},
    "head": {"sha": candidate, "repo_full_name": "fathomadb/fathomdb"},
    "user": {"login": "candidate-author"},
}
reviews = [{
    "id": 321,
    "user": {"login": "independent-reviewer"},
    "state": "APPROVED",
    "commit_id": candidate,
    "submitted_at": "2026-08-17T17:00:00Z",
}]
provenance_pr = {
    "number": 229,
    "state": "closed",
    "merged": True,
    "base": {"ref": "main", "repo_full_name": "fathomadb/fathomdb"},
    "merge_commit_sha": "1111111111111111111111111111111111111111",
    "user": {"login": "provenance-author"},
}
provenance_reviews = [{
    "id": 322,
    "user": {"login": "independent-provenance-reviewer"},
    "state": "APPROVED",
    "commit_id": "1111111111111111111111111111111111111111",
    "submitted_at": "2026-08-17T16:00:00Z",
}]
facts = {
    "schema_version": "fathomdb.cuda-unmerged-candidate-facts/v1",
    "protection_baseline": baseline,
    "pull_request": pr,
    "reviews": reviews,
    "reviews_pagination_complete": True,
    "provenance_pull_request": provenance_pr,
    "provenance_reviews": provenance_reviews,
    "provenance_reviews_pagination_complete": True,
    "api_response_sha256": {
        "protection_baseline": digest(baseline),
        "pull_request": digest(pr),
        "reviews": digest(reviews),
        "provenance_pull_request": digest(provenance_pr),
        "provenance_reviews": digest(provenance_reviews),
    },
}
open(manifest_path, "wb").write(canonical(manifest))
open(facts_path, "wb").write(canonical(facts))
PY
}

expect_authorize_pass() {
  local manifest="$1" facts="$2" receipt="$3" description="$4"
  if python3 "$AUTHORIZE" authorize \
    --manifest "$manifest" --facts "$facts" --candidate-sha "$CANDIDATE" \
    --workflow-ref "$WORKFLOW_REF" --workflow-sha "$WORKFLOW_SHA" \
    --run-id 714 --run-attempt 2 --now "$NOW" --receipt "$receipt" >/dev/null; then
    printf 'PASS  %s\n' "$description"
  else
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
}

expect_authorize_fail() {
  local manifest="$1" facts="$2" receipt="$3" description="$4"
  if python3 "$AUTHORIZE" authorize \
    --manifest "$manifest" --facts "$facts" --candidate-sha "$CANDIDATE" \
    --workflow-ref "$WORKFLOW_REF" --workflow-sha "$WORKFLOW_SHA" \
    --run-id 714 --run-attempt 2 --now "$NOW" --receipt "$receipt" >/dev/null 2>&1; then
    printf 'FAIL  %s\n' "$description" >&2
    exit 1
  fi
  printf 'PASS  %s\n' "$description"
}

empty_manifest="$TMPROOT/empty-manifest.json"
facts="$TMPROOT/facts.json"
receipt="$TMPROOT/receipt.json"
python3 - "$empty_manifest" <<'PY'
import json
import sys
open(sys.argv[1], "w", encoding="utf-8").write(json.dumps({
    "schema_version": "fathomdb.cuda-unmerged-candidates/v1", "candidates": []
}, sort_keys=True, separators=(",", ":")) + "\n")
PY
write_fixture "$TMPROOT/authorized.json" "$facts" "$CANDIDATE"
expect_authorize_fail "$empty_manifest" "$facts" "$receipt" 'rejects an absent candidate authorization record'

manifest="$TMPROOT/authorized.json"
expect_authorize_pass "$manifest" "$facts" "$receipt" 'accepts one exact, unexpired reviewed candidate record'

python3 "$VERIFY_RECEIPT" \
  --receipt "$receipt" --candidate-sha "$CANDIDATE" --workflow-ref "$WORKFLOW_REF" \
  --workflow-sha "$WORKFLOW_SHA" --run-id 714 --run-attempt 2 >/dev/null
printf 'PASS  receipt binds the exact hosted run, workflow, and candidate\n'

mutate_and_expect_fail() {
  local mutation="$1" description="$2"
  local mutated_manifest="$TMPROOT/${mutation}-manifest.json"
  local mutated_facts="$TMPROOT/${mutation}-facts.json"
  cp "$manifest" "$mutated_manifest"
  cp "$facts" "$mutated_facts"
  python3 - "$mutated_manifest" "$mutated_facts" "$mutation" "$CANDIDATE" <<'PY'
import json
import sys

manifest_path, facts_path, mutation, candidate = sys.argv[1:]
manifest = json.load(open(manifest_path, encoding="utf-8"))
facts = json.load(open(facts_path, encoding="utf-8"))
record = manifest["candidates"][0]
if mutation == "unknown-field":
    record["bypass"] = True
elif mutation == "duplicate":
    manifest["candidates"].append(dict(record))
elif mutation == "expired":
    record["expires_at"] = "2026-08-16T18:00:00Z"
elif mutation == "mismatched":
    record["candidate_pr_head_sha"] = "f" * 40
elif mutation == "foreign-pr":
    facts["pull_request"]["head"]["repo_full_name"] = "attacker/fork"
elif mutation == "draft-pr":
    facts["pull_request"]["draft"] = True
elif mutation == "retargeted-pr":
    facts["pull_request"]["base"]["ref"] = "release/0.8.23"
elif mutation == "changed-head":
    facts["pull_request"]["head"]["sha"] = "f" * 40
elif mutation == "self-approved":
    facts["pull_request"]["user"]["login"] = "independent-reviewer"
elif mutation == "stale-approval":
    facts["reviews"][0]["commit_id"] = "f" * 40
elif mutation == "change-request":
    facts["reviews"][0]["state"] = "CHANGES_REQUESTED"
elif mutation == "pagination":
    facts["reviews_pagination_complete"] = False
elif mutation == "api-digest":
    facts["api_response_sha256"]["reviews"] = "0" * 64
elif mutation == "provenance-unmerged":
    facts["provenance_pull_request"]["merged"] = False
elif mutation == "provenance-wrong-commit":
    facts["provenance_pull_request"]["merge_commit_sha"] = "f" * 40
elif mutation == "provenance-self-approved":
    facts["provenance_pull_request"]["user"]["login"] = "independent-provenance-reviewer"
elif mutation == "provenance-review-changed":
    facts["provenance_reviews"][0]["state"] = "CHANGES_REQUESTED"
else:
    raise SystemExit("unknown mutation: " + mutation)
for path, value in ((manifest_path, manifest), (facts_path, facts)):
    open(path, "w", encoding="utf-8").write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
PY
  expect_authorize_fail "$mutated_manifest" "$mutated_facts" "$TMPROOT/$mutation-receipt.json" "$description"
}

mutate_and_expect_fail unknown-field 'rejects unknown manifest fields'
mutate_and_expect_fail duplicate 'rejects duplicate candidate records'
mutate_and_expect_fail expired 'rejects expired candidate records'
mutate_and_expect_fail mismatched 'rejects a candidate record with mismatched head SHA'
mutate_and_expect_fail foreign-pr 'rejects a foreign candidate PR'
mutate_and_expect_fail draft-pr 'rejects a draft candidate PR'
mutate_and_expect_fail retargeted-pr 'rejects a PR not targeted at main'
mutate_and_expect_fail changed-head 'rejects a changed candidate PR head'
mutate_and_expect_fail self-approved 'rejects self-approval'
mutate_and_expect_fail stale-approval 'rejects approval for a different commit'
mutate_and_expect_fail change-request 'rejects non-approved latest reviewer state'
mutate_and_expect_fail pagination 'rejects incomplete review pagination'
mutate_and_expect_fail api-digest 'rejects API response digest mismatch'
mutate_and_expect_fail provenance-unmerged 'rejects an unmerged provenance PR'
mutate_and_expect_fail provenance-wrong-commit 'rejects a provenance PR with a different merge commit'
mutate_and_expect_fail provenance-self-approved 'rejects provenance self-approval'
mutate_and_expect_fail provenance-review-changed 'rejects a changed provenance review'

for mismatch in candidate workflow-ref workflow-sha run-id run-attempt; do
  args=(--receipt "$receipt" --candidate-sha "$CANDIDATE" --workflow-ref "$WORKFLOW_REF" --workflow-sha "$WORKFLOW_SHA" --run-id 714 --run-attempt 2)
  case "$mismatch" in
    candidate) args[3]="$OTHER_CANDIDATE" ;;
    workflow-ref) args[5]='fathomadb/fathomdb/.github/workflows/release.yml@refs/heads/release/0.8.23' ;;
    workflow-sha) args[7]="$OTHER_CANDIDATE" ;;
    run-id) args[9]=715 ;;
    run-attempt) args[11]=3 ;;
  esac
  if python3 "$VERIFY_RECEIPT" "${args[@]}" >/dev/null 2>&1; then
    printf 'FAIL  rejects receipt mismatch: %s\n' "$mismatch" >&2
    exit 1
  fi
  printf 'PASS  rejects receipt mismatch: %s\n' "$mismatch"
done

printf '\nCUDA unmerged-candidate provenance tests passed\n'
