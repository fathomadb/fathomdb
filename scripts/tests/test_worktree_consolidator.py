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


def test_ac_wtc_001_cli_exposes_exact_four_modes(fixture_repo: Path):
    """AC-WTC-001: the public CLI exposes the authority-separated pipeline."""
    result = run_tool(fixture_repo, "--help")
    assert result.returncode == 0, result.stderr
    for mode in ("audit", "manifest", "dryrun", "consolidate"):
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
