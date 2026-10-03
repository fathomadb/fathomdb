#!/usr/bin/env python3
"""Guard root identity helpers and the separate frozen-token codec."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
NAMES = ("hex_encode", "hex_nibble", "digest_record_identity", "legacy_revision_id")
FAMILY = re.compile(
    r"^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?fn[ \t]+"
    r"((?:hex_encode|hex_nibble|digest_record_identity|legacy_revision_id)\w*)[ \t]*\(",
    re.M,
)
CALLERS = (
    ("actuation.rs", "use crate::identity::hex_encode;"),
    ("operator.rs", "use crate::identity::hex_encode;"),
    ("write_commit.rs", "use crate::identity::hex_encode;"),
    ("erasure.rs", "use crate::identity::digest_record_identity;"),
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
    owner = (SRC / "identity.rs").read_text()
    modules = {
        path.relative_to(SRC).as_posix(): path.read_text()
        for path in sorted(SRC.rglob("*.rs"))
        if path.relative_to(SRC).as_posix() not in ("lib.rs", "identity.rs")
    }
    return root, owner, modules


def owner_errors(root: str, owner: str, modules: dict[str, str]) -> list[str]:
    errors = [f"root wrongly defines {name}" for name in FAMILY.findall(root)]
    for path, source in modules.items():
        names = FAMILY.findall(source)
        for name in names:
            if path == "frozen_read.rs" and name == "hex_encode" and names.count(name) == 1:
                continue
            errors.append(f"{path} wrongly defines {name}")
    module = re.search(r"^mod identity;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates identity module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("identity gates whole owner module")
    names = FAMILY.findall(owner)
    errors.extend(f"identity unexpectedly defines {name}" for name in names if name not in NAMES)
    for name in NAMES:
        visibility = "fn" if name == "hex_nibble" else "pub(crate) fn"
        found = list(re.finditer(r"^" + re.escape(visibility) + r" " + name + r"\(", owner, re.M))
        if len(found) != 1 or names.count(name) != 1:
            errors.append(f"identity lacks one {name}")
            continue
        actual_attrs = attrs(owner, found[0].start())
        expected_attrs = ['#[cfg(feature = "operator")]'] if name == "digest_record_identity" else []
        if actual_attrs != expected_attrs:
            errors.append(f"identity gates {name} incorrectly")
    for path, marker in CALLERS:
        if marker not in modules.get(path, ""):
            errors.append(f"{path} lacks identity owner path")
    for path in ("projection_generation.rs", "dependency_closure.rs"):
        if "crate::identity::hex_encode(" not in modules.get(path, ""):
            errors.append(f"{path} lacks identity owner call")
        if "super::hex_encode(" in modules.get(path, ""):
            errors.append(f"{path} still calls root hex")
    if "use super::identity::legacy_revision_id;" not in root:
        errors.append("root test module lacks identity owner path")
    return errors


class IdentityHelpersOwnerTest(unittest.TestCase):
    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*source_inventory()), [])

    def test_complete_owner_rejects_wrong_owner_declarations(self) -> None:
        root, owner, modules = source_inventory()
        for name in NAMES:
            pattern = re.compile(r"(?ms)^fn " + name + r"\(.*?^}\n")
            match = pattern.search(root)
            if match:
                visibility = "fn" if name == "hex_nibble" else "pub(crate) fn"
                prefix = '#[cfg(feature = "operator")]\n' if name == "digest_record_identity" else ""
                owner += "\n" + prefix + match.group().replace(f"fn {name}(", f"{visibility} {name}(", 1)
                root = root[: match.start()] + root[match.end() :]
        if "use super::identity::legacy_revision_id;" not in root:
            root += "\nuse super::identity::legacy_revision_id;"
        for path, marker in CALLERS:
            if marker not in modules[path]:
                modules[path] += "\n" + marker
        for path in ("projection_generation.rs", "dependency_closure.rs"):
            modules[path] = modules[path].replace("super::hex_encode(", "crate::identity::hex_encode(")
        self.assertEqual(owner_errors(root, owner, modules), [])
        for declaration, name in (
            ("fn hex_encode_new() {}", "hex_encode_new"),
            ("pub(crate) fn digest_record_identity() {}", "digest_record_identity"),
        ):
            self.assertIn(f"root wrongly defines {name}", owner_errors(root + "\n" + declaration, owner, modules))
        for path, declaration, name in (
            ("erasure.rs", "fn digest_record_identity_new() {}", "digest_record_identity_new"),
            ("write_commit.rs", "pub(crate) fn hex_encode_new() {}", "hex_encode_new"),
            ("operator.rs", "pub fn legacy_revision_id() {}", "legacy_revision_id"),
            ("frozen_read.rs", "fn hex_nibble_new() {}", "hex_nibble_new"),
        ):
            altered = modules | {path: modules[path] + "\n" + declaration}
            self.assertIn(f"{path} wrongly defines {name}", owner_errors(root, owner, altered))
        arbitrary = modules | {"future_owner.rs": "fn hex_encode_new() {}"}
        self.assertIn("future_owner.rs wrongly defines hex_encode_new", owner_errors(root, owner, arbitrary))

    def test_cfg_and_frozen_codec_mutants(self) -> None:
        root, owner, modules = source_inventory()
        self.assertEqual(owner_errors(root, owner, modules), [])
        self.assertEqual(FAMILY.findall(modules["frozen_read.rs"]).count("hex_encode"), 1)
        changed = modules | {"frozen_read.rs": modules["frozen_read.rs"] + "\nfn hex_encode_new() {}"}
        self.assertIn("frozen_read.rs wrongly defines hex_encode_new", owner_errors(root, owner, changed))
        changed = modules | {"frozen_read.rs": modules["frozen_read.rs"] + "\nfn hex_encode() {}"}
        self.assertIn("frozen_read.rs wrongly defines hex_encode", owner_errors(root, owner, changed))
        for attr in ('#[cfg(feature = "test-hooks")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            changed = owner.replace("pub(crate) fn hex_encode(", attr + "\npub(crate) fn hex_encode(", 1)
            self.assertIn("identity gates hex_encode incorrectly", owner_errors(root, changed, modules))
            changed = owner.replace('pub(crate) fn digest_record_identity(', attr + '\npub(crate) fn digest_record_identity(', 1)
            self.assertIn("identity gates digest_record_identity incorrectly", owner_errors(root, changed, modules))
            changed = root.replace("mod identity;", attr + "\nmod identity;", 1)
            self.assertIn("root gates identity module", owner_errors(changed, owner, modules))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("identity gates whole owner module", owner_errors(root, attr + "\n" + owner, modules))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-identity-helpers-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
