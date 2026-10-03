#!/usr/bin/env python3
"""Guard approved generation, registry, and commit Engine test seams."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
HOOKS = '#[cfg(feature = "test-hooks")]'
DEBUG = "#[cfg(debug_assertions)]"
DEBUG_OR_HOOKS = '#[cfg(any(debug_assertions, feature = "test-hooks"))]'
OWNERS = {
    "projection_generation.rs": {
        "transition_projection_generation_for_test": HOOKS,
        "projection_generation_status_full_owner_scan_count_for_test": HOOKS,
        "projection_generation_status_query_plans_for_test": HOOKS,
    },
    "projection_commit.rs": {"publish_projection_success_for_test": HOOKS},
    "projection_registry.rs": {
        "configure_vector_kind_for_test": None,
        "set_legacy_projection_vector_declared_for_test": DEBUG,
        "set_legacy_projection_search_subobjects_for_test": DEBUG_OR_HOOKS,
    },
}
FAMILY_NAME = (
    r"(?:transition_projection_generation_for_test|projection_generation_status_|"
    r"publish_projection_success_for_test|configure_vector_kind_for_test|"
    r"set_legacy_projection_)\w*"
)
DECLARATION = re.compile(
    r"^[ \t]*(pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(" + FAMILY_NAME + r")\b", re.M
)
CFG = re.compile(r"#\[\s*cfg(?:_attr)?\b")


def prelude(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if stripped.startswith("#["):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "//")):
            break
    return found


def inventory() -> tuple[str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() != "lib.rs"
    }
    return root, modules


def owner_errors(root: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for _, name in DECLARATION.findall(root)]
    for path, source in modules.items():
        declarations = [name for _, name in DECLARATION.findall(source)]
        if path not in OWNERS:
            errors.extend(f"{path} wrongly defines {name}" for name in declarations)
            continue
        expected = OWNERS[path]
        for name in declarations:
            if name not in expected:
                errors.append(f"{path} wrongly defines {name}")
        if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", source):
            errors.append(f"{path} gates whole owner module")
        impls = list(re.finditer(r"(?ms)^impl Engine \{.*?^}", source))
        for impl in impls:
            if any(CFG.match(attr) for attr in prelude(source, impl.start())):
                errors.append(f"{path} gates Engine impl")
        for name, gate in expected.items():
            found = [(match, impl) for impl in impls for match in DECLARATION.finditer(impl.group())
                     if match.group(2) == name]
            if len(found) != 1 or found[0][0].group(1) != "pub ":
                errors.append(f"{path} lacks public Engine::{name}")
                continue
            match, impl = found[0]
            attrs = prelude(impl.group(), match.start())
            if [attr for attr in attrs if CFG.match(attr)] != ([gate] if gate else []) or "#[doc(hidden)]" not in attrs:
                errors.append(f"{path} changes Engine::{name} attrs")
            if declarations.count(name) != 1:
                errors.append(f"{path} duplicates Engine::{name}")
    for path in OWNERS:
        module = re.search(r"^mod " + re.escape(path.removesuffix(".rs")) + r";", root, re.M)
        if not module or any(CFG.match(attr) for attr in prelude(root, module.start())):
            errors.append(f"root gates {path} owner module")
    return errors


def complete_fixture() -> tuple[str, dict[str, str]]:
    root, modules = inventory()
    for path, expected in OWNERS.items():
        moved = []
        for name in expected:
            pattern = re.compile(r"(?m)^(?:    (?:///[^\n]*|#\[[^\n]*\])\n)*    pub fn " + name + r"\b[\s\S]*?^    }\n")
            match = pattern.search(root)
            if match:
                moved.append(match.group())
                root = root[:match.start()] + root[match.end():]
        if moved:
            modules[path] += "\nimpl Engine {\n" + "\n".join(moved) + "}\n"
    return root, modules


class ProjectionMetadataTestSeamsOwnerTest(unittest.TestCase):
    def test_current_source_has_exact_owners(self) -> None:
        self.assertEqual(owner_errors(*inventory()), [])

    def test_complete_owner_rejects_cross_owner_and_unlisted_methods(self) -> None:
        root, modules = complete_fixture()
        self.assertEqual(owner_errors(root, modules), [])
        for declaration, name in (
            ("impl Engine {\n    fn publish_projection_success_for_test_new(&self) {}\n}", "publish_projection_success_for_test_new"),
            ("impl Engine {\n    pub(crate) fn configure_vector_kind_for_test(&self) {}\n}", "configure_vector_kind_for_test"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, modules))
        for path, name in (("projection_generation.rs", "publish_projection_success_for_test"),
                           ("projection_commit.rs", "configure_vector_kind_for_test_new"),
                           ("projection_registry.rs", "transition_projection_generation_for_test_new"),
                           ("future_owner.rs", "set_legacy_projection_new")):
            changed = modules | {path: modules.get(path, "") + "\nimpl Engine {\n    pub fn " + name + "(&self) {}\n}"}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, changed))
        changed = modules | {"future_owner.rs": "impl Engine {\n    fn projection_generation_status_new(&self) {}\n}"}
        self.assertIn("future_owner.rs wrongly defines projection_generation_status_new", owner_errors(root, changed))

    def test_cfg_cfg_attr_item_impl_file_and_root_module_mutants(self) -> None:
        root, modules = complete_fixture()
        self.assertEqual(owner_errors(root, modules), [])
        for path, name in (("projection_generation.rs", "transition_projection_generation_for_test"),
                           ("projection_commit.rs", "publish_projection_success_for_test"),
                           ("projection_registry.rs", "configure_vector_kind_for_test")):
            owner = modules[path]
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                changed = owner.replace("    pub fn " + name + "(", "    " + attr + "\n    pub fn " + name + "(", 1)
                self.assertIn(f"{path} changes Engine::{name} attrs", owner_errors(root, modules | {path: changed}))
                impl_start = owner.rfind("impl Engine {", 0, owner.index("pub fn " + name))
                changed = owner[:impl_start] + attr + "\n" + owner[impl_start:]
                self.assertIn(f"{path} gates Engine impl", owner_errors(root, modules | {path: changed}))
                changed = root.replace("mod " + path.removesuffix(".rs") + ";", attr + "\nmod " + path.removesuffix(".rs") + ";", 1)
                self.assertIn(f"root gates {path} owner module", owner_errors(changed, modules))
                self.assertIn(f"{path} gates whole owner module", owner_errors(root, modules | {path: "#!" + attr[1:] + "\n" + owner}))
        owner = modules["projection_registry.rs"]
        changed = owner.replace(DEBUG_OR_HOOKS + "\n    #[doc(hidden)]\n    pub fn set_legacy_projection_search_subobjects_for_test(", "    #[doc(hidden)]\n    pub fn set_legacy_projection_search_subobjects_for_test(", 1)
        self.assertIn("projection_registry.rs changes Engine::set_legacy_projection_search_subobjects_for_test attrs", owner_errors(root, modules | {"projection_registry.rs": changed}))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-projection-metadata-test-seams-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
