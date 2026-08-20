#!/usr/bin/env python3
"""Assert forced-device captures contain only root-bound harness execution."""

from pathlib import Path


REPO = Path(__file__).resolve().parents[2]
PREFLIGHT = (REPO / "scripts/release/cuda-preflight.sh").read_text(encoding="utf-8")


def section(start: str, end: str) -> str:
    return PREFLIGHT.split(start, 1)[1].split(end, 1)[0]


forced_python = section("run_forced_python() {", "run_forced_napi() {")
failures = []
if "pip install" in forced_python:
    failures.append("wheel installation output can pollute the Python capture")
if "src=$FORCED_PYTHON_SITE,dst=/fathomdb-site,readonly" not in forced_python:
    failures.append("forced Python does not execute from a separately installed read-only site")
if "PYTHONPATH=/fathomdb-site" not in forced_python:
    failures.append("forced Python does not bind execution to the separately installed site")

forced_napi = section("run_forced_napi() {", "run_forced_python\n")
if (
    "src=$WORK_DIR/forced-napi-open.mjs,dst=/fathomdb-harness/forced-napi-open.mjs,readonly"
    not in forced_napi
):
    failures.append("executed N-API harness is not the exact retained root-bound file")
if "FORCED_NAPI_HARNESS" in PREFLIGHT:
    failures.append("a second N-API harness copy can substitute execution bytes")

forced_embedder_records = section('for consumer in ("python", "napi"):', "prefix = hashlib")
if '"command": f"installed_{consumer}_engine_open",' not in forced_embedder_records:
    failures.append("forced embedder records do not describe their default-embedder harness")
if "engine_open_without_default_embedder" in forced_embedder_records:
    failures.append("forced embedder records mislabel the default-embedder harness")

for harness_name in ("forced-napi-open.mjs", "forced-reranker-napi.mjs"):
    harness = (REPO / "scripts/release" / harness_name).read_text(encoding="utf-8")
    if "JSON.stringify(payload, CANONICAL_JSON_KEYS)" not in harness:
        failures.append(f"{harness_name} does not emit canonical forced-policy JSON")

assert not failures, "\n".join(failures)

print("CUDA preflight capture boundary tests passed")
