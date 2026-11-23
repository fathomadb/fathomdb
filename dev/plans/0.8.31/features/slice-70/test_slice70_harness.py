"""TDD unit & real-SQLite verification tests for Slice 70 qualification harnesses."""

from __future__ import annotations

import importlib.util
import json
import math
import subprocess
import sys
from pathlib import Path
from types import ModuleType

import pytest

SLICE70_DIR = Path(__file__).resolve().parent
REPO_ROOT = SLICE70_DIR.parents[4]
PAIRED_GATE_PATH = SLICE70_DIR / "run-paired-latency-gate.py"
GOLDEN_BUILDER_PATH = SLICE70_DIR / "build-golden-0.8.31.py"


def _load_module(path: Path, name: str) -> ModuleType:
    if not path.is_file():
        raise FileNotFoundError(f"harness script not found: {path}")
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module spec from {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def _load_paired_gate() -> ModuleType:
    return _load_module(PAIRED_GATE_PATH, "slice70_paired_latency_gate")


def _load_golden_builder() -> ModuleType:
    return _load_module(GOLDEN_BUILDER_PATH, "slice70_build_golden_0831")


def test_paired_bootstrap_rejects_synthetic_8pct_p95_regression() -> None:
    mod = _load_paired_gate()
    # 50 paired samples around 40-50 ms so 5% (~2.4 ms) > 1.0 ms floor; +8% breaches clearly.
    baseline = [40.0 + (i % 10) * 1.0 for i in range(50)]
    candidate = [x * 1.08 for x in baseline]

    result = mod.evaluate_paired_latency_metric(
        baseline,
        candidate,
        min_samples=30,
        resamples=10_000,
        seed=20261122,
    )
    assert result["p95"]["resamples"] == 10_000
    assert result["p95"]["seed"] == 20261122
    assert result["p95"]["ci_95_upper_ms"] > result["p95"]["allowed_delta_ms"]
    assert result["p95"]["passed"] is False
    assert result["passed"] is False


def test_paired_bootstrap_accepts_identical_and_noise_bounded_pairs() -> None:
    mod = _load_paired_gate()
    baseline = [12.0 + (i % 7) * 0.5 for i in range(40)]
    identical = list(baseline)
    noisy = [x + (0.25 if i % 2 == 0 else -0.15) for i, x in enumerate(baseline)]

    res_identical = mod.evaluate_paired_latency_metric(
        baseline,
        identical,
        min_samples=30,
        resamples=10_000,
        seed=20261122,
    )
    assert res_identical["passed"] is True
    assert res_identical["p50"]["passed"] is True
    assert res_identical["p95"]["passed"] is True
    assert math.isclose(res_identical["p95"]["observed_delta_ms"], 0.0, abs_tol=1e-9)

    res_noisy = mod.evaluate_paired_latency_metric(
        baseline,
        noisy,
        min_samples=30,
        resamples=10_000,
        seed=20261122,
    )
    assert res_noisy["passed"] is True
    assert res_noisy["p50"]["ci_95_upper_ms"] <= 1.0
    assert res_noisy["p95"]["ci_95_upper_ms"] <= 1.0

    # Determinism: identical seed produces bit-identical CI bounds.
    res_repeat = mod.evaluate_paired_latency_metric(
        baseline,
        noisy,
        min_samples=30,
        resamples=10_000,
        seed=20261122,
    )
    assert res_noisy == res_repeat


def test_paired_bootstrap_fails_closed_below_min_samples() -> None:
    mod = _load_paired_gate()
    short = [10.0] * 10
    res = mod.evaluate_paired_latency_metric(
        short,
        short,
        min_samples=30,
        resamples=1000,
        seed=20261122,
    )
    assert res["passed"] is False


def test_resource_ceilings_fail_closed_on_breach() -> None:
    mod = _load_paired_gate()

    # database_bytes ceiling: <= +15%
    assert mod.evaluate_database_bytes(10_000_000, 11_500_000)["passed"] is True
    breach_db = mod.evaluate_database_bytes(10_000_000, 11_500_001)
    assert breach_db["passed"] is False
    assert breach_db["relative_delta"] > 0.15

    # peak_rss_bytes ceiling: <= +10%
    assert mod.evaluate_peak_rss_bytes(100_000_000, 110_000_000)["passed"] is True
    breach_rss = mod.evaluate_peak_rss_bytes(100_000_000, 110_000_001)
    assert breach_rss["passed"] is False
    assert breach_rss["relative_delta"] > 0.10

    # Zero/negative baseline must fail closed.
    assert mod.evaluate_database_bytes(0, 100)["passed"] is False
    assert mod.evaluate_peak_rss_bytes(0, 100)["passed"] is False


def test_high_fanout_predicate_and_as_of_seek_evaluators_fail_closed() -> None:
    mod = _load_paired_gate()

    # High-fanout declared predicate: 0.30x > 0.25x must fail closed; 0.10x passes.
    unindexed = [100.0] * 50
    indexed_bad = [30.0] * 50
    indexed_good = [10.0] * 50

    bad_pred = mod.evaluate_high_fanout_predicate_gate(
        indexed_bad, unindexed, max_ratio=0.25, min_samples=30
    )
    assert bad_pred["passed"] is False
    assert math.isclose(bad_pred["p50_ratio"], 0.30, rel_tol=1e-6)
    assert math.isclose(bad_pred["p95_ratio"], 0.30, rel_tol=1e-6)

    good_pred = mod.evaluate_high_fanout_predicate_gate(
        indexed_good, unindexed, max_ratio=0.25, min_samples=30
    )
    assert good_pred["passed"] is True

    # as_of composite seek: >= 5.0 ms (e.g., 5.5 ms or exact 5.0 ms) must fail closed; 2.5 ms passes.
    bad_as_of = mod.evaluate_as_of_seek_gate([5.5] * 50, max_p95_ms=5.0, min_samples=30)
    assert bad_as_of["passed"] is False

    edge_as_of = mod.evaluate_as_of_seek_gate([5.0] * 50, max_p95_ms=5.0, min_samples=30)
    assert edge_as_of["passed"] is False

    good_as_of = mod.evaluate_as_of_seek_gate([2.5] * 50, max_p95_ms=5.0, min_samples=30)
    assert good_as_of["passed"] is True


def test_golden_verifier_fails_closed_on_canary_reappearance_and_doctor_findings(
    tmp_path: Path,
) -> None:
    mod = _load_golden_builder()
    from fathomdb import Engine

    # 1. Real SQLite DB where canary bytes are still present (not erased) -> MUST fail closed.
    db_dirty = tmp_path / "dirty.sqlite"
    eng = Engine.open(str(db_dirty))
    try:
        eng.write(
            [
                {
                    "kind": "note",
                    "body": "Active note carrying CANARY_LEAK_0831 in body",
                    "logical_id": "leak:1",
                    "source_id": "src-leak",
                }
            ]
        )
        eng.drain()
    finally:
        eng.close()

    with pytest.raises(RuntimeError, match="canary"):
        mod.verify_erased_canaries_absent(db_dirty, [b"CANARY_LEAK_0831"])

    # 2. Real SQLite DB after clean autonomous erase_source -> canary scan AND real doctor pass.
    db_clean = tmp_path / "clean.sqlite"
    eng = Engine.open(str(db_clean))
    try:
        eng.write(
            [
                {
                    "kind": "note",
                    "body": "Surviving canonical record",
                    "logical_id": "keep:1",
                    "source_id": "src-keep",
                },
                {
                    "kind": "note",
                    "body": "Secret body CANARY_ERASE_BODY_0831",
                    "logical_id": "secret:1",
                    "source_id": "src-erase",
                    "properties": {"erasure_canary_tag": "CANARY_ERASE_PROP_0831"},
                },
            ]
        )
        eng.erase_source("src-erase")
        eng.drain()
    finally:
        eng.close()

    mod.verify_erased_canaries_absent(
        db_clean, [b"CANARY_ERASE_BODY_0831", b"CANARY_ERASE_PROP_0831"]
    )

    # Non-empty -wal file next to closed DB must fail closed.
    wal_file = Path(f"{db_clean}-wal")
    wal_file.write_bytes(b"nonempty-wal")
    try:
        with pytest.raises(RuntimeError, match="WAL"):
            mod.verify_erased_canaries_absent(db_clean, [b"CANARY_ERASE_BODY_0831"])
    finally:
        wal_file.unlink()

    # 3. Doctor integrity verifier: real binary passes on clean DB; fails closed on findings or non-zero exit.
    fathomdb_bin = REPO_ROOT / "target/release/fathomdb"
    if not fathomdb_bin.is_file():
        fathomdb_bin = REPO_ROOT / "target/debug/fathomdb"
    assert fathomdb_bin.is_file(), "expected built fathomdb binary in target/{release,debug}/fathomdb"

    summary = mod.verify_doctor_integrity(db_clean, fathomdb_bin)
    assert summary["physical"]["status"] == "clean"
    assert summary["logical"]["status"] == "clean"
    assert summary["semantic"]["status"] == "clean"

    # Non-clean doctor JSON payload must fail closed.
    bad_payload = json.dumps(
        {
            "verb": "check-integrity",
            "physical": {"status": "clean", "findings": []},
            "logical": {
                "status": "findings",
                "findings": [{"code": "E_CORRUPT", "detail": "synthetic finding"}],
            },
            "semantic": {"status": "clean", "findings": []},
        }
    )

    def _fake_findings_runner(cmd: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(cmd, returncode=0, stdout=bad_payload, stderr="")

    with pytest.raises(RuntimeError, match="doctor check-integrity"):
        mod.verify_doctor_integrity(db_clean, fathomdb_bin, runner=_fake_findings_runner)

    def _fake_nonzero_runner(cmd: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(cmd, returncode=65, stdout=bad_payload, stderr="failed")

    with pytest.raises(RuntimeError, match="doctor check-integrity"):
        mod.verify_doctor_integrity(db_clean, fathomdb_bin, runner=_fake_nonzero_runner)
