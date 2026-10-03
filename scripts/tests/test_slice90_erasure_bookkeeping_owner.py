#!/usr/bin/env python3
"""Guard erasure bookkeeping constants and refusal helper ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
CONSTANTS = {
    "REDACTED_STABLE_ID": ('const', ': &str = "[erased]";'),
    "ERASURE_AUDIT_COLLECTIONS": ('pub(crate) const', ': &[&str] =\n    &["excise_source_audit", "excise_record_audit"];'),
    "ERASURE_PENDING_REDACTION_COLLECTION": ('pub(crate) const', ': &str = "erasure_pending_redaction";'),
}
CONST_FAMILY = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?const\s+"
    r"((?:REDACTED_STABLE_ID|ERASURE_AUDIT_COLLECTIONS|ERASURE_PENDING_REDACTION_COLLECTION)\w*)\b",
    re.M,
)
HELPER_FAMILY = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+(is_erasure_bookkeeping_collection\w*)\s*\(",
    re.M,
)
OPERATOR_CFG = '#[cfg(feature = "operator")]'


def attrs(source: str, start: int) -> list[str]:
    found = []
    for line in reversed(source[:start].splitlines()):
        stripped = line.strip()
        if re.match(r"#\[\s*cfg(?:_attr)?\b", stripped):
            found.append(stripped)
        elif stripped and not stripped.startswith(("///", "#[", "//")):
            break
    return found


def owner_errors(root: str, owner: str, commit: str, wal: str, telemetry: str) -> list[str]:
    errors = []
    for label, source in (("root", root), ("write_commit", commit), ("wal_runtime", wal), ("telemetry", telemetry)):
        errors.extend(f"{label} wrongly defines {name}" for name in CONST_FAMILY.findall(source))
        errors.extend(f"{label} wrongly defines {name}" for name in HELPER_FAMILY.findall(source))
    module = re.search(r"^mod erasure;", root, re.M)
    if not module or attrs(root, module.start()):
        errors.append("root gates erasure module")
    if re.search(r"(?m)^[ \t]*#!\[\s*cfg(?:_attr)?\b", owner):
        errors.append("erasure gates whole owner module")
    errors.extend(
        f"erasure unexpectedly defines {name}"
        for name in CONST_FAMILY.findall(owner)
        if name not in CONSTANTS
    )
    errors.extend(
        f"erasure unexpectedly defines {name}"
        for name in HELPER_FAMILY.findall(owner)
        if name != "is_erasure_bookkeeping_collection"
    )
    for name, (visibility, suffix) in CONSTANTS.items():
        declaration = f"{visibility} {name}{suffix}"
        found = list(re.finditer(r"^" + re.escape(declaration) + r"$", owner, re.M))
        if len(found) != 1:
            errors.append(f"erasure lacks exact {name}")
        elif attrs(owner, found[0].start()):
            errors.append(f"erasure gates {name}")
    helpers = list(re.finditer(r"^fn is_erasure_bookkeeping_collection\(", owner, re.M))
    if len(helpers) != 1:
        errors.append("erasure lacks one is_erasure_bookkeeping_collection")
    elif attrs(owner, helpers[0].start()) != [OPERATOR_CFG]:
        errors.append("erasure changes is_erasure_bookkeeping_collection cfg")
    if "use crate::erasure::{ERASURE_AUDIT_COLLECTIONS, ERASURE_PENDING_REDACTION_COLLECTION};" not in commit:
        errors.append("write_commit lacks erasure owner paths")
    return errors


class ErasureBookkeepingOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str, str]:
        return tuple((SRC / name).read_text() for name in ("lib.rs", "erasure.rs", "write_commit.rs", "wal_runtime.rs", "telemetry.rs"))

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_complete_owner_rejects_root_and_wrong_owner_families(self) -> None:
        root, owner, commit, wal, telemetry = self.sources()
        for name, (visibility, suffix) in CONSTANTS.items():
            root = re.sub(r"(?m)^const " + name + r".*\n", "", root, count=1)
            declaration = f"{visibility} {name}{suffix}"
            if declaration not in owner:
                owner += "\n" + declaration
        root = re.sub(r"(?ms)^fn is_erasure_bookkeeping_collection\(.*?^}\n", "", root, count=1)
        if not re.search(r"^fn is_erasure_bookkeeping_collection\(", owner, re.M):
            owner += "\n" + OPERATOR_CFG + "\nfn is_erasure_bookkeeping_collection() {}"
        marker = "use crate::erasure::{ERASURE_AUDIT_COLLECTIONS, ERASURE_PENDING_REDACTION_COLLECTION};"
        if marker not in commit:
            commit += "\n" + marker
        self.assertEqual(owner_errors(root, owner, commit, wal, telemetry), [])
        for label, source, declaration, name in (
            ("root", root, 'pub(crate) const REDACTED_STABLE_ID_NEW: &str = "x";', "REDACTED_STABLE_ID_NEW"),
            ("root", root, "fn is_erasure_bookkeeping_collection_new() {}", "is_erasure_bookkeeping_collection_new"),
            ("write_commit", commit, 'const ERASURE_AUDIT_COLLECTIONS: &str = "wrong";', "ERASURE_AUDIT_COLLECTIONS"),
            ("write_commit", commit, 'pub(crate) const ERASURE_PENDING_REDACTION_COLLECTION_NEW: &str = "wrong";', "ERASURE_PENDING_REDACTION_COLLECTION_NEW"),
            ("wal_runtime", wal, "pub fn is_erasure_bookkeeping_collection_new() {}", "is_erasure_bookkeeping_collection_new"),
        ):
            altered = source + "\n" + declaration
            sources = {"root": root, "write_commit": commit, "wal_runtime": wal, "telemetry": telemetry}
            sources[label] = altered
            self.assertIn(
                f"{label} wrongly defines {name}",
                owner_errors(sources["root"], owner, sources["write_commit"], sources["wal_runtime"], sources["telemetry"]),
            )

    def test_telemetry_rejects_redaction_sentinel_duplicates(self) -> None:
        root, owner, commit, wal, telemetry = self.sources()
        self.assertEqual(owner_errors(root, owner, commit, wal, telemetry), [])
        for declaration, name in (
            ('const REDACTED_STABLE_ID: &str = "wrong";', "REDACTED_STABLE_ID"),
            ('pub(crate) const REDACTED_STABLE_ID_NEW: &str = "wrong";', "REDACTED_STABLE_ID_NEW"),
        ):
            self.assertIn(
                f"telemetry wrongly defines {name}",
                owner_errors(root, owner, commit, wal, telemetry + "\n" + declaration),
            )

    def test_owner_rejects_unlisted_same_family_declarations(self) -> None:
        root, owner, commit, wal, telemetry = self.sources()
        self.assertEqual(owner_errors(root, owner, commit, wal, telemetry), [])
        for declaration, name in (
            ('const REDACTED_STABLE_ID_NEW: &str = "x";', "REDACTED_STABLE_ID_NEW"),
            ('pub(crate) const ERASURE_AUDIT_COLLECTIONS_NEW: &str = "x";', "ERASURE_AUDIT_COLLECTIONS_NEW"),
            ('pub fn is_erasure_bookkeeping_collection_new() {}', "is_erasure_bookkeeping_collection_new"),
        ):
            self.assertIn(
                f"erasure unexpectedly defines {name}",
                owner_errors(root, owner + "\n" + declaration, commit, wal, telemetry),
            )

    def test_cfg_and_exact_value_mutants(self) -> None:
        root, owner, commit, wal, telemetry = self.sources()
        self.assertEqual(owner_errors(root, owner, commit, wal, telemetry), [])
        for name, (visibility, suffix) in CONSTANTS.items():
            declaration = f"{visibility} {name}{suffix}"
            for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
                altered = owner.replace(declaration, attr + "\n" + declaration, 1)
                self.assertNotEqual(altered, owner)
                self.assertIn(f"erasure gates {name}", owner_errors(root, altered, commit, wal, telemetry))
            altered = owner.replace(declaration, declaration.replace(suffix, ': u8 = 7;'), 1)
            self.assertIn(f"erasure lacks exact {name}", owner_errors(root, altered, commit, wal, telemetry))
        for attr in ('#[cfg(feature = "test-hooks")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = owner.replace(OPERATOR_CFG + "\nfn is_erasure_bookkeeping_collection(", attr + "\nfn is_erasure_bookkeeping_collection(", 1)
            self.assertNotEqual(altered, owner)
            self.assertIn("erasure changes is_erasure_bookkeeping_collection cfg", owner_errors(root, altered, commit, wal, telemetry))
        altered = owner.replace(OPERATOR_CFG + "\nfn is_erasure_bookkeeping_collection(", OPERATOR_CFG + '\n#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]\nfn is_erasure_bookkeeping_collection(', 1)
        self.assertIn("erasure changes is_erasure_bookkeeping_collection cfg", owner_errors(root, altered, commit, wal, telemetry))
        for attr in ('#![cfg(feature = "operator")]', '#![cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            self.assertIn("erasure gates whole owner module", owner_errors(root, attr + "\n" + owner, commit, wal, telemetry))
        for attr in ('#[cfg(feature = "operator")]', '#[cfg_attr(feature = "default-embedder", cfg(feature = "test-hooks"))]'):
            altered = root.replace("mod erasure;", attr + "\nmod erasure;", 1)
            self.assertIn("root gates erasure module", owner_errors(altered, owner, commit, wal, telemetry))

    def test_fast_tier_registration(self) -> None:
        self.assertIn("fast test-slice90-erasure-bookkeeping-owner", (ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
