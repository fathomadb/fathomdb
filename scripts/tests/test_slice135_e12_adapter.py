"""Adversarial checks for the source-bound E01–E12 adapter receipt."""

from __future__ import annotations

import copy
import gzip
import hashlib
import json
from pathlib import Path
import sys

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from slice135_e12_adapter import parse_resources, validate_measurement, validate_model_assets  # noqa: E402
from slice135_e12_audit import audit_receipt, audit_resource_report, audit_runner_sources  # noqa: E402


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _fixture(tmp_path: Path) -> tuple[dict, dict]:
    runner = tmp_path / "runner"
    binary = tmp_path / "binary"
    corpus = tmp_path / "corpus"
    model = tmp_path / "model"
    for path in (runner, binary, corpus, model):
        path.write_bytes(path.name.encode())
    checks = {"ordered_ids": ["doc-0", "doc-1"], "canonical_rows": 32}
    protocol = {
        "schema_version": 1,
        "source_sha": "a" * 40,
        "runner_sha256": _sha(runner.read_bytes()),
        "artifact_sha256": {path.name: _sha(path.read_bytes()) for path in (binary, corpus, model)},
        "settings": {"samples": 100, "device": "cpu"},
        "cells": {"text": {"kind": "query", "expected_checks": checks}},
    }
    raw = {
        "schema_version": 1,
        "source_sha": protocol["source_sha"],
        "runner_sha256": protocol["runner_sha256"],
        "protocol_sha256": _sha(json.dumps(protocol, sort_keys=True).encode()),
        "artifact_sha256": protocol["artifact_sha256"],
        "settings": protocol["settings"],
        "environment": {"start": {"host": "test"}, "end": {"host": "test"}, "invalidators": []},
        "cells": {"text": [{"valid": True, "latency_ns": 10, "observed_checks": checks} for _ in range(100)]},
    }
    return protocol, raw


def test_valid_receipt_and_negative_fixtures(tmp_path: Path) -> None:
    protocol, raw = _fixture(tmp_path)
    validate_measurement(raw, protocol, tmp_path)
    changes = (
        ("source_sha", "b" * 40),
        ("settings", {"samples": 7, "device": "cpu"}),
    )
    for key, value in changes:
        corrupted = copy.deepcopy(raw)
        corrupted[key] = value
        with pytest.raises(ValueError):
            validate_measurement(corrupted, protocol, tmp_path)
    corrupted = copy.deepcopy(raw)
    corrupted["cells"]["text"][0]["observed_checks"]["ordered_ids"] = ["doc-1", "doc-0"]
    with pytest.raises(ValueError):
        validate_measurement(corrupted, protocol, tmp_path)
    corrupted = copy.deepcopy(raw)
    corrupted["cells"]["text"].pop()
    with pytest.raises(ValueError):
        validate_measurement(corrupted, protocol, tmp_path)
    (tmp_path / "binary").write_bytes(b"changed")
    with pytest.raises(ValueError):
        validate_measurement(raw, protocol, tmp_path)


def test_independent_audit_rejects_wrong_source_state_sample_and_output(tmp_path: Path) -> None:
    protocol, raw = _fixture(tmp_path)
    audit_receipt(raw, protocol, tmp_path)
    for mutation in ("source", "state", "sample", "output"):
        corrupted = copy.deepcopy(raw)
        if mutation == "source":
            corrupted["source_sha"] = "b" * 40
        elif mutation == "state":
            corrupted["cells"]["text"][0]["observed_checks"]["canonical_rows"] = 31
        elif mutation == "sample":
            corrupted["cells"]["text"].pop()
        else:
            corrupted["cells"]["text"][0]["observed_checks"]["ordered_ids"] = ["doc-1", "doc-0"]
        with pytest.raises(ValueError):
            audit_receipt(corrupted, protocol, tmp_path)


def test_gnu_time_resources_are_materialized() -> None:
    report = """User time (seconds): 1.25
System time (seconds): 0.50
Elapsed (wall clock) time (h:mm:ss or m:ss): 0:02.10
Maximum resident set size (kbytes): 1024
Major (requiring I/O) page faults: 2
Swaps: 0
File system inputs: 3
File system outputs: 4
"""
    assert parse_resources(report) == {
        "method": "gnu-time", "scope": "measured child", "user_cpu_s": 1.25,
        "system_cpu_s": 0.5, "elapsed_wall_s": 2.1, "peak_rss_kib": 1024,
        "major_faults": 2, "swap_events": 0, "fs_inputs": 3, "fs_outputs": 4,
    }
    audit_resource_report(parse_resources(report), report)
    corrupted = parse_resources(report)
    corrupted["fs_outputs"] = 0
    with pytest.raises(ValueError):
        audit_resource_report(corrupted, report)


def test_host_only_swap_drift_is_reported(tmp_path: Path) -> None:
    protocol, raw = _fixture(tmp_path)
    raw["environment"]["start"]["swap_pages"] = 20
    raw["environment"]["end"]["swap_pages"] = 22
    assert audit_receipt(raw, protocol, tmp_path)["environment_warnings"] == [
        "host swap drift: 2 pages; child swap events: 0"
    ]


def test_archived_binary_bytes_still_bind(tmp_path: Path) -> None:
    protocol, raw = _fixture(tmp_path)
    binary = tmp_path / "binary"
    with gzip.GzipFile(filename=str(tmp_path / "binary.gz"), mode="wb", mtime=0) as stream:
        stream.write(binary.read_bytes())
    binary.unlink()
    audit_receipt(raw, protocol, tmp_path)
    (tmp_path / "binary.gz").write_bytes(b"corrupted")
    with pytest.raises(ValueError):
        audit_receipt(raw, protocol, tmp_path)


def test_archived_runner_source_bytes_bind(tmp_path: Path) -> None:
    shared = tmp_path / "shared"
    shared.mkdir()
    (shared / "pilot-adapter.py").write_bytes(b"adapter")
    (shared / "workload.rs").write_bytes(b"workload")
    block = tmp_path / "block"
    block.mkdir()
    (block / "runner").write_text(json.dumps({
        "adapter": _sha(b"adapter"), "workload": _sha(b"workload")
    }))
    audit_runner_sources(block, shared)
    (shared / "workload.rs").write_bytes(b"altered")
    with pytest.raises(ValueError):
        audit_runner_sources(block, shared)


def test_model_assets_are_pinned_before_timing(tmp_path: Path) -> None:
    names = ("config.json", "model.safetensors", "tokenizer.json")
    for name in names:
        (tmp_path / name).write_bytes(name.encode())
    pinned = {name: _sha(name.encode()) for name in names}
    validate_model_assets(tmp_path, pinned)
    (tmp_path / "tokenizer.json").write_bytes(b"changed")
    with pytest.raises(ValueError):
        validate_model_assets(tmp_path, pinned)
