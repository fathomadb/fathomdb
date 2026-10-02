#!/usr/bin/env python3
"""Guard erasure retry policy and projection worker inventory replies at their owners."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
CONSTANTS = ("ERASURE_WAL_TRUNCATE_ATTEMPTS", "ERASURE_WAL_TRUNCATE_BACKOFF_MS")
ARMS = (
    "report_runtime_connection_inventory_for_test",
    "report_runtime_native_state_inventory_for_test",
)


def owner_errors(root: str, wal: str, worker: str, erasure: str) -> list[str]:
    errors = []
    for name in re.findall(
        r"^(?:(?:pub(?:\([^)]*\))?) )?const (ERASURE_WAL_TRUNCATE_\w+)\b",
        root,
        re.M,
    ):
        errors.append(f"root still defines erasure policy {name}")
    for name in re.findall(
        r"^(?:(?:pub(?:\([^)]*\))?) )?fn (report_runtime_\w+_inventory\w*)\(",
        root,
        re.M,
    ):
        errors.append(f"root still defines worker arm {name}")
    for name in CONSTANTS:
        if len(re.findall(r"^pub\(crate\) const " + name + r"\b", erasure, re.M)) != 1:
            errors.append(f"erasure owner lacks one policy {name}")
        if re.search(r"^(?:(?:pub(?:\([^)]*\))?) )?const " + name + r"\b", wal, re.M):
            errors.append(f"WAL owner still defines erasure policy {name}")
    for name in ARMS:
        pattern = (
            r'^#\[cfg\(any\(test, feature = "test-hooks"\)\)\]\n'
            r"(?:#\[[^\n]+\]\n)*"
            r"fn " + name + r"\("
        )
        if len(re.findall(pattern, worker, re.M)) != 1:
            errors.append(f"projection worker lacks cfg arm {name}")
    return errors


class WalInventoryArmsOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return tuple(
            (SRC / name).read_text()
            for name in ("lib.rs", "wal_runtime.rs", "projection_worker.rs", "erasure.rs")
        )

    def test_current_source_has_one_owner_per_item(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_families_with_complete_owners(self) -> None:
        root, wal, worker, erasure = self.sources()
        for declaration, expected in (
            (
                "const ERASURE_WAL_TRUNCATE_NEW: u32 = 1;",
                "root still defines erasure policy ERASURE_WAL_TRUNCATE_NEW",
            ),
            (
                "fn report_runtime_connection_inventory_new() {}",
                "root still defines worker arm report_runtime_connection_inventory_new",
            ),
            (
                "fn report_runtime_native_state_inventory_new() {}",
                "root still defines worker arm report_runtime_native_state_inventory_new",
            ),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, wal, worker, erasure))

    def test_wal_owner_decoy_cannot_replace_erasure_owner(self) -> None:
        root, wal, worker, erasure = self.sources()
        self.assertIn(
            "WAL owner still defines erasure policy ERASURE_WAL_TRUNCATE_ATTEMPTS",
            owner_errors(
                root,
                wal + "\npub(crate) const ERASURE_WAL_TRUNCATE_ATTEMPTS: u32 = 5;",
                worker,
                erasure,
            ),
        )
        self.assertIn(
            "erasure owner lacks one policy ERASURE_WAL_TRUNCATE_BACKOFF_MS",
            owner_errors(
                root,
                wal,
                worker,
                erasure.replace(
                    "const ERASURE_WAL_TRUNCATE_BACKOFF_MS",
                    "const REMOVED_ERASURE_WAL_TRUNCATE_BACKOFF_MS",
                    1,
                ),
            ),
        )

    def test_cfg_arm_mutation_fails(self) -> None:
        root, wal, worker, erasure = self.sources()
        old = (
            '#[cfg(any(test, feature = "test-hooks"))]\n'
            "fn report_runtime_native_state_inventory_for_test("
        )
        new = old.replace('any(test, feature = "test-hooks")', 'feature = "test-hooks"')
        self.assertIn(old, worker)
        self.assertIn(
            "projection worker lacks cfg arm report_runtime_native_state_inventory_for_test",
            owner_errors(root, wal, worker.replace(old, new, 1), erasure),
        )

    def test_fast_tier_registration(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice90-wal-inventory-arms-owner "
            "python3 scripts/tests/test_slice90_wal_inventory_arms_owner.py",
            (ROOT / "scripts/agent-test.sh").read_text(),
        )


if __name__ == "__main__":
    unittest.main()
