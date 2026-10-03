#!/usr/bin/env python3
"""Guard the approved Engine vector row test seams in vector_storage."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
OWNER = "vector_storage.rs"
METHODS = (
    "write_vector_for_test",
    "vector_row_count_for_test",
    "has_vector_row_for_cursor_for_test",
    "read_vector_blob_for_test",
    "read_vector_bin_for_test",
)
FAMILY = r"(?:write_vector|vector_row_count|has_vector_row_for_cursor|read_vector_(?:blob|bin))\w*"
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
        if path != OWNER:
            errors.extend(f"{path} wrongly defines {name}" for name in names)
            continue
        errors.extend(f"{path} wrongly defines {name}" for name in names if name not in METHODS)
        if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", source):
            errors.append(f"{path} gates whole owner module")
        impls = list(IMPL.finditer(source))
        for impl in impls:
            if any(CFG.match(attr) for attr in prelude(source, impl.start())):
                errors.append(f"{path} gates Engine impl")
        for name in METHODS:
            found = [(match, impl) for impl in impls for match in DECL.finditer(impl.group())
                     if match.group(2) == name]
            if len(found) != 1 or found[0][0].group(1) != "pub ":
                errors.append(f"{path} lacks public Engine::{name}")
                continue
            match, impl = found[0]
            attrs = prelude(impl.group(), match.start())
            if any(CFG.match(attr) for attr in attrs) or "#[doc(hidden)]" not in attrs:
                errors.append(f"{path} changes Engine::{name} attrs")
            if names.count(name) != 1:
                errors.append(f"{path} duplicates Engine::{name}")
    module = re.search(r"(?m)^mod vector_storage;", root)
    if not module or any(CFG.match(attr) for attr in prelude(root, module.start())):
        errors.append("root gates vector_storage owner module")
    return errors


def complete_fixture() -> tuple[str, dict[str, str]]:
    root, modules = inventory()
    moved = []
    for name in METHODS:
        pattern = re.compile(r"(?m)^(?:    (?:///[^\n]*|#\[[^\n]*\])\n)*    pub fn " + name + r"\b[\s\S]*?^    }\n")
        match = pattern.search(root)
        if match:
            moved.append(match.group())
            root = root[:match.start()] + root[match.end():]
    if moved:
        modules[OWNER] += "\nimpl Engine {\n" + "\n".join(moved) + "}\n"
    return root, modules


class VectorStorageTestSeamsOwnerTest(unittest.TestCase):
    def test_current_source_has_exact_owners(self) -> None:
        self.assertEqual(owner_errors(*inventory()), [])

    def test_complete_owner_rejects_root_and_wrong_owner_methods(self) -> None:
        root, modules = complete_fixture()
        self.assertEqual(owner_errors(root, modules), [])
        for name, visibility in (("write_vector_for_test_new", ""),
                                 ("read_vector_blob_new_for_test", "pub(crate) "),
                                 ("vector_row_count_for_test", "pub ")):
            mutant = root + f"\nimpl Engine {{\n    {visibility}fn {name}(&self) {{}}\n}}\n"
            self.assertIn(f"root wrongly defines {name}", owner_errors(mutant, modules))
        for path, name in (("read_api.rs", "write_vector_for_test"),
                           ("embedding.rs", "has_vector_row_for_cursor_new_for_test"),
                           ("future_owner.rs", "read_vector_bin_for_test_new")):
            mutant = modules | {path: modules.get(path, "") + f"\nimpl Engine {{\n    fn {name}(&self) {{}}\n}}\n"}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, mutant))
        mutant = modules | {OWNER: modules[OWNER] + "\nimpl Engine {\n    fn vector_row_count_new_for_test(&self) {}\n}\n"}
        self.assertIn(f"{OWNER} wrongly defines vector_row_count_new_for_test", owner_errors(root, mutant))

    def test_cfg_and_cfg_attr_item_impl_file_and_root_mutants(self) -> None:
        root, modules = complete_fixture()
        self.assertEqual(owner_errors(root, modules), [])
        owner = modules[OWNER]
        for attr in ('#[cfg(feature = "operator")]',
                     '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace("    pub fn write_vector_for_test(",
                                    "    " + attr + "\n    pub fn write_vector_for_test(", 1)
            self.assertIn(f"{OWNER} changes Engine::write_vector_for_test attrs",
                          owner_errors(root, modules | {OWNER: changed}))
            at = owner.rfind("impl Engine {", 0, owner.index("pub fn write_vector_for_test"))
            changed = owner[:at] + attr + "\n" + owner[at:]
            self.assertIn(f"{OWNER} gates Engine impl", owner_errors(root, modules | {OWNER: changed}))
            self.assertIn(f"{OWNER} gates whole owner module",
                          owner_errors(root, modules | {OWNER: "#!" + attr[1:] + "\n" + owner}))
            changed = root.replace("mod vector_storage;", attr + "\nmod vector_storage;", 1)
            self.assertIn("root gates vector_storage owner module", owner_errors(changed, modules))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-vector-storage-test-seams-owner",
                      (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
