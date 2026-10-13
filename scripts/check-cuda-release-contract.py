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
import subprocess
import sys
from datetime import datetime, timezone
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
PYTHON_PYPROJECT = ROOT / "src/python/pyproject.toml"
WORKFLOW = ROOT / ".github/workflows/release.yml"
CUDA_CONTRACT = ROOT / "scripts/release/cuda-artifact-contract.sh"
CUDA_NAPI_BUILD = ROOT / "scripts/release/build-napi-cuda.sh"
CUDA_TEGRA_PYTHON_BUILD = ROOT / "scripts/release/build-python-cuda-tegra.sh"
CUDA_TEGRA_NAPI_BUILD = ROOT / "scripts/release/build-napi-cuda-tegra.sh"
TEGRA_NAPI_PACKAGE_JSON = ROOT / "src/ts/npm-tegra/linux-arm64-gnu-tegra/package.json"
GENERIC_ARM64_PACKAGE_JSON = ROOT / "src/ts/npm/linux-arm64-gnu/package.json"
CUDA_PREFLIGHT = ROOT / "scripts/release/cuda-preflight.sh"
CUDA_ARTIFACT_INSPECTOR = ROOT / "scripts/release/inspect-cuda-artifacts.py"
CUDA_MANYLINUX_DOCKERFILE = ROOT / "scripts/release/Dockerfile.cuda-manylinux"
CUDA_MANYLINUX_PROVISIONER = ROOT / "scripts/release/provision-cuda-manylinux.sh"
CUDA_IMAGE_ATTESTATION = ROOT / "scripts/release/cuda-image-attestation.sh"
CUDA_PREFLIGHT_WITNESS_SCHEMA = ROOT / "scripts/release/cuda-preflight-witness.schema.json"
CUDA_PREFLIGHT_WITNESS_VERIFIER = ROOT / "scripts/release/verify-cuda-preflight-witness.py"
UNMERGED_CANDIDATE_MANIFEST = ROOT / "dev/release/cuda-unmerged-candidates.json"
UNMERGED_CANDIDATE_SCHEMA = ROOT / "dev/release/cuda-unmerged-candidates.schema.json"
UNMERGED_PROTECTION_BASELINE = ROOT / "dev/release/cuda-protection-baseline.json"
UNMERGED_RECEIPT_SCHEMA = ROOT / "scripts/release/cuda-unmerged-route-receipt.schema.json"
UNMERGED_CANDIDATE_VERIFIER = ROOT / "scripts/release/verify-cuda-unmerged-candidate.py"
UNMERGED_RECEIPT_VERIFIER = ROOT / "scripts/release/verify-cuda-unmerged-receipt.py"
PACKAGE_REHEARSAL_SCHEMA = ROOT / "scripts/release/cuda-package-rehearsal.schema.json"
PACKAGE_REHEARSAL_VERIFIER = ROOT / "scripts/release/verify-cuda-package-rehearsal.py"
PACKAGE_REHEARSAL_HELPER = ROOT / "scripts/release/cuda-package-rehearsal.sh"
PACKAGE_REHEARSAL_SMOKE = ROOT / "scripts/release/cuda-package-rehearsal-smoke.sh"
CUDA_GPU_SELECTION = ROOT / "scripts/lib/cuda-gpu-selection.sh"

NAPI_CUDA_FEATURE = ["default-embedder", "fathomdb-engine/embed-cuda"]
NAPI_CUDA_BUILD = "bash ../../scripts/release/build-napi-cuda.sh"
PYTHON_CUDA_FEATURES = "pyo3/extension-module,embed-cuda"
PYTHON_CUDA_FEATURES_TEGRA = "pyo3/extension-module,embed-cuda,rerank-cuda,tegra-pool"
TEGRA_NAPI_FEATURES = "embed-cuda,rerank-cuda,tegra-pool"
TEGRA_POOL_FEATURE = "tegra-pool"
# References to either approved Tegra-only feature set.
TEGRA_FEATURE_SET_REF = re.compile(r"CUDA_(?:\w*FEATURES_TEGRA|TEGRA_NAPI_FEATURES)\b")
CUDA_RERANK_NAPI_FEATURES = "embed-cuda,rerank-cuda"
CUDA_TEGRA_NAPI_NODE_VERSION = "25.9.0"
CUDA_TEGRA_NAPI_NPM_VERSION = "11.12.1"
TEGRA_NAPI_PACKAGE_NAME = "fathomdb-linux-arm64-gnu-tegra"
TEGRA_NAPI_TRIPLE = "linux-arm64-gnu"
RUNNER_LABELS = ("self-hosted", "Linux", "X64", "gpu", "cuda-12")
CUDA_GPU_UUID_ENV = "env:\n      FATHOMDB_CUDA_GPU_UUID: ${{ vars.FATHOMDB_CUDA_GPU_UUID }}"
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
# 0.8.23 Slice 80.6 (D-80.6-4, AC80-8): host compiler is a per-target axis.
# The x86_64 literals above are unchanged; the Jetson Orin carries stock
# Ubuntu 22.04 gcc 11.4.0 and has no gcc-13, which CUDA 12.6 supports.
CUDA_HOST_GCC_VERSION_TEGRA_ORIN = "11.4.0"
CUDA_HOST_CC_TEGRA_ORIN = "/usr/bin/gcc"
CUDA_HOST_CXX_TEGRA_ORIN = "/usr/bin/g++"
CUDA_X64_HOST_TOOLKIT_ROOT = "/usr/local/cuda-12.6"
CUDA_X64_HOST_CUDART_LIBRARY_DIR = (
    "/usr/local/cuda-12.6/targets/x86_64-linux/lib"
)


def cudarc_cuda_version(toolkit_root: str) -> str:
    """The CUDARC_CUDA_VERSION value for a `/usr/local/cuda-<major>.<minor>` root.

    cudarc reads it as major * 1000 + minor * 10 (12.6 -> "12060"). Deriving
    it from the toolkit root keeps one literal for the release's CUDA version.
    """
    match = re.fullmatch(r"/usr/local/cuda-(\d+)\.(\d+)", toolkit_root)
    if match is None:
        raise ValueError(f"unexpected CUDA toolkit root {toolkit_root!r}")
    return str(int(match.group(1)) * 1000 + int(match.group(2)) * 10)


# Measured on the Jetson Orin AGX (design § 2.7): lib64 is a symlink to the
# aarch64-linux target lib dir, and the driver library lives in an
# L4T-specific `nvidia/` subdirectory rather than at the bare multiarch path.
CUDA_TEGRA_HOST_CUDART_LIB = "/usr/local/cuda-12.6/targets/aarch64-linux/lib"
CUDA_TEGRA_HOST_DRIVER_LIB = "/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1"
UPLOAD_ARTIFACT_ACTION = "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"
UPLOAD_RETRY_ACTION = ROOT / ".github/actions/upload-artifact-retry/action.yml"
# 0.8.28 Slice 31 (REQ31-16, OP-1): the only Candle source this contract
# governs is the candle-kernels git patch that carries the statically linked
# CUDA runtime into the artifacts. Its source and rev are read from the
# pinned-override record (one literal); the registry Candle fork crates are
# the pinned-override checker's.
PINNED_OVERRIDE_RECORD = ROOT / "scripts/pinned-override-rot.json"
CANDLE_KERNELS_PACKAGE = "candle-kernels"
CANDLE_FORK_PACKAGES = (
    "candle-core-fathomdb",
    "candle-nn-fathomdb",
    "candle-transformers-fathomdb",
)
DRIVERLESS_DEVICE_SELECTION_VARIABLES = (
    "FATHOMDB_EMBED_DEVICE",
    "FATHOMDB_RERANK_DEVICE",
    "CUDA_VISIBLE_DEVICES",
    "NVIDIA_VISIBLE_DEVICES",
    "HIP_VISIBLE_DEVICES",
    "ROCR_VISIBLE_DEVICES",
)
PUBLISHING_JOBS = (
    "publish-rust-t1-embedder-api", "publish-rust-t2-schema", "publish-rust-t3-query",
    "publish-rust-t4-embedder", "publish-rust-t5-engine", "publish-rust-t6-facade",
    "publish-rust-t7-cli", "publish-pypi", "publish-npm-platform-linux-x64-gnu",
    "publish-npm-platform-linux-arm64-gnu", "publish-npm-platform-darwin-x64",
    "publish-npm-platform-darwin-arm64", "publish-npm-platform-win32-x64-msvc", "publish-npm",
    "post-publish-smoke", "post-publish-smoke-aarch64", "post-publish-smoke-darwin-x64",
    "post-publish-smoke-darwin-arm64", "post-publish-smoke-win32-x64", "co-tagging-assert",
    "promote-npm-latest", "github-release", "record-v0820-partial-registry-recovery",
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


def require_upload_retry_action() -> None:
    action = read_text(UPLOAD_RETRY_ACTION)
    if action.count(f"uses: {UPLOAD_ARTIFACT_ACTION}") != 3:
        fail("artifact retry action must pin all three attempts to the reviewed full action SHA")


def forbid_fragment(block: str, fragment: str, label: str, why: str) -> None:
    if fragment in block:
        fail(f"{label} must not contain {fragment!r}: {why}")


def require_x64_cuda_toolkit_environment(block: str, label: str) -> None:
    """Require host search paths for the statically linked Candle CUDART."""
    require_fragment(
        block,
        f"CUDA_PATH: {CUDA_X64_HOST_TOOLKIT_ROOT}",
        f"{label} CUDA toolkit environment",
    )
    require_fragment(
        block,
        f"LIBRARY_PATH: {CUDA_X64_HOST_CUDART_LIBRARY_DIR}",
        f"{label} CUDART library environment",
    )
    # 0.8.28 Slice 31 (D-8W): the binding version is selected only by this
    # variable (or nvcc), never by a concrete cuda-NNNNN manifest feature.
    require_fragment(
        block,
        f'CUDARC_CUDA_VERSION: "{cudarc_cuda_version(CUDA_X64_HOST_TOOLKIT_ROOT)}"',
        f"{label} CUDA binding version",
    )


def require_unmerged_candidate_control_plane() -> None:
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    result = subprocess.run(
        [
            sys.executable,
            str(UNMERGED_CANDIDATE_VERIFIER),
            "validate-manifest",
            "--manifest",
            str(UNMERGED_CANDIDATE_MANIFEST),
            "--now",
            now,
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown failure"
        fail(f"unmerged CUDA candidate manifest must be canonical and contain zero or one valid record: {detail}")
    candidate_schema = load_json(UNMERGED_CANDIDATE_SCHEMA)
    candidates = candidate_schema.get("properties", {}).get("candidates")
    if not isinstance(candidates, dict) or candidates.get("maxItems") != 1:
        fail("unmerged CUDA candidate schema must permit at most one candidate record")
    for path, marker in (
        (UNMERGED_CANDIDATE_SCHEMA, "fathomdb.cuda-unmerged-candidate/v1"),
        (UNMERGED_RECEIPT_SCHEMA, "fathomdb.cuda-unmerged-route-receipt/v1"),
        (UNMERGED_CANDIDATE_VERIFIER, "def validate_manifest_entries"),
        (UNMERGED_RECEIPT_VERIFIER, "receipt manifest SHA does not match the main-owned manifest"),
    ):
        require_fragment(read_text(path), marker, str(path))
    baseline = load_json(UNMERGED_PROTECTION_BASELINE)
    if set(baseline) != {
        "schema_version", "expires_at", "main_protection_sha256", "runner_group_sha256", "environment_sha256"
    } or baseline.get("schema_version") != "fathomdb.cuda-protection-baseline/v1":
        fail("unmerged CUDA protection baseline has an unsupported shape")


def require_unmerged_workflow_route() -> None:
    trusted_route = workflow_job("verify-cuda-trusted-route")
    for fragment in (
        "runs-on: ubuntu-latest",
        "permissions:\n      contents: read",
        "ref: ${{ github.workflow_sha }}",
        "fetch-depth: 1",
        "persist-credentials: false",
        "scripts/release/verify-cuda-unmerged-candidate.py authorize",
        "dev/release/cuda-unmerged-candidates.json",
        "dev/release/cuda-protection-baseline.json",
        "${{ github.workflow_ref }}",
        "${{ github.workflow_sha }}",
        "${{ github.run_id }}",
        "${{ github.run_attempt }}",
        "uses: ./.github/actions/upload-artifact-retry",
        "cuda-unmerged-route-receipt-${{ github.run_id }}-${{ github.run_attempt }}",
    ):
        require_fragment(trusted_route, fragment, "GitHub-hosted unmerged CUDA route")
    for forbidden in (
        "ref: ${{ inputs.candidate_commit }}", "RELEASE_CHECKOUT_REF", "npm ", "cargo ", "pip ",
        "uses: ./candidate", "uses: actions/upload-artifact@",
        "run: ./candidate", "run: bash candidate", "uses: owner/action@ref",
    ):
        if forbidden in trusted_route:
            fail(f"GitHub-hosted unmerged CUDA route must not execute candidate code: {forbidden!r}")
    candidate_jobs = ("verify-release", "build-python", "build-napi", "build-rust")
    for name in candidate_jobs:
        job = workflow_job(name)
        require_fragment(job, "verify-cuda-trusted-route", f"{name} candidate route dependency")
        if not re.search(r"^    permissions:\n      contents: read$", job, re.MULTILINE):
            fail(f"{name} candidate execution must have read-only contents permission")
        if "contents: write" in job or "id-token: write" in job:
            fail(f"{name} candidate execution must not receive write or OIDC permission")
        require_fragment(job, "ref: ${{ env.RELEASE_CHECKOUT_REF }}", f"{name} candidate checkout")
        require_fragment(job, "persist-credentials: false", f"{name} candidate checkout")
        if "persist-credentials: true" in job:
            fail(f"{name} candidate checkout must not persist credentials")

    for name in PUBLISHING_JOBS:
        job = workflow_job(name)
        conditions = re.findall(r"^    if:\s*(.+)$", job, re.MULTILINE)
        if len(conditions) != 1:
            fail(f"{name} must have exactly one recognized job-level condition")
        condition = conditions[0]
        candidate_free = "inputs.candidate_commit == ''" in condition
        canonical_push = (
            "github.event_name == 'push'" in condition
            and "workflow_dispatch" not in condition
            and "||" not in condition
        )
        if not candidate_free and not canonical_push:
            fail(f"{name} must be unreachable from an unmerged candidate dispatch")


def driverless_smoke_sections(preflight: str) -> tuple[str, str]:
    python_marker = "printf 'cuda-preflight: prove the installed Python wheel defaults to CPU in a driverless container\\n'"
    napi_marker = "printf 'cuda-preflight: prove the installed N-API package defaults to CPU in a driverless container\\n'"
    gpu_marker = "run_forced_python()"
    try:
        _, after_python = preflight.split(python_marker, 1)
        python_smoke, after_napi = after_python.split(napi_marker, 1)
        napi_smoke, _ = after_napi.split(gpu_marker, 1)
    except ValueError:
        fail("CUDA preflight must contain both delimited driverless smoke sections")
    return python_smoke, napi_smoke


def require_driverless_device_absence(preflight: str) -> None:
    scrub = (
        "env -u FATHOMDB_EMBED_DEVICE -u FATHOMDB_RERANK_DEVICE -u CUDA_VISIBLE_DEVICES "
        "-u NVIDIA_VISIBLE_DEVICES -u HIP_VISIBLE_DEVICES -u ROCR_VISIBLE_DEVICES"
    )
    sections = driverless_smoke_sections(preflight)
    if any(section.count(scrub) != 1 for section in sections):
        fail("CUDA preflight must scrub device-selection variables in each driverless CPU-default smoke")
    for section in sections:
        for variable in DRIVERLESS_DEVICE_SELECTION_VARIABLES:
            assignment = re.compile(rf"(?:^|[;\s])(?:export\s+)?{variable}=", re.MULTILINE)
            docker_env = re.compile(rf"(?:^|\s)-e\s+{variable}(?:=|\s|$)", re.MULTILINE)
            if assignment.search(section) or docker_env.search(section):
                fail(f"CUDA preflight driverless smoke injects device-selection variable {variable}")


def require_driverless_runtime_absence(preflight: str) -> None:
    start = "printf 'cuda-preflight: prove the installed Python wheel defaults to CPU in a driverless container\\n'"
    end = "cat > \"$WORK_DIR/gpu-python-smoke.py\""
    try:
        isolated = preflight[preflight.index(start):preflight.index(end)]
    except ValueError:
        fail("CUDA preflight must delimit CPU and forced-driverless evidence before GPU smoke setup")
    for forbidden in (
        "dst=/opt/cuda/lib64,readonly",
        "dst=/usr/lib/x86_64-linux-gnu/libcudart.so.12,readonly",
        "LD_LIBRARY_PATH=/opt/cuda/lib64",
        "LD_LIBRARY_PATH=/usr/local/cuda/lib64",
    ):
        if forbidden in isolated:
            fail(f"CUDA preflight CPU or forced-driverless evidence exposes CUDA runtime: {forbidden!r}")


def require_cuda_package_rehearsal() -> None:
    schema = load_json(PACKAGE_REHEARSAL_SCHEMA)
    if schema.get("$id") != "https://fathomdb.dev/schemas/cuda-package-rehearsal/v2":
        fail("CUDA package rehearsal schema must declare its versioned schema ID")
    schema_version = schema.get("properties", {}).get("schema_version")
    if not isinstance(schema_version, dict) or schema_version.get("const") != "fathomdb.cuda-package-rehearsal/v2":
        fail("CUDA package rehearsal schema must pin its schema version")
    verifier = read_text(PACKAGE_REHEARSAL_VERIFIER)
    for fragment in (
        "package inventory must contain exactly four typed retained artifacts",
        "package digest mismatch",
        "route receipt does not bind the requested candidate",
        "preflight witness digest mismatch",
        "retained preflight witness fails Slice 10 validation",
        "verify-cuda-preflight-witness.py",
        "CPU {consumer} smoke does not prove the driverless installed-artifact contract",
        "GPU {consumer} smoke lacks GPU UUID/PID correlation",
        'raw != canonical_json(value)',
        "CLI archive is not deterministic gzip -n output",
        "unavailable real CLI hardware evidence must remain PENDING_EXTERNAL",
    ):
        require_fragment(verifier, fragment, "CUDA package rehearsal verifier")
    helper = read_text(PACKAGE_REHEARSAL_HELPER)
    for fragment in (
        'verify-cuda-unmerged-receipt.py',
        'verify-cuda-preflight-witness.py',
        '--candidate-sha "$candidate_sha"',
        'output directory must be new',
        'verify-cuda-package-rehearsal.py',
        '--cli-archive',
    ):
        require_fragment(helper, fragment, "CUDA package rehearsal helper")
    smoke = read_text(PACKAGE_REHEARSAL_SMOKE)
    for fragment in (
        'docker run --rm --network none',
        'env -i PATH=',
        'HF_HOME=/fathomdb-hf',
        'dst=/fathomdb-hf,readonly',
        'test ! -e /dev/nvidiactl',
        "--gpus ",
        "FATHOMDB_CUDA_GPU_UUID",
        'CUDA_GPU_DOCKER_SELECTOR="device=$CUDA_GPU_UUID"',
        '--gpus "$CUDA_GPU_DOCKER_SELECTOR"',
        '--query-compute-apps=pid',
        'source_imported',
        'gpu_uuid',
        'nvidia_smi_uuid',
        'XDG_CACHE_HOME=/fathomdb-product-cache',
        'model_cache_manifest',
        'doctor gpu --json',
    ):
        require_fragment(smoke, fragment, "CUDA package rehearsal installed-artifact smoke")
    if "device=0" in smoke:
        fail("CUDA package rehearsal must not pin evidence to mutable host index zero")
    for forbidden in (
        "--network host", "npm publish", "twine upload", "git tag", "CARGO_REGISTRY_TOKEN",
        "src=$PWD", "dst=/source", "src=$REPO_ROOT",
    ):
        if forbidden in helper or forbidden in smoke:
            fail(f"CUDA package rehearsal must not contain publication or broad-network escape: {forbidden!r}")

    job = workflow_job("cuda-package-rehearsal")
    if not re.search(
        r"^    if: \$\{\{ github\.event_name == 'workflow_dispatch' && inputs\.dry_run == true \}\}$",
        job,
        re.MULTILINE,
    ):
        fail("cuda-package-rehearsal must be dry-run-only at job scope")
    require_fragment(
        job,
        "needs: [verify-release, verify-cuda-trusted-route, cuda-contract-preflight]",
        "cuda-package-rehearsal",
    )
    require_fragment(job, "runs-on: [self-hosted, Linux, X64, gpu, cuda-12]", "cuda-package-rehearsal")
    require_fragment(job, CUDA_GPU_UUID_ENV, "cuda-package-rehearsal GPU UUID binding")
    if not re.search(r"^    environment: cuda-unmerged-preflight$", job, re.MULTILINE):
        fail("cuda-package-rehearsal must use the exact direct protected environment scalar")
    if not re.search(r"^    permissions:\n      contents: read\n", job, re.MULTILINE):
        fail("cuda-package-rehearsal must declare job-level read-only contents permission")
    if "contents: write" in job or "id-token: write" in job or "registry-url:" in job:
        fail("cuda-package-rehearsal must not receive publishing capabilities")
    for fragment in (
        "cuda-unmerged-route-receipt-${{ github.run_id }}-${{ github.run_attempt }}",
        "name: cuda-preflight-witness",
        "ref: ${{ github.workflow_sha }}",
        "persist-credentials: false",
        "control-plane/scripts/release/verify-cuda-unmerged-receipt.py",
        "control-plane/scripts/release/verify-cuda-preflight-witness.py",
        "ref: ${{ env.RELEASE_CHECKOUT_REF }}",
        "bash ../control-plane/scripts/release/cuda-package-rehearsal-smoke.sh",
        "--hf-home \"${FATHOMDB_CUDA_PREFLIGHT_HF_HOME:-${HF_HOME:-$HOME/.cache/huggingface}}\"",
        "bash control-plane/scripts/release/cuda-package-rehearsal.sh",
        "name: cuda-package-rehearsal",
    ):
        require_fragment(job, fragment, "cuda-package-rehearsal")
    candidate_checkout = job.index("ref: ${{ env.RELEASE_CHECKOUT_REF }}")
    receipt_check = job.index("control-plane/scripts/release/verify-cuda-unmerged-receipt.py")
    witness_check = job.index("control-plane/scripts/release/verify-cuda-preflight-witness.py")
    control_checkout = job.index("ref: ${{ github.workflow_sha }}")
    if not control_checkout < receipt_check < witness_check < candidate_checkout:
        fail("cuda-package-rehearsal must verify main-owned receipt and preflight witness before candidate checkout")

    python_build = workflow_job("build-python")
    napi_build = workflow_job("build-napi")
    if "x86_64-unknown-linux-gnu" in python_build or "linux-x64-gnu" in napi_build:
        fail("ordinary CPU build matrices must not duplicate the final Linux x64 CUDA producers")
    for publisher_artifact in (
        "name: python-dist-x86_64-unknown-linux-gnu",
        "name: napi-linux-x64-gnu",
    ):
        if publisher_artifact in job:
            fail("candidate CUDA rehearsal must not emit a canonical publisher-name artifact")
    for name in PUBLISHING_JOBS:
        if "cuda-package-rehearsal" in workflow_job(name):
            fail(f"{name} must not consume a candidate CUDA rehearsal artifact")

    canonical = workflow_job("build-cuda-linux-x64-gnu")
    if not re.search(
        r"^    if: \$\{\{ github\.event_name != 'workflow_dispatch' \|\| inputs\.dry_run != true \}\}$",
        canonical,
        re.MULTILINE,
    ):
        fail("canonical CUDA builder must run on every tag or non-dry-run route")
    require_fragment(canonical, "needs: [verify-release, verify-cuda-trusted-route]", "canonical CUDA builder")
    require_fragment(canonical, "runs-on: [self-hosted, Linux, X64, gpu, cuda-12]", "canonical CUDA builder")
    require_fragment(canonical, "environment: cuda-unmerged-preflight", "canonical CUDA builder")
    require_fragment(canonical, CUDA_GPU_UUID_ENV, "canonical CUDA builder GPU UUID binding")
    require_x64_cuda_toolkit_environment(canonical, "canonical CUDA builder")
    if not re.search(r"^    permissions:\n      contents: read$", canonical, re.MULTILINE):
        fail("canonical CUDA builder must declare read-only contents permission")
    if "id-token: write" in canonical or "registry-url:" in canonical:
        fail("canonical CUDA builder must not receive publishing capabilities")
    for fragment in (
        "ref: ${{ env.RELEASE_CHECKOUT_REF }}",
        "persist-credentials: false",
        "bash scripts/release/cuda-preflight.sh \"${{ github.workspace }}/cuda-canonical-preflight-witness\" --rerank-cuda",
        "name: python-dist-x86_64-unknown-linux-gnu",
        "name: napi-linux-x64-gnu",
    ):
        require_fragment(canonical, fragment, "canonical CUDA builder")
    for forbidden in (
        "candidate_commit", "cuda-unmerged-route-receipt", "cuda-package-rehearsal",
        "npm publish", "twine upload", "cargo publish", "id-token: write", "registry-url:",
    ):
        if forbidden in canonical:
            fail(f"canonical CUDA builder must not publish or consume a candidate route: {forbidden!r}")
    all_builds = workflow_job("all-builds-passed")
    require_fragment(all_builds, "- cuda-package-rehearsal", "all-builds-passed")
    require_fragment(all_builds, "- build-cuda-linux-x64-gnu", "all-builds-passed")
    require_fragment(
        all_builds,
        "github.event_name == 'workflow_dispatch' && inputs.dry_run == true && needs.cuda-package-rehearsal.result == 'success'",
        "all-builds-passed dry-run CUDA route",
    )
    require_fragment(
        all_builds,
        "(github.event_name != 'workflow_dispatch' || inputs.dry_run != true) && needs.build-cuda-linux-x64-gnu.result == 'success'",
        "all-builds-passed canonical CUDA route",
    )


def require_tegra_pool_only_in_tegra_sets(contract: str) -> None:
    """`tegra-pool` ships only in the two Tegra artifacts.

    The private CUDA pool and the import-time early cuInit hook are for
    aarch64 Linux integrated GPUs. Every x86_64 CUDA set and every CPU build
    must refuse the feature; both Tegra tuples carry it with `rerank-cuda`.
    """
    require_fragment(
        contract,
        f"CUDA_PYTHON_FEATURES_TEGRA='{PYTHON_CUDA_FEATURES_TEGRA}'",
        "CUDA artifact contract",
    )
    require_fragment(
        contract,
        f"CUDA_TEGRA_NAPI_FEATURES='{TEGRA_NAPI_FEATURES}'",
        "CUDA artifact contract",
    )
    # Every non-comment line of the contract that names the feature or the
    # Tegra set, in any quoting and whether literal or composed, must be the
    # Tegra set's own assignment.
    tegra_set = re.compile(
        r"^(?:export\s+)?(?:CUDA_PYTHON_FEATURES_TEGRA|CUDA_TEGRA_NAPI_FEATURES)="
    )
    for line in contract.splitlines():
        stripped = line.strip()
        if stripped.startswith("#"):
            continue
        if TEGRA_POOL_FEATURE not in stripped and not TEGRA_FEATURE_SET_REF.search(stripped):
            continue
        if not tegra_set.match(stripped):
            fail(
                "CUDA artifact contract may carry "
                f"{TEGRA_POOL_FEATURE!r} only in an approved Tegra feature set; got {stripped!r}"
            )
    non_tegra_builds = [CUDA_NAPI_BUILD, CUDA_PREFLIGHT, TS_PACKAGE, PYTHON_PYPROJECT]
    non_tegra_builds += sorted((ROOT / ".github/workflows").glob("*.y*ml"))
    non_tegra_builds += [
        path
        for path in sorted((ROOT / "scripts/release").glob("*"))
        if path.is_file() and "tegra" not in path.name and path != CUDA_CONTRACT
    ]
    for path in dict.fromkeys(non_tegra_builds):
        text = read_text(path)
        label = str(path.relative_to(ROOT))
        forbid_fragment(
            text,
            TEGRA_POOL_FEATURE,
            label,
            "tegra-pool belongs only to Tegra CUDA artifacts (R30-07)",
        )
        if TEGRA_FEATURE_SET_REF.search(text):
            fail(f"{label} must not use the Tegra feature set; it belongs to the Tegra build only")
    features = load_toml(NAPI_MANIFEST).get("features")
    if isinstance(features, dict):
        for name, members in features.items():
            if name != TEGRA_POOL_FEATURE and TEGRA_POOL_FEATURE in (members or []):
                fail(f"fathomdb-napi feature {name!r} must not enable {TEGRA_POOL_FEATURE!r}")
def require_tegra_napi_route(contract: str) -> None:
    """Require the Jetson Tegra N-API build to stay on its own contract axis.

    ADR-0.8.27-jetson-tegra-node-addon-distribution: the wrapper asserts every
    pin before its --assert-only exit, builds the dual-runtime tuple, proves
    the tegra glibc floor and CUDA linkage on the `.node`, records digests and
    publishes nothing. The x86_64 N-API wrapper and the generic
    linux-arm64-gnu package must not absorb any Tegra identity.
    """
    for fragment in (
        f"CUDA_RERANK_NAPI_FEATURES='{CUDA_RERANK_NAPI_FEATURES}'",
        f"CUDA_TEGRA_NAPI_FEATURES='{TEGRA_NAPI_FEATURES}'",
        'CUDA_TEGRA_NAPI_RUST_VERSION="$CUDA_RUST_VERSION"',
        f"CUDA_TEGRA_NAPI_NODE_VERSION='{CUDA_TEGRA_NAPI_NODE_VERSION}'",
        f"CUDA_TEGRA_NAPI_NPM_VERSION='{CUDA_TEGRA_NAPI_NPM_VERSION}'",
        f"CUDA_TEGRA_NAPI_PACKAGE='{TEGRA_NAPI_PACKAGE_NAME}'",
        f"CUDA_TEGRA_NAPI_TRIPLE='{TEGRA_NAPI_TRIPLE}'",
    ):
        require_fragment(contract, fragment, "CUDA artifact contract")
    if contract.count("CUDA_TEGRA_NAPI_RUST_VERSION=") != 1:
        fail("the Tegra N-API Rust pin must be shared with CUDA_RUST_VERSION by reference")
    ts_package = load_json(TS_PACKAGE)
    if ts_package.get("packageManager") != f"npm@{CUDA_TEGRA_NAPI_NPM_VERSION}":
        fail("src/ts packageManager must match the Tegra N-API npm pin")

    wrapper = read_text(CUDA_TEGRA_NAPI_BUILD)
    label = "Tegra CUDA N-API build wrapper"
    assertions = (
        '"$CUDA_TEGRA_HOST_ARCH"',
        '"$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc" --version | grep -F "$CUDA_TEGRA_HOST_NVCC_VERSION"',
        '"$CUDA_HOST_CC_TEGRA_ORIN" --version | grep -F "$CUDA_HOST_GCC_VERSION_TEGRA_ORIN"',
        '"$CUDA_HOST_CXX_TEGRA_ORIN" --version | grep -F "$CUDA_HOST_GCC_VERSION_TEGRA_ORIN"',
        '"$CUDA_TEGRA_HOST_CUDART_LIB/libcudart.so"',
        '"$CUDA_TEGRA_HOST_DRIVER_LIB"',
        '"rustc $CUDA_TEGRA_NAPI_RUST_VERSION "*)',
        '"v$CUDA_TEGRA_NAPI_NODE_VERSION"',
        '"$CUDA_TEGRA_NAPI_NPM_VERSION"',
        '"$CUDA_TEGRA_NAPI_PACKAGE"',
        'export CUDA_PATH="$CUDA_TEGRA_HOST_TOOLKIT_ROOT"',
        'export CUDACXX="$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc"',
        'export CC="$CUDA_HOST_CC_TEGRA_ORIN"',
        'export CXX="$CUDA_HOST_CXX_TEGRA_ORIN"',
        'export CUDAHOSTCXX="$CUDA_HOST_CXX_TEGRA_ORIN"',
        'export NVCC_CCBIN="$CUDA_HOST_CXX_TEGRA_ORIN"',
        'export CUDA_COMPUTE_CAP="$CUDA_COMPUTE_CAP_TEGRA_ORIN"',
        'export LIBRARY_PATH="$CUDA_TEGRA_HOST_CUDART_LIB${LIBRARY_PATH:+:$LIBRARY_PATH}"',
        'export PATH="$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin:$PATH"',
    )
    assert_only_exit = 'if [ "$ASSERT_ONLY" -eq 1 ]; then'
    build_command = "napi build --platform --release"
    for fragment in (*assertions, assert_only_exit, build_command):
        require_fragment(wrapper, fragment, label)
    exit_at = wrapper.index(assert_only_exit)
    if wrapper.index(build_command) < exit_at:
        fail(f"{label} must exit its --assert-only arm before building")
    for fragment in assertions:
        if wrapper.index(fragment) > exit_at:
            fail(f"{label} must assert {fragment!r} before the --assert-only exit")
    for fragment in (
        '--features "$CUDA_TEGRA_NAPI_FEATURES"',
        # @napi-rs/cli 2.x resolves the type-definition path against cwd.
        'NATIVE_DEST="$(realpath --relative-to="$TS_DIR" "$NATIVE_DIR")"',
        '  "$NATIVE_DEST"\n',
        "glibc-floor-contract.sh",
        'TEGRA_FLOOR="$(glibc_floor_for_family tegra)"',
        'check-glibc-floor.sh" --floor "$TEGRA_FLOOR" "$NATIVE_BINARY"',
        'bash "$SCRIPT_DIR/check-tegra-wheel-linkage.sh" "$NATIVE_BINARY"',
        "npm pack --ignore-scripts",
        "sha256sum --",
        "SHA256SUMS",
        "env -u LD_LIBRARY_PATH -u LIBRARY_PATH",
    ):
        require_fragment(wrapper, fragment, label)
    for forbidden, why in (
        ("npm publish", "the Tegra tarball is distributed only through the Tegra Pages route"),
        ("twine", "the wrapper publishes nothing"),
        ("cargo publish", "the wrapper publishes nothing"),
        ("maturin publish", "the wrapper publishes nothing"),
        ("maturin upload", "the wrapper publishes nothing"),
        ("git push", "only the hosted Pages job publishes"),
        ("gh workflow", "only the operator route dispatches publication"),
        ("gh release", "the wrapper publishes nothing"),
        ("gh api", "the wrapper publishes nothing"),
        ("gh-pages", "only the hosted Pages job publishes"),
        ("deploy-pages", "only the hosted Pages job publishes"),
        ("NODE_AUTH_TOKEN", "the Jetson build holds no publication credential"),
        ("NPM_TOKEN", "the Jetson build holds no publication credential"),
        ("src/ts/npm/", "the generic platform packages are not this wrapper's to touch"),
        ("CUDA_NAPI_HOST_", "the Tegra build never consumes the x86_64 N-API host axis"),
        ("CUDA_COMPUTE_CAP_X86_64", "Tegra is SM_87 only"),
        ("CUDA_NAPI_FEATURES", "the Tegra tuple is CUDA_TEGRA_NAPI_FEATURES"),
    ):
        forbid_fragment(wrapper, forbidden, label, why)

    x64_wrapper = read_text(CUDA_NAPI_BUILD)
    if "tegra" in x64_wrapper.lower():
        fail("the x86_64 CUDA N-API build wrapper must not consume the Tegra axis")

    tegra_package = load_json(TEGRA_NAPI_PACKAGE_JSON)
    binary = f"fathomdb.{TEGRA_NAPI_TRIPLE}.node"
    expected = {
        "name": TEGRA_NAPI_PACKAGE_NAME,
        "os": ["linux"],
        "cpu": ["arm64"],
        "libc": ["glibc"],
        "main": binary,
        "files": [binary],
    }
    for key, value in expected.items():
        if tegra_package.get(key) != value:
            fail(f"Tegra platform package {key!r} must be {value!r}; got {tegra_package.get(key)!r}")
    if "version" in tegra_package:
        fail("Tegra platform package version is stamped at build time, never committed")
    generic_package = load_json(GENERIC_ARM64_PACKAGE_JSON)
    if generic_package.get("name") != "fathomdb-linux-arm64-gnu" or generic_package.get("main") != binary:
        fail("the generic linux-arm64-gnu package must keep its name and binary")
    generic_text = json.dumps(generic_package).lower()
    if "tegra" in generic_text or "cuda" in generic_text:
        fail("the generic linux-arm64-gnu package must carry no Tegra or CUDA identity")


def main() -> None:
    record = load_json(PINNED_OVERRIDE_RECORD)
    candle_exception = record.get("cargo_candle_exception")
    if not isinstance(candle_exception, dict):
        fail(f"{PINNED_OVERRIDE_RECORD} has no cargo_candle_exception")
    candle_git = candle_exception.get("git")
    candle_rev = candle_exception.get("rev")
    if not isinstance(candle_git, str) or not isinstance(candle_rev, str):
        fail("cargo_candle_exception must name the candle-kernels git source and rev")
    workspace_manifest = load_toml(WORKSPACE_MANIFEST)
    patches = workspace_manifest.get("patch")
    if not isinstance(patches, dict):
        fail("workspace Cargo.toml has no [patch.crates-io] candle-kernels source pin")
    crates_io_patch = patches.get("crates-io")
    if not isinstance(crates_io_patch, dict):
        fail("workspace Cargo.toml has no [patch.crates-io] candle-kernels source pin")
    pin = crates_io_patch.get(CANDLE_KERNELS_PACKAGE)
    if not isinstance(pin, dict):
        fail(f"[patch.crates-io] lacks the {CANDLE_KERNELS_PACKAGE!r} source pin")
    if pin.get("git") != candle_git or pin.get("rev") != candle_rev:
        fail(
            f"[patch.crates-io] {CANDLE_KERNELS_PACKAGE} must pin {candle_git}@{candle_rev} "
            f"(scripts/pinned-override-rot.json cargo_candle_exception); got {pin!r}"
        )
    for package_name in CANDLE_FORK_PACKAGES:
        if package_name in crates_io_patch:
            fail(
                f"[patch.crates-io] must not patch {package_name!r}: the Candle fork crates "
                "come from crates.io"
            )

    lockfile = load_toml(LOCKFILE)
    locked_packages = lockfile.get("package")
    if not isinstance(locked_packages, list):
        fail("Cargo.lock has no package entries")
    expected_source = f"git+{candle_git}?rev={candle_rev}#{candle_rev}"
    entries = [
        entry
        for entry in locked_packages
        if isinstance(entry, dict) and entry.get("name") == CANDLE_KERNELS_PACKAGE
    ]
    if len(entries) != 1:
        fail(f"Cargo.lock must contain exactly one {CANDLE_KERNELS_PACKAGE!r} package; got {len(entries)}")
    source = entries[0].get("source")
    if source != expected_source:
        fail(
            f"Cargo.lock must resolve {CANDLE_KERNELS_PACKAGE!r} from {expected_source!r}; "
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
        "export CUDA_DRIVERLESS_NODE_IMAGE='node:25-trixie-slim'",
        "CUDA driverless N-API smoke glibc baseline",
    )
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
    require_tegra_pool_only_in_tegra_sets(contract)
    # 0.8.23 Slice 80.4 (R80-5, AC80-8): compute capability is a per-target
    # axis — x86_64's value stays the literal '75' this checker has always
    # pinned, Tegra Orin's '87' is new, and CUDA_COMPUTE_CAP itself (the
    # literal env var name candle-flash-attn-v3's build.rs in the pinned
    # Candle fork reads — not ours to rename) must still resolve to the
    # x86_64 value, since that is the only target build-napi-cuda.sh and
    # cuda-preflight.sh build for today.
    require_fragment(contract, "CUDA_COMPUTE_CAP_X86_64='75'", "CUDA artifact contract")
    require_fragment(contract, "CUDA_COMPUTE_CAP_TEGRA_ORIN='87'", "CUDA artifact contract")
    require_fragment(
        contract, 'CUDA_COMPUTE_CAP="$CUDA_COMPUTE_CAP_X86_64"', "CUDA artifact contract"
    )
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
    # 0.8.23 Slice 80.6 (D-80.6-4, AC80-8): host compiler becomes a per-target
    # axis, exactly as 80.4 did for compute capability. Each single x86_64
    # literal assertion becomes two — the literal itself, now on the x86_64
    # axis variable, plus the selector linkage proving the N-API host names
    # still resolve to that same literal. Strictly stronger than before: the
    # old assertion could not have caught a silent re-point of
    # CUDA_NAPI_HOST_CC at some other axis value.
    for axis_fragment, selector_fragment in (
        (
            f"CUDA_HOST_GCC_VERSION_X86_64='{CUDA_NAPI_HOST_GCC_VERSION}'",
            'CUDA_NAPI_HOST_GCC_VERSION="$CUDA_HOST_GCC_VERSION_X86_64"',
        ),
        (
            f"CUDA_HOST_CC_X86_64='{CUDA_NAPI_HOST_CC}'",
            'CUDA_NAPI_HOST_CC="$CUDA_HOST_CC_X86_64"',
        ),
        (
            f"CUDA_HOST_CXX_X86_64='{CUDA_NAPI_HOST_CXX}'",
            'CUDA_NAPI_HOST_CXX="$CUDA_HOST_CXX_X86_64"',
        ),
    ):
        require_fragment(contract, axis_fragment, "CUDA artifact contract")
        require_fragment(contract, selector_fragment, "CUDA artifact contract")
    for fragment in (
        f"CUDA_HOST_GCC_VERSION_TEGRA_ORIN='{CUDA_HOST_GCC_VERSION_TEGRA_ORIN}'",
        f"CUDA_HOST_CC_TEGRA_ORIN='{CUDA_HOST_CC_TEGRA_ORIN}'",
        f"CUDA_HOST_CXX_TEGRA_ORIN='{CUDA_HOST_CXX_TEGRA_ORIN}'",
        f"CUDA_TEGRA_HOST_CUDART_LIB='{CUDA_TEGRA_HOST_CUDART_LIB}'",
        f"CUDA_TEGRA_HOST_DRIVER_LIB='{CUDA_TEGRA_HOST_DRIVER_LIB}'",
    ):
        require_fragment(contract, fragment, "CUDA artifact contract")
    # The toolkit root and the nvcc version pin are measured IDENTICAL on the
    # Jetson (design § 2.7 / D-80.6-4), so they are shared by reference rather
    # than split into a second literal. Assert the linkage, and forbid the
    # second literal that would let the two targets drift apart silently.
    for fragment in (
        'CUDA_TEGRA_HOST_TOOLKIT_ROOT="$CUDA_NAPI_HOST_TOOLKIT_ROOT"',
        'CUDA_TEGRA_HOST_NVCC_VERSION="$CUDA_NAPI_HOST_NVCC_VERSION"',
    ):
        require_fragment(contract, fragment, "CUDA artifact contract")
    for fragment in (
        "CUDA_TEGRA_HOST_TOOLKIT_ROOT='",
        "CUDA_TEGRA_HOST_NVCC_VERSION='",
    ):
        forbid_fragment(
            contract,
            fragment,
            "CUDA artifact contract",
            "the Tegra toolkit root and nvcc pin are shared with x86_64 by "
            "reference (D-80.6-4), never re-declared as a second literal",
        )
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
        'export LIBRARY_PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT/targets/x86_64-linux/lib${LIBRARY_PATH:+:$LIBRARY_PATH}"',
        "CUDA N-API build wrapper",
    )
    require_fragment(napi_build, '"$CUDA_NAPI_HOST_CC" --version | grep -F "$CUDA_NAPI_HOST_GCC_VERSION"', "CUDA N-API build wrapper")
    require_fragment(napi_build, '"$CUDA_NAPI_HOST_CXX" --version | grep -F "$CUDA_NAPI_HOST_GCC_VERSION"', "CUDA N-API build wrapper")
    require_fragment(napi_build, "export CUDA_COMPUTE_CAP", "CUDA N-API build wrapper")
    require_fragment(napi_build, 'export PATH="$CUDA_NAPI_HOST_TOOLKIT_ROOT/bin:$PATH"', "CUDA N-API build wrapper")
    require_fragment(napi_build, 'grep -F "$CUDA_NAPI_HOST_NVCC_VERSION"', "CUDA N-API build wrapper")
    # 0.8.23 Slice 80.6 (D-80.6-1/2/4): the Tegra wheel is built host-natively
    # on the Jetson, so its build wrapper is the only place the per-target
    # toolchain axis is consumed. Every input comes from the contract, the
    # toolchain identity is self-asserted before the build the way
    # build-napi-cuda.sh does, and the wrapper publishes nothing.
    tegra_build = read_text(CUDA_TEGRA_PYTHON_BUILD)
    for fragment in (
        'export CUDA_PATH="$CUDA_TEGRA_HOST_TOOLKIT_ROOT"',
        'export CUDACXX="$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc"',
        'export CC="$CUDA_HOST_CC_TEGRA_ORIN"',
        'export CXX="$CUDA_HOST_CXX_TEGRA_ORIN"',
        'export CUDAHOSTCXX="$CUDA_HOST_CXX_TEGRA_ORIN"',
        'export NVCC_CCBIN="$CUDA_HOST_CXX_TEGRA_ORIN"',
        'export CUDA_COMPUTE_CAP="$CUDA_COMPUTE_CAP_TEGRA_ORIN"',
        'export PATH="$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin:$PATH"',
        # Measured: without LIBRARY_PATH naming the aarch64-linux target lib
        # dir the link fails `cannot find -lcudart` (design § 2.7).
        'export LIBRARY_PATH="$CUDA_TEGRA_HOST_CUDART_LIB${LIBRARY_PATH:+:$LIBRARY_PATH}"',
        'export LD_LIBRARY_PATH="$CUDA_TEGRA_HOST_CUDART_LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"',
        '"$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin/nvcc" --version',
        'grep -F "$CUDA_TEGRA_HOST_NVCC_VERSION"',
        '"$CUDA_HOST_CC_TEGRA_ORIN" --version | grep -F "$CUDA_HOST_GCC_VERSION_TEGRA_ORIN"',
        '"$CUDA_HOST_CXX_TEGRA_ORIN" --version | grep -F "$CUDA_HOST_GCC_VERSION_TEGRA_ORIN"',
        '"$CUDA_TEGRA_HOST_DRIVER_LIB"',
        '"$CUDA_TEGRA_HOST_ARCH"',
        '--features "$CUDA_PYTHON_FEATURES_TEGRA"',
        '--compatibility "$CUDA_TEGRA_WHEEL_COMPATIBILITY"',
        'grep -F "maturin $CUDA_MATURIN_VERSION"',
        # D-80.6-5: the Tegra floor is higher than the manylinux families'
        # BECAUSE the artifact is host-bound — it is declared in the contract
        # and asserted against the built artifact, never exempted from the
        # gate. Asserting it here is what makes that true of the wheel this
        # wrapper produces rather than only of the doc page.
        "glibc-floor-contract.sh",
        'glibc_floor_for_family tegra',
        "check-glibc-floor.sh",
    ):
        require_fragment(tegra_build, fragment, "Tegra CUDA Python build wrapper")
    # D-80.6-1: this wrapper BUILDS and PROVES; it never publishes. § 3 rules
    # publishing a Tegra artifact under the existing names out of scope, and
    # § 11 item 8 holds registry publication behind the staged gate.
    for forbidden in ("twine", "npm publish", "maturin publish", "maturin upload", "cargo publish"):
        forbid_fragment(
            tegra_build,
            forbidden,
            "Tegra CUDA Python build wrapper",
            "Slice 80.6 builds and proves the Tegra artifact but publishes nothing (D-80.6-1)",
        )
    require_tegra_napi_route(contract)
    preflight = read_text(CUDA_PREFLIGHT)
    inspector = read_text(CUDA_ARTIFACT_INSPECTOR)
    selection = read_text(CUDA_GPU_SELECTION)
    for fragment in (
        "FATHOMDB_CUDA_GPU_UUID must be a canonical GPU UUID",
        "requested GPU UUID must resolve exactly once on this host",
        "nvidia-smi --query-gpu=index,uuid --format=csv,noheader",
    ):
        require_fragment(selection, fragment, "CUDA GPU UUID selection")
    witness_schema = load_json(CUDA_PREFLIGHT_WITNESS_SCHEMA)
    if witness_schema.get("$id") != "https://fathomdb.dev/schemas/cuda-preflight-witness/v2":
        fail("CUDA preflight witness schema must declare its versioned schema ID")
    schema_version = witness_schema.get("properties", {}).get("schema_version")
    if not isinstance(schema_version, dict) or schema_version.get("const") != "fathomdb.cuda-preflight-witness/v2":
        fail("CUDA preflight witness schema must pin its schema version")
    verifier = read_text(CUDA_PREFLIGHT_WITNESS_VERIFIER)
    for fragment in (
        "EVIDENCE_NAMES",
        "candidate SHA does not match the requested candidate",
        "evidence digest mismatch",
        "is not canonical JSON",
        "witness root inventory is incomplete or contains unknown members",
        "sealed forced-device record is not installed-candidate evidence",
        "GPU {consumer} observation lacks PID correlation",
        "GPU {consumer} observation lacks an allocation witness",
        "ALLOCATION_WITNESS_KEYS",
        "does not prove isolated materialization",
    ):
        require_fragment(verifier, fragment, "CUDA preflight witness verifier")
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
        'inspect-cuda-artifacts.py',
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
        'Engine.open(str(pathlib.Path(directory) / f"driverless-{policy}.fdb"), use_default_embedder=True)',
        'engine.embed(f"driverless Python {policy} CPU proof")',
        '{ useDefaultEmbedder: true }',
        'await engine.embed(`driverless N-API ${policy} CPU proof`)',
        'npm install --offline --ignore-scripts --no-audit --no-fund',
        'test ! -e /dev/nvidiactl',
        'sha256sum --check --status',
        '. "$SCRIPT_DIR/cuda-image-attestation.sh"',
        'assert_cuda_manylinux_image',
        'auditwheel show "/witness/python-dist/$WHEEL_BASENAME"',
        "manylinux_2_28",
        "--query-compute-apps=pid,process_name --format=csv,noheader",
        "--gpus ",
        "FATHOMDB_EMBED_DEVICE=cuda:0",
        "FATHOMDB_GPU_ALLOCATION_WITNESS=1",
        "installed Python CUDA artifact GPU proof",
        "installed N-API CUDA artifact GPU proof",
        "gpu-python-cuda-witness.json",
        "gpu-napi-cuda-witness.json",
        "gpu-napi-cuda-smoke.txt",
        "forced-cuda-unavailable-python.json",
        "forced-cuda-unavailable-napi.json",
        "smoke-cache-topology.json",
        'python /fathomdb-harness/forced-python-open.py',
        'src=$FORCED_PYTHON_SITE,dst=/fathomdb-site,readonly',
        'PYTHONPATH=/fathomdb-site',
        'src=$WORK_DIR/forced-napi-open.mjs,dst=/fathomdb-harness/forced-napi-open.mjs,readonly',
        'src=$FORCED_NAPI_INSTALL/node_modules,dst=/fathomdb-harness/node_modules,readonly',
        'installed Python CUDA artifact GPU smoke: ok',
        'installed N-API CUDA artifact GPU smoke: ok',
        'product cache has unexpected files',
        'dst=/fathomdb-hf,readonly',
        'dst=/fathomdb-product-cache',
        'HOME=/fathomdb-unavailable-home',
        'XDG_CACHE_HOME=/fathomdb-product-cache',
        '--network none',
        "cuda-preflight-witness.json",
        'python3 "$SCRIPT_DIR/verify-cuda-preflight-witness.py"',
        '--candidate-sha "$CANDIDATE_SHA"',
        "FATHOMDB_CUDA_GPU_UUID",
        'CUDA_GPU_DOCKER_SELECTOR="device=$CUDA_GPU_UUID"',
        '--gpus "$CUDA_GPU_DOCKER_SELECTOR"',
        "requested_host_gpu_uuid",
        "resolved_host_gpu_index",
        "WHEEL_FILENAME",
    ):
        require_fragment(preflight, fragment, "CUDA preflight")
    for fragment in (
        "fathomdb.cuda-artifact-linkage/v1",
        "forbidden CUDA/NVIDIA ELF dependency or SONAME",
        "forbidden CUDA/NVIDIA shared-library payload",
        "--python-wheel",
        "--napi-tarball",
    ):
        require_fragment(inspector, fragment, "CUDA artifact linkage inspector")
    if preflight.count("FATHOMDB_GPU_ALLOCATION_WITNESS=1") != 2:
        fail("CUDA preflight must request one in-process allocation witness for each GPU consumer")
    if "/input/fathomdb.whl" in preflight or "/input/fathomdb.whl" in read_text(PACKAGE_REHEARSAL_SMOKE):
        fail("CUDA installed-wheel smokes must retain a valid wheel filename")
    if preflight.count('-e "WHEEL_FILENAME=$WHEEL_FILENAME"') != 3:
        fail("CUDA preflight must pass its local wheel filename explicitly to every wheel container")
    if read_text(PACKAGE_REHEARSAL_SMOKE).count('-e "WHEEL_FILENAME=$WHEEL_FILENAME"') != 2:
        fail("CUDA package rehearsal must pass its local wheel filename explicitly to every wheel container")
    if "device=0" in preflight:
        fail("CUDA preflight must not pin evidence to mutable host index zero")
    for forbidden in (
        '--mount "type=bind,src=$CUDA_NAPI_HOST_TOOLKIT_ROOT,dst=/opt/cuda,readonly"',
        '--mount "type=bind,src=$CUDA_TOOLKIT_ROOT,dst=/opt/cuda,readonly"',
        "CUDACXX=/opt/cuda/bin/nvcc",
        'ldd "$NAPI_BINARY" || true',
        'ldd "$PYTHON_EXTENSION" || true',
        'auditwheel show "$WHEEL"',
        '--mount "type=bind,src=$REPO_ROOT,dst=/workspace"',
        "FORCED_NAPI_HARNESS",
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
    if "use_default_embedder=False" in preflight:
        fail("CUDA preflight must not use a no-embedder Python smoke")
    if "{ useDefaultEmbedder: false }" in preflight:
        fail("CUDA preflight must not use a no-embedder N-API smoke")
    if preflight.count('--network none') != 10:
        fail(
            "CUDA preflight must isolate the install, auditwheel, driverless, "
            "embedding/reranker forced-device, and GPU containers"
        )
    if preflight.count(
        '--mount "type=bind,src=$DEFAULT_EMBEDDER_HF_HOME,dst=/fathomdb-hf,readonly"'
    ) != 4:
        fail("CUDA preflight must mount the pinned read-only model seed into all four model-loading smokes")
    if preflight.count('dst=/fathomdb-product-cache"') != 4:
        fail("CUDA preflight must mount four distinct writable product caches")
    if preflight.count("--target /fathomdb-tmp/python-site") != 2:
        fail("CUDA preflight must install each non-root Python smoke into its writable isolated target")
    if preflight.count("PYTHONPATH=/fathomdb-tmp/python-site") != 1:
        fail("CUDA preflight must expose the isolated Python smoke target")
    python_gpu_start = preflight.find('PYTHON_GPU_CONTAINER=')
    python_gpu_end = preflight.find('seal_gpu_observation "$PYTHON_GPU_CONTAINER"', python_gpu_start)
    if python_gpu_start < 0 or python_gpu_end < 0:
        fail("CUDA preflight must retain the Python GPU smoke section")
    python_gpu = preflight[python_gpu_start:python_gpu_end]
    for fragment in (
        '--mount "type=bind,src=$CUDA_NAPI_HOST_TOOLKIT_ROOT/lib64,dst=/opt/cuda/lib64,readonly"',
        "-e LD_LIBRARY_PATH=/opt/cuda/lib64",
    ):
        require_fragment(python_gpu, fragment, "CUDA Python GPU smoke runtime")
    if preflight.count("sha256sum --check --status") != 2:
        fail("CUDA preflight must verify both pinned embedder and TinyBERT reranker cache manifests")
    if preflight.count("--query-compute-apps=pid,process_name --format=csv,noheader") != 1:
        fail("CUDA preflight must observe each spawned GPU runtime PID and process name")
    require_driverless_device_absence(preflight)
    require_driverless_runtime_absence(preflight)
    for fragment in (
        "FATHOMDB_CUDA_PREFLIGHT_RERANKER_CACHE",
        "dst=/fathomdb-reranker-cache-root,readonly",
        "FATHOMDB_RERANKER_CACHE=/fathomdb-reranker-cache-root",
        "reranker-cache-manifest.json",
        "reranker-python-cpu-smoke.json",
        "reranker-napi-cpu-smoke.json",
        "forced-reranker-python.json",
        "forced-reranker-napi.json",
        "forced-reranker-python.py",
        "forced-reranker-napi.mjs",
        "from fathomdb import rerank",
    ):
        require_fragment(preflight, fragment, "CUDA reranker v3 preflight")
    for fragment in (
        "--reranker-cache-manifest",
        "reranker-cli-doctor.json",
        "doctor reranker-gpu --json",
        "FATHOMDB_RERANK_DEVICE",
    ):
        require_fragment(read_text(PACKAGE_REHEARSAL_SMOKE), fragment, "CUDA reranker v3 rehearsal smoke")

    require_upload_retry_action()
    require_unmerged_candidate_control_plane()
    require_unmerged_workflow_route()

    job = workflow_job("cuda-contract-preflight")
    if not re.search(
        r"^    if: \$\{\{ github\.event_name == 'workflow_dispatch' && inputs\.dry_run == true \}\}$",
        job,
        re.MULTILINE,
    ):
        fail("cuda-contract-preflight must be dry-run-only at job scope")
    require_fragment(job, "runs-on: [self-hosted, Linux, X64, gpu, cuda-12]", "cuda-contract-preflight")
    require_fragment(job, CUDA_GPU_UUID_ENV, "cuda-contract-preflight GPU UUID binding")
    for label in RUNNER_LABELS:
        require_fragment(job, label, "cuda-contract-preflight runner labels")
    require_fragment(
        job,
        "needs: [verify-release, verify-cuda-trusted-route]",
        "cuda-contract-preflight",
    )
    if not re.search(r"^    environment: cuda-unmerged-preflight$", job, re.MULTILINE):
        fail("cuda-contract-preflight must use the exact direct protected environment scalar")
    require_fragment(job, "actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093", "cuda-contract-preflight")
    if not re.search(
        r"python3 control-plane/scripts/release/verify-cuda-unmerged-receipt\.py \\\n"
        r"\s+--receipt control-plane/receipt/cuda-unmerged-route-receipt\.json \\\n"
        r"\s+--manifest control-plane/dev/release/cuda-unmerged-candidates\.json \\\n",
        job,
    ):
        fail("cuda-contract-preflight receipt verifier must bind the exact main-owned manifest")
    candidate_checkout = job.index("ref: ${{ env.RELEASE_CHECKOUT_REF }}")
    receipt_check = job.index("control-plane/scripts/release/verify-cuda-unmerged-receipt.py")
    control_plane_checkout = re.search(
        r"^      - uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0(?: #.*)?\n"
        r"        if: \$\{\{ github\.event_name == 'workflow_dispatch' && inputs\.dry_run == true \}\}\n"
        r"        with:\n"
        r"          ref: \$\{\{ github\.workflow_sha \}\}\n"
        r"          path: control-plane\n"
        r"          persist-credentials: false\n",
        job,
        re.MULTILINE,
    )
    if control_plane_checkout is None:
        fail("cuda-contract-preflight must check out the pinned main-owned control plane")
    if control_plane_checkout.start() >= receipt_check:
        fail("cuda-contract-preflight must check out the main-owned control plane before receipt verification")
    if receipt_check >= candidate_checkout:
        fail("cuda-contract-preflight must verify the hosted receipt before candidate checkout")
    if not re.search(
        r"^    permissions:\n      contents: read\n",
        job,
        re.MULTILINE,
    ):
        fail("cuda-contract-preflight must declare job-level read-only contents permission")
    if "contents: write" in job or "id-token: write" in job:
        fail("cuda-contract-preflight must not inherit release publishing permissions")
    require_fragment(job, "bash scripts/release/cuda-preflight.sh", "cuda-contract-preflight")
    require_x64_cuda_toolkit_environment(job, "cuda-contract-preflight")
    require_fragment(job, "${{ env.RELEASE_CHECKOUT_REF }}", "cuda-contract-preflight checkout")
    require_fragment(job, "uses: ./control-plane/.github/actions/upload-artifact-retry", "cuda-contract-preflight witness upload")
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

    require_cuda_package_rehearsal()

    reranker_job = workflow_job("cuda-reranker-package-rehearsal")
    require_fragment(
        reranker_job,
        CUDA_GPU_UUID_ENV,
        "cuda-reranker-package-rehearsal GPU UUID binding",
    )
    require_x64_cuda_toolkit_environment(
        workflow_job("cuda-package-rehearsal"), "cuda-package-rehearsal"
    )
    require_x64_cuda_toolkit_environment(
        reranker_job, "cuda-reranker-package-rehearsal"
    )

    print("cuda-release-contract: pass")


if __name__ == "__main__":
    main()
