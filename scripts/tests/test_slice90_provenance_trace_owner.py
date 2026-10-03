#!/usr/bin/env python3
"""Guard provenance trace carriers, method, and default cap ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"


def owner_errors(root: str, owner: str, config: str) -> list[str]:
    errors = []
    for name in re.findall(r"^pub (?:struct|enum) (Trace\w*)\b", root, re.M):
        errors.append(f"root still defines provenance::{name}")
    for name in re.findall(r"^impl[^\n{]*\b(Trace\w*)\b[^\n{]*\{", root, re.M):
        errors.append(f"root still implements provenance::{name}")
    for name in re.findall(r"^    pub fn (trace_source_ref\w*)\(", root, re.M):
        errors.append(f"root still defines Engine::{name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))? )?const (DEFAULT_PROVENANCE\w*)\b", root, re.M):
        errors.append(f"root still defines provenance::{name}")
    for name in ("TraceReport", "TraceEvent"):
        if len(re.findall(r"^pub struct " + name + r"\b", owner, re.M)) != 1:
            errors.append(f"provenance lacks one {name}")
    if len(re.findall(r"^pub\(crate\) const DEFAULT_PROVENANCE_ROW_CAP\b", owner, re.M)) != 1:
        errors.append("provenance lacks one default row cap")
    if len(re.findall(r'^    #\[cfg\(feature = "operator"\)\]\n    pub fn trace_source_ref\(', owner, re.M)) != 1:
        errors.append("provenance lacks cfg(operator) Engine::trace_source_ref")
    exports = " ".join(re.findall(r"^pub use provenance::\{(.*?)\};", root, re.M | re.S))
    for name in ("TraceReport", "TraceEvent"):
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root lacks provenance::{name} reexport")
    if "use super::provenance::DEFAULT_PROVENANCE_ROW_CAP;" not in config:
        errors.append("runtime configuration does not import provenance cap")
    return errors


class ProvenanceTraceOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "provenance.rs", "runtime_configuration.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner, config = self.sources()
        self.assertEqual(owner_errors(root, owner, config), [])
        for declaration, expected in (
            ("pub struct TraceNew;", "root still defines provenance::TraceNew"),
            ("impl TraceReport { fn extra(&self) {} }", "root still implements provenance::TraceReport"),
            ("impl Engine {\n    pub fn trace_source_ref_new(&self) {}\n}", "root still defines Engine::trace_source_ref_new"),
            ("const DEFAULT_PROVENANCE_NEW: u64 = 1;", "root still defines provenance::DEFAULT_PROVENANCE_NEW"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner, config))

    def test_cfg_reexport_and_consumer_mutants(self) -> None:
        root, owner, config = self.sources()
        self.assertEqual(owner_errors(root, owner, config), [])
        altered = owner.replace('    #[cfg(feature = "operator")]\n    pub fn trace_source_ref(', '    pub fn trace_source_ref(', 1)
        self.assertNotEqual(altered, owner)
        self.assertIn("provenance lacks cfg(operator) Engine::trace_source_ref", owner_errors(root, altered, config))
        altered = root.replace("TraceEvent, TraceReport,", "", 1)
        self.assertNotEqual(altered, root)
        self.assertIn("root lacks provenance::TraceEvent reexport", owner_errors(altered, owner, config))
        self.assertIn("runtime configuration does not import provenance cap", owner_errors(root, owner, config.replace("use super::provenance::DEFAULT_PROVENANCE_ROW_CAP;", "")))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-provenance-trace-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
