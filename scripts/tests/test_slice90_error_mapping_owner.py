#!/usr/bin/env python3
"""Guard the open and migration error conversion owner."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
NAMES = ("map_open_sqlite_error", "map_migration_error")


def attrs(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "#[", "//")):
            break
    return found


def owner_errors(root: str, owner: str, open_source: str, operator: str) -> list[str]:
    errors = []
    for name in re.findall(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+((?:map_open_sqlite_error|map_migration_error)\w*)\s*\(",
        root,
        re.M,
    ):
        errors.append(f"root still defines errors::{name}")
    module = re.search(r"^mod errors;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root lacks ungated errors module")
    for name in NAMES:
        found = list(re.finditer(r"^pub\(crate\) fn " + name + r"\(", owner, re.M))
        if len(found) != 1:
            errors.append(f"errors lacks one {name}")
        elif attrs(owner, found[0].start()):
            errors.append(f"errors gates {name}")
    if "use fathomdb_schema::MigrationError as SchemaMigrationError;" not in owner:
        errors.append("errors lacks direct migration error type")
    if "MigrationError as SchemaMigrationError" in root:
        errors.append("root still imports migration error type")
    if "use crate::errors::{map_migration_error, map_open_sqlite_error};" not in open_source:
        errors.append("open lacks error mapper owner paths")
    if "use crate::errors::map_open_sqlite_error;" not in operator:
        errors.append("operator lacks error mapper owner path")
    return errors


class ErrorMappingOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "errors.rs", "open.rs", "operator/data_plane.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner, open_source, operator = self.sources()
        # Complete the expected owner so each root mutant stands alone in RED.
        for name in NAMES:
            if not re.search(r"^pub\(crate\) fn " + name + r"\(", owner, re.M):
                owner += f"\npub(crate) fn {name}() {{}}"
        if "use fathomdb_schema::MigrationError as SchemaMigrationError;" not in owner:
            owner += "\nuse fathomdb_schema::MigrationError as SchemaMigrationError;"
        for name in NAMES:
            root = re.sub(r"(?ms)^fn " + name + r"\(.*?^}\n", "", root, count=1)
        root = root.replace("MigrationError as SchemaMigrationError, ", "")
        if "use crate::errors::{map_migration_error, map_open_sqlite_error};" not in open_source:
            open_source += "\nuse crate::errors::{map_migration_error, map_open_sqlite_error};"
        if "use crate::errors::map_open_sqlite_error;" not in operator:
            operator += "\nuse crate::errors::map_open_sqlite_error;"
        self.assertEqual(owner_errors(root, owner, open_source, operator), [])
        for declaration, name in (
            ("fn map_open_sqlite_error_new() {}", "map_open_sqlite_error_new"),
            ("pub(crate) fn map_migration_error_new() {}", "map_migration_error_new"),
            ("pub fn map_open_sqlite_error_future() {}", "map_open_sqlite_error_future"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(
                    f"root still defines errors::{name}",
                    owner_errors(root + "\n" + declaration, owner, open_source, operator),
                )

    def test_cfg_and_cfg_attr_mutants(self) -> None:
        root, owner, open_source, operator = self.sources()
        self.assertEqual(owner_errors(root, owner, open_source, operator), [])
        for name in NAMES:
            marker = f"pub(crate) fn {name}("
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                self.assertNotEqual(altered, owner)
                self.assertIn(f"errors gates {name}", owner_errors(root, altered, open_source, operator))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = root.replace("mod errors;", attr + "\nmod errors;", 1)
            self.assertIn("root lacks ungated errors module", owner_errors(altered, owner, open_source, operator))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-error-mapping-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
