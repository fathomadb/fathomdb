"""RED-first verification tests for ``scripts/worktree-consolidator.py``.

The consolidator does not exist yet.  Each test loads the command only inside
its body, so an absent implementation produces attributable failures instead
of aborting pytest collection and hiding criteria.  Every fixture is a fresh
Git repository under pytest's temporary directory; this suite never mutates
the FathomDB checkout, its worktrees, or its refs.
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from datetime import UTC, datetime, timedelta
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest


REPO_ROOT = Path(__file__).resolve().parents[2]
TOOL = REPO_ROOT / "scripts" / "worktree-consolidator.py"


def git(repo: Path, *args: str) -> str:
    """Run Git in a throwaway fixture and return standard output."""
    result = subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout


def require_tool(criterion: str) -> Path:
    """Return the implementation path or fail the named acceptance criterion."""
    if not TOOL.is_file():
        pytest.fail(
            f"{criterion} is NOT SATISFIED: {TOOL.relative_to(REPO_ROOT)} is "
            "not implemented. This is the expected RED state before the "
            "implementation slice."
        )
    return TOOL


def load_tool_module() -> Any:
    """Load an isolated module instance for narrow receipt fault-injection tests."""
    spec = importlib.util.spec_from_file_location("worktree_consolidator_test", require_tool("AC-WTC-008"))
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def run_tool(
    repo: Path, *args: str, criterion: str = "AC-WTC-001"
) -> subprocess.CompletedProcess[str]:
    """Invoke the local tool without a shell or any network access."""
    tool = require_tool(criterion)
    return subprocess.run(
        [sys.executable, str(tool), *args],
        cwd=repo,
        capture_output=True,
        text=True,
        check=False,
    )


def canonical_json(value: Any) -> bytes:
    """Encode fixture evidence with the required canonical JSON convention."""
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def write_json(path: Path, value: Any) -> str:
    """Write canonical fixture evidence and return its SHA-256."""
    payload = canonical_json(value)
    path.write_bytes(payload)
    return hashlib.sha256(payload).hexdigest()


def repo_fingerprint(repo: Path) -> tuple[str, str, str, tuple[tuple[str, str], ...]]:
    """Capture Git-visible and `.git` file content needed for audit no-write checks."""
    common_dir = Path(git(repo, "rev-parse", "--git-common-dir").strip()).resolve()
    if not common_dir.is_absolute():
        common_dir = (repo / common_dir).resolve()
    files = []
    for path in sorted(common_dir.rglob("*")):
        if path.is_file():
            files.append(
                (
                    str(path.relative_to(common_dir)),
                    hashlib.sha256(path.read_bytes()).hexdigest(),
                )
            )
    return (
        git(repo, "status", "--porcelain=v1", "--ignored"),
        git(repo, "worktree", "list", "--porcelain"),
        git(repo, "show-ref", "--head"),
        tuple(files),
    )


def registered_worktree_paths(repo: Path) -> list[Path]:
    """Return canonical registered paths from Git's porcelain worktree output."""
    return [
        Path(line.removeprefix("worktree ")).resolve()
        for line in git(repo, "worktree", "list", "--porcelain").splitlines()
        if line.startswith("worktree ")
    ]


@pytest.fixture
def fixture_repo(tmp_path: Path) -> Path:
    """Create a primary checkout and one clean linked worktree."""
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-b", "main")
    git(repo, "config", "user.email", "wtc-test@example.invalid")
    git(repo, "config", "user.name", "wtc test")
    git(repo, "config", "commit.gpgsign", "false")
    (repo / "README.md").write_text("fixture\n", encoding="utf-8")
    git(repo, "add", "README.md")
    git(repo, "commit", "-m", "fixture", "--quiet")
    git(repo, "update-ref", "refs/remotes/origin/main", "HEAD")
    git(repo, "worktree", "add", "-b", "retirable", str(tmp_path / "linked"))
    return repo


@pytest.fixture
def evidence_dir(tmp_path: Path) -> Path:
    """Create a private external evidence directory for fixture attestations."""
    path = tmp_path / "evidence"
    path.mkdir(mode=0o700)
    os.chmod(path, 0o700)
    return path


def owner_map(repo: Path) -> dict[str, Any]:
    """Return complete explicit no-owner evidence for the clean fixture."""
    worktrees = registered_worktree_paths(repo)
    heads = [
        line.split(" ", 1)[0]
        for line in git(repo, "for-each-ref", "--format=%(refname) %(objectname)", "refs/heads").splitlines()
    ]
    return {
        "schema": "fathomdb-worktree-owner-map/v1",
        "entries": sorted(
            [
                {
                    "target": str(path),
                    "owner": "none",
                    "release_role": "none",
                    "evidence": "fixture",
                }
                for path in worktrees
            ]
            + [
                {
                    "target": ref,
                    "owner": "none",
                    "release_role": "none",
                    "evidence": "fixture",
                }
                for ref in heads
            ],
            key=lambda entry: entry["target"],
        ),
    }


def now_window(seconds: int = 900) -> tuple[str, str]:
    """Return an evidence issue/expiry pair valid for the current test run."""
    issued = datetime.now(UTC)
    return (
        issued.isoformat().replace("+00:00", "Z"),
        (issued + timedelta(seconds=seconds)).isoformat().replace("+00:00", "Z"),
    )


def audit_snapshot(repo: Path, evidence_dir: Path) -> tuple[Path, dict[str, Any], Path]:
    """Produce a real audit snapshot and matching complete fixture owner map."""
    map_path = evidence_dir / "owner-map.json"
    write_json(map_path, owner_map(repo))
    result = run_tool(
        repo,
        "audit",
        "--repo",
        str(repo),
        "--owner-map",
        str(map_path),
        "--json",
        criterion="AC-WTC-001",
    )
    assert result.returncode == 0, result.stderr
    snapshot = json.loads(result.stdout)
    path = evidence_dir / "snapshot.json"
    path.write_text(result.stdout, encoding="utf-8")
    return path, snapshot, map_path


def policy_and_baseline(
    evidence_dir: Path,
    snapshot: dict[str, Any],
    *,
    target_range: list[int],
    retire_local_heads: bool = False,
) -> tuple[Path, Path]:
    """Write valid policy and baseline evidence for a real manifest fixture."""
    issued_at, expires_at = now_window()
    baseline_path = evidence_dir / "baseline.json"
    write_json(
        baseline_path,
        {
            "schema": "fathomdb-worktree-baseline-attestation/v1",
            "repository": snapshot["repository"],
            "baseline": snapshot["baseline"],
            "fetched_at": issued_at,
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    policy_path = evidence_dir / "policy.json"
    write_json(
        policy_path,
        {
            "primary_role": "main",
            "active_themes": [],
            "legacy_triage_required": False,
            "target_range": target_range,
            "baseline_max_age_seconds": 900,
            "dryrun_max_age_seconds": 900,
            "theme_targets": {},
            "retire_local_heads": retire_local_heads,
            "reflog_candidates": {
                candidate: "preserve-in-bundle"
                for candidate in snapshot["recovery_candidates"]
            },
        },
    )
    return policy_path, baseline_path


def owner_map_review(evidence_dir: Path, snapshot: dict[str, Any], owner_path: Path) -> Path:
    """Write independent approval evidence for the complete fixture owner map."""
    issued_at, expires_at = now_window()
    path = evidence_dir / "owner-map-review.json"
    write_json(
        path,
        {
            "schema": "fathomdb-worktree-owner-map-review-attestation/v1",
            "repository": snapshot["repository"],
            "owner_map_sha256": hashlib.sha256(owner_path.read_bytes()).hexdigest(),
            "reviewer": "fixture-owner-map-reviewer",
            "decision": "approved",
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    return path


def stable_patch_id(repo: Path, commit: str) -> str:
    """Return Git's stable patch ID for one fixture commit."""
    patch = subprocess.run(
        ["git", "show", "--pretty=format:", "--patch", "--no-ext-diff", "--no-textconv", "--no-renames", "--binary", commit],
        cwd=repo,
        check=True,
        capture_output=True,
    ).stdout
    result = subprocess.run(
        ["git", "patch-id", "--stable"],
        cwd=repo,
        input=patch,
        check=True,
        capture_output=True,
    )
    return result.stdout.decode("ascii").split()[0]


def commit_on_new_branch(
    repo: Path, name: str, base: str, changes: list[tuple[str, str]]
) -> list[str]:
    """Create fixture commits on a temporary linked worktree, then remove it."""
    path = repo.parent / f"{name}-worktree"
    git(repo, "worktree", "add", "-b", name, str(path), base)
    commits = []
    for index, (filename, contents) in enumerate(changes, start=1):
        (path / filename).write_text(contents, encoding="utf-8")
        git(path, "add", filename)
        git(path, "commit", "-m", f"{name}-{index}", "--quiet")
        commits.append(git(path, "rev-parse", "HEAD").strip())
    git(repo, "worktree", "remove", str(path))
    return commits


def proof_owner_map(repo: Path, retained_refs: tuple[str, ...] = ()) -> dict[str, Any]:
    """Return explicit fixture ownership with selected refs marked retained."""
    value = owner_map(repo)
    for entry in value["entries"]:
        if entry["target"] in retained_refs:
            entry.update(
                {
                    "owner": "fixture-retainer",
                    "release_role": "retained-proof-anchor",
                    "evidence": "explicit fixture retention",
                }
            )
    return value


def common_proof(target: str, tip: str, proof_type: str) -> dict[str, Any]:
    """Return the closed common fields for one reviewed fixture proof."""
    return {
        "target": target,
        "target_tip": tip,
        "proof_type": proof_type,
        "semantic_disposition": "retire-local-ref",
        "evidence_id": f"fixture-decision:{target}",
    }


def make_proof_scenario(
    repo: Path, proof_type: str, *, relation: str = "same-tip"
) -> tuple[dict[str, Any], tuple[str, ...], str | None]:
    """Materialize one valid non-ancestor proof scenario."""
    initial = git(repo, "rev-list", "--max-parents=0", "HEAD").strip()
    target_ref = "refs/heads/proof-target"
    retained: tuple[str, ...] = ()
    remote_ref: str | None = None
    if proof_type == "stable_patch_coverage":
        (repo / "covered.txt").write_text("covered\n", encoding="utf-8")
        git(repo, "add", "covered.txt")
        git(repo, "commit", "-m", "baseline-version", "--quiet")
        baseline_commit = git(repo, "rev-parse", "HEAD").strip()
        git(repo, "update-ref", "refs/remotes/origin/main", baseline_commit)
        [target_tip] = commit_on_new_branch(
            repo, "proof-target", initial, [("covered.txt", "covered\n")]
        )
        proof = common_proof(target_ref, target_tip, proof_type)
        proof["source_commits"] = [
            {
                "commit": target_tip,
                "stable_patch_id": stable_patch_id(repo, target_tip),
                "baseline_matches": [baseline_commit],
            }
        ]
    elif proof_type == "retained_local_ref":
        [target_tip] = commit_on_new_branch(
            repo, "proof-target", initial, [("unique.txt", "unique\n")]
        )
        git(repo, "branch", "proof-retained", target_tip)
        retained_tip = target_tip
        if relation == "ancestor":
            [retained_tip] = commit_on_new_branch(
                repo,
                "proof-descendant",
                target_tip,
                [("descendant.txt", "descendant\n")],
            )
            git(repo, "branch", "-f", "proof-retained", retained_tip)
            git(repo, "branch", "-D", "proof-descendant")
        retained_ref = "refs/heads/proof-retained"
        retained = (retained_ref,)
        proof = common_proof(target_ref, target_tip, proof_type)
        proof.update(
            {
                "relation": relation,
                "retained_ref": retained_ref,
                "retained_tip": retained_tip,
            }
        )
    else:
        [target_tip] = commit_on_new_branch(
            repo, "proof-target", initial, [("remote.txt", "remote\n")]
        )
        remote_ref = "refs/remotes/origin/proof-target"
        git(repo, "update-ref", remote_ref, target_tip)
        proof = common_proof(target_ref, target_tip, proof_type)
        proof.update({"remote_ref": remote_ref, "remote_tip": target_tip})
    return proof, retained, remote_ref


def proof_manifest_context(
    repo: Path,
    evidence_dir: Path,
    proofs: list[dict[str, Any]],
    *,
    retained_refs: tuple[str, ...] = (),
    author: str = "fixture-semantic-owner",
    reviewer: str = "fixture-independent-reviewer",
    approval_decision: str = "approved",
    approval_expired: bool = False,
    approval_hash: str | None = None,
    proof_schema: str = "fathomdb-worktree-retirement-proofs/v1",
) -> dict[str, Any]:
    """Build reviewed proof evidence and invoke manifest through the public CLI."""
    map_path = evidence_dir / "owner-map.json"
    owner_hash = write_json(map_path, proof_owner_map(repo, retained_refs))
    audit = run_tool(
        repo, "audit", "--repo", str(repo), "--owner-map", str(map_path), "--json",
        criterion="AC-WTC-P04",
    )
    assert audit.returncode == 0, audit.stderr
    snapshot = json.loads(audit.stdout)
    snapshot_path = evidence_dir / "snapshot.json"
    snapshot_path.write_text(audit.stdout, encoding="utf-8")
    owner_review_path = owner_map_review(evidence_dir, snapshot, map_path)
    policy_path, baseline_path = policy_and_baseline(
        evidence_dir, snapshot, target_range=[1, 8], retire_local_heads=True
    )
    issued_at, expires_at = now_window()
    proof_path = evidence_dir / "retirement-proofs.json"
    proof_hash = write_json(
        proof_path,
        {
            "schema": proof_schema,
            "repository": snapshot["repository"],
            "baseline": snapshot["baseline"],
            "owner_map_sha256": owner_hash,
            "author": author,
            "issued_at": issued_at,
            "proofs": proofs,
        },
    )
    proof_approval_path = evidence_dir / "retirement-proof-approval.json"
    write_json(
        proof_approval_path,
        {
            "schema": "fathomdb-worktree-retirement-proof-approval/v1",
            "repository": snapshot["repository"],
            "owner_map_sha256": owner_hash,
            "retirement_proofs_sha256": approval_hash or proof_hash,
            "reviewer": reviewer,
            "decision": approval_decision,
            "issued_at": issued_at,
            "expires_at": "2000-01-01T00:00:00Z" if approval_expired else expires_at,
        },
    )
    manifest_path = evidence_dir / "candidate.json"
    result = run_tool(
        repo,
        "manifest",
        "--repo", str(repo), "--audit", str(snapshot_path),
        "--owner-map", str(map_path),
        "--owner-map-review-attestation", str(owner_review_path),
        "--policy", str(policy_path), "--baseline-attestation", str(baseline_path),
        "--retirement-proofs", str(proof_path),
        "--retirement-proof-approval", str(proof_approval_path),
        "--evidence-dir", str(evidence_dir), "--target-worktrees", "2",
        "--output", str(manifest_path), "--json", criterion="AC-WTC-P01",
    )
    return {
        "result": result,
        "snapshot": snapshot,
        "manifest_path": manifest_path,
        "owner_path": map_path,
        "owner_review_path": owner_review_path,
        "baseline_path": baseline_path,
        "proof_path": proof_path,
        "proof_approval_path": proof_approval_path,
    }


def approve_and_dryrun(repo: Path, evidence_dir: Path, context: dict[str, Any]) -> dict[str, Any]:
    """Approve a valid proof manifest and complete its dry run."""
    manifest = json.loads(context["manifest_path"].read_text(encoding="utf-8"))
    manifest_hash = hashlib.sha256(context["manifest_path"].read_bytes()).hexdigest()
    issued_at, expires_at = now_window()
    approval_path = evidence_dir / "approval.json"
    write_json(
        approval_path,
        {
            "schema": "fathomdb-worktree-approval-attestation/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "reviewer": "fixture-manifest-reviewer",
            "decision": "approved",
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    archive_dir = repo.parent / "archive"
    archive_dir.mkdir(mode=0o700)
    result = run_tool(
        repo,
        "dryrun",
        "--repo", str(repo), "--manifest", str(context["manifest_path"]),
        "--owner-map", str(context["owner_path"]),
        "--owner-map-review-attestation", str(context["owner_review_path"]),
        "--approval-attestation", str(approval_path),
        "--baseline-attestation", str(context["baseline_path"]),
        "--retirement-proofs", str(context["proof_path"]),
        "--retirement-proof-approval", str(context["proof_approval_path"]),
        "--archive-dir", str(archive_dir), "--evidence-dir", str(evidence_dir),
        "--json", criterion="AC-WTC-P05",
    )
    receipt_path = evidence_dir / f"dryrun-{manifest_hash[:16]}.json"
    context.update(
        {
            "manifest": manifest,
            "manifest_hash": manifest_hash,
            "approval_path": approval_path,
            "archive_dir": archive_dir,
            "dryrun_result": result,
            "dryrun_path": receipt_path,
        }
    )
    return context


def consolidate_context(repo: Path, evidence_dir: Path, context: dict[str, Any]) -> subprocess.CompletedProcess[str]:
    """Freeze and invoke consolidation for a previously successful dry run."""
    issued_at, expires_at = now_window()
    freeze_path = evidence_dir / "freeze.json"
    write_json(
        freeze_path,
        {
            "schema": "fathomdb-worktree-freeze-attestation/v1",
            "repository": context["manifest"]["repository"],
            "manifest_sha256": context["manifest_hash"],
            "dryrun_receipt_sha256": hashlib.sha256(context["dryrun_path"].read_bytes()).hexdigest(),
            "snapshot_id": context["manifest"]["snapshot_id"],
            "operator": "fixture-operator",
            "writers": [],
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    return run_tool(
        repo,
        "consolidate",
        "--repo", str(repo), "--manifest", str(context["manifest_path"]),
        "--owner-map", str(context["owner_path"]),
        "--owner-map-review-attestation", str(context["owner_review_path"]),
        "--approval-attestation", str(context["approval_path"]),
        "--baseline-attestation", str(context["baseline_path"]),
        "--retirement-proofs", str(context["proof_path"]),
        "--retirement-proof-approval", str(context["proof_approval_path"]),
        "--dryrun-receipt", str(context["dryrun_path"]),
        "--freeze-attestation", str(freeze_path),
        "--archive-dir", str(context["archive_dir"]),
        "--evidence-dir", str(evidence_dir),
        "--confirm-manifest-sha256", context["manifest_hash"],
        "--confirm", f"CONSOLIDATE {context['manifest']['manifest_id']}",
        "--json", criterion="AC-WTC-P06",
    )


def generate_manifest(
    repo: Path, evidence_dir: Path, target: int, *, retire_local_heads: bool = False
) -> tuple[Path, dict[str, Any], Path, Path]:
    """Generate one candidate manifest through the public CLI."""
    snapshot_path, snapshot, map_path = audit_snapshot(repo, evidence_dir)
    policy_path, baseline_path = policy_and_baseline(
        evidence_dir,
        snapshot,
        target_range=[1, 8],
        retire_local_heads=retire_local_heads,
    )
    review_path = owner_map_review(evidence_dir, snapshot, map_path)
    manifest_path = evidence_dir / "candidate.json"
    result = run_tool(
        repo,
        "manifest",
        "--repo",
        str(repo),
        "--audit",
        str(snapshot_path),
        "--owner-map",
        str(map_path),
        "--owner-map-review-attestation",
        str(review_path),
        "--policy",
        str(policy_path),
        "--baseline-attestation",
        str(baseline_path),
        "--evidence-dir",
        str(evidence_dir),
        "--target-worktrees",
        str(target),
        "--output",
        str(manifest_path),
        "--json",
        criterion="AC-WTC-002",
    )
    assert result.returncode == 0, result.stderr
    return manifest_path, json.loads(result.stdout), map_path, baseline_path


def test_ac_wtc_001_cli_exposes_exact_five_modes(fixture_repo: Path):
    """AC-WTC-001: the public CLI exposes the authority-separated pipeline."""
    result = run_tool(fixture_repo, "--help")
    assert result.returncode == 0, result.stderr
    for mode in ("audit", "manifest", "dryrun", "consolidate", "status"):
        assert mode in result.stdout


def test_ac_wtc_001_audit_is_observational_and_classifies_fixture(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-001: audit writes neither Git state nor a report file."""
    map_path = evidence_dir / "owner-map.json"
    write_json(map_path, owner_map(fixture_repo))
    before = repo_fingerprint(fixture_repo)

    result = run_tool(
        fixture_repo,
        "audit",
        "--repo",
        str(fixture_repo),
        "--owner-map",
        str(map_path),
        "--json",
        criterion="AC-WTC-001",
    )

    assert result.returncode == 0, result.stderr
    snapshot = json.loads(result.stdout)
    assert snapshot["repository"]["primary_root"] == str(fixture_repo.resolve())
    assert snapshot["owner_map_sha256"] == hashlib.sha256(map_path.read_bytes()).hexdigest()
    assert {entry["path"] for entry in snapshot["worktrees"]} == {
        str(path) for path in registered_worktree_paths(fixture_repo)
    }
    assert repo_fingerprint(fixture_repo) == before


def test_ac_wtc_001_audit_explains_each_proposed_head_retirement(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-001: audit makes proposed branch retirement reviewable by predicate."""
    git(fixture_repo, "branch", "stale-local-head", "origin/main")
    map_path = evidence_dir / "owner-map.json"
    write_json(map_path, owner_map(fixture_repo))

    result = run_tool(
        fixture_repo,
        "audit",
        "--repo", str(fixture_repo), "--owner-map", str(map_path), "--json",
        criterion="AC-WTC-001",
    )

    assert result.returncode == 0, result.stderr
    review = json.loads(result.stdout)["retirement_review"]
    assert review["schema"] == "fathomdb-worktree-retirement-review/v1"
    proposed = next(
        entry for entry in review["entries"] if entry["target"] == "refs/heads/stale-local-head"
    )
    assert proposed == {
        "target": "refs/heads/stale-local-head",
        "tip": git(fixture_repo, "rev-parse", "refs/heads/stale-local-head").strip(),
        "is_main": False,
        "checked_out_by_worktree": False,
        "ancestor_of_baseline": True,
        "matching_remote_refs": ["refs/remotes/origin/main"],
        "owner_map_evidence": "fixture",
        "result": "mechanically-eligible",
    }
    main = next(entry for entry in review["entries"] if entry["target"] == "refs/heads/main")
    assert main["is_main"] is True
    assert main["result"] == "blocked"


def test_ac_wtc_002_manifest_refuses_goal_below_theme_lower_bound(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-002: target planning fails closed instead of deleting a theme."""
    map_path = evidence_dir / "owner-map.json"
    write_json(map_path, owner_map(fixture_repo))
    audit = run_tool(
        fixture_repo,
        "audit",
        "--repo",
        str(fixture_repo),
        "--owner-map",
        str(map_path),
        "--json",
        criterion="AC-WTC-002",
    )
    assert audit.returncode == 0, audit.stderr
    snapshot_path = evidence_dir / "snapshot.json"
    snapshot_path.write_text(audit.stdout, encoding="utf-8")
    snapshot = json.loads(audit.stdout)
    review_path = owner_map_review(evidence_dir, snapshot, map_path)

    baseline_path = evidence_dir / "baseline.json"
    issued_at = datetime.now(UTC)
    write_json(
        baseline_path,
        {
            "schema": "fathomdb-worktree-baseline-attestation/v1",
            "repository": snapshot["repository"],
            "baseline": snapshot["baseline"],
            "fetched_at": issued_at.isoformat().replace("+00:00", "Z"),
            "issued_at": issued_at.isoformat().replace("+00:00", "Z"),
            "expires_at": (issued_at + timedelta(minutes=15)).isoformat().replace(
                "+00:00", "Z"
            ),
        },
    )
    policy_path = evidence_dir / "policy.json"
    write_json(
        policy_path,
        {
            "primary_role": "main",
            "active_themes": ["campaign"],
            "legacy_triage_required": False,
            "target_range": [1, 7],
            "baseline_max_age_seconds": 900,
            "dryrun_max_age_seconds": 900,
            "theme_targets": {
                "campaign": str(fixture_repo.resolve()),
            },
            "retire_local_heads": False,
            "reflog_candidates": {},
        },
    )

    result = run_tool(
        fixture_repo,
        "manifest",
        "--repo",
        str(fixture_repo),
        "--audit",
        str(snapshot_path),
        "--owner-map",
        str(map_path),
        "--owner-map-review-attestation",
        str(review_path),
        "--policy",
        str(policy_path),
        "--baseline-attestation",
        str(baseline_path),
        "--evidence-dir",
        str(evidence_dir),
        "--target-worktrees",
        "1",
        "--json",
        criterion="AC-WTC-002",
    )

    assert result.returncode == 3
    assert "goal_inference_blocked" in result.stdout


def test_ac_wtc_002_manifest_announces_missing_owner_map_review(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-002: planning cannot turn an unreviewed map into an action plan."""
    snapshot_path, snapshot, map_path = audit_snapshot(fixture_repo, evidence_dir)
    policy_path, baseline_path = policy_and_baseline(evidence_dir, snapshot, target_range=[1, 8])
    output_path = evidence_dir / "must-not-exist.json"

    result = run_tool(
        fixture_repo,
        "manifest",
        "--repo", str(fixture_repo), "--audit", str(snapshot_path),
        "--owner-map", str(map_path), "--policy", str(policy_path),
        "--baseline-attestation", str(baseline_path), "--evidence-dir", str(evidence_dir),
        "--target-worktrees", "2", "--output", str(output_path), "--json", criterion="AC-WTC-002",
    )

    assert result.returncode == 3
    assert json.loads(result.stdout)["result"] == "owner_map_review_required"
    assert not output_path.exists()


@pytest.mark.parametrize(
    ("field", "replacement"),
    [
        ("reviewer", ""),
        ("decision", "rejected"),
        ("owner_map_sha256", "0" * 64),
        ("expires_at", "2000-01-01T00:00:00Z"),
    ],
)
def test_ac_wtc_002_manifest_rejects_invalid_owner_map_review(
    fixture_repo: Path, evidence_dir: Path, field: str, replacement: str
):
    """AC-WTC-002: review evidence is strict before a candidate can be written."""
    snapshot_path, snapshot, map_path = audit_snapshot(fixture_repo, evidence_dir)
    policy_path, baseline_path = policy_and_baseline(evidence_dir, snapshot, target_range=[1, 8])
    review_path = owner_map_review(evidence_dir, snapshot, map_path)
    review = json.loads(review_path.read_text(encoding="utf-8"))
    review[field] = replacement
    write_json(review_path, review)
    output_path = evidence_dir / "must-not-exist.json"

    result = run_tool(
        fixture_repo,
        "manifest",
        "--repo", str(fixture_repo), "--audit", str(snapshot_path),
        "--owner-map", str(map_path), "--owner-map-review-attestation", str(review_path),
        "--policy", str(policy_path), "--baseline-attestation", str(baseline_path),
        "--evidence-dir", str(evidence_dir), "--target-worktrees", "2",
        "--output", str(output_path), "--json", criterion="AC-WTC-002",
    )

    assert result.returncode != 0
    assert not output_path.exists()


def test_ac_wtc_004_dryrun_announces_missing_owner_map_review(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-004: rehearsal cannot use a manifest without reviewed ownership."""
    manifest_path, manifest, owner_path, baseline_path = generate_manifest(
        fixture_repo, evidence_dir, target=1
    )
    manifest_hash = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    issued_at, expires_at = now_window()
    approval_path = evidence_dir / "approval.json"
    write_json(
        approval_path,
        {
            "schema": "fathomdb-worktree-approval-attestation/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "reviewer": "fixture-reviewer",
            "decision": "approved",
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    archive_dir = fixture_repo.parent / "archive"
    archive_dir.mkdir(mode=0o700)
    before = repo_fingerprint(fixture_repo)

    result = run_tool(
        fixture_repo,
        "dryrun",
        "--repo", str(fixture_repo), "--manifest", str(manifest_path),
        "--owner-map", str(owner_path), "--approval-attestation", str(approval_path),
        "--baseline-attestation", str(baseline_path), "--archive-dir", str(archive_dir),
        "--evidence-dir", str(evidence_dir), "--json", criterion="AC-WTC-004",
    )

    assert result.returncode == 3
    assert json.loads(result.stdout)["result"] == "owner_map_review_required"
    assert repo_fingerprint(fixture_repo) == before
    assert not (evidence_dir / f"dryrun-{manifest_hash[:16]}.json").exists()


def test_ac_wtc_003_consolidate_rejects_unapproved_manifest_without_mutation(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-003/005: a filename is never authority to remove a worktree."""
    manifest_path = evidence_dir / "candidate.json"
    write_json(manifest_path, {"schema": "fathomdb-worktree-consolidator/v1"})
    before = repo_fingerprint(fixture_repo)

    result = run_tool(
        fixture_repo,
        "consolidate",
        "--repo",
        str(fixture_repo),
        "--manifest",
        str(manifest_path),
        "--evidence-dir",
        str(evidence_dir),
        criterion="AC-WTC-003",
    )

    assert result.returncode != 0
    assert repo_fingerprint(fixture_repo) == before


def test_ac_wtc_004_dryrun_never_mutates_git_state(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-004: dryrun is a rehearsal, not a cleanup command."""
    before = repo_fingerprint(fixture_repo)
    result = run_tool(
        fixture_repo,
        "dryrun",
        "--repo",
        str(fixture_repo),
        "--manifest",
        str(evidence_dir / "missing-manifest.json"),
        "--evidence-dir",
        str(evidence_dir),
        "--archive-dir",
        str(evidence_dir),
        criterion="AC-WTC-004",
    )
    assert result.returncode != 0
    assert repo_fingerprint(fixture_repo) == before


def test_ac_wtc_007_source_never_requests_forbidden_git_operations():
    """AC-WTC-007: the implementation has no remote or force-cleanup escape hatch."""
    tool = require_tool("AC-WTC-007")
    source = tool.read_text(encoding="utf-8")
    forbidden = (
        'git(repo.root, "push"',
        'git(repo.root, "worktree", "remove", "--force"',
        'git(repo.root, "reset"',
        'git(repo.root, "clean"',
    )
    assert not [token for token in forbidden if token in source]


def test_ac_wtc_005_consolidate_help_requires_full_preflight_contract(
    fixture_repo: Path,
):
    """AC-WTC-005: the destructive mode exposes every required gate."""
    result = run_tool(fixture_repo, "consolidate", "--help", criterion="AC-WTC-005")
    assert result.returncode == 0, result.stderr
    for option in (
        "--manifest",
        "--owner-map",
        "--approval-attestation",
        "--baseline-attestation",
        "--dryrun-receipt",
        "--freeze-attestation",
        "--archive-dir",
        "--evidence-dir",
        "--confirm-manifest-sha256",
        "--confirm",
    ):
        assert option in result.stdout


def test_ac_wtc_006_preflight_failure_creates_no_bundle(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-006: preservation is never attempted before all gates pass."""
    archive_dir = fixture_repo.parent / "archive"
    archive_dir.mkdir(mode=0o700)
    before = sorted(archive_dir.iterdir())
    result = run_tool(
        fixture_repo,
        "consolidate",
        "--repo",
        str(fixture_repo),
        "--manifest",
        str(evidence_dir / "missing.json"),
        "--evidence-dir",
        str(evidence_dir),
        "--archive-dir",
        str(archive_dir),
        criterion="AC-WTC-006",
    )
    assert result.returncode != 0
    assert sorted(archive_dir.iterdir()) == before


def test_ac_wtc_008_source_uses_non_force_worktree_removal_only():
    """AC-WTC-008: source makes the freeze assumption visible in the command."""
    tool = require_tool("AC-WTC-008")
    source = tool.read_text(encoding="utf-8")
    assert "worktree" in source and "remove" in source
    assert 'git(repo.root, "worktree", "remove", "--force"' not in source


def test_ac_wtc_009_source_never_mentions_remote_mutation_commands():
    """AC-WTC-009: end-state verification cannot hide a remote write path."""
    tool = require_tool("AC-WTC-009")
    source = tool.read_text(encoding="utf-8")
    assert 'git(repo.root, "push"' not in source
    assert 'git(repo.root, "branch", "-r", "-d"' not in source


def test_ac_wtc_008_partial_receipt_fallback_retries_a_colliding_name(
    evidence_dir: Path, monkeypatch: pytest.MonkeyPatch
):
    """AC-WTC-008: a failed final receipt still leaves an honest partial record."""
    module = load_tool_module()
    original = module.durable_write
    calls = 0

    def collide_once(path: Path, value: dict[str, Any]) -> None:
        nonlocal calls
        calls += 1
        if calls == 1:
            raise OSError("simulated storage failure")
        original(path, value)

    monkeypatch.setattr(module, "durable_write", collide_once)
    path = module.write_partial_fallback(
        evidence_dir,
        {"manifest_sha256": "a" * 64, "result": "success"},
        RuntimeError("simulated final receipt failure"),
    )

    receipt = json.loads(path.read_text(encoding="utf-8"))
    assert calls == 2
    assert receipt["result"] == "partial"
    assert receipt["failure"] == "simulated final receipt failure"


def test_ac_wtc_008_post_action_final_receipt_failure_writes_partial_evidence(
    fixture_repo: Path, evidence_dir: Path, monkeypatch: pytest.MonkeyPatch
):
    """AC-WTC-008: an I/O failure after retirement yields a linked partial receipt."""
    manifest_path, manifest, owner_path, baseline_path = generate_manifest(fixture_repo, evidence_dir, target=1)
    manifest_hash = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    issued_at, expires_at = now_window(900)
    approval_path = evidence_dir / "approval.json"
    write_json(
        approval_path,
        {
            "schema": "fathomdb-worktree-approval-attestation/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "reviewer": "fixture-reviewer",
            "decision": "approved",
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    archive_dir = fixture_repo.parent / "archive"
    archive_dir.mkdir(mode=0o700)
    dryrun = run_tool(
        fixture_repo,
        "dryrun",
        "--repo", str(fixture_repo), "--manifest", str(manifest_path),
        "--owner-map", str(owner_path), "--approval-attestation", str(approval_path),
        "--owner-map-review-attestation", str(evidence_dir / "owner-map-review.json"),
        "--baseline-attestation", str(baseline_path), "--archive-dir", str(archive_dir),
        "--evidence-dir", str(evidence_dir), criterion="AC-WTC-004",
    )
    assert dryrun.returncode == 0, dryrun.stderr
    dryrun_path = evidence_dir / f"dryrun-{manifest_hash[:16]}.json"
    issued_at, expires_at = now_window(900)
    freeze_path = evidence_dir / "freeze.json"
    write_json(
        freeze_path,
        {
            "schema": "fathomdb-worktree-freeze-attestation/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "dryrun_receipt_sha256": hashlib.sha256(dryrun_path.read_bytes()).hexdigest(),
            "snapshot_id": manifest["snapshot_id"],
            "operator": "fixture-operator",
            "writers": [],
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    module = load_tool_module()
    original = module.durable_write
    final_path = evidence_dir / f"execution-{manifest_hash[:16]}.json"

    def fail_final(path: Path, value: dict[str, Any]) -> None:
        if path == final_path:
            raise OSError("simulated final receipt disk failure")
        original(path, value)

    monkeypatch.setattr(module, "durable_write", fail_final)
    command = SimpleNamespace(
        repo=str(fixture_repo), manifest=str(manifest_path), owner_map=str(owner_path),
        owner_map_review_attestation=str(evidence_dir / "owner-map-review.json"),
        approval_attestation=str(approval_path), baseline_attestation=str(baseline_path),
        dryrun_receipt=str(dryrun_path), freeze_attestation=str(freeze_path),
        archive_dir=str(archive_dir), evidence_dir=str(evidence_dir),
        confirm_manifest_sha256=manifest_hash, confirm=f"CONSOLIDATE {manifest['manifest_id']}",
    )

    with pytest.raises(module.PartialBatch, match="execution receipt persistence failed"):
        module.command_consolidate(command)

    assert not (fixture_repo.parent / "linked").exists()
    preservation = evidence_dir / f"preservation-{manifest_hash[:16]}.json"
    partial_paths = list(evidence_dir.glob(f"partial-{manifest_hash[:16]}-*.json"))
    assert preservation.is_file()
    assert len(partial_paths) == 1
    partial = json.loads(partial_paths[0].read_text(encoding="utf-8"))
    assert partial["preservation_receipt_sha256"] == hashlib.sha256(preservation.read_bytes()).hexdigest()
    assert len(partial["completed_actions"]) == 1


def test_ac_wtc_002_clean_unmerged_worktree_is_not_retirement_candidate(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-002: a clean unmerged branch remains unresolved, never disposable."""
    linked = fixture_repo.parent / "linked"
    (linked / "unmerged.txt").write_text("must retain\n", encoding="utf-8")
    git(linked, "add", "unmerged.txt")
    git(linked, "commit", "-m", "unmerged", "--quiet")

    _, snapshot, _ = audit_snapshot(fixture_repo, evidence_dir)
    record = next(item for item in snapshot["worktrees"] if item["path"] == str(linked.resolve()))
    assert record["classification"] in {"unresolved", "integration-required"}


def test_ac_wtc_002_manifest_removes_only_required_surplus(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-002: planning for two worktrees does not retire every candidate."""
    third = fixture_repo.parent / "third"
    git(fixture_repo, "worktree", "add", "-b", "also-retirable", str(third))

    manifest_path, manifest, _, _ = generate_manifest(fixture_repo, evidence_dir, target=2)
    assert manifest_path.is_file()
    assert len(manifest["entries"]) == 1
    assert manifest["goal"]["source"] == "declared"


def test_ac_wtc_002_manifest_blocks_when_proven_candidates_cannot_meet_goal(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-002: a numerical target is blocked when safe candidates run out."""
    linked = fixture_repo.parent / "linked"
    (linked / "unmerged.txt").write_text("retain\n", encoding="utf-8")
    git(linked, "add", "unmerged.txt")
    git(linked, "commit", "-m", "unmerged", "--quiet")
    snapshot_path, snapshot, map_path = audit_snapshot(fixture_repo, evidence_dir)
    policy_path, baseline_path = policy_and_baseline(evidence_dir, snapshot, target_range=[1, 2])
    review_path = owner_map_review(evidence_dir, snapshot, map_path)
    result = run_tool(
        fixture_repo,
        "manifest",
        "--repo",
        str(fixture_repo),
        "--audit",
        str(snapshot_path),
        "--owner-map",
        str(map_path),
        "--owner-map-review-attestation",
        str(review_path),
        "--policy",
        str(policy_path),
        "--baseline-attestation",
        str(baseline_path),
        "--evidence-dir",
        str(evidence_dir),
        "--target-worktrees",
        "1",
        "--json",
        criterion="AC-WTC-002",
    )
    assert result.returncode == 3
    assert "insufficient proven retirement candidates" in result.stdout


def test_ac_wtc_002_manifest_rejects_a_ref_as_a_theme_worktree_target(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-002: themes retain protected worktrees, never bare branch names."""
    snapshot_path, snapshot, map_path = audit_snapshot(fixture_repo, evidence_dir)
    policy_path, baseline_path = policy_and_baseline(evidence_dir, snapshot, target_range=[1, 8])
    review_path = owner_map_review(evidence_dir, snapshot, map_path)
    policy = json.loads(policy_path.read_text(encoding="utf-8"))
    policy["active_themes"] = ["campaign"]
    policy["theme_targets"] = {"campaign": "refs/heads/main"}
    write_json(policy_path, policy)

    result = run_tool(
        fixture_repo,
        "manifest",
        "--repo",
        str(fixture_repo),
        "--audit",
        str(snapshot_path),
        "--owner-map",
        str(map_path),
        "--owner-map-review-attestation",
        str(review_path),
        "--policy",
        str(policy_path),
        "--baseline-attestation",
        str(baseline_path),
        "--evidence-dir",
        str(evidence_dir),
        "--target-worktrees",
        "2",
        "--json",
        criterion="AC-WTC-002",
    )

    assert result.returncode != 0
    assert "lacks a protected/integration retained target" in result.stderr


def test_ac_wtc_004_dryrun_rejects_forged_unmerged_retirement_entry(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-004: claimed manifest classification cannot override the audit."""
    linked = fixture_repo.parent / "linked"
    (linked / "unmerged.txt").write_text("retain\n", encoding="utf-8")
    git(linked, "add", "unmerged.txt")
    git(linked, "commit", "-m", "unmerged", "--quiet")
    manifest_path, manifest, owner_path, baseline_path = generate_manifest(
        fixture_repo, evidence_dir, target=2
    )
    tip = git(linked, "rev-parse", "HEAD").strip()
    manifest["entries"] = [
        {
            "kind": "worktree",
            "target": str(linked.resolve()),
            "classification": "merged-retirable",
            "owner": {
                "value": "none",
                "release_role": "none",
                "evidence_sha256": manifest["owner_map_sha256"],
            },
            "action": "remove_worktree",
            "witness": {
                "tip": tip,
                "clean": True,
                "unused_by_retained_worktree": True,
                "recovery_requirement": "execution_bundle",
            },
        }
    ]
    payload = {
        key: value for key, value in manifest.items() if key not in {"manifest_id", "plan_sha256"}
    }
    plan_hash = hashlib.sha256(canonical_json(payload)).hexdigest()
    manifest["plan_sha256"] = plan_hash
    manifest["manifest_id"] = f"wtc-{manifest['snapshot_id'][:8]}-{plan_hash[:8]}"
    write_json(manifest_path, manifest)
    manifest_hash = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    issued_at, expires_at = now_window()
    approval_path = evidence_dir / "approval.json"
    write_json(
        approval_path,
        {
            "schema": "fathomdb-worktree-approval-attestation/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "reviewer": "fixture-reviewer",
            "decision": "approved",
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    archive_dir = fixture_repo.parent / "archive"
    archive_dir.mkdir(mode=0o700)
    result = run_tool(
        fixture_repo,
        "dryrun",
        "--repo",
        str(fixture_repo),
        "--manifest",
        str(manifest_path),
        "--owner-map",
        str(owner_path),
        "--owner-map-review-attestation",
        str(evidence_dir / "owner-map-review.json"),
        "--approval-attestation",
        str(approval_path),
        "--baseline-attestation",
        str(baseline_path),
        "--archive-dir",
        str(archive_dir),
        "--evidence-dir",
        str(evidence_dir),
        criterion="AC-WTC-004",
    )
    assert result.returncode != 0
    assert "not merged-retirable" in result.stderr


def test_ac_wtc_003_to_009_valid_chain_rehearses_and_retires_exact_worktree(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-003..009: a valid complete chain preserves before exact retirement."""
    git(fixture_repo, "branch", "stale-local-head", "origin/main")
    manifest_path, manifest, owner_path, baseline_path = generate_manifest(
        fixture_repo, evidence_dir, target=1, retire_local_heads=True
    )
    assert {(entry["kind"], entry["target"]) for entry in manifest["entries"]} == {
        ("worktree", str((fixture_repo.parent / "linked").resolve())),
        ("branch", "refs/heads/stale-local-head"),
    }
    manifest_hash = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    issued_at, expires_at = now_window(900)
    approval_path = evidence_dir / "approval.json"
    write_json(
        approval_path,
        {
            "schema": "fathomdb-worktree-approval-attestation/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "reviewer": "fixture-reviewer",
            "decision": "approved",
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )
    archive_dir = fixture_repo.parent / "archive"
    archive_dir.mkdir(mode=0o700)
    dryrun = run_tool(
        fixture_repo,
        "dryrun",
        "--repo",
        str(fixture_repo),
        "--manifest",
        str(manifest_path),
        "--owner-map",
        str(owner_path),
        "--owner-map-review-attestation",
        str(evidence_dir / "owner-map-review.json"),
        "--approval-attestation",
        str(approval_path),
        "--baseline-attestation",
        str(baseline_path),
        "--archive-dir",
        str(archive_dir),
        "--evidence-dir",
        str(evidence_dir),
        "--json",
        criterion="AC-WTC-004",
    )
    assert dryrun.returncode == 0, dryrun.stderr
    receipt_path = evidence_dir / f"dryrun-{manifest_hash[:16]}.json"
    assert receipt_path.is_file()
    review_path = evidence_dir / "owner-map-review.json"
    review_bytes = review_path.read_bytes()
    assert manifest["owner_map_review_sha256"] == hashlib.sha256(review_bytes).hexdigest()
    assert json.loads(receipt_path.read_text(encoding="utf-8"))["owner_map_review_sha256"] == hashlib.sha256(review_bytes).hexdigest()
    receipt_hash = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
    issued_at, expires_at = now_window(900)
    freeze_path = evidence_dir / "freeze.json"
    write_json(
        freeze_path,
        {
            "schema": "fathomdb-worktree-freeze-attestation/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "dryrun_receipt_sha256": receipt_hash,
            "snapshot_id": manifest["snapshot_id"],
            "operator": "fixture-operator",
            "writers": [],
            "issued_at": issued_at,
            "expires_at": expires_at,
        },
    )

    changed_review = json.loads(review_bytes.decode("utf-8"))
    changed_review["reviewer"] = "replacement-reviewer"
    write_json(review_path, changed_review)
    replaced = run_tool(
        fixture_repo,
        "consolidate",
        "--repo", str(fixture_repo), "--manifest", str(manifest_path),
        "--owner-map", str(owner_path), "--owner-map-review-attestation", str(review_path),
        "--approval-attestation", str(approval_path), "--baseline-attestation", str(baseline_path),
        "--dryrun-receipt", str(receipt_path), "--freeze-attestation", str(freeze_path),
        "--archive-dir", str(archive_dir), "--evidence-dir", str(evidence_dir),
        "--confirm-manifest-sha256", manifest_hash,
        "--confirm", f"CONSOLIDATE {manifest['manifest_id']}",
        criterion="AC-WTC-005",
    )
    assert replaced.returncode != 0
    assert "owner-map review attestation hash mismatch" in replaced.stderr
    assert (fixture_repo.parent / "linked").is_dir()
    assert not list(archive_dir.iterdir())
    review_path.write_bytes(review_bytes)

    execution_path = evidence_dir / f"execution-{manifest_hash[:16]}.json"
    execution_path.write_bytes(b"occupied")
    collision = run_tool(
        fixture_repo,
        "consolidate",
        "--repo",
        str(fixture_repo),
        "--manifest",
        str(manifest_path),
        "--owner-map",
        str(owner_path),
        "--owner-map-review-attestation",
        str(evidence_dir / "owner-map-review.json"),
        "--approval-attestation",
        str(approval_path),
        "--baseline-attestation",
        str(baseline_path),
        "--dryrun-receipt",
        str(receipt_path),
        "--freeze-attestation",
        str(freeze_path),
        "--archive-dir",
        str(archive_dir),
        "--evidence-dir",
        str(evidence_dir),
        "--confirm-manifest-sha256",
        manifest_hash,
        "--confirm",
        f"CONSOLIDATE {manifest['manifest_id']}",
        "--json",
        criterion="AC-WTC-006",
    )
    assert collision.returncode != 0
    assert "execution evidence path already exists" in collision.stderr
    assert (fixture_repo.parent / "linked").is_dir()
    assert not list(archive_dir.iterdir())
    execution_path.unlink()

    result = run_tool(
        fixture_repo,
        "consolidate",
        "--repo",
        str(fixture_repo),
        "--manifest",
        str(manifest_path),
        "--owner-map",
        str(owner_path),
        "--owner-map-review-attestation",
        str(evidence_dir / "owner-map-review.json"),
        "--approval-attestation",
        str(approval_path),
        "--baseline-attestation",
        str(baseline_path),
        "--dryrun-receipt",
        str(receipt_path),
        "--freeze-attestation",
        str(freeze_path),
        "--archive-dir",
        str(archive_dir),
        "--evidence-dir",
        str(evidence_dir),
        "--confirm-manifest-sha256",
        manifest_hash,
        "--confirm",
        f"CONSOLIDATE {manifest['manifest_id']}",
        "--json",
        criterion="AC-WTC-005",
    )
    assert result.returncode == 0, result.stderr
    assert not (fixture_repo.parent / "linked").exists()
    assert git(fixture_repo, "rev-parse", "--verify", "refs/heads/retirable").strip()
    assert (
        subprocess.run(
            ["git", "rev-parse", "--verify", "--quiet", "refs/heads/stale-local-head"],
            cwd=fixture_repo,
            check=False,
            capture_output=True,
            text=True,
        ).returncode
        != 0
    )
    assert (archive_dir / f"refs-before-wtc-{manifest['snapshot_id'][:8]}-{manifest['plan_sha256'][:8]}.bundle").is_file()
    assert execution_path.is_file()
    preservation = json.loads((evidence_dir / f"preservation-{manifest_hash[:16]}.json").read_text(encoding="utf-8"))
    assert preservation["verified_before_retirement"] is True
    assert preservation["bundle_inputs"]
    assert preservation["bundle_verify"]


def test_bundle_accepts_recovery_commit_reachable_from_advertised_head(
    fixture_repo: Path, tmp_path: Path
):
    """A recovery commit may be covered without appearing in ``list-heads``."""
    module = load_tool_module()
    linked = fixture_repo.parent / "linked"
    git(fixture_repo, "worktree", "remove", str(linked))
    git(fixture_repo, "branch", "-D", "retirable")
    recovery_tip = git(fixture_repo, "rev-parse", "HEAD").strip()
    (fixture_repo / "README.md").write_text("fixture successor\n", encoding="utf-8")
    git(fixture_repo, "add", "README.md")
    git(fixture_repo, "commit", "-m", "successor", "--quiet")
    current_tip = git(fixture_repo, "rev-parse", "HEAD").strip()
    archive = tmp_path / "archive"
    archive.mkdir(mode=0o700)
    manifest = {
        "snapshot_id": "a" * 64,
        "plan_sha256": "b" * 64,
        "preservation": {
            "bundle_name_algorithm": "wtc-bundle-v1",
            "required_tips": [current_tip],
            "reflog_candidates": [recovery_tip],
        },
    }

    bundle, _, covered, _, _ = module.publish_bundle(
        module.repository(fixture_repo), archive, manifest
    )

    assert not list(archive.glob(".wtc-coverage-*"))
    advertised = {
        line.split()[0]
        for line in git(fixture_repo, "bundle", "list-heads", str(bundle)).splitlines()
    }
    assert recovery_tip not in advertised
    assert set(covered) == {current_tip, recovery_tip}
    recovery_repo = tmp_path / "recovery"
    recovery_repo.mkdir()
    git(recovery_repo, "init", "--quiet")
    git(recovery_repo, "bundle", "unbundle", str(bundle))
    git(recovery_repo, "cat-file", "-e", f"{recovery_tip}^{{commit}}")


@pytest.mark.parametrize(
    ("proof_type", "relation"),
    [
        ("stable_patch_coverage", "same-tip"),
        ("retained_local_ref", "same-tip"),
        ("retained_local_ref", "ancestor"),
        ("remote_tracking_ref", "same-tip"),
    ],
)
def test_ac_wtc_p01_to_p03_accepts_each_exact_proof_relation(
    fixture_repo: Path, evidence_dir: Path, proof_type: str, relation: str
):
    """AC-WTC-P01..P03: each closed live relation produces one bound action."""
    proof, retained, _ = make_proof_scenario(
        fixture_repo, proof_type, relation=relation
    )

    context = proof_manifest_context(
        fixture_repo, evidence_dir, [proof], retained_refs=retained
    )

    assert context["result"].returncode == 0, context["result"].stderr
    manifest = json.loads(context["result"].stdout)
    entry = next(item for item in manifest["entries"] if item["target"] == proof["target"])
    assert entry["classification"] == "proof-retirable"
    assert entry["witness"]["proof_type"] == proof_type
    assert entry["witness"]["proof_entry_sha256"] == hashlib.sha256(
        canonical_json(proof)
    ).hexdigest()
    assert manifest["owner_map_review_sha256"] == hashlib.sha256(
        context["owner_review_path"].read_bytes()
    ).hexdigest()


@pytest.mark.parametrize("fault", ["partial", "wrong-patch"])
def test_ac_wtc_p01_rejects_inexact_stable_patch_evidence(
    fixture_repo: Path, evidence_dir: Path, fault: str
):
    """AC-WTC-P01: stable-patch evidence is complete and recomputed."""
    initial = git(fixture_repo, "rev-list", "--max-parents=0", "HEAD").strip()
    baseline_commits = []
    for filename in ("one.txt", "two.txt"):
        (fixture_repo / filename).write_text(f"{filename}\n", encoding="utf-8")
        git(fixture_repo, "add", filename)
        git(fixture_repo, "commit", "-m", f"baseline-{filename}", "--quiet")
        baseline_commits.append(git(fixture_repo, "rev-parse", "HEAD").strip())
    git(fixture_repo, "update-ref", "refs/remotes/origin/main", baseline_commits[-1])
    commits = commit_on_new_branch(
        fixture_repo,
        "proof-target",
        initial,
        [("one.txt", "one.txt\n"), ("two.txt", "two.txt\n")],
    )
    proof = common_proof(
        "refs/heads/proof-target", commits[-1], "stable_patch_coverage"
    )
    records = [
        {
            "commit": commit,
            "stable_patch_id": stable_patch_id(fixture_repo, commit),
            "baseline_matches": [baseline],
        }
        for commit, baseline in zip(commits, baseline_commits, strict=True)
    ]
    if fault == "partial":
        records.pop()
    else:
        records[0]["stable_patch_id"] = "0" * 40
    proof["source_commits"] = records

    context = proof_manifest_context(fixture_repo, evidence_dir, [proof])

    assert context["result"].returncode != 0
    assert not context["manifest_path"].exists()


@pytest.mark.parametrize("fault", ["empty", "merge"])
def test_ac_wtc_p01_rejects_empty_and_merge_stable_patch_commits(
    fixture_repo: Path, evidence_dir: Path, fault: str
):
    """AC-WTC-P01: empty and merge commits are outside the proof domain."""
    initial = git(fixture_repo, "rev-list", "--max-parents=0", "HEAD").strip()
    if fault == "empty":
        path = fixture_repo.parent / "proof-target-worktree"
        git(fixture_repo, "worktree", "add", "-b", "proof-target", str(path), initial)
        git(path, "commit", "--allow-empty", "-m", "empty", "--quiet")
        tip = git(path, "rev-parse", "HEAD").strip()
        git(fixture_repo, "worktree", "remove", str(path))
        source_commits = [{"commit": tip, "stable_patch_id": "0" * 40, "baseline_matches": []}]
    else:
        baseline = []
        for filename in ("left.txt", "right.txt"):
            (fixture_repo / filename).write_text(f"{filename}\n", encoding="utf-8")
            git(fixture_repo, "add", filename)
            git(fixture_repo, "commit", "-m", f"baseline-{filename}", "--quiet")
            baseline.append(git(fixture_repo, "rev-parse", "HEAD").strip())
        git(fixture_repo, "update-ref", "refs/remotes/origin/main", baseline[-1])
        [left] = commit_on_new_branch(
            fixture_repo, "proof-target", initial, [("left.txt", "left.txt\n")]
        )
        [right] = commit_on_new_branch(
            fixture_repo, "proof-side", initial, [("right.txt", "right.txt\n")]
        )
        path = fixture_repo.parent / "proof-merge-worktree"
        git(fixture_repo, "worktree", "add", str(path), "proof-target")
        git(path, "merge", "--no-ff", "proof-side", "-m", "unique merge", "--quiet")
        tip = git(path, "rev-parse", "HEAD").strip()
        git(fixture_repo, "worktree", "remove", str(path))
        git(fixture_repo, "branch", "-D", "proof-side")
        source_commits = [
            {"commit": left, "stable_patch_id": stable_patch_id(fixture_repo, left), "baseline_matches": [baseline[0]]},
            {"commit": right, "stable_patch_id": stable_patch_id(fixture_repo, right), "baseline_matches": [baseline[1]]},
            {"commit": tip, "stable_patch_id": "0" * 40, "baseline_matches": []},
        ]
    proof = common_proof("refs/heads/proof-target", tip, "stable_patch_coverage")
    proof["source_commits"] = source_commits

    context = proof_manifest_context(fixture_repo, evidence_dir, [proof])

    assert context["result"].returncode != 0
    assert fault in context["result"].stderr.lower()


@pytest.mark.parametrize("fault", ["missing", "moved", "self", "unretained"])
def test_ac_wtc_p02_rejects_unsafe_retained_local_ref(
    fixture_repo: Path, evidence_dir: Path, fault: str
):
    """AC-WTC-P02: a retained anchor must remain exact, distinct, and owned."""
    proof, retained, _ = make_proof_scenario(fixture_repo, "retained_local_ref")
    if fault == "missing":
        proof["retained_ref"] = "refs/heads/missing-retained"
    elif fault == "moved":
        proof["retained_tip"] = git(fixture_repo, "rev-parse", "origin/main").strip()
    elif fault == "self":
        proof["retained_ref"] = proof["target"]
        proof["retained_tip"] = proof["target_tip"]
    else:
        retained = ()

    context = proof_manifest_context(
        fixture_repo, evidence_dir, [proof], retained_refs=retained
    )

    assert context["result"].returncode != 0
    assert not context["manifest_path"].exists()


def test_ac_wtc_p02_rejects_retained_ref_that_is_also_a_proof_target(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-P02: the preservation anchor cannot occur in retirement targets."""
    proof, _, _ = make_proof_scenario(fixture_repo, "retained_local_ref")
    retained_tip = proof["retained_tip"]
    remote_ref = "refs/remotes/origin/proof-retained"
    git(fixture_repo, "update-ref", remote_ref, retained_tip)
    second = common_proof(
        "refs/heads/proof-retained", retained_tip, "remote_tracking_ref"
    )
    second.update({"remote_ref": remote_ref, "remote_tip": retained_tip})

    context = proof_manifest_context(fixture_repo, evidence_dir, [proof, second])

    assert context["result"].returncode != 0


@pytest.mark.parametrize("fault", ["local", "symbolic", "missing"])
def test_ac_wtc_p03_rejects_non_direct_remote_evidence(
    fixture_repo: Path, evidence_dir: Path, fault: str
):
    """AC-WTC-P03: remote evidence is exact, direct, and in refs/remotes."""
    proof, _, _ = make_proof_scenario(fixture_repo, "remote_tracking_ref")
    if fault == "local":
        proof["remote_ref"] = proof["target"]
    elif fault == "symbolic":
        git(fixture_repo, "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")
        proof["remote_ref"] = "refs/remotes/origin/HEAD"
    else:
        proof["remote_ref"] = "refs/remotes/origin/missing"

    context = proof_manifest_context(fixture_repo, evidence_dir, [proof])

    assert context["result"].returncode != 0


@pytest.mark.parametrize(
    ("fault", "kwargs"),
    [
        ("rejected", {"approval_decision": "rejected"}),
        ("expired", {"approval_expired": True}),
        ("same-author", {"reviewer": "fixture-semantic-owner"}),
        ("hash-mismatch", {"approval_hash": "0" * 64}),
    ],
)
def test_ac_wtc_p04_rejects_invalid_proof_approval(
    fixture_repo: Path, evidence_dir: Path, fault: str, kwargs: dict[str, Any]
):
    """AC-WTC-P04: semantic approval is independent, current, and hash-bound."""
    proof, _, _ = make_proof_scenario(fixture_repo, "remote_tracking_ref")

    context = proof_manifest_context(fixture_repo, evidence_dir, [proof], **kwargs)

    assert context["result"].returncode != 0, fault
    assert not context["manifest_path"].exists()


@pytest.mark.parametrize(
    ("identity_field", "identity"),
    [
        ("author", " leading"),
        ("author", "trailing "),
        ("author", "control\u0007"),
        ("author", "format\u200e"),
        ("reviewer", " leading"),
        ("reviewer", "trailing "),
        ("reviewer", "control\u0007"),
        ("reviewer", "format\u200e"),
    ],
)
def test_ac_wtc_p04_rejects_noncanonical_accountable_identity(
    fixture_repo: Path, evidence_dir: Path, identity_field: str, identity: str
):
    """AC-WTC-P04: accountable identities have one canonical display form."""
    proof, _, _ = make_proof_scenario(fixture_repo, "remote_tracking_ref")
    kwargs = {identity_field: identity}

    context = proof_manifest_context(fixture_repo, evidence_dir, [proof], **kwargs)

    assert context["result"].returncode != 0


@pytest.mark.parametrize("fault", ["schema", "type", "privacy-field"])
def test_ac_wtc_p04_rejects_unknown_or_payload_bearing_proof(
    fixture_repo: Path, evidence_dir: Path, fault: str
):
    """AC-WTC-P04/P07: closed metadata schemas reject generic semantic payloads."""
    proof, _, _ = make_proof_scenario(fixture_repo, "remote_tracking_ref")
    schema = "fathomdb-worktree-retirement-proofs/v1"
    if fault == "schema":
        schema = "fathomdb-worktree-retirement-proofs/unknown"
    elif fault == "type":
        proof["proof_type"] = "generic_archive"
    else:
        proof["prompt"] = "durable source payload is forbidden"

    context = proof_manifest_context(
        fixture_repo, evidence_dir, [proof], proof_schema=schema
    )

    assert context["result"].returncode != 0
    assert not context["manifest_path"].exists()


def test_ac_wtc_p04_rejects_proof_file_altered_after_approval(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-P04: later stages bind the exact independently reviewed bytes."""
    proof, _, _ = make_proof_scenario(fixture_repo, "remote_tracking_ref")
    context = proof_manifest_context(fixture_repo, evidence_dir, [proof])
    assert context["result"].returncode == 0, context["result"].stderr
    value = json.loads(context["proof_path"].read_text(encoding="utf-8"))
    value["proofs"][0]["evidence_id"] = "fixture-decision:altered"
    write_json(context["proof_path"], value)

    context = approve_and_dryrun(fixture_repo, evidence_dir, context)

    assert context["dryrun_result"].returncode != 0
    assert not context["dryrun_path"].exists()


@pytest.mark.parametrize("fault", ["mismatch", "expired"])
def test_ac_wtc_p04_dryrun_preserves_owner_map_review_gate(
    fixture_repo: Path, evidence_dir: Path, fault: str
):
    """AC-WTC-P04: proof inputs cannot bypass owner review during rehearsal."""
    proof, _, _ = make_proof_scenario(fixture_repo, "remote_tracking_ref")
    context = proof_manifest_context(fixture_repo, evidence_dir, [proof])
    assert context["result"].returncode == 0, context["result"].stderr
    review = json.loads(context["owner_review_path"].read_text(encoding="utf-8"))
    if fault == "mismatch":
        review["reviewer"] = "replacement-owner-reviewer"
    else:
        review["expires_at"] = "2000-01-01T00:00:00Z"
    write_json(context["owner_review_path"], review)

    context = approve_and_dryrun(fixture_repo, evidence_dir, context)

    assert context["dryrun_result"].returncode != 0
    assert not context["dryrun_path"].exists()


@pytest.mark.parametrize("fault", ["missing", "mismatch", "expired"])
def test_ac_wtc_p04_consolidate_preserves_owner_map_review_gate(
    fixture_repo: Path, evidence_dir: Path, fault: str
):
    """AC-WTC-P04: execution rechecks the exact current owner-map review."""
    proof, _, _ = make_proof_scenario(fixture_repo, "remote_tracking_ref")
    context = proof_manifest_context(fixture_repo, evidence_dir, [proof])
    assert context["result"].returncode == 0, context["result"].stderr
    context = approve_and_dryrun(fixture_repo, evidence_dir, context)
    assert context["dryrun_result"].returncode == 0, context["dryrun_result"].stderr
    if fault != "missing":
        review = json.loads(context["owner_review_path"].read_text(encoding="utf-8"))
        if fault == "mismatch":
            review["reviewer"] = "replacement-owner-reviewer"
        else:
            review["expires_at"] = "2000-01-01T00:00:00Z"
        write_json(context["owner_review_path"], review)
    if fault == "missing":
        context["owner_review_path"] = evidence_dir / "missing-owner-review.json"

    result = consolidate_context(fixture_repo, evidence_dir, context)

    assert result.returncode != 0
    assert git(fixture_repo, "rev-parse", "--verify", "refs/heads/proof-target").strip()
    assert not list(context["archive_dir"].iterdir())


@pytest.mark.parametrize("drift", ["target", "retained", "baseline", "remote"])
def test_ac_wtc_p05_consolidate_rejects_every_proof_path_drift(
    fixture_repo: Path, evidence_dir: Path, drift: str
):
    """AC-WTC-P05: every live path is recomputed after a successful dry run."""
    proof_type = "retained_local_ref" if drift == "retained" else "remote_tracking_ref"
    proof, retained, remote_ref = make_proof_scenario(fixture_repo, proof_type)
    context = proof_manifest_context(
        fixture_repo, evidence_dir, [proof], retained_refs=retained
    )
    assert context["result"].returncode == 0, context["result"].stderr
    context = approve_and_dryrun(fixture_repo, evidence_dir, context)
    assert context["dryrun_result"].returncode == 0, context["dryrun_result"].stderr
    if drift == "target":
        git(fixture_repo, "update-ref", "refs/heads/proof-target", "origin/main")
    elif drift == "retained":
        git(fixture_repo, "update-ref", "refs/heads/proof-retained", "origin/main")
    elif drift == "remote":
        assert remote_ref
        git(fixture_repo, "update-ref", remote_ref, "origin/main")
    else:
        (fixture_repo / "baseline-drift.txt").write_text("drift\n", encoding="utf-8")
        git(fixture_repo, "add", "baseline-drift.txt")
        git(fixture_repo, "commit", "-m", "baseline drift", "--quiet")
        git(fixture_repo, "update-ref", "refs/remotes/origin/main", "HEAD")

    result = consolidate_context(fixture_repo, evidence_dir, context)

    assert result.returncode != 0
    assert not list(context["archive_dir"].iterdir())


@pytest.mark.parametrize(
    "proof_type",
    ["stable_patch_coverage", "retained_local_ref", "remote_tracking_ref"],
)
def test_ac_wtc_p06_end_to_end_proof_retirement_is_recoverable(
    fixture_repo: Path, evidence_dir: Path, tmp_path: Path, proof_type: str
):
    """AC-WTC-P06: each proof path bundles before expected-old-SHA deletion."""
    proof, retained, _ = make_proof_scenario(fixture_repo, proof_type)
    target_tip = proof["target_tip"]
    remote_before = git(
        fixture_repo, "for-each-ref", "--format=%(refname) %(objectname)", "refs/remotes"
    )
    context = proof_manifest_context(
        fixture_repo, evidence_dir, [proof], retained_refs=retained
    )
    assert context["result"].returncode == 0, context["result"].stderr
    context = approve_and_dryrun(fixture_repo, evidence_dir, context)
    assert context["dryrun_result"].returncode == 0, context["dryrun_result"].stderr

    result = consolidate_context(fixture_repo, evidence_dir, context)

    assert result.returncode == 0, result.stderr
    assert subprocess.run(
        ["git", "rev-parse", "--verify", "--quiet", "refs/heads/proof-target"],
        cwd=fixture_repo,
        check=False,
        capture_output=True,
    ).returncode != 0
    assert git(
        fixture_repo, "for-each-ref", "--format=%(refname) %(objectname)", "refs/remotes"
    ) == remote_before
    bundles = list(context["archive_dir"].glob("*.bundle"))
    assert len(bundles) == 1
    recovery = tmp_path / f"recovery-{proof_type}"
    recovery.mkdir()
    git(recovery, "init", "--bare", "--quiet")
    git(recovery, "bundle", "unbundle", str(bundles[0]))
    git(recovery, "cat-file", "-e", f"{target_tip}^{{commit}}")
    execution = list(evidence_dir.glob("execution-*.json"))
    assert len(execution) == 1
    completed = json.loads(execution[0].read_text(encoding="utf-8"))["completed_actions"]
    target_action = next(item for item in completed if item["target"] == "refs/heads/proof-target")
    assert target_action["witness"]["tip"] == target_tip
    assert target_action["witness"]["recovery_requirement"] == "execution_bundle"


def test_ac_wtc_p06_direct_ancestor_regression_needs_no_proof_files(
    fixture_repo: Path, evidence_dir: Path
):
    """AC-WTC-P06: reviewed direct ancestry retains the original protocol."""
    git(fixture_repo, "branch", "legacy-direct-ancestor", "origin/main")

    _, manifest, _, _ = generate_manifest(
        fixture_repo, evidence_dir, target=2, retire_local_heads=True
    )

    entry = next(
        item
        for item in manifest["entries"]
        if item["target"] == "refs/heads/legacy-direct-ancestor"
    )
    assert entry["classification"] == "merged-retirable"
    assert set(entry["witness"]) == {
        "tip",
        "clean",
        "unused_by_retained_worktree",
        "recovery_requirement",
    }
    assert manifest["retirement_proofs"] is None


def status_fixture(
    repo: Path, evidence_dir: Path, *, result: str = "success", progress: bool = True
) -> tuple[Path, dict[str, Any], str, Path, Path]:
    """Create manifest-bound receipt fixtures without performing a retirement."""
    manifest_path, manifest, _, _ = generate_manifest(repo, evidence_dir, target=1)
    manifest_hash = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
    prefix = manifest_hash[:16]
    covered_tips = sorted(
        set(manifest["preservation"]["required_tips"])
        | set(manifest["preservation"]["reflog_candidates"])
    )
    preservation_path = evidence_dir / f"preservation-{prefix}.json"
    preservation = {
        "schema": "fathomdb-worktree-preservation-receipt/v1",
        "repository": manifest["repository"],
        "manifest_sha256": manifest_hash,
        "bundle_path": str(evidence_dir / "fixture.bundle"),
        "bundle_sha256": "a" * 64,
        "covered_tips": covered_tips,
        "bundle_inputs": ["refs/heads/main", "refs/heads/retirable"],
        "bundle_verify": "fixture standalone verification",
        "verified_before_retirement": True,
        "issued_at": "2026-08-15T00:00:00Z",
    }
    preservation_hash = write_json(preservation_path, preservation)
    actions = manifest["entries"]
    if progress:
        write_json(
            evidence_dir / f"progress-{prefix}-0001.json",
            {
                "schema": "fathomdb-worktree-progress-receipt/v1",
                "repository": manifest["repository"],
                "manifest_sha256": manifest_hash,
                "preservation_receipt_sha256": preservation_hash,
                "completed_actions": actions,
                "issued_at": "2026-08-15T00:01:00Z",
            },
        )
    final_path = evidence_dir / f"execution-{prefix}.json"
    write_json(
        final_path,
        {
            "schema": "fathomdb-worktree-execution-receipt/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "preservation_receipt_sha256": preservation_hash,
            "bundle_path": preservation["bundle_path"],
            "bundle_sha256": preservation["bundle_sha256"],
            "covered_tips": covered_tips,
            "completed_actions": actions,
            "post_snapshot_id": "b" * 64 if result == "success" else None,
            "result": result,
            "issued_at": "2026-08-15T00:02:00Z",
        },
    )
    return manifest_path, manifest, manifest_hash, preservation_path, final_path


def run_status(repo: Path, manifest_path: Path, evidence_dir: Path) -> subprocess.CompletedProcess[str]:
    """Run the read-only execution-state observer through its public CLI."""
    return run_tool(
        repo,
        "status",
        "--repo",
        str(repo),
        "--manifest",
        str(manifest_path),
        "--evidence-dir",
        str(evidence_dir),
        "--json",
        criterion="AC-WTC-S01",
    )


def test_ac_wtc_s01_status_treats_any_lock_as_executing_without_mutation(
    fixture_repo: Path, evidence_dir: Path
):
    """A PID-looking lock is opaque; status must not probe or clear it."""
    manifest_path, _, _, _, _ = status_fixture(fixture_repo, evidence_dir)
    common_raw = Path(git(fixture_repo, "rev-parse", "--git-common-dir").strip())
    common = common_raw if common_raw.is_absolute() else (fixture_repo / common_raw).resolve()
    lock = common / "worktree-consolidator.lock"
    lock.write_text("pid=secret-unobservable-process\n", encoding="utf-8")
    before = repo_fingerprint(fixture_repo)

    result = run_status(fixture_repo, manifest_path, evidence_dir)

    assert result.returncode == 0, result.stderr
    observed = json.loads(result.stdout)
    assert observed["state"] == "executing"
    assert observed["phase"] == "finalizing"
    assert "pid" not in result.stdout.lower()
    assert lock.exists()
    assert repo_fingerprint(fixture_repo) == before


def test_ac_wtc_s02_status_reports_completed_only_for_complete_success_chain(
    fixture_repo: Path, evidence_dir: Path
):
    """A complete, unlocked success chain is the sole completed state."""
    manifest_path, _, _, _, _ = status_fixture(fixture_repo, evidence_dir)

    result = run_status(fixture_repo, manifest_path, evidence_dir)

    assert result.returncode == 0, result.stderr
    observed = json.loads(result.stdout)
    assert observed == {
        "schema": "fathomdb-worktree-execution-status/v1",
        "state": "completed",
        "phase": "complete",
        "completed_actions": 1,
        "total_actions": 1,
        "operator_action": "preserve evidence; no further action is required",
    }


def test_ac_wtc_s03_fallback_partial_dominates_a_coexisting_success_receipt(
    fixture_repo: Path, evidence_dir: Path
):
    """A directory-fsync fallback blocks false completion and hides failure text."""
    manifest_path, manifest, manifest_hash, preservation_path, _ = status_fixture(
        fixture_repo, evidence_dir
    )
    preservation = json.loads(preservation_path.read_text(encoding="utf-8"))
    fallback = evidence_dir / f"partial-{manifest_hash[:16]}-0123456789abcdef.json"
    write_json(
        fallback,
        {
            "schema": "fathomdb-worktree-execution-receipt/v1",
            "repository": manifest["repository"],
            "manifest_sha256": manifest_hash,
            "preservation_receipt_sha256": hashlib.sha256(
                preservation_path.read_bytes()
            ).hexdigest(),
            "bundle_path": preservation["bundle_path"],
            "bundle_sha256": preservation["bundle_sha256"],
            "covered_tips": preservation["covered_tips"],
            "completed_actions": manifest["entries"],
            "post_snapshot_id": None,
            "result": "partial",
            "issued_at": "2026-08-15T00:03:00Z",
            "failure": "do not expose this diagnostic",
        },
    )

    result = run_status(fixture_repo, manifest_path, evidence_dir)

    assert result.returncode == 0, result.stderr
    observed = json.loads(result.stdout)
    assert observed["state"] == "recovery_required"
    assert observed["phase"] == "partial"
    assert "diagnostic" not in result.stdout


@pytest.mark.parametrize("fault", ["foreign-manifest", "unexpected-progress"])
def test_ac_wtc_s04_status_rejects_malformed_exact_namespace_evidence(
    fixture_repo: Path, evidence_dir: Path, fault: str
):
    """Foreign hashes and invalid receipt numbering cannot be silently ignored."""
    manifest_path, manifest, manifest_hash, _, _ = status_fixture(fixture_repo, evidence_dir)
    prefix = manifest_hash[:16]
    if fault == "foreign-manifest":
        path = evidence_dir / f"partial-{prefix}-0123456789abcdef.json"
        value = {
            "schema": "fathomdb-worktree-execution-receipt/v1",
            "repository": manifest["repository"],
            "manifest_sha256": "c" * 64,
            "preservation_receipt_sha256": "d" * 64,
            "bundle_path": str(evidence_dir / "foreign.bundle"),
            "bundle_sha256": "e" * 64,
            "covered_tips": [],
            "completed_actions": [],
            "post_snapshot_id": None,
            "result": "partial",
            "issued_at": "2026-08-15T00:03:00Z",
            "failure": "foreign",
        }
    else:
        path = evidence_dir / f"progress-{prefix}-0002.json"
        value = {"foreign": "progress"}
    write_json(path, value)
    before = repo_fingerprint(fixture_repo)

    result = run_status(fixture_repo, manifest_path, evidence_dir)

    assert result.returncode == 1
    assert repo_fingerprint(fixture_repo) == before


def test_ac_wtc_s04_status_reports_not_started_without_execution_evidence(
    fixture_repo: Path, evidence_dir: Path
):
    """No receipt namespace is a valid non-mutating not-started observation."""
    manifest_path, _, _, _, final_path = status_fixture(fixture_repo, evidence_dir)
    for path in evidence_dir.glob("preservation-*.json"):
        path.unlink()
    for path in evidence_dir.glob("progress-*.json"):
        path.unlink()
    final_path.unlink()

    result = run_status(fixture_repo, manifest_path, evidence_dir)

    assert result.returncode == 0, result.stderr
    observed = json.loads(result.stdout)
    assert observed["state"] == "not_started"
    assert observed["completed_actions"] == 0
