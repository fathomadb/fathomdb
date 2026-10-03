#!/usr/bin/env python3
"""Guard mapped dependency-trace Engine test seams and feature paths."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
GATE = '#[cfg(feature = "test-hooks")]'
METHODS = {
    "dependency_trace_query_plans_for_test": False,
    "dependency_trace_candidate_queries_for_test": True,
    "measure_dependency_trace_for_test": False,
    "seed_hidden_dependency_trace_fixture_for_test": True,
}
FAMILY_NAME = (
    r"(?:dependency_trace_query_plans_for_test|dependency_trace_candidate_queries_for_test|"
    r"measure_dependency_trace_for_test|seed_hidden_dependency_trace_fixture_for_test)\w*"
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


def inventory() -> tuple[str, str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    owner = (SRC / "dependency_trace.rs").read_text()
    other = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "dependency_trace.rs")
    }
    return root, owner, other


def owner_errors(root: str, owner: str, other: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for _, name in DECLARATION.findall(root)]
    for path, source in other.items():
        errors.extend(f"{path} wrongly defines {name}" for _, name in DECLARATION.findall(source))
    module = re.search(r"^mod dependency_trace;", root, re.M)
    if not module or any(CFG.match(attr) for attr in prelude(root, module.start())):
        errors.append("root gates dependency-trace module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("dependency-trace owner gates whole module")
    impls = list(re.finditer(r"(?ms)^impl Engine \{.*?^}", owner))
    for impl in impls:
        if any(CFG.match(attr) for attr in prelude(owner, impl.start())):
            errors.append("dependency-trace owner gates Engine impl")
    for name, hidden in METHODS.items():
        found = [(match, impl) for impl in impls for match in DECLARATION.finditer(impl.group())
                 if match.group(2) == name]
        if len(found) != 1 or found[0][0].group(1) != "pub ":
            errors.append(f"dependency-trace owner lacks public Engine::{name}")
            continue
        match, impl = found[0]
        attrs = prelude(impl.group(), match.start())
        if [attr for attr in attrs if CFG.match(attr)] != [GATE] or ("#[doc(hidden)]" in attrs) != hidden:
            errors.append(f"dependency-trace owner changes Engine::{name} attrs")
        if [found_name for _, found_name in DECLARATION.findall(owner)].count(name) != 1:
            errors.append(f"dependency-trace owner duplicates Engine::{name}")
    for _, name in DECLARATION.findall(owner):
        if name not in METHODS:
            errors.append(f"dependency-trace owner adds unlisted {name}")
    return errors


def complete_fixture() -> tuple[str, str, dict[str, str]]:
    root, owner, other = inventory()
    moved = []
    for name in METHODS:
        pattern = re.compile(r"(?m)^(?:    (?:///[^\n]*|#\[[^\n]*\])\n)*    pub fn " + name + r"\b[\s\S]*?^    }\n")
        match = pattern.search(root)
        if match:
            moved.append(match.group())
            root = root[:match.start()] + root[match.end():]
    if moved:
        owner += "\nimpl Engine {\n" + "\n".join(moved) + "}\n"
    return root, owner, other


class DependencyTraceTestSeamsOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner_and_feature_paths(self) -> None:
        self.assertEqual(owner_errors(*inventory()), [])

    def test_complete_owner_rejects_wrong_owner_and_unlisted_families(self) -> None:
        root, owner, other = complete_fixture()
        self.assertEqual(owner_errors(root, owner, other), [])
        for declaration, name in (
            ("impl Engine {\n    fn measure_dependency_trace_for_test_new(&self) {}\n}", "measure_dependency_trace_for_test_new"),
            ("impl Engine {\n    pub(crate) fn dependency_trace_query_plans_for_test(&self) {}\n}", "dependency_trace_query_plans_for_test"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, other))
        for path in ("dependency.rs", "future_owner.rs"):
            changed = other | {path: other.get(path, "") + "\nimpl Engine {\n    pub fn seed_hidden_dependency_trace_fixture_for_test_new(&self) {}\n}"}
            self.assertIn(f"{path} wrongly defines seed_hidden_dependency_trace_fixture_for_test_new", owner_errors(root, owner, changed))
        changed = owner + "\nimpl Engine {\n    fn dependency_trace_candidate_queries_for_test_new(&self) {}\n}"
        self.assertIn("dependency-trace owner adds unlisted dependency_trace_candidate_queries_for_test_new", owner_errors(root, changed, other))

    def test_cfg_cfg_attr_item_impl_file_and_root_module_mutants(self) -> None:
        root, owner, other = complete_fixture()
        self.assertEqual(owner_errors(root, owner, other), [])
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace("    pub fn dependency_trace_query_plans_for_test(", "    " + attr + "\n    pub fn dependency_trace_query_plans_for_test(", 1)
            self.assertIn("dependency-trace owner changes Engine::dependency_trace_query_plans_for_test attrs", owner_errors(root, changed, other))
            impl_start = owner.rfind("impl Engine {", 0, owner.index("pub fn measure_dependency_trace_for_test"))
            changed = owner[:impl_start] + attr + "\n" + owner[impl_start:]
            self.assertIn("dependency-trace owner gates Engine impl", owner_errors(root, changed, other))
            changed = root.replace("mod dependency_trace;", attr + "\nmod dependency_trace;", 1)
            self.assertIn("root gates dependency-trace module", owner_errors(changed, owner, other))
            self.assertIn("dependency-trace owner gates whole module", owner_errors(root, "#!" + attr[1:] + "\n" + owner, other))
        changed = owner.replace('    #[cfg(feature = "test-hooks")]\n    pub fn dependency_trace_query_plans_for_test(', '    pub fn dependency_trace_query_plans_for_test(', 1)
        self.assertIn("dependency-trace owner changes Engine::dependency_trace_query_plans_for_test attrs", owner_errors(root, changed, other))
        changed = owner.replace('    #[doc(hidden)]\n    pub fn seed_hidden_dependency_trace_fixture_for_test(', '    pub fn seed_hidden_dependency_trace_fixture_for_test(', 1)
        self.assertIn("dependency-trace owner changes Engine::seed_hidden_dependency_trace_fixture_for_test attrs", owner_errors(root, changed, other))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-dependency-trace-test-seams-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
