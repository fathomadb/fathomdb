#!/usr/bin/env python3
"""Guard the common Engine open sequence after Slice 90 extraction."""

from pathlib import Path
import re
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
ENGINE_SRC = REPO_ROOT / "src/rust/crates/fathomdb-engine/src"
METHODS = {
    "open": 1,
    "open_with_choice": 1,
    "open_with_choice_and_config": 1,
    "open_default_embedder": 2,
    "open_with_migration_event_sink": 1,
    "open_with_migrations_for_test": 1,
    "open_with_subscriber_for_test": 1,
    "open_without_embedder_for_test": 1,
    "open_with_embedder_for_test": 1,
    "open_with_embedder_and_subscriber": 1,
    "open_with_embedder_and_subscriber_config": 1,
    "open_with_migrations": 1,
    "open_locked": 1,
}


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    engine_impl = re.compile(r"^impl Engine \{(?P<body>.*?)^}", re.M | re.S)
    open_method = re.compile(
        r"^    (?:pub(?:\([^)]*\))? )?(?:(?:async|const|unsafe) )*fn (open\w*)\(",
        re.M,
    )
    for impl_match in engine_impl.finditer(root):
        for method_match in open_method.finditer(impl_match.group("body")):
            errors.append(f"root still defines Engine::{method_match.group(1)}")

    for method, expected in METHODS.items():
        visibility = r"pub " if method == "open" else r"(?:(?:pub|pub\(crate\)) )?"
        pattern = re.compile(r"^    " + visibility + r"fn " + method + r"\(", re.M)
        if len(pattern.findall(owner)) != expected:
            errors.append(f"open owner does not define {expected} Engine::{method}")
    for predicate in (
        'feature = "default-embedder"',
        'not(feature = "default-embedder")',
    ):
        arm = re.compile(
            r"^    #\[cfg\(" + re.escape(predicate) + r"\)\]\n"
            r"    fn open_default_embedder\(",
            re.M,
        )
        if len(arm.findall(owner)) != 1:
            errors.append(
                f"open owner lacks cfg({predicate}) Engine::open_default_embedder arm"
            )
    return errors


class OpenMethodsOwnerTest(unittest.TestCase):
    def test_current_source_has_one_open_method_owner(self) -> None:
        root = (ENGINE_SRC / "lib.rs").read_text()
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors(root, owner), [])

    def test_old_root_location_fails_with_complete_owner(self) -> None:
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors("", owner), [])
        self.assertEqual(
            owner_errors("impl Engine {\n    fn open_locked() {}\n}", owner),
            ["root still defines Engine::open_locked"],
        )

    def test_unlisted_root_open_method_fails_with_complete_owner(self) -> None:
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors("", owner), [])
        self.assertEqual(
            owner_errors("impl Engine {\n    pub fn open_new() {}\n}", owner),
            ["root still defines Engine::open_new"],
        )
        self.assertEqual(
            owner_errors(
                'impl Engine {\n    #[cfg(feature = "operator")]\n'
                "    pub(crate) fn open_guarded() {}\n}",
                owner,
            ),
            ["root still defines Engine::open_guarded"],
        )
        self.assertEqual(
            owner_errors("impl RuntimeProbeConnection {\n    fn open() {}\n}", owner),
            [],
        )

    def test_default_embedder_cfg_arms_are_exact(self) -> None:
        owner = (ENGINE_SRC / "open.rs").read_text()
        self.assertEqual(owner_errors("", owner), [])
        mutated = owner.replace(
            '#[cfg(not(feature = "default-embedder"))]',
            '#[cfg(feature = "default-embedder")]',
            1,
        )
        self.assertIn(
            'open owner lacks cfg(not(feature = "default-embedder")) Engine::open_default_embedder arm',
            owner_errors("", mutated),
        )

    def test_missing_method_and_cfg_twin_fail(self) -> None:
        owner = (ENGINE_SRC / "open.rs").read_text()
        missing = owner.replace(
            "fn open_with_migrations(", "fn removed_open_with_migrations("
        )
        self.assertIn(
            "open owner does not define 1 Engine::open_with_migrations",
            owner_errors("", missing),
        )
        if owner.count("fn open_default_embedder(") == 2:
            missing_cfg_twin = owner.replace(
                "fn open_default_embedder(", "fn removed_default(", 1
            )
            self.assertIn(
                "open owner does not define 2 Engine::open_default_embedder",
                owner_errors("", missing_cfg_twin),
            )

    def test_fast_tier_runs_this_guard(self) -> None:
        registration = (
            "run_tier_suite fast test-slice90-open-methods-owner "
            "python3 scripts/tests/test_slice90_open_methods_owner.py"
        )
        self.assertIn(registration, (REPO_ROOT / "scripts/agent-test.sh").read_text())


if __name__ == "__main__":
    unittest.main()
