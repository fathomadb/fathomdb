#!/usr/bin/env python3
"""Keep the D27 observation adapter visible while its dispatch core stays standalone."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
HOOKS = '#[cfg(feature = "test-hooks")]'
METHODS = (
    "begin_d27_observation_for_test",
    "with_d27_foreground_owner_for_test",
    "d27_observation_for_test",
)
FAMILY = re.compile(
    r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+"
    r"((?:begin_d27_observation|with_d27_foreground_owner|d27_observation)\w*)\b"
)
CORE_METHODS = {"begin_d27_observation", "d27_observation"}
IMPL = re.compile(r"(?ms)^impl Engine \{.*?^}")
CFG = re.compile(r"#\[\s*cfg(?:_attr)?\b")


def prelude(source: str, start: int) -> list[str]:
    attrs = []
    for line in reversed(source[:start].splitlines()):
        line = line.strip()
        if line.startswith("#["):
            attrs.append(line)
        elif line and not line.startswith(("///", "//")):
            break
    return attrs


def inventory() -> tuple[str, dict[str, str], str]:
    root = (SRC / "lib.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() != "lib.rs"
    }
    standalone = (ROOT / "src/rust/crates/fathomdb-engine/tests/slice90_embed_dispatch.rs").read_text()
    return root, modules, standalone


def owner_errors(root: str, modules: dict[str, str], standalone: str) -> list[str]:
    errors = [f"root retains {name}" for name in FAMILY.findall(root)]
    owner = modules.get("embed_dispatch.rs", "")
    core = modules.get("embed_dispatch/core.rs", "")
    for path, source in modules.items():
        if path != "embed_dispatch.rs":
            errors.extend(
                f"{path} wrongly defines {name}"
                for name in FAMILY.findall(source)
                if path != "embed_dispatch/core.rs" or name not in CORE_METHODS
            )
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("embed_dispatch gates whole owner module")
    if not re.search(r"(?m)^mod core;$", owner) or not re.search(r"(?m)^pub\(crate\) use core::\*;$", owner):
        errors.append("embed_dispatch does not expose one ordinary core module")
    if "macro_rules!" in owner:
        errors.append("embed_dispatch hides adapter edges in a macro")
    impls = list(IMPL.finditer(owner))
    for impl in impls:
        if any(CFG.match(attr) for attr in prelude(owner, impl.start())):
            errors.append("embed_dispatch gates Engine impl")
    for name in METHODS:
        found = [(match, impl) for impl in impls for match in FAMILY.finditer(impl.group())
                 if match.group(1) == name]
        if len(found) != 1 or owner.count("fn " + name) != 1:
            errors.append(f"embed_dispatch lacks one Engine::{name}")
            continue
        match, impl = found[0]
        declaration = impl.group()[match.start():match.end()]
        if not declaration.lstrip().startswith("pub fn "):
            errors.append(f"embed_dispatch changes Engine::{name} visibility")
        attrs = prelude(impl.group(), match.start())
        if [attr for attr in attrs if CFG.match(attr)] != [HOOKS] or "#[doc(hidden)]" in attrs:
            errors.append(f"embed_dispatch changes Engine::{name} attrs")
    if not core:
        errors.append("embed_dispatch lacks standalone core")
    else:
        body = re.sub(r"(?m)//[^\n]*", "", core)
        if re.search(r"\b(?:Engine|Connection|ProjectionRuntime)\b|\brusqlite\b|\bcrate::(?:open|operator|projection|read_api|search)", body):
            errors.append("standalone dispatch core depends on Engine or another runtime owner")
        if "macro_rules!" in body:
            errors.append("standalone dispatch core hides edges in a macro")
    root_module = re.search(r"(?m)^mod embed_dispatch;$", root)
    if not root_module or any(CFG.match(attr) for attr in prelude(root, root_module.start())):
        errors.append("root gates embed_dispatch module")
    reexport = re.search(r"(?m)^pub use embed_dispatch::d27_observation::D27Observation;$", root)
    if not reexport or [attr for attr in prelude(root, reexport.start()) if CFG.match(attr)] != [HOOKS]:
        errors.append("root changes public D27Observation path or gate")
    if not re.search(r'(?m)^#\[path = "\.\./src/embed_dispatch/core\.rs"\]$\nmod embed_dispatch;$', standalone):
        errors.append("standalone dispatcher test does not include Engine-free core")
    return errors


class D27AdapterOwnerTest(unittest.TestCase):
    def test_current_source_has_exact_owner_and_standalone_core(self) -> None:
        self.assertEqual(owner_errors(*inventory()), [])

    def test_complete_owner_rejects_root_wrong_owner_and_hidden_adapter(self) -> None:
        root, modules, standalone = inventory()
        self.assertEqual(owner_errors(root, modules, standalone), [])
        for visibility, name in (("", "begin_d27_observation_new_for_test"),
                                 ("pub(crate) ", "d27_observation_new")):
            mutant = root + f"\nimpl Engine {{\n    {visibility}fn {name}(&self) {{}}\n}}\n"
            self.assertIn(f"root retains {name}", owner_errors(mutant, modules, standalone))
        trait_mutant = root + "\nimpl Observation for Engine {\n    fn with_d27_foreground_owner_for_test_new(&self) {}\n}\n"
        self.assertIn("root retains with_d27_foreground_owner_for_test_new",
                      owner_errors(trait_mutant, modules, standalone))
        changed = modules | {"open.rs": modules["open.rs"] + "\nfn begin_d27_observation_new() {}\n"}
        self.assertIn("open.rs wrongly defines begin_d27_observation_new",
                      owner_errors(root, changed, standalone))
        changed = modules | {"embed_dispatch.rs": modules["embed_dispatch.rs"] + "\nmacro_rules! hidden_adapter { () => {} }\n"}
        self.assertIn("embed_dispatch hides adapter edges in a macro", owner_errors(root, changed, standalone))

    def test_cfg_core_purity_and_standalone_include_mutants(self) -> None:
        root, modules, standalone = inventory()
        self.assertEqual(owner_errors(root, modules, standalone), [])
        owner = modules["embed_dispatch.rs"]
        for attr in ('#[cfg(feature = "operator")]',
                     '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            for name in METHODS:
                changed = owner.replace("    pub fn " + name, "    " + attr + "\n    pub fn " + name, 1)
                self.assertIn(f"embed_dispatch changes Engine::{name} attrs",
                              owner_errors(root, modules | {"embed_dispatch.rs": changed}, standalone))
            changed = owner.replace("impl Engine {", attr + "\nimpl Engine {", 1)
            self.assertIn("embed_dispatch gates Engine impl",
                          owner_errors(root, modules | {"embed_dispatch.rs": changed}, standalone))
            self.assertIn("embed_dispatch gates whole owner module",
                          owner_errors(root, modules | {"embed_dispatch.rs": "#!" + attr[1:] + "\n" + owner}, standalone))
            changed_root = root.replace("mod embed_dispatch;", attr + "\nmod embed_dispatch;", 1)
            self.assertIn("root gates embed_dispatch module", owner_errors(changed_root, modules, standalone))
        changed = modules | {"embed_dispatch/core.rs": modules["embed_dispatch/core.rs"] + "\nfn hidden_edge(_: crate::Engine) {}\n"}
        self.assertIn("standalone dispatch core depends on Engine or another runtime owner",
                      owner_errors(root, changed, standalone))
        changed = standalone.replace("../src/embed_dispatch/core.rs", "../src/embed_dispatch.rs", 1)
        self.assertIn("standalone dispatcher test does not include Engine-free core",
                      owner_errors(root, modules, changed))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-d27-adapter-owner",
                      (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
