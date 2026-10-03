#!/usr/bin/env python3
"""Guard operator diagnostic reports and Engine method ownership."""

from pathlib import Path
import re
import unittest

from rust_source_lex import brace_depth, has_cfg_attribute, outer_attributes


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
REPORT_TYPES = (
    "VerifyEmbedderStatus",
    "VerifyEmbedderReport",
    "SchemaObject",
    "DumpSchemaReport",
    "TableRowCount",
    "DumpRowCountsReport",
    "OrphanProvenanceSource",
    "OrphanProvenanceReport",
    "DumpProfileReport",
)
METHODS = (
    "verify_embedder",
    "dump_schema",
    "dump_row_counts",
    "orphan_provenance",
    "dump_profile",
)
HELPERS = ("read_schema_objects", "order_canonical_first")
METHOD_DECL = re.compile(
    r"^    (?:(?:pub(?:\([^)]*\))?) )?fn ((?:verify_embedder|dump_|orphan_provenance)\w*)\(",
    re.M,
)


def owner_errors(root: str, owner: str, others: dict[str, str] | None = None) -> list[str]:
    errors = []
    if others is None:
        others = {
            str(path.relative_to(SRC)): path.read_text()
            for path in SRC.rglob("*.rs")
            if path not in (SRC / "lib.rs", SRC / "operator.rs")
        }
    root_module = re.search(r"(?m)^mod operator;$", root)
    if root_module is None or has_cfg_attribute(outer_attributes(root, root_module.start())):
        errors.append("root gates operator owner module")
    if re.search(r"(?m)^\s*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("operator gates whole owner module")
    families = re.compile(
        r"^(?:(?:pub(?:\([^)]*\))?) )?(?:struct|enum) "
        r"((?:VerifyEmbedder|SchemaObject|Dump|TableRowCount|"
        r"OrphanProvenance)\w*)\b",
        re.M,
    )
    for match in families.finditer(root):
        errors.append(f"root still defines operator::{match.group(1)}")
    root_impl = re.compile(
        r"^impl[^\n{]*\b((?:VerifyEmbedder|SchemaObject|Dump|"
        r"TableRowCount|OrphanProvenance)\w*)\b"
        r"(?:\s*<[^>\n]*>)?\s*(?:\{|where\b)",
        re.M,
    )
    for match in root_impl.finditer(root):
        errors.append(f"root still defines operator::{match.group(1)}")
    for name in re.findall(
        r"^    (?:(?:pub(?:\([^)]*\))?) )?fn "
        r"((?:verify_embedder|dump_|orphan_provenance)\w*)\(",
        root,
        re.M,
    ):
        errors.append(f"root still defines Engine::{name}")
    for name in re.findall(r"^(?:(?:pub(?:\([^)]*\))?) )?fn ((?:read_schema_objects|order_canonical_first)\w*)\(", root, re.M):
        errors.append(f"root still defines operator helper {name}")
    for name in REPORT_TYPES:
        kind = "enum" if name == "VerifyEmbedderStatus" else "struct"
        if len(re.findall(r"^pub " + kind + " " + name + r"\b", owner, re.M)) != 1:
            errors.append(f"operator owner lacks one {name}")
    owner_impls = list(re.finditer(r"^impl Engine \{.*?^\}", owner, re.M | re.S))
    for name in METHODS:
        declarations = [match for match in METHOD_DECL.finditer(owner) if match.group(1) == name]
        valid = False
        if len(declarations) == 1:
            declaration = declarations[0]
            enclosing = next(
                (impl for impl in owner_impls if impl.start() < declaration.start() < impl.end()),
                None,
            )
            valid = (
                enclosing is not None
                and brace_depth(owner, enclosing.start()) == 0
                and not outer_attributes(owner, enclosing.start())
                and outer_attributes(owner, declaration.start()) == ['#[cfg(feature="operator")]']
                and declaration.group(0).startswith(f"    pub fn {name}(")
            )
        if not valid:
            errors.append(f"operator owner lacks cfg(operator) Engine::{name}")
    for path, source in others.items():
        for match in METHOD_DECL.finditer(source):
            errors.append(f"{path} still defines Engine::{match.group(1)}")
    for name in HELPERS:
        pattern = r'^#\[cfg\(feature = "operator"\)\]\nfn ' + name + r"\("
        if len(re.findall(pattern, owner, re.M)) != 1:
            errors.append(f"operator owner lacks cfg(operator) helper {name}")
    exports = " ".join(re.findall(r"^pub use operator::\{(.*?)\};", root, re.M | re.S))
    for name in REPORT_TYPES:
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root does not re-export operator::{name}")
    return errors


class OperatorDiagnosticsOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        path = SRC / "operator.rs"
        return (SRC / "lib.rs").read_text(), path.read_text() if path.exists() else ""

    def test_current_source_has_one_operator_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_family_with_complete_owner(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for declaration, expected in (
            ("struct DumpNew;", "root still defines operator::DumpNew"),
            ("enum VerifyEmbedderNew {}", "root still defines operator::VerifyEmbedderNew"),
            ("impl Engine {\n    pub fn dump_new(&self) {}\n}", "root still defines Engine::dump_new"),
            ("impl Engine {\n    pub fn orphan_provenance_new(&self) {}\n}", "root still defines Engine::orphan_provenance_new"),
            ("fn read_schema_objects_new() {}", "root still defines operator helper read_schema_objects_new"),
        ):
            with self.subTest(expected=expected):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner))

    def test_cfg_mutant_and_missing_reexport_fail(self) -> None:
        root, owner = self.sources()
        old = '#[cfg(feature = "operator")]\n    pub fn dump_profile('
        self.assertIn(old, owner)
        self.assertIn(
            "operator owner lacks cfg(operator) Engine::dump_profile",
            owner_errors(root, owner.replace(old, '#[cfg(test)]\n    pub fn dump_profile(', 1)),
        )
        self.assertIn(
            "root does not re-export operator::DumpProfileReport",
            owner_errors(root.replace("DumpProfileReport,", "RemovedDumpProfileReport,", 1), owner),
        )

    def test_root_report_impl_family_is_rejected(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for declaration, name in (
            ("impl DumpProfileReport { fn extra(&self) {} }", "DumpProfileReport"),
            ("impl VerifyEmbedderReport { fn extra(&self) {} }", "VerifyEmbedderReport"),
            ("impl DumpNew { fn extra(&self) {} }", "DumpNew"),
        ):
            with self.subTest(name=name):
                self.assertIn(
                    f"root still defines operator::{name}",
                    owner_errors(root + "\n" + declaration, owner),
                )

    def test_multiple_impls_keep_each_diagnostic_once(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for name in METHODS:
            with self.subTest(name=name):
                duplicate = owner + ('\nimpl Engine {\n    #[cfg(feature = "operator")]\n'
                                     f'    pub fn {name}(&self) {{}}\n}}\n')
                self.assertIn(f"operator owner lacks cfg(operator) Engine::{name}",
                              owner_errors(root, duplicate))
                removed = owner.replace(f"pub fn {name}(", f"pub fn removed_{name}(", 1)
                self.assertIn(f"operator owner lacks cfg(operator) Engine::{name}",
                              owner_errors(root, removed))
                marker = f'    #[cfg(feature = "operator")]\n    pub fn {name}('
                altered = owner.replace(
                    marker,
                    '    #[cfg_attr(\n        all(feature = "default-embedder", test),\n'
                    '        cfg(test)\n    )]\n    /* gate gap */\n' + marker,
                    1,
                )
                self.assertNotEqual(altered, owner)
                self.assertIn(f"operator owner lacks cfg(operator) Engine::{name}",
                              owner_errors(root, altered))
        first = re.search(r"(?ms)^impl Engine \{.*?^}", owner)
        self.assertIsNotNone(first)
        nested = (owner[:first.start()] + '#[cfg(feature = "test-hooks")]\n'
                  'mod gated_diagnostics {\nuse super::*;\n' + first.group() + '\n}\n' +
                  owner[first.end():])
        self.assertIn("operator owner lacks cfg(operator) Engine::verify_embedder",
                      owner_errors(root, nested))
        attr = '#[cfg_attr(\n    all(feature = "default-embedder", test),\n    cfg(test)\n)]'
        raw = 'const RAW_WITNESS: &str = r###"quote " /*"###;\n'
        changed = owner.replace("impl Engine {", raw + attr + "\n/* end */\nimpl Engine {", 1)
        self.assertIn("operator owner lacks cfg(operator) Engine::verify_embedder",
                      owner_errors(root, changed))

    def test_wrong_owner_and_file_gate_mutants(self) -> None:
        root, owner = self.sources()
        other = (SRC / "evidence.rs").read_text()
        for name in ("verify_embedder", "dump_new"):
            self.assertIn(
                f"evidence.rs still defines Engine::{name}",
                owner_errors(root, owner, {"evidence.rs": other +
                                           f"\nimpl Engine {{\n    fn {name}(&self) {{}}\n}}\n"}),
            )
        for attr in ('#![cfg(feature = "test-hooks")]',
                     '#![cfg_attr(feature = "default-embedder", cfg(test))]'):
            self.assertIn("operator gates whole owner module", owner_errors(root, attr + "\n" + owner))

    def test_fast_tier_registration(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice90-operator-diagnostics-owner "
            "python3 scripts/tests/test_slice90_operator_diagnostics_owner.py",
            (ROOT / "scripts/agent-test.sh").read_text(),
        )


if __name__ == "__main__":
    unittest.main()
