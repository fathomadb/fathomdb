#!/usr/bin/env python3
"""Enforce the narrow 0.8.25 Slice 7 dependency correction."""

from __future__ import annotations

import argparse
import pathlib
import sys
import tomllib


def _version_tuple(value: str) -> tuple[int, ...]:
    core = value.split("+", 1)[0].split("-", 1)[0]
    return tuple(int(part) for part in core.split("."))


def validate_dependency_policy(root: pathlib.Path) -> list[str]:
    failures: list[str] = []
    manifest_path = root / "src/rust/crates/fathomdb-embedder/Cargo.toml"
    lock_path = root / "Cargo.lock"
    manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    lock = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    httpmock = manifest.get("dev-dependencies", {}).get("httpmock")
    if httpmock != "=0.8.3":
        failures.append(f"httpmock manifest pin must be =0.8.3, found {httpmock!r}")

    packages = lock.get("package", [])
    by_name: dict[str, list[str]] = {}
    for package in packages:
        by_name.setdefault(package["name"], []).append(package["version"])
    if by_name.get("httpmock") != ["0.8.3"]:
        failures.append(f"Cargo.lock must resolve only httpmock 0.8.3, found {by_name.get('httpmock', [])}")
    if "async-std" in by_name:
        failures.append(f"async-std must be absent, found {by_name['async-std']}")

    floors = {
        "anyhow": "1.0.103",
        "crossbeam-epoch": "0.9.20",
        "memmap2": "0.9.11",
    }
    for name, floor in floors.items():
        versions = by_name.get(name, [])
        if not versions or any(_version_tuple(version) < _version_tuple(floor) for version in versions):
            failures.append(f"{name} must resolve at >= {floor}, found {versions}")
    event_5 = [version for version in by_name.get("event-listener", []) if version.startswith("5.4.")]
    if event_5 and any(_version_tuple(version) < (5, 4, 2) for version in event_5):
        failures.append(f"event-listener 5.4.x must resolve at >= 5.4.2, found {event_5}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    args = parser.parse_args()
    failures = validate_dependency_policy(args.root.resolve())
    if failures:
        for failure in failures:
            print(f"FAIL dependency-policy: {failure}", file=sys.stderr)
        return 1
    print("ok    dependency-policy")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
