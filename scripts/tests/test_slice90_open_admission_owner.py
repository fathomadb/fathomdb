#!/usr/bin/env python3
"""Guard the first Slice 90 open/admission owner extraction."""

from pathlib import Path
import re
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
ENGINE_SRC = REPO_ROOT / "src/rust/crates/fathomdb-engine/src"
DECLARATIONS = (
    r"enum DatabaseAdmission\b",
    r"struct DatabaseOpenPlan\b",
    r"struct ShmSnapshot\b",
    r"struct PendingDatabaseLock\b",
    r"fn read_effective_schema_version\b",
    r"fn admit_current_database\b",
    r"fn canonical_database_path\b",
    r"fn acquire_lock_without_metadata_mutation\b",
    r"fn lock_path\b",
    r"fn read_holder_pid\b",
)


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    for declaration in DECLARATIONS:
        pattern = re.compile(r"^(?:pub\(crate\) )?" + declaration, re.M)
        if pattern.search(root):
            errors.append(f"root still defines {declaration}")
        if len(pattern.findall(owner)) != 1:
            errors.append(f"open owner does not define one {declaration}")
    return errors


class OpenAdmissionOwnerTest(unittest.TestCase):
    def test_current_source_has_one_open_admission_owner(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner_path = ENGINE_SRC / "open.rs"
        owner = owner_path.read_text() if owner_path.exists() else ""
        self.assertEqual(owner_errors(root, owner), [])

    def test_old_root_location_is_rejected_with_complete_owner(self) -> None:
        owner = "\n".join(
            re.sub(r"\\b$", "", declaration) + " {}" for declaration in DECLARATIONS
        )
        self.assertEqual(owner_errors("", owner), [])
        self.assertEqual(
            owner_errors("fn lock_path() {}", owner),
            [r"root still defines fn lock_path\b"],
        )

    def test_missing_owner_is_rejected(self) -> None:
        owner = (
            (ENGINE_SRC / "open.rs").read_text()
            if (ENGINE_SRC / "open.rs").exists()
            else ""
        )
        without = owner.replace("fn admit_current_database(", "fn removed_admission(")
        self.assertIn(
            r"open owner does not define one fn admit_current_database\b",
            owner_errors("", without),
        )

    def test_fast_tier_runs_this_guard(self) -> None:
        registration = (
            "run_tier_suite fast test-slice90-open-admission-owner "
            "python3 scripts/tests/test_slice90_open_admission_owner.py"
        )
        self.assertIn(registration, (REPO_ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
