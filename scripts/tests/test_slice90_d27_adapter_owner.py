#!/usr/bin/env python3
"""Keep the D27 observation adapter visible while its dispatch core stays standalone."""

from pathlib import Path
import re
import unittest

from rust_source_lex import brace_depth, outer_attributes as prelude, rust_code_tokens


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
HOOKS = '#[cfg(feature = "test-hooks")]'
HOOKS_NORMALIZED = '#[cfg(feature="test-hooks")]'
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


def core_dependency_escape(core: str) -> bool:
    observation_import = re.compile(
        r'(?m)^#\[cfg\(feature = "test-hooks"\)\]$\nuse super::d27_observation;$'
    )
    matches = list(observation_import.finditer(core))
    if len(matches) != 1:
        return True
    core = core[:matches[0].start()] + core[matches[0].end():]
    tokens = rust_code_tokens(core)
    for index, token in enumerate(tokens):
        if token in ("Engine", "Connection", "ProjectionRuntime", "rusqlite", "super"):
            return True
        if token == "crate" and tokens[index - 2:index + 2] != ["pub", "(", "crate", ")"]:
            return True
    return False


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
    observation_module = re.search(r"(?m)^pub\(crate\) mod d27_observation;$", owner)
    if not observation_module or [attr for attr in prelude(owner, observation_module.start()) if CFG.match(attr)] != [HOOKS_NORMALIZED]:
        errors.append("embed_dispatch changes D27 observation type owner")
    if "macro_rules!" in owner:
        errors.append("embed_dispatch hides adapter edges in a macro")
    impls = list(IMPL.finditer(owner))
    for impl in impls:
        if brace_depth(owner, impl.start()) != 0:
            errors.append("embed_dispatch nests Engine impl")
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
        if [attr for attr in attrs if CFG.match(attr)] != [HOOKS_NORMALIZED] or "#[doc(hidden)]" in attrs:
            errors.append(f"embed_dispatch changes Engine::{name} attrs")
    if not core:
        errors.append("embed_dispatch lacks standalone core")
    else:
        if core_dependency_escape(core):
            errors.append("standalone dispatch core depends on Engine or another runtime owner")
        if "macro_rules" in rust_code_tokens(core):
            errors.append("standalone dispatch core hides edges in a macro")
    root_module = re.search(r"(?m)^mod embed_dispatch;$", root)
    if not root_module or any(CFG.match(attr) for attr in prelude(root, root_module.start())):
        errors.append("root gates embed_dispatch module")
    reexport = re.search(r"(?m)^pub use embed_dispatch::d27_observation::D27Observation;$", root)
    if not reexport or [attr for attr in prelude(root, reexport.start()) if CFG.match(attr)] != [HOOKS_NORMALIZED]:
        errors.append("root changes public D27Observation path or gate")
    if not re.search(r'(?m)^#\[path = "\.\./src/embed_dispatch/core\.rs"\]$\nmod embed_dispatch;$', standalone):
        errors.append("standalone dispatcher test does not include Engine-free core")
    if not re.search(r'(?m)^#\[path = "\.\./src/embed_dispatch/d27_observation\.rs"\]$\nmod d27_observation;$', standalone):
        errors.append("standalone dispatcher test does not include observation types")
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
                     '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]',
                     '#[cfg_attr(\n    all(feature = "default-embedder", test),\n'
                     '    cfg(feature = "test-hooks")\n)]'):
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
            prefix = 'const RAW_GATE_WITNESS: &str = r###"quote " /*"###;\n'
            changed = owner.replace("impl Engine {", prefix + attr + "\n/* end */\nimpl Engine {", 1)
            self.assertIn("embed_dispatch gates Engine impl",
                          owner_errors(root, modules | {"embed_dispatch.rs": changed}, standalone))
            changed_root = root.replace("mod embed_dispatch;", prefix + attr + "\n/* end */\nmod embed_dispatch;", 1)
            self.assertIn("root gates embed_dispatch module", owner_errors(changed_root, modules, standalone))
        changed = modules | {"embed_dispatch/core.rs": modules["embed_dispatch/core.rs"] + "\nfn hidden_edge(_: crate::Engine) {}\n"}
        self.assertIn("standalone dispatch core depends on Engine or another runtime owner",
                      owner_errors(root, changed, standalone))
        for path in ("connection_runtime", "reader_pool", "wal_runtime"):
            with self.subTest(path=path):
                edge = (f'\n#[cfg(not(test))]\n#[allow(unused_imports)]\n'
                        f'use crate::{path} as hidden_runtime;\n')
                changed = modules | {"embed_dispatch/core.rs": modules["embed_dispatch/core.rs"] + edge}
                self.assertIn("standalone dispatch core depends on Engine or another runtime owner",
                              owner_errors(root, changed, standalone))
        changed = modules | {"embed_dispatch/core.rs": modules["embed_dispatch/core.rs"] +
                   "\n#[allow(unused_imports)] use super::d27_observation as hidden_adapter;\n"}
        self.assertIn("standalone dispatch core depends on Engine or another runtime owner",
                      owner_errors(root, changed, standalone))
        long_raw = 'r' + '#' * 17 + '"quote " /*"' + '#' * 17
        for literal in ('"/*"', 'b"/*"', 'c"/*"', 'r#"/*"#', 'r###"quote " /*"###',
                        'br##"quote " /*"##', 'cr##"quote " /*"##', long_raw,
                        "'x'", "b'x'"):
            with self.subTest(literal=literal):
                edge = (f'\nfn literal_witness() {{ let _ = {literal}; }}\n'
                        "fn lifetime_identity<'a>(value: &'a str) -> &'a str { value }\n"
                        '#[cfg(not(test))]\n#[allow(unused_imports)]\n'
                        'use crate :: runtime_lifecycle as hidden_lifecycle;\n'
                        '/* nested /* ignored */ end */\n')
                changed = modules | {"embed_dispatch/core.rs": modules["embed_dispatch/core.rs"] + edge}
                self.assertIn("standalone dispatch core depends on Engine or another runtime owner",
                              owner_errors(root, changed, standalone))
        harmless = modules["embed_dispatch/core.rs"] + (
            '\n// use crate::runtime_lifecycle as commented;\n'
            '/* use crate::reader_pool as commented; /* nested */ */\n'
            'const PATH_TEXT: &str = r#"crate::wal_runtime"#;\n'
            f'fn literal_controls() {{ let _ = {long_raw}; let _ = c"/*"; '
            'let _ = cr##"quote " /*"##; }\n'
        )
        self.assertEqual(owner_errors(root, modules | {"embed_dispatch/core.rs": harmless}, standalone), [])
        with self.assertRaises(ValueError):
            owner_errors(root, modules | {"embed_dispatch/core.rs": modules["embed_dispatch/core.rs"] +
                                          '\nconst BAD: &str = r#"unterminated;\n'}, standalone)
        with self.assertRaises(ValueError):
            owner_errors(root, modules | {"embed_dispatch/core.rs": modules["embed_dispatch/core.rs"] +
                                          "\nconst BAD: u8 = b'unterminated;\n"}, standalone)

    def test_cfg_module_ancestor_cannot_hide_adapter(self) -> None:
        root, modules, standalone = inventory()
        owner = modules["embed_dispatch.rs"]
        impl = IMPL.search(owner)
        self.assertIsNotNone(impl)
        wrapped = (owner[:impl.start()] + '#[cfg(feature = "operator")]\n'
                   'mod gated_adapter {\nuse super::*;\n' + impl.group() + '\n}\n' +
                   owner[impl.end():])
        self.assertIn("embed_dispatch nests Engine impl",
                      owner_errors(root, modules | {"embed_dispatch.rs": wrapped}, standalone))
        changed = standalone.replace("../src/embed_dispatch/core.rs", "../src/embed_dispatch.rs", 1)
        self.assertIn("standalone dispatcher test does not include Engine-free core",
                      owner_errors(root, modules, changed))
        changed = owner.replace('pub(crate) mod d27_observation;',
                                'pub(crate) use core::d27_observation;', 1)
        self.assertIn("embed_dispatch changes D27 observation type owner",
                      owner_errors(root, modules | {"embed_dispatch.rs": changed}, standalone))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-d27-adapter-owner",
                      (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
