#!/usr/bin/env python3
"""Guard the process-wide SQLite configuration owner after the Slice 90 move."""

from pathlib import Path
import re
import unittest


ENGINE_SRC = Path(__file__).resolve().parents[2] / "src/rust/crates/fathomdb-engine/src"
REPO_ROOT = ENGINE_SRC.parents[4]
OWNED_DECLARATIONS = (
    r"pub enum RuntimeSqliteMode\b",
    r"pub struct RuntimeConfiguration\b",
    r"pub enum RuntimeConfigurationError\b",
    r"enum RuntimeState\b",
    r"static SQLITE_RUNTIME_STATE\b",
    r"pub fn configure_runtime\b",
    r"pub\(crate\) fn configure_runtime_for_open\b",
    r"fn configure_runtime_locked\b",
    r"pub\(crate\) fn effective_runtime_configuration\b",
)
COMPLETE_OWNER = """\
pub enum RuntimeSqliteMode { Performance }
pub struct RuntimeConfiguration;
pub enum RuntimeConfigurationError { TooLate }
enum RuntimeState { Unconfigured }
static SQLITE_RUNTIME_STATE: usize = 0;
pub fn configure_runtime() {}
pub(crate) fn configure_runtime_for_open() {}
fn configure_runtime_locked() {}
pub(crate) fn effective_runtime_configuration() {}
"""


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    for declaration in OWNED_DECLARATIONS:
        pattern = re.compile(r"^" + declaration, re.MULTILINE)
        if pattern.search(root):
            errors.append(f"root still defines {declaration}")
        if len(pattern.findall(owner)) != 1:
            errors.append(f"owner does not define exactly one {declaration}")
    return errors


class RuntimeConfigurationOwnerTest(unittest.TestCase):
    def test_current_source_has_one_semantic_owner(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner = (ENGINE_SRC / "runtime_configuration.rs").read_text()
        self.assertEqual(owner_errors(root, owner), [])

    def test_old_location_is_rejected_with_a_complete_owner(self) -> None:
        declaration = "pub enum RuntimeSqliteMode { Performance }\n"
        self.assertEqual(owner_errors("", COMPLETE_OWNER), [])
        self.assertEqual(
            owner_errors(declaration, COMPLETE_OWNER),
            [r"root still defines pub enum RuntimeSqliteMode\b"],
        )

    def test_missing_owner_is_rejected(self) -> None:
        missing = COMPLETE_OWNER.replace("pub fn configure_runtime() {}\n", "")
        self.assertEqual(
            owner_errors("", missing),
            [r"owner does not define exactly one pub fn configure_runtime\b"],
        )

    def test_duplicate_owner_is_rejected(self) -> None:
        declaration = "pub enum RuntimeSqliteMode { Performance }\n"
        self.assertEqual(
            owner_errors("", COMPLETE_OWNER + declaration),
            [r"owner does not define exactly one pub enum RuntimeSqliteMode\b"],
        )

    def test_fast_tier_runs_this_guard(self) -> None:
        registration = (
            "run_tier_suite fast test-slice90-runtime-configuration-owner "
            "python3 scripts/tests/test_slice90_runtime_configuration_owner.py"
        )
        self.assertIn(registration, (REPO_ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
