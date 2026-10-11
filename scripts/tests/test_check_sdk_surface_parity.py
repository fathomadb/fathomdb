#!/usr/bin/env python3
"""Mutation coverage for the canonical SDK-operation parity predicate."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import textwrap
import unittest


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "scripts" / "check-sdk-surface-parity.py"


def load_checker():
    spec = importlib.util.spec_from_file_location("check_sdk_surface_parity", CHECKER)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {CHECKER}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


checker = load_checker()


def endpoint(locator: str, spelling: str) -> dict[str, str]:
    return {"locator": locator, "spelling": spelling}


SIGNED = {
    "allowlist": ["Engine.open", "read.get_many", "future.operation"],
    "core": ["Engine.open"],
    "recovery_denylist": ["recover", "restore", "repair", "fix", "rebuild"],
    "runtime_controls": ["admin.configure_runtime", "admin.configureRuntime"],
}

COMPANION = {
    "schema_version": "fathomdb.governed-operation-parity/v1",
    "operations": [
        {
            "id": "engine.open",
            "state": "live",
            "signed_members": ["Engine.open"],
            "python": endpoint("engine_static", "open"),
            "typescript": endpoint("engine_static", "open"),
            "rust": endpoint("engine_static", "open"),
        },
        {
            "id": "read.get_many",
            "state": "live",
            "signed_members": ["read.get_many"],
            "python": endpoint("read", "get_many"),
            "typescript": endpoint("read", "getMany"),
            "rust": endpoint("read", "get_many"),
        },
        {
            "id": "future.operation",
            "state": "reserved",
            "signed_members": ["future.operation"],
            "python": endpoint("package", "future_operation"),
            "typescript": endpoint("package", "futureOperation"),
            "rust": endpoint("package", "future_operation"),
        },
    ],
}

PYTHON_LIVE = {"engine_static:open", "read:get_many"}
RUST_LIVE = {"engine_static:open", "read:get_many"}
RUST_SDK = ROOT / "src" / "rust" / "crates" / "fathomdb-sdk"

# A minimal SDK crate whose observed surface is exactly RUST_LIVE.
RUST_CRATE = {
    "lib.rs": """
        mod engine;
        pub mod read;
        mod embedding;
        pub use embedding::embed_batch_cls;
        pub use engine::Engine;
        pub(crate) fn helper() {}
    """,
    "engine.rs": """
        pub struct Engine { inner: u8 }
        impl Engine {
            pub fn open(
                path: &str,
            ) -> Result<Self, ()> {
                Ok(Self { inner: 0 })
            }
            pub fn drain(&self, timeout_ms: u64) -> Result<(), ()> { Ok(()) }
            pub(crate) fn core(&self) -> &u8 { &self.inner }
            fn private(&self) {}
        }
        impl Drop for Engine {
            fn drop(&mut self) {}
        }
        impl std::fmt::Debug for Engine {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
        }
        #[cfg(test)]
        mod tests {
            pub fn not_surface() {}
        }
    """,
    "read.rs": """
        use crate::Engine;
        pub fn get_many(engine: &Engine, ids: &[String]) -> Result<(), ()> {
            let _ = "{ braces in a string }";
            Ok(())
        }
        pub struct Opts { pub limit: usize }
        impl Opts {
            pub fn new() -> Self { Self { limit: 1 } }
        }
    """,
    "embedding.rs": """
        pub fn embed_batch_cls(texts: &[&str]) -> Result<Vec<Vec<f32>>, ()> { Ok(vec![]) }
    """,
}
TYPESCRIPT_LIVE = {"engine_static:open", "read:getMany"}


def write_crate(root: Path, files: dict[str, str]) -> Path:
    src = root / "src"
    src.mkdir(parents=True, exist_ok=True)
    for name, body in files.items():
        (src / name).write_text(textwrap.dedent(body), encoding="utf-8")
    return root


class ParityValidatorTests(unittest.TestCase):
    def assert_invalid(self, signed, companion, pattern: str) -> None:
        with self.assertRaisesRegex(checker.ParityError, pattern):
            checker.validate_contract(signed, companion)

    def assert_observed_invalid(self, binding: str, observed: set[str], pattern: str) -> None:
        checker.validate_contract(SIGNED, COMPANION)
        with self.assertRaisesRegex(checker.ParityError, pattern):
            checker.validate_observed(COMPANION, binding, observed)

    def test_valid_fixture(self) -> None:
        checker.validate_contract(SIGNED, COMPANION)
        self.assertEqual(
            checker.validate_observed(COMPANION, "python", PYTHON_LIVE),
            {"engine.open", "read.get_many"},
        )
        self.assertEqual(
            checker.validate_observed(COMPANION, "typescript", TYPESCRIPT_LIVE),
            {"engine.open", "read.get_many"},
        )
        self.assertEqual(
            checker.validate_observed(COMPANION, "rust", RUST_LIVE),
            {"engine.open", "read.get_many"},
        )

    def test_repository_contract_is_valid(self) -> None:
        with (ROOT / "src/conformance/governed-surface-allowlist.json").open(
            encoding="utf-8"
        ) as handle:
            signed = json.load(handle)
        with (ROOT / "src/conformance/governed-operation-parity.json").open(
            encoding="utf-8"
        ) as handle:
            companion = json.load(handle)
        checker.validate_contract(signed, companion)
        self.assertEqual(len(signed["allowlist"]), 69)
        self.assertEqual(len(checker.live_canonical_ids(companion)), 44)

    def test_one_sided_removal_fails(self) -> None:
        self.assert_observed_invalid(
            "typescript",
            {"engine_static:open"},
            "missing.*read:getMany.*read.get_many",
        )

    def test_one_sided_addition_fails(self) -> None:
        self.assert_observed_invalid(
            "python", PYTHON_LIVE | {"engine_instance:delete"}, "ungoverned.*delete"
        )

    def test_misspelling_fails(self) -> None:
        mutated = copy.deepcopy(COMPANION)
        mutated["operations"][1]["typescript"]["spelling"] = "get_many"
        checker.validate_contract(SIGNED, mutated)
        with self.assertRaisesRegex(checker.ParityError, "missing.*get_many"):
            checker.validate_observed(mutated, "typescript", TYPESCRIPT_LIVE)

    def test_duplicate_spelling_fails(self) -> None:
        mutated = copy.deepcopy(COMPANION)
        mutated["operations"][2]["state"] = "live"
        mutated["operations"][2]["python"] = endpoint("read", "get_many")
        self.assert_invalid(SIGNED, mutated, "duplicate.*python.*read:get_many")

    def test_reserved_as_live_fails_explicitly(self) -> None:
        self.assert_observed_invalid(
            "typescript", TYPESCRIPT_LIVE | {"package:futureOperation"}, "reserved.*future.operation"
        )

    def test_signed_flattening_drift_fails(self) -> None:
        mutated = copy.deepcopy(COMPANION)
        mutated["operations"][1]["signed_members"] = ["read.getMany"]
        self.assert_invalid(SIGNED, mutated, "signed member.*ADDED.*read.getMany.*REMOVED.*read.get_many")

    def test_incomplete_live_mapping_fails(self) -> None:
        mutated = copy.deepcopy(COMPANION)
        del mutated["operations"][1]["typescript"]
        self.assert_invalid(SIGNED, mutated, "live.*read.get_many.*typescript")

    def test_recovery_denylist_is_exact(self) -> None:
        mutated = copy.deepcopy(SIGNED)
        mutated["recovery_denylist"] = ["recover"]
        self.assert_invalid(mutated, COMPANION, "recovery_denylist")



class RustSdkObserverTests(unittest.TestCase):
    def observe(self, files: dict[str, str]) -> set[str]:
        with tempfile.TemporaryDirectory() as tmp:
            return checker.observe_rust_sdk(write_crate(Path(tmp), files))

    def assert_rust_invalid(self, files: dict[str, str], pattern: str) -> None:
        with self.assertRaisesRegex(checker.ParityError, pattern):
            checker.validate_observed(COMPANION, "rust", self.observe(files))

    def test_fixture_crate_observes_exact_live_set(self) -> None:
        observed = self.observe(RUST_CRATE)
        self.assertEqual(observed, RUST_LIVE)
        self.assertEqual(
            checker.validate_observed(COMPANION, "rust", observed),
            {"engine.open", "read.get_many"},
        )

    def test_missing_rust_operation_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["read.rs"] = "use crate::Engine;\n"
        self.assert_rust_invalid(files, "missing.*read:get_many")

    def test_extra_engine_method_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["engine.rs"] = files["engine.rs"].replace(
            "fn private(&self) {}", "pub fn read_get(&self, id: &str) {}"
        )
        self.assert_rust_invalid(files, "ungoverned.*engine_instance:read_get")

    def test_engine_method_in_another_file_is_observed(self) -> None:
        files = dict(RUST_CRATE)
        files["search.rs"] = "impl crate::Engine {\n    pub fn search_with_limit(&self) {}\n}\n"
        self.assert_rust_invalid(files, "ungoverned.*engine_instance:search_with_limit")

    def test_renamed_namespace_function_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["read.rs"] = files["read.rs"].replace("pub fn get_many", "pub fn getmany")
        self.assert_rust_invalid(files, "ungoverned.*read:getmany.*missing.*read:get_many")

    def test_extra_root_function_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["embedding.rs"] += "pub fn execute_sql(sql: &str) {}\n"
        self.assert_rust_invalid(files, "ungoverned.*package:execute_sql")

    def test_unapproved_trait_impl_for_engine_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["engine.rs"] += textwrap.dedent("""
            impl std::ops::Deref for Engine {
                type Target = u8;
                fn deref(&self) -> &u8 { &self.inner }
            }
        """)
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(checker.ParityError, "trait impl.*Deref.*Engine"):
                checker.observe_rust_sdk(write_crate(Path(tmp), files))

    def assert_observe_refused(self, files: dict[str, str], pattern: str) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(checker.ParityError, pattern):
                checker.observe_rust_sdk(write_crate(Path(tmp), files))

    def test_glob_reexport_of_core_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["lib.rs"] += "pub use fathomdb_engine::*;\n"
        self.assert_observe_refused(files, "re-export.*fathomdb_engine::\\*")

    def test_core_engine_reexport_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["lib.rs"] += "pub use fathomdb_engine::{ReadView, Engine as Core};\n"
        self.assert_observe_refused(files, "re-export.*Engine")

    def test_public_engine_field_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["engine.rs"] = files["engine.rs"].replace(
            "pub struct Engine { inner: u8 }", "pub struct Engine { pub inner: u8 }"
        )
        self.assert_observe_refused(files, "public field.*Engine")

    def test_unexpected_public_module_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["lib.rs"] += "pub mod ops;\n"
        files["ops.rs"] = "pub fn helper() {}\n"
        self.assert_observe_refused(files, "pub mod ops")

    def test_core_reexport_variants_fail(self) -> None:
        leaks = {
            "crate alias": "pub use fathomdb_engine as core_engine;\n",
            "self alias": "pub use fathomdb_engine::{self as fe};\n",
            "absolute path": "pub use ::fathomdb_engine::Engine as X;\n",
            "type alias": "pub type CoreEng = fathomdb_engine::Engine;\n",
            "extern crate": "pub extern crate fathomdb_engine;\n",
            "function": "pub use fathomdb_engine::recover_truncate_wal;\n",
        }
        for label, line in leaks.items():
            with self.subTest(label):
                files = dict(RUST_CRATE)
                files["lib.rs"] += line
                self.assert_observe_refused(files, "fathomdb_engine")

    def test_core_engine_in_public_signature_or_impl_fails(self) -> None:
        cases = {
            "method on helper": (
                "engine.rs",
                "use fathomdb_engine::Engine as CoreEngine;\n"
                "pub struct Slot;\nimpl Slot {\n    pub fn leak(e: &Engine) -> &CoreEngine { todo!() }\n}\n",
            ),
            "impl target": (
                "engine.rs",
                "use fathomdb_engine::Engine as CoreEngine;\n"
                "impl<'a> From<&'a Engine> for &'a CoreEngine {\n    fn from(e: &'a Engine) -> Self { todo!() }\n}\n",
            ),
        }
        for label, (name, extra) in cases.items():
            with self.subTest(label):
                files = dict(RUST_CRATE)
                files[name] += extra
                self.assert_observe_refused(files, "core engine")

    def test_public_tuple_engine_field_fails(self) -> None:
        files = dict(RUST_CRATE)
        files["engine.rs"] = files["engine.rs"].replace(
            "pub struct Engine { inner: u8 }", "pub struct Engine(pub u8);"
        )
        self.assert_observe_refused(files, "public field.*Engine")

    def test_escaped_char_literals_do_not_break_brace_matching(self) -> None:
        files = dict(RUST_CRATE)
        files["read.rs"] += "pub(crate) fn chars() -> [char; 3] { ['\\u{7b}', '\\x7b', '{'] }\n"
        self.assertEqual(self.observe(files), RUST_LIVE)

    def test_async_engine_method_is_observed(self) -> None:
        files = dict(RUST_CRATE)
        files["engine.rs"] = files["engine.rs"].replace(
            "fn private(&self) {}", "pub async fn search_later(&self) {}"
        )
        self.assert_rust_invalid(files, "ungoverned.*engine_instance:search_later")

    def test_repository_rust_sdk_matches_live_operations(self) -> None:
        with (ROOT / "src/conformance/governed-operation-parity.json").open(
            encoding="utf-8"
        ) as handle:
            companion = json.load(handle)
        observed = checker.observe_rust_sdk(RUST_SDK)
        self.assertEqual(
            checker.validate_observed(companion, "rust", observed),
            checker.live_canonical_ids(companion),
        )
        self.assertEqual(len(observed), 44)

    def test_cli_observes_rust_crate(self) -> None:
        self.assertEqual(checker.main(["--binding", "rust", "--rust-crate", str(RUST_SDK)]), 0)


if __name__ == "__main__":
    unittest.main()
