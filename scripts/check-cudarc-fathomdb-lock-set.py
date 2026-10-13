#!/usr/bin/env python3
"""The cudarc-fathomdb lock-set guard (0.8.28 Slice 31, design § 7.2, AC31-14).

The cudarc fork runs its hosted tests against FathomDB's Cargo.lock and
records the set of packages it compiled for FathomDB: per Linux target, the
normal-dependency closure of the one `cudarc-fathomdb` node (root included)
with the root's resolved features. This guard recomputes that set from
FathomDB's tree and fails when it differs from the copy of the fork's
record in scripts/cudarc-fathomdb-lock-set.json, so a lock change that moves
a crate the fork compiled cannot land without a new fork run.

Inputs: --root DIR (runs `cargo metadata --offline --locked
--filter-platform <target>` there) or --metadata FILE (a JSON object mapping
each target to pre-computed metadata, for hermetic tests); --lock FILE
(checksums; default ROOT/Cargo.lock); --record FILE.

Exit 0 on a match, 1 on drift or a bad record, 2 on a precondition error.
The number, version and source of cudarc-fathomdb lock entries are owned by
scripts/check-pinned-override-rot.py; this guard only requires exactly one
node to start from.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

try:
    import tomllib
except ImportError:  # Python 3.10 (Ubuntu 22.04, Jetson); tomli is in [dev].
    import tomli as tomllib  # type: ignore[no-redef]

SCHEMA = "fathomdb.cudarc-fathomdb-lock-set.v2"
TARGETS = ("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu")
FEATURES = "fathomdb-embedder/embed-cuda,fathomdb-embedder/rerank-cuda,fathomdb-embedder/tegra-pool"
ROOT_PACKAGE = "cudarc-fathomdb"
REFRESH = (
    "push the commit that carries this Cargo.lock to origin so raw.githubusercontent.com serves it; "
    "the cudarc fork must set FATHOMDB_LOCK_SHA to it, run its hosted test job and record the set "
    "(cudarc fork plan § 8, last row); then copy the record to scripts/cudarc-fathomdb-lock-set.json"
)


class Precondition(Exception):
    """The inputs cannot support a verdict."""


Member = dict[str, Any]


def lock_checksums(lock_path: Path) -> dict[tuple[str, str, str], str | None]:
    try:
        lock = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise Precondition(f"cannot read {lock_path}: {exc}") from exc
    sums: dict[tuple[str, str, str], str | None] = {}
    for package in lock.get("package", []):
        source = package.get("source")
        if source is None:
            continue
        sums[(package["name"], package["version"], source)] = package.get("checksum")
    return sums


def closure(metadata: dict[str, Any], sums: dict[tuple[str, str, str], str | None]) -> tuple[list[Member], list[str]]:
    packages = {package["id"]: package for package in metadata.get("packages", [])}
    nodes = {node["id"]: node for node in (metadata.get("resolve") or {}).get("nodes", [])}
    roots = [ident for ident in nodes if packages.get(ident, {}).get("name") == ROOT_PACKAGE]
    if len(roots) != 1:
        raise Precondition(
            f"expected exactly one {ROOT_PACKAGE} node in the resolved graph, found {len(roots)}; "
            "scripts/check-pinned-override-rot.py owns that fact"
        )
    root = roots[0]
    seen = {root}
    stack = [root]
    while stack:
        for dep in nodes[stack.pop()].get("deps", []):
            if not any(kind.get("kind") is None for kind in dep.get("dep_kinds", [])):
                continue
            if dep["pkg"] not in seen:
                seen.add(dep["pkg"])
                stack.append(dep["pkg"])

    def member(ident: str) -> Member:
        package = packages[ident]
        source = package.get("source")
        if source is None:
            raise Precondition(f"{package['name']} {package['version']} in the closure has no registry or git source")
        return {
            "name": package["name"],
            "version": package["version"],
            "source": source,
            "checksum": sums.get((package["name"], package["version"], source)),
        }

    others = sorted((member(ident) for ident in seen - {root}), key=lambda m: (m["name"], m["version"]))
    return [member(root), *others], sorted(nodes[root].get("features", []))


def canonical_text(targets: dict[str, list[Member]], root_features: list[str]) -> str:
    text = ""
    for target in TARGETS:
        text += f"# {target}\n"
        for m in targets[target]:
            text += f"{m['name']} {m['version']} {m['source']} {m['checksum']}\n"
    text += "# features " + ",".join(sorted(root_features)) + "\n"
    return text


def digest(targets: dict[str, list[Member]], root_features: list[str]) -> str:
    return hashlib.sha256(canonical_text(targets, root_features).encode("utf-8")).hexdigest()


def cargo_metadata(root: Path, target: str) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--offline", "--locked", "--format-version", "1",
         "--filter-platform", target, "--features", FEATURES],
        cwd=root, capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise Precondition(f"cargo metadata --filter-platform {target} failed: {result.stderr.strip()}")
    return json.loads(result.stdout)


def diff(target: str, recorded: list[Member], current: list[Member]) -> list[str]:
    before = {m["name"]: m for m in recorded}
    after = {m["name"]: m for m in current}
    lines = []
    for name in sorted(after.keys() - before.keys()):
        lines.append(f"{target}: added: {name} {after[name]['version']}")
    for name in sorted(before.keys() - after.keys()):
        lines.append(f"{target}: removed: {name} {before[name]['version']}")
    for name in sorted(before.keys() & after.keys()):
        old, new = before[name], after[name]
        if (old["version"], old["source"], old["checksum"]) != (new["version"], new["source"], new["checksum"]):
            lines.append(f"{target}: changed: {name} {old['version']} -> {new['version']}"
                         + ("" if old["version"] != new["version"] else " (source or checksum)"))
    return lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--root", type=Path)
    source.add_argument("--metadata", type=Path)
    parser.add_argument("--lock", type=Path)
    parser.add_argument("--record", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.root is not None:
            lock_path = args.lock or args.root / "Cargo.lock"
            metadata = {target: cargo_metadata(args.root, target) for target in TARGETS}
        else:
            if args.lock is None:
                raise Precondition("--metadata needs --lock for the package checksums")
            lock_path = args.lock
            metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
        sums = lock_checksums(lock_path)
        current: dict[str, list[Member]] = {}
        features: dict[str, list[str]] = {}
        for target in TARGETS:
            if target not in metadata:
                raise Precondition(f"no metadata for {target}")
            current[target], features[target] = closure(metadata[target], sums)
        if features[TARGETS[0]] != features[TARGETS[1]]:
            raise Precondition(f"the root features differ between targets: {features}")
        root_features = features[TARGETS[0]]
        if not args.record.is_file():
            print(f"FAIL  lock-set: no record at {args.record}; {REFRESH}", file=sys.stderr)
            return 1
        record = json.loads(args.record.read_text(encoding="utf-8"))
    except Precondition as exc:
        print(f"UNVERIFIED lock-set: {exc}", file=sys.stderr)
        return 2
    except (OSError, ValueError) as exc:
        print(f"UNVERIFIED lock-set: {exc}", file=sys.stderr)
        return 2

    failures: list[str] = []
    if record.get("schema") != SCHEMA or record.get("closure") != "linux-normal":
        failures.append(f"record schema must be {SCHEMA} with closure linux-normal")
    recorded_targets = record.get("targets", {})
    recorded_features = sorted(record.get("root_features", []))
    if set(recorded_targets) != set(TARGETS):
        failures.append(f"record targets must be exactly {', '.join(TARGETS)}")
    elif digest(recorded_targets, recorded_features) != record.get("set_sha256"):
        failures.append("record set_sha256 does not match its own contents (a hand-edited record)")
    if not failures and digest(current, root_features) != record["set_sha256"]:
        for target in TARGETS:
            failures.extend(diff(target, recorded_targets[target], current[target]))
        removed = sorted(set(recorded_features) - set(root_features))
        added = sorted(set(root_features) - set(recorded_features))
        if removed:
            failures.append("root features removed: " + ", ".join(removed))
        if added:
            failures.append("root features added: " + ", ".join(added))
        failures.append(REFRESH)
    if failures:
        for failure in failures:
            print(f"FAIL  lock-set: {failure}", file=sys.stderr)
        return 1
    count = len(current[TARGETS[0]])
    print(f"ok lock-set: the Linux-normal closure of {ROOT_PACKAGE} ({count} packages, root included) "
          f"matches the fork record {record['set_sha256'][:12]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
