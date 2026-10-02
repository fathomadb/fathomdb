#!/usr/bin/env python3
"""Guard ownership of open startup probes and compatibility repair."""

from pathlib import Path
import re
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
ENGINE_SRC = REPO_ROOT / "src/rust/crates/fathomdb-engine/src"
FUNCTIONS = (
    "edge_vector_prune_complete",
    "prune_orphaned_edge_vectors",
    "probe_open_integrity",
    "probe_database_header",
    "classify_wal_sidecar",
    "probe_wal_sidecar",
    "reject_legacy_shape",
    "validate_dependency_generation_on_open",
    "table_exists",
)


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    declaration = r"^(?:(?:pub(?:\([^)]*\))?) )?"
    root_functions = re.compile(
        declaration + r"fn ((?:probe_(?:open|database|wal)\w*|edge_vector_prune\w*|"
        r"prune_orphaned_edge_vector\w*))\(",
        re.M,
    )
    for match in root_functions.finditer(root):
        errors.append(f"root still defines open startup::{match.group(1)}")

    for name in FUNCTIONS:
        pattern = re.compile(declaration + r"fn " + name + r"\(", re.M)
        if (
            pattern.search(root)
            and f"root still defines open startup::{name}" not in errors
        ):
            errors.append(f"root still defines open startup::{name}")
        if len(pattern.findall(owner)) != 1:
            errors.append(f"open owner does not define one {name}")
    for kind, name in (
        ("const", "EDGE_VECTOR_PRUNE_MARKER_KEY"),
        ("enum", "WalSidecarHeader"),
    ):
        pattern = re.compile(declaration + kind + " " + name + r"\b", re.M)
        if pattern.search(root):
            errors.append(f"root still defines open startup::{name}")
        if len(pattern.findall(owner)) != 1:
            errors.append(f"open owner does not define one {name}")
    return errors


class OpenStartupOwnerTest(unittest.TestCase):
    def test_current_source_has_one_open_startup_owner(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors(root, owner), [])

    def test_old_root_location_fails_with_complete_owner(self) -> None:
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors("", owner), [])
        self.assertEqual(
            owner_errors("fn probe_open_integrity() {}", owner),
            ["root still defines open startup::probe_open_integrity"],
        )
        self.assertEqual(
            owner_errors("fn reject_legacy_shape() {}", owner),
            ["root still defines open startup::reject_legacy_shape"],
        )

    def test_unlisted_root_probe_fails_with_complete_owner(self) -> None:
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors("", owner), [])
        self.assertEqual(
            owner_errors(
                """#[cfg(feature = "operator")]
pub(crate) fn probe_open_new() {}""",
                owner,
            ),
            ["root still defines open startup::probe_open_new"],
        )

    def test_missing_owner_item_fails(self) -> None:
        owner = (ENGINE_SRC / "open.rs").read_text()
        missing = owner.replace(
            "fn prune_orphaned_edge_vectors(", "fn removed_prune(", 1
        )
        self.assertIn(
            "open owner does not define one prune_orphaned_edge_vectors",
            owner_errors("", missing),
        )

    def test_fast_tier_runs_this_guard(self) -> None:
        registration = (
            "run_tier_suite fast test-slice90-open-startup-owner "
            "python3 scripts/tests/test_slice90_open_startup_owner.py"
        )
        self.assertIn(registration, (REPO_ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
