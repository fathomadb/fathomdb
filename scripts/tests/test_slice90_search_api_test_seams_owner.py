#!/usr/bin/env python3
"""Guard approved search API test seams and unchanged public paths."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
METHODS = {
    "_graph_frontier_stats_for_test": (None, False),
    "set_search_limit_for_test": (None, True),
    "set_recency_reweight_enabled_for_test": (None, True),
    "set_importance_reweight_enabled_for_test": (None, True),
    "set_vector_stage_only_for_test": (None, True),
}
FREE = {
    "vector_phase1_sql_for_test": (None, True),
    "slice35_ranked_eligibility_sql_for_test": (None, True),
    "take_slice71_search_statement_trace_for_test": ('#[cfg(feature = "test-hooks")]', True),
}
FAMILY_NAME = (
    r"(?:_graph_frontier_stats_for_test|set_search_limit_for_test|"
    r"set_recency_reweight_enabled_for_test|set_importance_reweight_enabled_for_test|"
    r"set_vector_stage_only_for_test|vector_phase1_sql_for_test|"
    r"slice35_ranked_eligibility_sql_for_test|take_slice71_search_statement_trace_for_test)\w*"
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
    owner = (SRC / "search_api.rs").read_text()
    other = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "search_api.rs")
    }
    return root, owner, other


def owner_errors(root: str, owner: str, other: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for _, name in DECLARATION.findall(root)]
    for path, source in other.items():
        errors.extend(f"{path} wrongly defines {name}" for _, name in DECLARATION.findall(source))
    module = re.search(r"^mod search_api;", root, re.M)
    if not module or any(CFG.match(attr) for attr in prelude(root, module.start())):
        errors.append("root gates search API module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("search API gates whole owner module")
    impls = list(re.finditer(r"(?ms)^impl Engine \{.*?^}", owner))
    for impl in impls:
        if any(CFG.match(attr) for attr in prelude(owner, impl.start())):
            errors.append("search API gates Engine impl")
    for name, (gate, hidden) in METHODS.items():
        found = [(match, impl) for impl in impls for match in DECLARATION.finditer(impl.group())
                 if match.group(2) == name]
        if len(found) != 1 or found[0][0].group(1) != "pub ":
            errors.append(f"search API lacks public Engine::{name}")
            continue
        match, impl = found[0]
        attrs = prelude(impl.group(), match.start())
        if [attr for attr in attrs if CFG.match(attr)] != ([gate] if gate else []) or ("#[doc(hidden)]" in attrs) != hidden:
            errors.append(f"search API changes Engine::{name} attrs")
    for name, (gate, hidden) in FREE.items():
        found = [match for match in DECLARATION.finditer(owner) if match.group(2) == name and match.start() == owner.rfind("\n", 0, match.start()) + 1]
        # The free functions are at column zero; inherent methods are indented.
        found = [match for match in found if match.group().startswith("pub fn ")]
        if len(found) != 1:
            errors.append(f"search API lacks public free {name}")
        else:
            attrs = prelude(owner, found[0].start())
            if [attr for attr in attrs if CFG.match(attr)] != ([gate] if gate else []) or ("#[doc(hidden)]" in attrs) != hidden:
                errors.append(f"search API changes free {name} attrs")
            if name != "take_slice71_search_statement_trace_for_test" and "#[must_use]" not in attrs:
                errors.append(f"search API drops must_use {name}")
        export = re.search(r"(?m)^pub use search_api::" + name + r";$", root)
        if not export:
            errors.append(f"root loses {name} public path")
        else:
            attrs = prelude(root, export.start())
            if [attr for attr in attrs if CFG.match(attr)] != ([gate] if gate else []) or ("#[doc(hidden)]" in attrs) != hidden:
                errors.append(f"root changes {name} export attrs")
    owner_names = [name for _, name in DECLARATION.findall(owner)]
    for name in owner_names:
        if name not in METHODS and name not in FREE:
            errors.append(f"search API adds unlisted {name}")
    return errors


def complete_fixture() -> tuple[str, str, dict[str, str]]:
    root, owner, other = inventory()
    moved_methods = []
    for name in METHODS:
        pattern = re.compile(r"(?m)^(?:    (?:///[^\n]*|#\[[^\n]*\])\n)*    pub fn " + name + r"\b[\s\S]*?^    }\n")
        match = pattern.search(root)
        if match:
            moved_methods.append(match.group())
            root = root[:match.start()] + root[match.end():]
    if moved_methods:
        owner += "\nimpl Engine {\n" + "\n".join(moved_methods) + "}\n"
    for name, (gate, hidden) in FREE.items():
        pattern = re.compile(r"(?m)^(?:(?:///[^\n]*|#\[[^\n]*\])\n)*pub fn " + name + r"\b[\s\S]*?^}\n")
        match = pattern.search(root)
        if match:
            owner += "\n" + match.group()
            root = root[:match.start()] + root[match.end():]
        export = (gate + "\n" if gate else "") + ("#[doc(hidden)]\n" if hidden else "") + f"pub use search_api::{name};"
        if export not in root:
            root += "\n" + export + "\n"
    return root, owner, other


class SearchApiTestSeamsOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner_and_public_paths(self) -> None:
        self.assertEqual(owner_errors(*inventory()), [])

    def test_complete_owner_and_all_source_family_mutants(self) -> None:
        root, owner, other = complete_fixture()
        self.assertEqual(owner_errors(root, owner, other), [])
        for declaration, name in (
            ("fn vector_phase1_sql_for_test_new() {}", "vector_phase1_sql_for_test_new"),
            ("impl Engine {\n    pub(crate) fn set_search_limit_for_test_new(&self) {}\n}", "set_search_limit_for_test_new"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, other))
        for path in ("search.rs", "future_owner.rs"):
            changed = other | {path: other.get(path, "") + "\nfn take_slice71_search_statement_trace_for_test_new() {}"}
            self.assertIn(f"{path} wrongly defines take_slice71_search_statement_trace_for_test_new", owner_errors(root, owner, changed))
        changed = owner + "\nfn vector_phase1_sql_for_test_new() {}"
        self.assertIn("search API adds unlisted vector_phase1_sql_for_test_new", owner_errors(root, changed, other))

    def test_cfg_cfg_attr_and_export_mutants(self) -> None:
        root, owner, other = complete_fixture()
        self.assertEqual(owner_errors(root, owner, other), [])
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            impl_start = owner.rfind("impl Engine {", 0, owner.index("pub fn _graph_frontier_stats_for_test"))
            changed = owner[:impl_start] + attr + "\n" + owner[impl_start:]
            self.assertIn("search API gates Engine impl", owner_errors(root, changed, other))
            changed = owner.replace("    pub fn set_search_limit_for_test(", "    " + attr + "\n    pub fn set_search_limit_for_test(", 1)
            self.assertIn("search API changes Engine::set_search_limit_for_test attrs", owner_errors(root, changed, other))
            changed = owner.replace("pub fn vector_phase1_sql_for_test(", attr + "\npub fn vector_phase1_sql_for_test(", 1)
            self.assertIn("search API changes free vector_phase1_sql_for_test attrs", owner_errors(root, changed, other))
            changed = root.replace("mod search_api;", attr + "\nmod search_api;", 1)
            self.assertIn("root gates search API module", owner_errors(changed, owner, other))
            changed = root.replace("pub use search_api::vector_phase1_sql_for_test;", attr + "\npub use search_api::vector_phase1_sql_for_test;", 1)
            self.assertIn("root changes vector_phase1_sql_for_test export attrs", owner_errors(changed, owner, other))
            self.assertIn("search API gates whole owner module", owner_errors(root, "#!" + attr[1:] + "\n" + owner, other))
        changed = owner.replace("#[must_use]\npub fn vector_phase1_sql_for_test", "pub fn vector_phase1_sql_for_test", 1)
        self.assertIn("search API drops must_use vector_phase1_sql_for_test", owner_errors(root, changed, other))
        changed = root.replace("pub use search_api::slice35_ranked_eligibility_sql_for_test;", "", 1)
        self.assertIn("root loses slice35_ranked_eligibility_sql_for_test public path", owner_errors(changed, owner, other))
        trace_gate = '#[cfg(feature = "test-hooks")]\n#[doc(hidden)]\npub fn take_slice71_search_statement_trace_for_test'
        changed = owner.replace(trace_gate, '#[doc(hidden)]\npub fn take_slice71_search_statement_trace_for_test', 1)
        self.assertIn("search API changes free take_slice71_search_statement_trace_for_test attrs", owner_errors(root, changed, other))
        trace_export = '#[cfg(feature = "test-hooks")]\n#[doc(hidden)]\npub use search_api::take_slice71_search_statement_trace_for_test;'
        changed = root.replace(trace_export, '#[doc(hidden)]\npub use search_api::take_slice71_search_statement_trace_for_test;', 1)
        self.assertIn("root changes take_slice71_search_statement_trace_for_test export attrs", owner_errors(changed, owner, other))

    def test_fast_registration(self) -> None:
        self.assertIn("fast test-slice90-search-api-test-seams-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
