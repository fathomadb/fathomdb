"""Validate a pinned vec1bench HDF5 corpus and stream it to benchmark files.

Requires ``h5py`` and ``numpy`` only for this explicit, offline preparation step.
The resulting files belong in external scratch space, never in the repository.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
from typing import Any


REGISTRY = Path(__file__).resolve().parents[2] / "dev/vector-bench-corpora/vec1bench/datasets.json"
CHUNK_ROWS = 4096


def load_specs() -> dict[str, dict[str, Any]]:
    """Return the four pinned corpus specifications, keyed by dataset ID."""
    registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
    if registry.get("schema_version") != 1:
        raise ValueError("unsupported vec1bench registry schema")
    specs = {item["id"]: item for item in registry["datasets"]}
    if len(specs) != len(registry["datasets"]):
        raise ValueError("duplicate vec1bench dataset ID")
    return specs


def _text(value: Any) -> str:
    return value.decode("utf-8") if isinstance(value, bytes) else str(value)


def validate_layout(source: Any, spec: dict[str, Any]) -> tuple[int, int]:
    """Reject a changed metric, dimension, row count, or incomplete exact gold."""
    attrs = source.attrs
    if int(attrs.get("dimension", -1)) != spec["dimension"]:
        raise ValueError("dimension does not match pinned corpus")
    if _text(attrs.get("distance", "")) != spec["source_distance"]:
        raise ValueError("distance does not match pinned corpus")
    if _text(attrs.get("point_type", "")) != "float":
        raise ValueError("point_type must be float")

    train = source["train"]
    test = source["test"]
    neighbors = source["neighbors"]
    distances = source["distances"]
    n_train, dim = spec["train_rows"], spec["dimension"]
    if train.shape != (n_train, dim) or train.dtype.kind != "f":
        raise ValueError("train layout does not match pinned corpus")
    if len(test.shape) != 2 or test.shape[1] != dim or test.dtype.kind != "f":
        raise ValueError("test layout does not match pinned corpus")
    n_test = test.shape[0]
    if n_test < 100:
        raise ValueError("test needs at least 100 held-out queries")
    if len(neighbors.shape) != 2 or neighbors.shape[0] != n_test or neighbors.shape[1] < 10:
        raise ValueError("neighbors must contain exact top-10 for every test query")
    if neighbors.dtype.kind not in {"i", "u"}:
        raise ValueError("neighbors must contain integer corpus IDs")
    if distances.shape != neighbors.shape or distances.dtype.kind != "f":
        raise ValueError("distances must align with neighbors")
    return n_train, n_test


def query_indices(n_test: int, count: int) -> list[int]:
    """Select a deterministic sample spanning the complete held-out query set."""
    if count < 2 or n_test < count:
        raise ValueError("query sample count must be in 2..=n_test")
    return [(i * (n_test - 1)) // (count - 1) for i in range(count)]


def unit_l2(row: list[float]) -> list[float]:
    """Map cosine vectors to unit L2 vectors, rejecting undefined zero norms."""
    norm = math.sqrt(sum(float(value) * float(value) for value in row))
    if not math.isfinite(norm) or norm == 0.0:
        raise ValueError("zero or non-finite cosine vector")
    return [float(value) / norm for value in row]


def sha256_file(path: Path) -> str:
    """Hash a complete local source file before parsing its HDF5 contents."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _write_vectors(dataset: Any, target: Path, normalize: bool) -> str:
    import numpy as np

    digest = hashlib.sha256()
    with target.open("wb") as stream:
        for start in range(0, dataset.shape[0], CHUNK_ROWS):
            rows = np.asarray(dataset[start : start + CHUNK_ROWS], dtype="<f4")
            if not np.isfinite(rows).all():
                raise ValueError("vector payload contains non-finite values")
            if normalize:
                norms = np.linalg.norm(rows.astype(np.float64), axis=1)
                if not np.isfinite(norms).all() or np.any(norms == 0.0):
                    raise ValueError("zero or non-finite cosine vector")
                rows = np.asarray(rows / norms[:, None], dtype="<f4")
            payload = rows.tobytes(order="C")
            stream.write(payload)
            digest.update(payload)
    return digest.hexdigest()


def _write_gold(source: Any, target: Path, n_train: int) -> str:
    import numpy as np

    digest = hashlib.sha256()
    neighbors = source["neighbors"]
    distances = source["distances"]
    with target.open("wb") as stream:
        for start in range(0, neighbors.shape[0], CHUNK_ROWS):
            gold = np.asarray(neighbors[start : start + CHUNK_ROWS, :10], dtype=np.int64)
            dists = np.asarray(distances[start : start + CHUNK_ROWS, :10], dtype=np.float64)
            if np.any(gold < 0) or np.any(gold >= n_train):
                raise ValueError("exact-gold neighbor ID outside training population")
            if np.any(np.diff(np.sort(gold, axis=1), axis=1) == 0):
                raise ValueError("exact-gold top-10 contains duplicate IDs")
            if not np.isfinite(dists).all() or np.any(np.diff(dists, axis=1) < -1e-8):
                raise ValueError("exact-gold distances are invalid or unsorted")
            payload = gold.astype("<u4").tobytes(order="C")
            stream.write(payload)
            digest.update(payload)
    return digest.hexdigest()


def prepare_dataset(dataset_id: str, source_path: Path, output_root: Path) -> Path:
    """Validate a local pinned HDF5 file and materialize sealed binary inputs."""
    spec = load_specs()[dataset_id]
    if not source_path.is_file():
        raise FileNotFoundError(f"source corpus absent: {source_path}")
    observed = sha256_file(source_path)
    if observed != spec["sha256"]:
        raise ValueError(f"source SHA256 mismatch: expected {spec['sha256']}, got {observed}")

    try:
        import h5py
    except ImportError as error:
        raise RuntimeError("preparation requires h5py and numpy in this Python environment") from error

    destination = output_root / dataset_id
    destination.mkdir(parents=True, exist_ok=True)
    with h5py.File(source_path, "r") as source:
        n_train, n_test = validate_layout(source, spec)
        normalize = spec["fathom_transform"] == "unit_l2"
        train_tmp = destination / "train.f32.tmp"
        test_tmp = destination / "test.f32.tmp"
        gold_tmp = destination / "neighbors10.u32.tmp"
        train_sha = _write_vectors(source["train"], train_tmp, normalize)
        test_sha = _write_vectors(source["test"], test_tmp, normalize)
        gold_sha = _write_gold(source, gold_tmp, n_train)

    for name in ("train.f32", "test.f32", "neighbors10.u32"):
        os.replace(destination / f"{name}.tmp", destination / name)
    seal = {
        "schema_version": 1,
        "dataset_id": dataset_id,
        "source_sha256": observed,
        "train_rows": n_train,
        "test_rows": n_test,
        "dimension": spec["dimension"],
        "source_distance": spec["source_distance"],
        "fathom_distance": "l2",
        "transform": spec["fathom_transform"],
        "train_sha256": train_sha,
        "test_sha256": test_sha,
        "neighbors10_sha256": gold_sha,
    }
    seal_tmp = destination / "prepared.json.tmp"
    seal_tmp.write_text(json.dumps(seal, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    os.replace(seal_tmp, destination / "prepared.json")
    return destination / "prepared.json"


def main() -> None:
    """Prepare one explicitly supplied local source HDF5 for the Rust benchmarks."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dataset", required=True, choices=sorted(load_specs()))
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    args = parser.parse_args()
    print(prepare_dataset(args.dataset, args.source, args.output_root))


if __name__ == "__main__":
    main()
