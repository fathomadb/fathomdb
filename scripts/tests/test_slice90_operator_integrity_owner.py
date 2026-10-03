#!/usr/bin/env python3
"""Guard operator integrity and safe-export ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = ("CheckIntegrityOpts", "Section", "Finding", "IntegrityReport", "SafeExportArtifact")
METHODS = ("check_integrity", "safe_export")
HELPERS = (
    "physical_section",
    "logical_section",
    "semantic_section",
    "collect_integrity_check_findings",
    "locator_from_rusqlite_error",
)


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    root_types = re.compile(
        r"^(?:(?:pub(?:\([^)]*\))?) )?(?:struct|enum) "
        r"((?:CheckIntegrity|Section|Finding|IntegrityReport|SafeExport)\w*)\b",
        re.M,
    )
    for match in root_types.finditer(root):
        errors.append(f"root still defines operator::{match.group(1)}")
    root_impls = re.compile(
        r"^impl[^\n{]*\b((?:CheckIntegrity|Section|Finding|IntegrityReport|SafeExport)\w*)\b"
        r"(?:\s*<[^>\n]*>)?\s*(?:\{|where\b)",
        re.M,
    )
    for match in root_impls.finditer(root):
        errors.append(f"root still defines operator::{match.group(1)}")
    for name in re.findall(
        r"^    (?:(?:pub(?:\([^)]*\))?) )?fn ((?:check_integrity|safe_export)\w*)\(",
        root,
        re.M,
    ):
        errors.append(f"root still defines Engine::{name}")
    for name in re.findall(
        r"^(?:(?:pub(?:\([^)]*\))?) )?fn "
        r"((?:physical_section|logical_section|semantic_section|"
        r"collect_integrity_check_findings|locator_from_rusqlite_error)\w*)\(",
        root,
        re.M,
    ):
        errors.append(f"root still defines operator helper {name}")
    for name in TYPES:
        kind = "enum" if name == "Section" else "struct"
        if len(re.findall(r"^pub " + kind + " " + name + r"\b", owner, re.M)) != 1:
            errors.append(f"operator owner lacks one {name}")
    owner_impls = re.findall(r"^impl Engine \{.*?^\}", owner, re.M | re.S)
    if len(owner_impls) != 1:
        errors.append("operator owner lacks one Engine impl")
    owner_engine = owner_impls[0] if len(owner_impls) == 1 else ""
    for name in METHODS:
        pattern = r'^    #\[cfg\(feature = "operator"\)\]\n    pub fn ' + name + r"\("
        if len(re.findall(pattern, owner_engine, re.M)) != 1:
            errors.append(f"operator owner lacks cfg(operator) Engine::{name}")
    for name in HELPERS:
        pattern = r'^#\[cfg\(feature = "operator"\)\]\nfn ' + name + r"\("
        if len(re.findall(pattern, owner, re.M)) != 1:
            errors.append(f"operator owner lacks cfg(operator) helper {name}")
    exports = " ".join(re.findall(r"^pub use operator::\{(.*?)\};", root, re.M | re.S))
    for name in TYPES:
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root does not re-export operator::{name}")
    return errors


class OperatorIntegrityOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        return (SRC / "lib.rs").read_text(), (SRC / "operator.rs").read_text()

    def test_current_source_has_one_operator_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_family_with_complete_owner(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for declaration, expected in (
            ("struct SafeExportNew;", "root still defines operator::SafeExportNew"),
            ("impl IntegrityReport { fn extra(&self) {} }", "root still defines operator::IntegrityReport"),
            ("impl Engine {\n    pub fn check_integrity_new(&self) {}\n}", "root still defines Engine::check_integrity_new"),
            ("fn physical_section_new() {}", "root still defines operator helper physical_section_new"),
        ):
            with self.subTest(expected=expected):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner))

    def test_cfg_and_reexport_mutants_fail(self) -> None:
        root, owner = self.sources()
        old = '#[cfg(feature = "operator")]\n    pub fn safe_export('
        self.assertIn(old, owner)
        self.assertIn(
            "operator owner lacks cfg(operator) Engine::safe_export",
            owner_errors(root, owner.replace(old, '#[cfg(test)]\n    pub fn safe_export(', 1)),
        )
        self.assertIn(
            "root does not re-export operator::SafeExportArtifact",
            owner_errors(root.replace("SafeExportArtifact,", "RemovedSafeExportArtifact,", 1), owner),
        )

    def test_fast_tier_registration(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice90-operator-integrity-owner "
            "python3 scripts/tests/test_slice90_operator_integrity_owner.py",
            (ROOT / "scripts/agent-test.sh").read_text(),
        )


if __name__ == "__main__":
    unittest.main()
