#!/usr/bin/env python3
"""Same-host interleaved paired v0.8.31 vs. 0.8.32 latency, resource, and Gate1-D10 harness.

Also provides `--run-sdk-benchmark-10k` mode to execute `experiments/sdk_benchmark.py`
(`sdk.experiment.v1`) on a pinned 10k synthetic suite with external `receipts_root` so
tracked `experiments/` stays clean.
"""

from __future__ import annotations

import argparse
import gc
import hashlib
import json
import os
import platform
import random
import re
import resource
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
import tomllib
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[5]
DEFAULT_BASELINE_SHA = "8965af93d6f42d160474acb382596c5aab6b5c2d"
DEFAULT_WORK_DIR = Path("/tmp/fathomdb-0832-qual/paired-bench")
DEFAULT_OUT_JSON = (
    Path(__file__).resolve().parent / "paired-latency-report.json"
)
EXPECTED_ARM_A_SCHEMA = 38
EXPECTED_ARM_B_SCHEMA = 39
TOPICS = ("alpha", "beta", "gamma", "delta", "epsilon")
TIERS = ("gold", "silver", "bronze")
BATCH_SIZE = 200
GATE1_D10_NODES = 10_000
GATE1_D10_ADOPTED_NODES = 1_000
GATE1_D10_MAX_RATIO = 0.15
GATE1_D10_MIN_SAMPLES = 30
_COMMIT_SHA_RE = re.compile(r"^[0-9a-f]{40}$")


# ---------------------------------------------------------------------------
# Pure validators & statistical evaluators (unit-tested in test_slice60_harness.py)
# ---------------------------------------------------------------------------


def validate_warmup_rounds(warmup_rounds: int) -> int:
    """Enforce `--warmup-rounds >= 5` fail-closed per AC32-60-02."""
    if warmup_rounds < 5:
        raise ValueError(
            f"warmup_rounds must be >= 5 per AC32-60-02, got {warmup_rounds}"
        )
    return int(warmup_rounds)


def resolve_commit_sha(ref: str, *, repo_root: Path = REPO_ROOT) -> str:
    """Normalize any git ref or annotated tag SHA to its peeled 40-hex commit SHA."""
    ref_clean = ref.strip()
    if not ref_clean:
        raise RuntimeError("baseline git ref must be non-empty")
    peeled_expr = f"{ref_clean}^{{commit}}"
    proc = subprocess.run(
        ["git", "-C", str(repo_root), "rev-parse", "--verify", peeled_expr],
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"failed to resolve git ref {ref_clean!r} to a commit via {peeled_expr!r}: "
            f"{proc.stderr.strip()}"
        )
    commit_sha = proc.stdout.strip()
    if not _COMMIT_SHA_RE.match(commit_sha):
        raise RuntimeError(
            f"git rev-parse returned unexpected commit SHA {commit_sha!r} for {ref_clean!r}"
        )
    return commit_sha


def validate_arm_schema_versions(
    probe_a: Mapping[str, Any],
    probe_b: Mapping[str, Any],
    *,
    expected_a: int = EXPECTED_ARM_A_SCHEMA,
    expected_b: int = EXPECTED_ARM_B_SCHEMA,
) -> None:
    """Fail closed unless Arm A reports Schema 38 and Arm B reports Schema 39."""
    actual_a = probe_a.get("schema_version_after")
    if actual_a != expected_a:
        raise RuntimeError(
            f"Arm A (v0.8.31) expected schema_version_after == {expected_a}, got {actual_a!r}"
        )
    actual_b = probe_b.get("schema_version_after")
    if actual_b != expected_b:
        raise RuntimeError(
            f"Arm B (0.8.32) expected schema_version_after == {expected_b}, got {actual_b!r}"
        )


def percentile(values: Sequence[float], pct: float) -> float:
    """Nearest-rank percentile matching prior release qualification harnesses."""
    if not values:
        raise ValueError("percentile requires at least one observation")
    ordered = sorted(values)
    idx = min(len(ordered) - 1, max(0, int(round((pct / 100.0) * (len(ordered) - 1)))))
    return float(ordered[idx])


def summarize_samples(samples_ms: Sequence[float]) -> dict[str, Any]:
    """Compute summary latency statistics for a sample sequence."""
    if not samples_ms:
        raise ValueError("summarize_samples requires at least one observation")
    return {
        "n": len(samples_ms),
        "p50_ms": round(statistics.median(samples_ms), 4),
        "p95_ms": round(percentile(samples_ms, 95.0), 4),
        "p99_ms": round(percentile(samples_ms, 99.0), 4),
        "min_ms": round(min(samples_ms), 4),
        "max_ms": round(max(samples_ms), 4),
    }


def paired_bootstrap_percentile_ci(
    baseline: Sequence[float],
    candidate: Sequence[float],
    pct: float,
    *,
    resamples: int = 10_000,
    seed: int = 20261126,
) -> dict[str, Any]:
    """Paired percentile bootstrap 95% CI for `percentile(candidate, pct) - percentile(baseline, pct)`.

    Passes iff `ci_95_upper <= max(0.05 * baseline_stat, 1.0)` ms.
    """
    if len(baseline) != len(candidate):
        raise ValueError(
            f"paired samples must have equal length: baseline={len(baseline)}, candidate={len(candidate)}"
        )
    n = len(baseline)
    if n == 0:
        raise ValueError("paired_bootstrap_percentile_ci requires non-empty samples")
    if resamples < 1:
        raise ValueError("resamples must be >= 1")

    base_list = [float(x) for x in baseline]
    cand_list = [float(y) for y in candidate]
    k_idx = min(n - 1, max(0, int(round((pct / 100.0) * (n - 1)))))

    base_stat = sorted(base_list)[k_idx]
    cand_stat = sorted(cand_list)[k_idx]
    observed_delta = cand_stat - base_stat

    rng = random.Random(seed)
    idx_pop = range(n)
    deltas: list[float] = []
    for _ in range(resamples):
        chosen = rng.choices(idx_pop, k=n)
        bx = sorted(base_list[i] for i in chosen)[k_idx]
        cy = sorted(cand_list[i] for i in chosen)[k_idx]
        deltas.append(cy - bx)
    deltas.sort()

    lo_idx = max(0, round(0.025 * resamples) - 1)
    hi_idx = min(resamples - 1, round(0.975 * resamples) - 1)
    ci_lower = deltas[lo_idx]
    ci_upper = deltas[hi_idx]
    allowed_delta_ms = max(0.05 * base_stat, 1.0)
    passed = ci_upper <= allowed_delta_ms

    return {
        "pct": pct,
        "n": n,
        "resamples": resamples,
        "seed": seed,
        "baseline_ms": round(base_stat, 4),
        "candidate_ms": round(cand_stat, 4),
        "observed_delta_ms": round(observed_delta, 4),
        "ci_95_lower_ms": round(ci_lower, 4),
        "ci_95_upper_ms": round(ci_upper, 4),
        "allowed_delta_ms": round(allowed_delta_ms, 4),
        "passed": passed,
    }


def evaluate_paired_latency_metric(
    baseline: Sequence[float],
    candidate: Sequence[float],
    *,
    min_samples: int = 30,
    resamples: int = 10_000,
    seed: int = 20261126,
) -> dict[str, Any]:
    """Evaluate paired p50 and p95 bootstrap gates, failing closed if `n < min_samples`."""
    if len(baseline) != len(candidate):
        raise ValueError(
            f"paired samples length mismatch: {len(baseline)} vs {len(candidate)}"
        )
    n = len(baseline)
    p50 = paired_bootstrap_percentile_ci(
        baseline, candidate, 50.0, resamples=resamples, seed=seed
    )
    p95 = paired_bootstrap_percentile_ci(
        baseline, candidate, 95.0, resamples=resamples, seed=seed
    )
    sample_ok = n >= min_samples
    return {
        "n": n,
        "min_samples": min_samples,
        "baseline_summary": summarize_samples(baseline),
        "candidate_summary": summarize_samples(candidate),
        "p50": p50,
        "p95": p95,
        "passed": bool(sample_ok and p50["passed"] and p95["passed"]),
    }


def evaluate_resource_ceiling(
    baseline_bytes: int,
    candidate_bytes: int,
    *,
    max_relative_increase: float,
    label: str,
) -> dict[str, Any]:
    """Fail-closed relative byte growth guard."""
    if baseline_bytes <= 0 or candidate_bytes < 0:
        return {
            "label": label,
            "baseline_bytes": baseline_bytes,
            "candidate_bytes": candidate_bytes,
            "relative_delta": None,
            "max_relative_increase": max_relative_increase,
            "passed": False,
        }
    rel = (candidate_bytes - baseline_bytes) / float(baseline_bytes)
    return {
        "label": label,
        "baseline_bytes": baseline_bytes,
        "candidate_bytes": candidate_bytes,
        "relative_delta": rel,
        "max_relative_increase": max_relative_increase,
        "passed": rel <= max_relative_increase,
    }


def evaluate_database_bytes(baseline_bytes: int, candidate_bytes: int) -> dict[str, Any]:
    """Assert `database_bytes` growth <= +15.0%."""
    return evaluate_resource_ceiling(
        baseline_bytes,
        candidate_bytes,
        max_relative_increase=0.15,
        label="database_bytes",
    )


def evaluate_peak_rss_bytes(baseline_bytes: int, candidate_bytes: int) -> dict[str, Any]:
    """Assert `peak_rss_bytes` growth <= +10.0%."""
    return evaluate_resource_ceiling(
        baseline_bytes,
        candidate_bytes,
        max_relative_increase=0.10,
        label="peak_rss_bytes",
    )


def evaluate_gate1_d10_ratio_gate(
    baseline_samples_ms: Sequence[float],
    candidate_samples_ms: Sequence[float],
    *,
    max_ratio: float = GATE1_D10_MAX_RATIO,
    min_samples: int = GATE1_D10_MIN_SAMPLES,
) -> dict[str, Any]:
    """Assert 0.8.32 `search_with_evidence` p50 and p95 <= `max_ratio` * v0.8.31 baseline."""
    if not baseline_samples_ms or not candidate_samples_ms:
        return {"passed": False, "reason": "empty_samples", "max_ratio": max_ratio}
    if len(baseline_samples_ms) != len(candidate_samples_ms):
        return {
            "passed": False,
            "reason": "length_mismatch",
            "baseline_n": len(baseline_samples_ms),
            "candidate_n": len(candidate_samples_ms),
            "max_ratio": max_ratio,
        }
    base_sum = summarize_samples(baseline_samples_ms)
    cand_sum = summarize_samples(candidate_samples_ms)
    base_p50 = float(statistics.median(baseline_samples_ms))
    base_p95 = percentile(baseline_samples_ms, 95.0)
    cand_p50 = float(statistics.median(candidate_samples_ms))
    cand_p95 = percentile(candidate_samples_ms, 95.0)
    if base_p50 <= 0.0 or base_p95 <= 0.0:
        return {
            "baseline": base_sum,
            "candidate": cand_sum,
            "p50_ratio": None,
            "p95_ratio": None,
            "max_ratio": max_ratio,
            "min_samples": min_samples,
            "passed": False,
        }
    p50_ratio = cand_p50 / base_p50
    p95_ratio = cand_p95 / base_p95
    sample_ok = len(baseline_samples_ms) >= min_samples
    return {
        "n": len(baseline_samples_ms),
        "baseline": base_sum,
        "candidate": cand_sum,
        "p50_ratio": round(p50_ratio, 6),
        "p95_ratio": round(p95_ratio, 6),
        "max_ratio": max_ratio,
        "min_samples": min_samples,
        "passed": bool(sample_ok and p50_ratio <= max_ratio and p95_ratio <= max_ratio),
    }


def evaluate_logical_id_parity(
    arm_a_ids: Mapping[str, Sequence[Sequence[str]]],
    arm_b_ids: Mapping[str, Sequence[Sequence[str]]],
    *,
    context_label: str = "",
    query_indices: Sequence[int] | None = None,
) -> dict[str, Any]:
    """Compare ordered `logical_id` result lists across arms; fail closed on any mismatch."""
    mismatches: list[str] = []
    prefix = f"{context_label} " if context_label else ""
    all_keys = sorted(set(arm_a_ids.keys()) | set(arm_b_ids.keys()))
    for key in all_keys:
        if key not in arm_a_ids or key not in arm_b_ids:
            mismatches.append(f"{prefix}{key}: missing workload key on one arm")
            continue
        seq_a = arm_a_ids[key]
        seq_b = arm_b_ids[key]
        if len(seq_a) != len(seq_b):
            mismatches.append(
                f"{prefix}{key}: query count mismatch v0.8.31={len(seq_a)} vs 0.8.32={len(seq_b)}"
            )
            continue
        for local_i, (ids_a, ids_b) in enumerate(zip(seq_a, seq_b)):
            if list(ids_a) != list(ids_b):
                q_id = (
                    query_indices[local_i]
                    if query_indices is not None and local_i < len(query_indices)
                    else local_i
                )
                mismatches.append(
                    f"{prefix}q={q_id} {key}: v0.8.31={list(ids_a)} vs 0.8.32={list(ids_b)}"
                )
    return {
        "passed": len(mismatches) == 0,
        "mismatch_count": len(mismatches),
        "sample_mismatches": mismatches[:10],
    }


# ---------------------------------------------------------------------------
# Subprocess hygiene, artifact building, and isolated worker actions
# ---------------------------------------------------------------------------


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _db_total_bytes(db_path: Path) -> int:
    total = 0
    for suffix in ("", "-wal", "-shm"):
        p = Path(f"{db_path}{suffix}")
        if p.exists():
            total += p.stat().st_size
    return total


def _cpu_model() -> str:
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.is_file():
        for line in cpuinfo.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    return platform.processor() or platform.machine()


def _clean_worker_env() -> dict[str, str]:
    path_parts = [
        "/usr/local/sbin",
        "/usr/local/bin",
        "/usr/sbin",
        "/usr/bin",
        "/sbin",
        "/bin",
    ]
    return {
        "PATH": os.pathsep.join(path_parts),
        "HOME": os.environ.get("HOME", "/tmp"),
        "FATHOMDB_EMBED_DEVICE": "cpu",
        "FATHOMDB_RERANK_DEVICE": "cpu",
    }


def _invoke_worker(arm_python: Path, action: str, spec: dict[str, Any]) -> dict[str, Any]:
    with tempfile.NamedTemporaryFile(
        mode="w",
        suffix=".json",
        prefix=f"slice60-worker-{action}-",
        encoding="utf-8",
        delete=False,
    ) as fh:
        json.dump(spec, fh)
        spec_path = Path(fh.name)
    try:
        cmd = [
            "env",
            "-i",
            *(f"{k}={v}" for k, v in _clean_worker_env().items()),
            str(arm_python),
            "-I",
            str(Path(__file__).resolve()),
            "--worker",
            action,
            "--spec-json",
            str(spec_path),
        ]
        proc = subprocess.run(cmd, text=True, capture_output=True, check=False)
        if proc.returncode != 0:
            raise RuntimeError(
                f"worker {action!r} failed (rc={proc.returncode}): "
                f"stderr={proc.stderr.strip()}\nstdout={proc.stdout.strip()}"
            )
        return json.loads(proc.stdout)
    finally:
        spec_path.unlink(missing_ok=True)


def _find_base_python() -> str:
    for candidate in ("/usr/bin/python3.12", shutil.which("python3.12"), sys.executable):
        if candidate and Path(candidate).is_file():
            return str(candidate)
    return sys.executable


def _ensure_venv_with_wheel(venv_dir: Path, wheel_path: Path) -> Path:
    venv_py = venv_dir / "bin/python"
    if not venv_py.is_file():
        subprocess.run(
            [_find_base_python(), "-m", "venv", str(venv_dir)],
            check=True,
        )
    subprocess.run(
        [
            str(venv_dir / "bin/pip"),
            "install",
            "--quiet",
            "--no-index",
            "--no-deps",
            "--force-reinstall",
            str(wheel_path),
        ],
        check=True,
    )
    return venv_py


def _build_wheel(source_py_dir: Path, wheels_dir: Path, cargo_target_dir: Path) -> Path:
    wheels_dir.mkdir(parents=True, exist_ok=True)
    for old in wheels_dir.glob("fathomdb-*.whl"):
        old.unlink()
    maturin = REPO_ROOT / ".venv/bin/maturin"
    if not maturin.is_file():
        raise FileNotFoundError(f"maturin not found in checkout .venv: {maturin}")
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = str(cargo_target_dir)
    subprocess.run(
        [str(maturin), "build", "--release", "--out", str(wheels_dir)],
        cwd=source_py_dir,
        env=env,
        check=True,
    )
    wheels = sorted(wheels_dir.glob("fathomdb-*.whl"))
    if len(wheels) != 1:
        raise RuntimeError(f"expected exactly 1 wheel in {wheels_dir}, found {wheels}")
    return wheels[0]


def prepare_arms(
    *,
    work_dir: Path,
    baseline_sha: str,
    target_dir_0832: Path,
    skip_build: bool,
) -> tuple[Path, Path, str]:
    """Build or reuse isolated noneditable release venvs for Arm A (v0.8.31) and Arm B (0.8.32)."""
    baseline_commit = resolve_commit_sha(baseline_sha, repo_root=REPO_ROOT)
    work_dir.mkdir(parents=True, exist_ok=True)
    venv_a = work_dir / "venv-0831"
    venv_b = work_dir / "venv-0832"
    wheels_a = work_dir / "wheels-0831"
    wheels_b = work_dir / "wheels-0832"

    py_a = venv_a / "bin/python"
    if not (skip_build and py_a.is_file()):
        cached_a = sorted(wheels_a.glob("fathomdb-*.whl"))
        if skip_build and len(cached_a) == 1:
            wheel_a = cached_a[0]
        else:
            wt_a = work_dir / "wt-0831"
            if wt_a.exists():
                subprocess.run(
                    ["git", "-C", str(REPO_ROOT), "worktree", "remove", "--force", str(wt_a)],
                    check=False,
                )
                shutil.rmtree(wt_a, ignore_errors=True)
            subprocess.run(
                [
                    "git",
                    "-C",
                    str(REPO_ROOT),
                    "worktree",
                    "add",
                    "--detach",
                    str(wt_a),
                    baseline_commit,
                ],
                check=True,
            )
            try:
                wheel_a = _build_wheel(
                    wt_a / "src/python",
                    wheels_a,
                    work_dir / "target-0831",
                )
            finally:
                subprocess.run(
                    ["git", "-C", str(REPO_ROOT), "worktree", "remove", "--force", str(wt_a)],
                    check=False,
                )
        py_a = _ensure_venv_with_wheel(venv_a, wheel_a)

    py_b = venv_b / "bin/python"
    if not (skip_build and py_b.is_file()):
        cached_b = sorted(wheels_b.glob("fathomdb-*.whl"))
        if skip_build and len(cached_b) == 1:
            wheel_b = cached_b[0]
        else:
            wheel_b = _build_wheel(
                REPO_ROOT / "src/python",
                wheels_b,
                target_dir_0832,
            )
        py_b = _ensure_venv_with_wheel(venv_b, wheel_b)

    return py_a, py_b, baseline_commit


# ---------------------------------------------------------------------------
# Worker implementations (execute inside isolated arm venv via `env -i ... -I`)
# ---------------------------------------------------------------------------


def _worker_probe() -> dict[str, Any]:
    import fathomdb
    from fathomdb import Engine, _fathomdb

    so_path = Path(_fathomdb.__file__).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="slice60-probe-") as tmp:
        eng = Engine.open(str(Path(tmp) / "probe.sqlite"), use_default_embedder=False)
        try:
            schema_ver = int(eng.open_report().schema_version_after)
        finally:
            eng.close()
    return {
        "schema_version_after": schema_ver,
        "native_path": str(so_path),
        "native_sha256": _sha256_file(so_path),
        "package_version": getattr(fathomdb, "__version__", "unknown"),
        "python_version": platform.python_version(),
        "platform": platform.platform(),
        "cpu_model": _cpu_model(),
        "cpu_count": os.cpu_count(),
    }


def _worker_build_store(spec: dict[str, Any]) -> dict[str, Any]:
    from fathomdb import Engine, ProjectionSpec

    db_path = Path(spec["db_path"])
    num_nodes = int(spec["num_nodes"])
    warmup_batches = int(spec["warmup_batches"])
    db_path.parent.mkdir(parents=True, exist_ok=True)
    for suffix in ("", "-wal", "-shm", ".lock"):
        Path(f"{db_path}{suffix}").unlink(missing_ok=True)

    t0 = time.perf_counter()
    eng = Engine.open(str(db_path), use_default_embedder=False)
    cold_open_ms = (time.perf_counter() - t0) * 1000.0
    write_node_ms: list[float] = []
    write_edge_ms: list[float] = []
    try:
        eng.configure_projections(
            [
                ProjectionSpec(name="tier", roles=frozenset({"filterable"})),
                ProjectionSpec(name="topic", roles=frozenset({"filterable"})),
            ]
        )
        for start in range(0, num_nodes, BATCH_SIZE):
            batch = []
            for i in range(start, min(start + BATCH_SIZE, num_nodes)):
                topic = TOPICS[i % len(TOPICS)]
                tier = TIERS[i % len(TIERS)]
                batch.append(
                    {
                        "kind": "note",
                        "body": json.dumps(
                            {
                                "content": f"Document {i} discusses {topic} architecture verification metric_{i % 97} benchmark",
                                "tier": tier,
                                "topic": topic,
                            }
                        ),
                        "source_id": f"src-{i % 50}",
                        "logical_id": f"node:{i}",
                    }
                )
            tw0 = time.perf_counter()
            eng.write(batch)
            write_node_ms.append((time.perf_counter() - tw0) * 1000.0)

        edge_items = []
        for i in range(num_nodes):
            edge_items.append(
                {
                    "edge": {
                        "kind": "relates_to",
                        "from": f"node:{i}",
                        "to": f"node:{(i + 1) % num_nodes}",
                        "source_id": f"src-{i % 50}",
                        "logical_id": f"edge:a:{i}",
                        "confidence": round(0.5 + (i % 50) * 0.01, 2),
                    }
                }
            )
            if i % 2 == 0:
                edge_items.append(
                    {
                        "edge": {
                            "kind": "verifies",
                            "from": f"node:{i}",
                            "to": f"node:{(i + 7) % num_nodes}",
                            "source_id": f"src-{i % 50}",
                            "logical_id": f"edge:b:{i}",
                            "confidence": 0.95,
                        }
                    }
                )
        for start in range(0, len(edge_items), BATCH_SIZE):
            tw0 = time.perf_counter()
            eng.write(edge_items[start : start + BATCH_SIZE])
            write_edge_ms.append((time.perf_counter() - tw0) * 1000.0)
        schema_ver = int(eng.open_report().schema_version_after)
    finally:
        eng.close()

    return {
        "schema_version": schema_ver,
        "cold_open_ms": cold_open_ms,
        "write_node_batch_200": write_node_ms[warmup_batches:],
        "write_edge_batch_200": write_edge_ms[warmup_batches:],
        "database_bytes": _db_total_bytes(db_path),
        "peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024,
    }


def _worker_process_open(spec: dict[str, Any]) -> dict[str, Any]:
    from fathomdb import Engine

    db_path = str(spec["db_path"])
    t0 = time.perf_counter()
    eng = Engine.open(db_path, use_default_embedder=False)
    open_ms = (time.perf_counter() - t0) * 1000.0
    eng.close()
    return {
        "open_ms": open_ms,
        "peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024,
    }


def _worker_warm_reopen(spec: dict[str, Any]) -> dict[str, Any]:
    from fathomdb import Engine

    db_path = str(spec["db_path"])
    warmup = int(spec["warmup"])
    count = int(spec["count"])
    samples: list[float] = []
    for i in range(warmup + count):
        t0 = time.perf_counter()
        eng = Engine.open(db_path, use_default_embedder=False)
        elapsed_ms = (time.perf_counter() - t0) * 1000.0
        eng.close()
        if i >= warmup:
            samples.append(elapsed_ms)
    return {
        "samples_ms": samples,
        "peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024,
    }


def _worker_query_block(spec: dict[str, Any]) -> dict[str, Any]:
    from fathomdb import (
        CurrentGraphReadContextV1,
        Engine,
        GraphExpandRequestV1,
        GraphExplicitSeedV1,
        IdSpace,
        ReadContextV1,
        graph,
    )

    db_path = str(spec["db_path"])
    num_nodes = int(spec["num_nodes"])
    warmup = int(spec["warmup"])
    query_indices: list[int] = [int(q) for q in spec["query_indices"]]

    def make_expand_req(logical_id: str) -> GraphExpandRequestV1:
        return GraphExpandRequestV1(
            schema_version=1,
            seed=GraphExplicitSeedV1(
                schema_version=1,
                type="explicit",
                logical_ids=(IdSpace(space="logical", value=logical_id),),
            ),
            direction="outgoing",
            edge_kinds=("relates_to", "verifies"),
            target_kinds=(),
            context=CurrentGraphReadContextV1(
                schema_version=1,
                type="current",
                context=ReadContextV1(),
            ),
            max_depth=2,
            result_limit=20,
            max_work_units="5000",
            include_explanation=False,
            include_evidence=False,
        )

    fts_ms: list[float] = []
    hybrid_ms: list[float] = []
    expand_ms: list[float] = []
    fts_ids: list[list[str]] = []
    hybrid_ids: list[list[str]] = []
    expand_ids: list[list[str]] = []

    eng = Engine.open(db_path, use_default_embedder=False)
    gc_was_enabled = gc.isenabled()
    try:
        for w in range(warmup):
            eng.search_text_only(f"metric_{w % 97} benchmark", limit=10)
            eng.search(f"metric_{w % 97} benchmark", limit=10)
            graph.expand(eng, make_expand_req(f"node:{w % num_nodes}"))

        gc.collect()
        gc.disable()
        for q in query_indices:
            term = f"metric_{q % 97} benchmark"

            tf0 = time.perf_counter()
            res_fts = eng.search_text_only(term, limit=10)
            fts_ms.append((time.perf_counter() - tf0) * 1000.0)
            ids_f = [h.id.value for h in res_fts.results]
            if not ids_f:
                raise RuntimeError(f"search_text_only returned 0 hits for query {q}")
            fts_ids.append(ids_f)

            th0 = time.perf_counter()
            res_hyb = eng.search(term, limit=10)
            hybrid_ms.append((time.perf_counter() - th0) * 1000.0)
            ids_h = [h.id.value for h in res_hyb.results]
            if not ids_h:
                raise RuntimeError(f"search returned 0 hits for query {q}")
            hybrid_ids.append(ids_h)

            tg0 = time.perf_counter()
            res_exp = graph.expand(eng, make_expand_req(f"node:{q % num_nodes}"))
            expand_ms.append((time.perf_counter() - tg0) * 1000.0)
            ids_e = [t.logical_id for t in res_exp.targets]
            if not ids_e:
                raise RuntimeError(f"graph.expand returned 0 targets for query {q}")
            expand_ids.append(ids_e)
    finally:
        if gc_was_enabled:
            gc.enable()
        eng.close()

    return {
        "search_text_only_ms": fts_ms,
        "search_hybrid_cpu_no_embedder_ms": hybrid_ms,
        "graph_expand_2hop_ms": expand_ms,
        "fts_ids": fts_ids,
        "hybrid_ids": hybrid_ids,
        "expand_ids": expand_ids,
        "peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024,
    }


def _worker_build_gate1_d10_store(spec: dict[str, Any]) -> dict[str, Any]:
    """Build a 10,000-node store with declared node vector projection for Gate1-D10."""
    from fathomdb import Engine, ProjectionRole, ProjectionSpec

    db_path = Path(spec["db_path"])
    num_nodes = int(spec.get("num_nodes", GATE1_D10_NODES))
    adopted_nodes = int(spec.get("adopted_nodes", GATE1_D10_ADOPTED_NODES))
    db_path.parent.mkdir(parents=True, exist_ok=True)
    for suffix in ("", "-wal", "-shm", ".lock"):
        Path(f"{db_path}{suffix}").unlink(missing_ok=True)

    eng = Engine.open(str(db_path), use_default_embedder=False)
    try:
        eng.configure_projections(
            [
                ProjectionSpec(
                    name="vec_adopted",
                    roles=frozenset({ProjectionRole.SEARCHABLE}),
                    entity_class="node",
                    vector=True,
                    vector_kinds=["adopted"],
                )
            ]
        )
        source_padding = (
            " ".join(
                f"Historical corpus paragraph {p} documents operational telemetry, "
                f"storage invariants, frozen snapshot verification, and longitudinal "
                f"query scaling across multi-megabyte SQLite pages."
                for p in range(46)
            )
        )
        for start in range(0, num_nodes, BATCH_SIZE):
            batch = []
            for i in range(start, min(start + BATCH_SIZE, num_nodes)):
                is_adopted = i < adopted_nodes
                kind = "adopted" if is_adopted else "source"
                topic = TOPICS[i % len(TOPICS)]
                base_body = (
                    f"Record {i} ({kind}) discusses {topic} "
                    f"metric_{i % 97} adopted_gate verification"
                )
                body = base_body if is_adopted else f"{base_body}. {source_padding}"
                batch.append(
                    {
                        "kind": kind,
                        "body": body,
                        "source_id": f"src-d10-{i}",
                        "logical_id": f"{kind}:{i}",
                        "provenance": {
                            "schema_version": 1,
                            "role": "canonical",
                            "artifact_revision_id": f"rev-d10-{i}",
                            "source_version_id": f"ver-d10-{i}",
                        },
                    }
                )
            eng.write(batch)
        schema_ver = int(eng.open_report().schema_version_after)
    finally:
        eng.close()

    return {
        "schema_version": schema_ver,
        "num_nodes": num_nodes,
        "adopted_nodes": adopted_nodes,
        "database_bytes": _db_total_bytes(db_path),
        "peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024,
    }


def _worker_gate1_d10_block(spec: dict[str, Any]) -> dict[str, Any]:
    """Measure paired `search_with_evidence` with `include_explanation` False & True."""
    from fathomdb import Engine, EvidenceSearchRequestV1, ReadContextV1

    db_path = str(spec["db_path"])
    warmup = int(spec["warmup"])
    query_indices: list[int] = [int(q) for q in spec["query_indices"]]

    explain_false_ms: list[float] = []
    explain_true_ms: list[float] = []
    explain_false_ids: list[list[str]] = []
    explain_true_ids: list[list[str]] = []

    eng = Engine.open(db_path, use_default_embedder=False)
    gc_was_enabled = gc.isenabled()
    try:
        frozen = eng.freeze_read_context(ReadContextV1())
        for w in range(warmup):
            w_term = f"metric_{w % 97} adopted_gate"
            eng.search_with_evidence(
                EvidenceSearchRequestV1(
                    query=w_term,
                    context=frozen,
                    include_explanation=False,
                    limit=10,
                )
            )
            eng.search_with_evidence(
                EvidenceSearchRequestV1(
                    query=w_term,
                    context=frozen,
                    include_explanation=True,
                    limit=10,
                )
            )

        gc.collect()
        gc.disable()
        for q in query_indices:
            term = f"metric_{q % 97} adopted_gate"

            t0 = time.perf_counter()
            res_plain = eng.search_with_evidence(
                EvidenceSearchRequestV1(
                    query=term,
                    context=frozen,
                    include_explanation=False,
                    limit=10,
                )
            )
            explain_false_ms.append((time.perf_counter() - t0) * 1000.0)
            if res_plain.search_result.explanation is not None:
                raise RuntimeError("expected explanation=None when include_explanation=False")
            ids_plain = [h.id.value for h in res_plain.search_result.results]
            if not ids_plain or len(res_plain.evidence) != len(ids_plain):
                raise RuntimeError(
                    f"Gate1-D10 plain search_with_evidence returned invalid hits/evidence for q={q}"
                )
            explain_false_ids.append(ids_plain)

            t1 = time.perf_counter()
            res_expl = eng.search_with_evidence(
                EvidenceSearchRequestV1(
                    query=term,
                    context=frozen,
                    include_explanation=True,
                    limit=10,
                )
            )
            explain_true_ms.append((time.perf_counter() - t1) * 1000.0)
            if res_expl.search_result.explanation is None:
                raise RuntimeError("expected explanation when include_explanation=True")
            ids_expl = [h.id.value for h in res_expl.search_result.results]
            if not ids_expl or len(res_expl.evidence) != len(ids_expl):
                raise RuntimeError(
                    f"Gate1-D10 explained search_with_evidence returned invalid hits/evidence for q={q}"
                )
            explain_true_ids.append(ids_expl)
    finally:
        if gc_was_enabled:
            gc.enable()
        eng.close()

    return {
        "explain_false_ms": explain_false_ms,
        "explain_true_ms": explain_true_ms,
        "explain_false_ids": explain_false_ids,
        "explain_true_ids": explain_true_ids,
        "peak_rss_bytes": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024,
    }


def _dispatch_worker(action: str, spec_json: Path) -> int:
    spec = json.loads(spec_json.read_text(encoding="utf-8"))
    if action == "probe":
        out = _worker_probe()
    elif action == "build_store":
        out = _worker_build_store(spec)
    elif action == "process_open":
        out = _worker_process_open(spec)
    elif action == "warm_reopen":
        out = _worker_warm_reopen(spec)
    elif action == "query_block":
        out = _worker_query_block(spec)
    elif action == "build_gate1_d10_store":
        out = _worker_build_gate1_d10_store(spec)
    elif action == "gate1_d10_block":
        out = _worker_gate1_d10_block(spec)
    else:
        raise ValueError(f"unknown worker action: {action!r}")
    print(json.dumps(out))
    return 0


# ---------------------------------------------------------------------------
# Primary paired orchestrator (10k/50k/100k paired bootstrap + Gate1-D10)
# ---------------------------------------------------------------------------


def _run_paired_scale(
    *,
    scale: int,
    py_a: Path,
    py_b: Path,
    scale_dir: Path,
    warmup_rounds: int,
    min_samples: int,
    query_samples: int,
    bootstrap_resamples: int,
    seed: int,
) -> dict[str, Any]:
    scale_dir.mkdir(parents=True, exist_ok=True)
    node_a_all: list[float] = []
    node_b_all: list[float] = []
    edge_a_all: list[float] = []
    edge_b_all: list[float] = []
    db_bytes_a_list: list[int] = []
    db_bytes_b_list: list[int] = []
    rss_a_max = 0
    rss_b_max = 0

    # Counterbalanced store builds: 4 pairs (AB/BA/AB/BA) at 10k for tighter
    # bootstrap CI on small-scale write batches, 2 pairs (AB/BA) at 50k/100k.
    num_build_pairs = 4 if scale <= 10_000 else 2
    last_db_a = scale_dir / f"pair{num_build_pairs - 1}-a.sqlite"
    last_db_b = scale_dir / f"pair{num_build_pairs - 1}-b.sqlite"
    for pair_idx in range(num_build_pairs):
        db_a = scale_dir / f"pair{pair_idx}-a.sqlite"
        db_b = scale_dir / f"pair{pair_idx}-b.sqlite"
        last_db_a, last_db_b = db_a, db_b
        order = ("A", "B") if pair_idx % 2 == 0 else ("B", "A")
        pair_res: dict[str, dict[str, Any]] = {}
        for arm in order:
            # Flush dirty pages symmetrically before each arm's build_store so
            # prior-arm writeback cannot bleed into the next arm's batch timings.
            if hasattr(os, "sync"):
                os.sync()
            py = py_a if arm == "A" else py_b
            db = db_a if arm == "A" else db_b
            pair_res[arm] = _invoke_worker(
                py,
                "build_store",
                {
                    "db_path": str(db),
                    "num_nodes": scale,
                    "warmup_batches": warmup_rounds,
                },
            )
        validate_arm_schema_versions(
            {"schema_version_after": pair_res["A"]["schema_version"]},
            {"schema_version_after": pair_res["B"]["schema_version"]},
        )
        node_a_all.extend(pair_res["A"]["write_node_batch_200"])
        node_b_all.extend(pair_res["B"]["write_node_batch_200"])
        edge_a_all.extend(pair_res["A"]["write_edge_batch_200"])
        edge_b_all.extend(pair_res["B"]["write_edge_batch_200"])
        db_bytes_a_list.append(int(pair_res["A"]["database_bytes"]))
        db_bytes_b_list.append(int(pair_res["B"]["database_bytes"]))
        rss_a_max = max(rss_a_max, int(pair_res["A"]["peak_rss_bytes"]))
        rss_b_max = max(rss_b_max, int(pair_res["B"]["peak_rss_bytes"]))
        if pair_idx < num_build_pairs - 1:
            for stale_db in (db_a, db_b):
                for suffix in ("", "-wal", "-shm", ".lock"):
                    Path(f"{stale_db}{suffix}").unlink(missing_ok=True)

    # process_open: warmup_rounds discarded + min_samples measured, interleaved AB / BA
    proc_open_a: list[float] = []
    proc_open_b: list[float] = []
    for i in range(warmup_rounds + min_samples):
        order = ("A", "B") if i % 2 == 1 else ("B", "A")
        trial: dict[str, dict[str, Any]] = {}
        for arm in order:
            py = py_a if arm == "A" else py_b
            db = last_db_a if arm == "A" else last_db_b
            trial[arm] = _invoke_worker(py, "process_open", {"db_path": str(db)})
        rss_a_max = max(rss_a_max, int(trial["A"]["peak_rss_bytes"]))
        rss_b_max = max(rss_b_max, int(trial["B"]["peak_rss_bytes"]))
        if i >= warmup_rounds:
            proc_open_a.append(float(trial["A"]["open_ms"]))
            proc_open_b.append(float(trial["B"]["open_ms"]))

    # warm_reopen: 2 counterbalanced blocks of ceil(min_samples / 2) cycles after warmup_rounds
    warm_open_a: list[float] = []
    warm_open_b: list[float] = []
    half = (min_samples + 1) // 2
    block_counts = [half, min_samples - half]
    for b_idx, cnt in enumerate(block_counts):
        order = ("A", "B") if b_idx % 2 == 0 else ("B", "A")
        blk: dict[str, dict[str, Any]] = {}
        for arm in order:
            py = py_a if arm == "A" else py_b
            db = last_db_a if arm == "A" else last_db_b
            blk[arm] = _invoke_worker(
                py,
                "warm_reopen",
                {"db_path": str(db), "warmup": warmup_rounds, "count": cnt},
            )
        warm_open_a.extend(blk["A"]["samples_ms"])
        warm_open_b.extend(blk["B"]["samples_ms"])
        rss_a_max = max(rss_a_max, int(blk["A"]["peak_rss_bytes"]))
        rss_b_max = max(rss_b_max, int(blk["B"]["peak_rss_bytes"]))

    # Queries: 5 balanced AB/BA blocks covering `query_samples` queries (10 warmup per block)
    num_blocks = 5
    per_block = max(1, query_samples // num_blocks)
    fts_a: list[float] = []
    fts_b: list[float] = []
    hyb_a: list[float] = []
    hyb_b: list[float] = []
    exp_a: list[float] = []
    exp_b: list[float] = []
    parity_mismatches: list[str] = []

    for b_idx in range(num_blocks):
        start_q = b_idx * per_block
        end_q = query_samples if b_idx == num_blocks - 1 else min(query_samples, start_q + per_block)
        q_indices = list(range(start_q, end_q))
        order = ("A", "B") if b_idx % 2 == 0 else ("B", "A")
        q_res: dict[str, dict[str, Any]] = {}
        for arm in order:
            py = py_a if arm == "A" else py_b
            db = last_db_a if arm == "A" else last_db_b
            q_res[arm] = _invoke_worker(
                py,
                "query_block",
                {
                    "db_path": str(db),
                    "num_nodes": scale,
                    "warmup": max(10, warmup_rounds),
                    "query_indices": q_indices,
                },
            )
        fts_a.extend(q_res["A"]["search_text_only_ms"])
        fts_b.extend(q_res["B"]["search_text_only_ms"])
        hyb_a.extend(q_res["A"]["search_hybrid_cpu_no_embedder_ms"])
        hyb_b.extend(q_res["B"]["search_hybrid_cpu_no_embedder_ms"])
        exp_a.extend(q_res["A"]["graph_expand_2hop_ms"])
        exp_b.extend(q_res["B"]["graph_expand_2hop_ms"])
        rss_a_max = max(rss_a_max, int(q_res["A"]["peak_rss_bytes"]))
        rss_b_max = max(rss_b_max, int(q_res["B"]["peak_rss_bytes"]))

        block_parity = evaluate_logical_id_parity(
            {k: q_res["A"][k] for k in ("fts_ids", "hybrid_ids", "expand_ids")},
            {k: q_res["B"][k] for k in ("fts_ids", "hybrid_ids", "expand_ids")},
            context_label=f"scale={scale} block={b_idx}",
            query_indices=q_indices,
        )
        parity_mismatches.extend(block_parity["sample_mismatches"])

    latency_gates = {
        "process_open": evaluate_paired_latency_metric(
            proc_open_a,
            proc_open_b,
            min_samples=min_samples,
            resamples=bootstrap_resamples,
            seed=seed,
        ),
        "warm_reopen": evaluate_paired_latency_metric(
            warm_open_a,
            warm_open_b,
            min_samples=min_samples,
            resamples=bootstrap_resamples,
            seed=seed,
        ),
        "write_node_batch_200": evaluate_paired_latency_metric(
            node_a_all,
            node_b_all,
            min_samples=min_samples,
            resamples=bootstrap_resamples,
            seed=seed,
        ),
        "write_edge_batch_200": evaluate_paired_latency_metric(
            edge_a_all,
            edge_b_all,
            min_samples=min_samples,
            resamples=bootstrap_resamples,
            seed=seed,
        ),
        "search_text_only": evaluate_paired_latency_metric(
            fts_a,
            fts_b,
            min_samples=min_samples,
            resamples=bootstrap_resamples,
            seed=seed,
        ),
        "search_hybrid_cpu_no_embedder": evaluate_paired_latency_metric(
            hyb_a,
            hyb_b,
            min_samples=min_samples,
            resamples=bootstrap_resamples,
            seed=seed,
        ),
        "graph_expand_2hop": evaluate_paired_latency_metric(
            exp_a,
            exp_b,
            min_samples=min_samples,
            resamples=bootstrap_resamples,
            seed=seed,
        ),
    }

    db_gate = evaluate_database_bytes(max(db_bytes_a_list), max(db_bytes_b_list))
    rss_gate = evaluate_peak_rss_bytes(rss_a_max, rss_b_max)
    parity_passed = len(parity_mismatches) == 0

    all_passed = (
        all(g["passed"] for g in latency_gates.values())
        and db_gate["passed"]
        and rss_gate["passed"]
        and parity_passed
    )
    return {
        "scale_nodes": scale,
        "scale_edges": scale + (scale + 1) // 2,
        "latency_gates": latency_gates,
        "database_bytes": db_gate,
        "peak_rss_bytes": rss_gate,
        "logical_id_parity": {
            "passed": parity_passed,
            "mismatch_count": len(parity_mismatches),
            "sample_mismatches": parity_mismatches[:10],
        },
        "passed": all_passed,
    }


def _run_gate1_d10_paired(
    *,
    py_a: Path,
    py_b: Path,
    gate_dir: Path,
    warmup_rounds: int,
    samples: int,
) -> dict[str, Any]:
    """Run paired 10k `search_with_evidence` vector-projection Gate1-D10 check (<= 0.15x)."""
    gate_dir.mkdir(parents=True, exist_ok=True)
    db_a = gate_dir / "gate1-d10-v0831.sqlite"
    db_b = gate_dir / "gate1-d10-0832.sqlite"
    effective_samples = max(samples, GATE1_D10_MIN_SAMPLES)
    effective_warmup = max(warmup_rounds, 5)

    build_a = _invoke_worker(
        py_a,
        "build_gate1_d10_store",
        {
            "db_path": str(db_a),
            "num_nodes": GATE1_D10_NODES,
            "adopted_nodes": GATE1_D10_ADOPTED_NODES,
        },
    )
    build_b = _invoke_worker(
        py_b,
        "build_gate1_d10_store",
        {
            "db_path": str(db_b),
            "num_nodes": GATE1_D10_NODES,
            "adopted_nodes": GATE1_D10_ADOPTED_NODES,
        },
    )
    validate_arm_schema_versions(
        {"schema_version_after": build_a["schema_version"]},
        {"schema_version_after": build_b["schema_version"]},
    )

    # 2 counterbalanced AB / BA blocks covering `effective_samples` paired queries
    half = (effective_samples + 1) // 2
    block_ranges = [list(range(0, half)), list(range(half, effective_samples))]

    plain_a: list[float] = []
    plain_b: list[float] = []
    expl_a: list[float] = []
    expl_b: list[float] = []
    parity_mismatches: list[str] = []
    rss_a_max = int(build_a["peak_rss_bytes"])
    rss_b_max = int(build_b["peak_rss_bytes"])

    for b_idx, q_indices in enumerate(block_ranges):
        order = ("A", "B") if b_idx % 2 == 0 else ("B", "A")
        blk: dict[str, dict[str, Any]] = {}
        for arm in order:
            py = py_a if arm == "A" else py_b
            db = db_a if arm == "A" else db_b
            blk[arm] = _invoke_worker(
                py,
                "gate1_d10_block",
                {
                    "db_path": str(db),
                    "warmup": effective_warmup,
                    "query_indices": q_indices,
                },
            )
        plain_a.extend(blk["A"]["explain_false_ms"])
        plain_b.extend(blk["B"]["explain_false_ms"])
        expl_a.extend(blk["A"]["explain_true_ms"])
        expl_b.extend(blk["B"]["explain_true_ms"])
        rss_a_max = max(rss_a_max, int(blk["A"]["peak_rss_bytes"]))
        rss_b_max = max(rss_b_max, int(blk["B"]["peak_rss_bytes"]))

        blk_parity = evaluate_logical_id_parity(
            {
                "explain_false_ids": blk["A"]["explain_false_ids"],
                "explain_true_ids": blk["A"]["explain_true_ids"],
            },
            {
                "explain_false_ids": blk["B"]["explain_false_ids"],
                "explain_true_ids": blk["B"]["explain_true_ids"],
            },
            context_label=f"gate1_d10 block={b_idx}",
            query_indices=q_indices,
        )
        parity_mismatches.extend(blk_parity["sample_mismatches"])

    gate_plain = evaluate_gate1_d10_ratio_gate(
        plain_a,
        plain_b,
        max_ratio=GATE1_D10_MAX_RATIO,
        min_samples=GATE1_D10_MIN_SAMPLES,
    )
    gate_expl = evaluate_gate1_d10_ratio_gate(
        expl_a,
        expl_b,
        max_ratio=GATE1_D10_MAX_RATIO,
        min_samples=GATE1_D10_MIN_SAMPLES,
    )
    parity_passed = len(parity_mismatches) == 0
    passed = bool(gate_plain["passed"] and gate_expl["passed"] and parity_passed)

    return {
        "num_nodes": GATE1_D10_NODES,
        "adopted_nodes": GATE1_D10_ADOPTED_NODES,
        "warmup_rounds": effective_warmup,
        "paired_samples": effective_samples,
        "max_ratio": GATE1_D10_MAX_RATIO,
        "include_explanation_false": gate_plain,
        "include_explanation_true": gate_expl,
        "logical_id_parity": {
            "passed": parity_passed,
            "mismatch_count": len(parity_mismatches),
            "sample_mismatches": parity_mismatches[:10],
        },
        "arm_a_peak_rss_bytes": rss_a_max,
        "arm_b_peak_rss_bytes": rss_b_max,
        "passed": passed,
    }


def _run_primary_gate(args: argparse.Namespace) -> int:
    validate_warmup_rounds(args.warmup_rounds)
    work_dir = Path(args.work_dir).resolve()
    scales = [int(s.strip()) for s in args.scales.split(",") if s.strip()]
    if not scales:
        raise ValueError("--scales must contain at least one integer scale")

    py_a, py_b, baseline_commit = prepare_arms(
        work_dir=work_dir,
        baseline_sha=args.baseline_sha,
        target_dir_0832=Path(args.target_dir_0832).resolve(),
        skip_build=args.skip_build,
    )

    probe_a = _invoke_worker(py_a, "probe", {})
    probe_b = _invoke_worker(py_b, "probe", {})
    validate_arm_schema_versions(probe_a, probe_b)

    scale_reports: dict[str, dict[str, Any]] = {}
    for scale in scales:
        scale_reports[str(scale)] = _run_paired_scale(
            scale=scale,
            py_a=py_a,
            py_b=py_b,
            scale_dir=work_dir / f"scale-{scale}",
            warmup_rounds=args.warmup_rounds,
            min_samples=args.min_samples,
            query_samples=args.query_samples,
            bootstrap_resamples=args.bootstrap_resamples,
            seed=args.seed,
        )

    gate1_d10 = _run_gate1_d10_paired(
        py_a=py_a,
        py_b=py_b,
        gate_dir=work_dir / "gate1-d10-10k",
        warmup_rounds=args.warmup_rounds,
        samples=args.gate1_d10_samples,
    )

    head_sha = subprocess.check_output(
        ["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"], text=True
    ).strip()

    overall_passed = (
        all(r["passed"] for r in scale_reports.values())
        and gate1_d10["passed"]
    )

    report = {
        "schema": "fathomdb.0832-paired-latency-gate/v1",
        "baseline_ref": args.baseline_sha,
        "baseline_sha": baseline_commit,
        "candidate_sha": head_sha,
        "arm_a": probe_a,
        "arm_b": probe_b,
        "config": {
            "scales": scales,
            "warmup_rounds": args.warmup_rounds,
            "min_samples": args.min_samples,
            "query_samples": args.query_samples,
            "gate1_d10_samples": max(args.gate1_d10_samples, GATE1_D10_MIN_SAMPLES),
            "bootstrap_resamples": args.bootstrap_resamples,
            "seed": args.seed,
        },
        "paired_scales": scale_reports,
        "gate1_d10_10k": gate1_d10,
        "passed": overall_passed,
    }

    out_path = Path(args.out).resolve()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "out": str(out_path),
                "passed": overall_passed,
                "scales_passed": {k: v["passed"] for k, v in scale_reports.items()},
                "gate1_d10_10k_passed": gate1_d10["passed"],
            },
            indent=2,
        )
    )
    return 0 if overall_passed else 1


# ---------------------------------------------------------------------------
# Secondary mode: pinned 10k synthetic `experiments/sdk_benchmark.py` suite
# ---------------------------------------------------------------------------


def _write_json_binding(path: Path, data: dict[str, Any]) -> dict[str, str]:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(data, sort_keys=True, indent=2, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return {"path": str(path.resolve()), "sha256": _sha256_file(path)}


def _run_sdk_benchmark_10k(args: argparse.Namespace) -> int:
    """Generate a pinned 10k `sdk.experiment.v1` suite and execute `experiments/sdk_benchmark.py`."""
    if str(REPO_ROOT) not in sys.path:
        sys.path.insert(0, str(REPO_ROOT))
    from experiments import sdk_benchmark
    from fathomdb import Engine, ReadView, _fathomdb

    suite_out = Path(args.sdk_suite_out).resolve()
    sdk_out = Path(args.sdk_out).resolve()
    receipts_root = Path(args.sdk_receipts_root).resolve()
    suite_dir = suite_out.parent
    suite_dir.mkdir(parents=True, exist_ok=True)
    receipts_root.mkdir(parents=True, exist_ok=True)
    if sdk_out.exists():
        shutil.rmtree(sdk_out)

    # Clear undeclared FATHOMDB_* overrides so sdk_benchmark._check_treatment_environment passes.
    for key in list(os.environ):
        if key.startswith("FATHOMDB_") and key != "FATHOMDB_BENCH_NODE":
            os.environ.pop(key, None)

    head_sha = subprocess.check_output(
        ["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"], text=True
    ).strip()
    ws_ver = tomllib.loads((REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8"))[
        "workspace"
    ]["package"]["version"]

    # 1. Build 10,000-doc synthetic fixture + 30 queries + exact Python search_text_only oracle.
    corpus = [
        {
            "id": f"doc-{i:05d}",
            "body": (
                f"Document {i} covers {TOPICS[i % len(TOPICS)]} "
                f"component_{i % 101} benchmark synthetic retrieval"
            ),
        }
        for i in range(10_000)
    ]
    queries = [
        {"id": f"q{q:02d}", "text": f"component_{q % 101} synthetic"}
        for q in range(30)
    ]

    with tempfile.TemporaryDirectory(prefix="slice60-sdk-oracle-") as tmp:
        oracle_db = Path(tmp) / "oracle.sqlite"
        eng = Engine.open(str(oracle_db), use_default_embedder=False)
        try:
            for start in range(0, len(corpus), BATCH_SIZE):
                eng.write(
                    [
                        {
                            "kind": "doc",
                            "body": row["body"],
                            "logical_id": row["id"],
                            "source_id": f"sdk:{row['id']}",
                        }
                        for row in corpus[start : start + BATCH_SIZE]
                    ]
                )
            view = ReadView(valid_as_of=1700000000)
            oracle_fts: dict[str, list[str]] = {}
            gold: dict[str, list[str]] = {}
            for q in queries:
                res = eng.search_text_only(q["text"], view=view, limit=10)
                ids = [f"l:{h.id.value}" for h in res.results]
                oracle_fts[q["id"]] = ids
                gold[q["id"]] = [h.id.value for h in res.results]
        finally:
            eng.close()

    fixture_doc = {
        "schema_version": "sdk.fixture.v1",
        "corpus": corpus,
        "queries": queries,
        "gold": gold,
        "oracles": {"engine.search_text_only": oracle_fts},
    }
    fixture_binding = _write_json_binding(suite_dir / "sdk-fixture-10k.json", fixture_doc)

    # 2. Resolve pinned bootstrap CLI binary + dependency closure digest.
    cli_bin = REPO_ROOT / "target/release/fathomdb"
    if not cli_bin.is_file():
        cli_bin = REPO_ROOT / "target/debug/fathomdb"
    if not cli_bin.is_file():
        raise FileNotFoundError(
            "expected built fathomdb CLI binary under target/{release,debug}/fathomdb"
        )
    cli_binding = {"path": str(cli_bin.resolve()), "sha256": _sha256_file(cli_bin)}
    cargo_lock_sha = _sha256_file(REPO_ROOT / "Cargo.lock")

    arms_spec: list[dict[str, Any]] = []

    # Python arm (always present)
    py_runtime = Path(sys.executable).resolve(strict=True)
    py_driver = (REPO_ROOT / "experiments/sdk_drivers/python_driver.py").resolve(strict=True)
    py_native = Path(_fathomdb.__file__).resolve(strict=True)
    py_arm = {
        "schema_version": "sdk.arm.v1",
        "sdk": "python",
        "sdk_version": ws_ver,
        "engine_revision": head_sha,
        "dependency_closure_sha256": cargo_lock_sha,
        "wrapper_sha256": sdk_benchmark.wrapper_sha256("python", native_path=py_native),
        "features": [],
        "device": "cpu",
        "runtime": {"path": str(py_runtime), "sha256": _sha256_file(py_runtime)},
        "driver": {"path": str(py_driver), "sha256": _sha256_file(py_driver)},
        "native": {"path": str(py_native), "sha256": _sha256_file(py_native)},
        "bootstrap_cli": cli_binding,
    }
    arms_spec.append(
        {
            "sdk": "python",
            "artifact": _write_json_binding(suite_dir / "arm-python.json", py_arm),
        }
    )

    # TypeScript arm (if built .node addon + node runtime exist)
    ts_nodes = sorted((REPO_ROOT / "src/ts").glob("fathomdb.*.node"))
    node_bin = shutil.which("node")
    if ts_nodes and node_bin:
        node_path = Path(node_bin).resolve(strict=True)
        ts_driver = (REPO_ROOT / "experiments/sdk_drivers/typescript_driver.mjs").resolve(
            strict=True
        )
        ts_native = ts_nodes[0].resolve(strict=True)
        ts_arm = {
            "schema_version": "sdk.arm.v1",
            "sdk": "typescript",
            "sdk_version": ws_ver,
            "engine_revision": head_sha,
            "dependency_closure_sha256": cargo_lock_sha,
            "wrapper_sha256": sdk_benchmark.wrapper_sha256("typescript"),
            "features": [],
            "device": "cpu",
            "runtime": {"path": str(node_path), "sha256": _sha256_file(node_path)},
            "driver": {"path": str(ts_driver), "sha256": _sha256_file(ts_driver)},
            "native": {"path": str(ts_native), "sha256": _sha256_file(ts_native)},
            "bootstrap_cli": cli_binding,
        }
        arms_spec.append(
            {
                "sdk": "typescript",
                "artifact": _write_json_binding(suite_dir / "arm-typescript.json", ts_arm),
            }
        )

    # Rust arm (if standalone benchmark driver binary is built)
    rust_driver_bin = (
        REPO_ROOT / "experiments/sdk_drivers/rust/target/release/fathomdb-sdk-benchmark-driver"
    )
    if not rust_driver_bin.is_file():
        rust_driver_bin = REPO_ROOT / "target/release/fathomdb-sdk-benchmark-driver"
    if rust_driver_bin.is_file():
        rust_path = rust_driver_bin.resolve(strict=True)
        rust_sha = _sha256_file(rust_path)
        rust_arm = {
            "schema_version": "sdk.arm.v1",
            "sdk": "rust",
            "sdk_version": ws_ver,
            "engine_revision": head_sha,
            "dependency_closure_sha256": cargo_lock_sha,
            "wrapper_sha256": sdk_benchmark.wrapper_sha256("rust", native_sha256=rust_sha),
            "features": [],
            "device": "cpu",
            "runtime": {"path": str(rust_path), "sha256": rust_sha},
            "driver": {"path": str(rust_path), "sha256": rust_sha},
            "native": {"path": str(rust_path), "sha256": rust_sha},
            "bootstrap_cli": cli_binding,
        }
        arms_spec.append(
            {
                "sdk": "rust",
                "artifact": _write_json_binding(suite_dir / "arm-rust.json", rust_arm),
            }
        )

    suite_doc = {
        "schema_version": "sdk.experiment.v1",
        "program_track": None,
        "suite_id": "slice60-synthetic-10k-v1",
        "fixtures": {"manifest": fixture_binding},
        "arms": arms_spec,
        "cases": [
            {
                "id": "fts-10k-latency",
                "family": "latency_capacity",
                "operation_id": "engine.search_text_only",
                "operation_version": 1,
                "scale_tier": "synthetic-10k",
                "required": True,
                "arguments": {
                    "query_set": "all",
                    "limit": 10,
                    "view": {"valid_as_of": 1700000000},
                },
                "setup": "canonical-nodes-v1",
                "boundary": "sdk_call_to_materialized_result",
            }
        ],
        "sampling": {
            "warmup_per_query": 1,
            "repetitions": 4,
            "seed": 20261126,
            "cache_state": "steady_after_declared_warmup",
            "concurrency": 1,
            "order": "balanced",
        },
        "output": {
            "result_schema": "sdk.result.v1",
            "metrics": [
                "p50_ms",
                "p95_ms",
                "p99_ms",
                "queries_per_s",
                "database_bytes",
                "driver_peak_rss_bytes",
            ],
        },
    }
    _write_json_binding(suite_out, suite_doc)

    sdk_benchmark.validate_experiment(suite_out, sdk_out)
    manifest = sdk_benchmark.run_experiment(
        suite_out,
        sdk_out,
        receipts_root=receipts_root,
    )
    print(json.dumps(manifest, indent=2, sort_keys=True))
    failed_cells = [c for c in manifest.get("cells", []) if c.get("status") != "complete"]
    return 0 if not failed_cells else 1


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", default=None, help=argparse.SUPPRESS)
    parser.add_argument("--spec-json", type=Path, default=None, help=argparse.SUPPRESS)

    # Primary paired cross-version gate flags
    parser.add_argument("--baseline-sha", default=DEFAULT_BASELINE_SHA)
    parser.add_argument("--scales", default="10000,50000,100000")
    parser.add_argument("--warmup-rounds", type=int, default=5)
    parser.add_argument("--min-samples", type=int, default=30)
    parser.add_argument("--query-samples", type=int, default=100)
    parser.add_argument("--gate1-d10-samples", type=int, default=30)
    parser.add_argument("--bootstrap-resamples", type=int, default=10_000)
    parser.add_argument("--seed", type=int, default=20261126)
    parser.add_argument("--work-dir", type=Path, default=DEFAULT_WORK_DIR)
    parser.add_argument("--target-dir-0832", type=Path, default=REPO_ROOT / "target")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT_JSON)
    parser.add_argument("--skip-build", action="store_true")

    # Secondary `experiments/sdk_benchmark.py` 10k mode
    parser.add_argument("--run-sdk-benchmark-10k", action="store_true")
    parser.add_argument(
        "--sdk-suite-out",
        type=Path,
        default=Path("/tmp/fathomdb-0832-qual/sdk-suite-10k.json"),
    )
    parser.add_argument(
        "--sdk-out",
        type=Path,
        default=Path("/tmp/fathomdb-0832-qual/sdk-benchmark-out"),
    )
    parser.add_argument(
        "--sdk-receipts-root",
        type=Path,
        default=Path("/tmp/fathomdb-0832-qual/sdk-receipts"),
    )

    args = parser.parse_args(argv)
    if args.worker is not None:
        if args.spec_json is None:
            parser.error("--worker requires --spec-json")
        return _dispatch_worker(args.worker, args.spec_json)
    if args.run_sdk_benchmark_10k:
        return _run_sdk_benchmark_10k(args)
    if args.warmup_rounds < 5:
        parser.error("--warmup-rounds must be >= 5 per AC32-60-02")
    return _run_primary_gate(args)


if __name__ == "__main__":
    raise SystemExit(main())
