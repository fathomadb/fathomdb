#!/usr/bin/env python3
"""Guard connection setup and SQLite callback ownership after Slice 90 extraction."""

from pathlib import Path
import re
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
ENGINE_SRC = REPO_ROOT / "src/rust/crates/fathomdb-engine/src"
DECLARATIONS = {
    r"const READER_LOOKASIDE_SLOT_SIZE\b": 1,
    r"const READER_LOOKASIDE_SLOT_COUNT\b": 1,
    r"struct ProfileContext\b": 1,
    r"type ProfileContexts\b": 1,
    r"struct ProfileContexts\b": 1,
    r"struct ProfileReleaseObserver\b": 1,
    r"struct ProfileReleaseFact\b": 1,
    r"fn open_managed_connection\b": 1,
    r"fn open_runtime_connection\b": 1,
    r"fn register_sqlite_vec_extension\b": 1,
    r"fn install_profile_callback\b": 1,
    r"fn uninstall_profile_callback\b": 1,
    r"fn apply_perf_experiment_writer_pragmas\b": 1,
    r"fn apply_perf_experiment_reader_pragmas\b": 1,
    r"fn configure_reader_lookaside\b": 1,
    r"fn profile_callback_trampoline\b": 1,
}


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    for declaration, expected in DECLARATIONS.items():
        pattern = re.compile(r"^(?:pub\(crate\) )?(?:unsafe extern \"C\" )?" + declaration, re.M)
        if pattern.search(root):
            errors.append(f"root still defines {declaration}")
        if len(pattern.findall(owner)) != expected:
            errors.append(f"connection owner does not define {expected} {declaration}")
    return errors


class ConnectionRuntimeOwnerTest(unittest.TestCase):
    def test_current_source_has_one_connection_owner(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner = (ENGINE_SRC / "connection_runtime.rs").read_text()
        self.assertEqual(owner_errors(root, owner), [])

    def test_old_root_location_fails_with_complete_owner(self) -> None:
        owner = (ENGINE_SRC / "connection_runtime.rs").read_text()
        declaration = "fn open_managed_connection() {}\n"
        self.assertEqual(owner_errors("", owner), [])
        self.assertEqual(
            owner_errors(declaration, owner),
            [r"root still defines fn open_managed_connection\b"],
        )

    def test_missing_callback_and_duplicate_factory_fail(self) -> None:
        owner = (ENGINE_SRC / "connection_runtime.rs").read_text()
        self.assertEqual(
            owner_errors("", owner.replace("fn profile_callback_trampoline(", "fn missing_callback(")),
            [r"connection owner does not define 1 fn profile_callback_trampoline\b"],
        )
        self.assertEqual(
            owner_errors("", owner + "\nfn open_managed_connection() {}\n"),
            [r"connection owner does not define 1 fn open_managed_connection\b"],
        )

    def test_fast_tier_runs_this_guard(self) -> None:
        registration = (
            "run_tier_suite fast test-slice90-connection-runtime-owner "
            "python3 scripts/tests/test_slice90_connection_runtime_owner.py"
        )
        self.assertIn(registration, (REPO_ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
