#!/usr/bin/env python3
"""Offline gate for npm overrides that can outlive their security purpose."""

from __future__ import annotations

import argparse
from datetime import date
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit


class Unverified(Exception):
    """The checked-in evidence cannot support a trustworthy verdict."""


SEMVER = r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
VERSION = re.compile(rf"^{SEMVER}$")
PRERELEASE_VERSION = re.compile(rf"^{SEMVER}-[0-9A-Za-z.-]+$")
COMPARATOR = re.compile(r"^(<=|>=|<|>|=)?\s*(\d+\.\d+\.\d+)$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
GIT_REV = re.compile(r"^[0-9a-f]{40}$")
ISO_DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
GHSA_ID = re.compile(r"^GHSA-[23456789cfghjmpqrvwx]{4}-[23456789cfghjmpqrvwx]{4}-[23456789cfghjmpqrvwx]{4}$")
GITHUB_ADVISORY_SOURCE = "GitHub Advisory Database"
# This is deliberately source-owned rather than read from metadata: otherwise
# a forged snapshot can recompute the metadata checksum and self-authenticate.
# Refresh procedure: independently review the upstream GHSA records; write the
# reviewed snapshot; compute its SHA-256; update this constant and metadata's
# advisory_snapshot.sha256 together; then run the pin-rot fixture. Never derive
# this value from metadata or make it configurable at runtime: either mismatch
# is an UNVERIFIED hard failure.
PINNED_ADVISORY_SNAPSHOT_SHA256 = "0aee0fc7be3dceb63bcd5abcb4877eaac256a03ca9511b37448f235a1a3c1f97"
EXTERNAL_SOURCE_ADVISORY_STATUS = "external-source-unassessed"
EXTERNAL_SOURCE_ADVISORY_SCOPE = "outside the checked-in npm advisory snapshot"
CARGO_PACKAGE_NAME = r"[A-Za-z0-9][A-Za-z0-9_-]*"
BARE_REPLACE_PACKAGE_ID = re.compile(rf"^(?P<name>{CARGO_PACKAGE_NAME}):(?P<version>{SEMVER})$")
URL_REPLACE_PACKAGE_FRAGMENT = re.compile(rf"^(?P<name>{CARGO_PACKAGE_NAME})@(?P<version>{SEMVER})$")
LOCK_PACKAGE_UNSPECIFIED = object()


def read_json(path: Path, label: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise Unverified(f"cannot read {label} {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise Unverified(f"{label} {path} is malformed JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise Unverified(f"{label} {path} must be a JSON object")
    return value


def version_tuple(version: str) -> tuple[int, int, int]:
    if PRERELEASE_VERSION.fullmatch(version):
        raise Unverified(
            f"prerelease version {version!r} is unsupported by the stable-only semver comparator"
        )
    match = VERSION.fullmatch(version)
    if not match:
        raise Unverified(f"unsupported non-exact semver version {version!r}")
    return tuple(int(component) for component in version.split("."))  # type: ignore[return-value]


def satisfies(version: str, range_text: str) -> bool:
    """Evaluate the deliberately small comparator grammar used by the snapshot."""
    value = version_tuple(version)
    comparators = [item.strip() for item in range_text.split(",") if item.strip()]
    if not comparators:
        raise Unverified(f"empty version range for {version}")
    for item in comparators:
        match = COMPARATOR.fullmatch(item)
        if not match:
            raise Unverified(f"unsupported range {range_text!r}; use comma-separated exact semver comparators")
        op, required_text = match.groups()
        required = version_tuple(required_text)
        if op in (None, "=") and value != required:
            return False
        if op == ">=" and value < required:
            return False
        if op == ">" and value <= required:
            return False
        if op == "<=" and value > required:
            return False
        if op == "<" and value >= required:
            return False
    return True


def nonempty_string(value: Any, name: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise Unverified(f"{name} must be a non-empty string")
    return value


def advisory_date(value: Any, name: str) -> str:
    """Require an unambiguous calendar date for checked-in advisory evidence."""
    text = nonempty_string(value, name)
    if not ISO_DATE.fullmatch(text):
        raise Unverified(f"{name} must be an ISO-8601 calendar date")
    try:
        date.fromisoformat(text)
    except ValueError as exc:
        raise Unverified(f"{name} must be an ISO-8601 calendar date") from exc
    return text


def records_by_package(metadata: dict[str, Any]) -> dict[str, dict[str, Any]]:
    records = metadata.get("npm_overrides")
    if not isinstance(records, list):
        raise Unverified("metadata has no npm_overrides list")
    indexed: dict[str, dict[str, Any]] = {}
    for index, record in enumerate(records):
        if not isinstance(record, dict):
            raise Unverified(f"npm_overrides[{index}] must be an object")
        package = nonempty_string(record.get("package"), f"npm_overrides[{index}].package")
        if package in indexed:
            raise Unverified(f"npm_overrides records {package!r} more than once")
        indexed[package] = record
    return indexed


def cargo_records(metadata: dict[str, Any]) -> dict[tuple[str, str, str], dict[str, Any]]:
    """Index the explicit records required for every Cargo override source."""
    records = metadata.get("cargo_pins")
    if not isinstance(records, list):
        raise Unverified("metadata has no cargo_pins list")
    indexed: dict[tuple[str, str, str], dict[str, Any]] = {}
    for index, record in enumerate(records):
        if not isinstance(record, dict):
            raise Unverified(f"cargo_pins[{index}] must be an object")
        manifest = nonempty_string(record.get("manifest"), f"cargo_pins[{index}].manifest")
        mechanism = nonempty_string(record.get("mechanism"), f"cargo_pins[{index}].mechanism")
        package = nonempty_string(record.get("package"), f"cargo_pins[{index}].package")
        key = (manifest, mechanism, package)
        if key in indexed:
            raise Unverified(f"cargo_pins records {manifest}:{mechanism}.{package} more than once")
        indexed[key] = record
    return indexed


def advisory_snapshot_path(root: Path, metadata: dict[str, Any]) -> Path:
    snapshot = metadata.get("advisory_snapshot")
    if not isinstance(snapshot, dict):
        raise Unverified("metadata has no advisory_snapshot object")
    source = nonempty_string(snapshot.get("source"), "advisory_snapshot.source")
    if source != GITHUB_ADVISORY_SOURCE:
        raise Unverified(
            f"advisory_snapshot.source must be {GITHUB_ADVISORY_SOURCE!r}, got {source!r}"
        )
    advisory_date(snapshot.get("retrieved_at"), "advisory_snapshot.retrieved_at")
    for key in ("path", "provenance"):
        nonempty_string(snapshot.get(key), f"advisory_snapshot.{key}")
    expected_digest = nonempty_string(snapshot.get("sha256"), "advisory_snapshot.sha256")
    if not SHA256.fullmatch(expected_digest):
        raise Unverified("advisory_snapshot.sha256 must be a lowercase SHA-256 digest")
    path = root / snapshot["path"]
    try:
        actual_digest = hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError as exc:
        raise Unverified(f"cannot read advisory snapshot {path}: {exc}") from exc
    if actual_digest != expected_digest:
        raise Unverified(
            f"advisory snapshot sha256 {actual_digest} does not match governed digest {expected_digest}"
        )
    if expected_digest != PINNED_ADVISORY_SNAPSHOT_SHA256:
        raise Unverified(
            "advisory snapshot sha256 does not match independently pinned checker digest"
        )
    return path


def validate_advisories(snapshot: dict[str, Any]) -> list[dict[str, Any]]:
    if snapshot.get("schema_version") != 1:
        raise Unverified("advisory snapshot schema_version must be 1")
    source = snapshot.get("source")
    if not isinstance(source, dict):
        raise Unverified("advisory snapshot has no source object")
    source_name = nonempty_string(source.get("name"), "advisory snapshot source.name")
    if source_name != GITHUB_ADVISORY_SOURCE:
        raise Unverified(
            f"advisory snapshot source.name must be {GITHUB_ADVISORY_SOURCE!r}, got {source_name!r}"
        )
    advisory_date(source.get("retrieved_at"), "advisory snapshot source.retrieved_at")
    nonempty_string(source.get("provenance"), "advisory snapshot source.provenance")
    advisories = snapshot.get("advisories")
    if not isinstance(advisories, list) or not advisories:
        raise Unverified("advisory snapshot advisories must be a non-empty list")
    seen_ids: set[str] = set()
    validated: list[dict[str, Any]] = []
    for index, advisory in enumerate(advisories):
        if not isinstance(advisory, dict):
            raise Unverified(f"advisory snapshot advisories[{index}] must be an object")
        advisory_id = nonempty_string(advisory.get("id"), f"advisory snapshot advisories[{index}].id")
        if not GHSA_ID.fullmatch(advisory_id):
            raise Unverified(
                f"advisory snapshot advisories[{index}].id must be a canonical GitHub GHSA identifier"
            )
        if advisory_id in seen_ids:
            raise Unverified(f"advisory snapshot advisory {advisory_id!r} appears more than once")
        seen_ids.add(advisory_id)
        nonempty_string(advisory.get("package"), f"advisory snapshot advisories[{index}].package")
        nonempty_string(
            advisory.get("vulnerable_range"), f"advisory snapshot advisories[{index}].vulnerable_range"
        )
        url = nonempty_string(advisory.get("url"), f"advisory snapshot advisories[{index}].url")
        expected_url = f"https://github.com/advisories/{advisory_id}"
        if url != expected_url:
            raise Unverified(
                f"advisory snapshot advisories[{index}].url must exactly equal {expected_url!r}"
            )
        satisfies("0.0.0", advisory["vulnerable_range"])
        validated.append(advisory)
    return validated


def validate_metadata(metadata: dict[str, Any], advisories: list[dict[str, Any]]) -> None:
    if metadata.get("schema_version") != 3:
        raise Unverified("metadata schema_version must be 3")
    records = records_by_package(metadata)
    cargo_records(metadata)
    scope = metadata.get("scope")
    if not isinstance(scope, dict):
        raise Unverified("metadata has no scope object")
    for key in ("npm", "cargo", "governed_commit_pins"):
        nonempty_string(scope.get(key), f"scope.{key}")
    advisory_ids = {advisory["id"]: advisory for advisory in advisories}
    for package, record in records.items():
        mapped = record.get("advisory_ids")
        if not isinstance(mapped, list) or not mapped or not all(isinstance(item, str) and item for item in mapped):
            raise Unverified(f"npm override {package}.advisory_ids must be a non-empty string list")
        if len(set(mapped)) != len(mapped):
            raise Unverified(f"npm override {package}.advisory_ids names an advisory more than once")
        expected = {advisory["id"] for advisory in advisories if advisory["package"] == package}
        if set(mapped) != expected:
            raise Unverified(
                f"npm override {package}.advisory_ids must exactly map this package's snapshot advisories "
                f"(expected {sorted(expected)}, got {sorted(mapped)})"
            )
        if any(advisory_id not in advisory_ids for advisory_id in mapped):
            raise Unverified(f"npm override {package}.advisory_ids names an unknown snapshot advisory")


def advisory_index(advisories: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    indexed: dict[str, list[dict[str, Any]]] = {}
    for advisory in advisories:
        indexed.setdefault(advisory["package"], []).append(advisory)
    return indexed


def replace_lock_package(package_id: str) -> str | None:
    """Resolve only Cargo's supported replace-key forms to a lock package name."""
    bare = BARE_REPLACE_PACKAGE_ID.fullmatch(package_id)
    if bare:
        return bare.group("name")
    parsed = urlsplit(package_id)
    if (
        parsed.scheme != "https"
        or not parsed.netloc
        or not parsed.path
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
    ):
        return None
    fragment = URL_REPLACE_PACKAGE_FRAGMENT.fullmatch(parsed.fragment)
    return fragment.group("name") if fragment else None


def cargo_governed_pins(root: Path) -> list[dict[str, str | None]]:
    """Identify Cargo override sources without calling Cargo or the network."""
    try:
        import tomllib
    except ImportError as exc:  # pragma: no cover - supported CI Python has it
        raise Unverified("python3.11+ tomllib is required to inspect Cargo override scope") from exc
    found: list[dict[str, str | None]] = []
    for manifest in sorted(root.glob("**/Cargo.toml")):
        if any(part in {"target", ".git"} for part in manifest.parts):
            continue
        try:
            parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as exc:
            raise Unverified(f"cannot parse Cargo manifest {manifest}: {exc}") from exc
        relative = manifest.relative_to(root)
        def add_pin(
            mechanism: str,
            package: str,
            spec: Any,
            lock_package: str | None | object = LOCK_PACKAGE_UNSPECIFIED,
        ) -> None:
            if lock_package is LOCK_PACKAGE_UNSPECIFIED:
                resolved_lock_package: str | None = package
            elif isinstance(lock_package, str) or lock_package is None:
                resolved_lock_package = lock_package
            else:  # pragma: no cover - internal callers use only the declared forms
                raise AssertionError("invalid Cargo lock package sentinel")
            pin: dict[str, str | None] = {
                "manifest": str(relative),
                "mechanism": mechanism,
                "package": package,
                "lock_package": resolved_lock_package,
                "git": None,
                "rev": None,
            }
            if isinstance(spec, dict):
                renamed_package = spec.get("package")
                if renamed_package is not None:
                    pin["lock_package"] = renamed_package if isinstance(renamed_package, str) else None
                git = spec.get("git")
                rev = spec.get("rev")
                if isinstance(git, str):
                    pin["git"] = git
                if isinstance(rev, str):
                    pin["rev"] = rev
            found.append(pin)

        for section_name in ("patch", "replace"):
            section = parsed.get(section_name)
            if section is None:
                continue
            if not isinstance(section, dict):
                raise Unverified(f"Cargo {section_name} table {relative} must be an object")
            if section_name == "patch":
                for registry, entries in section.items():
                    if not isinstance(entries, dict):
                        raise Unverified(f"Cargo patch table {relative}:patch.{registry} must be an object")
                    for package, spec in entries.items():
                        add_pin(f"patch.{registry}", str(package), spec)
            else:
                for package, spec in section.items():
                    package_id = str(package)
                    add_pin("replace", package_id, spec, replace_lock_package(package_id))

        def find_git_dependencies(value: Any, path: list[str]) -> None:
            if not isinstance(value, dict):
                return
            for key, child in value.items():
                child_path = [*path, str(key)]
                if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
                    if not isinstance(child, dict):
                        raise Unverified(
                            f"Cargo dependency table {relative}:{'.'.join(child_path)} must be an object"
                        )
                    for name, spec in child.items():
                        if isinstance(spec, dict) and "git" in spec:
                            add_pin(".".join(child_path), str(name), spec)
                find_git_dependencies(child, child_path)

        find_git_dependencies(parsed, [])
    return found


def cargo_pin_label(pin: dict[str, str | None]) -> str:
    """Render the manifest location used in actionable Cargo-pin diagnostics."""
    return f"{pin['manifest']}:{pin['mechanism']}.{pin['package']}"


def cargo_lock_sources(lockfile_path: Path) -> dict[tuple[str, str], set[str]]:
    """Read Cargo's checked-in package-source provenance without resolving."""
    try:
        import tomllib
    except ImportError as exc:  # pragma: no cover - supported CI Python has it
        raise Unverified("python3.11+ tomllib is required to inspect Cargo.lock") from exc
    try:
        parsed = tomllib.loads(lockfile_path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise Unverified(f"cannot parse Cargo lockfile {lockfile_path}: {exc}") from exc
    packages = parsed.get("package")
    if not isinstance(packages, list):
        raise Unverified("Cargo.lock has no package list")
    sources: dict[tuple[str, str], set[str]] = {}
    for index, package in enumerate(packages):
        if not isinstance(package, dict):
            raise Unverified(f"Cargo.lock package[{index}] must be an object")
        name = nonempty_string(package.get("name"), f"Cargo.lock package[{index}].name")
        version = nonempty_string(package.get("version"), f"Cargo.lock package[{index}].version")
        source = package.get("source")
        if source is None:
            continue
        if not isinstance(source, str):
            raise Unverified(f"Cargo.lock package[{index}].source must be a string")
        sources.setdefault((name, version), set()).add(source)
    return sources


def validate_cargo_pins(root: Path, metadata: dict[str, Any]) -> list[str]:
    """Require exact manifest, metadata, and lock provenance for every Cargo pin."""
    records = cargo_records(metadata)
    pins = cargo_governed_pins(root)
    if not pins:
        return [
            f"metadata records Cargo pin {manifest}:{mechanism}.{package}, but no such Cargo pin exists"
            for manifest, mechanism, package in sorted(records)
        ]
    lock_sources = cargo_lock_sources(root / "Cargo.lock")
    failures: list[str] = []
    for pin in pins:
        label = cargo_pin_label(pin)
        key = (str(pin["manifest"]), str(pin["mechanism"]), str(pin["package"]))
        record = records.pop(key, None)
        if record is None:
            failures.append(f"Cargo pin {label} has no governed record")
            continue
        git = pin["git"]
        rev = pin["rev"]
        if not isinstance(git, str) or not isinstance(rev, str) or not GIT_REV.fullmatch(rev):
            failures.append(f"Cargo pin {label} must use an immutable 40-character Git revision")
            continue
        recorded_git = nonempty_string(record.get("git"), f"Cargo pin {label}.git")
        recorded_rev = nonempty_string(record.get("rev"), f"Cargo pin {label}.rev")
        if recorded_git != git:
            failures.append(f"Cargo pin {label} Git source disagrees with metadata")
        if recorded_rev != rev:
            failures.append(f"Cargo pin {label} revision disagrees with metadata")
        if not GIT_REV.fullmatch(recorded_rev):
            failures.append(f"Cargo pin {label} metadata revision must be an immutable 40-character Git revision")
        lock_package = pin["lock_package"]
        if not isinstance(lock_package, str) or not lock_package:
            failures.append(f"Cargo pin {label} must identify a resolved Cargo.lock package")
            continue
        recorded_lock_package = nonempty_string(record.get("lock_package"), f"Cargo pin {label}.lock_package")
        if recorded_lock_package != lock_package:
            failures.append(f"Cargo pin {label} resolved lock package disagrees with metadata")
        version = nonempty_string(record.get("version"), f"Cargo pin {label}.version")
        version_tuple(version)
        rationale = record.get("rationale")
        if not isinstance(rationale, str) or not rationale.strip():
            failures.append(f"Cargo pin {label} has no recorded rationale")
        posture = record.get("advisory_posture")
        if not isinstance(posture, dict):
            failures.append(f"Cargo pin {label} has no explicit advisory posture")
        else:
            if posture.get("status") != EXTERNAL_SOURCE_ADVISORY_STATUS:
                failures.append(f"Cargo pin {label} advisory posture must be {EXTERNAL_SOURCE_ADVISORY_STATUS}")
            if posture.get("scope") != EXTERNAL_SOURCE_ADVISORY_SCOPE:
                failures.append(f"Cargo pin {label} advisory posture must scope to the checked-in npm snapshot")
            posture_rationale = posture.get("rationale")
            if not isinstance(posture_rationale, str) or not posture_rationale.strip():
                failures.append(f"Cargo pin {label} advisory posture has no recorded rationale")
        expected_source = f"git+{git}?rev={rev}#{rev}"
        if expected_source not in lock_sources.get((lock_package, version), set()):
            failures.append(f"Cargo pin {label} has no matching Cargo.lock source")
    for manifest, mechanism, package in sorted(records):
        failures.append(
            f"metadata records Cargo pin {manifest}:{mechanism}.{package}, but no such Cargo pin exists"
        )
    return failures


def check(root: Path, manifest_path: Path, lockfile_path: Path, metadata_path: Path) -> list[str]:
    manifest = read_json(manifest_path, "npm manifest")
    read_json(lockfile_path, "npm lockfile")
    metadata = read_json(metadata_path, "pinned-override metadata")
    snapshot_path = advisory_snapshot_path(root, metadata)
    advisories = validate_advisories(read_json(snapshot_path, "advisory snapshot"))
    validate_metadata(metadata, advisories)
    indexed_advisories = advisory_index(advisories)

    overrides = manifest.get("overrides", {})
    if not isinstance(overrides, dict):
        raise Unverified("package.json overrides must be an object")
    records = records_by_package(metadata)
    failures: list[str] = []
    for package, override in overrides.items():
        if not isinstance(override, str):
            raise Unverified(f"npm override {package!r} is not an exact string version")
        version_tuple(override)
        record = records.pop(package, None)
        if record is None:
            failures.append(f"R3 npm override {package}@{override} has no recorded rationale")
            continue
        recorded_version = nonempty_string(record.get("version"), f"npm override {package}.version")
        if recorded_version != override:
            failures.append(f"npm override {package}@{override} disagrees with metadata version {recorded_version}")
        rationale = record.get("rationale")
        if not isinstance(rationale, str) or not rationale.strip():
            failures.append(f"R3 npm override {package}@{override} has no recorded rationale")
        for advisory in indexed_advisories.get(package, []):
            if satisfies(override, advisory["vulnerable_range"]):
                failures.append(
                    f"R1 npm override {package}@{override} is vulnerable to {advisory['id']} "
                    f"({advisory['vulnerable_range']})"
                )
    for extra in sorted(records):
        failures.append(f"metadata records npm override {extra!r}, but package.json has no such override")

    failures.extend(validate_cargo_pins(root, metadata))
    if not failures and overrides:
        packages = ", ".join(sorted(overrides))
        raise Unverified(
            "R2 cannot derive a no-override resolution from package.json and a lockfile generated with "
            f"overrides ({packages}); self-attested unpin_evidence is not accepted"
        )
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--lockfile", type=Path)
    parser.add_argument("--metadata", type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    manifest = args.manifest or root / "package.json"
    lockfile = args.lockfile or root / "package-lock.json"
    metadata = args.metadata or root / "scripts/pinned-override-rot.json"
    try:
        failures = check(root, manifest, lockfile, metadata)
    except Unverified as exc:
        print(f"UNVERIFIED pinned-override-rot: {exc}; refusing to report a clean result", file=sys.stderr)
        return 2
    if failures:
        for failure in failures:
            print(f"FAIL  pinned-override-rot: {failure}", file=sys.stderr)
        return 1
    print(
        "ok pinned-override-rot: every governed npm override and Cargo Git source has exact offline provenance"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
