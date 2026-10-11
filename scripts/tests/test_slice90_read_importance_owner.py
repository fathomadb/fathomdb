#!/usr/bin/env python3
"""Guard the public node importance read in read_api."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
NAME = "node_importance"
FAMILY = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(node_importance\w*)\b[ \t]*\(",
    re.M,
)
ORDER = (
    "self.ensure_open()?;",
    "self.connection.lock().map_err(|_| EngineError::Storage)?;",
    "connection.as_ref().ok_or(EngineError::Closing)?;",
    'dependency_closure::read_eligibility_sql("canonical_nodes", false, false, false, 2)',
    '"SELECT importance FROM canonical_nodes \\\n                     WHERE write_cursor = ?1{eligibility} LIMIT 1"',
    "params![write_cursor, current_epoch_seconds()]",
    ".optional()",
    ".map(Option::flatten)",
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
    owner = (SRC / "read_api.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "read_api.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines Engine::{name}" for name in FAMILY.findall(root)]
    for path, source in modules.items():
        errors.extend(f"{path} wrongly defines Engine::{name}" for name in FAMILY.findall(source))
    module = re.search(r"^mod read_api;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates read_api module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("read_api gates whole owner module")
    errors.extend(f"read_api unexpectedly defines {name}" for name in FAMILY.findall(owner) if name != NAME)
    method = list(re.finditer(r"^    pub fn node_importance\(&self, write_cursor: u64\) -> Result<Option<f64>, EngineError> \{", owner, re.M))
    if len(method) != 1 or FAMILY.findall(owner).count(NAME) != 1:
        errors.append("read_api lacks one public importance reader")
        return errors
    impls = list(re.finditer(r"^impl Engine \{", owner[: method[0].start()], re.M))
    if not impls or attrs(owner, impls[-1].start()):
        errors.append("read_api gates Engine impl")
    if attrs(owner, method[0].start()):
        errors.append("read_api gates importance reader")
    body = re.search(r"(?ms)^    pub fn node_importance\(.*?^    }", owner)
    if not body or any(marker not in body.group() for marker in ORDER):
        errors.append("read_api changes importance read contract")
    else:
        positions = [body.group().find(marker) for marker in ORDER]
        if positions != sorted(positions):
            errors.append("read_api changes importance read ordering")
    if body and "self.reader_pool.dispatch(" in body.group():
        errors.append("read_api reroutes importance through reader pool")
    return errors


class ReadImportanceOwnerTest(unittest.TestCase):
    def test_current_source_has_one_ungated_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_root_and_other_owner_methods(self) -> None:
        root, owner, modules = source_inventory()
        method = re.search(r"(?ms)^    pub fn node_importance\(&self, write_cursor: u64\).*?^    }\n", root)
        if method:
            owner = owner.replace("impl Engine {", "impl Engine {\n" + method.group(), 1)
            root = root[: method.start()] + root[method.end() :]
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("impl Engine {\n    fn node_importance_new(&self) {}\n}", "node_importance_new"),
            ("impl ImportanceRead for Engine {\n    pub(crate) fn node_importance(&self) {}\n}", NAME),
        ):
            self.assertIn(f"root wrongly defines Engine::{name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("write.rs", "impl Engine {\n    pub fn node_importance_new(&self) {}\n}", "node_importance_new"),
            ("read.rs", "impl Engine {\n    fn node_importance(&self) {}\n}", NAME),
        ):
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path} wrongly defines Engine::{name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "impl ImportanceRead for Engine {\n    fn node_importance_new(&self) {}\n}"}
        self.assertIn("future_owner.rs wrongly defines Engine::node_importance_new", owner_errors(root, owner, arbitrary))

    def test_cfg_cfg_attr_and_writer_connection_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        changed = owner.replace("self.connection.lock()", "self.reader_pool.lock()", 1)
        self.assertIn("read_api changes importance read contract", owner_errors(root, changed, modules))
        changed = owner.replace("false, false, false, 2", "false, true, false, 2", 1)
        self.assertIn("read_api changes importance read contract", owner_errors(root, changed, modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace("    pub fn node_importance(", "    " + attr + "\n    pub fn node_importance(", 1)
            self.assertIn("read_api gates importance reader", owner_errors(root, changed, modules))
            changed = owner.replace("impl Engine {", attr + "\nimpl Engine {", 1)
            self.assertIn("read_api gates Engine impl", owner_errors(root, changed, modules))
            changed = root.replace("mod read_api;", attr + "\nmod read_api;", 1)
            self.assertIn("root gates read_api module", owner_errors(changed, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("read_api gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-read-importance-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
