#!/usr/bin/env python3
"""Validate the governed canonical-operation map and observed SDK surface."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any


SCHEMA = "fathomdb.governed-operation-parity/v1"
BINDINGS = ("python", "typescript")
LOCATORS = {"package", "engine_static", "engine_instance", "admin", "read", "graph"}
RECOVERY_DENYLIST = ["recover", "restore", "repair", "fix", "rebuild"]


class ParityError(ValueError):
    """The canonical map or an observed binding surface is inconsistent."""


def _strings(value: Any, label: str, *, nonempty: bool = False) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise ParityError(f"{label} must be a list of non-empty strings")
    if nonempty and not value:
        raise ParityError(f"{label} must not be empty")
    if len(value) != len(set(value)):
        raise ParityError(f"{label} contains duplicate members")
    return value


def _endpoint(operation: dict[str, Any], binding: str) -> str:
    operation_id = operation.get("id", "<missing>")
    value = operation.get(binding)
    if not isinstance(value, dict):
        raise ParityError(
            f"{operation.get('state', '<missing>')} operation {operation_id} has no {binding} mapping"
        )
    locator = value.get("locator")
    spelling = value.get("spelling")
    if locator not in LOCATORS:
        raise ParityError(
            f"operation {operation_id} {binding} locator {locator!r} is not in {sorted(LOCATORS)}"
        )
    if not isinstance(spelling, str) or not spelling:
        raise ParityError(f"operation {operation_id} {binding} spelling must be non-empty")
    if ":" in spelling:
        raise ParityError(f"operation {operation_id} {binding} spelling must not contain ':'")
    return f"{locator}:{spelling}"


def validate_contract(signed: dict[str, Any], companion: dict[str, Any]) -> None:
    """Validate mapping structure and exact signed-member preservation."""
    if not isinstance(signed, dict) or not isinstance(companion, dict):
        raise ParityError("signed and companion contracts must be JSON objects")
    allowlist = _strings(signed.get("allowlist"), "signed allowlist", nonempty=True)
    denylist = _strings(signed.get("recovery_denylist"), "recovery_denylist")
    if denylist != RECOVERY_DENYLIST:
        raise ParityError(
            f"recovery_denylist must remain exactly {RECOVERY_DENYLIST!r}, got {denylist!r}"
        )
    if companion.get("schema_version") != SCHEMA:
        raise ParityError(f"schema_version must be {SCHEMA!r}")
    operations = companion.get("operations")
    if not isinstance(operations, list) or not operations:
        raise ParityError("operations must be a non-empty list")

    ids: set[str] = set()
    signed_members: list[str] = []
    per_binding: dict[str, dict[str, str]] = {binding: {} for binding in BINDINGS}
    cross_binding: dict[str, str] = {}
    for index, raw in enumerate(operations):
        if not isinstance(raw, dict):
            raise ParityError(f"operations[{index}] must be an object")
        operation_id = raw.get("id")
        if not isinstance(operation_id, str) or not operation_id:
            raise ParityError(f"operations[{index}].id must be a non-empty string")
        if operation_id in ids:
            raise ParityError(f"duplicate canonical operation id {operation_id}")
        ids.add(operation_id)
        state = raw.get("state")
        if state not in {"live", "reserved"}:
            raise ParityError(f"operation {operation_id} has invalid state {state!r}")
        members = _strings(raw.get("signed_members"), f"operation {operation_id} signed_members", nonempty=True)
        signed_members.extend(members)
        for binding in BINDINGS:
            located = _endpoint(raw, binding)
            prior = per_binding[binding].get(located)
            if prior is not None:
                raise ParityError(
                    f"duplicate {binding} locator/spelling {located} for {prior} and {operation_id}"
                )
            per_binding[binding][located] = operation_id
            cross_prior = cross_binding.get(located)
            if cross_prior is not None and cross_prior != operation_id:
                raise ParityError(
                    f"locator/spelling {located} maps across bindings to {cross_prior} and {operation_id}"
                )
            cross_binding[located] = operation_id

    if len(signed_members) != len(set(signed_members)):
        duplicates = sorted({member for member in signed_members if signed_members.count(member) > 1})
        raise ParityError(f"duplicate signed member(s): {', '.join(duplicates)}")
    got = set(signed_members)
    want = set(allowlist)
    if got != want:
        added = sorted(got - want)
        removed = sorted(want - got)
        details = []
        if added:
            details.append(f"ADDED {', '.join(added)}")
        if removed:
            details.append(f"REMOVED {', '.join(removed)}")
        raise ParityError("signed member flattening differs from allowlist: " + "; ".join(details))


def live_canonical_ids(companion: dict[str, Any]) -> set[str]:
    return {
        operation["id"]
        for operation in companion["operations"]
        if operation.get("state") == "live"
    }


def validate_observed(
    companion: dict[str, Any], binding: str, observed: set[str] | list[str]
) -> set[str]:
    """Require one binding's observed commands to equal its live mapping."""
    if binding not in BINDINGS:
        raise ParityError(f"binding must be one of {BINDINGS!r}, got {binding!r}")
    if not isinstance(observed, (set, list)) or not all(
        isinstance(item, str) and item for item in observed
    ):
        raise ParityError("observed surface must be a set/list of non-empty strings")
    observed_set = set(observed)
    expected: dict[str, str] = {}
    reserved: dict[str, str] = {}
    for operation in companion["operations"]:
        located = _endpoint(operation, binding)
        target = expected if operation["state"] == "live" else reserved
        target[located] = operation["id"]
    reserved_hits = sorted(observed_set & set(reserved))
    if reserved_hits:
        located = reserved_hits[0]
        raise ParityError(
            f"{binding} observed reserved operation {reserved[located]} at {located}"
        )
    extra = sorted(observed_set - set(expected))
    missing = sorted(set(expected) - observed_set)
    if extra or missing:
        details = []
        if extra:
            details.append(f"observed ungoverned command(s): {', '.join(extra)}")
        if missing:
            details.append(f"missing live command(s): {', '.join(missing)}")
        raise ParityError(f"{binding} " + "; ".join(details))
    return {expected[located] for located in observed_set}


def _load(path: Path) -> dict[str, Any]:
    try:
        with path.open(encoding="utf-8") as handle:
            value = json.load(handle)
    except (OSError, json.JSONDecodeError) as error:
        raise ParityError(f"cannot load {path}: {error}") from error
    if not isinstance(value, dict):
        raise ParityError(f"{path} must contain a JSON object")
    return value


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--signed", type=Path, default=Path("src/conformance/governed-surface-allowlist.json")
    )
    parser.add_argument(
        "--companion", type=Path, default=Path("src/conformance/governed-operation-parity.json")
    )
    parser.add_argument("--binding", choices=BINDINGS)
    parser.add_argument("--observed-json")
    args = parser.parse_args(argv)
    try:
        signed = _load(args.signed)
        companion = _load(args.companion)
        validate_contract(signed, companion)
        if args.binding is None:
            if args.observed_json is not None:
                raise ParityError("--observed-json requires --binding")
            print(
                "ok    sdk-surface-parity: companion preserves "
                f"{len(signed['allowlist'])} signed members / "
                f"{len(live_canonical_ids(companion))} live canonical operations"
            )
            return 0
        if args.observed_json is None:
            raise ParityError("--binding requires --observed-json")
        try:
            observed = json.loads(args.observed_json)
        except json.JSONDecodeError as error:
            raise ParityError(f"observed JSON is invalid: {error}") from error
        canonical = validate_observed(companion, args.binding, observed)
        expected = live_canonical_ids(companion)
        if canonical != expected:
            raise ParityError(
                f"{args.binding} canonical set differs: got {sorted(canonical)!r}, "
                f"expected {sorted(expected)!r}"
            )
        print(
            f"ok    sdk-surface-parity: {args.binding} "
            f"{len(canonical)}/{len(expected)} live canonical operations"
        )
        return 0
    except ParityError as error:
        print(f"FAIL  sdk-surface-parity: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
