#!/usr/bin/env python3
"""Guard mean test seams and their unchanged public paths."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
MODULE = "mean_centering_internals_for_test"
METHOD = "force_next_recompute_failure_for_test"
REEXPORT = "pub use mean::mean_centering_internals_for_test;"
MODULE_FAMILY = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?mod[ \t]+(mean_centering_internals_for_test\w*)\b", re.M)
METHOD_FAMILY = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+(force_next_recompute_failure_for_test\w*)\b[ \t]*\(", re.M)
API = (
    "pub struct AccumulatorHandle(MeanAccumulator);",
    "pub fn new_mean_accumulator(dim: usize) -> AccumulatorHandle",
    "pub fn accumulator_add(handle: &mut AccumulatorHandle, v: &[f32])",
    "pub fn accumulator_materialize(handle: &AccumulatorHandle) -> Vec<f32>",
    "pub fn accumulator_count(handle: &AccumulatorHandle) -> u64",
    "pub fn run_requantize_pass(rows: &[(i64, Vec<u8>)], mean: &[f32]) -> (u64, Vec<EmbedderEvent>)",
)


def prelude(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if stripped.startswith("#["):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "//")):
            break
    return found


def cfg_attrs(source: str, start: int) -> list[str]:
    return [line for line in prelude(source, start) if re.match(r"#\[\s*cfg(?:_attr)?\b", line)]


def source_inventory() -> tuple[str, str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    owner = (SRC / "mean.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "mean.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in MODULE_FAMILY.findall(root) + METHOD_FAMILY.findall(root)]
    for path, source in modules.items():
        errors.extend(f"{path} wrongly defines {name}" for name in MODULE_FAMILY.findall(source) + METHOD_FAMILY.findall(source))
    root_module = re.search(r"^mod mean;", root, re.M)
    if not root_module or cfg_attrs(root, root_module.start()):
        errors.append("root gates mean owner module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("mean gates whole owner module")
    errors.extend(f"mean unexpectedly defines {name}" for name in MODULE_FAMILY.findall(owner) + METHOD_FAMILY.findall(owner) if name not in (MODULE, METHOD))
    module = list(re.finditer(r"^pub mod mean_centering_internals_for_test \{", owner, re.M))
    if len(module) != 1 or MODULE_FAMILY.findall(owner).count(MODULE) != 1:
        errors.append("mean lacks one public internals module")
    elif cfg_attrs(owner, module[0].start()) or "#[doc(hidden)]" not in prelude(owner, module[0].start()):
        errors.append("mean changes internals module attrs")
    if module:
        module_end = re.search(r"^}\s*$", owner[module[0].end() :], re.M)
        body = owner[module[0].end() : module[0].end() + module_end.start()] if module_end else owner[module[0].end() :]
        if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", body):
            errors.append("mean gates internals module body")
    reexport = list(re.finditer(r"^" + re.escape(REEXPORT) + r"$", root, re.M))
    if len(reexport) != 1 or cfg_attrs(root, reexport[0].start()) or "#[doc(hidden)]" not in prelude(root, reexport[0].start()):
        errors.append("root loses hidden ungated internals path")
    for marker in API:
        if marker not in owner:
            errors.append(f"mean internals lacks {marker.split('(')[0]}")
    for name in ("AccumulatorHandle", "new_mean_accumulator", "accumulator_add", "accumulator_materialize", "accumulator_count", "run_requantize_pass"):
        declaration = re.search(r"^[ \t]+pub (?:struct|fn) " + name + r"\b", body if module else "", re.M)
        if not declaration or cfg_attrs(body, declaration.start()):
            errors.append(f"mean gates internals {name}")
    for name in ("new_mean_accumulator", "accumulator_materialize", "accumulator_count", "run_requantize_pass"):
        found = list(re.finditer(r"^    pub fn " + name + r"\(", owner, re.M))
        if len(found) != 1 or "#[must_use]" not in prelude(owner, found[0].start()):
            errors.append(f"mean internals loses must_use {name}")
    if "super::run_requantize_pass(rows, mean)" not in owner:
        errors.append("mean internals loses requantize forwarding")
    method = list(re.finditer(r"^[ \t]+pub fn force_next_recompute_failure_for_test\(&self\) \{", owner, re.M))
    if len(method) != 1 or METHOD_FAMILY.findall(owner).count(METHOD) != 1:
        errors.append("mean lacks one public recompute fault hook")
    else:
        impls = list(re.finditer(r"^impl Engine \{", owner[: method[0].start()], re.M))
        if not impls or cfg_attrs(owner, impls[-1].start()):
            errors.append("mean gates Engine impl")
        attrs = prelude(owner, method[0].start())
        if "#[cfg(debug_assertions)]" not in attrs or "#[doc(hidden)]" not in attrs or cfg_attrs(owner, method[0].start()) != ["#[cfg(debug_assertions)]"]:
            errors.append("mean changes recompute fault hook attrs")
        if "self.projection_runtime.shared.force_recompute_failure.store(true, Ordering::SeqCst);" not in owner[method[0].start() :]:
            errors.append("mean changes recompute fault hook body")
    return errors


class MeanTestSeamsOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner_and_public_paths(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_root_and_other_owner_families(self) -> None:
        root, owner, modules = source_inventory()
        block = re.search(r"(?ms)^#\[doc\(hidden\)\]\npub mod mean_centering_internals_for_test \{.*?^}\n", root)
        if block:
            owner += "\n" + block.group()
            root = root[: block.start()] + root[block.end() :]
        method = re.search(r"(?ms)^    pub fn force_next_recompute_failure_for_test\(&self\) \{.*?^    }\n", root)
        if method:
            owner = owner.replace("impl Engine {", "impl Engine {\n    #[doc(hidden)]\n    #[cfg(debug_assertions)]\n" + method.group(), 1)
            root = root[: method.start()] + root[method.end() :]
        if REEXPORT not in root:
            root += "\n#[doc(hidden)]\n" + REEXPORT
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("pub mod mean_centering_internals_for_test_new {}", "mean_centering_internals_for_test_new"),
            ("impl Engine {\n    fn force_next_recompute_failure_for_test_new(&self) {}\n}", "force_next_recompute_failure_for_test_new"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("embedding.rs", "pub(crate) mod mean_centering_internals_for_test_new {}", "mean_centering_internals_for_test_new"),
            ("projection_runtime.rs", "impl Engine {\n    pub fn force_next_recompute_failure_for_test_new(&self) {}\n}", "force_next_recompute_failure_for_test_new"),
        ):
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "fn force_next_recompute_failure_for_test_new() {}"}
        self.assertIn("future_owner.rs wrongly defines force_next_recompute_failure_for_test_new", owner_errors(root, owner, arbitrary))

    def test_cfg_cfg_attr_and_public_attr_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace("pub mod mean_centering_internals_for_test {", attr + "\npub mod mean_centering_internals_for_test {", 1)
            self.assertIn("mean changes internals module attrs", owner_errors(root, changed, modules))
            changed = owner.replace("impl Engine {", attr + "\nimpl Engine {", 1)
            self.assertIn("mean gates Engine impl", owner_errors(root, changed, modules))
            changed = owner.replace("    pub fn force_next_recompute_failure_for_test(", "    " + attr + "\n    pub fn force_next_recompute_failure_for_test(", 1)
            self.assertIn("mean changes recompute fault hook attrs", owner_errors(root, changed, modules))
            changed = root.replace(REEXPORT, attr + "\n" + REEXPORT, 1)
            self.assertIn("root loses hidden ungated internals path", owner_errors(changed, owner, modules))
            changed = owner.replace("    pub fn accumulator_add(", "    " + attr + "\n    pub fn accumulator_add(", 1)
            self.assertIn("mean gates internals accumulator_add", owner_errors(root, changed, modules))
            changed = owner.replace("pub mod mean_centering_internals_for_test {", "pub mod mean_centering_internals_for_test {\n    #!" + attr[1:], 1)
            self.assertIn("mean gates internals module body", owner_errors(root, changed, modules))
        changed = owner.replace("#[cfg(debug_assertions)]\n    pub fn force_next_recompute_failure_for_test(", "    pub fn force_next_recompute_failure_for_test(", 1)
        self.assertIn("mean changes recompute fault hook attrs", owner_errors(root, changed, modules))
        changed = root.replace("#[doc(hidden)]\n" + REEXPORT, REEXPORT, 1)
        self.assertIn("root loses hidden ungated internals path", owner_errors(changed, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("mean gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-mean-test-seams-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
