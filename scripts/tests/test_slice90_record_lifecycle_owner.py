#!/usr/bin/env python3
"""Guard lifecycle transition legality and drain budget ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
CONSTANT = "LIFECYCLE_DRAIN_TIMEOUT_MS"
HELPER = "is_legal_transition_move"
CONST_FAMILY = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+(LIFECYCLE_DRAIN_TIMEOUT_MS\w*)\b", re.M)
HELPER_FAMILY = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(is_legal_transition_move\w*)\s*\(", re.M)


def attrs(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "#[", "//")):
            break
    return found


def owner_errors(root: str, owner: str, actuation: str, runtime: str, erasure: str, rebuild: str) -> list[str]:
    errors = []
    for label, source in (("root", root), ("actuation", actuation), ("runtime_lifecycle", runtime), ("erasure", erasure), ("projection_rebuild", rebuild)):
        errors.extend(f"{label} wrongly defines {name}" for name in CONST_FAMILY.findall(source))
        errors.extend(f"{label} wrongly defines {name}" for name in HELPER_FAMILY.findall(source))
    module = re.search(r"^mod record_lifecycle;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates record_lifecycle module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("record_lifecycle gates whole owner module")
    errors.extend(f"record_lifecycle unexpectedly defines {name}" for name in CONST_FAMILY.findall(owner) if name != CONSTANT)
    errors.extend(f"record_lifecycle unexpectedly defines {name}" for name in HELPER_FAMILY.findall(owner) if name != HELPER)
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
    for label, source, marker in (
        ("actuation", actuation, "use crate::record_lifecycle::is_legal_transition_move;"),
        ("runtime_lifecycle", runtime, "use crate::record_lifecycle::LIFECYCLE_DRAIN_TIMEOUT_MS;"),
        ("erasure", erasure, "use crate::record_lifecycle::LIFECYCLE_DRAIN_TIMEOUT_MS;"),
    ):
        if marker not in source:
            errors.append(f"{label} lacks record_lifecycle owner path")
    return errors


class RecordLifecycleOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "record_lifecycle.rs", "actuation.rs", "runtime_lifecycle.rs", "erasure.rs", "projection_rebuild.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_and_wrong_owner_families(self) -> None:
        root, owner, actuation, runtime, erasure, rebuild = self.sources()
        root = re.sub(r"(?m)^const LIFECYCLE_DRAIN_TIMEOUT_MS.*\n", "", root, count=1)
        if "pub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS: u64 = 30_000;" not in owner:
            owner += "\npub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS: u64 = 30_000;"
        old_helper = re.search(r"(?ms)^#\[must_use\]\nfn is_legal_transition_move\(.*?^}\n", root)
        if old_helper:
            root = root[: old_helper.start()] + root[old_helper.end() :]
            owner += "\n" + old_helper.group().replace("fn is_legal_transition_move(", "pub(crate) fn is_legal_transition_move(", 1)
        for source_name, marker in (
            ("actuation", "use crate::record_lifecycle::is_legal_transition_move;"),
            ("runtime", "use crate::record_lifecycle::LIFECYCLE_DRAIN_TIMEOUT_MS;"),
            ("erasure", "use crate::record_lifecycle::LIFECYCLE_DRAIN_TIMEOUT_MS;"),
        ):
            if source_name == "actuation" and marker not in actuation:
                actuation += "\n" + marker
            elif source_name == "runtime" and marker not in runtime:
                runtime += "\n" + marker
            elif source_name == "erasure" and marker not in erasure:
                erasure += "\n" + marker
        self.assertEqual(owner_errors(root, owner, actuation, runtime, erasure, rebuild), [])
        for label, declaration, name in (
            ("root", "fn is_legal_transition_move_new() {}", "is_legal_transition_move_new"),
            ("root", "pub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS_NEW: u64 = 1;", "LIFECYCLE_DRAIN_TIMEOUT_MS_NEW"),
            ("actuation", "pub fn is_legal_transition_move_new() {}", "is_legal_transition_move_new"),
            ("runtime_lifecycle", "const LIFECYCLE_DRAIN_TIMEOUT_MS: u64 = 1;", "LIFECYCLE_DRAIN_TIMEOUT_MS"),
            ("erasure", "pub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS_NEW: u64 = 1;", "LIFECYCLE_DRAIN_TIMEOUT_MS_NEW"),
            ("projection_rebuild", "fn is_legal_transition_move_new() {}", "is_legal_transition_move_new"),
        ):
            sources = {"root": root, "actuation": actuation, "runtime_lifecycle": runtime, "erasure": erasure, "projection_rebuild": rebuild}
            sources[label] += "\n" + declaration
            self.assertIn(
                f"{label} wrongly defines {name}",
                owner_errors(sources["root"], owner, sources["actuation"], sources["runtime_lifecycle"], sources["erasure"], sources["projection_rebuild"]),
            )

    def test_owner_unlisted_and_cfg_mutants(self) -> None:
        root, owner, actuation, runtime, erasure, rebuild = self.sources()
        self.assertEqual(owner_errors(root, owner, actuation, runtime, erasure, rebuild), [])
        for declaration, name in (
            ("const LIFECYCLE_DRAIN_TIMEOUT_MS_NEW: u64 = 1;", "LIFECYCLE_DRAIN_TIMEOUT_MS_NEW"),
            ("pub(crate) fn is_legal_transition_move_new() {}", "is_legal_transition_move_new"),
        ):
            self.assertIn(f"record_lifecycle unexpectedly defines {name}", owner_errors(root, owner + "\n" + declaration, actuation, runtime, erasure, rebuild))
        for marker, expected in (
            ("pub(crate) const LIFECYCLE_DRAIN_TIMEOUT_MS:", "record_lifecycle gates drain budget"),
            ("pub(crate) fn is_legal_transition_move(", "record_lifecycle gates transition helper"),
        ):
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                self.assertNotEqual(altered, owner)
                self.assertIn(expected, owner_errors(root, altered, actuation, runtime, erasure, rebuild))
        altered = owner.replace("30_000;", "1;", 1)
        self.assertIn("record_lifecycle lacks exact drain budget", owner_errors(root, altered, actuation, runtime, erasure, rebuild))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("record_lifecycle gates whole owner module", owner_errors(root, attr + "\n" + owner, actuation, runtime, erasure, rebuild))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = root.replace("mod record_lifecycle;", attr + "\nmod record_lifecycle;", 1)
            self.assertIn("root gates record_lifecycle module", owner_errors(altered, owner, actuation, runtime, erasure, rebuild))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-record-lifecycle-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
