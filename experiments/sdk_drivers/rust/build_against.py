"""Build the Rust SDK driver against another FathomDB source checkout.

The driver crate's path dependency points at this checkout's ``fathomdb-sdk``.
To compare SDKs on another engine (for example a release slice head), this
copies the driver source into a scratch tree, points ``fathomdb-sdk`` and the
``[patch.crates-io]`` table at that checkout, builds it, and writes an
attestation with the engine commit and the binary digest.

    python3 experiments/sdk_drivers/rust/build_against.py --engine-checkout DIR --out DIR \\
        [--features default-embedder]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import tomllib

HERE = pathlib.Path(__file__).resolve().parent
BINARY = "fathomdb-sdk-benchmark-driver"


def _patch_table(checkout: pathlib.Path) -> str:
    patches = tomllib.loads((checkout / "Cargo.toml").read_text())["patch"]["crates-io"]
    lines = ["[patch.crates-io]"]
    for name, spec in patches.items():
        fields = []
        for key, value in spec.items():
            if key == "path":
                value = str((checkout / value).resolve())
            fields.append(f'{key} = "{value}"')
        lines.append(f"{name} = {{ {', '.join(fields)} }}")
    return "\n".join(lines) + "\n"


def stage(checkout: pathlib.Path, scratch: pathlib.Path) -> pathlib.Path:
    """Lay out ``scratch/experiments/sdk_drivers/rust`` beside a copy of the checkout's root manifest."""
    crate = scratch / "experiments/sdk_drivers/rust"
    if scratch.exists():
        shutil.rmtree(scratch)
    crate.mkdir(parents=True)
    for name in ("build.rs", "Cargo.lock"):
        shutil.copy2(HERE / name, crate / name)
    for name in ("src", "tests"):
        shutil.copytree(HERE / name, crate / name)
    # build.rs reads the SDK version from ../../../Cargo.toml.
    shutil.copy2(checkout / "Cargo.toml", scratch / "Cargo.toml")
    manifest = (HERE / "Cargo.toml").read_text()
    head, _, _ = manifest.partition("[patch.crates-io]")
    sdk = (checkout / "src/rust/crates/fathomdb-sdk").resolve()
    head = head.replace('path = "../../../src/rust/crates/fathomdb-sdk"', f'path = "{sdk}"')
    if str(sdk) not in head:
        raise ValueError("driver manifest no longer names the in-tree fathomdb-sdk path")
    (crate / "Cargo.toml").write_text(head + _patch_table(checkout))
    return crate


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine-checkout", type=pathlib.Path, required=True)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument("--features", default="")
    args = parser.parse_args()
    checkout = args.engine_checkout.resolve()
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=checkout, text=True).strip():
        raise SystemExit("engine checkout is not clean")
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=checkout, text=True).strip()
    out = args.out.resolve()
    crate = stage(checkout, out / "src")
    command = ["cargo", "build", "--release", "--manifest-path", str(crate / "Cargo.toml")]
    if args.features:
        command += ["--features", args.features]
    subprocess.run(command, check=True, env={**dict(__import__("os").environ),
                                             "CARGO_TARGET_DIR": str(out / "target")})
    binary = out / "target/release" / BINARY
    attestation = {
        "schema_version": "sdk.rust-build-against.v1",
        "engine_checkout": str(checkout),
        "engine_commit": commit,
        "features": [f for f in args.features.split(",") if f],
        "binary": str(binary),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "cargo_lock_sha256": hashlib.sha256((crate / "Cargo.lock").read_bytes()).hexdigest(),
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
    }
    (out / "rust-build-against.json").write_text(json.dumps(attestation, indent=2) + "\n")
    print(json.dumps(attestation))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
