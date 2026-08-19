#!/usr/bin/env python3
"""Fail-closed verifier for the retained Tegra GPU allocation witness.

0.8.23 Slice 80.5 (D-80.5-5, AC80-18). This verifies
`fathomdb.tegra-gpu-allocation-witness/v1`, the record the in-process witness
in `fathomdb-embedder`'s `gpu_witness` module emits on a Tegra/iGPU host where
`nvidia-smi --query-compute-apps` reports nothing and the x86_64 PID witness
therefore cannot port.

It is deliberately a SEPARATE verifier from
`scripts/release/verify-cuda-preflight-witness.py`, which stays untouched in
80.5 (AC80-19). It reuses that file's idioms — exact-key checking, a canonical
JSON round-trip, `fail()` to exit 1 — and shares no code with it.

Everything the witness used to reach its verdict is re-derived here rather than
trusted (R80-13): the delta must follow from the retained free-memory samples,
the control delta must follow from its own samples, and the floor must be at
least the declared model floor.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, NoReturn

SCHEMA = "fathomdb.tegra-gpu-allocation-witness/v1"
# Mirrors `gpu_witness::DEFAULT_DELTA_FLOOR_BYTES`: a witness may declare a
# stricter floor, never a weaker one.
MODEL_FLOOR_BYTES = 67_108_864
# Mirrors `gpu_witness::MAX_CONTROL_BLOCKS`.
MAX_CONTROL_BLOCKS = 16
# Mirrors `gpu_witness::WITNESS_VECTOR_DIM` (bge-small-en-v1.5).
VECTOR_DIM = 384
# Mirrors `gpu_witness::SOLE_GPU_CONSUMER_PRECONDITION` (D-80.5-3).
PRECONDITION = (
    "the witness run must be the sole GPU consumer: cuMemGetInfo reports a shared, "
    "system-wide counter on an integrated GPU"
)
COMPUTE_CAPABILITY = re.compile(r"[0-9]+\.[0-9]+\Z")
# § 2.7: x86_64 reports `GPU-<uuid>`, Tegra reports the bare `<uuid>`. The
# driver bytes are identical, so the prefix is normalized away before any
# comparison rather than being required or forbidden.
CUDA_UUID = re.compile(r"(GPU-)?[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\Z", re.IGNORECASE)
REQUIRED_KEYS = frozenset({
    "compute_capability",
    "control_allocation_request_bytes",
    "control_block_count",
    "control_delta_bytes",
    "control_free_after_bytes",
    "control_free_before_bytes",
    "delta_bytes",
    "delta_floor_bytes",
    "device_name",
    "device_ordinal_actual",
    "device_ordinal_requested",
    "device_uuid",
    "embedded_vector_dim",
    "free_after_bytes",
    "free_before_bytes",
    "schema",
    "sole_gpu_consumer_precondition",
    "total_bytes",
})


def fail(message: str) -> NoReturn:
    print(f"tegra-gpu-witness: {message}", file=sys.stderr)
    raise SystemExit(1)


def canonical_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"


def normalize_uuid(value: str) -> str:
    """Strip the `GPU-` prefix x86_64 carries and Tegra omits (§ 2.7)."""
    lowered = value.strip().lower()
    return lowered[4:] if lowered.startswith("gpu-") else lowered


def require_exact_keys(value: dict[str, Any]) -> None:
    missing = sorted(REQUIRED_KEYS - set(value))
    if missing:
        fail(f"witness is missing required field(s): {', '.join(missing)}")
    unknown = sorted(set(value) - REQUIRED_KEYS)
    if unknown:
        fail(f"witness carries unknown field(s): {', '.join(unknown)}")


def require_int(value: dict[str, Any], key: str) -> int:
    raw = value[key]
    if not isinstance(raw, int) or isinstance(raw, bool):
        fail(f"{key} must be an integer")
    return raw


def require_string(value: dict[str, Any], key: str) -> str:
    raw = value[key]
    if not isinstance(raw, str) or not raw:
        fail(f"{key} must be a nonempty string")
    return raw


def load_canonical_witness(path: Path) -> dict[str, Any]:
    if path.is_symlink() or not path.is_file():
        fail(f"witness {path} must be a regular non-symlink file")
    try:
        raw = path.read_bytes()
        value = json.loads(raw)
    except (OSError, json.JSONDecodeError) as error:
        fail(f"cannot read witness {path}: {error}")
    if not isinstance(value, dict):
        fail("witness must be a JSON object")
    if raw != canonical_json(value):
        fail("witness is not canonical JSON")
    return value


def validate_identity(witness: dict[str, Any]) -> None:
    if witness["schema"] != SCHEMA:
        fail(f"witness schema must be {SCHEMA}")
    requested = require_int(witness, "device_ordinal_requested")
    actual = require_int(witness, "device_ordinal_actual")
    if requested < 0 or actual < 0:
        fail("device ordinals must be non-negative")
    if requested != actual:
        fail(f"witness ordinal does not correlate: requested cuda:{requested}, retained cuda:{actual}")
    uuid = require_string(witness, "device_uuid")
    if CUDA_UUID.fullmatch(uuid) is None:
        fail("device UUID is not a CUDA driver UUID")
    require_string(witness, "device_name")
    capability = require_string(witness, "compute_capability")
    if COMPUTE_CAPABILITY.fullmatch(capability) is None:
        fail("compute capability is invalid")
    if witness["sole_gpu_consumer_precondition"] != PRECONDITION:
        fail("witness does not carry the verbatim sole-GPU-consumer precondition")
    if require_int(witness, "embedded_vector_dim") != VECTOR_DIM:
        fail(f"witness must record one real forward pass of {VECTOR_DIM} dimensions")


def validate_load_delta(witness: dict[str, Any]) -> None:
    total = require_int(witness, "total_bytes")
    before = require_int(witness, "free_before_bytes")
    after = require_int(witness, "free_after_bytes")
    delta = require_int(witness, "delta_bytes")
    floor = require_int(witness, "delta_floor_bytes")
    if total <= 0:
        fail("total device memory must be positive")
    if not 0 < before <= total or not 0 < after <= total:
        fail("free memory samples must be positive and within the device total")
    if delta != before - after:
        fail("delta does not follow from the retained free-memory samples")
    if delta == 0:
        fail("allocation delta is zero: no device memory was claimed across the load")
    if delta < 0:
        fail(f"allocation delta is negative ({delta}): memory was released, not claimed")
    if floor < MODEL_FLOOR_BYTES:
        fail(f"declared floor {floor} is weaker than the model floor {MODEL_FLOOR_BYTES}")
    if delta < floor:
        fail(f"allocation delta {delta} is below the declared floor {floor}")


def validate_control(witness: dict[str, Any]) -> None:
    requested = require_int(witness, "control_allocation_request_bytes")
    blocks = require_int(witness, "control_block_count")
    before = require_int(witness, "control_free_before_bytes")
    after = require_int(witness, "control_free_after_bytes")
    delta = require_int(witness, "control_delta_bytes")
    if requested <= 0:
        fail("control allocation request must be positive")
    if not 1 <= blocks <= MAX_CONTROL_BLOCKS:
        fail(f"control block count must be between 1 and {MAX_CONTROL_BLOCKS}")
    if before <= 0 or after <= 0:
        fail("control free-memory samples must be positive")
    if delta != before - after:
        fail("control delta does not follow from the retained control samples")
    if delta < requested:
        fail(f"control allocation of {requested} bytes moved the counter by only {delta}")


def validate_uuid_correlation(witness: dict[str, Any], expected: str) -> None:
    if CUDA_UUID.fullmatch(expected.strip()) is None:
        fail("expected UUID is not a CUDA driver UUID")
    if normalize_uuid(witness["device_uuid"]) != normalize_uuid(expected):
        fail(f"witness UUID {witness['device_uuid']} does not correlate with {expected}")


def query_nvidia_smi_uuid() -> str:
    """Read the device UUID from `nvidia-smi`, exit status first (R80-13)."""
    try:
        completed = subprocess.run(
            ["nvidia-smi", "--query-gpu=uuid", "--format=csv,noheader"],
            capture_output=True,
            check=False,
            text=True,
        )
    except OSError as error:
        fail(f"cannot run nvidia-smi: {error}")
    if completed.returncode != 0:
        fail(f"nvidia-smi exited {completed.returncode}; its output is not evidence")
    lines = [line.strip() for line in completed.stdout.splitlines() if line.strip()]
    if len(lines) != 1:
        fail("nvidia-smi did not report exactly one device UUID")
    return lines[0]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--witness", required=True, type=Path)
    parser.add_argument(
        "--expect-uuid",
        help="cross-check the recorded device UUID against this value, normalizing the GPU- prefix",
    )
    parser.add_argument(
        "--nvidia-smi",
        action="store_true",
        help="cross-check the recorded device UUID against nvidia-smi on this host",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    witness = load_canonical_witness(args.witness)
    require_exact_keys(witness)
    validate_identity(witness)
    validate_control(witness)
    validate_load_delta(witness)
    if args.expect_uuid is not None:
        validate_uuid_correlation(witness, args.expect_uuid)
    if args.nvidia_smi:
        validate_uuid_correlation(witness, query_nvidia_smi_uuid())
    print("tegra-gpu-witness: pass")


if __name__ == "__main__":
    main()
