#!/usr/bin/env python3
"""Guard ownership of Engine shutdown and drain methods."""

from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src/rust/crates/fathomdb-engine/src"
METHODS = ("close", "drain", "drain_for_non_embedding_mutation")
OTHER_OWNER_DRAINS = {
    "drain_actual_checkpoint_observations_for_test",
    "drain_binding_native_state_observations_for_test",
    "drain_mean_centering_events_for_test",
    "drain_embedder_events",
}


def owner_errors(root: str, owner: str) -> list[str]:
    errors = []
    for engine_impl in re.finditer(r"^impl Engine \{.*?^\}", root, re.M | re.S):
        for match in re.finditer(
            r"^    (?:(?:pub(?:\([^)]*\))?) )?fn ((?:close|drain)\w*)\(",
            engine_impl.group(),
            re.M,
        ):
            if match.group(1) not in OTHER_OWNER_DRAINS:
                errors.append(f"root still defines lifecycle::{match.group(1)}")
    if re.search(r"^impl Drop for Engine \{", root, re.M):
        errors.append("root still defines Engine Drop")
    owner_impls = list(re.finditer(r"^impl Engine \{.*?^\}", owner, re.M | re.S))
    if len(owner_impls) != 1:
        errors.append("lifecycle owner does not define one Engine impl")
    owner_engine = owner_impls[0].group() if len(owner_impls) == 1 else ""
    if len(re.findall(r"^impl Drop for Engine \{", owner, re.M)) != 1:
        errors.append("lifecycle owner does not define one Engine Drop")
    for name in METHODS:
        visibility = "pub " if name != "drain_for_non_embedding_mutation" else "pub(crate) "
        pattern = re.compile(r"^    " + re.escape(visibility) + r"fn " + name + r"\(&self", re.M)
        if len(pattern.findall(owner_engine)) != 1:
            errors.append(f"lifecycle owner does not define one Engine::{name}")
    return errors


class RuntimeLifecycleOwnerTest(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        return (SRC / "lib.rs").read_text(), (SRC / "runtime_lifecycle.rs").read_text() if (SRC / "runtime_lifecycle.rs").exists() else ""

    def test_current_source_has_one_owner(self) -> None:
        self.assertEqual(owner_errors(*self.sources()), [])

    def test_unlisted_root_method_with_complete_owner(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        for method, visibility in (
            ("close_new", "pub "),
            ("drain_new", "pub(crate) "),
            ("drain_for_non_embedding_mutation_new", ""),
        ):
            with self.subTest(method=method):
                injected = f"\nimpl Engine {{\n    {visibility}fn {method}(&self) {{}}\n}}\n"
                self.assertIn(
                    f"root still defines lifecycle::{method}",
                    owner_errors(root + injected, owner),
                )

    def test_root_drop_and_moved_method_mutants(self) -> None:
        root, owner = self.sources()
        self.assertEqual(owner_errors(root, owner), [])
        self.assertIn(
            "root still defines Engine Drop",
            owner_errors(root + "\nimpl Drop for Engine { fn drop(&mut self) {} }\n", owner),
        )
        self.assertIn(
            "lifecycle owner does not define one Engine::close",
            owner_errors(root, owner.replace("pub fn close(&self)", "pub fn stop(&self)", 1)),
        )
        moved_out = owner.replace("    pub fn close(&self)", "    pub fn stop(&self)", 1)
        moved_out += "\nimpl Other {\n    pub fn close(&self) {}\n}\n"
        self.assertIn(
            "lifecycle owner does not define one Engine::close",
            owner_errors(root, moved_out),
        )

    def test_fast_tier_registration(self) -> None:
        self.assertIn(
            "run_tier_suite fast test-slice90-runtime-lifecycle-owner "
            "python3 scripts/tests/test_slice90_runtime_lifecycle_owner.py",
            (ROOT / "scripts/agent-test.sh").read_text(),
        )


if __name__ == "__main__":
    unittest.main()
