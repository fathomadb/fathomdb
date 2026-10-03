#!/usr/bin/env python3
"""Guard feature-gated Engine data-plane integrity ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
METHODS = (
    ("check_data_plane_integrity", '#[cfg(feature = "operator")]'),
    ("data_plane_integrity_query_plans_for_test", '#[cfg(all(feature = "operator", feature = "test-hooks"))]'),
    ("data_plane_integrity_candidate_queries_for_test", '#[cfg(all(feature = "operator", feature = "test-hooks"))]'),
)


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    for name in re.findall(
        r"^    (?:pub(?:\([^)]*\))? )?fn ((?:check_data_plane|data_plane_integrity_)\w*)\(",
        root,
        re.M,
    ):
        errors.append(f"root still defines Engine::{name}")
    for name, cfg in METHODS:
        matches = list(re.finditer(r"^    pub fn " + name + r"\(", owner, re.M))
        if len(matches) != 1:
            errors.append(f"data-plane owner lacks one Engine::{name}")
            continue
        preceding = []
        for line in reversed(owner[: matches[0].start()].splitlines()):
            stripped = line.strip()
            if stripped.startswith(("///", "#[")) or not stripped:
                preceding.append(stripped)
            else:
                break
        if (
            [line for line in preceding if re.match(r"#\[\s*cfg\s*\(", line)] != [cfg]
            or any(re.match(r"#\[\s*cfg_attr\b", line) for line in preceding)
        ):
            errors.append(f"data-plane owner has wrong cfg for Engine::{name}")
    return errors


class DataPlaneIntegrityOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        return (SRC / "lib.rs").read_text(), (SRC / "data_plane_integrity.rs").read_text()

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for declaration, expected in (
            ("impl Engine {\n    fn check_data_plane_integrity_new(&self) {}\n}", "root still defines Engine::check_data_plane_integrity_new"),
            ("impl Engine {\n    fn check_data_plane_new(&self) {}\n}", "root still defines Engine::check_data_plane_new"),
            ("impl Engine {\n    pub(crate) fn data_plane_integrity_new(&self) {}\n}", "root still defines Engine::data_plane_integrity_new"),
            ("impl Engine {\n    pub fn data_plane_integrity_newer(&self) {}\n}", "root still defines Engine::data_plane_integrity_newer"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner))

    def test_each_cfg_arm_mutant_fails(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for name, cfg in METHODS:
            marker = f"    {cfg}\n"
            method = f"    pub fn {name}("
            prefix, suffix = owner.split(method, 1)
            self.assertIn(marker, prefix)
            head, tail = prefix.rsplit(marker, 1)
            altered = head + tail + method + suffix
            with self.subTest(name=name):
                self.assertIn(f"data-plane owner has wrong cfg for Engine::{name}", owner_errors(root, altered))
                wrong = '    #[cfg(feature = "test-hooks")]\n' if name == "check_data_plane_integrity" else '    #[cfg(feature = "operator")]\n'
                flipped = head + wrong + tail + method + suffix
                self.assertIn(f"data-plane owner has wrong cfg for Engine::{name}", owner_errors(root, flipped))
                conditional = (
                    head
                    + marker
                    + '    #[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]\n'
                    + tail
                    + method
                    + suffix
                )
                self.assertIn(f"data-plane owner has wrong cfg for Engine::{name}", owner_errors(root, conditional))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-data-plane-integrity-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
