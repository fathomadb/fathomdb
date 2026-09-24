#!/usr/bin/env python3
"""The feature-complete test gate behind `scripts/test-feature-complete.sh`.

Lists every test binary of each crate under the workspace gate's features
and under each feature set of the committed `scripts/test-feature-matrix.toml`,
then runs every test that appears only under a matrix set (whole feature-gated
targets and item-level `#[cfg(feature = ...)]` tests alike), once, under the
smallest set that has it. It runs with CUDA on the two RTX 3090s, real model
weights (embedder, reranker, nomic), and the ONNX Runtime assets. A test that
skips itself or is `#[ignore]`d fails the gate unless
`scripts/test-skip-allowlist.toml` names it with a class and reason; an
allowlist entry that matches nothing is stale and also fails the gate.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
import zipfile
from typing import Callable, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))
import test_targets  # noqa: E402


REPO_ROOT = test_targets.REPO_ROOT
CUDA_ROOT = Path("/usr/local/cuda")
REQUIRED_GPU = "RTX 3090"
CLI_BINARY = "fathomdb"
# Self-skip phrasing in this workspace varies ("[SKIP] ...", "SKIP name: ...",
# "skipping ...", "gated-to-skip", "PENDING_EXTERNAL ..."); match words, not
# exact tokens.
SKIP_MARKERS = (
    re.compile(r"\bskip(?:s|ped|ping)?\b", re.IGNORECASE),
    re.compile(r"\bPENDING(?:_EXTERNAL)?\b"),
)
MARKER_CLASSES = frozenset({"opt-in-experiment", "benign-message"})
# Inventory-style builds need no debug info; it keeps listing and run
# artifacts to a few GB per feature set.
GATE_BUILD_ENV = {
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
}
_RUNNING = re.compile(r"^\s*Running (?:unittests )?(\S+) \(")
_TEST_LINE = re.compile(r"^test (\S+) \.\.\. ?(.*)$")
_RESULT = re.compile(r"^test result: (\S+)\.")


class FeatureCompleteError(Exception):
    """A preflight or gate failure."""


class ScanResult:
    """Skip markers, ignored tests, and failures seen in one cargo run."""

    __slots__ = ("markers", "ignored", "failed", "ran_targets", "seen")

    def __init__(self) -> None:
        self.markers: list[tuple[str, str]] = []
        self.ignored: list[str] = []
        self.failed: list[str] = []
        self.ran_targets: list[str] = []
        self.seen: list[str] = []


def _source_label(source: str, crate: test_targets.Crate) -> str:
    for target in crate.targets:
        if target.source == source:
            return target.name
    if source == "src/lib.rs":
        return "lib"
    if source == "src/main.rs":
        return f"bin:{crate.name}"
    if source.startswith("src/bin/"):
        return f"bin:{Path(source).stem}"
    return source


def scan_output(text: str, crate: test_targets.Crate) -> ScanResult:
    """Attribute every marker to the test whose `test <name> ...` line precedes
    it (the gate runs one test thread per binary)."""

    result = ScanResult()
    target_id: str | None = None
    current: str | None = None
    for line in text.splitlines():
        running = _RUNNING.match(line)
        if running is not None:
            target_id = f"{crate.name}::{_source_label(running.group(1), crate)}"
            result.ran_targets.append(target_id)
            current = None
            continue
        if target_id is None:
            # Build output before the first test binary is not a test's.
            continue
        rest = line
        test_line = _TEST_LINE.match(line)
        if test_line is not None:
            current = f"{target_id}::{test_line.group(1)}"
            result.seen.append(current)
            rest = test_line.group(2)
            status = rest.strip()
            if status == "ignored" or status.startswith("ignored,"):
                result.ignored.append(current)
                continue
            if status == "FAILED":
                result.failed.append(current)
                continue
        elif line.strip() == "FAILED" and current is not None:
            result.failed.append(current)
            continue
        summary = _RESULT.match(line)
        if summary is not None:
            if summary.group(1) != "ok":
                result.failed.append(f"{target_id}: test result {summary.group(1)}")
            current = None
            continue
        for pattern in SKIP_MARKERS:
            found = pattern.search(rest)
            if found is not None:
                result.markers.append((current or target_id, found.group(0)))
                break
    return result


def _matches(entry_id: str, test_id: str) -> bool:
    return test_id == entry_id or test_id.startswith(entry_id + "::")


def contract_failures(
    results: Sequence[ScanResult],
    allowlist: Sequence[dict[str, object]],
    listed: set[str] | frozenset[str] = frozenset(),
) -> list[str]:
    """Apply the skip contract: unexcused markers and ignored tests, failed
    tests, and stale allowlist entries each fail the gate. An `exclude` entry
    (a test the gate deliberately does not run) is used when `listed` contains
    it."""

    entries = [e for e in allowlist if e.get("class") != "platform-excluded"]
    used: set[int] = {
        n for n, e in enumerate(entries) if e.get("exclude") and str(e["id"]) in listed
    }
    failures: list[str] = []
    for result in results:
        failures.extend(f"test failed: {name}" for name in result.failed)
        for test_id, marker in result.markers:
            hits = [n for n, e in enumerate(entries) if _matches(str(e["id"]), test_id)]
            used.update(hits)
            if not any(entries[n]["class"] in MARKER_CLASSES for n in hits):
                failures.append(
                    f"skip marker {marker!r} from {test_id} is not allowlisted"
                )
        for test_id in result.ignored:
            hits = [n for n, e in enumerate(entries) if _matches(str(e["id"]), test_id)]
            used.update(hits)
            excused = any(
                entries[n]["class"] == "ignored-by-design"
                or (
                    entries[n]["id"] != test_id
                    and entries[n]["class"] == "opt-in-experiment"
                )
                for n in hits
            )
            if not excused:
                failures.append(
                    f"ignored test {test_id} is not allowlisted as ignored-by-design"
                )
    for n, entry in enumerate(entries):
        if n not in used:
            failures.append(
                f"stale allowlist entry {entry['id']!r} ({entry['class']}): matched no skip or ignored test"
            )
    return failures


def cuda_environment(
    base: dict[str, str],
    cuda_root: Path,
    run: Callable[[list[str], dict[str, str]], str],
) -> dict[str, str]:
    """Return `base` with the CUDA toolchain on the path and the two RTX 3090s
    selected by PCI bus order; fail if either is not an RTX 3090."""

    nvcc = cuda_root / "bin" / "nvcc"
    if not os.access(nvcc, os.X_OK):
        raise FeatureCompleteError(f"nvcc not found at {nvcc}")
    env = dict(base)
    env["PATH"] = f"{cuda_root / 'bin'}:{base.get('PATH', '')}".rstrip(":")
    env["CUDA_ROOT"] = str(cuda_root)
    env["CUDA_PATH"] = str(cuda_root)
    libraries = [str(cuda_root / "lib64"), str(cuda_root / "lib64" / "stubs")]
    if base.get("LIBRARY_PATH"):
        libraries.append(base["LIBRARY_PATH"])
    env["LIBRARY_PATH"] = ":".join(libraries)
    env["CUDA_DEVICE_ORDER"] = "PCI_BUS_ID"
    env["CUDA_VISIBLE_DEVICES"] = "0,1"
    listing = run(
        ["nvidia-smi", "--query-gpu=index,pci.bus_id,name", "--format=csv,noheader"],
        env,
    )
    names = {}
    for line in listing.splitlines():
        fields = [field.strip() for field in line.split(",")]
        if len(fields) >= 3:
            names[fields[0]] = fields[2]
    for index in ("0", "1"):
        if REQUIRED_GPU not in names.get(index, ""):
            raise FeatureCompleteError(
                f"CUDA device {index} in PCI bus order is {names.get(index)!r}, not an {REQUIRED_GPU}"
            )
    return env


def runner_environment(base: dict[str, str], scratch: Path) -> dict[str, str]:
    env = {k: v for k, v in base.items() if k != "FATHOMDB_SKIP_NETWORK_TESTS"}
    env["FATHOMDB_SLICE72_RUNNER"] = "approved-nvidia"
    env["FATHOMDB_SLICE72_RECEIPT_DIR"] = str(scratch / "slice72-receipts")
    return env


SLICE72_TARGET = "slice72_concurrent_gpu"
SLICE72_SUPPORT = (
    REPO_ROOT / "src/rust/crates/fathomdb-engine/tests/support/slice72_gpu_telemetry.rs"
)
_CACHE_PREFIX = re.compile(r'cache_prefix\(\s*"([^"]+)"')
SLICE72_MODEL_FILES = ("config.json", "tokenizer.json", "model.safetensors")


def stage_slice72_assets(support: Path, cache_root: Path, destination: Path) -> Path:
    """Build Slice 72's immutable asset root (`bge/`, `reranker/`) from the
    warmed model caches, located by the cache identities its test harness
    names; the harness itself verifies every file's SHA-256."""

    identities = _CACHE_PREFIX.findall(support.read_text(encoding="utf-8"))
    sources = {
        "bge": [i for i in identities if i.startswith("BAAI/")],
        "reranker": [i for i in identities if i.startswith("cross-encoder/")],
    }
    for kind, found in sources.items():
        if len(found) != 1:
            raise FeatureCompleteError(
                f"expected one Slice 72 {kind} cache identity in {support}, found {found}"
            )
    for kind, cache_kind in (("bge", "embedders"), ("reranker", "reranker")):
        prefix = hashlib.sha256(sources[kind][0].encode()).hexdigest()[:12]
        source = cache_root / "fathomdb" / cache_kind / prefix
        target = destination / kind
        target.mkdir(parents=True, exist_ok=True)
        for name in SLICE72_MODEL_FILES:
            if not (source / name).is_file():
                raise FeatureCompleteError(
                    f"Slice 72 {kind} cache file missing: {source / name} (warm the cache first)"
                )
            copy = target / name
            if not copy.exists():
                shutil.copyfile(source / name, copy)
                copy.chmod(0o444)
    return destination


# nomic-embed-text-v1.5 for `nomic_smoke`. The embedder crate loads it with
# `NomicEmbedder::from_dir` but has no fetcher, so the gate provisions it at a
# pinned revision into the embedder cache root the test resolves.
NOMIC_REVISION = "e9b6763023c676ca8431644204f50c2b100d9aab"
_NOMIC_URL = (
    f"https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/{NOMIC_REVISION}"
)
NOMIC_FILES = {
    "tokenizer.json": (
        "git-blob-sha1",
        "688882a79f44442ddc1f60d70334a7ff5df0fb47",
        f"{_NOMIC_URL}/tokenizer.json",
    ),
    "model.safetensors": (
        "sha256",
        "9e7d262b1fe5ea350782829496efa831901b77486bbde1cea54a4c822d010d5c",
        f"{_NOMIC_URL}/model.safetensors",
    ),
}


def _digest(path: Path, algorithm: str) -> str:
    if algorithm == "sha256":
        digest = hashlib.sha256()
    elif algorithm == "git-blob-sha1":
        digest = hashlib.sha1()
        digest.update(b"blob %d\0" % path.stat().st_size)
    else:
        raise FeatureCompleteError(f"unknown digest algorithm {algorithm}")
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def provision_weights(
    directory: Path,
    files: dict[str, tuple[str, str, str]],
    fetch: Callable[[str, Path], None],
) -> list[str]:
    """Fetch each pinned file that is missing or wrong, verifying it before it
    is renamed into place. Returns the names fetched."""

    directory.mkdir(parents=True, exist_ok=True)
    fetched: list[str] = []
    for name, (algorithm, expected, url) in sorted(files.items()):
        final = directory / name
        if final.is_file() and _digest(final, algorithm) == expected:
            continue
        partial = directory / f"{name}.partial"
        partial.unlink(missing_ok=True)
        try:
            fetch(url, partial)
            actual = _digest(partial, algorithm)
            if actual != expected:
                raise FeatureCompleteError(
                    f"{name}: {algorithm} {actual} does not match pinned {expected}"
                )
            os.replace(partial, final)
        finally:
            partial.unlink(missing_ok=True)
        fetched.append(name)
    return fetched


def _http_fetch(url: str, destination: Path) -> None:
    request = urllib.request.Request(
        url, headers={"User-Agent": "fathomdb-feature-complete"}
    )
    with (
        urllib.request.urlopen(request, timeout=60) as response,
        open(destination, "wb") as out,
    ):
        shutil.copyfileobj(response, out, 1 << 20)


def entry_environment(
    env: dict[str, str], targets: Sequence[str], assets: Path
) -> dict[str, str]:
    """Per-run overrides: Slice 72 requires exactly one process-visible CUDA
    device (the first RTX 3090) and its staged asset root."""

    if SLICE72_TARGET not in targets:
        return dict(env)
    result = dict(env)
    result["CUDA_VISIBLE_DEVICES"] = "0"
    result["FATHOMDB_SLICE72_ASSET_ROOT"] = str(assets)
    return result


# ONNX Runtime and the exported bge-small ONNX graph for the `onnx-embedder`
# targets (`dev/tools/onnx/README.md`). The runtime library is the one inside
# the pinned onnxruntime wheel (ABI-compatible with `ort =2.0.0-rc.10`); the
# graph is the byte-deterministic export of the pinned bge-small weights.
ONNXRUNTIME_WHEEL = {
    "url": "https://files.pythonhosted.org/packages/7c/43/2a4e04f8dbeffad19bbcced4bcd4289bf478921518437404d6b92bdf213b/"
    "onnxruntime-1.26.0-cp312-cp312-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl",
    "sha256": "9b6dd70599005bd1bf29779f04a91978b92b5e719c11a20068a8f8e535f725b6",
    "member": "onnxruntime/capi/libonnxruntime.so.1.26.0",
    "member_sha256": "50775d390eb55e7abd9f6d734da103a04f0e5342ef0a76b1c6ec795544439295",
}
ONNXRUNTIME_HOSTS = frozenset({"x86_64-unknown-linux-gnu"})
ONNX_MODEL_SHA256 = "c92689ecdefc35c3b533f68449cf1a50e7a1301736d8031372b0fa70e862c5a5"
ONNX_EXPORT_SCRIPT = REPO_ROOT / "dev/tools/onnx/export_bge_small_onnx.py"
ONNX_EXPORT_REQUIREMENTS = (
    ("torch==2.4.1", "https://download.pytorch.org/whl/cpu"),
    ("transformers==4.44.2", None),
    ("numpy<2", None),
    ("onnx", None),
)
# The bge-small tokenizer the embedder loader pins (TOKENIZER_JSON_SHA256).
BGE_TOKENIZER_SHA256 = (
    "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66"
)


def provision_wheel_member(
    cache: Path, spec: dict[str, str], fetch: Callable[[str, Path], None]
) -> Path:
    """Extract one pinned file from a pinned wheel into `cache`, verifying the
    wheel and the file; an existing valid file is reused."""

    member = spec["member"]
    final = cache / Path(member).name
    if final.is_file() and _digest(final, "sha256") == spec["member_sha256"]:
        return final
    cache.mkdir(parents=True, exist_ok=True)
    wheel = cache / "download.whl.partial"
    partial = cache / f"{final.name}.partial"
    try:
        fetch(spec["url"], wheel)
        if _digest(wheel, "sha256") != spec["sha256"]:
            raise FeatureCompleteError(
                f"{spec['url']}: wheel sha256 does not match the pin"
            )
        with zipfile.ZipFile(wheel) as archive, archive.open(member) as source:
            with open(partial, "wb") as out:
                shutil.copyfileobj(source, out)
        if _digest(partial, "sha256") != spec["member_sha256"]:
            raise FeatureCompleteError(f"{member}: sha256 does not match the pin")
        os.replace(partial, final)
    finally:
        wheel.unlink(missing_ok=True)
        partial.unlink(missing_ok=True)
    return final


def provision_generated(
    path: Path, sha256: str, generate: Callable[[Path], None]
) -> bool:
    """Ensure a generated asset exists with the pinned sha256, generating it
    when missing or wrong. Returns whether it was generated."""

    if path.is_file() and _digest(path, "sha256") == sha256:
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = path.with_name(f"{path.name}.partial")
    try:
        generate(partial)
        actual = _digest(partial, "sha256")
        if actual != sha256:
            raise FeatureCompleteError(
                f"{path.name}: generated sha256 {actual} does not match pinned {sha256}"
            )
        os.replace(partial, path)
    finally:
        partial.unlink(missing_ok=True)
    return True


def _export_onnx_model(venv: Path) -> Callable[[Path], None]:
    def generate(destination: Path) -> None:
        python = venv / "bin" / "python"
        if not python.exists():
            subprocess.run([sys.executable, "-m", "venv", str(venv)], check=True)
        for requirement, index in ONNX_EXPORT_REQUIREMENTS:
            command = [str(python), "-m", "pip", "install", "-q", requirement]
            if index:
                command += ["--index-url", index]
            subprocess.run(command, check=True)
        out = destination.with_suffix(".onnx")
        subprocess.run(
            [str(python), str(ONNX_EXPORT_SCRIPT), "--out", str(out)],
            cwd=REPO_ROOT,
            check=True,
        )
        os.replace(out, destination)

    return generate


def onnx_environment(
    env: dict[str, str], library: Path, model: Path, tokenizer: Path
) -> dict[str, str]:
    result = dict(env)
    result["ORT_DYLIB_PATH"] = str(library)
    result["FATHOMDB_ONNX_MODEL_PATH"] = str(model)
    result["FATHOMDB_ONNX_TOKENIZER_PATH"] = str(tokenizer)
    return result


def test_command(
    package: str,
    features: Sequence[str],
    selectors: Sequence[str],
    tests: Sequence[str],
) -> list[str]:
    """One run of exact test names from the selected binaries of a crate."""

    command = ["cargo", "test", "--locked", "-p", package, "--no-default-features"]
    if features:
        command += ["--features", ",".join(features)]
    command += list(selectors)
    return command + ["--", "--exact", *tests, "--nocapture", "--test-threads=1"]


def selector(label: str) -> list[str]:
    """Cargo arguments that select one listed test binary."""

    if label == "lib":
        return ["--lib"]
    if label.startswith("bin:"):
        return ["--bin", label[len("bin:") :]]
    return ["--test", label]


def _binary_label(target: dict[str, object]) -> str:
    kinds = [str(kind) for kind in target.get("kind") or []]  # type: ignore[union-attr]
    name = str(target.get("name"))
    if kinds == ["test"]:
        return name
    if "bin" in kinds:
        return f"bin:{name}"
    return "lib"


def _terse(listing: str) -> set[str]:
    return {
        line[: -len(": test")]
        for line in listing.splitlines()
        if line.endswith(": test")
    }


def parse_listing(
    messages: str, lister: Callable[[str], tuple[str, str]]
) -> dict[str, set[str]]:
    """Tests per binary label from `cargo build --tests --message-format json`
    output, listing each executable with `--list --format terse`."""

    listing: dict[str, set[str]] = {}
    for line in messages.splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if record.get("reason") != "compiler-artifact":
            continue
        if not (record.get("profile") or {}).get("test") or not record.get(
            "executable"
        ):
            continue
        listed, _ = lister(record["executable"])
        listing.setdefault(_binary_label(record.get("target") or {}), set()).update(
            _terse(listed)
        )
    return listing


def build_failures(messages: str) -> list[str]:
    """Binary labels whose build reported an error."""

    failed = set()
    for line in messages.splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if record.get("reason") == "compiler-message" and (
            (record.get("message") or {}).get("level") == "error"
        ):
            failed.add(_binary_label(record.get("target") or {}))
    return sorted(failed)


def run_pairs(
    workspace: dict[str, set[str]],
    by_set: dict[tuple[str, ...], dict[str, set[str]]],
    excluded: set[str] | frozenset[str] = frozenset(),
) -> list[tuple[tuple[str, ...], dict[str, list[str]]]]:
    """Assign each test that is absent under the workspace features to the
    smallest matrix set that lists it; `excluded` holds `<label>::<test>` ids
    the gate must not run."""

    assigned: set[tuple[str, str]] = set()
    pairs: list[tuple[tuple[str, ...], dict[str, list[str]]]] = []
    for features in sorted(by_set, key=lambda f: (len(f), f)):
        chosen: dict[str, list[str]] = {}
        for label, tests in sorted(by_set[features].items()):
            new = sorted(
                test
                for test in tests - workspace.get(label, set())
                if (label, test) not in assigned and f"{label}::{test}" not in excluded
            )
            if new:
                chosen[label] = new
                assigned.update((label, test) for test in new)
        if chosen:
            pairs.append((features, chosen))
    return pairs


def _list_tests(
    crate: str, features: Sequence[str], env: dict[str, str], log: Path
) -> tuple[dict[str, set[str]], list[str]]:
    command = ["cargo", "build", "--locked", "-p", crate, "--no-default-features"]
    if features:
        command += ["--features", ",".join(features)]
    command += ["--tests", "--keep-going", "--message-format", "json"]
    completed = subprocess.run(
        command, cwd=REPO_ROOT, env=env, check=False, capture_output=True, text=True
    )
    log.write_text(completed.stderr)
    executables: list[str] = []

    def lister(executable: str) -> tuple[str, str]:
        executables.append(executable)
        run = subprocess.run(
            [executable, "--list", "--format", "terse"],
            cwd=REPO_ROOT,
            env=env,
            check=True,
            capture_output=True,
            text=True,
            timeout=300,
        )
        return run.stdout, ""

    try:
        listing = parse_listing(completed.stdout, lister)
    finally:
        for executable in executables:
            Path(executable).unlink(missing_ok=True)
    failed = build_failures(completed.stdout)
    if completed.returncode != 0 and not failed:
        raise FeatureCompleteError(
            f"listing build for {crate} {list(features)} failed; see {log}"
        )
    return listing, failed


def _nvidia_smi(command: list[str], env: dict[str, str]) -> str:
    return subprocess.run(
        command, env=env, check=True, capture_output=True, text=True
    ).stdout


def _warm_embedder(env: dict[str, str], log: Path) -> None:
    build = [
        "cargo",
        "build",
        "--locked",
        "-p",
        "fathomdb-cli",
        "--bin",
        CLI_BINARY,
        "--features",
        "default-embedder",
    ]
    subprocess.run(build, cwd=REPO_ROOT, env=env, check=True)
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline"],
            cwd=REPO_ROOT,
            env=env,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    target_dir = Path(metadata["target_directory"])
    completed = subprocess.run(
        [str(target_dir / "debug" / CLI_BINARY), "doctor", "warm-cache"],
        cwd=REPO_ROOT,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )
    log.write_text(completed.stdout + completed.stderr)
    if completed.returncode != 0:
        raise FeatureCompleteError(
            f"doctor warm-cache failed ({completed.returncode}); see {log}"
        )


def run_gate(scratch: Path) -> int:
    host = test_targets.host_triple()
    crates = test_targets.read_workspace(REPO_ROOT)
    matrix = test_targets.load_matrix(test_targets.MATRIX_PATH)
    allowlist = test_targets.load_allowlist(test_targets.ALLOWLIST_PATH)
    workspace = test_targets.workspace_features(REPO_ROOT, host)
    failures, _ = test_targets.check_coverage(
        crates, workspace, matrix, allowlist, host
    )
    if failures:
        for failure in failures:
            print(f"FAIL coverage: {failure}", file=sys.stderr)
        return 1
    if host not in ONNXRUNTIME_HOSTS:
        raise FeatureCompleteError(f"no pinned ONNX Runtime library for {host}")
    env = cuda_environment(dict(os.environ), CUDA_ROOT, _nvidia_smi)
    env = runner_environment(env, scratch)
    env.update(GATE_BUILD_ENV)
    Path(env["FATHOMDB_SLICE72_RECEIPT_DIR"]).mkdir(parents=True, exist_ok=True)
    _warm_embedder(env, scratch / "warm-cache.log")
    cache_root = Path(env.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    fetched = provision_weights(
        cache_root / "fathomdb" / "embedders" / "nomic-v1.5", NOMIC_FILES, _http_fetch
    )
    print(
        f"==> nomic-embed-text-v1.5 weights: fetched {fetched or 'none (cached)'}",
        flush=True,
    )
    assets = stage_slice72_assets(
        SLICE72_SUPPORT, cache_root, scratch / "slice72-assets"
    )
    tokenizer = assets / "bge" / "tokenizer.json"
    if _digest(tokenizer, "sha256") != BGE_TOKENIZER_SHA256:
        raise FeatureCompleteError(f"{tokenizer}: not the pinned bge-small tokenizer")
    library = provision_wheel_member(
        cache_root / "fathomdb" / "onnxruntime" / "1.26.0",
        ONNXRUNTIME_WHEEL,
        _http_fetch,
    )
    model = (
        cache_root
        / "fathomdb"
        / "embedders"
        / "onnx"
        / "bge-small-en-v1.5"
        / "model.onnx"
    )
    exported = provision_generated(
        model,
        ONNX_MODEL_SHA256,
        _export_onnx_model(cache_root / "fathomdb" / "onnx-export-venv"),
    )
    print(
        f"==> ONNX Runtime {library}; model {model} ({'exported' if exported else 'cached'})",
        flush=True,
    )
    env = onnx_environment(env, library, model, tokenizer)

    excluded = {
        str(entry["id"])
        for entry in allowlist
        if entry.get("exclude") and entry.get("class") == "opt-in-experiment"
    }
    by_name = {crate.name: crate for crate in crates}
    listed: set[str] = set()
    plan: list[tuple[str, tuple[str, ...], dict[str, list[str]]]] = []
    failures = []
    for crate_name in sorted({name for name, _ in matrix}):
        crate = by_name[crate_name]
        sets = sorted({features for name, features in matrix if name == crate_name})
        workspace_set = tuple(sorted(workspace.get(crate_name, frozenset())))
        print(
            f"==> listing {crate_name}: workspace {list(workspace_set)} + {len(sets)} set(s)",
            flush=True,
        )
        ws_listing, ws_failed = _list_tests(
            crate_name,
            workspace_set,
            env,
            scratch / f"list-{crate_name}--workspace.log",
        )
        failures.extend(f"{crate_name}::{label}: does not build" for label in ws_failed)
        by_set: dict[tuple[str, ...], dict[str, set[str]]] = {}
        for features in sets:
            listing, failed = _list_tests(
                crate_name,
                features,
                env,
                scratch / f"list-{crate_name}--{'+'.join(features) or 'none'}.log",
            )
            failures.extend(
                f"{crate_name}::{label}: does not build with {list(features)}"
                for label in failed
            )
            by_set[features] = listing
        for label_tests in [ws_listing, *by_set.values()]:
            for label, tests in label_tests.items():
                listed.update(f"{crate_name}::{label}::{test}" for test in tests)
        crate_excluded = {
            entry[len(crate_name) + 2 :]
            for entry in excluded
            if entry.startswith(f"{crate_name}::")
        }
        for features, chosen in run_pairs(ws_listing, by_set, crate_excluded):
            plan.append((crate_name, features, chosen))
    del crate

    results: list[ScanResult] = []
    summary: list[dict[str, object]] = []
    for package, features, chosen in plan:
        selectors = [arg for label in sorted(chosen) for arg in selector(label)]
        tests = sorted({test for names in chosen.values() for test in names})
        command = test_command(package, features, selectors, tests)
        label = f"{package}[{','.join(features) or 'none'}]"
        log = scratch / f"{package}--{'+'.join(features) or 'none'}.log"
        print(f"==> {label}: {len(tests)} test(s) in {sorted(chosen)}", flush=True)
        started = time.monotonic()
        with open(log, "w", encoding="utf-8") as handle:
            completed = subprocess.run(
                command,
                cwd=REPO_ROOT,
                env=entry_environment(env, list(chosen), assets),
                stdout=handle,
                stderr=subprocess.STDOUT,
                check=False,
            )
        seconds = round(time.monotonic() - started)
        result = scan_output(
            log.read_text(encoding="utf-8", errors="replace"), by_name[package]
        )
        if completed.returncode != 0:
            result.failed.append(
                f"{label}: cargo test exited {completed.returncode} (see {log})"
            )
        expected = {
            f"{package}::{lab}::{test}"
            for lab, names in chosen.items()
            for test in names
        }
        missing = sorted(expected - set(result.seen))
        result.failed.extend(f"{test}: did not run" for test in missing)
        results.append(result)
        summary.append(
            {
                "set": label,
                "binaries": sorted(chosen),
                "tests": sorted(expected),
                "exit": completed.returncode,
                "seconds": seconds,
                "markers": result.markers,
                "ignored": result.ignored,
                "log": str(log),
            }
        )
        print(f"    exit {completed.returncode} in {seconds}s; log {log}", flush=True)
    failures.extend(contract_failures(results, allowlist, listed))
    (scratch / "summary.json").write_text(
        json.dumps({"plan": summary, "failures": failures}, indent=2)
    )
    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(
        f"feature-complete: {len(plan)} run(s), "
        f"{sum(len(e['tests']) for e in summary)} test(s), {len(failures)} failure(s); "  # type: ignore[arg-type]
        f"summary {scratch / 'summary.json'}"
    )
    return 1 if failures else 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--scratch", type=Path, help="log directory (default: a new temporary one)"
    )
    args = parser.parse_args(argv)
    scratch = args.scratch or Path(
        tempfile.mkdtemp(prefix="fathomdb-feature-complete-")
    )
    scratch.mkdir(parents=True, exist_ok=True)
    try:
        return run_gate(scratch)
    except (
        FeatureCompleteError,
        test_targets.TestTargetsError,
        subprocess.CalledProcessError,
    ) as exc:
        print(f"feature-complete: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
