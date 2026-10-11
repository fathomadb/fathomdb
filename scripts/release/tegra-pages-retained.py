#!/usr/bin/env python3
"""Read and validate the retained Tegra Pages artifact list.

Every Pages deployment replaces the whole site, so each deployer rebuilds
every artifact in `scripts/release/tegra-pages-retained.json` from its
published URL and pinned SHA-256. Removing an entry withdraws a published URL
and needs its own decision (ADR-0.8.27-jetson-tegra-node-addon-distribution,
decision 6).

  validate --manifest FILE   exit 0 iff the list is well formed
  list --manifest FILE       one tab-separated line per artifact:
                             kind, version, filename, sha256, url
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

SCHEMA = "fathomdb.tegra-pages-retained/v1"
PAGES_PACKAGES_URL = "https://fathomadb.github.io/fathomdb/tegra/packages/"
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
RELEASE = r"[0-9]+\.[0-9]+\.[0-9]+"
KINDS = {
    # kind: (version pattern, filename pattern built from the version)
    "wheel": (
        re.compile(rf"{RELEASE}\+tegra\Z"),
        lambda version: re.compile(rf"fathomdb-{re.escape(version)}-[A-Za-z0-9_.]+-[A-Za-z0-9_.]+-linux_aarch64\.whl\Z"),
    ),
    "node-tarball": (
        re.compile(rf"{RELEASE}\Z"),
        lambda version: re.compile(rf"fathomdb-linux-arm64-gnu-tegra-{re.escape(version)}\.tgz\Z"),
    ),
}
FIELDS = {"kind", "version", "filename", "sha256", "url"}


class InvalidManifest(Exception):
    pass


def load(path: Path) -> list[dict[str, Any]]:
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InvalidManifest(f"cannot read {path}: {error}") from error
    if not isinstance(manifest, dict) or manifest.get("schema") != SCHEMA:
        raise InvalidManifest(f"schema must be {SCHEMA}")
    if set(manifest) != {"schema", "artifacts"}:
        raise InvalidManifest("manifest must carry exactly schema and artifacts")
    artifacts = manifest["artifacts"]
    if not isinstance(artifacts, list):
        raise InvalidManifest("artifacts must be a list")
    seen: set[str] = set()
    for index, artifact in enumerate(artifacts):
        where = f"artifacts[{index}]"
        if not isinstance(artifact, dict) or set(artifact) != FIELDS:
            raise InvalidManifest(f"{where} must carry exactly {', '.join(sorted(FIELDS))}")
        if not all(isinstance(artifact[field], str) for field in FIELDS):
            raise InvalidManifest(f"{where} fields must be strings")
        kind = artifact["kind"]
        if kind not in KINDS:
            raise InvalidManifest(f"{where}: kind must be one of {', '.join(sorted(KINDS))}")
        version_pattern, filename_pattern = KINDS[kind]
        if not version_pattern.fullmatch(artifact["version"]):
            raise InvalidManifest(f"{where}: version {artifact['version']!r} is not a {kind} version")
        filename = artifact["filename"]
        if not filename_pattern(artifact["version"]).fullmatch(filename):
            raise InvalidManifest(f"{where}: filename {filename!r} does not match its {kind} version")
        if not SHA256.fullmatch(artifact["sha256"]):
            raise InvalidManifest(f"{where}: sha256 must be 64 lowercase hexadecimal characters")
        if artifact["url"] != PAGES_PACKAGES_URL + filename:
            raise InvalidManifest(f"{where}: url must be {PAGES_PACKAGES_URL}{filename}")
        if filename in seen:
            raise InvalidManifest(f"{where}: duplicate filename {filename}")
        seen.add(filename)
    return artifacts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=("validate", "list"))
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    try:
        artifacts = load(args.manifest)
    except InvalidManifest as error:
        print(f"tegra-pages-retained: {error}", file=sys.stderr)
        return 1
    if args.command == "list":
        for artifact in artifacts:
            print("\t".join(artifact[field] for field in ("kind", "version", "filename", "sha256", "url")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
