#!/usr/bin/env python3
"""Guard the exact approved Slice 90 root residuals and SQL fixture docs."""

from pathlib import Path
import hashlib
import re
import unittest

from rust_source_lex import brace_depth, outer_attributes


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "src/rust/crates/fathomdb-engine/src/lib.rs"
ROOT_METHODS = {
    "path",
    "ensure_open",
    "execute_for_test",
    "run_one_thread_poison_for_test",
    "query_i64_col_for_test",
    "query_text_col_for_test",
}
VISIBILITY = r"(?:pub(?:\([^)]*\))?\s+)?"
METHOD_ATTRIBUTES = {
    "path": {"#[must_use]"},
    "ensure_open": set(),
    "execute_for_test": {"#[doc(hidden)]", "#[cfg(any(test,debug_assertions,feature=\"test-hooks\"))]"},
    "run_one_thread_poison_for_test": {"#[doc(hidden)]", "#[cfg(debug_assertions)]"},
    "query_i64_col_for_test": {"#[doc(hidden)]"},
    "query_text_col_for_test": {"#[doc(hidden)]"},
}


def root_engine_body(source: str) -> str:
    match = re.search(r"(?ms)^impl Engine \{.*?^}", source)
    if match is None:
        raise AssertionError("root Engine facade impl missing")
    return match.group()


def root_method_names(source: str) -> set[str]:
    return set(re.findall(r"(?m)^    " + VISIBILITY + r"fn\s+(\w+)\s*\(",
                          root_engine_body(source)))


def root_declarations(source: str) -> list[tuple[str, str]]:
    declarations = []
    for match in re.finditer(
        r"(?m)^" + VISIBILITY + r"(struct|enum|const|static|fn|impl|type|trait)\s+([^\s{(:]+)",
        source,
    ):
        if brace_depth(source, match.start()) == 0:
            declarations.append((match.group(1), match.group(2)))
    return declarations


def root_method_attributes(source: str, name: str) -> set[str]:
    match = re.search(r"(?m)^    " + VISIBILITY + r"fn\s+" + re.escape(name) + r"\(",
                      root_engine_body(source))
    if match is None:
        raise AssertionError(f"root Engine::{name} missing")
    start = source.index("impl Engine {") + match.start() + 4
    return set(outer_attributes(source, start))


def method_docs(source: str, name: str) -> str:
    match = re.search(r"(?m)^    pub fn " + re.escape(name) + r"\(", source)
    if match is None:
        raise AssertionError(f"root Engine::{name} missing")
    lines = source[:match.start()].splitlines()
    docs = []
    for line in reversed(lines):
        stripped = line.strip()
        if stripped.startswith("///"):
            docs.append(stripped.removeprefix("///").strip())
        elif stripped.startswith("#["):
            continue
        else:
            break
    return " ".join(reversed(docs))


class RootReconciliationTest(unittest.TestCase):
    def test_root_has_only_approved_facade_methods(self) -> None:
        source = SOURCE.read_text()
        methods = root_method_names(source)
        self.assertEqual(methods, ROOT_METHODS)
        for name, attributes in METHOD_ATTRIBUTES.items():
            self.assertEqual(root_method_attributes(source, name), attributes)

    def test_only_test_worker_fixture_remains_as_root_constant(self) -> None:
        source = SOURCE.read_text()
        declarations = root_declarations(source)
        self.assertEqual(declarations, [
            ("const", "PROJECTION_WORKERS"),
            ("struct", "Engine"),
            ("impl", "std"),
            ("impl", "Engine"),
        ])
        constant = source.index("const PROJECTION_WORKERS: usize = 2;")
        self.assertEqual(outer_attributes(source, constant), ["#[cfg(test)]"])
        test_module = source.index("mod tests {")
        for match in re.finditer(r"\bPROJECTION_WORKERS\b", source):
            self.assertTrue(match.start() == constant + len("const ") or match.start() > test_module)

    def test_arbitrary_writer_sql_docs_do_not_claim_read_only(self) -> None:
        source = SOURCE.read_text()
        for name in ("query_i64_col_for_test", "query_text_col_for_test"):
            with self.subTest(name=name):
                docs = method_docs(source, name)
                self.assertIn("arbitrary SQL", docs)
                self.assertIn("writer connection", docs)
                self.assertIn("may mutate", docs)
                self.assertNotIn("read-only", docs)

    def test_public_reexports_and_d27_root_ownership(self) -> None:
        source = SOURCE.read_text()
        self.assertIn("pub use embed_dispatch::d27_observation::D27Observation;", source)
        self.assertIn("pub use mean::{MeanRecomputeReport, MEAN_VEC_PIN_THRESHOLD};", source)
        self.assertIn("pub use mean::mean_centering_internals_for_test;", source)
        self.assertIn("pub use write_types::RowKind;", source)
        for name in ("begin_d27_observation_for_test", "with_d27_foreground_owner_for_test",
                     "d27_observation_for_test"):
            self.assertNotRegex(source, r"(?m)^    pub fn " + name + r"\(")

    def test_inventory_matches_exact_root_storage_blob(self) -> None:
        source_bytes = SOURCE.read_bytes()
        source = source_bytes.decode()
        fields = source.split("pub struct Engine {", 1)[1].split("\n}\n", 1)[0]
        actual = re.findall(r"(?m)^    ([a-z_][a-z_0-9]*):", fields)
        inventory = (ROOT / "dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md").read_text()
        section = inventory.split("| Storage role | Root field names |", 1)[1].split("Root also composes", 1)[0]
        documented = re.findall(r"`([a-z_][a-z_0-9]*)`", section)
        self.assertEqual(len(actual), 43)
        self.assertCountEqual(documented, actual)
        blob = hashlib.sha1(b"blob " + str(len(source_bytes)).encode() + b"\0" + source_bytes).hexdigest()
        self.assertIn("lib_rs_blob: " + blob, inventory)

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-root-reconciliation ",
                      (ROOT / "scripts/agent-test.sh").read_text())

    def test_unlisted_root_declaration_mutants(self) -> None:
        source = SOURCE.read_text()
        baseline_methods = root_method_names(source)
        baseline_declarations = root_declarations(source)
        self.assertEqual(baseline_methods, ROOT_METHODS)
        self.assertEqual(len(baseline_declarations), 4)
        for visibility in ("", "pub(crate) ", "pub(super) ", "pub "):
            with self.subTest(visibility=visibility):
                method_mutant = source.replace(
                    "impl Engine {", "impl Engine {\n    " + visibility + "fn unlisted_root_method(&self) {}", 1
                )
                self.assertIn("unlisted_root_method", root_method_names(method_mutant))
                declaration_mutant = source + "\n" + visibility + "const UNLISTED_ROOT_CONST: usize = 1;\n"
                self.assertIn(("const", "UNLISTED_ROOT_CONST"),
                              root_declarations(declaration_mutant))

    def test_extra_cfg_and_cfg_attr_mutants_change_method_attributes(self) -> None:
        source = SOURCE.read_text()
        for attribute in ('#[cfg(feature = "operator")]',
                          '#[cfg_attr(feature = "operator", cfg(test))]'):
            with self.subTest(attribute=attribute):
                mutant = source.replace(
                    "    pub fn query_i64_col_for_test(",
                    "    " + attribute + "\n    pub fn query_i64_col_for_test(", 1,
                )
                self.assertNotEqual(root_method_attributes(mutant, "query_i64_col_for_test"),
                                    METHOD_ATTRIBUTES["query_i64_col_for_test"])


if __name__ == "__main__":
    unittest.main()
