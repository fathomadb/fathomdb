#!/usr/bin/env python3
"""Reject any current-tree Gitleaks exception outside the reviewed false positives."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path


EXPECTED_ALLOWLISTS = [
    {
        "description": "CUDA tokenizer digest map is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [r"^(?:.*/)?scripts/check-cuda-release-contract\.py$"],
        "regexes": [r'^tokenizer\.json": "[0-9a-f]{64}"$'],
    },
    {
        "description": "CUDA tokenizer digest export is artifact-integrity metadata",
        "condition": "AND",
        "regexTarget": "match",
        "paths": [r"^(?:.*/)?scripts/release/cuda-artifact-contract\.sh$"],
        "regexes": [r"^CUDA_DEFAULT_EMBEDDER_TOKENIZER_SHA256='[0-9a-f]{64}'$"],
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
]


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: check-gitleaks-current-config.py PATH", file=sys.stderr)
        return 2
    try:
        value = tomllib.loads(Path(sys.argv[1]).read_text())
    except (OSError, tomllib.TOMLDecodeError) as exc:
        print(f"invalid current-tree Gitleaks policy: {exc}", file=sys.stderr)
        return 1

    expected = {
        "title": "FathomDB current-tree Gitleaks policy",
        "extend": {"useDefault": True},
        "rules": [{"id": "generic-api-key", "allowlists": EXPECTED_ALLOWLISTS}],
    }
    if value != expected:
        print("current-tree Gitleaks policy differs from the reviewed exact exception set", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
