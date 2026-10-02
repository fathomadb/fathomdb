#!/usr/bin/env python3
"""Guard the process-wide SQLite configuration owner after the Slice 90 move."""

from pathlib import Path
import re
import unittest


ENGINE_SRC = Path(__file__).resolve().parents[2] / "src/rust/crates/fathomdb-engine/src"
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

    def test_old_location_and_missing_owner_are_rejected(self) -> None:
        declaration = "pub enum RuntimeSqliteMode { Performance }\n"
        self.assertTrue(owner_errors(declaration, ""))
        self.assertTrue(owner_errors("", ""))

    def test_duplicate_owner_is_rejected(self) -> None:
        declaration = "pub enum RuntimeSqliteMode { Performance }\n"
        errors = owner_errors("", declaration * 2)
        self.assertTrue(any("exactly one" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
