#!/usr/bin/env python3
"""Fast-tier self-tests for the permanent test-target coverage pieces.

Covers `scripts/lib/test_targets.py` (the shared requirement reader),
`scripts/check-test-target-coverage.py` (RH-15), and the skip contract and
CUDA preflight of `scripts/test-feature-complete.sh` (RH-14). No build, no
network, no GPU.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
from types import ModuleType
from typing import Any, Callable


ROOT = Path(__file__).resolve().parents[2]
FIXTURE_CRATE = ROOT / "scripts" / "tests" / "fixtures" / "hidden-surface" / "crate"
HOST = "x86_64-unknown-linux-gnu"


def load(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def expect_error(error: type, call: Callable[[], Any], fragment: str) -> None:
    try:
        call()
    except error as exc:
        assert fragment in str(exc), (fragment, str(exc))
        return
    raise AssertionError(f"expected {error.__name__} containing {fragment!r}")


def copy_fixture(directory: Path) -> Path:
    root = directory / "crate"
    shutil.copytree(FIXTURE_CRATE, root)
    return root


def test_cfg_parsing(tt: ModuleType) -> None:
    ast = tt.parse_cfg(
        'all(feature = "hooks", not(any(windows, target_os = "macos")), test)'
    )
    assert tt.canonical_cfg(ast) == (
        'all(feature = "hooks", not(target_os = "macos"), not(windows), test)'
    )
    assert tt.canonical_cfg(tt.parse_cfg("any(unix, not(unix))")) is None
    for bad in ("feature = hooks", "all(unix", "all(unix) trailing", "any(,)"):
        expect_error(tt.TestTargetsError, lambda bad=bad: tt.parse_cfg(bad), "cfg")
    linux = tt.host_assignment(HOST)
    assert linux(("atom", "unix", None)) is True
    assert linux(("atom", "target_os", "linux")) is True
    assert linux(("atom", "target_arch", "aarch64")) is False
    assert linux(("atom", "windows", None)) is False
    expect_error(
        tt.TestTargetsError, lambda: linux(("atom", "sanitize", "address")), "sanitize"
    )


def test_read_fixture_workspace(tt: ModuleType) -> None:
    (crate,) = tt.read_workspace(FIXTURE_CRATE)
    assert crate.name == "hs_fixture"
    summary = {t.name: (t.source, t.required_features, t.cfg) for t in crate.targets}
    assert summary == {
        "broken": ("tests/broken.rs", (), None),
        "extra": ("tests/extra.rs", (), 'all(feature = "extra", feature = "hooks")'),
        "hooked": ("tests/hooked.rs", (), 'feature = "hooks"'),
        "platform": ("tests/platform.rs", (), "unix"),
        "plain": ("tests/plain.rs", (), None),
        "required": ("tests/required.rs", ("hooks",), None),
    }, summary
    assert tt.feature_closure(crate, ("extra",)) == frozenset({"extra", "hooks"})
    requirements = {t.name: tt.requirement(crate, t) for t in crate.targets}
    assert requirements == {
        "broken": (),
        "extra": ("extra",),
        "hooked": ("hooks",),
        "platform": (),
        "plain": (),
        "required": ("hooks",),
    }
    assert tt.derive_matrix([crate]) == [
        ("hs_fixture", ()),
        ("hs_fixture", ("extra",)),
        ("hs_fixture", ("hooks",)),
    ]


def test_matrix_round_trip(tt: ModuleType) -> None:
    entries = [("b", ("x", "y")), ("a", ())]
    text = tt.render_matrix(entries)
    assert "do not edit" in text and "--write-matrix" in text
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "matrix.toml"
        path.write_text(text)
        assert tt.load_matrix(path) == [("a", ()), ("b", ("x", "y"))]
    assert tt.render_matrix(entries) == tt.render_matrix(list(reversed(entries)))


def test_coverage_check(tt: ModuleType) -> None:
    crates = tt.read_workspace(FIXTURE_CRATE)
    complete = tt.derive_matrix(crates)
    workspace = {"hs_fixture": frozenset()}
    failures, gate_only = tt.check_coverage(crates, workspace, complete, [], HOST)
    assert failures == [], failures
    assert gate_only == [
        "hs_fixture::extra",
        "hs_fixture::hooked",
        "hs_fixture::required",
    ]

    # An uncovered required-features target and an uncovered file-level cfg
    # target are named; the missing matrix entry is also drift.
    without_hooks = [entry for entry in complete if entry[1] != ("hooks",)]
    failures, _ = tt.check_coverage(crates, workspace, without_hooks, [], HOST)
    text = "\n".join(failures)
    assert "hs_fixture::required" in text and "hs_fixture::hooked" in text, text
    assert "drift" in text, text
    without_extra = [entry for entry in complete if entry[1] != ("extra",)]
    failures, _ = tt.check_coverage(crates, workspace, without_extra, [], HOST)
    assert any("hs_fixture::extra" in f and "not covered" in f for f in failures), (
        failures
    )

    # Matrix drift alone.
    failures, _ = tt.check_coverage(
        crates, workspace, complete + [("hs_fixture", ("bogus",))], [], HOST
    )
    assert len(failures) == 1 and "drift" in failures[0], failures

    # Stale allowlist entries: a target that does not exist, and one the
    # feature-complete gate does not run. (A single test in a workspace-run
    # target may be item-gated and run by the gate, so only the runtime skip
    # contract can call it stale.)
    for entry in (
        {"id": "hs_fixture::gone", "class": "ignored-by-design", "reason": "r"},
        {"id": "hs_fixture::plain", "class": "ignored-by-design", "reason": "r"},
    ):
        failures, _ = tt.check_coverage(crates, workspace, complete, [entry], HOST)
        assert len(failures) == 1 and "stale" in failures[0], (entry, failures)
    single = {
        "id": "hs_fixture::plain::runs",
        "class": "ignored-by-design",
        "reason": "r",
    }
    assert tt.check_coverage(crates, workspace, complete, [single], HOST)[0] == []
    ok_entry = {
        "id": "hs_fixture::hooked::hooked",
        "class": "opt-in-experiment",
        "reason": "r",
    }
    assert tt.check_coverage(crates, workspace, complete, [ok_entry], HOST)[0] == []
    for malformed in (
        {"id": "hs_fixture::hooked", "class": "because", "reason": "r"},
        {"id": "hs_fixture::hooked", "class": "opt-in-experiment", "reason": ""},
    ):
        failures, _ = tt.check_coverage(crates, workspace, complete, [malformed], HOST)
        assert failures, malformed

    # A target no gate can run on a host needs a platform-excluded entry whose
    # `excluded_on` predicate names the hosts it applies to; elsewhere the entry
    # is inert, and it is stale only on a matching host that covers the target.
    windows = "x86_64-pc-windows-msvc"
    failures, _ = tt.check_coverage(crates, workspace, complete, [], windows)
    assert any("hs_fixture::platform" in f for f in failures), failures
    excluded = {
        "id": "hs_fixture::platform",
        "class": "platform-excluded",
        "excluded_on": "not(unix)",
        "reason": "unix only",
    }
    for host in (HOST, windows, "aarch64-unknown-linux-gnu", "aarch64-apple-darwin"):
        assert (
            tt.check_coverage(crates, workspace, complete, [excluded], host)[0] == []
        ), host
    wrong_host = dict(excluded, excluded_on="unix")
    failures, _ = tt.check_coverage(crates, workspace, complete, [wrong_host], HOST)
    assert len(failures) == 1 and "stale" in failures[0], failures
    missing = {k: v for k, v in excluded.items() if k != "excluded_on"}
    failures, _ = tt.check_coverage(crates, workspace, complete, [missing], windows)
    assert any("excluded_on" in f for f in failures), failures

    # Unknown cfg atoms in a target's file cfg fail closed.
    with tempfile.TemporaryDirectory() as directory:
        root = copy_fixture(Path(directory))
        (root / "tests" / "odd.rs").write_text('#![cfg(sanitize = "address")]\n')
        expect_error(
            tt.TestTargetsError,
            lambda: tt.derive_matrix(tt.read_workspace(root)),
            "sanitize",
        )

    with tempfile.TemporaryDirectory() as directory:
        root = copy_fixture(Path(directory))
        (root / "tests" / "new_gated.rs").write_text(
            '#![cfg(feature = "extra")]\n\n#[test]\nfn new_gated() {}\n'
        )
        grown = tt.read_workspace(root)
        failures, _ = tt.check_coverage(grown, workspace, complete, [], HOST)
        assert failures == [], failures
        (root / "tests" / "needs_other.rs").write_text(
            '#![cfg(all(feature = "hooks", not(feature = "extra")))]\n\n#[test]\nfn x() {}\n'
        )
        failures, _ = tt.check_coverage(
            tt.read_workspace(root), workspace, complete, [], HOST
        )
        assert failures == [], failures


def test_run_pairs(fc: ModuleType) -> None:
    """Tests present under a matrix set but not under the workspace features
    (whole targets or item-level `#[cfg(feature = ...)]` tests) are run once,
    under the smallest set that has them; excluded tests are not run."""

    recorded = json.loads(
        (
            ROOT
            / "scripts/tests/fixtures/hidden-surface/json/inventory-base-hooks.json"
        ).read_text()
    )
    messages = "".join(json.dumps(m) + "\n" for m in recorded["messages"])

    def lister(executable: str) -> tuple[str, str]:
        listing = recorded["listings"][executable]
        return listing["list"], listing["ignored"]

    hooks = fc.parse_listing(messages, lister)
    assert hooks == {
        "lib": {"tests::unit_in_lib"},
        "extra": set(),
        "hooked": {"hooked"},
        "plain": {"ignored_by_design", "runs", "second"},
        "platform": {"on_unix"},
        "required": {"required"},
    }, hooks
    workspace = {
        "lib": {"tests::unit_in_lib"},
        "plain": {"ignored_by_design", "runs"},
        "platform": {"on_unix"},
    }
    both = {label: set(tests) for label, tests in hooks.items()}
    both["plain"] = both["plain"] | {"only_with_both"}
    pairs = fc.run_pairs(
        workspace,
        {("hooks",): hooks, ("extra", "hooks"): both},
        excluded={"required::required"},
    )
    assert pairs == [
        (("hooks",), {"hooked": ["hooked"], "plain": ["second"]}),
        (("extra", "hooks"), {"plain": ["only_with_both"]}),
    ], pairs
    assert fc.selector("lib") == ["--lib"]
    assert fc.selector("bin:fathomdb") == ["--bin", "fathomdb"]
    assert fc.selector("plain") == ["--test", "plain"]


def feature_complete_output() -> str:
    return (
        "warning: skipping an unused dependency during the build\n"
        "     Running unittests src/lib.rs (target/debug/deps/hs_fixture-89ab)\n"
        "\n"
        "running 1 test\n"
        "test tests::unit_in_lib ... ok\n"
        "\n"
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
        "\n"
        "     Running tests/plain.rs (target/debug/deps/plain-0123)\n"
        "\n"
        "running 3 tests\n"
        "test ignored_by_design ... ignored, ignored by design\n"
        "test runs ... [SKIP] no GPU visible\n"
        "ok\n"
        "test second ... ok\n"
        "\n"
        "test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out\n"
        "\n"
        "     Running tests/hooked.rs (target/debug/deps/hooked-4567)\n"
        "\n"
        "running 1 test\n"
        "test hooked ... \n"
        "note: skipping IR measurements JSON write\n"
        "ok\n"
        "\n"
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
    )


def test_skip_contract(fc: ModuleType, tt: ModuleType) -> None:
    (crate,) = tt.read_workspace(FIXTURE_CRATE)
    result = fc.scan_output(feature_complete_output(), crate)
    assert result.markers == [
        ("hs_fixture::plain::runs", "SKIP"),
        ("hs_fixture::hooked::hooked", "skipping"),
    ], result.markers
    assert result.ignored == ["hs_fixture::plain::ignored_by_design"]
    assert result.ran_targets == [
        "hs_fixture::lib",
        "hs_fixture::plain",
        "hs_fixture::hooked",
    ]
    allowlist = [
        {
            "id": "hs_fixture::plain::ignored_by_design",
            "class": "ignored-by-design",
            "reason": "r",
        },
        {"id": "hs_fixture::plain::runs", "class": "opt-in-experiment", "reason": "r"},
        {"id": "hs_fixture::hooked::hooked", "class": "benign-message", "reason": "r"},
    ]
    assert fc.contract_failures([result], allowlist) == []
    # Each missing entry fails the gate.
    for dropped in range(3):
        partial = [entry for n, entry in enumerate(allowlist) if n != dropped]
        failures = fc.contract_failures([result], partial)
        assert len(failures) == 1, (dropped, failures)
    # A stale entry fails the gate.
    stale = allowlist + [
        {"id": "hs_fixture::plain::second", "class": "ignored-by-design", "reason": "r"}
    ]
    failures = fc.contract_failures([result], stale)
    assert len(failures) == 1 and "stale" in failures[0], failures
    # A target-level entry covers every test of that target.
    target_level = [
        {"id": "hs_fixture::plain", "class": "opt-in-experiment", "reason": "r"},
        {"id": "hs_fixture::hooked::hooked", "class": "benign-message", "reason": "r"},
    ]
    assert fc.contract_failures([result], target_level) == []
    # An ignored test is only excused by ignored-by-design (or a target entry).
    wrong_class = [dict(allowlist[0], **{"class": "benign-message"})] + allowlist[1:]
    assert len(fc.contract_failures([result], wrong_class)) == 1
    # A skip marker is only excused by opt-in-experiment or benign-message.
    marker_as_ignored = (
        allowlist[:1]
        + [dict(allowlist[1], **{"class": "ignored-by-design"})]
        + allowlist[2:]
    )
    failures = fc.contract_failures([result], marker_as_ignored)
    assert len(failures) == 1 and "hs_fixture::plain::runs" in failures[0], failures
    # An excluded test is used when it is listed, and stale when it is not.
    excluded = {
        "id": "hs_fixture::plain::gone_measurement",
        "class": "opt-in-experiment",
        "exclude": True,
        "reason": "r",
    }
    failures = fc.contract_failures([result], allowlist + [excluded])
    assert len(failures) == 1 and "stale" in failures[0], failures
    failures = fc.contract_failures(
        [result], allowlist + [excluded], listed={"hs_fixture::plain::gone_measurement"}
    )
    assert failures == [], failures
    # A failing test run is reported.
    failed = fc.scan_output(
        feature_complete_output().replace(
            "test second ... ok", "test second ... FAILED"
        ),
        crate,
    )
    assert any(
        "FAILED" in f or "failed" in f
        for f in fc.contract_failures([failed], allowlist)
    )


def test_skip_markers(fc: ModuleType, tt: ModuleType) -> None:
    """Every self-skip phrasing found in gate-run targets is detected: word
    boundary, case-insensitive `skip` forms and the `PENDING` markers."""

    (crate,) = tt.read_workspace(FIXTURE_CRATE)
    phrasings = [
        "SKIP candle_onnx_equivalence_measurement: set ORT_DYLIB_PATH + FATHOMDB_ONNX_MODEL_PATH",
        "SKIP cpu_legs_reproduce_0816_baseline: ONNX asset env unset — set ORT_DYLIB_PATH",
        "SKIP calibration_reports_p1_flips_and_p2_l2: ONNX asset env unset",
        "R-CAL-4 candle-CUDA leg gated-to-skip (effective=cpu) — recorded PENDING (run on MAIN)",
        "EU_DUMP not set; skipping candle dump diagnostic",
        "[skip] AGENT_LONG not set; PR-9 micro-benchmark is opt-in",
        "[SKIP] CE model unavailable — pool_n bound needs the cached reranker",
        "PENDING_EXTERNAL Slice 72 requires immutable FATHOMDB_SLICE72_ASSET_ROOT caches",
        "Skipped: nothing to measure",
    ]
    for phrase in phrasings:
        text = (
            "     Running tests/plain.rs (target/debug/deps/plain-0123)\n"
            f"test runs ... {phrase}\nok\n"
        )
        markers = fc.scan_output(text, crate).markers
        assert [owner for owner, _ in markers] == ["hs_fixture::plain::runs"], (
            phrase,
            markers,
        )
    for benign in (
        "test harness_skips_unavailable_backends_cleanly ... ok",
        "note: skipjack and pendingly are not markers",
        "pending writes flushed",
    ):
        text = (
            "     Running tests/plain.rs (target/debug/deps/plain-0123)\n"
            + benign
            + "\n"
        )
        assert fc.scan_output(text, crate).markers == [], benign


def test_onnx_provisioning(fc: ModuleType) -> None:
    import hashlib
    import zipfile

    with tempfile.TemporaryDirectory() as directory:
        base = Path(directory)
        wheel = base / "ort.whl"
        with zipfile.ZipFile(wheel, "w") as archive:
            archive.writestr("onnxruntime/capi/libonnxruntime.so.9", b"native")
        member = hashlib.sha256(b"native").hexdigest()
        spec = {
            "url": "u/ort.whl",
            "sha256": hashlib.sha256(wheel.read_bytes()).hexdigest(),
            "member": "onnxruntime/capi/libonnxruntime.so.9",
            "member_sha256": member,
        }

        def fetch(url: str, destination: Path) -> None:
            assert url == "u/ort.whl"
            destination.write_bytes(wheel.read_bytes())

        library = fc.provision_wheel_member(base / "cache", spec, fetch)
        assert (
            library.read_bytes() == b"native" and library.name == "libonnxruntime.so.9"
        )
        # Present and valid: no second fetch.
        assert (
            fc.provision_wheel_member(base / "cache", spec, lambda *_: 1 / 0) == library
        )
        bad = dict(spec, member_sha256="0" * 64)
        library.unlink()
        expect_error(
            fc.FeatureCompleteError,
            lambda: fc.provision_wheel_member(base / "cache", bad, fetch),
            "sha256",
        )
        # A wheel that is not the pinned wheel is rejected before extraction,
        # and neither the download nor a partial member is left behind.
        wrong_wheel = dict(spec, sha256="0" * 64)
        expect_error(
            fc.FeatureCompleteError,
            lambda: fc.provision_wheel_member(base / "cache", wrong_wheel, fetch),
            "wheel sha256",
        )
        assert sorted(p.name for p in (base / "cache").iterdir()) == []

        model = base / "onnx" / "model.onnx"
        exported: list[Path] = []

        def exporter(destination: Path) -> None:
            exported.append(destination)
            destination.write_bytes(b"graph")

        digest = hashlib.sha256(b"graph").hexdigest()
        assert fc.provision_generated(model, digest, exporter) is True
        assert fc.provision_generated(model, digest, exporter) is False
        assert len(exported) == 1
        model.unlink()
        expect_error(
            fc.FeatureCompleteError,
            lambda: fc.provision_generated(model, "0" * 64, exporter),
            "sha256",
        )
        assert not model.exists()
        assert sorted(p.name for p in model.parent.iterdir()) == []

        def failing(destination: Path) -> None:
            destination.write_bytes(b"half")
            raise RuntimeError("export died")

        try:
            fc.provision_generated(model, digest, failing)
        except RuntimeError:
            pass
        else:
            raise AssertionError("a failing generator must propagate")
        assert sorted(p.name for p in model.parent.iterdir()) == []

        # The export venv installs only the hash-pinned lock, and the
        # exporter's own `.onnx` output never outlives the generator.
        commands: list[list[str]] = []
        fail_export = [False]

        def run(command: list[str], **_: object) -> None:
            commands.append([str(part) for part in command])
            if "--out" in command:
                out = Path(command[command.index("--out") + 1])
                out.write_bytes(b"graph")
                if fail_export[0]:
                    raise fc.subprocess.CalledProcessError(1, command)

        venv = base / "venv"
        (venv / "bin").mkdir(parents=True)
        (venv / "bin" / "python").write_text("")
        destination = base / "gen" / "model.onnx.partial"
        destination.parent.mkdir()
        fc._export_onnx_model(venv, run)(destination)
        assert destination.read_bytes() == b"graph"
        assert sorted(p.name for p in destination.parent.iterdir()) == [
            destination.name
        ]
        (install,) = [c for c in commands if "install" in c]
        assert "--require-hashes" in install and "--no-deps" in install, install
        assert install[install.index("-r") + 1] == str(fc.ONNX_EXPORT_LOCK)
        destination.unlink()
        fail_export[0] = True
        try:
            fc._export_onnx_model(venv, run)(destination)
        except fc.subprocess.CalledProcessError:
            pass
        else:
            raise AssertionError("a failing export must propagate")
        assert sorted(p.name for p in destination.parent.iterdir()) == []

        env = fc.onnx_environment({"A": "1"}, library, model, base / "tok.json")
        assert env["ORT_DYLIB_PATH"] == str(library)
        assert env["FATHOMDB_ONNX_MODEL_PATH"] == str(model)
        assert env["FATHOMDB_ONNX_TOKENIZER_PATH"] == str(base / "tok.json")
    assert fc.ONNXRUNTIME_WHEEL["sha256"] and fc.ONNX_MODEL_SHA256

    # Every export requirement is an exact, hash-pinned version.
    import re

    lock = fc.ONNX_EXPORT_LOCK.read_text().splitlines()
    pins = [line for line in lock if line and not line.startswith(("#", "--"))]
    assert pins, lock
    for line in pins:
        assert re.fullmatch(
            r"[A-Za-z0-9_.-]+==[A-Za-z0-9_.+-]+ --hash=sha256:[0-9a-f]{64}", line
        ), line
    names = {line.split("==")[0].lower() for line in pins}
    assert {"torch", "transformers", "numpy", "onnx"} <= names, names


def test_canonical_properties(tt: ModuleType) -> None:
    """canonical_cfg is equivalent, idempotent, and the minimal sum of products
    (fewest terms, then literals, then lexicographic) over all implicants."""

    from itertools import combinations, product

    from hypothesis import given, settings, strategies as st

    names = ["a", "b", "c"]
    leaves = st.sampled_from(names)
    exprs = st.recursive(
        leaves,
        lambda inner: st.one_of(
            inner.map(lambda e: f"not({e})"),
            st.lists(inner, min_size=1, max_size=3).map(
                lambda xs: f"any({', '.join(xs)})"
            ),
            st.lists(inner, min_size=1, max_size=3).map(
                lambda xs: f"all({', '.join(xs)})"
            ),
        ),
        max_leaves=6,
    )

    def table(text: str) -> tuple[bool, ...]:
        node = tt.parse_cfg(text)
        return tuple(
            tt.evaluate(node, lambda atom, row=row: row[names.index(atom[1])])
            for row in product([False, True], repeat=3)
        )

    terms = [
        term
        for term in product([None, True, False], repeat=3)
        if any(value is not None for value in term)
    ]

    def render(term: tuple) -> str:
        parts = sorted(
            names[i] if value else f"not({names[i]})"
            for i, value in enumerate(term)
            if value is not None
        )
        return parts[0] if len(parts) == 1 else f"all({', '.join(parts)})"

    def brute_force(truth: tuple[bool, ...]) -> str:
        rows = list(product([False, True], repeat=3))
        want = {row for row, value in zip(rows, truth) if value}
        for size in range(1, 5):
            best = None
            for chosen in combinations(terms, size):
                covered = {
                    row
                    for row in rows
                    for term in chosen
                    if all(v is None or row[i] == v for i, v in enumerate(term))
                }
                if covered == want:
                    rendered = sorted(render(term) for term in chosen)
                    text = rendered[0] if size == 1 else f"any({', '.join(rendered)})"
                    literals = sum(sum(v is not None for v in term) for term in chosen)
                    key = (literals, text)
                    best = key if best is None or key < best else best
            if best is not None:
                return best[1]
        raise AssertionError("no cover of four terms")

    @settings(max_examples=150, deadline=None, derandomize=True)
    @given(exprs)
    def check(expr: str) -> None:
        canonical = tt.canonical_cfg_text(expr)
        truth = table(expr)
        if canonical is None:
            assert all(truth)
            return
        if canonical == "any()":
            assert not any(truth)
            return
        assert table(canonical) == truth, (expr, canonical)
        assert tt.canonical_cfg_text(canonical) == canonical
        if sum(truth) <= 6:
            assert canonical == brute_force(truth), (expr, canonical)

    check()
    # Ties between equally small covers resolve to the lexicographically
    # smallest rendering. The cyclic function ¬a¬b + b¬c + ac has two minimal
    # forms of three terms and six literals; the other is ab + ¬bc + ¬a¬c.
    assert (
        tt.canonical_cfg_text("any(all(not(a), not(b)), all(b, not(c)), all(a, c))")
        == "any(all(a, b), all(c, not(b)), all(not(a), not(c)))"
    )
    too_many = "any(" + ", ".join(f"x{n:02}" for n in range(13)) + ")"
    expect_error(tt.TestTargetsError, lambda: tt.canonical_cfg_text(too_many), "12")


def test_cuda_preflight(fc: ModuleType) -> None:
    good = (
        "0, 00000000:41:00.0, NVIDIA GeForce RTX 3090\n"
        "1, 00000000:42:00.0, NVIDIA GeForce RTX 3090\n"
        "2, 00000000:61:00.0, Quadro K620\n"
    )

    def runner(output: str) -> Callable[[list[str], dict[str, str]], str]:
        def run(command: list[str], env: dict[str, str]) -> str:
            assert env["CUDA_DEVICE_ORDER"] == "PCI_BUS_ID"
            return output

        return run

    with tempfile.TemporaryDirectory() as directory:
        cuda = Path(directory) / "cuda"
        (cuda / "bin").mkdir(parents=True)
        (cuda / "lib64").mkdir()
        nvcc = cuda / "bin" / "nvcc"
        nvcc.write_text("#!/bin/sh\n")
        nvcc.chmod(0o755)
        env = fc.cuda_environment({"PATH": "/usr/bin"}, cuda, runner(good))
        assert env["CUDA_VISIBLE_DEVICES"] == "0,1"
        assert env["CUDA_DEVICE_ORDER"] == "PCI_BUS_ID"
        assert env["CUDA_ROOT"] == env["CUDA_PATH"] == str(cuda)
        assert env["PATH"].split(":")[0] == str(cuda / "bin")
        assert str(cuda / "lib64") in env["LIBRARY_PATH"]
        swapped = good.replace(
            "1, 00000000:42:00.0, NVIDIA GeForce RTX 3090",
            "1, 00000000:61:00.0, Quadro K620",
        )
        expect_error(
            fc.FeatureCompleteError,
            lambda: fc.cuda_environment({}, cuda, runner(swapped)),
            "3090",
        )
        nvcc.unlink()
        expect_error(
            fc.FeatureCompleteError,
            lambda: fc.cuda_environment({}, cuda, runner(good)),
            "nvcc",
        )


def test_runner_environment(fc: ModuleType) -> None:
    env = fc.runner_environment(
        {"FATHOMDB_SKIP_NETWORK_TESTS": "1", "HOME": "/h"}, Path("/scratch")
    )
    assert "FATHOMDB_SKIP_NETWORK_TESTS" not in env
    assert env["FATHOMDB_SLICE72_RUNNER"] == "approved-nvidia"
    assert env["FATHOMDB_SLICE72_RECEIPT_DIR"].startswith("/scratch/")
    command = fc.test_command(
        "fathomdb-engine", ("a", "b"), ["--test", "t1", "--lib"], ["x::one", "two"]
    )
    assert command == [
        "cargo",
        "test",
        "--locked",
        "-p",
        "fathomdb-engine",
        "--no-default-features",
        "--features",
        "a,b",
        "--test",
        "t1",
        "--lib",
        "--no-fail-fast",
        "--",
        "--exact",
        "x::one",
        "two",
        "--nocapture",
        "--test-threads=1",
    ]


def test_slice72_assets(fc: ModuleType) -> None:
    """Slice 72's CUDA target needs exactly one visible device and an immutable
    asset root staged from the warmed caches it names by identity."""

    import hashlib

    with tempfile.TemporaryDirectory() as directory:
        base = Path(directory)
        support = base / "slice72_gpu_telemetry.rs"
        support.write_text(
            'cache_prefix("BAAI/bge-small-en-v1.5@abc")\n'
            'cache_prefix(\n    "cross-encoder/ms-marco-TinyBERT-L2-v2@def",\n)\n'
        )

        def prefix(identity: str) -> str:
            return hashlib.sha256(identity.encode()).hexdigest()[:12]

        cache = base / "cache"
        for kind, identity in (
            ("embedders", "BAAI/bge-small-en-v1.5@abc"),
            ("reranker", "cross-encoder/ms-marco-TinyBERT-L2-v2@def"),
        ):
            model = cache / "fathomdb" / kind / prefix(identity)
            model.mkdir(parents=True)
            for name in ("config.json", "tokenizer.json", "model.safetensors"):
                (model / name).write_text(f"{kind}:{name}")
        root = fc.stage_slice72_assets(support, cache, base / "assets")
        assert (
            root / "bge" / "model.safetensors"
        ).read_text() == "embedders:model.safetensors"
        assert (root / "reranker" / "config.json").read_text() == "reranker:config.json"
        env = fc.entry_environment(
            {"CUDA_VISIBLE_DEVICES": "0,1"}, ["slice72_concurrent_gpu"], root
        )
        assert env["CUDA_VISIBLE_DEVICES"] == "0"
        assert env["FATHOMDB_SLICE72_ASSET_ROOT"] == str(root)
        other = fc.entry_environment({"CUDA_VISIBLE_DEVICES": "0,1"}, ["loader"], root)
        assert other == {"CUDA_VISIBLE_DEVICES": "0,1"}
        (cache / "fathomdb" / "reranker").rename(base / "moved")
        expect_error(
            fc.FeatureCompleteError,
            lambda: fc.stage_slice72_assets(support, cache, base / "assets2"),
            "reranker",
        )


def test_weight_provisioning(fc: ModuleType) -> None:
    """Pinned weights are fetched once, verified before they are visible, and
    a digest mismatch leaves nothing behind."""

    import hashlib

    def blob_sha1(data: bytes) -> str:
        return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()

    contents = {"tokenizer.json": b'{"t": 1}', "model.safetensors": b"weights"}
    files = {
        "tokenizer.json": (
            "git-blob-sha1",
            blob_sha1(contents["tokenizer.json"]),
            "u/tok",
        ),
        "model.safetensors": (
            "sha256",
            hashlib.sha256(contents["model.safetensors"]).hexdigest(),
            "u/model",
        ),
    }
    fetched: list[str] = []

    def fetch(url: str, destination: Path) -> None:
        fetched.append(url)
        name = "tokenizer.json" if url == "u/tok" else "model.safetensors"
        destination.write_bytes(contents[name])

    with tempfile.TemporaryDirectory() as directory:
        target = Path(directory) / "fathomdb" / "embedders" / "nomic-v1.5"
        assert fc.provision_weights(target, files, fetch) == [
            "model.safetensors",
            "tokenizer.json",
        ]
        assert (target / "model.safetensors").read_bytes() == b"weights"
        assert sorted(fetched) == ["u/model", "u/tok"]
        # Present and valid: nothing is fetched again.
        fetched.clear()
        assert fc.provision_weights(target, files, fetch) == []
        assert fetched == []
        # A mismatch fails and leaves no partial or final file.
        bad = dict(files, **{"model.safetensors": ("sha256", "0" * 64, "u/model")})
        (target / "model.safetensors").unlink()
        expect_error(
            fc.FeatureCompleteError,
            lambda: fc.provision_weights(target, bad, fetch),
            "sha256",
        )
        assert sorted(p.name for p in target.iterdir()) == ["tokenizer.json"]
    assert fc.NOMIC_FILES["model.safetensors"][0] == "sha256"
    assert fc.NOMIC_REVISION in fc.NOMIC_FILES["model.safetensors"][2]


def test_extra_sets(tt: ModuleType) -> None:
    """Extra sets (gate-derived, for tests behind item-level feature cfgs that
    no target requires) round-trip through the matrix, are preserved by
    `--write-matrix`, and are checked statically for staleness."""

    entries = [("a", ()), ("b", ("x",))]
    extras = [("b", ("lonely",)), ("a", ("gpu",))]
    text = tt.render_matrix(entries, extras)
    assert "[[extra]]" in text
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "matrix.toml"
        path.write_text(text)
        assert tt.load_matrix(path) == sorted(entries)
        assert tt.load_extra_sets(path) == sorted(extras)
        path.write_text(tt.render_matrix(entries))
        assert tt.load_extra_sets(path) == []

    # Host-buildable features: everything but features whose closure, across
    # workspace crates, reaches a dependency feature whose platform cfg is
    # false on the host. `default` is never a candidate.
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / "Cargo.toml").write_text('[workspace]\nmembers = ["a", "b"]\n')
        (root / "a").mkdir()
        (root / "a" / "Cargo.toml").write_text(
            '[package]\nname = "a"\nversion = "0.1.0"\n\n[features]\n'
            'default = []\nhooks = []\nextra = ["hooks"]\nlonely = []\n'
            'gpu = ["b/shiny"]\n'
        )
        (root / "b").mkdir()
        (root / "b" / "Cargo.toml").write_text(
            '[package]\nname = "b"\nversion = "0.1.0"\n\n[features]\n'
            'shiny = ["somedep/metal"]\nplain = []\n'
        )
        crates = tt.read_workspace(root)
        table = {"somedep/metal": 'target_os = "macos"'}
        assert tt.host_buildable_features(crates, "a", HOST, table) == (
            "extra",
            "hooks",
            "lonely",
        )
        assert tt.host_buildable_features(
            crates, "a", "aarch64-apple-darwin", table
        ) == ("extra", "gpu", "hooks", "lonely")
        assert tt.host_buildable_features(crates, "b", HOST, table) == ("plain",)
    assert tt.PLATFORM_DEPENDENCY_FEATURES["candle-core/metal"]

    crates = tt.read_workspace(FIXTURE_CRATE)
    complete = tt.derive_matrix(crates)
    workspace = {"hs_fixture": frozenset()}
    # Declared features that are not a target-derived set: valid.
    failures, _ = tt.check_coverage(
        crates,
        workspace,
        complete,
        [],
        HOST,
        extras=[("hs_fixture", ("extra", "hooks"))],
    )
    assert failures == [], failures
    # A duplicate of a target-derived set, an undeclared feature, and an
    # unknown crate are each stale.
    for bad in (
        [("hs_fixture", ("hooks",))],
        [("hs_fixture", ("nope",))],
        [("gone", ("extra",))],
    ):
        failures, _ = tt.check_coverage(
            crates, workspace, complete, [], HOST, extras=bad
        )
        assert len(failures) == 1 and "extra set" in failures[0], (bad, failures)


def test_derive_extra_sets(fc: ModuleType, tt: ModuleType) -> None:
    """A test present under the union of host-buildable features but under
    neither the workspace features nor any target-derived set (an item-level
    `#[cfg(feature = "lonely")]` test in a crate where no target requires
    `lonely`) is assigned the smallest single feature that lists it, else the
    union; the derived sets are compared with the committed ones."""

    (crate,) = tt.read_workspace(FIXTURE_CRATE)
    assert fc.extra_candidates(crate, ("extra", "hooks")) == [("hooks",), ("extra",)]

    workspace = {"lib": {"a"}, "plain": {"runs"}}
    static = {("hooks",): {"lib": {"a", "h"}, "plain": {"runs", "second"}}}
    union_features = ("extra", "gpu", "hooks", "lonely")
    union = {
        "lib": {"a", "h", "lonely_test", "both_test"},
        "plain": {"runs", "second", "gpu_only"},
    }
    singles = {
        ("extra",): {"lib": {"a", "h"}},
        ("lonely",): {"lib": {"a", "lonely_test"}},
        ("gpu",): {"plain": {"runs", "gpu_only"}},
        ("zzz",): {"lib": {"a"}},
    }
    listed: list[tuple[str, ...]] = []

    def lister(features: tuple[str, ...]) -> dict[str, set[str]]:
        listed.append(features)
        return singles[features]

    candidates = [("extra",), ("hooks",), ("lonely",), ("gpu",), ("zzz",)]
    derived, listings = fc.derive_extra_sets(
        workspace, static, union, union_features, candidates, lister
    )
    assert derived == [("lonely",), ("gpu",), union_features], derived
    # A target-derived set is never re-listed.
    assert ("hooks",) not in listed
    pairs = fc.run_pairs(workspace, {**static, **listings})
    assert pairs == [
        (("gpu",), {"plain": ["gpu_only"]}),
        (("hooks",), {"lib": ["h"], "plain": ["second"]}),
        (("lonely",), {"lib": ["lonely_test"]}),
        (union_features, {"lib": ["both_test"]}),
    ], pairs

    # Nothing uncovered: nothing derived and nothing listed.
    listed.clear()
    covered = {"lib": {"a", "h"}, "plain": {"runs", "second"}}
    assert fc.derive_extra_sets(
        workspace, static, covered, union_features, candidates, lister
    ) == ([], {})
    assert listed == []

    assert fc.extra_set_drift("c", [("lonely",)], [("lonely",)]) == []
    drift = fc.extra_set_drift("c", [("lonely",)], [("lonely",), ("old",)])
    assert len(drift) == 1 and "old" in drift[0] and "--write-matrix" in drift[0]
    drift = fc.extra_set_drift("c", [("lonely",), ("new",)], [("lonely",)])
    assert len(drift) == 1 and "new" in drift[0], drift


def child_process_output() -> str:
    """A parent test that re-executes its own binary for a worker test prints
    the child's libtest lines inside its own run; the worker is also planned
    and runs once on its own."""

    return (
        "     Running tests/plain.rs (target/debug/deps/plain-0123)\n"
        "\n"
        "running 5 tests\n"
        "test worker ... ok\n"
        "test runs ... \n"
        "running 1 test\n"
        "test worker ... ok\n"
        "\n"
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out\n"
        "ok\n"
        "test second ... \n"
        "running 1 test\n"
        "test worker ... ok\n"
        "\n"
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out\n"
        "FAILED\n"
        "test ignored_by_design ... ignored\n"
        "\n"
        "test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out\n"
    )


def test_parent_counts(fc: ModuleType, tt: ModuleType) -> None:
    """Counts are per planned (parent) test from the gate's own bookkeeping:
    a child worker's `test result` lines are not counted, and a parent's bare
    status line is attributed to the parent even after child output."""

    (crate,) = tt.read_workspace(FIXTURE_CRATE)
    planned = {
        "hs_fixture::plain::worker",
        "hs_fixture::plain::runs",
        "hs_fixture::plain::second",
        "hs_fixture::plain::ignored_by_design",
        "hs_fixture::plain::never",
    }
    result = fc.scan_output(child_process_output(), crate, planned)
    assert "hs_fixture::plain::second" in result.failed, result.failed
    assert "hs_fixture::plain::worker" not in result.failed, result.failed
    assert fc.parent_counts(planned, result) == {
        "planned": 5,
        "passed": 2,
        "failed": 2,
        "ignored": 1,
    }
    ok = fc.scan_output(
        child_process_output().replace("FAILED\n", "ok\n", 1), crate, planned
    )
    assert fc.parent_counts(planned - {"hs_fixture::plain::never"}, ok) == {
        "planned": 4,
        "passed": 3,
        "failed": 0,
        "ignored": 1,
    }


def test_run_failures(fc: ModuleType, tt: ModuleType) -> None:
    """A planned test missing from the output, a non-zero cargo exit, and a
    binary that does not build each fail the gate."""

    (crate,) = tt.read_workspace(FIXTURE_CRATE)
    planned = {"hs_fixture::plain::runs", "hs_fixture::plain::never"}
    result = fc.scan_output(child_process_output(), crate, planned)
    failures = fc.run_failures("hs_fixture[hooks]", planned, result, 101, Path("/l"))
    assert failures == [
        "hs_fixture[hooks]: cargo test exited 101 (see /l)",
        "hs_fixture::plain::never: did not run",
    ], failures
    assert (
        fc.run_failures(
            "hs_fixture[hooks]", {"hs_fixture::plain::runs"}, result, 0, Path("/l")
        )
        == []
    )
    assert fc.listing_failures("c", ["lib", "plain"], None) == [
        "c::lib: does not build",
        "c::plain: does not build",
    ]
    assert fc.listing_failures("c", ["lib"], ("a", "b")) == [
        "c::lib: does not build with ['a', 'b']"
    ]
    assert fc.listing_failures("c", [], ("a",)) == []


def test_repository_wiring(tt: ModuleType) -> None:
    script = (ROOT / "scripts" / "test-feature-complete.sh").read_text()
    assert "feature_complete.py" in script
    check = (ROOT / "scripts" / "check.sh").read_text()
    assert 'FATHOMDB_FEATURE_COMPLETE:-0}" = "1"' in check
    assert "scripts/test-feature-complete.sh" in check
    agent_test = (ROOT / "scripts" / "agent-test.sh").read_text()
    for line in (
        "run_tier_suite fast check-test-target-coverage python3 scripts/check-test-target-coverage.py",
        "run_tier_suite fast test-test-targets python3 scripts/tests/test_test_targets.py",
    ):
        assert line in agent_test, line
    # The committed matrix is exactly what --write-matrix renders, including
    # the gate-derived extra sets for item-level feature tests no target
    # requires.
    crates = tt.read_workspace(ROOT)
    matrix = ROOT / "scripts" / "test-feature-matrix.toml"
    extras = tt.load_extra_sets(matrix)
    assert matrix.read_text() == tt.render_matrix(tt.derive_matrix(crates), extras)
    for known in (
        ("fathomdb-cli", ("default-embedder",)),
        ("fathomdb-cli", ("default-reranker",)),
        ("fathomdb-embedder", ("embed-cuda",)),
        ("fathomdb-embedder", ("rerank-cuda",)),
    ):
        assert known in extras, (known, extras)
    # The calibration's assertions run in a gate-run test; only the writer of
    # the committed record is excluded.
    calibration = (
        ROOT / "src/rust/crates/fathomdb-embedder/tests/cross_backend_calibration.rs"
    ).read_text()
    assert "#[test]\nfn calibration_cpu_baseline_components_hold() {" in calibration
    for assertion in (
        'assert!(mean_l2 > 1e-3, "pinned mean fixture must be non-degenerate (‖mean‖₂={mean_l2:.6})");',
        'assert_eq!(m.mean_centered_flips_total, 0, "CPU baseline mean-centered flips must be 0");',
        'assert!(m.p2_l2_max < 1e-4, "CPU baseline P2 L2 max {:.3e} unexpectedly large", m.p2_l2_max);',
    ):
        assert calibration.count(assertion) == 1, assertion
    body = calibration.split("fn calibration_cpu_baseline_components_hold() {")[1]
    body = body.split("\n}\n")[0]
    assert "assert_cpu_baseline_components(" in body, body
    assert "write_durable_doc" not in body, body
    allowlist = tt.load_allowlist(ROOT / "scripts" / "test-skip-allowlist.toml")
    assert all(entry["reason"].strip() for entry in allowlist)
    excluded = [entry["id"] for entry in allowlist if entry.get("exclude")]
    assert (
        "fathomdb-embedder::cross_backend_calibration::calibration_reports_p1_flips_and_p2_l2"
        in excluded
    )
    assert not any("calibration_cpu_baseline_components_hold" in e for e in excluded)


def main() -> None:
    tt = load("test_targets", ROOT / "scripts" / "lib" / "test_targets.py")
    fc = load("feature_complete", ROOT / "scripts" / "lib" / "feature_complete.py")
    for test in (
        test_cfg_parsing,
        test_read_fixture_workspace,
        test_matrix_round_trip,
        test_coverage_check,
        test_extra_sets,
        test_repository_wiring,
    ):
        test(tt)
        print(f"ok    {test.__name__}")
    test_skip_contract(fc, tt)
    print("ok    test_skip_contract")
    test_skip_markers(fc, tt)
    print("ok    test_skip_markers")
    for paired in (test_derive_extra_sets, test_parent_counts, test_run_failures):
        paired(fc, tt)
        print(f"ok    {paired.__name__}")
    test_canonical_properties(tt)
    print("ok    test_canonical_properties")
    for test in (
        test_cuda_preflight,
        test_runner_environment,
        test_slice72_assets,
        test_weight_provisioning,
        test_onnx_provisioning,
        test_run_pairs,
    ):
        test(fc)
        print(f"ok    {test.__name__}")
    print("ok    test-targets")


if __name__ == "__main__":
    main()
