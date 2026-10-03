#!/usr/bin/env python3
"""Guard vector storage and equivalence constant ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
STORAGE = {
    "DEFAULT_VECTOR_PROFILE": ': &str = "default";',
    "DEFAULT_VECTOR_PARTITION": ': &str = "vector_default";',
}
EQUIVALENCE = {
    "VECTOR_EQUIVALENCE_PROBE_FIXTURE": ': &str = include_str!("vector_equivalence_probes.txt");',
    "VECTOR_EQUIVALENCE_L2_EPSILON": ": f32 = 1e-5;",
    "VECTOR_EQUIVALENCE_P1_FLIP_FLOOR": ": u64 = 0;",
    "VECTOR_EQUIVALENCE_VERDICT_CACHE_KEY": ': &str = "vector_equivalence_verified_fingerprint";',
    "VECTOR_EQUIVALENCE_FINGERPRINT_RECIPE": ': &str = "fathomdb-veq-verdict-v1";',
}
ROOT_FAMILY = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?const\s+((?:DEFAULT_VECTOR_|VECTOR_EQUIVALENCE_)\w*)\b",
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


def owner_errors(root: str, storage: str, equivalence: str, open_source: str, registry: str) -> list[str]:
    errors = [f"root still defines vector constant {name}" for name in ROOT_FAMILY.findall(root)]
    for module_name in ("vector_storage", "vector_equivalence"):
        module = re.search(r"^mod " + module_name + r";", root, re.M)
        if not module or attrs(root, module.start()):
            errors.append(f"root gates {module_name} module")
    for owner_name, source, constants, visibility in (
        ("vector_storage", storage, STORAGE, "pub(crate) const"),
        ("vector_equivalence", equivalence, EQUIVALENCE, "const"),
    ):
        if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", source):
            errors.append(f"{owner_name} gates whole owner module")
        for name, suffix in constants.items():
            found = list(re.finditer(r"^" + re.escape(visibility) + r" " + name + re.escape(suffix) + r"$", source, re.M))
            if len(found) != 1:
                errors.append(f"{owner_name} lacks exact {name}")
            elif attrs(source, found[0].start()):
                errors.append(f"{owner_name} gates {name}")
    for label, source, marker in (
        ("root", root, "use vector_storage::DEFAULT_VECTOR_PROFILE;"),
        ("open", open_source, "use crate::vector_storage::DEFAULT_VECTOR_PROFILE;"),
        ("registry", registry, "use crate::vector_storage::{DEFAULT_VECTOR_PARTITION, DEFAULT_VECTOR_PROFILE};"),
    ):
        if marker not in source:
            errors.append(f"{label} lacks vector storage owner path")
    return errors


class VectorConstantsOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "vector_storage.rs", "vector_equivalence.rs", "open.rs", "projection_registry.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_unlisted_root_constants(self) -> None:
        root, storage, equivalence, open_source, registry = self.sources()
        for name, suffix in STORAGE.items():
            root = re.sub(r"(?m)^const " + name + r".*\n", "", root, count=1)
            if f"pub(crate) const {name}{suffix}" not in storage:
                storage += f"\npub(crate) const {name}{suffix}"
        for name, suffix in EQUIVALENCE.items():
            root = re.sub(r"(?m)^const " + name + r".*\n", "", root, count=1)
            if f"const {name}{suffix}" not in equivalence:
                equivalence += f"\nconst {name}{suffix}"
        for source_name, marker in (
            ("root", "use vector_storage::DEFAULT_VECTOR_PROFILE;"),
            ("open", "use crate::vector_storage::DEFAULT_VECTOR_PROFILE;"),
            ("registry", "use crate::vector_storage::{DEFAULT_VECTOR_PARTITION, DEFAULT_VECTOR_PROFILE};"),
        ):
            if source_name == "root" and marker not in root:
                root += "\n" + marker
            elif source_name == "open" and marker not in open_source:
                open_source += "\n" + marker
            elif source_name == "registry" and marker not in registry:
                registry += "\n" + marker
        self.assertEqual(owner_errors(root, storage, equivalence, open_source, registry), [])
        for declaration, name in (
            ('const DEFAULT_VECTOR_PROFILE_NEW: &str = "x";', "DEFAULT_VECTOR_PROFILE_NEW"),
            ('pub(crate) const DEFAULT_VECTOR_PARTITION_NEW: &str = "x";', "DEFAULT_VECTOR_PARTITION_NEW"),
            ('pub const VECTOR_EQUIVALENCE_NEW: u64 = 1;', "VECTOR_EQUIVALENCE_NEW"),
        ):
            self.assertIn(
                f"root still defines vector constant {name}",
                owner_errors(root + "\n" + declaration, storage, equivalence, open_source, registry),
            )

    def test_cfg_and_exact_value_mutants(self) -> None:
        root, storage, equivalence, open_source, registry = self.sources()
        self.assertEqual(owner_errors(root, storage, equivalence, open_source, registry), [])
        for owner_name, source, constants, visibility in (
            ("vector_storage", storage, STORAGE, "pub(crate) const"),
            ("vector_equivalence", equivalence, EQUIVALENCE, "const"),
        ):
            for name, suffix in constants.items():
                declaration = f"{visibility} {name}{suffix}"
                for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                    altered = source.replace(declaration, attr + "\n" + declaration, 1)
                    self.assertNotEqual(altered, source)
                    sources = (root, altered, equivalence, open_source, registry) if owner_name == "vector_storage" else (root, storage, altered, open_source, registry)
                    self.assertIn(f"{owner_name} gates {name}", owner_errors(*sources))
                altered = source.replace(declaration, declaration.replace(suffix, ': u8 = 7;'), 1)
                sources = (root, altered, equivalence, open_source, registry) if owner_name == "vector_storage" else (root, storage, altered, open_source, registry)
                self.assertIn(f"{owner_name} lacks exact {name}", owner_errors(*sources))
            for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = attr + "\n" + source
                sources = (root, altered, equivalence, open_source, registry) if owner_name == "vector_storage" else (root, storage, altered, open_source, registry)
                self.assertIn(f"{owner_name} gates whole owner module", owner_errors(*sources))
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = root.replace(f"mod {owner_name};", attr + f"\nmod {owner_name};", 1)
                self.assertIn(f"root gates {owner_name} module", owner_errors(altered, storage, equivalence, open_source, registry))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-vector-constants-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
