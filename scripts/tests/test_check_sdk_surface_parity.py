#!/usr/bin/env python3
"""Mutation coverage for the canonical SDK-operation parity predicate."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
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
        },
        {
            "id": "read.get_many",
            "state": "live",
            "signed_members": ["read.get_many"],
            "python": endpoint("read", "get_many"),
            "typescript": endpoint("read", "getMany"),
        },
        {
            "id": "future.operation",
            "state": "reserved",
            "signed_members": ["future.operation"],
            "python": endpoint("package", "future_operation"),
            "typescript": endpoint("package", "futureOperation"),
        },
    ],
}

PYTHON_LIVE = {"engine_static:open", "read:get_many"}
TYPESCRIPT_LIVE = {"engine_static:open", "read:getMany"}


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


if __name__ == "__main__":
    unittest.main()
