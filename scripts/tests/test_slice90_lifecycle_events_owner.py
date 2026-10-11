#!/usr/bin/env python3
"""Guard lifecycle event dispatch and SQLite diagnostic ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
METHODS = ("detect_slow", "emit_event", "emit_sqlite_internal_error", "subscribe")
HELPERS = ("sqlite_extended_code_name", "sqlite_extended_code_name_from_int", "emit_open_error_event")
ROOT_FAMILY = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+"
    r"((?:detect_slow|emit_event|emit_sqlite_internal_error|subscribe|"
    r"sqlite_extended_code_name|emit_open_error_event)\w*)\s*\(",
    re.M,
)


def attrs(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "#[", "//")):
            break
    return found


def owner_errors(root: str, owner: str, open_source: str, worker: str) -> list[str]:
    errors = [f"root still defines lifecycle::{name}" for name in ROOT_FAMILY.findall(root)]
    module = re.search(r"^pub mod lifecycle;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root lacks ungated lifecycle module")
    engine_impls = list(re.finditer(r"^impl Engine \{", owner, re.M))
    if not engine_impls:
        errors.append("lifecycle lacks Engine impl")
    elif any(attrs(owner, impl.start()) for impl in engine_impls):
        errors.append("lifecycle gates Engine impl")
    for name in METHODS:
        visibility = "pub" if name == "subscribe" else "pub(crate)"
        found = list(re.finditer(r"^    " + re.escape(visibility) + r" fn " + name + r"\(", owner, re.M))
        if len(found) != 1:
            errors.append(f"lifecycle lacks one Engine::{name}")
        elif attrs(owner, found[0].start()):
            errors.append(f"lifecycle gates Engine::{name}")
    for name in HELPERS:
        visibility = "fn" if name == "sqlite_extended_code_name_from_int" else "pub(crate) fn"
        found = list(re.finditer(r"^" + re.escape(visibility) + r" " + name + r"\(", owner, re.M))
        if len(found) != 1:
            errors.append(f"lifecycle lacks one {name}")
        elif attrs(owner, found[0].start()):
            errors.append(f"lifecycle gates {name}")
    if "use crate::lifecycle::emit_open_error_event;" not in open_source:
        errors.append("open lacks lifecycle event owner path")
    if "use crate::lifecycle::sqlite_extended_code_name;" not in worker:
        errors.append("worker lacks lifecycle SQLite owner path")
    return errors


class LifecycleEventsOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "lifecycle.rs", "open.rs", "projection_worker.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_families(self) -> None:
        root, owner, open_source, worker = self.sources()
        # Make the owner complete so every mutant probes root detection alone.
        if owner_errors(root, owner, open_source, worker):
            owner += "\n" + "\n".join(
                f"    {'pub' if name == 'subscribe' else 'pub(crate)'} fn {name}(&self) {{}}"
                for name in METHODS
            )
            owner += "\n" + "\n".join(
                f"{'fn' if name == 'sqlite_extended_code_name_from_int' else 'pub(crate) fn'} {name}() {{}}"
                for name in HELPERS
            )
            open_source += "\nuse crate::lifecycle::emit_open_error_event;"
            worker += "\nuse crate::lifecycle::sqlite_extended_code_name;"
        for declaration, expected in (
            ("impl Engine {\n    fn detect_slow_new(&self) {}\n}", "detect_slow_new"),
            ("impl Engine {\n    pub(crate) fn emit_event_new(&self) {}\n}", "emit_event_new"),
            ("impl Engine {\n    pub fn emit_sqlite_internal_error_new(&self) {}\n}", "emit_sqlite_internal_error_new"),
            ("impl Engine {\n    fn subscribe_new(&self) {}\n}", "subscribe_new"),
            ("impl LifecycleExtension for Engine {\n    fn emit_event_trait_new(&self) {}\n}", "emit_event_trait_new"),
            ("pub(crate) fn sqlite_extended_code_name_new() {}", "sqlite_extended_code_name_new"),
            ("fn sqlite_extended_code_name_from_int_new() {}", "sqlite_extended_code_name_from_int_new"),
            ("pub fn emit_open_error_event_new() {}", "emit_open_error_event_new"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(
                    f"root still defines lifecycle::{expected}",
                    owner_errors(root + "\n" + declaration, owner, open_source, worker),
                )

    def test_cfg_and_cfg_attr_mutants(self) -> None:
        root, owner, open_source, worker = self.sources()
        self.assertEqual(owner_errors(root, owner, open_source, worker), [])
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = owner.replace("impl Engine {", attr + "\nimpl Engine {", 1)
            self.assertNotEqual(altered, owner)
            self.assertIn("lifecycle gates Engine impl", owner_errors(root, altered, open_source, worker))
        for name in METHODS:
            marker = f"    {'pub' if name == 'subscribe' else 'pub(crate)'} fn {name}("
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                self.assertNotEqual(altered, owner)
                self.assertIn(f"lifecycle gates Engine::{name}", owner_errors(root, altered, open_source, worker))
        for name in HELPERS:
            marker = f"{'fn' if name == 'sqlite_extended_code_name_from_int' else 'pub(crate) fn'} {name}("
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                self.assertNotEqual(altered, owner)
                self.assertIn(f"lifecycle gates {name}", owner_errors(root, altered, open_source, worker))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = root.replace("pub mod lifecycle;", attr + "\npub mod lifecycle;", 1)
            self.assertIn("root lacks ungated lifecycle module", owner_errors(altered, owner, open_source, worker))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-lifecycle-events-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
