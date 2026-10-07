#!/usr/bin/env python3
"""Reject any current-tree Gitleaks exception outside the reviewed false positives."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # 0.8.23 Slice 80.3: Python 3.10 fallback (Ubuntu
    # 22.04 / every Jetson ship 3.10, which lacks stdlib tomllib).
    import tomli as tomllib  # type: ignore[import-not-found,no-redef]


EXPECTED_ALLOWLISTS = [
    {
        "description": "Vendored SQLite extension API field names are C identifiers",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [
            r"^third_party/libsqlite3-sys-0\.38\.1/(?:sqlite3|sqlcipher)/"
            r"(?:sqlite3ext\.h|sqlite3\.c)$"
        ],
        "regexes": [r"^sqlite3_api->(?:column_bytes16|soft_heap_limit64|hard_heap_limit64)$"],
    },
    {
        "description": "Vendored SQLite FTS5 row identifier is not a credential",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [r"^third_party/libsqlite3-sys-0\.38\.1/(?:sqlite3|sqlcipher)/sqlite3\.c$"],
        "regexes": [r"^iKey==FTS5_AVERAGES_ROWID\s*$"],
    },
    {
        "description": "CUDA tokenizer digest map is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [r"^(?:.*/)?scripts/check-cuda-release-contract\.py$"],
        "regexes": [r'^tokenizer\.json": "[0-9a-f]{64}"$'],
    },
    {
        "description": "Slice 115 model tokenizer digest is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [
            r"^dev/plans/0\.8\.27/features/slice-115/(?:protocol\.json|evidence/(?:final/"
            r"(?:attempt|model-assets|protocol)\.json|invalid/attempt\.json|"
            r"superseded-attempt[56]/(?:attempt|model-assets|protocol)\.json))$"
        ],
        "regexes": [
            r'^tokenizer\.json": "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66"$'
        ],
    },
    {
        "description": "REASON-01 tokenizer digests are artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "secret",
        "paths": [r"^experiments/configs/reason-01/profile-registry\.v1\.json$"],
        "regexes": [r"^[0-9a-f]{64}$"],
    },
    {
        "description": "CUDA tokenizer digest export is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [r"^(?:.*/)?scripts/release/cuda-artifact-contract\.sh$"],
        "regexes": [r"^CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256='[0-9a-f]{64}'$"],
    },
    {
        "description": "TinyBERT tokenizer digest export is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [r"^(?:.*/)?scripts/release/cuda-artifact-contract\.sh$"],
        "regexes": [r"^CUDA_RERANKER_TOKENIZER_SHA256='[0-9a-f]{64}'$"],
    },
    {
        "description": "TinyBERT tokenizer digest is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [r"^(?:.*/)?scripts/release/cuda-package-rehearsal-smoke\.sh$"],
        "regexes": [r'^tokenizer\.json": "[0-9a-f]{64}"$'],
    },
    {
        "description": "N-API rustup bootstrap digest is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [r"^(?:.*/)?scripts/release/napi-artifact-contract\.sh$"],
        "regexes": [r"^NAPI_RUSTUP_INIT_SHA256='[0-9a-f]{64}'$"],
    },
    {
        "description": "CUDA preflight-v2 fixtures contain only artifact-integrity digests",
        "condition": "AND",
        "regexTarget": "secret",
        "paths": [
            r"^scripts/tests/fixtures/cuda-preflight-v2/valid/"
            r"(?:cuda-preflight-witness|model-cache-manifest|smoke-cache-topology)\.json$"
        ],
        "regexes": [r"^[0-9a-f]{64}$"],
    },
    {
        "description": "Slice 72 CE tokenizer digest is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [
            r"^dev/plans/0\.8\.25/features/slice-72/ce-profile-manifest\.json$"
        ],
        "regexes": [r'^tokenizer\.json": "[0-9a-f]{64}"$'],
    },
    {
        "description": "Slice 72 CE receipts contain only artifact and evidence digests",
        "condition": "AND",
        "regexTarget": "secret",
        "paths": [
            r"^dev/plans/runs/0\.8\.25-slice-72/(?:receipt\.json|"
            r"(?:baseline|candidate)-(?:cpu|cuda)/"
            r"(?:baseline|candidate)-(?:cpu|cuda)\.json)$"
        ],
        "regexes": [r"^[0-9a-f]{64}$"],
    },
    {
        "description": "Slice 85 CE overlay contains only copied artifact-integrity digests",
        "condition": "AND",
        "regexTarget": "secret",
        "paths": [
            r"^dev/plans/runs/0\.8\.25-slice-85/slice72-ce-manifest\.json$"
        ],
        "regexes": [r"^[0-9a-f]{64}$"],
    },
    {
        "description": "Slice 75 closure manifest contains only reviewed artifact and evidence digests",
        "condition": "AND",
        "regexTarget": "secret",
        "paths": [
            r"^dev/plans/0\.8\.25/features/slice-75/slice75-closure-manifest\.json$"
        ],
        "regexes": [r"^[0-9a-f]{64}$"],
    },
    {
        "description": "code-marker drift evidence carries two reviewed marker identifiers",
        "condition": "AND",
        "regexTarget": "secret",
        "paths": [r"^dev/experiments/code-markers-eval/out/drift\.jsonl$"],
        "regexes": [r"^(?:ADR-0\.6\.0-retrieval-latency-gates|ADR-0\.8\.1-byo-llm)$"],
    },
    {
        "description": "code-marker inventory evidence carries two reviewed marker identifiers",
        "condition": "AND",
        "regexTarget": "secret",
        "paths": [r"^dev/experiments/code-markers-eval/out/incode_markers\.jsonl$"],
        "regexes": [r"^(?:ADR-0\.6\.0-retrieval-latency-gates|ADR-0\.8\.1-byo-llm)$"],
    },
    {
        "description": "Slice 135 E01-E12 pinned tokenizer digest is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "secret",
        "paths": [
            r"^(?:scripts/slice135_e12_adapter\.py|dev/plans/0\.8\.27/features/"
            r"slice-135/results/2026-10-07-e12-adapter-pilot/(?:block-[1-5]|"
            r"baseline-all|candidate-all)/(?:protocol|raw)\.json)$"
        ],
        "regexes": [r"^d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66$"],
    },
]


def performance_allowlist(authority_path: Path) -> dict:
    """Load and validate the sole owner of the performance digest exception."""
    try:
        authority = json.loads(authority_path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"invalid performance digest authority: {exc}") from exc
    required = {"schema", "description", "key", "sha256", "paths"}
    if set(authority) != required:
        raise ValueError("performance digest authority has unexpected fields")
    if authority["schema"] != "fathomdb.performance-digest-allowlist.v1":
        raise ValueError("performance digest authority has an unexpected schema")
    if authority["description"] != "Performance gauntlet tokenizer digests are artifact-integrity metadata":
        raise ValueError("performance digest authority has an unexpected description")
    if authority["key"] != "tokenizer.json":
        raise ValueError("performance digest authority has an unexpected key")
    digest = authority["sha256"]
    if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
        raise ValueError("performance digest authority has an invalid SHA-256")
    paths = authority["paths"]
    if not isinstance(paths, list) or len(paths) != 4 or len(set(paths)) != 4:
        raise ValueError("performance digest authority must have four unique paths")
    root = Path(__file__).resolve().parents[2]
    for relative in paths:
        if not isinstance(relative, str) or relative.startswith("/") or re.search(r"[*?{}[\]]", relative):
            raise ValueError("performance digest authority path is not one exact relative path")
        try:
            text = (root / relative).read_text()
        except OSError as exc:
            raise ValueError(f"performance digest authority path is unreadable: {relative}: {exc}") from exc
        pattern = rf'["\']tokenizer\.json["\']\s*:\s*["\']{re.escape(digest)}["\']'
        if re.search(pattern, text) is None:
            raise ValueError(f"performance digest authority does not match {relative}")
    path_regex = "^(?:" + "|".join(re.escape(path).replace(r"\-", "-") for path in paths) + ")$"
    return {
        "description": authority["description"],
        "condition": "AND",
        "regexTarget": "match",
        "paths": [path_regex],
        "regexes": [rf'^{re.escape(authority["key"])}": "{digest}"$'],
    }


def main() -> int:
    if len(sys.argv) not in {2, 3}:
        print("usage: check-gitleaks-current-config.py PATH [PERFORMANCE_AUTHORITY]", file=sys.stderr)
        return 2
    authority_path = (
        Path(sys.argv[2])
        if len(sys.argv) == 3
        else Path(__file__).with_name("performance-digest-allowlist.json")
    )
    try:
        value = tomllib.loads(Path(sys.argv[1]).read_text())
    except (OSError, tomllib.TOMLDecodeError) as exc:
        print(f"invalid current-tree Gitleaks policy: {exc}", file=sys.stderr)
        return 1

    try:
        performance = performance_allowlist(authority_path)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    expected_allowlists = EXPECTED_ALLOWLISTS.copy()
    expected_allowlists.insert(3, performance)
    expected = {
        "title": "FathomDB current-tree Gitleaks policy",
        "extend": {"useDefault": True},
        "rules": [{"id": "generic-api-key", "allowlists": expected_allowlists}],
    }
    if value != expected:
        print("current-tree Gitleaks policy differs from the reviewed exact exception set", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
