#!/usr/bin/env python3
"""Guard the ungated Engine::config accessor in runtime_configuration."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
METHOD = "pub fn config(&self) -> &EngineConfig {"
FAMILY = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(config(?:_\w+)?)\b[ \t]*\(",
    re.M,
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
    owner = (SRC / "runtime_configuration.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "runtime_configuration.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines Engine::{name}" for name in FAMILY.findall(root)]
    for path, source in modules.items():
        errors.extend(f"{path} wrongly defines Engine::{name}" for name in FAMILY.findall(source))
    module = re.search(r"^mod runtime_configuration;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates runtime_configuration module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("runtime_configuration gates whole owner module")
    errors.extend(f"runtime_configuration unexpectedly defines {name}" for name in FAMILY.findall(owner) if name != "config")
    methods = list(re.finditer(r"^    " + re.escape(METHOD) + r"$", owner, re.M))
    if len(methods) != 1 or FAMILY.findall(owner).count("config") != 1:
        errors.append("runtime_configuration lacks one public Engine::config")
        return errors
    method = methods[0]
    impls = list(re.finditer(r"^impl Engine \{", owner[: method.start()], re.M))
    if not impls or attrs(owner, impls[-1].start()):
        errors.append("runtime_configuration gates Engine impl")
    if attrs(owner, method.start()):
        errors.append("runtime_configuration gates Engine::config")
    if not re.search(r"(?ms)^    " + re.escape(METHOD) + r"\n        &self\.requested_config\n    }", owner):
        errors.append("runtime_configuration changes requested config return")
    return errors


class EngineConfigOwnerTest(unittest.TestCase):
    def test_current_source_has_one_ungated_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_root_and_other_module_methods(self) -> None:
        root, owner, modules = source_inventory()
        method = re.search(r"(?ms)^    pub fn config\(&self\) -> &EngineConfig \{.*?^    }\n", root)
        if method:
            owner += "\nimpl Engine {\n" + method.group() + "}\n"
            root = root[: method.start()] + root[method.end() :]
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("impl Engine { fn config_new(&self) {} }", "config_new"),
            ("impl ConfigView for Engine { pub(crate) fn config(&self) {} }", "config"),
        ):
            # The declaration scanner is line anchored like real Rust methods.
            declaration = declaration.replace(" { fn ", " {\n    fn ").replace(" { pub(crate) fn ", " {\n    pub(crate) fn ")
            self.assertIn(f"root wrongly defines Engine::{name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("open.rs", "impl Engine {\n    pub fn config_new(&self) {}\n}", "config_new"),
            ("read_api.rs", "impl Engine {\n    pub(crate) fn config(&self) {}\n}", "config"),
        ):
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path} wrongly defines Engine::{name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "impl ConfigView for Engine {\n    fn config_new(&self) {}\n}"}
        self.assertIn("future_owner.rs wrongly defines Engine::config_new", owner_errors(root, owner, arbitrary))

    def test_cfg_cfg_attr_and_public_body_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        changed = owner.replace("&self.requested_config", "&self.effective_config", 1)
        self.assertIn("runtime_configuration changes requested config return", owner_errors(root, changed, modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace(METHOD, attr + "\n    " + METHOD, 1)
            self.assertIn("runtime_configuration gates Engine::config", owner_errors(root, changed, modules))
            changed = owner.replace("impl Engine {", attr + "\nimpl Engine {", 1)
            self.assertIn("runtime_configuration gates Engine impl", owner_errors(root, changed, modules))
            changed = root.replace("mod runtime_configuration;", attr + "\nmod runtime_configuration;", 1)
            self.assertIn("root gates runtime_configuration module", owner_errors(changed, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("runtime_configuration gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-engine-config-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
