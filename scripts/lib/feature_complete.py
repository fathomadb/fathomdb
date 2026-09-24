#!/usr/bin/env python3
"""The feature-complete test gate behind `scripts/test-feature-complete.sh`.

Lists every test binary of each crate under the workspace gate's features,
under each target-derived feature set of the committed
`scripts/test-feature-matrix.toml`, and under the union of the crate's
host-buildable features. A test listed under the union but under no other set
(an item-level `#[cfg(feature = ...)]` on a feature no target requires) gets an
extra set: the first single feature, smallest closure first, that lists it,
else the union. The derived extra sets must equal the matrix's `[[extra]]`
sets (`--write-matrix` rewrites them). Every test that appears only under a
matrix or extra set runs once, under the smallest set that has it, and is
counted per test, never by summing libtest summaries (a test that re-executes
its own binary prints its child's). It runs with CUDA on the two RTX 3090s, real model
weights (embedder, reranker, nomic), and the ONNX Runtime assets, with
`FATHOMDB_REQUIRE_LIVE=1`, under which a test whose live prerequisite is
missing panics instead of skipping. Opt-in experiments are excluded, never
run. A skip marker in the output fails the gate unless
`scripts/test-skip-allowlist.toml` names its test as a `benign-message`, and
an `#[ignore]`d test fails it unless named as `ignored-by-design`; an
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
MARKER_CLASSES = frozenset({"benign-message"})
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
_CHILD_RUN = re.compile(r"^running \d+ tests?$")


class FeatureCompleteError(Exception):
    """A preflight or gate failure."""


class ScanResult:
    """Skip markers, ignored tests, and failures seen in one cargo run."""

    __slots__ = ("markers", "ignored", "failed", "ran_targets", "seen", "status")

    def __init__(self) -> None:
        self.markers: list[tuple[str, str]] = []
        self.ignored: list[str] = []
        self.failed: list[str] = []
        self.ran_targets: list[str] = []
        self.seen: list[str] = []
        # Final libtest status ("ok", "FAILED", "ignored") per test.
        self.status: dict[str, str] = {}


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


def scan_output(
    text: str,
    crate: test_targets.Crate,
    planned: set[str] | frozenset[str] | None = None,
) -> ScanResult:
    """Attribute every marker to the test whose `test <name> ...` line precedes
    it (the gate runs one test thread per binary). With `planned`, a bare
    status line (`--nocapture` prints it after the test's output) belongs to
    the last planned test still running.

    A test that re-executes its own binary prints its child's libtest run
    inside its own output. A `running N test(s)` line while a test is pending
    opens such a child scope, and the child's `test result:` line closes it.
    Inside it, test and status lines are the child's: they record no status,
    failure, or sighting, and any marker is the pending test's.

    The scoping fails closed, never open. A planned test that itself prints a
    bare `running N test(s)` line opens a scope that its own status line
    cannot close, and a child that crashes before its `test result:` line
    leaves its scope open until the next binary's `Running` line; either way
    the pending test is reported as `no result` and any later test of that
    binary as `did not run` (a false red or a misattribution, not a pass)."""

    result = ScanResult()
    target_id: str | None = None
    current: str | None = None
    pending: str | None = None
    depth = 0
    for line in text.splitlines():
        running = _RUNNING.match(line)
        if running is not None:
            target_id = f"{crate.name}::{_source_label(running.group(1), crate)}"
            result.ran_targets.append(target_id)
            current = pending = None
            depth = 0
            continue
        if target_id is None:
            # Build output before the first test binary is not a test's.
            continue
        if pending is not None and _CHILD_RUN.match(line.strip()):
            depth += 1
            continue
        if depth:
            if _RESULT.match(line):
                depth -= 1
                continue
            child_line = _TEST_LINE.match(line)
            rest = line if child_line is None else child_line.group(2)
            for pattern in SKIP_MARKERS:
                found = pattern.search(rest)
                if found is not None:
                    result.markers.append((pending or target_id, found.group(0)))
                    break
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
                result.status[current] = "ignored"
                continue
            if status == "FAILED":
                result.failed.append(current)
                result.status[current] = "FAILED"
                continue
            if status == "ok":
                result.status[current] = "ok"
                continue
            if planned is None or current in planned:
                pending = current
        elif line.strip() in ("ok", "FAILED"):
            owner = current if planned is None else pending
            if owner is not None:
                result.status[owner] = line.strip()
                if line.strip() == "FAILED":
                    result.failed.append(owner)
                pending = None if pending == owner else pending
                continue
            if line.strip() == "FAILED" and current is not None:
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
    """Apply the skip contract: failed tests, skip markers not allowlisted as
    `benign-message`, ignored tests not allowlisted as `ignored-by-design`,
    and stale allowlist entries each fail the gate. An `exclude` entry (an
    opt-in experiment the gate does not run) is used when `listed` contains
    it, and excuses nothing."""

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
            excused = any(entries[n]["class"] == "ignored-by-design" for n in hits)
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


def parent_counts(
    planned: set[str] | frozenset[str], result: ScanResult
) -> dict[str, int]:
    """Planned, passed, failed, and ignored counts over the planned tests only;
    a planned test with no final status (never ran, or cut off) is failed."""

    passed = sum(1 for test in planned if result.status.get(test) == "ok")
    ignored = sum(1 for test in planned if result.status.get(test) == "ignored")
    return {
        "planned": len(planned),
        "passed": passed,
        "failed": len(planned) - passed - ignored,
        "ignored": ignored,
    }


def run_failures(
    label: str,
    planned: set[str] | frozenset[str],
    result: ScanResult,
    returncode: int,
    log: Path,
) -> list[str]:
    """Failures of one cargo run beyond its test results: a non-zero exit,
    every planned test that never ran, and every one that started but has no
    final status."""

    failures = []
    if returncode != 0:
        failures.append(f"{label}: cargo test exited {returncode} (see {log})")
    seen = set(result.seen)
    failures.extend(f"{test}: did not run" for test in sorted(planned - seen))
    failures.extend(
        f"{test}: no result"
        for test in sorted(planned & seen)
        if test not in result.status
    )
    return failures


def listing_failures(
    crate: str, failed: Sequence[str], features: Sequence[str] | None
) -> list[str]:
    """One failure per test binary that did not build; `features` is None for
    the workspace features."""

    suffix = "" if features is None else f" with {list(features)}"
    return [f"{crate}::{label}: does not build{suffix}" for label in sorted(failed)]


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
    env["FATHOMDB_REQUIRE_LIVE"] = "1"
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
# Exact, hash-pinned export toolchain (and every transitive dependency); a
# fresh export with it reproduces ONNX_MODEL_SHA256 byte for byte.
ONNX_EXPORT_LOCK = REPO_ROOT / "dev/tools/onnx/export-requirements.txt"
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


def _export_onnx_model(
    venv: Path, run: Callable[..., object] = subprocess.run
) -> Callable[[Path], None]:
    def generate(destination: Path) -> None:
        python = venv / "bin" / "python"
        if not python.exists():
            run([sys.executable, "-m", "venv", str(venv)], check=True)
        run(
            [
                str(python),
                "-m",
                "pip",
                "install",
                "-q",
                "--require-hashes",
                "--no-deps",
                "-r",
                str(ONNX_EXPORT_LOCK),
            ],
            check=True,
        )
        # The export is written under a `.onnx` name beside `destination`
        # and is moved there or removed, never left behind.
        out = destination.with_suffix(".onnx")
        try:
            run(
                [str(python), str(ONNX_EXPORT_SCRIPT), "--out", str(out)],
                cwd=REPO_ROOT,
                check=True,
            )
            os.replace(out, destination)
        finally:
            out.unlink(missing_ok=True)

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
    # Every selected binary must run even when an earlier one fails.
    command += [*selectors, "--no-fail-fast"]
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


Listing = dict[str, set[str]]


def extra_candidates(
    crate: test_targets.Crate, buildable: Sequence[str]
) -> list[tuple[str, ...]]:
    """Single host-buildable features, smallest feature closure first."""

    return [
        (feature,)
        for feature in sorted(
            buildable,
            key=lambda f: (len(test_targets.feature_closure(crate, [f])), f),
        )
    ]


def derive_extra_sets(
    workspace: Listing,
    static: dict[tuple[str, ...], Listing],
    union: Listing,
    union_features: tuple[str, ...],
    candidates: Sequence[tuple[str, ...]],
    lister: Callable[[tuple[str, ...]], Listing],
) -> tuple[list[tuple[str, ...]], dict[tuple[str, ...], Listing]]:
    """The extra sets a crate needs: every test listed under the union of its
    host-buildable features but under neither the workspace features nor a
    target-derived set goes to the first candidate that lists it, else to the
    union itself. Returns the sets, in derivation order, and their listings.
    Target-derived sets are never listed again."""

    known: set[tuple[str, str]] = {
        (label, test) for label, tests in workspace.items() for test in tests
    }
    for listing in static.values():
        known.update(
            (label, test) for label, tests in listing.items() for test in tests
        )
    uncovered = {
        (label, test)
        for label, tests in union.items()
        for test in tests
        if (label, test) not in known
    }
    derived: list[tuple[str, ...]] = []
    listings: dict[tuple[str, ...], Listing] = {}
    for features in candidates:
        if not uncovered:
            break
        if features in static:
            continue
        listing = lister(features)
        hit = {
            (label, test)
            for label, tests in listing.items()
            for test in tests
            if (label, test) in uncovered
        }
        if hit:
            derived.append(features)
            listings[features] = listing
            uncovered -= hit
    if uncovered:
        derived.append(union_features)
        listings[union_features] = union
    return derived, listings


def extra_set_drift(
    crate: str,
    derived: Sequence[tuple[str, ...]],
    committed: Sequence[tuple[str, ...]],
) -> list[str]:
    """Differences between the extra sets derived now and the committed ones."""

    added = sorted(set(derived) - set(committed))
    removed = sorted(set(committed) - set(derived))
    if not added and not removed:
        return []
    return [
        f"extra-set drift in {crate} (run `scripts/test-feature-complete.sh "
        f"--write-matrix`): derived-only {[list(f) for f in added]}, "
        f"matrix-only {[list(f) for f in removed]}"
    ]


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


Plan = list[tuple[str, tuple[str, ...], dict[str, list[str]]]]


def plan_runs(
    crates: Sequence[test_targets.Crate],
    workspace: dict[str, frozenset[str]],
    matrix: Sequence[tuple[str, tuple[str, ...]]],
    extras: Sequence[tuple[str, tuple[str, ...]]],
    excluded: set[str],
    host: str,
    env: dict[str, str],
    scratch: Path,
    list_tests: Callable[
        [str, Sequence[str], dict[str, str], Path], tuple[Listing, list[str]]
    ] = _list_tests,
) -> tuple[Plan, set[str], list[tuple[str, tuple[str, ...]]], list[str]]:
    """List each crate's test binaries (with `list_tests`) under its workspace
    features, its target-derived sets, and the union of its host-buildable
    features, derive its extra sets, and assign every test the workspace gate
    cannot run to the smallest set that lists it. Returns the plan, every
    listed test id, the derived extra sets, and failures (build failures and
    extra-set drift)."""

    plan: Plan = []
    listed: set[str] = set()
    derived_extras: list[tuple[str, tuple[str, ...]]] = []
    failures: list[str] = []
    for crate in crates:
        name = crate.name
        sets = sorted(
            {features for crate_name, features in matrix if crate_name == name}
        )
        committed = sorted(f for crate_name, f in extras if crate_name == name)
        workspace_set = tuple(sorted(workspace.get(name, frozenset())))
        workspace_closure = test_targets.feature_closure(crate, workspace_set)
        buildable = test_targets.host_buildable_features(crates, name, host)
        union_needed = not test_targets.feature_closure(crate, buildable) <= (
            workspace_closure
        )
        if not sets and not union_needed and not committed:
            continue
        print(
            f"==> listing {name}: workspace {list(workspace_set)} + {len(sets)} set(s)"
            f"{f' + union {list(buildable)}' if union_needed else ''}",
            flush=True,
        )
        cache: dict[tuple[str, ...], Listing] = {}

        def lister(features: tuple[str, ...], name: str = name) -> Listing:
            if features not in cache:
                listing, failed = list_tests(
                    name,
                    features,
                    env,
                    scratch / f"list-{name}--{'+'.join(features) or 'none'}.log",
                )
                failures.extend(listing_failures(name, failed, features))
                cache[features] = listing
            return cache[features]

        ws_listing, ws_failed = list_tests(
            name, workspace_set, env, scratch / f"list-{name}--workspace.log"
        )
        failures.extend(listing_failures(name, ws_failed, None))
        by_set = {features: lister(features) for features in sets}
        derived: list[tuple[str, ...]] = []
        if union_needed:
            union = lister(tuple(buildable))
            derived, found = derive_extra_sets(
                ws_listing,
                by_set,
                union,
                tuple(buildable),
                extra_candidates(crate, buildable),
                lister,
            )
            by_set.update(found)
        failures.extend(extra_set_drift(name, derived, committed))
        derived_extras.extend((name, features) for features in derived)
        for label_tests in [ws_listing, *cache.values()]:
            for label, tests in label_tests.items():
                listed.update(f"{name}::{label}::{test}" for test in tests)
        crate_excluded = {
            entry[len(name) + 2 :]
            for entry in excluded
            if entry.startswith(f"{name}::")
        }
        for features, chosen in run_pairs(ws_listing, by_set, crate_excluded):
            plan.append((name, features, chosen))
    return plan, listed, derived_extras, failures


def run_gate(scratch: Path, write_matrix: bool = False) -> int:
    host = test_targets.host_triple()
    crates = test_targets.read_workspace(REPO_ROOT)
    matrix = test_targets.load_matrix(test_targets.MATRIX_PATH)
    extras = test_targets.load_extra_sets(test_targets.MATRIX_PATH)
    allowlist = test_targets.load_allowlist(test_targets.ALLOWLIST_PATH)
    workspace = test_targets.workspace_features(REPO_ROOT, host)
    failures, _ = test_targets.check_coverage(
        crates, workspace, matrix, allowlist, host, extras
    )
    if failures and not write_matrix:
        for failure in failures:
            print(f"FAIL coverage: {failure}", file=sys.stderr)
        return 1
    if host not in ONNXRUNTIME_HOSTS:
        raise FeatureCompleteError(f"no pinned ONNX Runtime library for {host}")
    env = cuda_environment(dict(os.environ), CUDA_ROOT, _nvidia_smi)
    env = runner_environment(env, scratch)
    env.update(GATE_BUILD_ENV)
    excluded = {
        str(entry["id"])
        for entry in allowlist
        if entry.get("exclude") and entry.get("class") == "opt-in-experiment"
    }
    if write_matrix:
        _, _, derived, failures = plan_runs(
            crates, workspace, matrix, extras, excluded, host, env, scratch
        )
        building = [f for f in failures if "does not build" in f]
        for failure in building:
            print(f"FAIL {failure}", file=sys.stderr)
        if building:
            return 1
        test_targets.MATRIX_PATH.write_text(
            test_targets.render_matrix(test_targets.derive_matrix(crates), derived),
            encoding="utf-8",
        )
        print(f"wrote {test_targets.MATRIX_PATH} ({len(derived)} extra set(s))")
        return 0
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

    by_name = {crate.name: crate for crate in crates}
    plan, listed, _, failures = plan_runs(
        crates, workspace, matrix, extras, excluded, host, env, scratch
    )

    results: list[ScanResult] = []
    summary: list[dict[str, object]] = []
    totals = {"planned": 0, "passed": 0, "failed": 0, "ignored": 0}
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
        expected = {
            f"{package}::{lab}::{test}"
            for lab, names in chosen.items()
            for test in names
        }
        result = scan_output(
            log.read_text(encoding="utf-8", errors="replace"),
            by_name[package],
            expected,
        )
        failures.extend(
            run_failures(label, expected, result, completed.returncode, log)
        )
        results.append(result)
        counts = parent_counts(expected, result)
        for key in totals:
            totals[key] += counts[key]
        summary.append(
            {
                "set": label,
                "binaries": sorted(chosen),
                "tests": sorted(expected),
                "counts": counts,
                "exit": completed.returncode,
                "seconds": seconds,
                "markers": result.markers,
                "ignored": result.ignored,
                "log": str(log),
            }
        )
        print(
            f"    exit {completed.returncode} in {seconds}s: {counts['passed']} passed, "
            f"{counts['failed']} failed, {counts['ignored']} ignored; log {log}",
            flush=True,
        )
    failures.extend(contract_failures(results, allowlist, listed))
    (scratch / "summary.json").write_text(
        json.dumps({"plan": summary, "totals": totals, "failures": failures}, indent=2)
    )
    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    print(
        f"feature-complete: {len(plan)} run(s), {totals['planned']} planned test(s): "
        f"{totals['passed']} passed, {totals['failed']} failed, "
        f"{totals['ignored']} ignored; {len(failures)} failure(s); "
        f"summary {scratch / 'summary.json'}"
    )
    return 1 if failures else 0


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--scratch", type=Path, help="log directory (default: a new temporary one)"
    )
    parser.add_argument(
        "--write-matrix",
        action="store_true",
        help="list tests, rewrite the matrix's derived extra sets, and run nothing",
    )
    args = parser.parse_args(argv)
    scratch = args.scratch or Path(
        tempfile.mkdtemp(prefix="fathomdb-feature-complete-")
    )
    scratch.mkdir(parents=True, exist_ok=True)
    try:
        return run_gate(scratch, args.write_matrix)
    except (
        FeatureCompleteError,
        test_targets.TestTargetsError,
        subprocess.CalledProcessError,
    ) as exc:
        print(f"feature-complete: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
