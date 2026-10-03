#!/usr/bin/env python3
"""Guard lifecycle transition legality and drain budget ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
CONSTANT = "LIFECYCLE_DRAIN_TIMEOUT_MS"
HELPER = "is_legal_transition_move"
CONST_FAMILY = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?const[ \t]+(LIFECYCLE_DRAIN_TIMEOUT_MS\w*)\b", re.M)
HELPER_FAMILY = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(is_legal_transition_move\w*)[ \t]*\(", re.M)


def attrs(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "#[", "//")):
            break
    return found


def source_inventory() -> tuple[str, str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    owner = (SRC / "record_lifecycle.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "record_lifecycle.rs")
    }
    return root, owner, modules


def declarations(source: str) -> list[str]:
    return CONST_FAMILY.findall(source) + HELPER_FAMILY.findall(source)


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in declarations(root)]
    for path, source in modules.items():
        label = path.removesuffix(".rs")
        errors.extend(f"{label} wrongly defines {name}" for name in declarations(source))
    module = re.search(r"^mod record_lifecycle;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates record_lifecycle module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("record_lifecycle gates whole owner module")
    errors.extend(f"record_lifecycle unexpectedly defines {name}" for name in declarations(owner) if name not in (CONSTANT, HELPER))
    constant = list(re.finditer(r"^pub\(crate\) const LIFECYCLE_DRAIN_TIMEOUT_MS: u64 = 30_000;$", owner, re.M))
    if len(constant) != 1:
        errors.append("record_lifecycle lacks exact drain budget")
    elif attrs(owner, constant[0].start()):
        errors.append("record_lifecycle gates drain budget")
    helper = list(re.finditer(r"^pub\(crate\) fn is_legal_transition_move\(", owner, re.M))
    if len(helper) != 1:
        errors.append("record_lifecycle lacks one transition helper")
    elif attrs(owner, helper[0].start()):
        errors.append("record_lifecycle gates transition helper")
    for path, marker in (
        ("actuation.rs", "use crate::record_lifecycle::is_legal_transition_move;"),
        ("runtime_lifecycle.rs", "use crate::record_lifecycle::LIFECYCLE_DRAIN_TIMEOUT_MS;"),
        ("erasure.rs", "use crate::record_lifecycle::LIFECYCLE_DRAIN_TIMEOUT_MS;"),
    ):
        if marker not in modules.get(path, ""):
            errors.append(f"{path.removesuffix('.rs')} lacks record_lifecycle owner path")
    return errors


class RecordLifecycleOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_root_and_other_module_declarations(self) -> None:
        root, owner, modules = source_inventory()
        root = re.sub(r"(?m)^const LIFECYCLE_DRAIN_TIMEOUT_MS.*\n", "", root, count=1)
        if "pub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS: u64 = 30_000;" not in owner:
            owner += "\npub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS: u64 = 30_000;"
        old_helper = re.search(r"(?ms)^#\[must_use\]\nfn is_legal_transition_move\(.*?^}\n", root)
        if old_helper:
            root = root[: old_helper.start()] + root[old_helper.end() :]
            owner += "\n" + old_helper.group().replace("fn is_legal_transition_move(", "pub(crate) fn is_legal_transition_move(", 1)
        for path, marker in (
            ("actuation.rs", "use crate::record_lifecycle::is_legal_transition_move;"),
            ("runtime_lifecycle.rs", "use crate::record_lifecycle::LIFECYCLE_DRAIN_TIMEOUT_MS;"),
            ("erasure.rs", "use crate::record_lifecycle::LIFECYCLE_DRAIN_TIMEOUT_MS;"),
        ):
            if marker not in modules[path]:
                modules[path] += "\n" + marker
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("fn is_legal_transition_move_new() {}", "is_legal_transition_move_new"),
            ("pub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS_NEW: u64 = 1;", "LIFECYCLE_DRAIN_TIMEOUT_MS_NEW"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("lifecycle.rs", "fn is_legal_transition_move() {}", HELPER),
            ("lifecycle.rs", "pub(crate) fn is_legal_transition_move_new() {}", "is_legal_transition_move_new"),
            ("lifecycle.rs", "const LIFECYCLE_DRAIN_TIMEOUT_MS: u64 = 1;", CONSTANT),
            ("lifecycle.rs", "pub const LIFECYCLE_DRAIN_TIMEOUT_MS_NEW: u64 = 1;", "LIFECYCLE_DRAIN_TIMEOUT_MS_NEW"),
            ("search_api.rs", "pub(crate) fn is_legal_transition_move_new() {}", "is_legal_transition_move_new"),
            ("search_api.rs", "const LIFECYCLE_DRAIN_TIMEOUT_MS_NEW: u64 = 1;", "LIFECYCLE_DRAIN_TIMEOUT_MS_NEW"),
        ):
            self.assertIn(path, modules)
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path.removesuffix('.rs')} wrongly defines {name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "pub fn is_legal_transition_move_new() {}"}
        self.assertIn("future_owner wrongly defines is_legal_transition_move_new", owner_errors(root, owner, arbitrary))

    def test_owner_unlisted_and_cfg_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("const LIFECYCLE_DRAIN_TIMEOUT_MS_NEW: u64 = 1;", "LIFECYCLE_DRAIN_TIMEOUT_MS_NEW"),
            ("pub(crate) fn is_legal_transition_move_new() {}", "is_legal_transition_move_new"),
        ):
            self.assertIn(f"record_lifecycle unexpectedly defines {name}", owner_errors(root, owner + "\n" + declaration, modules))
        for marker, expected in (
            ("pub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS:", "record_lifecycle gates drain budget"),
            ("pub(crate) fn is_legal_transition_move(", "record_lifecycle gates transition helper"),
        ):
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                self.assertNotEqual(altered, owner)
                self.assertIn(expected, owner_errors(root, altered, modules))
        altered = owner.replace("30_000;", "1;", 1)
        self.assertIn("record_lifecycle lacks exact drain budget", owner_errors(root, altered, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("record_lifecycle gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = root.replace("mod record_lifecycle;", attr + "\nmod record_lifecycle;", 1)
            self.assertIn("root gates record_lifecycle module", owner_errors(altered, owner, modules))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-record-lifecycle-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
