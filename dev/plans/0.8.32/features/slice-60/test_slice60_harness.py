"""TDD unit & real-SQLite verification tests for Slice 60 qualification harnesses."""

from __future__ import annotations

import importlib.util
import json
import math
import os
import stat
import subprocess
import sys
from pathlib import Path
from types import ModuleType

import pytest

SLICE60_DIR = Path(__file__).resolve().parent
REPO_ROOT = SLICE60_DIR.parents[4]
PAIRED_GATE_PATH = SLICE60_DIR / "run-paired-latency-gate.py"
GOLDEN_BUILDER_PATH = SLICE60_DIR / "build-golden-0.8.32.py"
EXPECTED_V0831_COMMIT_SHA = "8965af93d6f42d160474acb382596c5aab6b5c2d"


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
    return _load_module(PAIRED_GATE_PATH, "slice60_paired_latency_gate")


def _load_golden_builder() -> ModuleType:
    return _load_module(GOLDEN_BUILDER_PATH, "slice60_build_golden_0832")


def _resolve_cli_bin() -> Path:
    for rel in ("target/release/fathomdb", "target/debug/fathomdb"):
        candidate = REPO_ROOT / rel
        if candidate.is_file():
            return candidate
    raise AssertionError("expected built fathomdb binary in target/{release,debug}/fathomdb")


def test_paired_gate_rejects_warmup_rounds_below_five() -> None:
    """AC32-60-02 requires --warmup-rounds >= 5; lower values must fail closed."""
    mod = _load_paired_gate()
    with pytest.raises(SystemExit) as excinfo:
        mod.main(["--warmup-rounds", "4", "--skip-build"])
    assert excinfo.value.code == 2

    with pytest.raises(ValueError, match="warmup_rounds"):
        mod.validate_warmup_rounds(4)
    assert mod.validate_warmup_rounds(5) == 5


def test_resolve_commit_sha_peels_annotated_tag_and_rejects_nonexistent() -> None:
    """Commit-peeling helper peels annotated tag names/SHAs to commit SHA and rejects bad refs."""
    mod = _load_paired_gate()
    assert mod.DEFAULT_BASELINE_SHA == EXPECTED_V0831_COMMIT_SHA

    # Annotated tag name peels to the commit SHA.
    assert mod.resolve_commit_sha("v0.8.31", repo_root=REPO_ROOT) == EXPECTED_V0831_COMMIT_SHA

    # Annotated tag object SHA also peels to the exact same commit SHA.
    tag_obj_sha = subprocess.check_output(
        ["git", "-C", str(REPO_ROOT), "rev-parse", "--verify", "v0.8.31"],
        text=True,
    ).strip()
    assert mod.resolve_commit_sha(tag_obj_sha, repo_root=REPO_ROOT) == EXPECTED_V0831_COMMIT_SHA

    # Already-peeled commit SHA passes through identically.
    assert (
        mod.resolve_commit_sha(EXPECTED_V0831_COMMIT_SHA, repo_root=REPO_ROOT)
        == EXPECTED_V0831_COMMIT_SHA
    )

    # Nonexistent ref fails closed.
    with pytest.raises(RuntimeError, match="commit"):
        mod.resolve_commit_sha("nonexistent-ref-0832-does-not-exist", repo_root=REPO_ROOT)


def test_arm_schema_version_check_enforces_38_vs_39() -> None:
    """Arm A must report schema_version_after == 38 and Arm B == 39; mismatches fail closed."""
    mod = _load_paired_gate()

    mod.validate_arm_schema_versions(
        {"schema_version_after": 38},
        {"schema_version_after": 39},
    )

    with pytest.raises(RuntimeError, match="38"):
        mod.validate_arm_schema_versions(
            {"schema_version_after": 37},
            {"schema_version_after": 39},
        )

    with pytest.raises(RuntimeError, match="39"):
        mod.validate_arm_schema_versions(
            {"schema_version_after": 38},
            {"schema_version_after": 38},
        )

    with pytest.raises(RuntimeError, match="38"):
        mod.validate_arm_schema_versions({}, {"schema_version_after": 39})


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
        seed=20261126,
    )
    assert result["p95"]["resamples"] == 10_000
    assert result["p95"]["seed"] == 20261126
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
        seed=20261126,
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
        seed=20261126,
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
        seed=20261126,
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
        seed=20261126,
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


def test_gate1_d10_ratio_gate_enforces_015x_bound_fail_closed() -> None:
    """Gate1-D10 requires candidate p50 <= 0.15x AND p95 <= 0.15x baseline."""
    mod = _load_paired_gate()
    baseline = [200.0] * 30

    # Clear pass (0.05x)
    good = mod.evaluate_gate1_d10_ratio_gate(
        baseline, [10.0] * 30, max_ratio=0.15, min_samples=30
    )
    assert good["passed"] is True
    assert math.isclose(good["p50_ratio"], 0.05, rel_tol=1e-6)
    assert math.isclose(good["p95_ratio"], 0.05, rel_tol=1e-6)

    # Exact boundary (0.15x) passes
    boundary = mod.evaluate_gate1_d10_ratio_gate(
        baseline, [30.0] * 30, max_ratio=0.15, min_samples=30
    )
    assert boundary["passed"] is True

    # Breaches both p50 and p95 (0.20x) -> fails closed
    bad_both = mod.evaluate_gate1_d10_ratio_gate(
        baseline, [40.0] * 30, max_ratio=0.15, min_samples=30
    )
    assert bad_both["passed"] is False
    assert math.isclose(bad_both["p50_ratio"], 0.20, rel_tol=1e-6)

    # p50 passes (0.05x) but p95 breaches (0.20x) -> fails closed
    tail_breach = [10.0] * 28 + [40.0, 40.0]
    bad_p95 = mod.evaluate_gate1_d10_ratio_gate(
        baseline, tail_breach, max_ratio=0.15, min_samples=30
    )
    assert bad_p95["p50_ratio"] <= 0.15
    assert bad_p95["p95_ratio"] > 0.15
    assert bad_p95["passed"] is False

    # Below min_samples -> fails closed
    short = mod.evaluate_gate1_d10_ratio_gate(
        [200.0] * 10, [5.0] * 10, max_ratio=0.15, min_samples=30
    )
    assert short["passed"] is False

    # Length mismatch -> fails closed
    mismatch = mod.evaluate_gate1_d10_ratio_gate(
        [200.0] * 30, [5.0] * 29, max_ratio=0.15, min_samples=30
    )
    assert mismatch["passed"] is False


def test_logical_id_parity_mismatch_fails_closed() -> None:
    """Zero logical_id parity mismatches permitted across query workloads."""
    mod = _load_paired_gate()

    ok = mod.evaluate_logical_id_parity(
        {"fts_ids": [["node:1", "node:2"], ["node:3"]]},
        {"fts_ids": [["node:1", "node:2"], ["node:3"]]},
        context_label="scale=10000 block=0",
        query_indices=[0, 1],
    )
    assert ok["passed"] is True
    assert ok["mismatch_count"] == 0
    assert ok["sample_mismatches"] == []

    # Reordered / differing logical IDs must fail closed.
    bad = mod.evaluate_logical_id_parity(
        {"fts_ids": [["node:1", "node:2"], ["node:3"]]},
        {"fts_ids": [["node:2", "node:1"], ["node:3"]]},
        context_label="scale=10000 block=0",
        query_indices=[0, 1],
    )
    assert bad["passed"] is False
    assert bad["mismatch_count"] == 1
    assert "fts_ids" in bad["sample_mismatches"][0]

    # Query count mismatch must also fail closed.
    count_bad = mod.evaluate_logical_id_parity(
        {"fts_ids": [["node:1"]]},
        {"fts_ids": [["node:1"], ["node:2"]]},
        context_label="scale=10000 block=0",
        query_indices=[0, 1],
    )
    assert count_bad["passed"] is False
    assert count_bad["mismatch_count"] >= 1


def test_golden_verifier_checks_canaries_doctor_and_pre_chmod_ordering(
    tmp_path: Path,
) -> None:
    """Verify canary scan, both doctor checks, and strict pre-chmod-0444 ordering."""
    mod = _load_golden_builder()
    from fathomdb import Engine

    fathomdb_bin = _resolve_cli_bin()

    # 1. Real SQLite DB where canary bytes survive -> fails closed.
    db_dirty = tmp_path / "dirty.sqlite"
    eng = Engine.open(str(db_dirty), use_default_embedder=False)
    try:
        eng.write(
            [
                {
                    "kind": "note",
                    "body": "Active note carrying CANARY_LEAK_0832 in body",
                    "logical_id": "leak:1",
                    "source_id": "src-leak",
                }
            ]
        )
        eng.drain()
    finally:
        eng.close()

    with pytest.raises(RuntimeError, match="canary"):
        mod.verify_erased_canaries_absent(db_dirty, [b"CANARY_LEAK_0832"])

    # 2. Real SQLite DB after clean erase_source -> canary scan AND both real doctor verbs pass.
    db_clean = tmp_path / "clean.sqlite"
    eng = Engine.open(str(db_clean), use_default_embedder=False)
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
                    "body": "Secret body CANARY_ERASE_BODY_0832",
                    "logical_id": "secret:1",
                    "source_id": "src-erase",
                    "properties": {"erasure_canary_tag": "CANARY_ERASE_PROP_0832"},
                },
            ]
        )
        eng.erase_source("src-erase")
        eng.drain()
    finally:
        eng.close()

    mod.verify_erased_canaries_absent(
        db_clean, [b"CANARY_ERASE_BODY_0832", b"CANARY_ERASE_PROP_0832"]
    )

    # Non-empty -wal file next to closed DB must fail closed.
    wal_file = Path(f"{db_clean}-wal")
    wal_file.write_bytes(b"nonempty-wal")
    try:
        with pytest.raises(RuntimeError, match="WAL"):
            mod.verify_erased_canaries_absent(db_clean, [b"CANARY_ERASE_BODY_0832"])
    finally:
        wal_file.unlink()

    # Real doctor check-integrity and data-plane-integrity pass on clean DB.
    ci_summary = mod.verify_doctor_integrity(db_clean, fathomdb_bin)
    assert ci_summary["physical"]["status"] == "clean"
    assert ci_summary["logical"]["status"] == "clean"
    assert ci_summary["semantic"]["status"] == "clean"

    dpi_summary = mod.verify_doctor_data_plane_integrity(db_clean, fathomdb_bin)
    assert dpi_summary["status"] == "clean"
    assert dpi_summary["report"]["findings"] == []
    assert dpi_summary["report"]["complete"] is True

    # Synthetic findings in data-plane-integrity must fail closed.
    bad_dpi = json.dumps(
        {
            "schemaVersion": "fathomdb.doctor.data-plane-integrity.v1",
            "status": "findings",
            "report": {
                "complete": True,
                "findings": [{"code": "dangling_edge_endpoint", "severity": "critical"}],
            },
        }
    )

    def _fake_dpi_findings(cmd: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(cmd, returncode=0, stdout=bad_dpi, stderr="")

    with pytest.raises(RuntimeError, match="data-plane-integrity"):
        mod.verify_doctor_data_plane_integrity(
            db_clean, fathomdb_bin, runner=_fake_dpi_findings
        )

    # 3. Pre-chmod 0444 ordering test on full build_golden_schema39 (use_default_embedder=False):
    #    Observe file mode at every doctor call and assert BOTH doctor verbs execute BEFORE chmod 0444.
    golden_db = tmp_path / "golden-0.8.32-schema39.sqlite"
    manifest_path = tmp_path / "golden-0.8.32-manifest.json"
    observed_events: list[tuple[str, int]] = []

    def _recording_runner(cmd: list[str]) -> subprocess.CompletedProcess[str]:
        verb = cmd[2]
        mode = stat.S_IMODE(golden_db.stat().st_mode)
        observed_events.append((verb, mode))
        # Must NOT be chmod 0444 yet when doctor commands run!
        assert mode != 0o444, f"doctor {verb} ran after chmod 0444"
        assert mode & stat.S_IWUSR, f"expected writable DB before chmod 0444, got {oct(mode)}"
        return subprocess.run(cmd, text=True, capture_output=True, check=False)

    manifest = mod.build_golden_schema39(
        golden_db,
        manifest_path,
        fathomdb_bin=fathomdb_bin,
        use_default_embedder=False,
        doctor_runner=_recording_runner,
    )
    try:
        assert [verb for verb, _ in observed_events] == [
            "check-integrity",
            "data-plane-integrity",
        ]
        # STRICTLY AFTER doctor checks pass, file is sealed at 0o444 and manifest written.
        final_mode = stat.S_IMODE(golden_db.stat().st_mode)
        assert final_mode == 0o444
        assert manifest_path.is_file()
        loaded = json.loads(manifest_path.read_text(encoding="utf-8"))
        assert loaded == manifest
        assert manifest["schema_version"] == 39
        assert manifest["producer_release"] == "0.8.32"
        assert manifest["file_mode"] == "0444"
        assert manifest["scopes_present"] == ["", "team-a", "team-b"]
        assert manifest["shared_projection_name"] == "domain_tier"
        assert manifest["counts"]["scope_visibility_watermarks_count"] > 0
        assert manifest["doctor_check_integrity"]["physical"]["status"] == "clean"
        assert manifest["doctor_data_plane_integrity"]["status"] == "clean"
    finally:
        if golden_db.exists():
            os.chmod(golden_db, 0o644)

    # 4. If doctor fails, DB must NOT be chmod 0444 and manifest must NOT be written.
    fail_db = tmp_path / "golden-fail.sqlite"
    fail_manifest = tmp_path / "golden-fail-manifest.json"
    with pytest.raises(RuntimeError, match="data-plane-integrity"):
        mod.build_golden_schema39(
            fail_db,
            fail_manifest,
            fathomdb_bin=fathomdb_bin,
            use_default_embedder=False,
            doctor_runner=lambda cmd: (
                subprocess.run(cmd, text=True, capture_output=True, check=False)
                if cmd[2] == "check-integrity"
                else _fake_dpi_findings(cmd)
            ),
        )
    assert stat.S_IMODE(fail_db.stat().st_mode) != 0o444
    assert not fail_manifest.exists()


def test_gate1_d10_workers_smoke_on_real_engine(tmp_path: Path) -> None:
    """Verify Gate1-D10 store builder and search_with_evidence block execute cleanly on live Engine."""
    mod = _load_paired_gate()
    db_path = tmp_path / "d10-smoke.sqlite"
    build_res = mod._worker_build_gate1_d10_store(
        {
            "db_path": str(db_path),
            "num_nodes": 50,
            "adopted_nodes": 10,
        }
    )
    assert build_res["schema_version"] == 39
    assert build_res["num_nodes"] == 50
    assert build_res["adopted_nodes"] == 10
    assert build_res["database_bytes"] > 0

    blk_res = mod._worker_gate1_d10_block(
        {
            "db_path": str(db_path),
            "warmup": 2,
            "query_indices": [0, 1, 2],
        }
    )
    assert len(blk_res["explain_false_ms"]) == 3
    assert len(blk_res["explain_true_ms"]) == 3
    assert len(blk_res["explain_false_ids"]) == 3
    assert len(blk_res["explain_true_ids"]) == 3
    assert blk_res["explain_false_ids"] == blk_res["explain_true_ids"]
    assert all(len(ids) > 0 for ids in blk_res["explain_false_ids"])

    # Verify realistic Gate1-D10 fixture shape: non-adopted `source` bodies are ~8.4 KB
    # (crossing 2 SQLite overflow pages so v0.8.31's uncached scan is ~120+ ms) while
    # `adopted` bodies stay concise.
    import sqlite3

    conn = sqlite3.connect(str(db_path))
    try:
        avg_source_len = conn.execute(
            "SELECT AVG(LENGTH(body)) FROM canonical_nodes WHERE kind = 'source'"
        ).fetchone()[0]
        avg_adopted_len = conn.execute(
            "SELECT AVG(LENGTH(body)) FROM canonical_nodes WHERE kind = 'adopted'"
        ).fetchone()[0]
    finally:
        conn.close()
    assert 8000 <= avg_source_len <= 9000, f"expected ~8.4 KB source bodies, got {avg_source_len}"
    assert avg_adopted_len < 300, f"expected concise adopted bodies, got {avg_adopted_len}"


