#!/usr/bin/env python3
"""Guard registry contract carriers and their stable public paths."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = {
    "ProjectionRole": "enum",
    "ProjectionFts": "struct",
    "DenseReadiness": "enum",
    "ProjectionVector": "struct",
    "ProjectionSpec": "struct",
    "ProjectionDelta": "struct",
}
FAMILY = r"(?:Projection(?:Role|Fts|Vector|Spec|Delta)|DenseReadiness)\w*"


def gated(source: str, start: int) -> bool:
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            return True
        if stripped and not stripped.startswith(("///", "#[")):
            break
    return False


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    for name in re.findall(r"^(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|type|trait|union)\s+(" + FAMILY + r")\b", root, re.M):
        errors.append(f"root still defines registry::{name}")
    for name in re.findall(r"^impl\b[^\n{]*\b(" + FAMILY + r")\b[^\n{]*\{", root, re.M):
        errors.append(f"root still implements registry::{name}")
    for name, kind in TYPES.items():
        found = list(re.finditer(r"^pub " + kind + " " + name + r"\b", owner, re.M))
        if len(found) != 1:
            errors.append(f"registry lacks one {name}")
        elif gated(owner, found[0].start()):
            errors.append(f"registry gates always-on {name}")
    for name in ("ProjectionRole", "DenseReadiness"):
        found = list(re.finditer(r"^impl " + name + r" \{", owner, re.M))
        if len(found) != 1:
            errors.append(f"registry lacks one impl {name}")
        elif gated(owner, found[0].start()):
            errors.append(f"registry gates always-on impl {name}")
    exports = " ".join(re.findall(r"^pub use projection_registry::\{(.*?)\};", root, re.M | re.S))
    for name in TYPES:
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root lacks registry::{name} reexport")
    return errors


class ProjectionRegistryCarriersOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        return (SRC / "lib.rs").read_text(), (SRC / "projection_registry.rs").read_text()

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_family(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for declaration, expected in (
            ("struct ProjectionRoleNew;", "root still defines registry::ProjectionRoleNew"),
            ("pub(crate) enum DenseReadinessNew { A }", "root still defines registry::DenseReadinessNew"),
            ("pub struct ProjectionDeltaNew;", "root still defines registry::ProjectionDeltaNew"),
            ("impl ProjectionSpec { fn extra(&self) {} }", "root still implements registry::ProjectionSpec"),
            ("impl SomeTrait for ProjectionVector {}", "root still implements registry::ProjectionVector"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, owner))

    def test_cfg_and_reexport_mutants(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        attrs = ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "operator"))]')
        for name, kind in TYPES.items():
            marker = f"pub {kind} {name}"
            for attr in attrs:
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                with self.subTest(name=name, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(f"registry gates always-on {name}", owner_errors(root, altered))
        for name in ("ProjectionRole", "DenseReadiness"):
            marker = f"impl {name} {{"
            for attr in attrs:
                altered = owner.replace(marker, attr + "\n" + marker, 1)
                with self.subTest(implementation=name, attr=attr):
                    self.assertNotEqual(altered, owner)
                    self.assertIn(f"registry gates always-on impl {name}", owner_errors(root, altered))
        altered = root.replace("ProjectionRole,", "", 1)
        self.assertNotEqual(altered, root)
        self.assertIn("root lacks registry::ProjectionRole reexport", owner_errors(altered, owner))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-projection-registry-carriers-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
