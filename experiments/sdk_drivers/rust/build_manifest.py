"""Build the fixed Rust SDK driver and attest its source/dependency closure."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import subprocess
import tomllib


HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BINARY = "fathomdb-sdk-benchmark-driver"
FEATURES = {
    "default-embedder",
    "embed-cuda",
    "embed-metal",
    "default-reranker",
    "rerank-cuda",
    "rerank-metal",
}


def output(*args: str, cwd: pathlib.Path = ROOT) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def checked_patches() -> dict[str, object]:
    own = tomllib.loads((HERE / "Cargo.toml").read_text())["patch"]["crates-io"]
    root = tomllib.loads((ROOT / "Cargo.toml").read_text())["patch"]["crates-io"]
    if own.keys() != root.keys():
        raise ValueError("standalone patch names differ from repository patches")
    for name, patch in own.items():
        expected = root[name]
        if "path" in patch:
            actual_path = (HERE / patch["path"]).resolve()
            expected_path = (ROOT / expected["path"]).resolve()
            if actual_path != expected_path:
                raise ValueError(f"patch path differs: {name}")
        elif patch != expected:
            raise ValueError(f"patch source differs: {name}")
    return own


def cargo_profile_name(requested_profile: str) -> str:
    """Return Cargo's reported profile and output-directory name."""
    return "debug" if requested_profile == "dev" else "release"


def resolved_closure(metadata: dict[str, object], lockfile: dict[str, object]) -> list[dict[str, object]]:
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    locked = {
        (package["name"], package["version"], package.get("source")): package
        for package in lockfile["package"]
    }
    pending = [metadata["resolve"]["root"]]
    seen: set[str] = set()
    closure = []
    while pending:
        package_id = pending.pop()
        if package_id in seen:
            continue
        seen.add(package_id)
        package = packages[package_id]
        node = nodes[package_id]
        key = (package["name"], package["version"], package["source"])
        locked_package = locked.get(key)
        if locked_package is None:
            raise ValueError(f"resolved package absent from Cargo.lock: {package['name']}")
        checksum = locked_package.get("checksum")
        if package["source"] and package["source"].startswith("registry+"):
            if (
                not isinstance(checksum, str)
                or len(checksum) != 64
                or any(char not in "0123456789abcdef" for char in checksum)
            ):
                raise ValueError(f"missing or invalid registry checksum: {package['name']}")
        if package.get("checksum") is not None and package["checksum"] != checksum:
            raise ValueError(f"metadata/lockfile checksum differs: {package['name']}")
        closure.append(
            {
                "name": package["name"],
                "version": package["version"],
                "source": package["source"],
                "checksum": checksum,
                "features": sorted(node["features"]),
            }
        )
        pending.extend(dependency["pkg"] for dependency in node["deps"])
    return sorted(closure, key=lambda item: (item["name"], item["version"], str(item["source"])))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    parser.add_argument("--profile", choices=("dev", "release"), default="release")
    parser.add_argument("--features", default="")
    parser.add_argument("--target", default="")
    args = parser.parse_args()

    features = sorted(filter(None, args.features.split(",")))
    if any(feature not in FEATURES for feature in features) or len(features) != len(set(features)):
        parser.error("unknown or duplicate feature")
    if output("git", "status", "--porcelain", "--untracked-files=all"):
        parser.error("source checkout must be clean and committed")
    patches = checked_patches()

    common = ["--manifest-path", str(HERE / "Cargo.toml"), "--locked"]
    if features:
        common += ["--features", ",".join(features)]
    build = ["cargo", "build", *common]
    metadata_command = ["cargo", "metadata", "--format-version", "1", *common]
    if args.target:
        build += ["--target", args.target]
        metadata_command += ["--filter-platform", args.target]
    if args.profile == "release":
        build.append("--release")
    subprocess.run(build, cwd=ROOT, check=True)
    metadata = json.loads(output(*metadata_command))
    target_dir = pathlib.Path(metadata["target_directory"])
    binary_dir = target_dir / args.target if args.target else target_dir
    binary = binary_dir / cargo_profile_name(args.profile) / BINARY
    lockfile = tomllib.loads((HERE / "Cargo.lock").read_text())
    manifest = {
        "schema_version": "sdk.rust-build.v1",
        "source_revision": output("git", "rev-parse", "HEAD"),
        "sdk_version": next(
            package["version"] for package in metadata["packages"] if package["name"] == "fathomdb-sdk"
        ),
        "binary_sha256": sha256(binary),
        "cargo_lock_sha256": sha256(HERE / "Cargo.lock"),
        "rustc": output("rustc", "--version"),
        "cargo": output("cargo", "--version"),
        "target": args.target or output("rustc", "-vV").split("host: ", 1)[1].splitlines()[0],
        "profile": cargo_profile_name(args.profile),
        "features": features,
        "rustflags": os.environ.get("RUSTFLAGS", ""),
        "patches": patches,
        "resolved_closure": resolved_closure(metadata, lockfile),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")


if __name__ == "__main__":
    main()
