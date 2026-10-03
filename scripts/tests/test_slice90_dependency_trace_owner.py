#!/usr/bin/env python3
"""Guard dependency trace runtime, measurement, and constant ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
CONSTANTS = (
    "DEPENDENCY_GENERATION_KEY",
    "SOURCE_DEPENDENCY_SCHEMA_VERSION",
    "DEPENDENCY_LOOKUP_LIMIT",
)


def owner_errors(root: str, owner: str, dependency: str, opened: str) -> list[str]:
    errors = []
    for name in re.findall(
        r"^(?:pub(?:\([^)]*\))? )?(?:struct|enum|type|trait) (DependencyTrace\w*)\b",
        root,
        re.M,
    ):
        errors.append(f"root still defines dependency_trace::{name}")
    for name in re.findall(r"^impl[^\n{]*\b(DependencyTrace\w*)\b[^\n{]*\{", root, re.M):
        errors.append(f"root still implements dependency_trace::{name}")
    for name in re.findall(
        r"^    (?:pub(?:\([^)]*\))? )?fn (trace_dependency\w*)\(", root, re.M
    ):
        errors.append(f"root still defines Engine::{name}")
    for name in re.findall(
        r"^(?:pub(?:\([^)]*\))? )?const ((?:DEPENDENCY_GENERATION|SOURCE_DEPENDENCY_SCHEMA|DEPENDENCY_LOOKUP)\w*)\b",
        root,
        re.M,
    ):
        errors.append(f"root still defines dependency_trace::{name}")
    for name in CONSTANTS:
        if len(re.findall(r"^pub\(crate\) const " + name + r"\b", owner, re.M)) != 1:
            errors.append(f"dependency trace lacks one {name}")
    if len(re.findall(r'^#\[cfg\(feature = "test-hooks"\)\]\n#\[derive\(Clone, Debug\)\]\n#\[doc\(hidden\)\]\npub struct DependencyTraceMeasurement\b', owner, re.M)) != 1:
        errors.append("dependency trace lacks gated measurement carrier")
    if len(re.findall(r"^    pub fn trace_dependency\(", owner, re.M)) != 1:
        errors.append("dependency trace lacks Engine::trace_dependency")
    if not re.search(r'^#\[cfg\(feature = "test-hooks"\)\]\n#\[doc\(hidden\)\]\npub use dependency_trace::DependencyTraceMeasurement;', root, re.M):
        errors.append("root lacks gated measurement reexport")
    if "use crate::dependency_trace::{DEPENDENCY_GENERATION_KEY, DEPENDENCY_LOOKUP_LIMIT};" not in dependency:
        errors.append("dependency consumer does not import owner constants")
    if "use crate::dependency_trace::{DEPENDENCY_GENERATION_KEY, SOURCE_DEPENDENCY_SCHEMA_VERSION};" not in opened:
        errors.append("open consumer does not import owner constants")
    return errors


class DependencyTraceOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return tuple(
            (SRC / name).read_text()
            for name in ("lib.rs", "dependency_trace.rs", "dependency.rs", "open.rs")
        )

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner, dependency, opened = self.sources()
        self.assertEqual(owner_errors(root, owner, dependency, opened), [])
        for declaration, expected in (
            ("struct DependencyTraceNew;", "root still defines dependency_trace::DependencyTraceNew"),
            ("pub(crate) enum DependencyTraceCrate { A }", "root still defines dependency_trace::DependencyTraceCrate"),
            ("impl DependencyTraceMeasurement { fn extra(&self) {} }", "root still implements dependency_trace::DependencyTraceMeasurement"),
            ("impl SomeTrait for DependencyTraceMeasurement {}", "root still implements dependency_trace::DependencyTraceMeasurement"),
            ("impl Engine {\n    fn trace_dependency_new(&self) {}\n}", "root still defines Engine::trace_dependency_new"),
            ("impl Engine {\n    pub(crate) fn trace_dependency_crate(&self) {}\n}", "root still defines Engine::trace_dependency_crate"),
            ("const DEPENDENCY_GENERATION_NEW: &str = \"x\";", "root still defines dependency_trace::DEPENDENCY_GENERATION_NEW"),
            ("pub(crate) const SOURCE_DEPENDENCY_SCHEMA_NEW: u32 = 1;", "root still defines dependency_trace::SOURCE_DEPENDENCY_SCHEMA_NEW"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner, dependency, opened))

    def test_cfg_reexport_and_consumer_mutants(self) -> None:
        root, owner, dependency, opened = self.sources()
        self.assertEqual(owner_errors(root, owner, dependency, opened), [])
        altered = owner.replace('#[cfg(feature = "test-hooks")]\n#[derive(Clone, Debug)]\n#[doc(hidden)]\npub struct DependencyTraceMeasurement', '#[derive(Clone, Debug)]\n#[doc(hidden)]\npub struct DependencyTraceMeasurement', 1)
        self.assertNotEqual(altered, owner)
        self.assertIn("dependency trace lacks gated measurement carrier", owner_errors(root, altered, dependency, opened))
        altered = root.replace("pub use dependency_trace::DependencyTraceMeasurement;", "", 1)
        self.assertNotEqual(altered, root)
        self.assertIn("root lacks gated measurement reexport", owner_errors(altered, owner, dependency, opened))
        self.assertIn("dependency consumer does not import owner constants", owner_errors(root, owner, dependency.replace("use crate::dependency_trace::{DEPENDENCY_GENERATION_KEY, DEPENDENCY_LOOKUP_LIMIT};", ""), opened))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-dependency-trace-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
