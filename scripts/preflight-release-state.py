#!/usr/bin/env python3
"""Validate release-state facts consumed by ``scripts/preflight.sh``."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
from typing import Any


class StateError(ValueError):
    """The selected release state cannot authorize preflight."""


def git(repo_root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo_root), "--no-optional-locks", *args],
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise StateError(detail or f"git {' '.join(args)} failed")
    return result.stdout.strip()


def resolve_commit(repo_root: Path, ref: str, label: str) -> str:
    try:
        return git(repo_root, "rev-parse", "--verify", f"{ref}^{{commit}}")
    except StateError as exc:
        raise StateError(f"{label} {ref} does not resolve to a commit") from exc


def require_ancestor(repo_root: Path, ancestor: str, descendant: str, label: str) -> None:
    result = subprocess.run(
        ["git", "-C", str(repo_root), "merge-base", "--is-ancestor", ancestor, descendant],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode:
        raise StateError(label)


def require_tracked(repo_root: Path, path: str, label: str) -> None:
    try:
        git(repo_root, "ls-files", "--error-unmatch", "--", path)
    except StateError as exc:
        raise StateError(f"{label} is not tracked: {path}") from exc


def normalize_repo_path(repo_root: Path, raw: str, label: str) -> str:
    root = repo_root.resolve()
    candidate = Path(raw)
    resolved = (candidate if candidate.is_absolute() else root / candidate).resolve()
    if not resolved.is_relative_to(root):
        raise StateError(f"{label} escapes the repository: {raw}")
    return resolved.relative_to(root).as_posix()


def load_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise StateError(f"cannot parse release state {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise StateError("release state must be a JSON object")
    return value


def validate_state(
    *,
    repo_root: Path,
    release: str,
    board: str,
    state_file: str,
    plan: str | None,
    expect_closed: int | str | None,
    target_head: str,
) -> dict[str, str | None]:
    """Return verified baseline/dependency facts or raise ``StateError``."""

    state_file = normalize_repo_path(repo_root, state_file, "release state")
    board = normalize_repo_path(repo_root, board, "release board")
    plan = normalize_repo_path(repo_root, plan, "release plan") if plan else None
    expected_state = f"dev/plans/release-state-{release}.json"
    expected_board = f"dev/plans/runs/STATUS-{release}.md"
    if state_file != expected_state:
        raise StateError(f"state filename must be {expected_state}")
    if board != expected_board:
        raise StateError(f"board selected by release-current must be {expected_board}")
    require_tracked(repo_root, state_file, "release state")
    require_tracked(repo_root, board, "release board")

    state = load_object(repo_root / state_file)
    if state.get("release") != release:
        raise StateError(f"state release must be {release}")
    if state.get("board") != board:
        raise StateError(f"state board must be {board}")

    state_plan = state.get("plan")
    if not isinstance(state_plan, str) or not state_plan:
        raise StateError("state plan must be a non-empty repository path")
    state_plan = normalize_repo_path(repo_root, state_plan, "state plan")
    require_tracked(repo_root, state_plan, "release plan")
    if plan is not None and plan != state_plan:
        raise StateError(f"--plan must match state plan {state_plan}")

    expected_active_ref = f"refs/heads/release/{release}"
    if state.get("active_ref") != expected_active_ref:
        raise StateError(f"active_ref must be {expected_active_ref}")

    completion = state.get("completion")
    if completion is None:
        baseline_ref = expected_active_ref
    else:
        if not isinstance(completion, dict) or set(completion) != {
            "ref",
            "main_integration",
        }:
            raise StateError("completion must contain exactly ref and main_integration")
        completion_ref = f"origin/release/{release}"
        if completion.get("ref") != completion_ref:
            raise StateError(f"completion.ref must be {completion_ref}")
        integration = completion.get("main_integration")
        if integration == "PENDING":
            baseline_ref = completion_ref
        elif integration == "COMPLETE":
            completion_sha = resolve_commit(
                repo_root, completion_ref, "release completion ref"
            )
            origin_main_sha = resolve_commit(repo_root, "origin/main", "origin/main")
            require_ancestor(
                repo_root,
                completion_sha,
                origin_main_sha,
                f"release completion ref {completion_ref} is not reachable from origin/main",
            )
            baseline_ref = "origin/main"
        else:
            raise StateError("completion.main_integration must be PENDING or COMPLETE")

    baseline_sha = resolve_commit(repo_root, baseline_ref, "release baseline ref")
    target_sha = resolve_commit(repo_root, target_head, "target HEAD")
    require_ancestor(
        repo_root,
        baseline_sha,
        target_sha,
        f"target HEAD {target_sha} is not a descendant of release baseline {baseline_ref}",
    )

    dependency_sha: str | None = None
    if expect_closed is not None:
        try:
            wanted = int(str(expect_closed))
        except ValueError as exc:
            raise StateError("--expect-closed must be an integer slice id") from exc
        ladder = state.get("ladder")
        if not isinstance(ladder, list):
            raise StateError("state ladder must be an array")
        matches = [
            item
            for item in ladder
            if isinstance(item, dict) and item.get("slice") == wanted
        ]
        if len(matches) != 1:
            raise StateError(f"dependency Slice {wanted} must have exactly one ladder entry")
        entry = matches[0]
        if entry.get("status") not in {"COMPLETE_ON_RELEASE_BRANCH", "LANDED"}:
            raise StateError(f"dependency Slice {wanted} is not closed in release state")
        raw_sha = entry.get("sha")
        if not isinstance(raw_sha, str) or not raw_sha:
            raise StateError(f"dependency Slice {wanted} has no commit SHA")
        dependency_sha = resolve_commit(repo_root, raw_sha, "dependency SHA")
        require_ancestor(
            repo_root,
            dependency_sha,
            baseline_sha,
            f"dependency Slice {wanted} SHA is not an ancestor of {baseline_ref}",
        )
        require_ancestor(
            repo_root,
            dependency_sha,
            target_sha,
            f"dependency Slice {wanted} SHA is not an ancestor of target HEAD",
        )

    return {
        "release": release,
        "state_file": state_file,
        "baseline_ref": baseline_ref,
        "baseline_sha": baseline_sha,
        "dependency_sha": dependency_sha,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--release", required=True)
    parser.add_argument("--board", required=True)
    parser.add_argument("--state", required=True)
    parser.add_argument("--plan")
    parser.add_argument("--expect-closed")
    parser.add_argument("--target-head", required=True)
    parser.add_argument("--format", choices=["json", "tsv"], default="json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        facts = validate_state(
            repo_root=args.repo_root,
            release=args.release,
            board=args.board,
            state_file=args.state,
            plan=args.plan,
            expect_closed=args.expect_closed,
            target_head=args.target_head,
        )
    except StateError as exc:
        print(f"FAIL release-state preflight: {exc}", file=sys.stderr)
        return 1
    if args.format == "tsv":
        print(
            "\t".join(
                str(facts[key] or "")
                for key in [
                    "release",
                    "state_file",
                    "baseline_ref",
                    "baseline_sha",
                    "dependency_sha",
                ]
            )
        )
    else:
        print(json.dumps(facts, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
