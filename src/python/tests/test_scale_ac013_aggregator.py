"""Contract tests for the retained AC-013 scale-artifact aggregator."""

from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
AGGREGATOR = ROOT / "scripts" / "perf-experiments" / "aggregate-scale-ac013.py"
ROWS = (10_000, 100_000, 1_000_000)
TREATMENTS = ("process_cold", "warm")


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def _package(name: str) -> dict[str, str]:
    lock = ROOT / "Cargo.lock"
    text = lock.read_text(encoding="utf-8")
    block = next(block for block in text.split("[[package]]") if f'name = "{name}"' in block)
    values = dict(line.split(" = ", 1) for line in block.splitlines() if " = " in line)
    return {key: values[key].strip('"') for key in ("name", "version", "source", "checksum")}


def _provenance(head: str, *, clean: bool = True) -> dict[str, object]:
    state_path = "dev/plans/release-state-0.8.22.json"
    state_blob = _git("rev-parse", f"HEAD:{state_path}")
    state_bytes = (ROOT / state_path).read_bytes()
    state = json.loads(state_bytes)
    slice5 = next(item for item in state["ladder"] if item["slice"] == 5)
    lock = (ROOT / "Cargo.lock").read_bytes()
    return {
        "schema_version": 1,
        "protocol": "0.8.23-scale-characterization-v1",
        "candidate": {
            "head_sha": head,
            "ref": "test",
            "commit_timestamp": "2026-08-17T00:00:00Z",
            "git_status_porcelain": "" if clean else " M dirty",
            "worktree_clean": clean,
            "schema_version": 26,
        },
        "slice5_provenance": {
            "release": "0.8.22",
            "authoritative_release_state": {
                "path": state_path, "git_blob": state_blob, "sha256": _sha(state_bytes),
                "slice": 5, "status": "LANDED", "landed_sha": slice5["sha"],
                "evidence_sha256": _sha(slice5["evidence"].encode()),
            },
            "measured_lock_stack": {
                "cargo_lock_sha256": _sha(lock), "rusqlite": _package("rusqlite"),
                "sqlite_vec": _package("sqlite-vec"), "libsqlite3_sys": _package("libsqlite3-sys"),
            },
        },
        "lock_build": {
            "cargo_lock_git_blob": _git("rev-parse", "HEAD:Cargo.lock"), "cargo_lock_sha256": _sha(lock),
            "cargo_metadata_locked_sha256": "0" * 64, "schema_version": 26,
            "schema_source_git_blob": _git("rev-parse", "HEAD:src/rust/crates/fathomdb-schema/src/lib.rs"),
            "rustc_vv": "test", "cargo_v": "test", "target": "test", "profile": "release",
            "enabled_features": [], "command": "test", "built_process_sqlite_version": "test",
        },
        "fixture": {
            "id": "ac013-deterministic-vector-v1", "generator": "seed_ac013_corpus",
            "corpus_seed": "0x0AC0_13D0_13D0", "query_generator": "ac013_query_bodies",
            "query_seed": "0x0AC0_130D_EC0D_E000", "source_identity": "test:fixture",
            "vector_dimension": 384, "query_count": 1000, "scale_points": list(ROWS),
            "perf_gates_git_blob": _git("rev-parse", "HEAD:src/rust/crates/fathomdb-engine/tests/perf_gates.rs"),
            "descriptor_sha256": "1" * 64,
        },
        "environment": {
            "host": {"collect_host_spec_sha256": "2" * 64, "total_ram_bytes": 1,
                     "available_ram_bytes": 1, "database_filesystem": "test", "database_free_bytes": 1,
                     "cpu_governor_frequency_policy": "test"},
            "gpu": {"state": "not_present_or_unavailable"},
            "isolation": {"timestamp": "2026-08-17T00:00:00Z", "load_evidence_sha256": "3" * 64,
                          "process_inventory_sha256": "4" * 64, "cpu_affinity": "test",
                          "governor_policy": "test", "power_policy": "test", "container_or_vm": "test",
                          "no_concurrent_benchmark": True},
        },
    }


def _root(*, clean: bool = True, partial: bool = False, malformed: bool = False) -> Path:
    head = _git("rev-parse", "HEAD")
    directory = Path(tempfile.mkdtemp(prefix="0.8.23-scale-", dir=ROOT / "dev" / "plans" / "runs"))
    root = directory / f"0.8.23-scale-{head}-raw"
    root.mkdir()
    (root / "provenance.json").write_text(json.dumps(_provenance(head, clean=clean)), encoding="utf-8")
    entries: list[dict[str, object]] = []
    for rows in ROWS:
        for treatment in TREATMENTS:
            for repetition in range(1, 6):
                if partial and rows == 1_000_000 and treatment == "warm" and repetition == 5:
                    continue
                name = f"ac013-{rows}-{treatment}-rep{repetition}.log"
                samples = "7" if treatment == "process_cold" else ",".join(["7"] * 1000)
                counts = "1" if treatment == "process_cold" else ",".join(["1"] * 1000)
                if malformed and treatment == "warm" and repetition == 1:
                    counts = "not_retained_per_query"
                line = (f"AC013_TREATMENT_RECORD treatment={treatment} n={rows} seed_write_ms=1 "
                        f"embedding_ms=not_separately_observable projection_drain_ms=1 accepted_writes={rows} "
                        f"vector_rows_after_drain={rows} drain_outcome=ok samples_us={samples} result_counts={counts}\n")
                raw = line.encode()
                (root / name).write_bytes(raw)
                digest = _sha(raw)
                (root / f"{name}.sha256").write_text(f"{digest}  {name}\n", encoding="utf-8")
                entries.append({"rows": rows, "treatment": treatment, "repetition": repetition,
                                "log": name, "sidecar": f"{name}.sha256", "sha256": digest})
    manifest = {"schema_version": 1, "protocol": "0.8.23-scale-characterization-v1",
                "candidate_head_sha": head, "entries": entries}
    (root / "matrix-manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
    return root


def _aggregate(root: Path) -> dict[str, object]:
    result = subprocess.run(["python3", str(AGGREGATOR), "--input-root", str(root.relative_to(ROOT))],
                            cwd=ROOT, check=False, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    return json.loads(result.stdout)


def test_aggregator_derives_characterized_only_from_complete_valid_matrix() -> None:
    root = _root()
    try:
        artifact = _aggregate(root)
    finally:
        subprocess.run(["rm", "-rf", str(root.parent)], check=True)
    assert artifact["status"] == "CHARACTERIZED"
    assert len(artifact["matrix"]) == 6
    assert artifact["summary"] is not None
    warm = next(cell for cell in artifact["matrix"] if cell["treatment"] == "warm")
    assert len(warm["repetitions"][0]["samples_us"]) == 1000
    assert "p99_us" in warm["repetitions"][0]["query_statistics"]


@pytest.mark.parametrize("kind,expected", [("partial", "INSUFFICIENT_SAMPLES"), ("dirty", "ENVIRONMENT_INVALID"), ("malformed", "ENVIRONMENT_INVALID")])
def test_aggregator_fail_closed_statuses(kind: str, expected: str) -> None:
    root = _root(clean=kind != "dirty", partial=kind == "partial", malformed=kind == "malformed")
    try:
        artifact = _aggregate(root)
    finally:
        subprocess.run(["rm", "-rf", str(root.parent)], check=True)
    assert artifact["status"] == expected
    assert artifact["summary"] is None


def test_aggregator_rejects_extra_file_and_corrupt_sidecar() -> None:
    root = _root()
    try:
        (root / "extra").write_text("no", encoding="utf-8")
        assert _aggregate(root)["status"] == "ENVIRONMENT_INVALID"
        (root / "extra").unlink()
        sidecar = next(root.glob("*.sha256"))
        sidecar.write_text("0" * 64 + "  wrong.log\n", encoding="utf-8")
        assert _aggregate(root)["status"] == "ENVIRONMENT_INVALID"
    finally:
        subprocess.run(["rm", "-rf", str(root.parent)], check=True)
