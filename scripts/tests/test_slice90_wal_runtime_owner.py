#!/usr/bin/env python3
"""Guard Engine WAL orchestration and runtime inventory ownership."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
TYPES = (
    "ManagedConnectionRegistry",
    "ManagedConnectionCategory",
    "ManagedConnectionRegistration",
    "RuntimeProbeLifecycle",
    "RuntimeProbeRegistration",
    "RuntimeProbeConnection",
    "RuntimeConnectionInventoryRequest",
    "RuntimeNativeStateRequest",
    "TruncateWalStatus",
    "TruncateWalReport",
)
METHODS = (
    "d27_connection_inventory_for_test",
    "wal_attribution_snapshot",
    "wal_attribution_checkpoints_for_test",
    "pause_reader_after_wal_snapshot_for_test",
    "pause_reader_with_timeout_for_test",
    "post_commit_ack_for_test",
    "arm_next_reader_snapshot_pause_for_test",
    "arm_next_reader_completion_pause_for_test",
    "pause_next_reader_handoff_for_test",
    "wal_attribution_checkpoint_records_for_test",
    "wal_attribution_idle_for_test",
    "arm_actual_checkpoint_observation_for_test",
    "actual_checkpoint_observation_for_test",
    "actual_checkpoint_direct_inventory_for_test",
    "take_actual_checkpoint_observations_for_test",
    "arm_python_serial_actual_checkpoint_observation_for_test",
    "drain_actual_checkpoint_observations_for_test",
    "arm_binding_native_state_observation_for_test",
    "binding_native_state_observation_for_test",
    "drain_binding_native_state_observations_for_test",
    "native_state_inventory_for_test",
    "binding_native_state_inventory_for_test",
    "binding_connection_inventory_for_test",
    "checkpoint_at_rest_for_test",
    "native_raw_wal_checkpoint_for_test",
    "truncate_wal",
    "wal_checkpoint_truncate_once",
)
ROOT_PUBLIC = ("TruncateWalStatus", "TruncateWalReport")


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    families = re.compile(
        r"^(?:(?:pub(?:\([^)]*\))?) )?(?:struct|enum) "
        r"((?:ManagedConnection|RuntimeProbe|RuntimeConnectionInventory|"
        r"RuntimeNativeState|TruncateWal)\w*)\b",
        re.M,
    )
    for match in families.finditer(root):
        errors.append(f"root still defines wal::{match.group(1)}")
    root_impls = re.findall(r"^impl Engine \{.*?^\}", root, re.M | re.S)
    method_family = re.compile(
        r"^    (?:(?:pub(?:\([^)]*\))?) )?fn "
        r"((?:d27_connection_inventory|wal_attribution_|pause_reader_|"
        r"post_commit_ack|arm_next_reader_|pause_next_reader_|"
        r"arm_actual_checkpoint|actual_checkpoint_|take_actual_checkpoint|"
        r"arm_python_serial_actual_checkpoint|drain_actual_checkpoint|"
        r"arm_binding_native_state|binding_native_state_|drain_binding_native_state|"
        r"native_state_inventory|binding_connection_inventory|checkpoint_at_rest|"
        r"native_raw_wal_checkpoint|truncate_wal|wal_checkpoint_truncate)\w*)\(",
        re.M,
    )
    for engine_impl in root_impls:
        for match in method_family.finditer(engine_impl):
            errors.append(f"root still defines wal::{match.group(1)}")
    for name in TYPES:
        kind = "enum" if name in ("ManagedConnectionCategory", "TruncateWalStatus") else "struct"
        pattern = r"^(?:(?:pub(?:\([^)]*\))?) )?" + kind + " " + name + r"\b"
        if len(re.findall(pattern, owner, re.M)) != 1:
            errors.append(f"WAL owner does not define one {name}")
    owner_impls = re.findall(r"^impl Engine \{.*?^\}", owner, re.M | re.S)
    if len(owner_impls) != 1:
        errors.append("WAL owner does not define one Engine impl")
    owner_engine = owner_impls[0] if len(owner_impls) == 1 else ""
    for name in METHODS:
        pattern = r"^    (?:(?:pub(?:\([^)]*\))?) )?fn " + name + r"\("
        if len(re.findall(pattern, owner_engine, re.M)) != 1:
            errors.append(f"WAL owner does not define one Engine::{name}")
    exports = " ".join(re.findall(r"^pub use wal_runtime::\{(.*?)\};", root, re.M | re.S))
    for name in ROOT_PUBLIC:
        if not re.search(r"\b" + name + r"\b", exports):
            errors.append(f"root does not re-export wal_runtime::{name}")
    cfg_items = (
        ("test", "struct RuntimeProbeLifecycle"),
        ("test", "struct RuntimeProbeRegistration"),
        ("test", "struct RuntimeProbeConnection"),
        ('any(test, feature = "test-hooks")', "struct RuntimeConnectionInventoryRequest"),
        ('any(test, feature = "test-hooks")', "struct RuntimeNativeStateRequest"),
    )
    for predicate, declaration in cfg_items:
        pattern = (
            r"^#\[cfg\(" + re.escape(predicate) + r"\)\]\n"
            r"(?:#\[[^\n]+\]\n)*"
            r"(?:(?:pub(?:\([^)]*\))?) )?" + declaration + r"\b"
        )
        if len(re.findall(pattern, owner, re.M)) != 1:
            errors.append(f"WAL owner lacks cfg({predicate}) {declaration}")
    cfg_methods = (
        ('any(test, debug_assertions, feature = "test-hooks")', "pause_reader_after_wal_snapshot_for_test"),
        ('any(test, feature = "test-hooks")', "arm_actual_checkpoint_observation_for_test"),
        ('feature = "test-hooks"', "native_raw_wal_checkpoint_for_test"),
    )
    for predicate, method in cfg_methods:
        pattern = (
            r"^    #\[cfg\(" + re.escape(predicate) + r"\)\]\n"
            r"(?:    #\[[^\n]+\]\n)*"
            r"    (?:(?:pub(?:\([^)]*\))?) )?fn " + method + r"\("
        )
        if len(re.findall(pattern, owner_engine, re.M)) != 1:
            errors.append(f"WAL owner lacks cfg({predicate}) Engine::{method}")
    return errors


class WalRuntimeOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        path = SRC / "wal_runtime.rs"
        return (SRC / "lib.rs").read_text(), path.read_text() if path.exists() else ""

    def test_current_source_has_one_wal_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_family_with_complete_owner(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for item, name in (
            ("struct ManagedConnectionExtra;", "ManagedConnectionExtra"),
            ("struct RuntimeProbeExtra;", "RuntimeProbeExtra"),
            ("struct RuntimeConnectionInventoryExtra;", "RuntimeConnectionInventoryExtra"),
            ("struct RuntimeNativeStateExtra;", "RuntimeNativeStateExtra"),
            ("struct TruncateWalExtra;", "TruncateWalExtra"),
            ("impl Engine {\n    fn wal_checkpoint_truncate_new(&self) {}\n}", "wal_checkpoint_truncate_new"),
            ("impl Engine {\n    fn actual_checkpoint_new(&self) {}\n}", "actual_checkpoint_new"),
            ("impl Engine {\n    fn native_state_inventory_new(&self) {}\n}", "native_state_inventory_new"),
            ("impl Engine {\n    fn binding_native_state_new(&self) {}\n}", "binding_native_state_new"),
            ("impl Engine {\n    fn pause_reader_new(&self) {}\n}", "pause_reader_new"),
            ("impl Engine {\n    fn truncate_wal_new(&self) {}\n}", "truncate_wal_new"),
        ):
            with self.subTest(name=name):
                self.assertIn(f"root still defines wal::{name}", owner_errors(root + "\n" + item, owner))

    def test_missing_public_reexport_fails(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        changed, count = re.subn(
            r"(^pub use wal_runtime::\{[^}]*?)\bTruncateWalReport\b",
            r"\1RemovedTruncateWalReport",
            root,
            count=1,
            flags=re.M | re.S,
        )
        self.assertEqual(count, 1)
        self.assertIn("root does not re-export wal_runtime::TruncateWalReport", owner_errors(changed, owner))

    def test_cfg_arms_are_exact(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for old, new, expected in (
            (
                "#[cfg(test)]\npub(crate) struct RuntimeProbeRegistration",
                '#[cfg(feature = "test-hooks")]\npub(crate) struct RuntimeProbeRegistration',
                "WAL owner lacks cfg(test) struct RuntimeProbeRegistration",
            ),
            (
                '#[cfg(any(test, feature = "test-hooks"))]\npub(crate) struct RuntimeNativeStateRequest',
                '#[cfg(feature = "test-hooks")]\npub(crate) struct RuntimeNativeStateRequest',
                'WAL owner lacks cfg(any(test, feature = "test-hooks")) struct RuntimeNativeStateRequest',
            ),
            (
                '    #[cfg(any(test, debug_assertions, feature = "test-hooks"))]',
                '    #[cfg(feature = "test-hooks")]',
                'WAL owner lacks cfg(any(test, debug_assertions, feature = "test-hooks")) Engine::pause_reader_after_wal_snapshot_for_test',
            ),
        ):
            with self.subTest(expected=expected):
                self.assertIn(old, owner)
                self.assertIn(expected, owner_errors(root, owner.replace(old, new, 1)))

    def test_fast_tier_registration(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice90-wal-runtime-owner "
            "python3 scripts/tests/test_slice90_wal_runtime_owner.py",
            (ROOT / "scripts/agent-test.sh").read_text(),
        )


if __name__ == "__main__":
    unittest.main()
