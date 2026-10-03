#!/usr/bin/env python3
"""Guard rebuild contract ownership and its always-on public types."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = ("RebuildKind", "RebuildReport")
METHODS = ("rebuild_projections", "rebuild_vec0", "run_rebuild", "rebuild_shadow_state")


def prelude(source: str, start: int) -> list[str]:
    lines = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if stripped.startswith(("///", "#[")) or not stripped:
            lines.append(stripped)
        else:
            break
    return lines


def gated(source: str, start: int) -> bool:
    return any(re.match(r"#\[\s*cfg(?:_attr)?\b", line) for line in prelude(source, start))


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|type|trait|union)\s+(Rebuild(?:Kind|Report)\w*)\b", root, re.M):
        errors.append(f"root still defines rebuild::{name}")
    for name in re.findall(r"^impl\b[^\n{]*\b(Rebuild(?:Kind|Report)\w*)\b[^\n{]*\{", root, re.M):
        errors.append(f"root still implements rebuild::{name}")
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?const\s+(REBUILD_DRAIN_TIMEOUT_MS\w*)\b", root, re.M):
        errors.append(f"root still defines rebuild::{name}")
    for name in re.findall(r"^    (?:pub(?:\([^)]*\))?\s+)?fn\s+((?:rebuild_projections|rebuild_vec0|run_rebuild|rebuild_shadow_state)\w*)\s*\(", root, re.M):
        errors.append(f"root still defines Engine::{name}")
    module = re.search(r"^mod projection_rebuild;", root, re.M)
    if not module:
        errors.append("root lacks ungated rebuild module")
    elif gated(root, module.start()):
        errors.append("root gates rebuild module")
    for name, kind in (("RebuildKind", "enum"), ("RebuildReport", "struct")):
        found = list(re.finditer(r"^pub " + kind + " " + name + r"\b", owner, re.M))
        if len(found) != 1:
            errors.append(f"rebuild lacks one {name}")
        elif gated(owner, found[0].start()):
            errors.append(f"rebuild gates always-on {name}")
    timeout = list(re.finditer(r"^const REBUILD_DRAIN_TIMEOUT_MS\b", owner, re.M))
    if len(timeout) != 1:
        errors.append("rebuild lacks one drain timeout")
    elif [line for line in prelude(owner, timeout[0].start()) if re.match(r"#\[\s*cfg(?:_attr)?\b", line)] != ['#[cfg(feature = "operator")]']:
        errors.append("rebuild drain timeout lacks cfg(operator)")
    for name in METHODS:
        visibility = "pub " if name in ("rebuild_projections", "rebuild_vec0") else ""
        found = list(re.finditer(r"^    " + visibility + "fn " + name + r"\(", owner, re.M))
        if len(found) != 1:
            errors.append(f"rebuild lacks one Engine::{name}")
        elif [line for line in prelude(owner, found[0].start()) if re.match(r"#\[\s*cfg(?:_attr)?\b", line)] != ['#[cfg(feature = "operator")]']:
            errors.append(f"rebuild Engine::{name} lacks exact cfg(operator)")
    export_matches = list(re.finditer(r"^pub use projection_rebuild::\{(.*?)\};", root, re.M | re.S))
    exports = " ".join(match.group(1) for match in export_matches)
    if len(export_matches) != 1 or gated(root, export_matches[0].start()):
        errors.append("root gates rebuild reexport")
    for name in TYPES:
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root lacks rebuild::{name} reexport")
    return errors


class ProjectionRebuildOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        return (SRC / "lib.rs").read_text(), (SRC / "projection_rebuild.rs").read_text()

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for declaration, expected in (
            ("struct RebuildReportNew;", "root still defines rebuild::RebuildReportNew"),
            ("pub(crate) enum RebuildKindNew { A }", "root still defines rebuild::RebuildKindNew"),
            ("pub struct RebuildReportExtra;", "root still defines rebuild::RebuildReportExtra"),
            ("impl RebuildKind { fn extra(&self) {} }", "root still implements rebuild::RebuildKind"),
            ("impl SomeTrait for RebuildReport {}", "root still implements rebuild::RebuildReport"),
            ("pub(crate) const REBUILD_DRAIN_TIMEOUT_MS_NEW: u64 = 1;", "root still defines rebuild::REBUILD_DRAIN_TIMEOUT_MS_NEW"),
            ("impl Engine {\n    pub fn rebuild_projections_new(&self) {}\n}", "root still defines Engine::rebuild_projections_new"),
            ("impl Engine {\n    fn run_rebuild_new(&self) {}\n}", "root still defines Engine::run_rebuild_new"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner))

    def test_cfg_and_reexport_mutants(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for name, kind in (("RebuildKind", "enum"), ("RebuildReport", "struct")):
            marker = f"pub {kind} {name} {{"
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]'):
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                with self.subTest(name=name, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(f"rebuild gates always-on {name}", owner_errors(root, altered))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]'):
            altered = root.replace("mod projection_rebuild;", attr + '\nmod projection_rebuild;', 1)
            self.assertIn("root gates rebuild module", owner_errors(altered, owner))
        altered = root.replace("RebuildKind,", "", 1)
        self.assertNotEqual(altered, root)
        self.assertIn("root lacks rebuild::RebuildKind reexport", owner_errors(altered, owner))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]'):
            altered = root.replace("pub use projection_rebuild::{", attr + '\npub use projection_rebuild::{', 1)
            self.assertIn("root gates rebuild reexport", owner_errors(altered, owner))
        altered = owner.replace('#[cfg(feature = "operator")]\nconst REBUILD_DRAIN_TIMEOUT_MS', '#[cfg(feature = "test-hooks")]\nconst REBUILD_DRAIN_TIMEOUT_MS', 1)
        self.assertNotEqual(altered, owner)
        self.assertIn("rebuild drain timeout lacks cfg(operator)", owner_errors(root, altered))
        altered = owner.replace('const REBUILD_DRAIN_TIMEOUT_MS', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]\nconst REBUILD_DRAIN_TIMEOUT_MS', 1)
        self.assertIn("rebuild drain timeout lacks cfg(operator)", owner_errors(root, altered))
        for name in METHODS:
            visibility = "pub " if name in ("rebuild_projections", "rebuild_vec0") else ""
            marker = f"    {visibility}fn {name}("
            for attr in ('#[cfg(feature = "test-hooks")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(marker, "    " + attr + "\n" + marker, 1)
                with self.subTest(method=name, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(f"rebuild Engine::{name} lacks exact cfg(operator)", owner_errors(root, altered))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-projection-rebuild-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
