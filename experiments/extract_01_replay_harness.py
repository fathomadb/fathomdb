#!/usr/bin/env python3
"""Replay checkpointed EXTRACT-01 ELPS results over the native subprocess wire."""

from __future__ import annotations

import json
import sys
from pathlib import Path


PROTOCOL = "fathomdb.extract.v1"


def _write(value: object) -> None:
    print(json.dumps(value, ensure_ascii=False), flush=True)


def main() -> int:
    if len(sys.argv) != 2:
        return 2
    results = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    for line in sys.stdin:
        message = json.loads(line)
        if message.get("type") == "hello":
            _write(
                {
                    "protocol": PROTOCOL,
                    "type": "ready",
                    "schema_version": 1,
                    "provider": "extract-01-checkpoint-replay",
                    "model": results["model"],
                    "supports": {},
                    "max_docs_per_request": 8,
                }
            )
            continue
        if message.get("type") != "extract":
            continue
        entities: list[object] = []
        edges: list[object] = []
        warnings: list[object] = []
        for document in message.get("documents", []):
            result = results["documents"][document["source_doc_id"]]
            entities.extend(result["entities"])
            edges.extend(result["edges"])
            warnings.extend(result["warnings"])
        _write(
            {
                "protocol": PROTOCOL,
                "type": "result",
                "request_id": message["request_id"],
                "entities": entities,
                "edges": edges,
                "warnings": warnings,
            }
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
