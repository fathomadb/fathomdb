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


class Unverified(Exception):
    """The checked-in evidence cannot support a trustworthy verdict."""


def _import_tomllib(context: str):
    """Import a TOML parser, preferring stdlib tomllib (3.11+) with a tomli
    fallback for older interpreters (0.8.23 Slice 80.3 — Ubuntu 22.04 and
    every Jetson ship Python 3.10). Raises Unverified, never silently
    degrading, when neither is available."""
    try:
        import tomllib

        return tomllib
    except ImportError:
        pass
    try:
        import tomli

        return tomli
    except ImportError as exc:
        raise Unverified(
            f"python3.11+ tomllib (or a tomli fallback) is required to {context}"
        ) from exc


VERSION = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")
PRERELEASE_VERSION = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-[0-9A-Za-z.-]+$")
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
CANDLE_MANIFEST = "Cargo.toml"
CANDLE_MECHANISM = "patch.crates-io"
CANDLE_PACKAGES = (
    "candle-core-fathomdb",
    "candle-nn-fathomdb",
    "candle-transformers-fathomdb",
)
CANDLE_GIT = "https://github.com/coreyt/candle-fathomdb.git"
CANDLE_REV = "5719d90e60edd14c4c1a3bf87952648131b2153a"
CANDLE_VERSION = "0.10.2"


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


def validate_candle_exception(metadata: dict[str, Any]) -> None:
    """Authenticate the one Cargo exception this guard deliberately supports."""
    exception = metadata.get("cargo_candle_exception")
    if not isinstance(exception, dict):
        raise Unverified("metadata has no cargo_candle_exception object")
    expected = {
        "manifest": CANDLE_MANIFEST,
        "mechanism": CANDLE_MECHANISM,
        "git": CANDLE_GIT,
        "rev": CANDLE_REV,
        "version": CANDLE_VERSION,
    }
    for key, value in expected.items():
        if exception.get(key) != value:
            raise Unverified(f"cargo_candle_exception.{key} must equal the checker-owned Candle exception")
    packages = exception.get("packages")
    if packages != list(CANDLE_PACKAGES):
        raise Unverified("cargo_candle_exception.packages must equal the checker-owned Candle cohort")
    rationale = exception.get("rationale")
    if not isinstance(rationale, str) or not rationale.strip():
        raise Unverified("cargo_candle_exception.rationale must be a non-empty string")
    posture = exception.get("advisory_posture")
    if not isinstance(posture, dict):
        raise Unverified("cargo_candle_exception.advisory_posture must be an object")
    if posture.get("status") != EXTERNAL_SOURCE_ADVISORY_STATUS:
        raise Unverified(
            f"cargo_candle_exception.advisory_posture.status must be {EXTERNAL_SOURCE_ADVISORY_STATUS}"
        )
    if posture.get("scope") != EXTERNAL_SOURCE_ADVISORY_SCOPE:
        raise Unverified("cargo_candle_exception.advisory_posture.scope must name the npm advisory snapshot")
    posture_rationale = posture.get("rationale")
    if not isinstance(posture_rationale, str) or not posture_rationale.strip():
        raise Unverified("cargo_candle_exception.advisory_posture.rationale must be a non-empty string")


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
    if metadata.get("schema_version") != 4:
        raise Unverified("metadata schema_version must be 4")
    records = records_by_package(metadata)
    validate_candle_exception(metadata)
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


def cargo_config_failures(root: Path) -> list[str]:
    """Reject repository Cargo config patches without claiming to model their keys."""
    tomllib = _import_tomllib("inspect Cargo config")
    failures: list[str] = []
    for relative in (Path(".cargo/config.toml"), Path(".cargo/config")):
        path = root / relative
        if not path.is_file():
            continue
        try:
            parsed = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError):
            failures.append(f"unparseable Cargo config {relative}")
            continue
        if parsed.get("patch") is not None:
            failures.append(f"ungoverned Cargo config patch {relative}")
    return failures


def cargo_governed_pins(root: Path) -> list[dict[str, str | None]]:
    """Identify every Cargo override source so unsupported forms cannot hide."""
    tomllib = _import_tomllib("inspect Cargo override scope")
    found: list[dict[str, str | None]] = []
    for manifest in sorted(root.glob("**/Cargo.toml")):
        if any(part in {"target", ".git"} for part in manifest.parts):
            continue
        try:
            parsed = tomllib.loads(manifest.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as exc:
            raise Unverified(f"cannot parse Cargo manifest {manifest}: {exc}") from exc
        relative = manifest.relative_to(root)
        def add_pin(mechanism: str, package: str, spec: Any) -> None:
            pin: dict[str, str | None] = {
                "manifest": str(relative),
                "mechanism": mechanism,
                "package": package,
                "git": None,
                "rev": None,
            }
            if isinstance(spec, dict):
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
                    add_pin("replace", str(package), spec)

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
    tomllib = _import_tomllib("inspect Cargo.lock")
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


def validate_cargo_pins(root: Path) -> list[str]:
    """Allow exactly FathomDB's root Candle patch cohort and nothing else."""
    pins = cargo_governed_pins(root)
    failures = cargo_config_failures(root)
    expected = {(CANDLE_MANIFEST, CANDLE_MECHANISM, package) for package in CANDLE_PACKAGES}
    found: set[tuple[str, str, str]] = set()
    for pin in pins:
        label = cargo_pin_label(pin)
        key = (str(pin["manifest"]), str(pin["mechanism"]), str(pin["package"]))
        if key not in expected:
            failures.append(f"unsupported Cargo override/git source {label}")
            continue
        if key in found:
            failures.append(f"duplicate approved Candle patch {pin['package']}")
            continue
        found.add(key)
        git = pin["git"]
        rev = pin["rev"]
        if git != CANDLE_GIT:
            failures.append(f"Candle patch {pin['package']} Git source is not the approved source")
        if rev != CANDLE_REV:
            failures.append(f"Candle patch {pin['package']} revision is not the approved immutable revision")
    for manifest, mechanism, package in sorted(expected - found):
        failures.append(f"missing approved Candle patch {package}")
    lock_sources = cargo_lock_sources(root / "Cargo.lock") if found else {}
    expected_source = f"git+{CANDLE_GIT}?rev={CANDLE_REV}#{CANDLE_REV}"
    for package in CANDLE_PACKAGES:
        key = (CANDLE_MANIFEST, CANDLE_MECHANISM, package)
        if key in found and expected_source not in lock_sources.get((package, CANDLE_VERSION), set()):
            failures.append(f"Candle patch {package} has no matching Cargo.lock source")
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

    failures.extend(validate_cargo_pins(root))
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
        "ok pinned-override-rot: governed npm overrides and the approved Candle patch cohort have exact provenance"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
