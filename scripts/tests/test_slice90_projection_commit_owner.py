#!/usr/bin/env python3
"""Guard projection commit batching and trigger-safety ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"


def gated(source: str, start: int) -> bool:
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            return True
        if stripped and not stripped.startswith(("///", "#[")):
            break
    return False


def owner_errors(root: str, owner: str, worker: str, configuration: str) -> list[str]:
    errors = []
    module = re.search(r"^mod projection_commit;", root, re.M)
    if not module:
        errors.append("root lacks ungated commit module")
    elif gated(root, module.start()):
        errors.append("root gates commit module")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?const\s+(PROJECTION_COMMIT_BATCH\w*)\b", root, re.M):
        errors.append(f"root still defines commit::{name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?fn\s+(projection_batch_has_no_custom_triggers\w*)\s*\(", root, re.M):
        errors.append(f"root still defines commit helper {name}")
    constant = list(re.finditer(r"^pub\(crate\) const PROJECTION_COMMIT_BATCH\b", owner, re.M))
    if len(constant) != 1:
        errors.append("commit lacks one PROJECTION_COMMIT_BATCH")
    elif gated(owner, constant[0].start()):
        errors.append("commit gates always-on PROJECTION_COMMIT_BATCH")
    helper = list(re.finditer(r"^fn projection_batch_has_no_custom_triggers\(", owner, re.M))
    if len(helper) != 1:
        errors.append("commit lacks one trigger-safety helper")
    elif gated(owner, helper[0].start()):
        errors.append("commit gates always-on trigger-safety helper")
    if "use crate::projection_commit::PROJECTION_COMMIT_BATCH;" not in worker:
        errors.append("worker lacks commit-owner batch import")
    if "use super::projection_commit::PROJECTION_COMMIT_BATCH;" not in configuration:
        errors.append("configuration lacks commit-owner batch import")
    return errors


class ProjectionCommitOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "projection_commit.rs", "projection_worker.rs", "runtime_configuration.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner, worker, configuration = self.sources()
        self.assertEqual(owner_errors(root, owner, worker, configuration), [])
        for declaration, expected in (
            ("const PROJECTION_COMMIT_BATCH_NEW: usize = 1;", "root still defines commit::PROJECTION_COMMIT_BATCH_NEW"),
            ("pub(crate) const PROJECTION_COMMIT_BATCH_NEW: usize = 1;", "root still defines commit::PROJECTION_COMMIT_BATCH_NEW"),
            ("fn projection_batch_has_no_custom_triggers_new() {}", "root still defines commit helper projection_batch_has_no_custom_triggers_new"),
            ("pub(crate) fn projection_batch_has_no_custom_triggers_new() {}", "root still defines commit helper projection_batch_has_no_custom_triggers_new"),
            ("pub fn projection_batch_has_no_custom_triggers_new() {}", "root still defines commit helper projection_batch_has_no_custom_triggers_new"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner, worker, configuration))

    def test_cfg_and_import_mutants(self) -> None:
        root, owner, worker, configuration = self.sources()
        self.assertEqual(owner_errors(root, owner, worker, configuration), [])
        for marker, expected in (
            ("pub(crate) const PROJECTION_COMMIT_BATCH", "commit gates always-on PROJECTION_COMMIT_BATCH"),
            ("fn projection_batch_has_no_custom_triggers(", "commit gates always-on trigger-safety helper"),
        ):
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                with self.subTest(marker=marker, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(expected, owner_errors(root, altered, worker, configuration))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = root.replace("mod projection_commit;", attr + "\nmod projection_commit;", 1)
            self.assertIn("root gates commit module", owner_errors(altered, owner, worker, configuration))
        self.assertIn("worker lacks commit-owner batch import", owner_errors(root, owner, worker.replace("use crate::projection_commit::PROJECTION_COMMIT_BATCH;", "use crate::PROJECTION_COMMIT_BATCH;"), configuration))
        self.assertIn("configuration lacks commit-owner batch import", owner_errors(root, owner, worker, configuration.replace("use super::projection_commit::PROJECTION_COMMIT_BATCH;", "use super::PROJECTION_COMMIT_BATCH;")))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-projection-commit-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
