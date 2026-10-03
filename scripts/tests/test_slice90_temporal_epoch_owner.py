#!/usr/bin/env python3
"""Guard temporal migration epoch ownership across all engine modules."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
NAME = "EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION"
DECLARATION = f"pub(crate) const {NAME}: u32 = 23;"
FAMILY = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?const[ \t]+(EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION\w*)\b",
    re.M,
)


def attrs(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "#[", "//")):
            break
    return found


def source_inventory() -> tuple[str, str, dict[str, str]]:
    root = (SRC / "lib.rs").read_text()
    owner = (SRC / "temporal.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "temporal.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in FAMILY.findall(root)]
    for path, source in modules.items():
        errors.extend(f"{path.removesuffix('.rs')} wrongly defines {name}" for name in FAMILY.findall(source))
    module = re.search(r"^mod temporal;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates temporal module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("temporal gates whole owner module")
    errors.extend(f"temporal unexpectedly defines {name}" for name in FAMILY.findall(owner) if name != NAME)
    constant = list(re.finditer(r"^" + re.escape(DECLARATION) + r"$", owner, re.M))
    if len(constant) != 1:
        errors.append("temporal lacks exact edge epoch")
    elif attrs(owner, constant[0].start()):
        errors.append("temporal gates edge epoch")
    if "use crate::temporal::EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION;" not in modules.get("open.rs", ""):
        errors.append("open lacks temporal epoch owner path")
    return errors


class TemporalEpochOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_wrong_module_exact_and_unlisted(self) -> None:
        root, owner, modules = source_inventory()
        root = re.sub(r"(?m)^const EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION.*\n", "", root, count=1)
        if DECLARATION not in owner:
            owner += "\n" + DECLARATION
        marker = "use crate::temporal::EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION;"
        if marker not in modules["open.rs"]:
            modules["open.rs"] += "\n" + marker
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("const EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW: u32 = 1;", "EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW"),
            ("pub(crate) const EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION: u32 = 1;", NAME),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("open.rs", "const EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION: u32 = 1;", NAME),
            ("open.rs", "pub(crate) const EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW: u32 = 1;", "EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW"),
            ("index_projector.rs", "pub const EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW: u32 = 1;", "EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW"),
        ):
            self.assertIn(path, modules)
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path.removesuffix('.rs')} wrongly defines {name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "const EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW: u32 = 1;"}
        self.assertIn("future_owner wrongly defines EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW", owner_errors(root, owner, arbitrary))

    def test_cfg_value_and_owner_family_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        altered = owner + "\nconst EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW: u32 = 1;"
        self.assertIn("temporal unexpectedly defines EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION_NEW", owner_errors(root, altered, modules))
        altered = owner.replace(DECLARATION, DECLARATION.replace("23", "22"), 1)
        self.assertIn("temporal lacks exact edge epoch", owner_errors(root, altered, modules))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = owner.replace(DECLARATION, attr + "\n" + DECLARATION, 1)
            self.assertIn("temporal gates edge epoch", owner_errors(root, altered, modules))
            altered = root.replace("mod temporal;", attr + "\nmod temporal;", 1)
            self.assertIn("root gates temporal module", owner_errors(altered, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("temporal gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-temporal-epoch-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
