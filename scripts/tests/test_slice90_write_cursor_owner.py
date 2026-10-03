#!/usr/bin/env python3
"""Guard write cursor high-water helpers in their single semantic owner."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
NAMES = ("load_next_cursor", "reserved_write_cursor", "max_cursor")
FAMILY = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+"
    r"((?:load_next_cursor|reserved_write_cursor|max_cursor)\w*)[ \t]*\(",
    re.M,
)
CALLERS = (
    "actuation.rs",
    "dependency_trace.rs",
    "frozen_read.rs",
    "search.rs",
    "open.rs",
    "data_plane_integrity.rs",
    "graph_expand/execution.rs",
)
OWNER_IMPORT = "use crate::write_commit::load_next_cursor;"


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
    owner = (SRC / "write_commit.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "write_commit.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in FAMILY.findall(root)]
    for path, source in modules.items():
        errors.extend(f"{path} wrongly defines {name}" for name in FAMILY.findall(source))
    module = re.search(r"^mod write_commit;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates write_commit module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("write_commit gates whole owner module")
    names = FAMILY.findall(owner)
    for name in names:
        if name not in NAMES:
            errors.append(f"write_commit unexpectedly defines {name}")
    for name in NAMES:
        declarations = list(re.finditer(r"^pub\(crate\) fn " + name + r"\(", owner, re.M))
        if len(declarations) != 1 or names.count(name) != 1:
            errors.append(f"write_commit lacks one {name}")
        elif attrs(owner, declarations[0].start()):
            errors.append(f"write_commit gates {name}")
    for path in CALLERS:
        source = modules.get(path, "")
        if OWNER_IMPORT not in source:
            errors.append(f"{path} lacks write_commit owner path")
        if "crate::load_next_cursor" in source:
            errors.append(f"{path} still uses root cursor path")
    return errors


class WriteCursorOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_root_and_other_module_declarations(self) -> None:
        root, owner, modules = source_inventory()
        for name in NAMES:
            pattern = re.compile(r"(?ms)^fn " + name + r"\(.*?^}\n")
            match = pattern.search(root)
            if match:
                owner += "\n" + match.group().replace(f"fn {name}(", f"pub(crate) fn {name}(", 1)
                root = root[: match.start()] + root[match.end() :]
        for path in CALLERS:
            source = modules[path].replace("crate::load_next_cursor", "crate::write_commit::load_next_cursor")
            if OWNER_IMPORT not in source:
                source += "\n" + OWNER_IMPORT
            modules[path] = source
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("fn load_next_cursor_new() {}", "load_next_cursor_new"),
            ("pub(crate) fn reserved_write_cursor() {}", "reserved_write_cursor"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("open.rs", "fn load_next_cursor() {}", "load_next_cursor"),
            ("actuation.rs", "pub(crate) fn reserved_write_cursor_new() {}", "reserved_write_cursor_new"),
            ("search.rs", "pub fn max_cursor_new() {}", "max_cursor_new"),
        ):
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "fn load_next_cursor_new() {}"}
        self.assertIn("future_owner.rs wrongly defines load_next_cursor_new", owner_errors(root, owner, arbitrary))

    def test_owner_family_and_cfg_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        altered = owner + "\nfn max_cursor_new() {}"
        self.assertIn("write_commit unexpectedly defines max_cursor_new", owner_errors(root, altered, modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = owner.replace("pub(crate) fn load_next_cursor(", attr + "\npub(crate) fn load_next_cursor(", 1)
            self.assertIn("write_commit gates load_next_cursor", owner_errors(root, altered, modules))
            altered = root.replace("mod write_commit;", attr + "\nmod write_commit;", 1)
            self.assertIn("root gates write_commit module", owner_errors(altered, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("write_commit gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))
        altered = modules | {"frozen_read.rs": modules["frozen_read.rs"].replace(OWNER_IMPORT, "use crate::load_next_cursor;")}
        self.assertIn("frozen_read.rs still uses root cursor path", owner_errors(root, owner, altered))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-write-cursor-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
