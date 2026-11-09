#!/usr/bin/env python3
"""Fail-closed current v2 Tegra private-pool allocation verifier.

Re-derive live USED/model/control arithmetic from the exact tagged record;
shared global readings remain truthful diagnostics. Historical v1 is refused.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, NoReturn

SCHEMA = "fathomdb.tegra-gpu-allocation-witness/v2"
# Mirrors `gpu_witness::DEFAULT_DELTA_FLOOR_BYTES`: a witness may declare a
# stricter floor, never a weaker one.
MODEL_FLOOR_BYTES = 67_108_864
# Mirrors `gpu_witness::WITNESS_VECTOR_DIM` (bge-small-en-v1.5).
VECTOR_DIM = 384
PRECONDITION = (
    "no competing allocations or frees may occur in the witnessed private pool "
    "during model and control observation"
)
COMPUTE_CAPABILITY = re.compile(r"[0-9]+\.[0-9]+\Z")
# § 2.7: x86_64 reports `GPU-<uuid>`, Tegra reports the bare `<uuid>`. The
# driver bytes are identical, so the prefix is normalized away before any
# comparison rather than being required or forbidden.
CUDA_UUID = re.compile(r"(GPU-)?[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\Z", re.IGNORECASE)
REQUIRED_KEYS = frozenset({
    "compute_capability", "delta_bytes", "delta_floor_bytes", "device_integrated",
    "device_name", "device_ordinal_actual", "device_ordinal_requested", "device_uuid",
    "embedded_vector_dim", "free_after_bytes", "free_before_bytes", "measurement",
    "process_id", "schema", "total_bytes",
})
PRIVATE_KEYS = frozenset({
    "kind", "context_id", "pool_identity", "used_before_bytes", "used_after_bytes",
    "delta_bytes", "control_request_bytes", "control_before_bytes", "control_live_bytes",
    "control_released_bytes", "pool_exclusivity_precondition",
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


def require_exact_keys(
    value: dict[str, Any], keys: frozenset[str] = REQUIRED_KEYS, label: str = "witness",
) -> None:
    missing = sorted(keys - set(value))
    if missing:
        fail(f"{label} is missing required field(s): {', '.join(missing)}")
    unknown = sorted(set(value) - keys)
    if unknown:
        fail(f"{label} carries unknown field(s): {', '.join(unknown)}")


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
    if require_int(witness, "process_id") < 1:
        fail("process_id must be positive")
    if witness["device_integrated"] is not True:
        fail("Tegra proof requires device_integrated=true")
    dimension = require_int(witness, "embedded_vector_dim")
    if dimension != VECTOR_DIM:
        fail(f"witness forward dimension {dimension} differs from {VECTOR_DIM}")


def validate_load_delta(witness: dict[str, Any]) -> None:
    total = require_int(witness, "total_bytes")
    before = require_int(witness, "free_before_bytes")
    after = require_int(witness, "free_after_bytes")
    delta = require_int(witness, "delta_bytes")
    floor = require_int(witness, "delta_floor_bytes")
    if total <= 0:
        fail("total device memory must be positive")
    if not 0 <= before <= total or not 0 <= after <= total:
        fail("global diagnostic samples must be nonnegative and within the device total")
    if delta != before - after:
        fail("delta does not follow from the retained free-memory samples")
    if floor < MODEL_FLOOR_BYTES:
        fail(f"declared floor {floor} is weaker than the model floor {MODEL_FLOOR_BYTES}")
    proof = witness["measurement"]
    before_used = require_int(proof, "used_before_bytes")
    after_used = require_int(proof, "used_after_bytes")
    used_delta = require_int(proof, "delta_bytes")
    if not 0 <= before_used <= (1 << 64) - 1 or not 0 <= after_used <= (1 << 64) - 1:
        fail("private USED samples must be uint64 byte counts")
    if used_delta != after_used - before_used:
        fail("private delta does not follow from retained USED samples")
    if used_delta == 0:
        fail("private allocation delta is zero")
    if used_delta < 0:
        fail(f"private allocation delta is negative ({used_delta})")
    if used_delta < floor:
        fail(f"private allocation delta {used_delta} is below declared floor {floor}")


def validate_control(witness: dict[str, Any]) -> None:
    proof = witness["measurement"]
    if not isinstance(proof, dict):
        fail("measurement must be an object")
    require_exact_keys(proof, PRIVATE_KEYS, "measurement")
    if proof["kind"] != "private_pool_used":
        fail(f"Tegra measurement kind must be private_pool_used, saw {proof['kind']!r}")
    context_id = require_string(proof, "context_id")
    if re.fullmatch(r"0|[1-9][0-9]*", context_id) is None or int(context_id) > (1 << 64) - 1:
        fail("context_id must be a canonical uint64 decimal string")
    require_string(proof, "pool_identity")
    if proof["pool_exclusivity_precondition"] != PRECONDITION:
        fail("measurement does not carry the verbatim private-pool exclusivity precondition")
    requested = require_int(proof, "control_request_bytes")
    if requested != MODEL_FLOOR_BYTES:
        fail(f"private control request {requested} must be exactly {MODEL_FLOOR_BYTES}")
    before = require_int(proof, "control_before_bytes")
    live = require_int(proof, "control_live_bytes")
    released = require_int(proof, "control_released_bytes")
    if any(not 0 <= value <= (1 << 64) - 1 for value in (before, live, released)):
        fail("private control samples must be uint64 byte counts")
    if live - before < requested:
        fail(f"private live control rose by {live - before}, below request {requested}")
    if released != before:
        fail("private control release did not restore exact USED baseline")


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
