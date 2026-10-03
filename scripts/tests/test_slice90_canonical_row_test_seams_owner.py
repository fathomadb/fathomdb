#!/usr/bin/env python3
"""Guard approved Engine canonical row and page query test seams."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
HOOKS = '#[cfg(feature = "test-hooks")]'
OWNERS = {
    "write.rs": {"write_canonical_row_with_kind_for_test": None},
    "read_api.rs": {
        "canonical_rows_with_row_kind_for_test": None,
        "slice45_page_query_plans_for_test": HOOKS,
    },
}
FAMILY = r"(?:write_canonical_row|canonical_rows_with_row_kind|slice45_page_query_plans)\w*"
DECL = re.compile(r"^[ \t]*(pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(" + FAMILY + r")\b", re.M)
CFG = re.compile(r"#\[\s*cfg(?:_attr)?\b")
IMPL = re.compile(r"(?ms)^impl Engine \{.*?^}")


def prelude(source: str, start: int) -> list[str]:
    attrs = []
    for line in reversed(source[:start].splitlines()):
        line = line.strip()
        if line.startswith("#["):
            attrs.append(line)
        elif line and not line.startswith(("///", "//")):
            break
    return attrs


def inventory() -> tuple[str, dict[str, str]]:
    return (SRC / "lib.rs").read_text(), {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() != "lib.rs"
    }


def owner_errors(root: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for _, name in DECL.findall(root)]
    for path, source in modules.items():
        names = [name for _, name in DECL.findall(source)]
        if path not in OWNERS:
            errors.extend(f"{path} wrongly defines {name}" for name in names)
            continue
        expected = OWNERS[path]
        errors.extend(f"{path} wrongly defines {name}" for name in names if name not in expected)
        if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", source):
            errors.append(f"{path} gates whole owner module")
        impls = list(IMPL.finditer(source))
        for impl in impls:
            if any(CFG.match(attr) for attr in prelude(source, impl.start())):
                errors.append(f"{path} gates Engine impl")
        for name, gate in expected.items():
            found = [(match, impl) for impl in impls for match in DECL.finditer(impl.group())
                     if match.group(2) == name]
            if len(found) != 1 or found[0][0].group(1) != "pub ":
                errors.append(f"{path} lacks public Engine::{name}")
                continue
            match, impl = found[0]
            attrs = prelude(impl.group(), match.start())
            if [attr for attr in attrs if CFG.match(attr)] != ([gate] if gate else []) or "#[doc(hidden)]" not in attrs:
                errors.append(f"{path} changes Engine::{name} attrs")
            if names.count(name) != 1:
                errors.append(f"{path} duplicates Engine::{name}")
    for path in OWNERS:
        module = re.search(r"(?m)^mod " + path.removesuffix(".rs") + r";", root)
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


class CanonicalRowTestSeamsOwnerTest(unittest.TestCase):
    def test_current_source_has_exact_owners(self) -> None:
        self.assertEqual(owner_errors(*inventory()), [])

    def test_complete_owner_rejects_root_wrong_owner_and_unlisted_family(self) -> None:
        root, modules = complete_fixture()
        self.assertEqual(owner_errors(root, modules), [])
        for visibility, name in (("", "write_canonical_row_new_for_test"),
                                 ("pub(crate) ", "canonical_rows_with_row_kind_for_test_new"),
                                 ("pub ", "slice45_page_query_plans_for_test")):
            mutant = root + f"\nimpl Engine {{\n    {visibility}fn {name}(&self) {{}}\n}}\n"
            self.assertIn(f"root wrongly defines {name}", owner_errors(mutant, modules))
        for path, name in (("read_api.rs", "write_canonical_row_with_kind_for_test"),
                           ("write.rs", "slice45_page_query_plans_new_for_test"),
                           ("future_owner.rs", "canonical_rows_with_row_kind_new_for_test")):
            mutant = modules | {path: modules.get(path, "") + f"\nimpl Engine {{\n    fn {name}(&self) {{}}\n}}\n"}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, mutant))
        mutant = modules | {"write.rs": modules["write.rs"] + "\nimpl Engine {\n    fn write_canonical_row_new_for_test(&self) {}\n}\n"}
        self.assertIn("write.rs wrongly defines write_canonical_row_new_for_test", owner_errors(root, mutant))

    def test_cfg_and_cfg_attr_item_impl_file_and_root_mutants(self) -> None:
        root, modules = complete_fixture()
        self.assertEqual(owner_errors(root, modules), [])
        for path, name in (("write.rs", "write_canonical_row_with_kind_for_test"),
                           ("read_api.rs", "canonical_rows_with_row_kind_for_test")):
            owner = modules[path]
            for attr in ('#[cfg(feature = "operator")]',
                         '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                changed = owner.replace("    pub fn " + name + "(", "    " + attr + "\n    pub fn " + name + "(", 1)
                self.assertIn(f"{path} changes Engine::{name} attrs", owner_errors(root, modules | {path: changed}))
                at = owner.rfind("impl Engine {", 0, owner.index("pub fn " + name))
                changed = owner[:at] + attr + "\n" + owner[at:]
                self.assertIn(f"{path} gates Engine impl", owner_errors(root, modules | {path: changed}))
                self.assertIn(f"{path} gates whole owner module",
                              owner_errors(root, modules | {path: "#!" + attr[1:] + "\n" + owner}))
                changed = root.replace("mod " + path.removesuffix(".rs") + ";",
                                       attr + "\nmod " + path.removesuffix(".rs") + ";", 1)
                self.assertIn(f"root gates {path} owner module", owner_errors(changed, modules))
        owner = modules["read_api.rs"]
        changed = owner.replace(HOOKS + "\n    #[doc(hidden)]\n    pub fn slice45_page_query_plans_for_test(",
                                "    #[doc(hidden)]\n    pub fn slice45_page_query_plans_for_test(", 1)
        self.assertIn("read_api.rs changes Engine::slice45_page_query_plans_for_test attrs",
                      owner_errors(root, modules | {"read_api.rs": changed}))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-canonical-row-test-seams-owner",
                      (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
