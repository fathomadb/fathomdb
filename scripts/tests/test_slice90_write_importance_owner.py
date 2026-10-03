#!/usr/bin/env python3
"""Guard Engine::write_node_importance in the write owner."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
NAME = "write_node_importance"
FAMILY = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(write_node_importance\w*)\b[ \t]*\(",
    re.M,
)
ORDER = (
    "if !importance.is_finite() || !(0.0..=1.0).contains(&importance)",
    "self.ensure_open()?;",
    "dependency_closure::maintain_before_writer(connection)?;",
    "transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)",
    "dependency_closure::guard_no_pending_physical(&tx)?;",
    '"UPDATE canonical_nodes SET importance = ?1 WHERE write_cursor = ?2"',
    "tx.commit().map_err(|_| EngineError::Storage)?;",
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


def source_inventory() -> tuple[str, str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    owner = (SRC / "write.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "write.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines Engine::{name}" for name in FAMILY.findall(root)]
    for path, source in modules.items():
        errors.extend(f"{path} wrongly defines Engine::{name}" for name in FAMILY.findall(source))
    module = re.search(r"^mod write;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates write module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("write gates whole owner module")
    errors.extend(f"write unexpectedly defines {name}" for name in FAMILY.findall(owner) if name != NAME)
    found = list(re.finditer(r"^    pub fn write_node_importance\(", owner, re.M))
    if len(found) != 1 or FAMILY.findall(owner).count(NAME) != 1:
        errors.append("write lacks one public importance writer")
        return errors
    method = found[0]
    impls = list(re.finditer(r"^impl Engine \{", owner[: method.start()], re.M))
    if not impls or attrs(owner, impls[-1].start()):
        errors.append("write gates Engine impl")
    if attrs(owner, method.start()):
        errors.append("write gates importance writer")
    body = re.search(r"(?ms)^    pub fn write_node_importance\(.*?^    }", owner)
    if not body or any(body.group().find(marker) < 0 for marker in ORDER):
        errors.append("write changes importance transaction contract")
    else:
        positions = [body.group().find(marker) for marker in ORDER]
        if positions != sorted(positions):
            errors.append("write changes importance transaction ordering")
    return errors


class WriteImportanceOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_root_and_other_modules(self) -> None:
        root, owner, modules = source_inventory()
        method = re.search(r"(?ms)^    pub fn write_node_importance\(.*?^    }\n", root)
        if method:
            owner = owner.replace("impl Engine {", "impl Engine {\n" + method.group(), 1)
            root = root[: method.start()] + root[method.end() :]
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("impl Engine {\n    fn write_node_importance_new(&self) {}\n}", "write_node_importance_new"),
            ("impl ImportanceWriter for Engine {\n    pub(crate) fn write_node_importance(&self) {}\n}", NAME),
        ):
            self.assertIn(f"root wrongly defines Engine::{name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("read_api.rs", "impl Engine {\n    pub fn write_node_importance_new(&self) {}\n}", "write_node_importance_new"),
            ("write_commit.rs", "impl Engine {\n    fn write_node_importance(&self) {}\n}", NAME),
        ):
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path} wrongly defines Engine::{name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "impl ImportanceWriter for Engine {\n    fn write_node_importance_new(&self) {}\n}"}
        self.assertIn("future_owner.rs wrongly defines Engine::write_node_importance_new", owner_errors(root, owner, arbitrary))

    def test_cfg_cfg_attr_and_order_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        changed = owner.replace(ORDER[0], "REORDER_ME", 1).replace(ORDER[1], ORDER[0], 1).replace("REORDER_ME", ORDER[1], 1)
        self.assertIn("write changes importance transaction ordering", owner_errors(root, changed, modules))
        changed = owner.replace("dependency_closure::guard_no_pending_physical(&tx)?;", "", 1)
        self.assertIn("write changes importance transaction contract", owner_errors(root, changed, modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace("    pub fn write_node_importance(", "    " + attr + "\n    pub fn write_node_importance(", 1)
            self.assertIn("write gates importance writer", owner_errors(root, changed, modules))
            changed = owner.replace("impl Engine {", attr + "\nimpl Engine {", 1)
            self.assertIn("write gates Engine impl", owner_errors(root, changed, modules))
            changed = root.replace("mod write;", attr + "\nmod write;", 1)
            self.assertIn("root gates write module", owner_errors(changed, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("write gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-write-importance-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
