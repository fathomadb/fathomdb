#!/usr/bin/env python3
"""Guard the operator-only offline recovery and inspection owner."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
PUBLIC = ("recover_truncate_wal", "inspect_data_plane_integrity")
HELPERS = (
    "data_plane_inspection_error",
    "data_plane_sidecar_path",
    "immutable_sqlite_uri",
    "validate_recovery_database_file",
    "validate_effective_recovery_schema",
    "validate_recovery_schema_invariants",
    "recovery_schema_corruption",
)
SHARED_URI = ("read_only_sqlite_uri", "sqlite_uri")


def owner_errors(root: str, operator: str, data_plane: str, opened: str) -> list[str]:
    errors = []
    family = (
        r"(?:recover_truncate_wal|inspect_data_plane_integrity|data_plane_"
        r"|immutable_sqlite_uri|validate_recovery_|recovery_schema_corruption)\w*"
    )
    for name in re.findall(r"^(?:pub(?:\([^)]*\))? )?fn (" + family + r")\(", root, re.M):
        errors.append(f"root still defines operator recovery {name}")
    for name in SHARED_URI:
        if re.search(r"^fn " + name + r"\(", root, re.M):
            errors.append(f"root still defines shared URI helper {name}")
        if len(re.findall(r"^pub\(crate\) fn " + name + r"\(", opened, re.M)) != 1:
            errors.append(f"open owner lacks one shared URI helper {name}")
    for name in PUBLIC + HELPERS:
        visibility = "pub " if name in PUBLIC else ""
        pattern = (
            r'^#\[cfg\(feature = "operator"\)\]\n'
            + visibility
            + r"fn "
            + name
            + r"\("
        )
        if len(re.findall(pattern, data_plane, re.M)) != 1:
            errors.append(f"operator data-plane owner lacks cfg(operator) {name}")
    if not re.search(r"^#\[cfg\(feature = \"operator\"\)\]\nmod data_plane;", operator, re.M):
        errors.append("operator lacks gated private data_plane submodule")
    operator_export = re.search(r'^#\[cfg\(feature = "operator"\)\]\npub use data_plane::\{([^}]*)\};', operator, re.M)
    if not operator_export or any(name not in operator_export.group(1).split(", ") for name in PUBLIC):
        errors.append("operator lacks gated public function reexports")
    root_export = re.search(r'^#\[cfg\(feature = "operator"\)\]\npub use operator::\{([^}]*)\};', root, re.M)
    if not root_export or any(name not in root_export.group(1).split(", ") for name in PUBLIC):
        errors.append("root lacks gated public function reexports")
    if "use crate::open::{read_only_sqlite_uri, sqlite_uri};" not in data_plane:
        errors.append("operator data-plane does not import the shared URI owner")
    return errors


class OperatorRecoveryOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return tuple(
            (SRC / name).read_text()
            for name in ("lib.rs", "operator.rs", "operator/data_plane.rs", "open.rs")
        )

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_family_with_complete_owner(self) -> None:
        root, operator, data_plane, opened = self.sources()
        self.assertEqual(owner_errors(root, operator, data_plane, opened), [])
        for declaration, expected in (
            ("fn validate_recovery_new() {}", "root still defines operator recovery validate_recovery_new"),
            ("fn data_plane_new() {}", "root still defines operator recovery data_plane_new"),
            ("pub fn inspect_data_plane_integrity_new() {}", "root still defines operator recovery inspect_data_plane_integrity_new"),
            ("pub fn recover_truncate_wal_new() {}", "root still defines operator recovery recover_truncate_wal_new"),
        ):
            with self.subTest(declaration=declaration):
                self.assertIn(expected, owner_errors(root + "\n" + declaration, operator, data_plane, opened))

    def test_cfg_and_reexport_mutants(self) -> None:
        root, operator, data_plane, opened = self.sources()
        self.assertEqual(owner_errors(root, operator, data_plane, opened), [])
        for name in PUBLIC + HELPERS:
            altered = re.sub(
                r'(#\[cfg\(feature = "operator"\)\]\n)(pub )?fn ' + name + r"\(",
                lambda match: (match.group(2) or "") + "fn " + name + "(",
                data_plane,
                count=1,
            )
            with self.subTest(name=name):
                self.assertNotEqual(altered, data_plane)
                self.assertIn(
                    f"operator data-plane owner lacks cfg(operator) {name}",
                    owner_errors(root, operator, altered, opened),
                )
        self.assertIn(
            "root lacks gated public function reexports",
            owner_errors(root.replace("inspect_data_plane_integrity, ", "", 1), operator, data_plane, opened),
        )

    def test_fast_tier_registration(self) -> None:
        tier = (ROOT / "scripts/agent-test.sh").read_text()
        self.assertIn("fast test-slice90-operator-recovery-owner", tier)


if __name__ == "__main__":
    unittest.main()
