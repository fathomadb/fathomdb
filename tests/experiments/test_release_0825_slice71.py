from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import pytest

from experiments.release_0825_slice71 import (
    Slice71ContractError,
    validate_manifest,
    validate_receipt,
)


ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / "experiments" / "configs" / "release-0825-slice71-manifest.v1.json"
RECEIPT_PATH = ROOT / "dev" / "plans" / "runs" / "0.8.25-slice-71" / "receipt.v1.json"


def manifest() -> dict[str, object]:
    return json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))


def receipt() -> dict[str, object]:
    return json.loads(RECEIPT_PATH.read_text(encoding="utf-8"))


def test_checked_in_manifest_is_strict_and_valid() -> None:
    validate_manifest(manifest())


@pytest.mark.parametrize(
    ("path", "value"),
    [
        (("unexpected",), True),
        (("workloads", "ac013", "samples"), 999),
        (("workloads", "ac013", "vector_dim"), 768),
        (("arm_order",), ["B", "C", "B", "C", "B", "C"]),
        (("workloads", "ingest", "batch_size"), 200),
        (("workloads", "ingest", "expected_ingest_events"), 10_000),
    ],
)
def test_manifest_rejects_drift(path: tuple[str, ...], value: object) -> None:
    document = copy.deepcopy(manifest())
    target = document
    for key in path[:-1]:
        target = target[key]  # type: ignore[index,assignment]
    target[path[-1]] = value  # type: ignore[index]
    with pytest.raises(Slice71ContractError):
        validate_manifest(document)


def test_checked_in_receipt_is_strict_and_valid() -> None:
    validate_receipt(receipt(), manifest(), MANIFEST_PATH.read_bytes())


@pytest.mark.parametrize(
    ("path", "value"),
    [
        (("unexpected",), True),
        (("candidate_ref",), "0" * 40),
        (("cells", 0, "unexpected"), True),
        (("cells", 0, "metrics", "samples"), 999),
        (("cells", 0, "process_identity", "test_threads"), 2),
        (("classifications", "ingest"), "within_limit"),
    ],
)
def test_receipt_rejects_drift(path: tuple[object, ...], value: object) -> None:
    document = copy.deepcopy(receipt())
    target: object = document
    for key in path[:-1]:
        target = target[key]  # type: ignore[index]
    target[path[-1]] = value  # type: ignore[index]
    with pytest.raises(Slice71ContractError):
        validate_receipt(document, manifest(), MANIFEST_PATH.read_bytes())


def test_receipt_recomputes_manifest_digest() -> None:
    document = receipt()
    document["manifest_sha256"] = "0" * 64
    for cell in document["cells"]:  # type: ignore[union-attr]
        cell["config_sha256"] = "0" * 64
    with pytest.raises(Slice71ContractError, match="manifest_sha256"):
        validate_receipt(document, manifest(), MANIFEST_PATH.read_bytes())


def test_receipt_derives_classification_from_metrics() -> None:
    document = receipt()
    document["cells"][2]["metrics"]["p99_ms"] = 260  # type: ignore[index]
    with pytest.raises(Slice71ContractError, match="classifications/ac013"):
        validate_receipt(document, manifest(), MANIFEST_PATH.read_bytes())


def test_manifest_bytes_are_the_document_being_validated() -> None:
    document = manifest()
    different_bytes = json.dumps(document, sort_keys=True).encode()
    assert hashlib.sha256(different_bytes).hexdigest() != receipt()["manifest_sha256"]
    with pytest.raises(Slice71ContractError, match="manifest bytes"):
        validate_receipt(receipt(), document, different_bytes)
