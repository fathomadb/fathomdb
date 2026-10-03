#!/usr/bin/env python3
"""Guard approved evidence, embed, open, operator, and connection test seams."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
HOOKS = '#[cfg(feature = "test-hooks")]'
OWNERS = {
    "evidence.rs": {"explain_graph_evidence_preflights_for_test": (HOOKS, True)},
    "embedding.rs": {"drain_mean_centering_events_for_test": (None, True)},
    "open.rs": {"default_embedder_profile_for_test": (None, True)},
    "operator.rs": {"schema_objects_for_test": (HOOKS, False)},
}
FREE_OWNER = "connection_runtime.rs"
FREE_NAME = "record_writer_pragma_witness_for_test"
FAMILY = (
    r"(?:explain_graph_evidence_preflights|drain_mean_centering_events|"
    r"default_embedder_profile|schema_objects|record_writer_pragma_witness)\w*"
)
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
        if path not in OWNERS and path != FREE_OWNER:
            errors.extend(f"{path} wrongly defines {name}" for name in names)
            continue
        expected = OWNERS.get(path, {FREE_NAME: (HOOKS, False)})
        errors.extend(f"{path} wrongly defines {name}" for name in names if name not in expected)
        if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", source):
            errors.append(f"{path} gates whole owner module")
        impls = list(IMPL.finditer(source))
        for impl in impls:
            if any(CFG.match(attr) for attr in prelude(source, impl.start())):
                errors.append(f"{path} gates Engine impl")
        for name, (gate, hidden) in expected.items():
            if path == FREE_OWNER:
                found = list(re.finditer(r"(?m)^pub\(super\) fn " + re.escape(name) + r"\b", source))
                if len(found) != 1 or names.count(name) != 1:
                    errors.append(f"{path} lacks private {name}")
                    continue
                attrs = prelude(source, found[0].start())
                if [attr for attr in attrs if CFG.match(attr)] != [HOOKS]:
                    errors.append(f"{path} changes {name} attrs")
                continue
            found = [(match, impl) for impl in impls for match in DECL.finditer(impl.group())
                     if match.group(2) == name]
            if len(found) != 1 or found[0][0].group(1) != "pub ":
                errors.append(f"{path} lacks public Engine::{name}")
                continue
            match, impl = found[0]
            attrs = prelude(impl.group(), match.start())
            if [attr for attr in attrs if CFG.match(attr)] != ([gate] if gate else []) or ("#[doc(hidden)]" in attrs) != hidden:
                errors.append(f"{path} changes Engine::{name} attrs")
            if names.count(name) != 1:
                errors.append(f"{path} duplicates Engine::{name}")
    for path in (*OWNERS, FREE_OWNER):
        module_name = path.removesuffix(".rs")
        module = re.search(r"(?m)^mod " + module_name + r";", root)
        if not module or any(CFG.match(attr) for attr in prelude(root, module.start())):
            errors.append(f"root gates {module_name} owner module")
    open_source = modules.get("open.rs", "")
    owner_import = re.search(r"(?m)^use crate::connection_runtime::record_writer_pragma_witness_for_test;", open_source)
    if not owner_import or [attr for attr in prelude(open_source, owner_import.start()) if CFG.match(attr)] != [HOOKS]:
        errors.append("open lacks private writer pragma owner import")
    if open_source.count("record_writer_pragma_witness_for_test(&connection);") != 1:
        errors.append("open loses writer pragma observation call")
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
    pattern = re.compile(r"(?ms)^#\[cfg\(feature = \"test-hooks\"\)\]\nfn " + FREE_NAME + r"\b.*?^}\n")
    match = pattern.search(root)
    if match:
        modules[FREE_OWNER] += "\n" + match.group().replace("\nfn " + FREE_NAME,
                                                               "\npub(super) fn " + FREE_NAME, 1)
        root = root[:match.start()] + root[match.end():]
    if "use crate::connection_runtime::record_writer_pragma_witness_for_test;" not in modules["open.rs"]:
        modules["open.rs"] += "\n" + HOOKS + "\nuse crate::connection_runtime::record_writer_pragma_witness_for_test;\n"
    return root, modules


class FinalObservationTestSeamsOwnerTest(unittest.TestCase):
    def test_current_source_has_exact_owners(self) -> None:
        self.assertEqual(owner_errors(*inventory()), [])

    def test_complete_owner_rejects_root_wrong_owner_and_unlisted_family(self) -> None:
        root, modules = complete_fixture()
        self.assertEqual(owner_errors(root, modules), [])
        for visibility, name in (("", "explain_graph_evidence_preflights_new"),
                                 ("pub(crate) ", "record_writer_pragma_witness_for_test"),
                                 ("pub ", "schema_objects_new")):
            mutant = root + f"\nimpl Engine {{\n    {visibility}fn {name}(&self) {{}}\n}}\n"
            self.assertIn(f"root wrongly defines {name}", owner_errors(mutant, modules))
        trait_mutant = root + "\nimpl Unexpected for Engine {\n    fn drain_mean_centering_events_new(&self) {}\n}\n"
        self.assertIn("root wrongly defines drain_mean_centering_events_new",
                      owner_errors(trait_mutant, modules))
        for path, name in (("embedding.rs", "default_embedder_profile_for_test"),
                           ("evidence.rs", "schema_objects_new"),
                           ("operator.rs", "drain_mean_centering_events_for_test"),
                           ("connection_runtime.rs", "explain_graph_evidence_preflights_new"),
                           ("future_owner.rs", "record_writer_pragma_witness_new")):
            mutant = modules | {path: modules.get(path, "") + f"\nfn {name}() {{}}\n"}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, mutant))

    def test_cfg_cfg_attr_item_impl_file_root_and_open_import_mutants(self) -> None:
        root, modules = complete_fixture()
        self.assertEqual(owner_errors(root, modules), [])
        for path, name in (("evidence.rs", "explain_graph_evidence_preflights_for_test"),
                           ("embedding.rs", "drain_mean_centering_events_for_test"),
                           ("open.rs", "default_embedder_profile_for_test"),
                           ("operator.rs", "schema_objects_for_test")):
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
                module_name = path.removesuffix(".rs")
                changed = root.replace("mod " + module_name + ";", attr + "\nmod " + module_name + ";", 1)
                self.assertIn(f"root gates {module_name} owner module", owner_errors(changed, modules))
        witness = modules[FREE_OWNER]
        for attr in ('#[cfg(feature = "operator")]',
                     '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = witness.replace(HOOKS + "\npub(super) fn " + FREE_NAME,
                                      HOOKS + "\n" + attr + "\npub(super) fn " + FREE_NAME, 1)
            self.assertIn(f"{FREE_OWNER} changes {FREE_NAME} attrs",
                          owner_errors(root, modules | {FREE_OWNER: changed}))
            self.assertIn(f"{FREE_OWNER} gates whole owner module",
                          owner_errors(root, modules | {FREE_OWNER: "#!" + attr[1:] + "\n" + witness}))
            changed_root = root.replace("mod connection_runtime;", attr + "\nmod connection_runtime;", 1)
            self.assertIn("root gates connection_runtime owner module", owner_errors(changed_root, modules))
        changed = modules["open.rs"].replace("use crate::connection_runtime::record_writer_pragma_witness_for_test;", "", 1)
        self.assertIn("open lacks private writer pragma owner import", owner_errors(root, modules | {"open.rs": changed}))
        changed = modules["open.rs"].replace("record_writer_pragma_witness_for_test(&connection);", "", 1)
        self.assertIn("open loses writer pragma observation call", owner_errors(root, modules | {"open.rs": changed}))
        changed = modules["open.rs"].replace(HOOKS + "\nuse crate::connection_runtime::record_writer_pragma_witness_for_test;",
                                               '#[cfg(feature = "operator")]\nuse crate::connection_runtime::record_writer_pragma_witness_for_test;', 1)
        self.assertIn("open lacks private writer pragma owner import", owner_errors(root, modules | {"open.rs": changed}))
        changed = witness.replace("pub(super) fn " + FREE_NAME, "fn " + FREE_NAME, 1)
        self.assertIn(f"{FREE_OWNER} lacks private {FREE_NAME}", owner_errors(root, modules | {FREE_OWNER: changed}))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-final-observation-test-seams-owner",
                      (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
