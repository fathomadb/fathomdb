#!/usr/bin/env python3
"""Validate the 0.8.23 Linux CUDA release-contract seam without building it.

The real CUDA build and smokes run only from the workflow-restricted release
runner group.  This checker makes the inputs to that run reviewable in ordinary
CPU CI: the feature forwarding, the Linux-only build entry points, and the
non-publishing preflight job must agree exactly.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 CI fallback
    import tomli as tomllib  # type: ignore[import-not-found]


ROOT = Path(os.environ.get("REPO_ROOT", Path(__file__).resolve().parents[1]))
WORKSPACE_MANIFEST = ROOT / "Cargo.toml"
LOCKFILE = ROOT / "Cargo.lock"
NAPI_MANIFEST = ROOT / "src/rust/crates/fathomdb-napi/Cargo.toml"
TS_PACKAGE = ROOT / "src/ts/package.json"
WORKFLOW = ROOT / ".github/workflows/release.yml"
CUDA_CONTRACT = ROOT / "scripts/release/cuda-artifact-contract.sh"
CUDA_NAPI_BUILD = ROOT / "scripts/release/build-napi-cuda.sh"
CUDA_PREFLIGHT = ROOT / "scripts/release/cuda-preflight.sh"
CUDA_MANYLINUX_DOCKERFILE = ROOT / "scripts/release/Dockerfile.cuda-manylinux"
CUDA_MANYLINUX_PROVISIONER = ROOT / "scripts/release/provision-cuda-manylinux.sh"
CUDA_IMAGE_ATTESTATION = ROOT / "scripts/release/cuda-image-attestation.sh"

NAPI_CUDA_FEATURE = ["default-embedder", "fathomdb-engine/embed-cuda"]
NAPI_CUDA_BUILD = "bash ../../scripts/release/build-napi-cuda.sh"
PYTHON_CUDA_FEATURES = "pyo3/extension-module,embed-cuda"
RUNNER_LABELS = ("self-hosted", "Linux", "X64", "gpu", "cuda-12")
CUDA_MANYLINUX_BASE_IMAGE = (
    "quay.io/pypa/manylinux_2_28_x86_64@sha256:"
    "aba9efd7dec389abd76506219e461014015b1c1cb95f2a36f27946128910dd07"
)
CUDA_MANYLINUX_IMAGE = "fathomdb-cuda-manylinux:12.6-manylinux_2_28"
CUDA_TOOLKIT_IMAGE = (
    "nvidia/cuda:12.6.3-devel-rockylinux8@sha256:"
    "83bc2b9fcf3ab1a4e324f81e962b58957370fa71f7ac61e3a24af399a0ba7595"
)
CUDA_CACHE_DIGESTS = {
    "config.json": "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750",
    "tokenizer.json": "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
    "model.safetensors": "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad",
}
CUDA_RUSTUP_INIT_URL = (
    "https://static.rust-lang.org/rustup/archive/1.29.0/x86_64-unknown-linux-gnu/rustup-init"
)
CUDA_RUSTUP_INIT_SHA256 = "4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10"
CUDA_RUSTUP_TOOLCHAIN = "1.95.0-x86_64-unknown-linux-gnu"
CUDA_MANYLINUX_GCC_TOOLSET = "gcc-toolset-13"
CUDA_MANYLINUX_GCC_VERSION = "13.3.1"
CUDA_MANYLINUX_CC = "/opt/rh/gcc-toolset-13/root/usr/bin/gcc"
CUDA_MANYLINUX_CXX = "/opt/rh/gcc-toolset-13/root/usr/bin/g++"
CUDA_MANYLINUX_CUDA_LIB64 = "/usr/local/cuda-12.6/lib64"
CUDA_MANYLINUX_GCC_LIB = "/opt/rh/gcc-toolset-13/root/usr/lib/gcc/x86_64-redhat-linux/13"
CUDA_MANYLINUX_GCC_RPM = "gcc-toolset-13-gcc-13.3.1-2.2.el8_10.x86_64"
CUDA_MANYLINUX_GXX_RPM = "gcc-toolset-13-gcc-c++-13.3.1-2.2.el8_10.x86_64"
CUDA_NAPI_HOST_GCC_VERSION = "13.3.0"
CUDA_NAPI_HOST_CC = "/usr/bin/gcc-13"
CUDA_NAPI_HOST_CXX = "/usr/bin/g++-13"
UPLOAD_ARTIFACT_ACTION = "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"
CANDLE_GIT_URL = "https://github.com/coreyt/candle-fathomdb.git"
CANDLE_GIT_REV = "5719d90e60edd14c4c1a3bf87952648131b2153a"
CANDLE_PACKAGES = (
    "candle-core-fathomdb",
    "candle-nn-fathomdb",
    "candle-transformers-fathomdb",
)


def fail(message: str) -> None:
    print(f"cuda-release-contract: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_toml(path: Path) -> dict[str, object]:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot read {path}: {error}")


def load_json(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read {path}: {error}")
    if not isinstance(value, dict):
        fail(f"{path} must contain a JSON object")
    return value


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        fail(f"cannot read {path}: {error}")


def workflow_job(name: str) -> str:
    try:
        text = WORKFLOW.read_text(encoding="utf-8")
    except OSError as error:
        fail(f"cannot read {WORKFLOW}: {error}")
    match = re.search(
        rf"^  {re.escape(name)}:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
        text,
        re.MULTILINE | re.DOTALL,
    )
    if match is None:
        fail(f"release workflow lacks {name!r} job")
    return match.group(0)


def require_fragment(block: str, fragment: str, label: str) -> None:
    if fragment not in block:
        fail(f"{label} is missing {fragment!r}")


def main() -> None:
    workspace_manifest = load_toml(WORKSPACE_MANIFEST)
    patches = workspace_manifest.get("patch")
    if not isinstance(patches, dict):
        fail("workspace Cargo.toml has no [patch.crates-io] Candle source pin")
    crates_io_patch = patches.get("crates-io")
    if not isinstance(crates_io_patch, dict):
        fail("workspace Cargo.toml has no [patch.crates-io] Candle source pin")
    for package_name in CANDLE_PACKAGES:
        pin = crates_io_patch.get(package_name)
        if not isinstance(pin, dict):
            fail(f"[patch.crates-io] lacks a source pin for {package_name!r}")
        if pin.get("git") != CANDLE_GIT_URL or pin.get("rev") != CANDLE_GIT_REV:
            fail(
                f"{package_name!r} must pin {CANDLE_GIT_URL}@{CANDLE_GIT_REV}; "
                f"got {pin!r}"
            )

    lockfile = load_toml(LOCKFILE)
    locked_packages = lockfile.get("package")
    if not isinstance(locked_packages, list):
        fail("Cargo.lock has no package entries")
    expected_source = f"git+{CANDLE_GIT_URL}?rev={CANDLE_GIT_REV}#{CANDLE_GIT_REV}"
    for package_name in CANDLE_PACKAGES:
        entries = [
            entry
            for entry in locked_packages
            if isinstance(entry, dict) and entry.get("name") == package_name
        ]
        if len(entries) != 1:
            fail(f"Cargo.lock must contain exactly one {package_name!r} package; got {len(entries)}")
        source = entries[0].get("source")
        if source != expected_source:
            fail(
                f"Cargo.lock must resolve {package_name!r} from {expected_source!r}; "
                f"got {source!r}"
            )

    manifest = load_toml(NAPI_MANIFEST)
    features = manifest.get("features")
    if not isinstance(features, dict):
        fail("fathomdb-napi Cargo.toml has no [features] table")
    if features.get("embed-cuda") != NAPI_CUDA_FEATURE:
        fail(
            "fathomdb-napi embed-cuda feature must forward exactly "
            f"{NAPI_CUDA_FEATURE!r}; got {features.get('embed-cuda')!r}"
        )

    package = load_json(TS_PACKAGE)
    scripts = package.get("scripts")
    if not isinstance(scripts, dict):
        fail("src/ts/package.json has no scripts object")
    if scripts.get("build:native:cuda") != NAPI_CUDA_BUILD:
        fail(
            "src/ts/package.json build:native:cuda must own the Linux CUDA N-API "
            f"arguments exactly; got {scripts.get('build:native:cuda')!r}"
        )

    contract = read_text(CUDA_CONTRACT)
    require_fragment(
        contract,
        "CUDA_NAPI_FEATURES='embed-cuda'",
        "CUDA artifact contract",
    )
    require_fragment(
        contract,
        "CUDA_PYTHON_FEATURES='pyo3/extension-module,embed-cuda'",
        "CUDA artifact contract",
    )
    require_fragment(contract, "CUDA_MANYLINUX='2_28'", "CUDA artifact contract")
    require_fragment(contract, "CUDA_COMPUTE_CAP='75'", "CUDA artifact contract")
    require_fragment(
        contract,
        "CUDA_NAPI_HOST_TOOLKIT_ROOT='/usr/local/cuda-12.6'",
        "CUDA artifact contract",
    )
    require_fragment(
        contract,
        "CUDA_NAPI_HOST_NVCC_VERSION='Cuda compilation tools, release 12.6, V12.6.68'",
        "CUDA artifact contract",
    )
    for fragment in (
        f"CUDA_NAPI_HOST_GCC_VERSION='{CUDA_NAPI_HOST_GCC_VERSION}'",
        f"CUDA_NAPI_HOST_CC='{CUDA_NAPI_HOST_CC}'",
        f"CUDA_NAPI_HOST_CXX='{CUDA_NAPI_HOST_CXX}'",
    ):
        require_fragment(contract, fragment, "CUDA artifact contract")
    rustup_init_download = (
        'curl --proto \'=https\' --tlsv1.2 --fail --silent --show-error '
        '--output /tmp/rustup-init "$RUSTUP_INIT_URL"'
    )
    for fragment in (
        f"CUDA_MANYLINUX_IMAGE='{CUDA_MANYLINUX_IMAGE}'",
        "CUDA_MANYLINUX_PYTHON=",
        "CUDA_MANYLINUX_PLATFORM='linux/amd64'",
        "CUDA_TOOLKIT_VERSION='12.6.3'",
        "CUDA_MANYLINUX_PYTHON_ABI='cp311-cp311'",
        f"CUDA_MANYLINUX_BASE_IMAGE='{CUDA_MANYLINUX_BASE_IMAGE}'",
        f"CUDA_TOOLKIT_IMAGE='{CUDA_TOOLKIT_IMAGE}'",
        "CUDA_RUST_VERSION='1.95.0'",
        f"CUDA_RUSTUP_TOOLCHAIN='{CUDA_RUSTUP_TOOLCHAIN}'",
        "CUDA_MATURIN_VERSION='1.14.1'",
        f"CUDA_MANYLINUX_GCC_TOOLSET='{CUDA_MANYLINUX_GCC_TOOLSET}'",
        f"CUDA_MANYLINUX_GCC_VERSION='{CUDA_MANYLINUX_GCC_VERSION}'",
        "CUDA_MANYLINUX_GCC_ROOT='/opt/rh/gcc-toolset-13/root/usr'",
        f"CUDA_MANYLINUX_CC='{CUDA_MANYLINUX_CC}'",
        f"CUDA_MANYLINUX_CXX='{CUDA_MANYLINUX_CXX}'",
        f"CUDA_MANYLINUX_CUDA_LIB64='{CUDA_MANYLINUX_CUDA_LIB64}'",
        f"CUDA_MANYLINUX_GCC_LIB='{CUDA_MANYLINUX_GCC_LIB}'",
        f"CUDA_MANYLINUX_GCC_RPM='{CUDA_MANYLINUX_GCC_RPM}'",
        f"CUDA_MANYLINUX_GXX_RPM='{CUDA_MANYLINUX_GXX_RPM}'",
        "CUDA_MANYLINUX_DOCKERFILE='scripts/release/Dockerfile.cuda-manylinux'",
        f"CUDA_RUSTUP_INIT_URL='{CUDA_RUSTUP_INIT_URL}'",
        f"CUDA_RUSTUP_INIT_SHA256='{CUDA_RUSTUP_INIT_SHA256}'",
        "CUDA_DRIVERLESS_PYTHON_IMAGE=",
        "CUDA_DRIVERLESS_NODE_IMAGE=",
        "CUDA_DEFAULT_EMBEDDER_HF_REPO=",
        "CUDA_DEFAULT_EMBEDDER_HF_REVISION=",
    ):
        require_fragment(contract, fragment, "CUDA artifact contract")
    for file_name, digest in CUDA_CACHE_DIGESTS.items():
        if file_name == "config.json":
            variable = "CUDA_DEFAULT_EMBEDDER_CONFIG_SHA256"
        elif file_name == "tokenizer.json":
            variable = "CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256"
        else:
            variable = "CUDA_DEFAULT_EMBEDDER_MODEL_SHA256"
        require_fragment(contract, f"{variable}='{digest}'", "CUDA artifact contract")

    dockerfile = read_text(CUDA_MANYLINUX_DOCKERFILE)
    require_fragment(dockerfile, f"FROM {CUDA_TOOLKIT_IMAGE} AS cuda", "CUDA manylinux Dockerfile")
    require_fragment(dockerfile, f"FROM {CUDA_MANYLINUX_BASE_IMAGE}", "CUDA manylinux Dockerfile")
    for fragment in (
        "ARG RUST_VERSION=1.95.0",
        "ARG MATURIN_VERSION=1.14.1",
        f"ARG CUDA_GCC_TOOLSET={CUDA_MANYLINUX_GCC_TOOLSET}",
        f"ARG CUDA_GCC_VERSION={CUDA_MANYLINUX_GCC_VERSION}",
        f"ARG CUDA_GCC_RPM={CUDA_MANYLINUX_GCC_RPM}",
        f"ARG CUDA_GXX_RPM={CUDA_MANYLINUX_GXX_RPM}",
        f"ARG RUSTUP_INIT_URL={CUDA_RUSTUP_INIT_URL}",
        f"ARG RUSTUP_INIT_SHA256={CUDA_RUSTUP_INIT_SHA256}",
        "COPY --from=cuda /usr/local/cuda-12.6 /usr/local/cuda-12.6",
        "test -x /opt/python/cp311-cp311/bin/python",
        'grep -F "rustc $RUST_VERSION"',
        'grep -F "maturin $MATURIN_VERSION"',
        "grep -F 'release 12.6'",
        "cargo install maturin --version \"$MATURIN_VERSION\" --locked",
        'dnf install -y "$CUDA_GCC_RPM" "$CUDA_GXX_RPM"',
        "CUDACXX=/usr/local/cuda-12.6/bin/nvcc",
        "CC=/opt/rh/gcc-toolset-13/root/usr/bin/gcc",
        "CXX=/opt/rh/gcc-toolset-13/root/usr/bin/g++",
        "CUDAHOSTCXX=/opt/rh/gcc-toolset-13/root/usr/bin/g++",
        "NVCC_CCBIN=/opt/rh/gcc-toolset-13/root/usr/bin/g++",
        "CARGO_HOME=/opt/fathomdb/cargo",
        "RUSTUP_HOME=/opt/fathomdb/rustup",
        "/opt/fathomdb/cargo/bin",
        'install -d -m 0755 "$CARGO_HOME" "$RUSTUP_HOME"',
        'chmod -R a+rX "$CARGO_HOME" "$RUSTUP_HOME"',
        '"$CC" --version | grep -F "$CUDA_GCC_VERSION"',
        '"$CXX" --version | grep -F "$CUDA_GCC_VERSION"',
        'test "$CUDAHOSTCXX" = "$CXX"',
        'test "$NVCC_CCBIN" = "$CXX"',
        "io.fathomdb.cuda.manylinux=2_28",
        "io.fathomdb.cuda.rust=$RUST_VERSION",
        "io.fathomdb.cuda.maturin=$MATURIN_VERSION",
        "io.fathomdb.cuda.compiler=$CUDA_GCC_TOOLSET",
        "io.fathomdb.cuda.compiler-version=$CUDA_GCC_VERSION",
        "io.fathomdb.cuda.compiler-rpm=$CUDA_GCC_RPM",
        "io.fathomdb.cuda.compiler-cxx-rpm=$CUDA_GXX_RPM",
        "io.fathomdb.cuda.manylinux-base=quay.io/pypa/manylinux_2_28_x86_64@sha256:",
        "io.fathomdb.cuda.toolkit-base=nvidia/cuda:12.6.3-devel-rockylinux8@sha256:",
        "io.fathomdb.cuda.rustup-init-sha256=$RUSTUP_INIT_SHA256",
        rustup_init_download,
        "sha256sum --check --status",
        '"$RUSTUP_INIT_SHA256" /tmp/rustup-init',
    ):
        require_fragment(dockerfile, fragment, "CUDA manylinux Dockerfile")
    if dockerfile.count(rustup_init_download) != 1 or dockerfile.count("curl --proto '=https'") != 1:
        fail("CUDA manylinux Dockerfile must have exactly one fixed, verified rustup-init download")

    image_attestation = read_text(CUDA_IMAGE_ATTESTATION)
    for fragment in (
        "assert_cuda_manylinux_image()",
        "io.fathomdb.cuda.manylinux-base=$CUDA_MANYLINUX_BASE_IMAGE",
        "io.fathomdb.cuda.toolkit-base=$CUDA_TOOLKIT_IMAGE",
        "io.fathomdb.cuda.toolkit=$CUDA_TOOLKIT_VERSION",
        "io.fathomdb.cuda.manylinux=$CUDA_MANYLINUX",
        "io.fathomdb.cuda.python=$CUDA_MANYLINUX_PYTHON_ABI",
        "io.fathomdb.cuda.rust=$CUDA_RUST_VERSION",
        "io.fathomdb.cuda.maturin=$CUDA_MATURIN_VERSION",
        "io.fathomdb.cuda.compiler=$CUDA_MANYLINUX_GCC_TOOLSET",
        "io.fathomdb.cuda.compiler-version=$CUDA_MANYLINUX_GCC_VERSION",
        "io.fathomdb.cuda.compiler-rpm=$CUDA_MANYLINUX_GCC_RPM",
        "io.fathomdb.cuda.compiler-cxx-rpm=$CUDA_MANYLINUX_GXX_RPM",
        "io.fathomdb.cuda.rustup-init-sha256=$CUDA_RUSTUP_INIT_SHA256",
    ):
        require_fragment(image_attestation, fragment, "CUDA image attestation")

    provisioner = read_text(CUDA_MANYLINUX_PROVISIONER)
    for fragment in (
        '. "$SCRIPT_DIR/cuda-artifact-contract.sh"',
        '. "$SCRIPT_DIR/cuda-image-attestation.sh"',
        "DEFAULT_EMBEDDER_SNAPSHOT=",
        "models--${CUDA_DEFAULT_EMBEDDER_HF_REPO//\\//--}/snapshots/$CUDA_DEFAULT_EMBEDDER_HF_REVISION",
        "https://huggingface.co/${CUDA_DEFAULT_EMBEDDER_HF_REPO}/resolve/${CUDA_DEFAULT_EMBEDDER_HF_REVISION}/${file_name}",
        "sha256sum --check --status",
        'docker build --platform "$CUDA_MANYLINUX_PLATFORM"',
        '--tag "$CUDA_MANYLINUX_IMAGE"',
        '--file "$REPO_ROOT/$CUDA_MANYLINUX_DOCKERFILE"',
        'docker run --rm --network none --platform "$CUDA_MANYLINUX_PLATFORM"',
        '-e CUDA_MANYLINUX_GCC_VERSION -e CUDA_MANYLINUX_CC -e CUDA_MANYLINUX_CXX',
        'test -x /opt/python/cp311-cp311/bin/python',
        "assert_cuda_manylinux_image",
        '--build-arg "CUDA_GCC_TOOLSET=$CUDA_MANYLINUX_GCC_TOOLSET"',
        '--build-arg "CUDA_GCC_VERSION=$CUDA_MANYLINUX_GCC_VERSION"',
        '--build-arg "CUDA_GCC_RPM=$CUDA_MANYLINUX_GCC_RPM"',
        '--build-arg "CUDA_GXX_RPM=$CUDA_MANYLINUX_GXX_RPM"',
    ):
        require_fragment(provisioner, fragment, "CUDA manylinux provisioner")
    if provisioner.count("sha256sum --check --status") != 1:
        fail("CUDA manylinux provisioner must verify the complete cache through one pinned manifest")
    for forbidden in ("docker pull", "docker image rm", "docker system prune", "--privileged"):
        if forbidden in provisioner:
            fail(f"CUDA manylinux provisioner must not contain {forbidden!r}")
    napi_build = read_text(CUDA_NAPI_BUILD)
    require_fragment(napi_build, '"$CUDA_NAPI_FEATURES"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export CUDA_PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export CUDACXX="$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export CC="$CUDA_NAPI_HOST_CC"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export CXX="$CUDA_NAPI_HOST_CXX"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export CUDAHOSTCXX="$CUDA_NAPI_HOST_CXX"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export NVCC_CCBIN="$CUDA_NAPI_HOST_CXX"', "CUDA N-API build wrapper")
    require_fragment(
        napi_build,
        'export LIBRARY_PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64${LIBRARY_PATH:+:$LIBRARY_PATH}"',
        "CUDA N-API build wrapper",
    )
    require_fragment(napi_build, '"$CUDA_NAPI_HOST_CC" --version | grep -F "$CUDA_NAPI_HOST_GCC_VERSION"', "CUDA N-API build wrapper")
    require_fragment(napi_build, '"$CUDA_NAPI_HOST_CXX" --version | grep -F "$CUDA_NAPI_HOST_GCC_VERSION"', "CUDA N-API build wrapper")
    require_fragment(napi_build, "export CUDA_COMPUTE_CAP", "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin:$PATH"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'grep -F "$CUDA_NAPI_HOST_NVCC_VERSION"', "CUDA N-API build wrapper")
    preflight = read_text(CUDA_PREFLIGHT)
    for fragment in (
        'CONTAINER_UID="$(id -u)"',
        'CONTAINER_GID="$(id -g)"',
        'CONTAINER_USER="$CONTAINER_UID:$CONTAINER_GID"',
    ):
        require_fragment(preflight, fragment, "CUDA preflight container ownership")
    for fragment in (
        '"$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin/nvcc" --version',
        'CC="$CUDA_NAPI_HOST_CC" CXX="$CUDA_NAPI_HOST_CXX"',
        'CUDAHOSTCXX="$CUDA_NAPI_HOST_CXX" NVCC_CCBIN="$CUDA_NAPI_HOST_CXX"',
        '"$SCRIPT_DIR/build-napi-cuda.sh"',
        'maturin build --release --out /witness/python-dist',
        '--features "$CUDA_PYTHON_FEATURES"',
        '--manylinux "$CUDA_MANYLINUX"',
        'readelf -d "$NAPI_BINARY"',
        'readelf -d "$PYTHON_EXTENSION"',
        'docker run --rm --network none',
        'CUDA_MANYLINUX_IMAGE',
        'maturin --version',
        'rustc --version',
        'grep -F "maturin $CUDA_MATURIN_VERSION"',
        'grep -F "rustc $CUDA_RUST_VERSION"',
        'test "$RUSTUP_TOOLCHAIN" = "$CUDA_RUSTUP_TOOLCHAIN"',
        'test ! -w /opt/fathomdb/rustup',
        'CUDACXX=/usr/local/cuda-12.6/bin/nvcc',
        'CUDA_PATH=/usr/local/cuda-12.6',
        '-e "CC=$CUDA_MANYLINUX_CC"',
        '-e "CXX=$CUDA_MANYLINUX_CXX"',
        '-e "CUDAHOSTCXX=$CUDA_MANYLINUX_CXX"',
        '-e "NVCC_CCBIN=$CUDA_MANYLINUX_CXX"',
        '-e "LIBRARY_PATH=$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB"',
        '-e "LD_LIBRARY_PATH=$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB"',
        '-e CUDA_MANYLINUX_GCC_VERSION -e CUDA_MANYLINUX_CC -e CUDA_MANYLINUX_CXX',
        'test "$CC" = "$CUDA_MANYLINUX_CC"',
        'test "$CXX" = "$CUDA_MANYLINUX_CXX"',
        'test "$CUDAHOSTCXX" = "$CUDA_MANYLINUX_CXX"',
        'test "$NVCC_CCBIN" = "$CUDA_MANYLINUX_CXX"',
        'test "$LIBRARY_PATH" = "$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB"',
        'test "$LD_LIBRARY_PATH" = "$CUDA_MANYLINUX_CUDA_LIB64:$CUDA_MANYLINUX_GCC_LIB"',
        '"$CC" --version | grep -F "$CUDA_MANYLINUX_GCC_VERSION"',
        '"$CXX" --version | grep -F "$CUDA_MANYLINUX_GCC_VERSION"',
        '--mount "type=bind,src=$REPO_ROOT,dst=/workspace,readonly"',
        'manylinux-build.txt',
        'CUDA_DRIVERLESS_PYTHON_IMAGE',
        'CUDA_DRIVERLESS_NODE_IMAGE',
        'DEFAULT_EMBEDDER_HF_HOME',
        'Engine.open(str(db_path), use_default_embedder=True)',
        'engine.embed("driverless Python CUDA-capable default-embedder proof")',
        '{ useDefaultEmbedder: true }',
        'await engine.embed("driverless N-API CUDA-capable default-embedder proof")',
        'npm install --offline --ignore-scripts --no-audit --no-fund',
        'test ! -e /dev/nvidiactl',
        'sha256sum --check --status',
        '. "$SCRIPT_DIR/cuda-image-attestation.sh"',
        'assert_cuda_manylinux_image',
        'auditwheel show "/witness/python-dist/$WHEEL_BASENAME"',
        "manylinux_2_28",
        "--query-compute-apps=pid --format=csv,noheader",
        "--gpus ",
        "FATHOMDB_EMBED_DEVICE=cuda:0",
        "installed Python CUDA artifact GPU proof",
        "installed N-API CUDA artifact GPU proof",
        "gpu-python-cuda-witness.txt",
        "gpu-node-cuda-witness.txt",
    ):
        require_fragment(preflight, fragment, "CUDA preflight")
    for forbidden in (
        '--mount "type=bind,src=$CUDA_NAPI_HOST_TOOLKIT_ROOT,dst=/opt/cuda,readonly"',
        '--mount "type=bind,src=$CUDA_TOOLKIT_ROOT,dst=/opt/cuda,readonly"',
        "CUDACXX=/opt/cuda/bin/nvcc",
        'ldd "$NAPI_BINARY" || true',
        'ldd "$PYTHON_EXTENSION" || true',
        'auditwheel show "$WHEEL"',
        '--mount "type=bind,src=$REPO_ROOT,dst=/workspace"',
    ):
        if forbidden in preflight:
            fail(f"CUDA preflight must not contain {forbidden!r}")
    workspace_build = re.search(
        r'docker run --rm \\\n(?P<options>.*?--mount "type=bind,src=\$REPO_ROOT,dst=/workspace,readonly".*?)\n'
        r'  "\$CUDA_MANYLINUX_IMAGE" \\\n  sh -ceu',
        preflight,
        re.DOTALL,
    )
    if workspace_build is None:
        fail("CUDA preflight must have exactly one read-only workspace CUDA wheel build")
    for fragment in (
        '--user "$CONTAINER_USER"',
        "-e HOME=/tmp",
        "-e CARGO_HOME=/tmp/fathomdb-cargo",
        "-e RUSTUP_HOME=/opt/fathomdb/rustup",
        "-e CUDA_RUSTUP_TOOLCHAIN",
        '-e "RUSTUP_TOOLCHAIN=$CUDA_RUSTUP_TOOLCHAIN"',
        "-e CARGO_TARGET_DIR=/tmp/fathomdb-cargo-target",
    ):
        require_fragment(workspace_build.group("options"), fragment, "CUDA workspace wheel build")
    for path in (
        CUDA_CONTRACT,
        CUDA_NAPI_BUILD,
        CUDA_PREFLIGHT,
        CUDA_MANYLINUX_DOCKERFILE,
        CUDA_MANYLINUX_PROVISIONER,
    ):
        if "--allow-unsupported-compiler" in read_text(path):
            fail(f"CUDA release tooling must not contain unsupported-compiler override: {path}")
    if "Engine.open(str(db_path), use_default_embedder=False)" in preflight:
        fail("CUDA preflight must not use a no-embedder Python smoke")
    if "{ useDefaultEmbedder: false }" in preflight:
        fail("CUDA preflight must not use a no-embedder N-API smoke")
    if preflight.count('docker run --rm --network none') != 3:
        fail("CUDA preflight must isolate both installed-artifact CPU smokes and the image-owned auditwheel report from the network")
    if preflight.count(
        '--mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly"'
    ) != 3:
        fail("CUDA preflight must mount the pinned local default-embedder mirror into both Python smokes and the driverless N-API smoke")
    if preflight.count("-e HF_HOME=/fathomdb-hf") != 3:
        fail("CUDA preflight must make its containerized CPU and GPU smokes load only from the mounted local mirror")
    require_fragment(
        preflight,
        'HF_HOME="$DEFAULT_EMBEDDER_HF_HOME" FATHOMDB_EMBED_DEVICE=cuda:0 exec node "$NODE_GPU_SMOKE"',
        "CUDA preflight N-API GPU smoke",
    )
    if preflight.count("sha256sum --check --status") != 1:
        fail("CUDA preflight must verify the complete pinned default-embedder cache manifest")
    if preflight.count("--query-compute-apps=pid --format=csv,noheader") != 2:
        fail("CUDA preflight must observe the spawned Python and N-API GPU smoke PIDs on CUDA:0")

    job = workflow_job("cuda-contract-preflight")
    require_fragment(
        job,
        "if: ${{ github.event_name == 'workflow_dispatch' && inputs.dry_run == true }}",
        "cuda-contract-preflight",
    )
    require_fragment(job, "runs-on: [self-hosted, Linux, X64, gpu, cuda-12]", "cuda-contract-preflight")
    for label in RUNNER_LABELS:
        require_fragment(job, label, "cuda-contract-preflight runner labels")
    require_fragment(job, "needs: verify-release", "cuda-contract-preflight")
    if not re.search(
        r"^    permissions:\n      contents: read\n",
        job,
        re.MULTILINE,
    ):
        fail("cuda-contract-preflight must declare job-level read-only contents permission")
    if "contents: write" in job or "id-token: write" in job:
        fail("cuda-contract-preflight must not inherit release publishing permissions")
    require_fragment(job, "bash scripts/release/cuda-preflight.sh", "cuda-contract-preflight")
    require_fragment(job, "${{ env.RELEASE_CHECKOUT_REF }}", "cuda-contract-preflight checkout")
    require_fragment(job, UPLOAD_ARTIFACT_ACTION, "cuda-contract-preflight witness upload")
    require_fragment(job, "name: cuda-preflight-witness", "cuda-contract-preflight witness upload")
    require_fragment(job, "${{ github.workspace }}/cuda-preflight-witness", "cuda-contract-preflight witness path")

    python_build = workflow_job("build-python")
    require_fragment(python_build, "args: --release --out dist --features pyo3/extension-module,default-embedder", "ordinary build-python")
    if PYTHON_CUDA_FEATURES in python_build:
        fail("ordinary cross-platform build-python must stay CPU-only; CUDA belongs to the restricted preflight")

    ordinary_napi_build = workflow_job("build-napi")
    require_fragment(ordinary_napi_build, "run: npm run build:native", "ordinary build-napi")
    if "build:native:cuda" in ordinary_napi_build:
        fail("ordinary cross-platform build-napi must stay CPU-only; CUDA belongs to the restricted preflight")

    print("cuda-release-contract: pass")


if __name__ == "__main__":
    main()
