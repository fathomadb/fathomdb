#!/usr/bin/env python3
"""Validate structured, candidate-bound runtime checkpoints in release state."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
HASH_RE = re.compile(r"^[0-9a-f]{64}$")
CHECKPOINT_KEYS = {
    "status",
    "candidate_sha",
    "binding_sha",
    "stage3_start_sha",
    "receipts",
}
RECEIPT_KEYS = {"path", "status", "candidate_sha", "sha256"}
RECEIPT_NAMES = {"performance", "code_review", "verification"}


class Validation:
    def __init__(self, root: Path) -> None:
        self.root = root.resolve()
        self.errors: list[str] = []
        self.checked = 0

    def fail(self, location: str, message: str) -> None:
        self.errors.append(f"FAIL check-runtime-checkpoints: {location}: {message}")

    def commit_exists(self, sha: str) -> bool:
        result = subprocess.run(
            ["git", "-C", str(self.root), "cat-file", "-e", f"{sha}^{{commit}}"],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return result.returncode == 0

    def is_ancestor(self, older: str, newer: str) -> bool:
        result = subprocess.run(
            ["git", "-C", str(self.root), "merge-base", "--is-ancestor", older, newer],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return result.returncode == 0

    def safe_path(self, location: str, raw: object) -> Path | None:
        if not isinstance(raw, str) or not raw:
            self.fail(location, "receipt path must be a non-empty repository-relative string")
            return None
        relative = Path(raw)
        if relative.is_absolute() or ".." in relative.parts:
            self.fail(location, f"receipt path escapes the repository: {raw!r}")
            return None
        resolved = (self.root / relative).resolve()
        try:
            resolved.relative_to(self.root)
        except ValueError:
            self.fail(location, f"receipt path escapes the repository: {raw!r}")
            return None
        return resolved

    def validate_receipt(
        self,
        location: str,
        name: str,
        receipt: object,
        checkpoint_status: str,
        candidate_sha: object,
    ) -> None:
        if not isinstance(receipt, dict):
            self.fail(location, f"{name} receipt must be an object")
            return
        unknown = set(receipt) ^ RECEIPT_KEYS
        if unknown:
            self.fail(location, f"{name} receipt keys must be exactly {sorted(RECEIPT_KEYS)}; delta={sorted(unknown)}")
            return

        receipt_location = f"{location}.{name}"
        path = self.safe_path(receipt_location, receipt["path"])
        if checkpoint_status == "PENDING":
            if receipt["status"] != "PENDING":
                self.fail(receipt_location, "pending checkpoint requires receipt status PENDING")
            for field in ("candidate_sha", "sha256"):
                if receipt[field] is not None:
                    self.fail(receipt_location, f"pending receipt requires {field}=null")
            return

        if receipt["status"] != "PASS":
            self.fail(receipt_location, "PASS checkpoint requires receipt status PASS")
        if receipt["candidate_sha"] != candidate_sha:
            self.fail(
                receipt_location,
                f"{name} receipt candidate_sha {receipt['candidate_sha']!r} does not equal checkpoint candidate {candidate_sha!r}",
            )
        digest = receipt["sha256"]
        if not isinstance(digest, str) or HASH_RE.fullmatch(digest) is None:
            self.fail(receipt_location, "PASS receipt sha256 must be 64 lowercase hex characters")
        if path is None:
            return
        if not path.is_file():
            self.fail(receipt_location, f"receipt file does not exist: {receipt['path']}")
            return
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != actual:
            self.fail(
                receipt_location,
                f"{name} receipt sha256 mismatch: state={digest!r} actual={actual!r}",
            )

    def validate_checkpoint(self, state_path: Path, entry: dict[str, Any]) -> None:
        checkpoint = entry["runtime_checkpoint"]
        location = f"{state_path.relative_to(self.root)} slice {entry.get('slice', '?')} runtime_checkpoint"
        self.checked += 1
        if not isinstance(checkpoint, dict):
            self.fail(location, "must be an object")
            return
        delta = set(checkpoint) ^ CHECKPOINT_KEYS
        if delta:
            self.fail(location, f"keys must be exactly {sorted(CHECKPOINT_KEYS)}; delta={sorted(delta)}")
            return

        status = checkpoint["status"]
        if status not in {"PENDING", "PASS"}:
            self.fail(location, "status must be PENDING or PASS")
            return
        candidate_sha = checkpoint["candidate_sha"]
        binding_sha = checkpoint["binding_sha"]
        stage3_sha = checkpoint["stage3_start_sha"]
        if status == "PENDING":
            for field, value in (
                ("candidate_sha", candidate_sha),
                ("binding_sha", binding_sha),
            ):
                if value is not None:
                    self.fail(location, f"PENDING checkpoint requires {field}=null")
            if stage3_sha is not None:
                self.fail(location, "stage3_start_sha requires a PASS checkpoint")
        else:
            for field, value in (("candidate_sha", candidate_sha), ("binding_sha", binding_sha)):
                if not isinstance(value, str) or SHA_RE.fullmatch(value) is None:
                    self.fail(location, f"PASS checkpoint requires {field} as 40 lowercase hex characters")

        receipts = checkpoint["receipts"]
        if not isinstance(receipts, dict):
            self.fail(location, "receipts must be an object")
        else:
            delta = set(receipts) ^ RECEIPT_NAMES
            if delta:
                self.fail(location, f"receipt names must be exactly {sorted(RECEIPT_NAMES)}; delta={sorted(delta)}")
            for name in sorted(RECEIPT_NAMES & set(receipts)):
                self.validate_receipt(location, name, receipts[name], status, candidate_sha)

        checkpoint_commits_exist = False
        if status == "PASS" and isinstance(candidate_sha, str) and isinstance(binding_sha, str):
            checkpoint_commits_exist = True
            for field, sha in (("candidate_sha", candidate_sha), ("binding_sha", binding_sha)):
                if not self.commit_exists(sha):
                    checkpoint_commits_exist = False
                    self.fail(location, f"{field} does not resolve to a commit: {sha}")
            if checkpoint_commits_exist and not self.is_ancestor(candidate_sha, binding_sha):
                self.fail(location, "checkpoint candidate must be an ancestor of its binding commit")

        if stage3_sha is None:
            return
        if not isinstance(stage3_sha, str) or SHA_RE.fullmatch(stage3_sha) is None:
            self.fail(location, "stage3_start_sha must be null or 40 lowercase hex characters")
            return
        if status != "PASS" or not isinstance(candidate_sha, str) or not isinstance(binding_sha, str):
            return
        stage3_exists = self.commit_exists(stage3_sha)
        if not stage3_exists:
            self.fail(location, f"stage3_start_sha does not resolve to a commit: {stage3_sha}")
        if checkpoint_commits_exist and stage3_exists and not self.is_ancestor(binding_sha, stage3_sha):
            self.fail(location, "checkpoint binding commit must be an ancestor of stage3_start_sha")

    def run(self) -> int:
        state_paths = sorted((self.root / "dev" / "plans").glob("release-state-*.json"))
        if not state_paths:
            self.fail("dev/plans", "no release-state files found")
        for state_path in state_paths:
            try:
                state = json.loads(state_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as error:
                self.fail(str(state_path.relative_to(self.root)), f"cannot parse JSON: {error}")
                continue
            ladder = state.get("ladder")
            if not isinstance(ladder, list):
                self.fail(str(state_path.relative_to(self.root)), "ladder must be an array")
                continue
            for entry in ladder:
                if isinstance(entry, dict) and "runtime_checkpoint" in entry:
                    self.validate_checkpoint(state_path, entry)
        if self.checked == 0:
            self.fail("dev/plans", "zero structured checkpoints discovered")
        if self.errors:
            print("\n".join(self.errors), file=sys.stderr)
            return 1
        print(f"ok    check-runtime-checkpoints: {self.checked} structured checkpoint(s) valid")
        return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    return Validation(args.root).run()


if __name__ == "__main__":
    raise SystemExit(main())
