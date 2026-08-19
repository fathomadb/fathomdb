#!/usr/bin/env python3
"""Fixture-driven contract for the Tegra GPU allocation witness verifier.

0.8.23 Slice 80.5 (AC80-18): every arm here runs on a host with no GPU. The
committed fixture is a REAL record produced by
`tests/slice80_gpu_allocation_witness.rs` on a Jetson Orin AGX, so the verifier
is exercised against the bytes the producer actually emits rather than against
a hand-written approximation.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
VERIFIER = REPO / "scripts/release/verify-tegra-gpu-witness.py"
FIXTURE = REPO / "scripts/tests/fixtures/tegra-gpu-witness/valid/tegra-gpu-allocation-witness.json"
NVIDIA_SMI_UUID = "bbbe9f37-7028-556a-930b-54e5f3b67a82"

REQUIRED_KEYS = (
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
)


def canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"


def run(path: Path, *extra: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(VERIFIER), "--witness", str(path), *extra],
        capture_output=True,
        check=False,
        text=True,
    )


def write_record(root: Path, record: object, *, raw: bytes | None = None) -> Path:
    path = root / "tegra-gpu-allocation-witness.json"
    path.write_bytes(canonical(record) if raw is None else raw)
    return path


def accept(root: Path, record: dict[str, object], label: str, *extra: str) -> None:
    result = run(write_record(root, record), *extra)
    assert result.returncode == 0, f"rejected a valid witness ({label}): {result.stderr}"


def reject(root: Path, record: dict[str, object], label: str, *extra: str) -> str:
    result = run(write_record(root, record), *extra)
    assert result.returncode == 1, f"accepted {label}: {result.stdout}{result.stderr}"
    diagnostic = result.stderr.strip()
    assert diagnostic, f"{label} was rejected without a diagnostic"
    return diagnostic


def main() -> None:
    valid = json.loads(FIXTURE.read_bytes())
    assert FIXTURE.read_bytes() == canonical(valid), "the committed fixture is not canonical JSON"
    diagnostics: dict[str, str] = {}

    with tempfile.TemporaryDirectory() as raw_root:
        root = Path(raw_root)

        # The committed real-hardware record passes, as itself and against the
        # Tegra `nvidia-smi` UUID spelling (§ 2.7: no `GPU-` prefix).
        result = run(FIXTURE)
        assert result.returncode == 0, f"rejected the committed fixture: {result.stderr}"
        assert "pass" in result.stdout, result.stdout
        accept(root, valid, "valid record with normalized nvidia-smi UUID", "--expect-uuid", NVIDIA_SMI_UUID)
        accept(root, valid, "valid record with prefixed UUID", "--expect-uuid", valid["device_uuid"])

        # AC80-18: zero, negative and below-floor deltas.
        zero = dict(valid, free_after_bytes=valid["free_before_bytes"], delta_bytes=0)
        diagnostics["zero delta"] = reject(root, zero, "a zero delta")
        negative = dict(
            valid,
            free_after_bytes=valid["free_before_bytes"] + 4096,
            delta_bytes=-4096,
        )
        diagnostics["negative delta"] = reject(root, negative, "a negative delta")
        below = dict(
            valid,
            free_after_bytes=valid["free_before_bytes"] - 1_048_576,
            delta_bytes=1_048_576,
        )
        diagnostics["below floor"] = reject(root, below, "a below-floor delta")

        # A delta that does not follow from the retained samples (R80-13).
        diagnostics["unre-derivable delta"] = reject(
            root, dict(valid, delta_bytes=valid["delta_bytes"] + 1), "an unre-derivable delta"
        )
        # A floor weaker than the declared model floor.
        diagnostics["weak floor"] = reject(
            root, dict(valid, delta_floor_bytes=1024), "a weakened floor"
        )

        # Ordinal and UUID correlation.
        diagnostics["ordinal mismatch"] = reject(
            root, dict(valid, device_ordinal_actual=1), "an ordinal mismatch"
        )
        diagnostics["uuid mismatch"] = reject(
            root,
            valid,
            "a UUID that does not correlate",
            "--expect-uuid",
            "GPU-00000000-0000-0000-0000-000000000000",
        )
        diagnostics["uuid malformed"] = reject(
            root, dict(valid, device_uuid="not-a-uuid"), "a malformed UUID"
        )

        # Control allocation (D-80.5-3).
        diagnostics["control not observed"] = reject(
            root,
            dict(
                valid,
                control_free_after_bytes=valid["control_free_before_bytes"] - 1024,
                control_delta_bytes=1024,
            ),
            "a control allocation the counter never saw",
        )
        diagnostics["control unre-derivable"] = reject(
            root,
            dict(valid, control_delta_bytes=valid["control_delta_bytes"] + 1),
            "an unre-derivable control delta",
        )
        diagnostics["control blocks"] = reject(
            root, dict(valid, control_block_count=0), "a control step with no blocks"
        )

        # Identity and provenance.
        diagnostics["schema"] = reject(
            root,
            dict(valid, schema="fathomdb.cuda-device-observation/v1"),
            "the x86_64 observation schema",
        )
        diagnostics["dimension"] = reject(
            root, dict(valid, embedded_vector_dim=0), "a witness with no forward pass"
        )
        diagnostics["precondition"] = reject(
            root, dict(valid, sole_gpu_consumer_precondition="fine"), "a softened precondition"
        )
        diagnostics["compute capability"] = reject(
            root, dict(valid, compute_capability="sm_87"), "a malformed compute capability"
        )
        diagnostics["device name"] = reject(
            root, dict(valid, device_name=""), "an empty device name"
        )
        diagnostics["total"] = reject(
            root,
            dict(valid, total_bytes=valid["free_before_bytes"] - 1),
            "free memory above the device total",
        )
        diagnostics["boolean int"] = reject(
            root, dict(valid, embedded_vector_dim=True), "a boolean where an int is required"
        )
        diagnostics["unknown field"] = reject(
            root, dict(valid, unexpected="x"), "an unknown field"
        )

        # AC80-18: each required field, missing.
        for key in REQUIRED_KEYS:
            record = {name: value for name, value in valid.items() if name != key}
            diagnostics[f"missing {key}"] = reject(root, record, f"a record missing {key}")

        # Malformed and non-canonical bytes.
        malformed = write_record(root, valid, raw=b"{not json")
        assert run(malformed).returncode == 1, "accepted malformed JSON"
        diagnostics["malformed json"] = run(malformed).stderr.strip()
        non_canonical = write_record(root, valid, raw=json.dumps(valid, indent=2).encode("ascii"))
        assert run(non_canonical).returncode == 1, "accepted non-canonical JSON"
        diagnostics["non-canonical json"] = run(non_canonical).stderr.strip()
        array = write_record(root, valid, raw=canonical([valid]))
        assert run(array).returncode == 1, "accepted a JSON array"
        diagnostics["not an object"] = run(array).stderr.strip()
        missing_file = run(root / "absent.json")
        assert missing_file.returncode == 1, "accepted a missing witness file"
        diagnostics["absent file"] = missing_file.stderr.strip()

    # Every rejection above must be individually diagnosable.
    duplicates = len(diagnostics) - len(set(diagnostics.values()))
    assert duplicates == 0, f"{duplicates} rejection diagnostics are not distinct"
    print(f"tegra-gpu-witness-contract: pass ({len(diagnostics)} distinct rejections)")


if __name__ == "__main__":
    main()
