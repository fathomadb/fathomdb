#!/usr/bin/env python3
"""Manifest-gated local Git worktree and branch consolidation.

This is a deliberately conservative local operator tool.  It never modifies a
remote and will stop instead of guessing when evidence is incomplete.  See
``dev/design/worktree-branch-consolidator.md`` for the protocol.
"""

from __future__ import annotations

import argparse
import contextlib
import datetime as dt
import errno
import hashlib
import json
import os
import re
import secrets
import shutil
import stat
import subprocess
import sys
import tempfile
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


SCHEMA = "fathomdb-worktree-consolidator/v2"
OWNER_SCHEMA = "fathomdb-worktree-owner-map/v1"
OWNER_MAP_REVIEW_SCHEMA = "fathomdb-worktree-owner-map-review-attestation/v1"
BASELINE_SCHEMA = "fathomdb-worktree-baseline-attestation/v1"
APPROVAL_SCHEMA = "fathomdb-worktree-approval-attestation/v1"
DRYRUN_SCHEMA = "fathomdb-worktree-dryrun-receipt/v1"
FREEZE_SCHEMA = "fathomdb-worktree-freeze-attestation/v1"
RETIREMENT_PROOFS_SCHEMA = "fathomdb-worktree-retirement-proofs/v1"
RETIREMENT_PROOF_APPROVAL_SCHEMA = "fathomdb-worktree-retirement-proof-approval/v1"
STATUS_SCHEMA = "fathomdb-worktree-execution-status/v1"


class SafetyError(RuntimeError):
    """A failed safety precondition that must not trigger cleanup."""


class GoalBlocked(SafetyError):
    """The requested topology cannot be reached without an unsafe transition."""


class ReviewRequired(SafetyError):
    """A required human review attestation has not been supplied."""


class PartialBatch(SafetyError):
    """A preservation-complete retirement batch stopped before all actions."""


def canonical_bytes(value: Any) -> bytes:
    """Return the sole JSON encoding used for all hash-bound evidence."""
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode("utf-8")


def sha256(value: Any) -> str:
    """Return the SHA-256 of canonical JSON data or already encoded bytes."""
    raw = value if isinstance(value, bytes) else canonical_bytes(value)
    return hashlib.sha256(raw).hexdigest()


def utcnow() -> dt.datetime:
    """Return a timezone-aware current timestamp."""
    return dt.datetime.now(dt.UTC)


def rfc3339(value: dt.datetime) -> str:
    """Render a UTC timestamp in the evidence format."""
    return value.astimezone(dt.UTC).isoformat().replace("+00:00", "Z")


def parse_time(value: object, field: str) -> dt.datetime:
    """Parse an RFC3339 timestamp, failing closed for non-string values."""
    if not isinstance(value, str):
        raise SafetyError(f"{field} must be an RFC3339 string")
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise SafetyError(f"{field} is not RFC3339") from exc
    if parsed.tzinfo is None:
        raise SafetyError(f"{field} must include a timezone")
    return parsed.astimezone(dt.UTC)


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    """Run a shell-free Git command with optional index locking disabled."""
    env = os.environ.copy()
    for key in tuple(env):
        if key.startswith("GIT_"):
            env.pop(key)
    env["GIT_OPTIONAL_LOCKS"] = "0"
    result = subprocess.run(
        ["git", *args], cwd=repo, capture_output=True, text=True, env=env, check=False
    )
    if check and result.returncode:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown git error"
        raise SafetyError(f"git {' '.join(args)} failed: {detail}")
    return result


def git_bytes(repo: Path, *args: str, stdin: bytes | None = None) -> bytes:
    """Run shell-free Git without decoding or logging a proof payload."""
    env = os.environ.copy()
    for key in tuple(env):
        if key.startswith("GIT_"):
            env.pop(key)
    env["GIT_OPTIONAL_LOCKS"] = "0"
    result = subprocess.run(
        ["git", *args], cwd=repo, input=stdin, capture_output=True, env=env, check=False
    )
    if result.returncode:
        raise SafetyError(f"git {' '.join(args)} failed while computing proof")
    return result.stdout


def realpath(path: str | Path) -> Path:
    """Resolve a path without accepting a non-existent target silently."""
    return Path(path).expanduser().resolve(strict=True)


def git_path(repo: Path, *args: str) -> Path:
    """Resolve a Git path that may be relative to a worktree."""
    raw = Path(git(repo, *args).stdout.strip())
    return raw if raw.is_absolute() else (repo / raw).resolve()


@dataclass(frozen=True)
class Repository:
    """Canonical local repository identity and worktree information."""

    root: Path
    common_dir: Path
    primary: Path
    worktrees: tuple[dict[str, Any], ...]

    @property
    def identity(self) -> dict[str, str]:
        """Return hashable identity used by plans and attestations."""
        return {"primary_root": str(self.primary), "git_common_dir": str(self.common_dir)}


def parse_worktrees(repo: Path) -> tuple[dict[str, Any], ...]:
    """Parse ``git worktree list --porcelain`` without relying on display output."""
    blocks = git(repo, "worktree", "list", "--porcelain").stdout.strip().split("\n\n")
    entries: list[dict[str, Any]] = []
    for block in blocks:
        fields: dict[str, str | bool] = {}
        for line in block.splitlines():
            key, _, value = line.partition(" ")
            fields[key] = value if value else True
        if "worktree" not in fields:
            continue
        path = realpath(str(fields["worktree"]))
        git_dir = git_path(path, "rev-parse", "--git-dir")
        common_dir = git_path(path, "rev-parse", "--git-common-dir")
        status = git(path, "status", "--porcelain=v1", "--ignored").stdout.splitlines()
        counts = {"tracked": 0, "untracked": 0, "ignored": 0}
        for line in status:
            if line.startswith("!!"):
                counts["ignored"] += 1
            elif line.startswith("??"):
                counts["untracked"] += 1
            else:
                counts["tracked"] += 1
        entries.append(
            {
                "path": str(path),
                "head": str(fields.get("HEAD", "")),
                "branch": str(fields["branch"]) if "branch" in fields else None,
                "detached": "detached" in fields,
                "locked": "locked" in fields,
                "primary": git_dir == common_dir,
                "clean": not any(counts.values()),
                "changes": counts,
            }
        )
    return tuple(sorted(entries, key=lambda entry: entry["path"]))


def repository(path: str | Path) -> Repository:
    """Resolve a repository root and its registered worktree topology."""
    root = git_path(Path(path).resolve(), "rev-parse", "--show-toplevel")
    common = git_path(root, "rev-parse", "--git-common-dir")
    worktrees = parse_worktrees(root)
    primary = [entry for entry in worktrees if entry["primary"]]
    if len(primary) != 1:
        raise SafetyError("could not identify exactly one primary checkout")
    return Repository(root, common, Path(primary[0]["path"]), worktrees)


def read_json(path: Path, label: str) -> Any:
    """Read JSON and reject non-object inputs at the named boundary."""
    try:
        raw = path.read_bytes()
        value = json.loads(raw.decode("utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SafetyError(f"cannot read {label}: {exc}") from exc
    if not isinstance(value, dict):
        raise SafetyError(f"{label} must be a JSON object")
    if raw != canonical_bytes(value):
        raise SafetyError(f"{label} is not canonical JSON")
    return value


def regular_child(path: str | Path, evidence_dir: Path, label: str) -> Path:
    """Require an existing regular non-symlink file directly in evidence dir."""
    candidate = Path(path)
    if candidate.is_symlink() or not candidate.is_file():
        raise SafetyError(f"{label} must be a regular non-symlink file")
    resolved = candidate.resolve(strict=True)
    if resolved.parent != evidence_dir:
        raise SafetyError(f"{label} must reside directly under --evidence-dir")
    return resolved


def validate_private_directory(path: str | Path, repo: Repository, label: str) -> Path:
    """Validate one existing external evidence/archive directory."""
    candidate = Path(path)
    if candidate.is_symlink() or not candidate.is_dir():
        raise SafetyError(f"{label} must be an existing non-symlink directory")
    resolved = candidate.resolve(strict=True)
    if any(resolved == Path(item["path"]) or Path(item["path"]) in resolved.parents for item in repo.worktrees):
        raise SafetyError(f"{label} must be outside every registered worktree")
    info = resolved.stat()
    if info.st_uid != os.geteuid() or info.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
        raise SafetyError(f"{label} must be effective-user-owned and not group/world writable")
    return resolved


def directory_attributes(path: Path) -> dict[str, int | str | bool]:
    """Return the recheckable attributes dryrun binds to execution."""
    info = path.stat()
    return {
        "path": str(path),
        "device": info.st_dev,
        "inode": info.st_ino,
        "uid": info.st_uid,
        "gid": info.st_gid,
        "mode": stat.S_IMODE(info.st_mode),
        "no_symlink": not path.is_symlink(),
    }


def durable_write(path: Path, value: dict[str, Any]) -> None:
    """No-clobber publish canonical evidence with file and directory fsyncs."""
    if path.exists() or path.is_symlink():
        raise SafetyError(f"evidence file already exists: {path.name}")
    fd, raw_temp = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temp = Path(raw_temp)
    try:
        payload = canonical_bytes(value)
        with os.fdopen(fd, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temp, path)
        except FileExistsError as exc:
            raise SafetyError(f"evidence file already exists: {path.name}") from exc
        temp.unlink()
        parent_fd = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(parent_fd)
        finally:
            os.close(parent_fd)
    except Exception:
        temp.unlink(missing_ok=True)
        raise


def execution_evidence_paths(evidence: Path, manifest_hash: str, action_count: int) -> list[Path]:
    """Return every deterministic receipt path that an execution can publish."""
    prefix = manifest_hash[:16]
    return [
        evidence / f"preservation-{prefix}.json",
        evidence / f"execution-{prefix}.json",
        *(evidence / f"progress-{prefix}-{index:04d}.json" for index in range(1, action_count + 1)),
    ]


def reserve_execution_evidence(paths: Iterable[Path]) -> None:
    """Fail before preservation when a deterministic evidence name is occupied."""
    occupied = [path.name for path in paths if path.exists() or path.is_symlink()]
    if occupied:
        raise SafetyError(f"execution evidence path already exists: {', '.join(sorted(occupied))}")


def write_partial_fallback(evidence: Path, receipt: dict[str, Any], failure: Exception) -> Path:
    """Publish an honest non-clobber partial receipt if the final path cannot be written."""
    partial = dict(receipt)
    partial["result"] = "partial"
    partial["failure"] = str(failure)
    for _ in range(4):
        path = evidence / f"partial-{receipt['manifest_sha256'][:16]}-{secrets.token_hex(8)}.json"
        try:
            durable_write(path, partial)
            return path
        except Exception:
            continue
    raise PartialBatch("could not persist fallback partial execution receipt") from failure


def status_regular_json(path: Path, label: str) -> tuple[dict[str, Any], str] | None:
    """Read and hash one optional receipt without following a filesystem link."""
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except FileNotFoundError:
        return None
    except OSError as exc:
        raise SafetyError(f"cannot safely open {label}") from exc
    try:
        with os.fdopen(fd, "rb") as handle:
            if not stat.S_ISREG(os.fstat(handle.fileno()).st_mode):
                raise SafetyError(f"{label} must be a regular non-symlink file")
            raw = handle.read()
    except OSError as exc:
        raise SafetyError(f"cannot read {label}") from exc
    try:
        value = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise SafetyError(f"cannot parse {label}") from exc
    if not isinstance(value, dict) or raw != canonical_bytes(value):
        raise SafetyError(f"{label} is not canonical JSON object evidence")
    return value, sha256(raw)


def status_hex(value: object, label: str) -> str:
    """Require a lower-case SHA-256 digest in status evidence."""
    if not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{64}", value) is None:
        raise SafetyError(f"{label} must be a SHA-256 digest")
    return value


def status_string_list(value: object, label: str, *, unique: bool = True) -> list[str]:
    """Require sorted string-list evidence, optionally preserving repeated tips."""
    if not isinstance(value, list) or any(not isinstance(item, str) for item in value):
        raise SafetyError(f"{label} must be a list of strings")
    expected = sorted(set(value)) if unique else sorted(value)
    if value != expected:
        raise SafetyError(f"{label} must be sorted" + (" and duplicate-free" if unique else ""))
    return value


def status_manifest_inputs(manifest: dict[str, Any]) -> tuple[list[dict[str, Any]], list[str]]:
    """Extract the immutable action and preservation facts a receipt must bind."""
    preservation = manifest.get("preservation")
    required = {
        "bundle_name_algorithm",
        "include_all_local_refs",
        "required_tips",
        "reflog_candidates",
    }
    if not isinstance(preservation, dict) or set(preservation) != required:
        raise SafetyError("manifest preservation inputs are invalid")
    if (
        preservation["bundle_name_algorithm"] != "wtc-bundle-v1"
        or preservation["include_all_local_refs"] is not True
    ):
        raise SafetyError("manifest preservation policy is invalid")
    required_tips = status_string_list(
        preservation["required_tips"], "manifest required_tips", unique=False
    )
    reflog_candidates = status_string_list(
        preservation["reflog_candidates"], "manifest reflog_candidates"
    )
    entries = manifest.get("entries")
    if not isinstance(entries, list) or any(not isinstance(entry, dict) for entry in entries):
        raise SafetyError("manifest entries are invalid")
    return entries, sorted(set(required_tips) | set(reflog_candidates))


def validate_status_preservation(
    value: dict[str, Any], repo: Repository, manifest_hash: str, covered_tips: list[str]
) -> None:
    """Validate the pre-retirement preservation receipt used by all later evidence."""
    required = {
        "schema",
        "repository",
        "manifest_sha256",
        "bundle_path",
        "bundle_sha256",
        "covered_tips",
        "bundle_inputs",
        "bundle_verify",
        "verified_before_retirement",
        "issued_at",
    }
    if set(value) != required or value.get("schema") != "fathomdb-worktree-preservation-receipt/v1":
        raise SafetyError("preservation receipt has unknown or missing fields")
    if value.get("repository") != repo.identity or value.get("manifest_sha256") != manifest_hash:
        raise SafetyError("preservation receipt does not bind this manifest")
    if (
        not isinstance(value.get("bundle_path"), str)
        or not isinstance(value.get("bundle_verify"), str)
        or value.get("verified_before_retirement") is not True
    ):
        raise SafetyError("preservation receipt has invalid metadata")
    status_hex(value.get("bundle_sha256"), "preservation bundle_sha256")
    if status_string_list(value.get("covered_tips"), "preservation covered_tips") != covered_tips:
        raise SafetyError("preservation receipt covered tips differ from manifest")
    if not isinstance(value.get("bundle_inputs"), list) or any(
        not isinstance(item, str) for item in value["bundle_inputs"]
    ):
        raise SafetyError("preservation receipt bundle_inputs are invalid")
    parse_time(value.get("issued_at"), "preservation receipt issued_at")


def validate_status_progress(
    value: dict[str, Any],
    repo: Repository,
    manifest_hash: str,
    preservation_hash: str,
    entries: list[dict[str, Any]],
    index: int,
) -> None:
    """Validate one contiguous durable action-prefix receipt."""
    required = {
        "schema",
        "repository",
        "manifest_sha256",
        "preservation_receipt_sha256",
        "completed_actions",
        "issued_at",
    }
    if set(value) != required or value.get("schema") != "fathomdb-worktree-progress-receipt/v1":
        raise SafetyError("progress receipt has unknown or missing fields")
    if (
        value.get("repository") != repo.identity
        or value.get("manifest_sha256") != manifest_hash
        or value.get("preservation_receipt_sha256") != preservation_hash
        or value.get("completed_actions") != entries[:index]
    ):
        raise SafetyError("progress receipt does not bind the completed action prefix")
    parse_time(value.get("issued_at"), "progress receipt issued_at")


def validate_status_final(
    value: dict[str, Any],
    repo: Repository,
    manifest_hash: str,
    preservation_hash: str,
    preservation: dict[str, Any],
    entries: list[dict[str, Any]],
    progress_count: int,
    *,
    fallback: bool,
) -> str:
    """Validate a deterministic or randomized terminal execution receipt."""
    required = {
        "schema",
        "repository",
        "manifest_sha256",
        "preservation_receipt_sha256",
        "bundle_path",
        "bundle_sha256",
        "covered_tips",
        "completed_actions",
        "post_snapshot_id",
        "result",
        "issued_at",
    }
    if fallback:
        required |= {"failure"}
    if set(value) != required or value.get("schema") != "fathomdb-worktree-execution-receipt/v1":
        raise SafetyError("execution receipt has unknown or missing fields")
    if (
        value.get("repository") != repo.identity
        or value.get("manifest_sha256") != manifest_hash
        or value.get("preservation_receipt_sha256") != preservation_hash
        or value.get("bundle_path") != preservation["bundle_path"]
        or value.get("bundle_sha256") != preservation["bundle_sha256"]
        or value.get("covered_tips") != preservation["covered_tips"]
    ):
        raise SafetyError("execution receipt does not bind preservation evidence")
    if not isinstance(value.get("completed_actions"), list):
        raise SafetyError("execution receipt completed_actions are invalid")
    completed = value["completed_actions"]
    if completed != entries[: len(completed)] or len(completed) > len(entries):
        raise SafetyError("execution receipt actions are not a manifest prefix")
    result = value.get("result")
    if result not in {"success", "partial"}:
        raise SafetyError("execution receipt result is invalid")
    post_snapshot = value.get("post_snapshot_id")
    if result == "success":
        if (
            fallback
            or len(completed) != len(entries)
            or progress_count != len(entries)
            or not isinstance(post_snapshot, str)
        ):
            raise SafetyError("success receipt is incomplete")
        status_hex(post_snapshot, "execution receipt post_snapshot_id")
    elif post_snapshot is not None or len(completed) not in {progress_count, progress_count + 1}:
        raise SafetyError("partial receipt action prefix is invalid")
    if fallback and not isinstance(value.get("failure"), str):
        raise SafetyError("fallback partial receipt failure is invalid")
    parse_time(value.get("issued_at"), "execution receipt issued_at")
    return result


def status_receipts(
    evidence: Path, repo: Repository, manifest: dict[str, Any], manifest_hash: str
) -> tuple[int, int, bool, bool]:
    """Validate one exact evidence namespace and return its terminal facts."""
    entries, covered_tips = status_manifest_inputs(manifest)
    prefix = manifest_hash[:16]
    preservation_path = evidence / f"preservation-{prefix}.json"
    final_path = evidence / f"execution-{prefix}.json"
    fallback_pattern = re.compile(rf"partial-{re.escape(prefix)}-[0-9a-f]{{16}}\.json")
    progress_pattern = re.compile(rf"progress-{re.escape(prefix)}-([0-9]{{4}})\.json")
    fallback_paths: list[Path] = []
    progress_paths: dict[int, Path] = {}
    for path in evidence.iterdir():
        name = path.name
        if name.startswith(f"partial-{prefix}-"):
            if fallback_pattern.fullmatch(name) is None:
                raise SafetyError("fallback partial receipt name is invalid")
            fallback_paths.append(path)
        elif name.startswith(f"progress-{prefix}-"):
            matched = progress_pattern.fullmatch(name)
            if matched is None:
                raise SafetyError("progress receipt name is invalid")
            index = int(matched.group(1))
            if index < 1 or index > len(entries):
                raise SafetyError("progress receipt number is outside manifest range")
            progress_paths[index] = path
        elif name.startswith(f"preservation-{prefix}") and name != preservation_path.name:
            raise SafetyError("preservation receipt name is invalid")
        elif name.startswith(f"execution-{prefix}") and name != final_path.name:
            raise SafetyError("execution receipt name is invalid")
    if len(fallback_paths) > 1:
        raise SafetyError("multiple fallback partial receipts are unsafe")
    preservation_record = status_regular_json(preservation_path, "preservation receipt")
    final_record = status_regular_json(final_path, "execution receipt")
    preservation = preservation_record[0] if preservation_record is not None else None
    final = final_record[0] if final_record is not None else None
    progress_count = 0
    if progress_paths and preservation is None:
        raise SafetyError("progress receipts require preservation evidence")
    if preservation is not None:
        validate_status_preservation(preservation, repo, manifest_hash, covered_tips)
        assert preservation_record is not None
        preservation_hash = preservation_record[1]
        for index in range(1, len(entries) + 1):
            path = progress_paths.get(index)
            if path is None:
                if any(number > index for number in progress_paths):
                    raise SafetyError("progress receipts are not contiguous")
                break
            progress_record = status_regular_json(path, f"progress receipt {index}")
            assert progress_record is not None
            value = progress_record[0]
            validate_status_progress(
                value, repo, manifest_hash, preservation_hash, entries, index
            )
            progress_count = index
        if final is not None:
            validate_status_final(
                final,
                repo,
                manifest_hash,
                preservation_hash,
                preservation,
                entries,
                progress_count,
                fallback=False,
            )
        for path in fallback_paths:
            fallback_record = status_regular_json(path, "fallback partial receipt")
            assert fallback_record is not None
            fallback = fallback_record[0]
            validate_status_final(
                fallback,
                repo,
                manifest_hash,
                preservation_hash,
                preservation,
                entries,
                progress_count,
                fallback=True,
            )
    elif final is not None or fallback_paths:
        raise SafetyError("terminal receipt requires preservation evidence")
    final_success = final is not None and final.get("result") == "success"
    return progress_count, len(entries), preservation is not None, bool(fallback_paths) or not final_success


def status_lock_present(repo: Repository) -> bool:
    """Observe the cooperative lock without reading or interpreting its payload."""
    try:
        os.lstat(repo.common_dir / "worktree-consolidator.lock")
    except OSError as exc:
        if exc.errno == errno.ENOENT:
            return False
        raise SafetyError("cannot observe consolidator lock") from exc
    return True


def command_status(args: argparse.Namespace) -> int:
    """Report durable execution state without changing Git or evidence."""
    repo = repository(args.repo)
    evidence = validate_private_directory(args.evidence_dir, repo, "evidence directory")
    manifest_path = regular_child(args.manifest, evidence, "manifest")
    manifest = read_json(manifest_path, "manifest")
    verify_manifest(manifest, repo)
    manifest_hash = sha256(manifest_path.read_bytes())
    completed, total, has_preservation, terminal_incomplete = status_receipts(
        evidence, repo, manifest, manifest_hash
    )
    lock_present = status_lock_present(repo)
    if lock_present:
        state = "executing"
        phase = "finalizing" if has_preservation and not terminal_incomplete else "running"
        action = "monitor durable receipts; do not clear lock or start another consolidation"
    elif not has_preservation:
        state = "not_started"
        phase = "none"
        action = "review the manifest before any consolidation"
    elif terminal_incomplete:
        state = "recovery_required"
        phase = "partial"
        action = "preserve evidence; investigate; do not clear lock or resume this manifest"
    else:
        state = "completed"
        phase = "complete"
        action = "preserve evidence; no further action is required"
    output(
        {
            "schema": STATUS_SCHEMA,
            "state": state,
            "phase": phase,
            "completed_actions": completed,
            "total_actions": total,
            "operator_action": action,
        },
        args,
    )
    return 0


def baseline(repo: Repository, ref: str) -> dict[str, str | None]:
    """Record selected baseline without fetching or assuming it is fresh."""
    result = git(repo.root, "rev-parse", "--verify", "--quiet", ref, check=False)
    return {"ref": ref, "sha": result.stdout.strip() or None}


def local_refs(repo: Repository, base: dict[str, str | None]) -> list[dict[str, Any]]:
    """Collect local-head topology and advisory ancestry evidence."""
    raw = git(repo.root, "for-each-ref", "--format=%(refname) %(objectname)", "refs/heads").stdout
    used = {entry["branch"] for entry in repo.worktrees if entry["branch"]}
    refs = []
    for line in raw.splitlines():
        ref, tip = line.split(" ", 1)
        ancestor = False
        if base["sha"]:
            ancestor = git(repo.root, "merge-base", "--is-ancestor", tip, str(base["sha"]), check=False).returncode == 0
        refs.append({"ref": ref, "tip": tip, "used_by_worktree": ref in used, "baseline_ancestor": ancestor})
    return sorted(refs, key=lambda entry: entry["ref"])


def retirement_review(
    repo: Repository, refs: Iterable[dict[str, Any]], owners: dict[str, dict[str, Any]]
) -> dict[str, Any]:
    """Explain every owner-proposed local-head retirement with observed predicates."""
    remote_by_tip: dict[str, list[str]] = {}
    remote_raw = git(
        repo.root, "for-each-ref", "--format=%(refname) %(objectname)", "refs/remotes"
    ).stdout
    for line in remote_raw.splitlines():
        ref, tip = line.split(" ", 1)
        remote_by_tip.setdefault(tip, []).append(ref)
    entries = []
    for entry in refs:
        owner = owners.get(entry["ref"])
        if owner is None or owner["owner"] != "none" or owner["release_role"] != "none":
            continue
        is_main = entry["ref"] == "refs/heads/main"
        eligible = not is_main and not entry["used_by_worktree"] and entry["baseline_ancestor"]
        entries.append(
            {
                "target": entry["ref"],
                "tip": entry["tip"],
                "is_main": is_main,
                "checked_out_by_worktree": entry["used_by_worktree"],
                "ancestor_of_baseline": entry["baseline_ancestor"],
                "matching_remote_refs": sorted(remote_by_tip.get(entry["tip"], [])),
                "owner_map_evidence": owner["evidence"],
                "result": "mechanically-eligible" if eligible else "blocked",
            }
        )
    return {
        "schema": "fathomdb-worktree-retirement-review/v1",
        "entries": sorted(entries, key=lambda entry: entry["target"]),
    }


def recovery_candidates(repo: Repository, refs: Iterable[dict[str, Any]]) -> list[str]:
    """Return locally visible reflog/unreachable commit candidates conservatively."""
    reachable = {entry["tip"] for entry in refs}
    candidates: set[str] = set()
    reflog = git(repo.root, "reflog", "--all", "--format=%H", check=False).stdout.splitlines()
    for tip in reflog:
        if tip and tip not in reachable:
            candidates.add(tip)
    fsck = git(repo.root, "fsck", "--no-reflogs", "--unreachable", "--no-progress", check=False).stdout
    for line in fsck.splitlines():
        pieces = line.split()
        if len(pieces) >= 3 and pieces[1] == "commit":
            candidates.add(pieces[2])
    return sorted(candidates)


def owner_entries(owner_map: dict[str, Any], repo: Repository) -> dict[str, dict[str, Any]]:
    """Validate the strict owner map and return entries keyed by target."""
    if owner_map.get("schema") != OWNER_SCHEMA or not isinstance(owner_map.get("entries"), list):
        raise SafetyError("owner map has an invalid schema")
    entries: dict[str, dict[str, Any]] = {}
    for entry in owner_map["entries"]:
        if not isinstance(entry, dict) or set(entry) != {"target", "owner", "release_role", "evidence"}:
            raise SafetyError("owner map entry has unknown or missing fields")
        target = entry["target"]
        if not isinstance(target, str) or target in entries:
            raise SafetyError("owner map targets must be unique strings")
        if entry["owner"] != "none" and not isinstance(entry["owner"], str):
            raise SafetyError("owner map owner must be a string")
        if entry["release_role"] != "none" and not isinstance(entry["release_role"], str):
            raise SafetyError("owner map release_role must be a string")
        entries[target] = entry
    required = {entry["path"] for entry in repo.worktrees}
    required.update(item["ref"] for item in local_refs(repo, baseline(repo, "origin/main")))
    missing = sorted(required - entries.keys())
    if missing:
        raise SafetyError(f"owner map lacks entries for: {', '.join(missing)}")
    return entries


def classify_worktree(
    entry: dict[str, Any], owner: dict[str, Any] | None, branch: dict[str, Any] | None
) -> str:
    """Classify a worktree conservatively without making a retirement decision."""
    if owner is None or owner["owner"] != "none" or owner["release_role"] != "none":
        return "protected-active"
    if entry["primary"]:
        return "protected-active"
    if entry["locked"] or entry["detached"]:
        return "unresolved"
    if not entry["clean"]:
        return "archive-required"
    if branch is None or not branch["baseline_ancestor"]:
        return "unresolved"
    return "merged-retirable"


def make_snapshot(repo: Repository, base_ref: str, owner_map_path: str | None) -> dict[str, Any]:
    """Build one observational snapshot and its deterministic identifier."""
    repo = repository(repo.root)
    base = baseline(repo, base_ref)
    refs = local_refs(repo, base)
    owner_hash: str | None = None
    owners: dict[str, dict[str, Any]] = {}
    executable = False
    if owner_map_path:
        source = realpath(owner_map_path)
        owner_value = read_json(source, "owner map")
        owners = owner_entries(owner_value, repo)
        owner_hash = sha256(source.read_bytes())
        executable = True
    ref_by_name = {entry["ref"]: entry for entry in refs}
    worktrees = []
    for entry in repo.worktrees:
        output = dict(entry)
        output["classification"] = classify_worktree(
            entry, owners.get(entry["path"]), ref_by_name.get(entry["branch"])
        )
        worktrees.append(output)
    output_refs = []
    for entry in refs:
        owner = owners.get(entry["ref"])
        classification = "unresolved"
        if owner and owner["owner"] == "none" and owner["release_role"] == "none":
            classification = "merged-retirable" if entry["baseline_ancestor"] and not entry["used_by_worktree"] else "integration-required"
        output = dict(entry)
        output["classification"] = classification
        output_refs.append(output)
    body: dict[str, Any] = {
        "schema": SCHEMA,
        "repository": repo.identity,
        "baseline": base,
        "baseline_freshness": "unattested",
        "owner_map_sha256": owner_hash,
        "executable": executable,
        "worktrees": worktrees,
        "local_refs": output_refs,
        "retirement_review": retirement_review(repo, refs, owners),
        "recovery_candidates": recovery_candidates(repo, refs),
    }
    body["snapshot_id"] = sha256(body)
    return body


def valid_attestation(
    value: dict[str, Any], *, schema: str, repo: Repository, maximum_age: int, label: str
) -> None:
    """Validate common signed-by-hash evidence timing and repository binding."""
    if value.get("schema") != schema or value.get("repository") != repo.identity:
        raise SafetyError(f"{label} schema or repository identity mismatch")
    issued = parse_time(value.get("issued_at"), f"{label}.issued_at")
    expiry = parse_time(value.get("expires_at"), f"{label}.expires_at")
    now = utcnow()
    if issued > now or expiry < now or expiry < issued:
        raise SafetyError(f"{label} is expired, future-dated, or inverted")
    if now - issued > dt.timedelta(seconds=maximum_age) or expiry - issued > dt.timedelta(seconds=maximum_age):
        raise SafetyError(f"{label} exceeds its maximum age")


def validate_baseline_attestation(
    path: Path, repo: Repository, snapshot: dict[str, Any], maximum_age: int
) -> str:
    """Check baseline freshness against the exact snapshot baseline."""
    value = read_json(path, "baseline attestation")
    if set(value) != {"schema", "repository", "baseline", "fetched_at", "issued_at", "expires_at"}:
        raise SafetyError("baseline attestation has unknown or missing fields")
    valid_attestation(value, schema=BASELINE_SCHEMA, repo=repo, maximum_age=maximum_age, label="baseline attestation")
    parse_time(value["fetched_at"], "baseline attestation.fetched_at")
    if value.get("baseline") != snapshot.get("baseline"):
        raise SafetyError("baseline attestation does not bind the snapshot baseline")
    return sha256(path.read_bytes())


def policy_value(path: Path) -> dict[str, Any]:
    """Load and strictly validate the planning policy."""
    value = read_json(path, "policy")
    required = {
        "primary_role",
        "active_themes",
        "legacy_triage_required",
        "target_range",
        "baseline_max_age_seconds",
        "dryrun_max_age_seconds",
        "theme_targets",
        "retire_local_heads",
        "reflog_candidates",
    }
    if set(value) != required or not isinstance(value["active_themes"], list):
        raise SafetyError("policy has unknown or missing fields")
    if len(set(value["active_themes"])) != len(value["active_themes"]):
        raise SafetyError("policy active themes must be unique")
    bounds = value["target_range"]
    if not (isinstance(bounds, list) and len(bounds) == 2 and all(isinstance(x, int) for x in bounds) and 1 <= bounds[0] <= bounds[1] <= 64):
        raise SafetyError("policy target_range is invalid")
    for key in ("baseline_max_age_seconds", "dryrun_max_age_seconds"):
        if not isinstance(value[key], int) or value[key] <= 0:
            raise SafetyError(f"policy {key} is invalid")
    if not isinstance(value["reflog_candidates"], dict):
        raise SafetyError("policy reflog_candidates must be an object")
    if not isinstance(value["theme_targets"], dict) or set(value["theme_targets"]) != set(value["active_themes"]):
        raise SafetyError("policy must map every active theme to one retained target")
    if not all(isinstance(target, str) for target in value["theme_targets"].values()):
        raise SafetyError("policy theme targets must be strings")
    if len(set(value["theme_targets"].values())) != len(value["theme_targets"]):
        raise SafetyError("policy theme targets must be distinct")
    if not isinstance(value["retire_local_heads"], bool):
        raise SafetyError("policy retire_local_heads must be boolean")
    if any(choice not in {"preserve-in-bundle", "unresolved"} for choice in value["reflog_candidates"].values()):
        raise SafetyError("policy has an invalid reflog disposition")
    return value


def output(value: dict[str, Any], args: argparse.Namespace, evidence_dir: Path | None = None) -> None:
    """Emit canonical JSON to stdout and an allowed manifest file when asked."""
    payload = canonical_bytes(value)
    sys.stdout.write(payload.decode("utf-8"))
    sys.stdout.flush()
    if getattr(args, "output", None):
        if evidence_dir is None:
            raise SafetyError("--output is only available to manifest")
        destination = Path(args.output)
        if destination.parent.resolve() != evidence_dir or destination.exists() or destination.is_symlink():
            raise SafetyError("--output must be a new direct child of --evidence-dir")
        destination.write_bytes(payload)
        os.chmod(destination, 0o600)


def command_audit(args: argparse.Namespace) -> int:
    """Emit a read-only snapshot."""
    repo = repository(args.repo)
    output(make_snapshot(repo, args.baseline, args.owner_map), args)
    return 0


def candidate_manifest(
    snapshot: dict[str, Any],
    policy: dict[str, Any],
    owner_hash: str,
    owner_review_hash: str,
    target: int,
    source: str,
    proof_entries: dict[str, dict[str, Any]],
    proof_chain: dict[str, str] | None,
) -> dict[str, Any]:
    """Create a conservative immutable candidate from a complete snapshot."""
    required_tips = sorted(item["tip"] for item in snapshot["local_refs"])
    selected = sorted(
        candidate
        for candidate, disposition in policy["reflog_candidates"].items()
        if disposition == "preserve-in-bundle"
    )
    retained_theme_targets = set(policy["theme_targets"].values())
    candidates = [
        item
        for item in snapshot["worktrees"]
        if item["classification"] == "merged-retirable"
        and not item["primary"]
        and item["path"] not in retained_theme_targets
    ]
    surplus = max(0, len(snapshot["worktrees"]) - target)
    if len(candidates) < surplus:
        raise GoalBlocked("goal_inference_blocked: insufficient proven retirement candidates")
    entries = []
    for item in sorted(candidates, key=lambda candidate: candidate["path"])[:surplus]:
            entries.append(
                {
                    "kind": "worktree",
                    "target": item["path"],
                    "classification": "merged-retirable",
                    "owner": {"value": "none", "release_role": "none", "evidence_sha256": owner_hash},
                    "action": "remove_worktree",
                    "witness": {"tip": item["head"], "clean": True, "unused_by_retained_worktree": True, "recovery_requirement": "execution_bundle"},
                }
            )
    if policy["retire_local_heads"]:
        for item in snapshot["local_refs"]:
            if (
                item["classification"] == "merged-retirable"
                and item["baseline_ancestor"]
                and not item["used_by_worktree"]
                and item["ref"] not in retained_theme_targets
            ):
                entries.append(
                    {
                        "kind": "branch",
                        "target": item["ref"],
                        "classification": "merged-retirable",
                        "owner": {"value": "none", "release_role": "none", "evidence_sha256": owner_hash},
                        "action": "delete_local_ref",
                        "witness": {"tip": item["tip"], "clean": True, "unused_by_retained_worktree": True, "recovery_requirement": "execution_bundle"},
                    }
                )
        refs = {item["ref"]: item for item in snapshot["local_refs"]}
        for target_ref, proof in sorted(proof_entries.items()):
            item = refs.get(target_ref)
            if (
                item is None
                or item["used_by_worktree"]
                or item["baseline_ancestor"]
                or item["classification"] != "integration-required"
            ):
                raise SafetyError("proof-backed target is not an unused non-ancestor local ref")
            entries.append(
                {
                    "kind": "branch",
                    "target": target_ref,
                    "classification": "proof-retirable",
                    "owner": {
                        "value": "none",
                        "release_role": "none",
                        "evidence_sha256": owner_hash,
                    },
                    "action": "delete_local_ref",
                    "witness": {
                        "tip": item["tip"],
                        "clean": True,
                        "unused_by_retained_worktree": True,
                        "recovery_requirement": "execution_bundle",
                        "proof_type": proof["proof_type"],
                        "proof_entry_sha256": sha256(proof),
                    },
                }
            )
    body: dict[str, Any] = {
        "schema": SCHEMA,
        "snapshot_id": snapshot["snapshot_id"],
        "repository": snapshot["repository"],
        "baseline": snapshot["baseline"],
        "baseline_requirement": {"max_age_seconds": policy["baseline_max_age_seconds"]},
        "policy_sha256": sha256(policy),
        "owner_map_sha256": owner_hash,
        "owner_map_review_sha256": owner_review_hash,
        "retirement_proofs": proof_chain,
        "goal": {"target_worktrees": target, "source": source},
        "preservation": {"bundle_name_algorithm": "wtc-bundle-v1", "include_all_local_refs": True, "required_tips": required_tips, "reflog_candidates": selected},
        "entries": entries,
        "approval_requirement": {"max_age_seconds": 86400},
        "dryrun_requirement": {"max_age_seconds": policy["dryrun_max_age_seconds"]},
        "freeze_requirement": {"snapshot_id": snapshot["snapshot_id"], "max_age_seconds": 900},
    }
    plan_payload = dict(body)
    body["plan_sha256"] = sha256(plan_payload)
    body["manifest_id"] = f"wtc-{snapshot['snapshot_id'][:8]}-{body['plan_sha256'][:8]}"
    return body


def command_manifest(args: argparse.Namespace) -> int:
    """Plan a bounded candidate manifest without changing Git state."""
    repo = repository(args.repo)
    evidence = validate_private_directory(args.evidence_dir, repo, "evidence directory")
    snapshot = read_json(realpath(args.audit), "audit snapshot")
    if snapshot.get("repository") != repo.identity:
        raise SafetyError("audit snapshot repository identity mismatch")
    owners_path = regular_child(args.owner_map, evidence, "owner map")
    owners = owner_entries(read_json(owners_path, "owner map"), repo)
    owner_hash = sha256(owners_path.read_bytes())
    if snapshot.get("owner_map_sha256") != owner_hash:
        raise SafetyError("owner map does not match audit snapshot")
    if not args.owner_map_review_attestation:
        raise ReviewRequired("owner_map_review_required: provide --owner-map-review-attestation")
    owner_review_path = regular_child(
        args.owner_map_review_attestation, evidence, "owner-map review attestation"
    )
    owner_review_hash = verify_owner_map_review(owner_review_path, repo, owner_hash)
    policy_path = regular_child(args.policy, evidence, "policy")
    policy = policy_value(policy_path)
    if set(policy["reflog_candidates"]) != set(snapshot.get("recovery_candidates", [])):
        raise SafetyError("policy must disposition every and only audit recovery candidate")
    baseline_path = regular_child(args.baseline_attestation, evidence, "baseline attestation")
    validate_baseline_attestation(baseline_path, repo, snapshot, policy["baseline_max_age_seconds"])
    snapshot_targets = {item["path"]: item for item in snapshot["worktrees"]}
    for theme, target_name in policy["theme_targets"].items():
        target_entry = snapshot_targets.get(target_name)
        if target_entry is None or target_entry["classification"] not in {"protected-active", "integration-required"}:
            raise SafetyError(f"theme {theme} lacks a protected/integration retained target")
    lower = 1 + len(policy["active_themes"]) + int(policy["legacy_triage_required"])
    min_target, max_target = policy["target_range"]
    if args.infer_target:
        if lower > max_target:
            raise GoalBlocked("goal_inference_blocked: lower bound exceeds policy maximum")
        target = max(min_target, lower)
        source = "inferred"
    else:
        target = args.target_worktrees
        source = "declared"
        if target is None:
            raise SafetyError("manifest requires --target-worktrees or --infer-target")
        if target < lower or target < min_target or target > max_target:
            raise GoalBlocked("goal_inference_blocked: requested target violates policy lower bound")
    proof_entries: dict[str, dict[str, Any]] = {}
    proof_chain: dict[str, str] | None = None
    proof_requested = args.retirement_proofs is not None
    proof_approval_requested = args.retirement_proof_approval is not None
    if proof_requested != proof_approval_requested:
        raise SafetyError("retirement proofs require a matching retirement proof approval")
    if proof_requested:
        if not policy["retire_local_heads"]:
            raise SafetyError("retirement proofs require retire_local_heads policy")
        proof_path = regular_child(args.retirement_proofs, evidence, "retirement proofs")
        proof_approval_path = regular_child(
            args.retirement_proof_approval, evidence, "retirement proof approval"
        )
        proof_value = read_json(proof_path, "retirement proofs")
        proof_targets = {
            item.get("target")
            for item in proof_value.get("proofs", [])
            if isinstance(item, dict) and isinstance(item.get("target"), str)
        }
        proof_entries, proof_hash, proof_approval_hash = load_retirement_proof_chain(
            proof_path,
            proof_approval_path,
            repo,
            snapshot["baseline"],
            owner_hash,
            owners,
            proof_targets,
        )
        proof_chain = {
            "proofs_sha256": proof_hash,
            "approval_sha256": proof_approval_hash,
        }
    manifest = candidate_manifest(
        snapshot,
        policy,
        owner_hash,
        owner_review_hash,
        target,
        source,
        proof_entries,
        proof_chain,
    )
    output(manifest, args, evidence)
    return 0


def verify_manifest(manifest: dict[str, Any], repo: Repository) -> None:
    """Validate core immutable manifest fields before any dependent evidence."""
    required = {"schema", "plan_sha256", "manifest_id", "snapshot_id", "repository", "baseline", "baseline_requirement", "policy_sha256", "owner_map_sha256", "owner_map_review_sha256", "retirement_proofs", "goal", "preservation", "entries", "approval_requirement", "dryrun_requirement", "freeze_requirement"}
    if set(manifest) != required or manifest.get("schema") != SCHEMA or manifest.get("repository") != repo.identity:
        raise SafetyError("manifest schema or repository identity mismatch")
    payload = {key: value for key, value in manifest.items() if key not in {"manifest_id", "plan_sha256"}}
    if manifest["plan_sha256"] != sha256(payload):
        raise SafetyError("manifest plan_sha256 mismatch")
    expected_id = f"wtc-{manifest['snapshot_id'][:8]}-{manifest['plan_sha256'][:8]}"
    if manifest["manifest_id"] != expected_id:
        raise SafetyError("manifest_id mismatch")
    proof_chain = manifest["retirement_proofs"]
    if proof_chain is not None and (
        not isinstance(proof_chain, dict)
        or set(proof_chain) != {"proofs_sha256", "approval_sha256"}
        or not all(isinstance(value, str) and len(value) == 64 for value in proof_chain.values())
    ):
        raise SafetyError("manifest retirement proof chain is invalid")


def accountable_identity(value: object, field: str) -> str:
    """Require one canonical, printable accountable identity."""
    if (
        not isinstance(value, str)
        or not value
        or len(value) > 256
        or value != value.strip()
        or any(unicodedata.category(character).startswith("C") for character in value)
    ):
        raise SafetyError(f"{field} must be a canonical accountable identity")
    return value


def verify_approval(path: Path, manifest_hash: str, repo: Repository, maximum_age: int) -> str:
    """Validate a hash-bound independent approval attestation."""
    value = read_json(path, "approval attestation")
    if set(value) != {"schema", "repository", "manifest_sha256", "reviewer", "decision", "issued_at", "expires_at"}:
        raise SafetyError("approval attestation has unknown or missing fields")
    valid_attestation(value, schema=APPROVAL_SCHEMA, repo=repo, maximum_age=maximum_age, label="approval attestation")
    accountable_identity(value.get("reviewer"), "approval attestation.reviewer")
    if value.get("manifest_sha256") != manifest_hash or value.get("decision") != "approved":
        raise SafetyError("approval attestation does not approve this manifest")
    return sha256(path.read_bytes())


def verify_owner_map_review(path: Path, repo: Repository, owner_map_hash: str) -> str:
    """Validate independent review of the exact owner-map bytes."""
    value = read_json(path, "owner-map review attestation")
    required = {"schema", "repository", "owner_map_sha256", "reviewer", "decision", "issued_at", "expires_at"}
    if set(value) != required:
        raise SafetyError("owner-map review attestation has unknown or missing fields")
    valid_attestation(
        value,
        schema=OWNER_MAP_REVIEW_SCHEMA,
        repo=repo,
        maximum_age=86400,
        label="owner-map review attestation",
    )
    accountable_identity(value.get("reviewer"), "owner-map review attestation.reviewer")
    if value.get("owner_map_sha256") != owner_map_hash or value.get("decision") != "approved":
        raise SafetyError("owner-map review attestation does not approve this owner map")
    return sha256(path.read_bytes())


def check_manifest_owner_map(
    path: Path, repo: Repository, manifest: dict[str, Any]
) -> tuple[str, dict[str, dict[str, Any]]]:
    """Require current explicit ownership evidence to match immutable planning evidence."""
    value = read_json(path, "owner map")
    entries = owner_entries(value, repo)
    digest = sha256(path.read_bytes())
    if digest != manifest["owner_map_sha256"]:
        raise SafetyError("owner map hash mismatch")
    return digest, entries


def exact_ref_tip(repo: Repository, ref: object, namespace: str) -> str:
    """Resolve one exact, non-symbolic ref in the required namespace."""
    if not isinstance(ref, str) or not ref.startswith(namespace):
        raise SafetyError(f"proof ref must be in {namespace}")
    if git(repo.root, "check-ref-format", ref, check=False).returncode:
        raise SafetyError("proof ref name is invalid")
    if git(repo.root, "symbolic-ref", "-q", ref, check=False).returncode == 0:
        raise SafetyError("proof ref must not be symbolic")
    tip = git(repo.root, "show-ref", "--verify", "--hash", ref, check=False).stdout.strip()
    if not tip:
        raise SafetyError(f"proof ref is missing: {ref}")
    return tip


def stable_patch_id(repo: Repository, commit: str) -> str:
    """Compute one config-independent stable patch ID without persisting its patch."""
    patch = git_bytes(
        repo.root,
        "show",
        "--pretty=format:",
        "--patch",
        "--no-ext-diff",
        "--no-textconv",
        "--no-renames",
        "--binary",
        commit,
    )
    raw = git_bytes(repo.root, "patch-id", "--stable", stdin=patch)
    fields = raw.decode("ascii", errors="strict").split()
    if len(fields) != 2:
        raise SafetyError("stable patch proof contains an empty commit")
    try:
        int(fields[0], 16)
    except ValueError as exc:
        raise SafetyError("stable patch proof produced a non-hex patch ID") from exc
    if len(fields[0]) not in {40, 64}:
        raise SafetyError("stable patch proof produced an invalid patch ID")
    return fields[0]


def baseline_patch_index(repo: Repository, baseline_sha: str) -> dict[str, list[str]]:
    """Index all non-merge, non-empty baseline commits by stable patch ID."""
    commits = git(
        repo.root, "rev-list", "--reverse", "--no-merges", baseline_sha
    ).stdout.splitlines()
    index: dict[str, list[str]] = {}
    for commit in commits:
        try:
            patch_id = stable_patch_id(repo, commit)
        except SafetyError as exc:
            if "empty commit" in str(exc):
                continue
            raise
        index.setdefault(patch_id, []).append(commit)
    return {patch_id: sorted(matches) for patch_id, matches in index.items()}


def validate_stable_patch_proof(
    repo: Repository,
    proof: dict[str, Any],
    baseline_sha: str,
    patch_index: dict[str, list[str]],
) -> None:
    """Require complete exact stable-patch coverage of one live target."""
    source = proof.get("source_commits")
    if not isinstance(source, list) or not source:
        raise SafetyError("complete stable-patch coverage requires non-empty source commits")
    live_commits = git(
        repo.root,
        "rev-list",
        "--reverse",
        proof["target_tip"],
        "--not",
        baseline_sha,
    ).stdout.splitlines()
    if not live_commits or len(source) != len(live_commits):
        raise SafetyError("complete stable-patch coverage does not match every source commit")
    expected: list[dict[str, Any]] = []
    for commit in live_commits:
        parents = git(repo.root, "rev-list", "--parents", "-n", "1", commit).stdout.split()
        if len(parents) > 2:
            raise SafetyError("complete stable-patch coverage cannot cover merge commits")
        patch_id = stable_patch_id(repo, commit)
        matches = patch_index.get(patch_id, [])
        if not matches:
            raise SafetyError("stable patch has no exact match on the captured baseline")
        expected.append(
            {
                "commit": commit,
                "stable_patch_id": patch_id,
                "baseline_matches": matches,
            }
        )
    if source != expected:
        raise SafetyError("complete stable patch coverage evidence differs from live Git")


def validate_retirement_proof_set(
    value: dict[str, Any],
    repo: Repository,
    baseline_value: dict[str, Any],
    owner_hash: str,
    owners: dict[str, dict[str, Any]],
    retiring_targets: set[str],
    live_targets: set[str] | None = None,
) -> dict[str, dict[str, Any]]:
    """Validate closed proof schemas and recompute selected live Git relations."""
    required = {
        "schema", "repository", "baseline", "owner_map_sha256", "author", "issued_at", "proofs"
    }
    if (
        set(value) != required
        or value.get("schema") != RETIREMENT_PROOFS_SCHEMA
        or value.get("repository") != repo.identity
        or value.get("baseline") != baseline_value
        or value.get("owner_map_sha256") != owner_hash
    ):
        raise SafetyError("retirement proof set schema or hash binding mismatch")
    accountable_identity(value.get("author"), "retirement proof author")
    if parse_time(value.get("issued_at"), "retirement proofs.issued_at") > utcnow():
        raise SafetyError("retirement proof set is future-dated")
    if not isinstance(value.get("proofs"), list) or not value["proofs"]:
        raise SafetyError("retirement proof set must contain at least one proof")
    baseline_sha = baseline_value.get("sha")
    if not isinstance(baseline_sha, str) or not baseline_sha:
        raise SafetyError("retirement proof baseline must resolve to a commit")
    common = {
        "target", "target_tip", "proof_type", "semantic_disposition", "evidence_id"
    }
    patch_index: dict[str, list[str]] | None = None
    proofs: dict[str, dict[str, Any]] = {}
    for raw in value["proofs"]:
        if not isinstance(raw, dict):
            raise SafetyError("retirement proof entry must be an object")
        target = raw.get("target")
        if not isinstance(target, str) or target in proofs:
            raise SafetyError("retirement proof targets must be unique local refs")
        proof_type = raw.get("proof_type")
        if live_targets is not None and target not in live_targets:
            if not common <= set(raw) or proof_type not in {
                "stable_patch_coverage", "retained_local_ref", "remote_tracking_ref"
            }:
                raise SafetyError("retirement proof entry schema changed")
            proofs[target] = raw
            continue
        target_tip = exact_ref_tip(repo, target, "refs/heads/")
        if raw.get("target_tip") != target_tip:
            raise SafetyError("retirement proof target tip differs from live Git")
        if any(item["branch"] == target for item in repo.worktrees):
            raise SafetyError("retirement proof target remains used by a worktree")
        mapped = owners.get(target)
        if mapped is None or mapped["owner"] != "none" or mapped["release_role"] != "none":
            raise SafetyError("retirement proof target lacks explicit no-owner evidence")
        if raw.get("semantic_disposition") != "retire-local-ref":
            raise SafetyError("retirement proof lacks reviewed retire-local-ref disposition")
        accountable_identity(raw.get("evidence_id"), "retirement proof evidence_id")
        if git(
            repo.root, "merge-base", "--is-ancestor", target_tip, baseline_sha, check=False
        ).returncode == 0:
            raise SafetyError("retirement proof is only valid for a non-baseline-ancestor tip")
        if proof_type == "stable_patch_coverage":
            if set(raw) != common | {"source_commits"}:
                raise SafetyError("stable-patch proof has unknown or missing fields")
            if patch_index is None:
                patch_index = baseline_patch_index(repo, baseline_sha)
            validate_stable_patch_proof(repo, raw, baseline_sha, patch_index)
        elif proof_type == "retained_local_ref":
            if set(raw) != common | {"relation", "retained_ref", "retained_tip"}:
                raise SafetyError("retained-local-ref proof has unknown or missing fields")
            retained_ref = raw.get("retained_ref")
            if retained_ref == target:
                raise SafetyError("retained local ref must differ from its retirement target")
            retained_tip = exact_ref_tip(repo, retained_ref, "refs/heads/")
            retained_owner = owners.get(str(retained_ref))
            if (
                retained_owner is None
                or (
                    retained_owner["owner"] == "none"
                    and retained_owner["release_role"] == "none"
                )
                or retained_ref in retiring_targets
            ):
                raise SafetyError("proof does not name one explicitly retained local ref")
            if raw.get("retained_tip") != retained_tip:
                raise SafetyError("retained local ref tip differs from live Git")
            relation = raw.get("relation")
            same_tip = retained_tip == target_tip
            ancestor = git(
                repo.root, "merge-base", "--is-ancestor", target_tip, retained_tip, check=False
            ).returncode == 0
            if relation not in {"same-tip", "ancestor"} or (
                relation == "same-tip" and not same_tip
            ) or (relation == "ancestor" and (same_tip or not ancestor)):
                raise SafetyError("retained local ref relation differs from live Git")
        elif proof_type == "remote_tracking_ref":
            if set(raw) != common | {"remote_ref", "remote_tip"}:
                raise SafetyError("remote-tracking proof has unknown or missing fields")
            remote_tip = exact_ref_tip(repo, raw.get("remote_ref"), "refs/remotes/")
            if raw.get("remote_tip") != remote_tip or remote_tip != target_tip:
                raise SafetyError("proof does not name an exact remote-tracking ref")
        else:
            raise SafetyError("retirement proof type is unsupported")
        proofs[target] = raw
    return proofs


def verify_retirement_proof_approval(
    value: dict[str, Any],
    repo: Repository,
    proof_hash: str,
    owner_hash: str,
    author: str,
) -> None:
    """Validate independent hash-bound semantic approval of one proof set."""
    required = {
        "schema", "repository", "owner_map_sha256", "retirement_proofs_sha256",
        "reviewer", "decision", "issued_at", "expires_at"
    }
    if set(value) != required:
        raise SafetyError("retirement proof approval has unknown or missing fields")
    valid_attestation(
        value,
        schema=RETIREMENT_PROOF_APPROVAL_SCHEMA,
        repo=repo,
        maximum_age=86400,
        label="retirement proof approval",
    )
    reviewer = accountable_identity(
        value.get("reviewer"), "retirement proof approval reviewer"
    )
    if (
        value.get("owner_map_sha256") != owner_hash
        or value.get("retirement_proofs_sha256") != proof_hash
        or value.get("decision") != "approved"
        or reviewer == author
    ):
        raise SafetyError("retirement proof approval is not independent or hash-bound")


def load_retirement_proof_chain(
    proof_path: Path,
    approval_path: Path,
    repo: Repository,
    baseline_value: dict[str, Any],
    owner_hash: str,
    owners: dict[str, dict[str, Any]],
    retiring_targets: set[str],
    live_targets: set[str] | None = None,
) -> tuple[dict[str, dict[str, Any]], str, str]:
    """Read, approve, and live-validate one retirement proof chain."""
    proof_value = read_json(proof_path, "retirement proofs")
    proof_hash = sha256(proof_path.read_bytes())
    approval_value = read_json(approval_path, "retirement proof approval")
    verify_retirement_proof_approval(
        approval_value,
        repo,
        proof_hash,
        owner_hash,
        accountable_identity(proof_value.get("author"), "retirement proof author"),
    )
    proofs = validate_retirement_proof_set(
        proof_value,
        repo,
        baseline_value,
        owner_hash,
        owners,
        retiring_targets,
        live_targets,
    )
    return proofs, proof_hash, sha256(approval_path.read_bytes())


def validate_entry(entry: object, owners: dict[str, dict[str, Any]], owner_hash: str) -> dict[str, Any]:
    """Validate one executable action and its explicit no-owner witness."""
    if not isinstance(entry, dict) or set(entry) != {"kind", "target", "classification", "owner", "action", "witness"}:
        raise SafetyError("manifest entry has unknown or missing fields")
    if entry["kind"] not in {"worktree", "branch"} or not isinstance(entry["target"], str):
        raise SafetyError("manifest entry kind or target is invalid")
    if entry["classification"] not in {"merged-retirable", "proof-retirable"}:
        raise SafetyError("only reviewed retirable entries are executable")
    if entry["classification"] == "proof-retirable" and entry["kind"] != "branch":
        raise SafetyError("proof-retirable entries must be local branches")
    expected_action = "remove_worktree" if entry["kind"] == "worktree" else "delete_local_ref"
    if entry["action"] != expected_action:
        raise SafetyError("manifest entry action does not match target kind")
    owner = entry["owner"]
    if not isinstance(owner, dict) or set(owner) != {"value", "release_role", "evidence_sha256"}:
        raise SafetyError("manifest entry owner witness is invalid")
    mapped = owners.get(entry["target"])
    if (
        mapped is None
        or owner["evidence_sha256"] != owner_hash
        or owner["value"] != mapped["owner"]
        or owner["release_role"] != mapped["release_role"]
        or owner["value"] != "none"
        or owner["release_role"] != "none"
    ):
        raise SafetyError("manifest entry ownership or release-role evidence is unsafe")
    witness = entry["witness"]
    required_witness = {
        "tip", "clean", "unused_by_retained_worktree", "recovery_requirement"
    }
    if entry["classification"] == "proof-retirable":
        required_witness |= {"proof_type", "proof_entry_sha256"}
    if not isinstance(witness, dict) or set(witness) != required_witness:
        raise SafetyError("manifest entry witness is invalid")
    if witness["clean"] is not True or witness["unused_by_retained_worktree"] is not True or witness["recovery_requirement"] != "execution_bundle":
        raise SafetyError("manifest entry witness is unsafe")
    if entry["classification"] == "proof-retirable" and (
        witness["proof_type"] not in {
            "stable_patch_coverage", "retained_local_ref", "remote_tracking_ref"
        }
        or not isinstance(witness["proof_entry_sha256"], str)
        or len(witness["proof_entry_sha256"]) != 64
    ):
        raise SafetyError("manifest entry retirement proof witness is invalid")
    return entry


def current_snapshot_matches(repo: Repository, manifest: dict[str, Any], owner_map_path: Path) -> dict[str, Any]:
    """Re-audit and require current core state to match an approved candidate."""
    snapshot = make_snapshot(repo, str(manifest["baseline"]["ref"]), str(owner_map_path))
    if snapshot["snapshot_id"] != manifest["snapshot_id"]:
        raise SafetyError("current snapshot differs from manifest")
    if snapshot["baseline"] != manifest["baseline"]:
        raise SafetyError("current baseline differs from manifest")
    return snapshot


def revalidate_manifest_retirement_proofs(
    args: argparse.Namespace,
    evidence: Path,
    repo: Repository,
    manifest: dict[str, Any],
    owner_hash: str,
    owners: dict[str, dict[str, Any]],
    live_targets: set[str] | None = None,
) -> tuple[str | None, str | None]:
    """Re-read the manifest-bound proof chain and recompute live relations."""
    chain = manifest["retirement_proofs"]
    proof_arg = getattr(args, "retirement_proofs", None)
    approval_arg = getattr(args, "retirement_proof_approval", None)
    if chain is None:
        if proof_arg is not None or approval_arg is not None:
            raise SafetyError("manifest does not bind retirement proof evidence")
        if any(
            isinstance(entry, dict) and entry.get("classification") == "proof-retirable"
            for entry in manifest["entries"]
        ):
            raise SafetyError("proof-retirable entry lacks a manifest proof chain")
        return None, None
    if proof_arg is None or approval_arg is None:
        raise SafetyError("manifest requires retirement proofs and their approval")
    proof_path = regular_child(proof_arg, evidence, "retirement proofs")
    approval_path = regular_child(
        approval_arg, evidence, "retirement proof approval"
    )
    retiring_targets = {
        entry["target"]
        for entry in manifest["entries"]
        if isinstance(entry, dict) and entry.get("kind") == "branch"
    }
    try:
        proofs, proof_hash, approval_hash = load_retirement_proof_chain(
            proof_path,
            approval_path,
            repo,
            manifest["baseline"],
            owner_hash,
            owners,
            retiring_targets,
            live_targets,
        )
    except SafetyError as exc:
        raise SafetyError(f"retirement proof live revalidation failed: {exc}") from exc
    if chain != {"proofs_sha256": proof_hash, "approval_sha256": approval_hash}:
        raise SafetyError("retirement proof evidence hashes differ from manifest")
    proof_entries = {
        entry["target"]: entry
        for entry in manifest["entries"]
        if isinstance(entry, dict) and entry.get("classification") == "proof-retirable"
    }
    if set(proof_entries) != set(proofs):
        raise SafetyError("manifest proof-backed targets differ from reviewed proofs")
    for target, proof in proofs.items():
        if live_targets is not None and target not in live_targets:
            continue
        witness = proof_entries[target].get("witness", {})
        if (
            witness.get("proof_type") != proof["proof_type"]
            or witness.get("proof_entry_sha256") != sha256(proof)
        ):
            raise SafetyError("manifest proof-entry hash or type mismatch")
    return proof_hash, approval_hash


def derived_bundle_name(manifest: dict[str, Any]) -> str:
    """Derive the non-recursive archive basename required by wtc-bundle-v1."""
    if manifest["preservation"].get("bundle_name_algorithm") != "wtc-bundle-v1":
        raise SafetyError("unsupported bundle name algorithm")
    return f"refs-before-wtc-{manifest['snapshot_id'][:8]}-{manifest['plan_sha256'][:8]}.bundle"


def command_dryrun(args: argparse.Namespace) -> int:
    """Rehearse one exact approved manifest without changing Git state."""
    repo = repository(args.repo)
    evidence = validate_private_directory(args.evidence_dir, repo, "evidence directory")
    manifest_path = regular_child(args.manifest, evidence, "manifest")
    approval_path = regular_child(args.approval_attestation, evidence, "approval attestation")
    baseline_path = regular_child(args.baseline_attestation, evidence, "baseline attestation")
    owner_path = regular_child(args.owner_map, evidence, "owner map")
    if not args.owner_map_review_attestation:
        raise ReviewRequired("owner_map_review_required: provide --owner-map-review-attestation")
    owner_review_path = regular_child(
        args.owner_map_review_attestation, evidence, "owner-map review attestation"
    )
    manifest = read_json(manifest_path, "manifest")
    verify_manifest(manifest, repo)
    manifest_hash = sha256(manifest_path.read_bytes())
    approval_hash = verify_approval(approval_path, manifest_hash, repo, manifest["approval_requirement"]["max_age_seconds"])
    baseline_hash = validate_baseline_attestation(baseline_path, repo, manifest, manifest["baseline_requirement"]["max_age_seconds"])
    owner_hash, owners = check_manifest_owner_map(owner_path, repo, manifest)
    owner_review_hash = verify_owner_map_review(owner_review_path, repo, owner_hash)
    if owner_review_hash != manifest["owner_map_review_sha256"]:
        raise SafetyError("owner-map review attestation hash mismatch")
    proof_hash, proof_approval_hash = revalidate_manifest_retirement_proofs(
        args, evidence, repo, manifest, owner_hash, owners
    )
    snapshot = current_snapshot_matches(repo, manifest, owner_path)
    archive = validate_private_directory(args.archive_dir, repo, "archive directory")
    bundle = archive / derived_bundle_name(manifest)
    if bundle.exists():
        raise SafetyError("final bundle already exists")
    seen: set[tuple[str, str]] = set()
    for entry in manifest["entries"]:
        valid = validate_entry(entry, owners, owner_hash)
        key = (valid["kind"], valid["target"])
        if key in seen:
            raise SafetyError("manifest has duplicate executable targets")
        seen.add(key)
        assert_entry_snapshot(snapshot, valid)
        assert_entry_live(repo, valid)
    predicted = {
        "worktree_count": len(snapshot["worktrees"])
        - sum(entry["kind"] == "worktree" for entry in manifest["entries"]),
        "local_ref_deletions": sum(entry["kind"] == "branch" for entry in manifest["entries"]),
    }
    issued = utcnow()
    receipt = {
        "schema": DRYRUN_SCHEMA,
        "repository": repo.identity,
        "manifest_sha256": manifest_hash,
        "approval_attestation_sha256": approval_hash,
        "baseline_attestation_sha256": baseline_hash,
        "owner_map_sha256": owner_hash,
        "owner_map_review_sha256": owner_review_hash,
        "retirement_proofs_sha256": proof_hash,
        "retirement_proof_approval_sha256": proof_approval_hash,
        "snapshot_id": snapshot["snapshot_id"],
        "archive": directory_attributes(archive),
        "evidence": directory_attributes(evidence),
        "bundle_path": str(bundle),
        "expected_actions": manifest["entries"],
        "predicted_post_state": predicted,
        "result": "success",
        "issued_at": rfc3339(issued),
        "expires_at": rfc3339(issued + dt.timedelta(seconds=manifest["dryrun_requirement"]["max_age_seconds"])),
    }
    receipt_path = evidence / f"dryrun-{manifest_hash[:16]}.json"
    durable_write(receipt_path, receipt)
    output(receipt, argparse.Namespace(output=None))
    return 0


def verify_dryrun(path: Path, manifest_hash: str, repo: Repository, maximum_age: int) -> dict[str, Any]:
    """Validate a successful hash-bound dry-run receipt."""
    value = read_json(path, "dry-run receipt")
    required = {
        "schema", "repository", "manifest_sha256", "approval_attestation_sha256",
        "baseline_attestation_sha256", "owner_map_sha256", "owner_map_review_sha256",
        "retirement_proofs_sha256", "retirement_proof_approval_sha256", "snapshot_id", "archive",
        "evidence", "bundle_path", "expected_actions", "predicted_post_state", "result",
        "issued_at", "expires_at",
    }
    if set(value) != required:
        raise SafetyError("dry-run receipt has unknown or missing fields")
    valid_attestation(value, schema=DRYRUN_SCHEMA, repo=repo, maximum_age=maximum_age, label="dry-run receipt")
    if value.get("manifest_sha256") != manifest_hash or value.get("result") != "success":
        raise SafetyError("dry-run receipt does not validate this manifest")
    return value


def verify_freeze(path: Path, manifest_hash: str, dryrun_hash: str, repo: Repository, requirement: dict[str, Any]) -> None:
    """Validate explicit human freeze evidence."""
    value = read_json(path, "freeze attestation")
    required = {
        "schema", "repository", "manifest_sha256", "dryrun_receipt_sha256", "snapshot_id",
        "operator", "writers", "issued_at", "expires_at",
    }
    if set(value) != required:
        raise SafetyError("freeze attestation has unknown or missing fields")
    valid_attestation(value, schema=FREEZE_SCHEMA, repo=repo, maximum_age=requirement["max_age_seconds"], label="freeze attestation")
    if value.get("manifest_sha256") != manifest_hash or value.get("dryrun_receipt_sha256") != dryrun_hash or value.get("snapshot_id") != requirement["snapshot_id"]:
        raise SafetyError("freeze attestation does not bind this rehearsal")


def assert_entry_live(repo: Repository, entry: dict[str, Any]) -> None:
    """Recheck one retirement target immediately before its action."""
    if entry["kind"] == "worktree":
        live = next((item for item in repository(repo.root).worktrees if item["path"] == entry["target"]), None)
        if live is None or live["primary"] or live["locked"] or live["detached"] or not live["clean"] or live["head"] != entry["witness"]["tip"]:
            raise SafetyError("worktree target witness changed or is unsafe")
    elif entry["kind"] == "branch":
        ref = entry["target"]
        tip = git(repo.root, "rev-parse", "--verify", "--quiet", ref, check=False).stdout.strip()
        if not tip or tip != entry["witness"]["tip"]:
            raise SafetyError("branch target witness changed")
        if any(item["branch"] == ref for item in repository(repo.root).worktrees):
            raise SafetyError("branch remains used by a worktree")
    else:
        raise SafetyError("manifest has invalid target kind")


def assert_entry_snapshot(snapshot: dict[str, Any], entry: dict[str, Any]) -> None:
    """Prove an entry is retirement-classified in the corresponding audit."""
    collection = snapshot["worktrees"] if entry["kind"] == "worktree" else snapshot["local_refs"]
    key = "path" if entry["kind"] == "worktree" else "ref"
    audited = next((item for item in collection if item[key] == entry["target"]), None)
    expected_class = (
        "integration-required"
        if entry["classification"] == "proof-retirable"
        else "merged-retirable"
    )
    if audited is None or audited["classification"] != expected_class:
        if entry["classification"] == "merged-retirable":
            raise SafetyError("target is not merged-retirable in the current audit")
        raise SafetyError("proof-backed target is not integration-required in the current audit")
    if entry["classification"] == "proof-retirable" and (
        audited.get("baseline_ancestor") is not False
        or audited.get("used_by_worktree") is not False
    ):
        raise SafetyError("proof-backed target is no longer an unused non-ancestor ref")
    tip = audited["head"] if entry["kind"] == "worktree" else audited["tip"]
    if tip != entry["witness"]["tip"]:
        raise SafetyError("target audit tip differs from manifest witness")


def unresolved_or_dirty_counts(snapshot: dict[str, Any]) -> tuple[int, int]:
    """Count unresolved classifications and non-clean worktrees for monotonic checks."""
    unresolved = sum(
        item["classification"] in {"unresolved", "archive-required", "integration-required"}
        for item in snapshot["worktrees"]
    ) + sum(
        item["classification"] in {"unresolved", "integration-required"}
        for item in snapshot["local_refs"]
    )
    dirty = sum(not item["clean"] for item in snapshot["worktrees"])
    return unresolved, dirty


def verify_post_state(
    repo: Repository,
    before: dict[str, Any],
    after: dict[str, Any],
    completed: list[dict[str, Any]],
    bundle: Path,
    bundle_hash: str,
    covered: list[str],
) -> None:
    """Require the exact completed retirements and a still-verifiable recovery bundle."""
    if hashlib.sha256(bundle.read_bytes()).hexdigest() != bundle_hash:
        raise SafetyError("preservation bundle hash changed after publication")
    verification = git(repo.root, "bundle", "verify", str(bundle))
    text = verification.stdout.lower() + verification.stderr.lower()
    if "requires" in text:
        raise SafetyError("preservation bundle gained prerequisites")
    verify_bundle_coverage(bundle, set(covered))
    removed_worktrees = {entry["target"] for entry in completed if entry["kind"] == "worktree"}
    removed_refs = {entry["target"] for entry in completed if entry["kind"] == "branch"}
    before_worktrees = {item["path"]: item for item in before["worktrees"]}
    after_worktrees = {item["path"]: item for item in after["worktrees"]}
    if set(after_worktrees) != set(before_worktrees) - removed_worktrees:
        raise SafetyError("post-state worktree set differs from completed actions")
    for target in set(before_worktrees) - removed_worktrees:
        old, new = before_worktrees[target], after_worktrees[target]
        if (old["head"], old["branch"], old["clean"]) != (new["head"], new["branch"], new["clean"]):
            raise SafetyError("a retained worktree changed during consolidation")
    before_refs = {item["ref"]: item["tip"] for item in before["local_refs"]}
    after_refs = {item["ref"]: item["tip"] for item in after["local_refs"]}
    if after_refs != {ref: tip for ref, tip in before_refs.items() if ref not in removed_refs}:
        raise SafetyError("post-state local refs differ from completed actions")
    before_counts = unresolved_or_dirty_counts(before)
    after_counts = unresolved_or_dirty_counts(after)
    if after_counts[0] > before_counts[0] or after_counts[1] > before_counts[1]:
        raise SafetyError("consolidation increased unresolved or dirty state")


def verify_bundle_coverage(bundle: Path, required: set[str]) -> None:
    """Prove every required commit can be recovered from a standalone bundle."""
    probe = Path(tempfile.mkdtemp(prefix=".wtc-coverage-", dir=bundle.parent))
    try:
        git(probe, "init", "--bare", "--quiet")
        git(probe, "bundle", "unbundle", str(bundle))
        missing = [
            tip
            for tip in sorted(required)
            if git(probe, "cat-file", "-e", f"{tip}^{{commit}}", check=False).returncode != 0
        ]
        if missing:
            raise SafetyError("bundle does not cover every required tip")
    finally:
        try:
            shutil.rmtree(probe)
        except OSError as exc:
            raise SafetyError("bundle coverage probe cleanup failed") from exc


def publish_bundle(
    repo: Repository, archive: Path, manifest: dict[str, Any]
) -> tuple[Path, str, list[str], list[str], str]:
    """Create, independently verify, and no-clobber publish a standalone bundle."""
    final = archive / derived_bundle_name(manifest)
    if final.exists():
        raise SafetyError("final bundle already exists")
    fd, raw_temp = tempfile.mkstemp(prefix=".wtc-", suffix=".bundle", dir=archive)
    temp = Path(raw_temp)
    os.close(fd)
    try:
        local_head_refs = [
            line.strip()
            for line in git(repo.root, "for-each-ref", "--format=%(refname)", "refs/heads").stdout.splitlines()
            if line.strip()
        ]
        revisions = sorted(set(local_head_refs) | set(manifest["preservation"]["reflog_candidates"]))
        git(repo.root, "bundle", "create", str(temp), *revisions)
        with temp.open("rb") as handle:
            os.fsync(handle.fileno())
        verification = git(repo.root, "bundle", "verify", str(temp))
        if "requires" in verification.stdout.lower() or "requires" in verification.stderr.lower():
            raise SafetyError("bundle has prerequisites")
        required = set(manifest["preservation"]["required_tips"]) | set(manifest["preservation"]["reflog_candidates"])
        verify_bundle_coverage(temp, required)
        digest = hashlib.sha256(temp.read_bytes()).hexdigest()
        try:
            os.link(temp, final)
        except FileExistsError as exc:
            raise SafetyError("final bundle already exists") from exc
        os.unlink(temp)
        directory_fd = os.open(archive, os.O_RDONLY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
        verification_text = (verification.stdout + verification.stderr).strip()
        return final, digest, sorted(required), revisions, verification_text
    except Exception:
        temp.unlink(missing_ok=True)
        raise


@contextlib.contextmanager
def consolidation_lock(repo: Repository) -> Iterable[None]:
    """Hold the cooperative common-directory lock for one retirement batch."""
    path = repo.common_dir / "worktree-consolidator.lock"
    try:
        fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError as exc:
        raise SafetyError("another consolidator instance holds the repository lock") from exc
    try:
        os.write(fd, f"pid={os.getpid()}\n".encode("ascii"))
        os.fsync(fd)
        yield
    finally:
        os.close(fd)
        path.unlink(missing_ok=True)


def command_consolidate(args: argparse.Namespace) -> int:
    """Execute one fully gated local retirement batch."""
    repo = repository(args.repo)
    evidence = validate_private_directory(args.evidence_dir, repo, "evidence directory")
    manifest_path = regular_child(args.manifest, evidence, "manifest")
    owner_path = regular_child(args.owner_map, evidence, "owner map")
    if not args.owner_map_review_attestation:
        raise ReviewRequired("owner_map_review_required: provide --owner-map-review-attestation")
    owner_review_path = regular_child(
        args.owner_map_review_attestation, evidence, "owner-map review attestation"
    )
    approval_path = regular_child(args.approval_attestation, evidence, "approval attestation")
    baseline_path = regular_child(args.baseline_attestation, evidence, "baseline attestation")
    dryrun_path = regular_child(args.dryrun_receipt, evidence, "dry-run receipt")
    freeze_path = regular_child(args.freeze_attestation, evidence, "freeze attestation")
    manifest = read_json(manifest_path, "manifest")
    verify_manifest(manifest, repo)
    manifest_hash = sha256(manifest_path.read_bytes())
    if args.confirm_manifest_sha256 != manifest_hash or args.confirm != f"CONSOLIDATE {manifest['manifest_id']}":
        raise SafetyError("manifest confirmation mismatch")
    approval_hash = verify_approval(approval_path, manifest_hash, repo, manifest["approval_requirement"]["max_age_seconds"])
    baseline_hash = validate_baseline_attestation(baseline_path, repo, manifest, manifest["baseline_requirement"]["max_age_seconds"])
    owner_hash, owners = check_manifest_owner_map(owner_path, repo, manifest)
    owner_review_hash = verify_owner_map_review(owner_review_path, repo, owner_hash)
    if owner_review_hash != manifest["owner_map_review_sha256"]:
        raise SafetyError("owner-map review attestation hash mismatch")
    proof_hash, proof_approval_hash = revalidate_manifest_retirement_proofs(
        args, evidence, repo, manifest, owner_hash, owners
    )
    dryrun = verify_dryrun(dryrun_path, manifest_hash, repo, manifest["dryrun_requirement"]["max_age_seconds"])
    dryrun_hash = sha256(dryrun_path.read_bytes())
    verify_freeze(freeze_path, manifest_hash, dryrun_hash, repo, manifest["freeze_requirement"])
    if dryrun.get("approval_attestation_sha256") != approval_hash or dryrun.get("baseline_attestation_sha256") != baseline_hash or dryrun.get("owner_map_sha256") != owner_hash or dryrun.get("owner_map_review_sha256") != owner_review_hash or dryrun.get("retirement_proofs_sha256") != proof_hash or dryrun.get("retirement_proof_approval_sha256") != proof_approval_hash:
        raise SafetyError("dry-run evidence chain mismatch")
    archive = validate_private_directory(args.archive_dir, repo, "archive directory")
    with consolidation_lock(repo):
        revalidate_manifest_retirement_proofs(
            args, evidence, repo, manifest, owner_hash, owners
        )
        snapshot = current_snapshot_matches(repo, manifest, owner_path)
        if dryrun.get("snapshot_id") != snapshot["snapshot_id"]:
            raise SafetyError("dry-run snapshot differs from current state")
        if dryrun.get("archive") != directory_attributes(archive) or dryrun.get("evidence") != directory_attributes(evidence):
            raise SafetyError("archive or evidence directory differs from dry run")
        if dryrun.get("bundle_path") != str(archive / derived_bundle_name(manifest)):
            raise SafetyError("bundle path differs from dry run")
        entries = [validate_entry(entry, owners, owner_hash) for entry in manifest["entries"]]
        if len({(entry["kind"], entry["target"]) for entry in entries}) != len(entries):
            raise SafetyError("manifest has duplicate executable targets")
        for entry in entries:
            assert_entry_snapshot(snapshot, entry)
            assert_entry_live(repo, entry)
        evidence_paths = execution_evidence_paths(evidence, manifest_hash, len(entries))
        reserve_execution_evidence(evidence_paths)
        bundle, bundle_hash, covered, bundle_inputs, bundle_verify = publish_bundle(
            repo, archive, manifest
        )
        preservation = {
            "schema": "fathomdb-worktree-preservation-receipt/v1",
            "repository": repo.identity,
            "manifest_sha256": manifest_hash,
            "bundle_path": str(bundle),
            "bundle_sha256": bundle_hash,
            "covered_tips": covered,
            "bundle_inputs": bundle_inputs,
            "bundle_verify": bundle_verify,
            "verified_before_retirement": True,
            "issued_at": rfc3339(utcnow()),
        }
        preservation_path = evidence_paths[0]
        durable_write(preservation_path, preservation)
        preservation_hash = hashlib.sha256(preservation_path.read_bytes()).hexdigest()
        completed: list[dict[str, Any]] = []
        failure: Exception | None = None
        for entry in entries:
            try:
                if entry["classification"] == "proof-retirable":
                    revalidate_manifest_retirement_proofs(
                        args,
                        evidence,
                        repo,
                        manifest,
                        owner_hash,
                        owners,
                        {entry["target"]},
                    )
                assert_entry_snapshot(
                    make_snapshot(repo, str(manifest["baseline"]["ref"]), str(owner_path)), entry
                )
                assert_entry_live(repo, entry)
                if entry["action"] == "remove_worktree":
                    git(repo.root, "worktree", "remove", entry["target"])
                else:
                    git(repo.root, "update-ref", "-d", entry["target"], entry["witness"]["tip"])
                completed.append(entry)
                durable_write(
                    evidence_paths[1 + len(completed)],
                    {
                        "schema": "fathomdb-worktree-progress-receipt/v1",
                        "repository": repo.identity,
                        "manifest_sha256": manifest_hash,
                        "preservation_receipt_sha256": preservation_hash,
                        "completed_actions": completed,
                        "issued_at": rfc3339(utcnow()),
                    },
                )
            except Exception as exc:
                failure = exc
                break
        try:
            post = make_snapshot(repo, str(manifest["baseline"]["ref"]), str(owner_path))
            verify_post_state(repo, snapshot, post, completed, bundle, bundle_hash, covered)
        except Exception as exc:
            failure = failure or exc
        receipt = {
            "schema": "fathomdb-worktree-execution-receipt/v1",
            "repository": repo.identity,
            "manifest_sha256": manifest_hash,
            "preservation_receipt_sha256": preservation_hash,
            "bundle_path": str(bundle),
            "bundle_sha256": bundle_hash,
            "covered_tips": covered,
            "completed_actions": completed,
            "post_snapshot_id": post["snapshot_id"] if "post" in locals() else None,
            "result": "success" if failure is None else "partial",
            "issued_at": rfc3339(utcnow()),
        }
        path = evidence_paths[1]
        try:
            durable_write(path, receipt)
        except Exception as exc:
            try:
                write_partial_fallback(evidence, receipt, exc)
            except PartialBatch as fallback:
                raise PartialBatch(
                    "execution receipt persistence failed; "
                    f"preservation={preservation_path}; completed={len(completed)}; {fallback}"
                ) from exc
            raise PartialBatch(f"execution receipt persistence failed: {exc}") from exc
        if failure is not None:
            raise PartialBatch(str(failure))
    output({"result": "success", "snapshot_id": post["snapshot_id"]}, argparse.Namespace(output=None))
    return 0


def parser() -> argparse.ArgumentParser:
    """Build the public authority-separated CLI."""
    root = argparse.ArgumentParser(description=__doc__)
    sub = root.add_subparsers(dest="mode", required=True)

    def common(command: argparse.ArgumentParser, *, owner: bool = False) -> None:
        command.add_argument("--repo", default=".")
        command.add_argument("--baseline", default="origin/main")
        if owner:
            command.add_argument("--owner-map", required=True)
        command.add_argument("--json", action="store_true")

    audit = sub.add_parser("audit", help="observe and classify without mutation")
    common(audit)
    audit.add_argument("--owner-map")
    audit.set_defaults(handler=command_audit)

    manifest = sub.add_parser("manifest", help="generate a candidate manifest")
    common(manifest, owner=True)
    manifest.add_argument("--audit", required=True)
    manifest.add_argument("--policy", required=True)
    manifest.add_argument("--owner-map-review-attestation")
    manifest.add_argument("--baseline-attestation", required=True)
    manifest.add_argument("--retirement-proofs")
    manifest.add_argument("--retirement-proof-approval")
    manifest.add_argument("--evidence-dir", required=True)
    group = manifest.add_mutually_exclusive_group(required=True)
    group.add_argument("--target-worktrees", type=int)
    group.add_argument("--infer-target", action="store_true")
    manifest.add_argument("--output")
    manifest.set_defaults(handler=command_manifest)

    dryrun = sub.add_parser("dryrun", help="rehearse an approved manifest without cleanup")
    common(dryrun, owner=True)
    dryrun.add_argument("--manifest", required=True)
    dryrun.add_argument("--approval-attestation", required=True)
    dryrun.add_argument("--owner-map-review-attestation")
    dryrun.add_argument("--baseline-attestation", required=True)
    dryrun.add_argument("--retirement-proofs")
    dryrun.add_argument("--retirement-proof-approval")
    dryrun.add_argument("--archive-dir", required=True)
    dryrun.add_argument("--evidence-dir", required=True)
    dryrun.set_defaults(handler=command_dryrun)

    consolidate = sub.add_parser("consolidate", help="apply a confirmed manifest")
    common(consolidate, owner=True)
    consolidate.add_argument("--manifest", required=True)
    consolidate.add_argument("--approval-attestation", required=True)
    consolidate.add_argument("--owner-map-review-attestation")
    consolidate.add_argument("--baseline-attestation", required=True)
    consolidate.add_argument("--retirement-proofs")
    consolidate.add_argument("--retirement-proof-approval")
    consolidate.add_argument("--dryrun-receipt", required=True)
    consolidate.add_argument("--freeze-attestation", required=True)
    consolidate.add_argument("--archive-dir", required=True)
    consolidate.add_argument("--evidence-dir", required=True)
    consolidate.add_argument("--confirm-manifest-sha256", required=True)
    consolidate.add_argument("--confirm", required=True)
    consolidate.set_defaults(handler=command_consolidate)

    status = sub.add_parser("status", help="observe durable execution state without mutation")
    status.add_argument("--repo", default=".")
    status.add_argument("--manifest", required=True)
    status.add_argument("--evidence-dir", required=True)
    status.add_argument("--json", action="store_true")
    status.set_defaults(handler=command_status)
    return root


def main(argv: list[str] | None = None) -> int:
    """Run the command and map safety failures to documented status codes."""
    args = parser().parse_args(argv)
    try:
        return args.handler(args)
    except ReviewRequired as exc:
        print(json.dumps({"result": "owner_map_review_required", "error": str(exc)}, separators=(",", ":")))
        return 3
    except GoalBlocked as exc:
        print(json.dumps({"result": "goal_inference_blocked", "error": str(exc)}, separators=(",", ":")))
        return 3
    except PartialBatch as exc:
        print(f"partial consolidation: {exc}", file=sys.stderr)
        return 4
    except SafetyError as exc:
        print(f"safety error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
