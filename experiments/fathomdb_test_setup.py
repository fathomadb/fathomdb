"""Create one configured FathomDB database and doctor evidence per test cell."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Callable


_DEVICE = re.compile(r"(?:auto|cpu|cuda:[0-9]+)")
_TEST_ID = re.compile(r"[a-z0-9][a-z0-9-]*")


@dataclass(frozen=True)
class PreparedDatabase:
    """Paths and safe diagnostics for one newly-created experiment database."""

    database_path: Path
    config_path: Path
    doctor_path: Path


DoctorRunner = Callable[..., str]
DatabaseOpener = Callable[[Path, bool], dict[str, Any]]


def _default_doctor(command: list[str], *, env: dict[str, str]) -> str:
    result = subprocess.run(command, check=False, text=True, capture_output=True, env=env)
    if result.returncode != 0:
        raise RuntimeError(f"fathomdb doctor failed: {' '.join(command[1:3])}")
    return result.stdout


def _default_open(path: Path, use_default_embedder: bool) -> dict[str, Any]:
    from fathomdb import Engine

    engine = Engine.open(str(path), use_default_embedder=use_default_embedder)
    try:
        report = engine.open_report()
        resolution = report.embedder_device_resolution
        if use_default_embedder and resolution is None:
            raise RuntimeError("default embedder opened without a device resolution")
        requested = os.environ["FATHOMDB_EMBED_DEVICE"]
        if requested.startswith("cuda:") and (resolution is None or resolution.effective_device.kind != "cuda" or not resolution.selected_cuda_uuid):
            raise RuntimeError("explicit CUDA policy did not produce a selected CUDA embedder")
        return {
            "query_backend": report.query_backend,
            "schema_version_after": report.schema_version_after,
            "embedder_download_ms": report.embedder_download_ms,
            "embedder_events": report.embedder_events,
            "embedder_device_resolution": None if resolution is None else asdict(resolution),
            "embedder_gpu_allocation_witness": None if report.embedder_gpu_allocation_witness is None else asdict(report.embedder_gpu_allocation_witness),
        }
    finally:
        engine.close()


def prepare_test_database(
    root: str | Path,
    *,
    test_id: str,
    embed_device: str = "auto",
    rerank_device: str = "auto",
    embedder: str = "none",
    warm_cache: bool = False,
    check_reranker: bool = True,
    fathomdb_bin: str = "fathomdb",
    doctor_runner: DoctorRunner = _default_doctor,
    database_opener: DatabaseOpener = _default_open,
) -> PreparedDatabase:
    """Create an unreused database, configure its runtime policy, and attest it.

    ``root/test_id`` must not already exist: reusing state would invalidate a
    measurement cell. ``default`` is FathomDB's only Python-selectable model;
    it must use explicit cache warming so a measured open never hides network
    download time. CUDA is strict rather than an ``auto`` CPU fallback.
    """
    if not _TEST_ID.fullmatch(test_id):
        raise ValueError("test_id must contain only lowercase letters, digits, and hyphens")
    if not _DEVICE.fullmatch(embed_device) or not _DEVICE.fullmatch(rerank_device):
        raise ValueError("device policies must be auto, cpu, or cuda:N")
    if embedder not in {"none", "default"}:
        raise ValueError("embedder must be none or default")
    if embedder == "default" and not warm_cache:
        raise ValueError("default embedder requires warm_cache=True")
    if embed_device.startswith("cuda:") and embedder != "default":
        raise ValueError("CUDA embedder policy requires embedder='default'")
    test_root = Path(root) / test_id
    if test_root.exists():
        raise FileExistsError(f"test database root already exists: {test_root}")
    test_root.mkdir(parents=True, mode=0o700)
    database_path = test_root / "fathomdb.sqlite"
    config_path = test_root / "fathomdb-config.v1.json"
    doctor_path = test_root / "fathomdb-doctor.v1.json"
    config = {
        "schema_version": "fathomdb.test-setup.v1",
        "embed_device": embed_device,
        "rerank_device": rerank_device if check_reranker else None,
        "cross_encoder": "enabled" if check_reranker else "disabled",
        "embedder": embedder,
    }
    config_path.write_text(json.dumps(config, sort_keys=True) + "\n", encoding="utf-8")
    config_path.chmod(0o600)
    environment = dict(os.environ)
    environment.update({"FATHOMDB_EMBED_DEVICE": embed_device, "FATHOMDB_RERANK_DEVICE": rerank_device})
    gpu = doctor_runner([fathomdb_bin, "doctor", "gpu", "--json"], env=environment)
    reranker = (
        json.loads(doctor_runner([fathomdb_bin, "doctor", "reranker-gpu", "--json"], env=environment))
        if check_reranker
        else {"status": "not_applicable", "reason": "cross_encoder_disabled"}
    )
    cache: dict[str, Any] | None = None
    if warm_cache:
        started = time.monotonic()
        cache = {"doctor": json.loads(doctor_runner([fathomdb_bin, "doctor", "warm-cache", "--json"], env=environment)), "elapsed_ms": round((time.monotonic() - started) * 1000)}
    prior_embed, prior_rerank = os.environ.get("FATHOMDB_EMBED_DEVICE"), os.environ.get("FATHOMDB_RERANK_DEVICE")
    os.environ.update({"FATHOMDB_EMBED_DEVICE": embed_device, "FATHOMDB_RERANK_DEVICE": rerank_device})
    try:
        opened = database_opener(database_path, embedder == "default")
        database_path.chmod(0o600)
    finally:
        for key, prior in (("FATHOMDB_EMBED_DEVICE", prior_embed), ("FATHOMDB_RERANK_DEVICE", prior_rerank)):
            if prior is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = prior
    integrity = doctor_runner([fathomdb_bin, "doctor", "check-integrity", "--json", str(database_path)], env=environment)
    doctor_path.write_text(json.dumps({"schema_version": "fathomdb.test-doctor.v1", "gpu": json.loads(gpu), "reranker_gpu": reranker, "cache": cache, "open_report": opened, "integrity": json.loads(integrity)}, sort_keys=True) + "\n", encoding="utf-8")
    doctor_path.chmod(0o600)
    return PreparedDatabase(database_path=database_path, config_path=config_path, doctor_path=doctor_path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("test_id")
    parser.add_argument("--embed-device", default="auto")
    parser.add_argument("--rerank-device", default="auto")
    parser.add_argument("--embedder", choices=("none", "default"), default="none")
    parser.add_argument("--warm-cache", action="store_true")
    parser.add_argument("--disable-cross-encoder", action="store_true")
    parser.add_argument("--fathomdb-bin", default="fathomdb")
    args = parser.parse_args()
    prepared = prepare_test_database(args.root, test_id=args.test_id, embed_device=args.embed_device, rerank_device=args.rerank_device, embedder=args.embedder, warm_cache=args.warm_cache, check_reranker=not args.disable_cross_encoder, fathomdb_bin=args.fathomdb_bin)
    print(json.dumps({"database_path": str(prepared.database_path), "config_path": str(prepared.config_path), "doctor_path": str(prepared.doctor_path)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
