#!/usr/bin/env python3
"""CPU-only contract for current v2 Tegra proof and strict v1 refusal.

The v2 fixtures are explicit synthetic examples of the approved tagged schema,
not new hardware evidence. The historical real v1 bytes remain untouched.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
VERIFIER = REPO / "scripts/release/verify-tegra-gpu-witness.py"
HISTORICAL = REPO / "scripts/tests/fixtures/tegra-gpu-witness/valid/tegra-gpu-allocation-witness.json"
FIXTURE = REPO / "scripts/tests/fixtures/tegra-gpu-witness/v2/private-pool-used.json"
GLOBAL_FIXTURE = FIXTURE.with_name("global-free.json")
NVIDIA_SMI_UUID = "bbbe9f37-7028-556a-930b-54e5f3b67a82"

REQUIRED_KEYS = (
    "compute_capability", "delta_bytes", "delta_floor_bytes", "device_integrated",
    "device_name", "device_ordinal_actual", "device_ordinal_requested", "device_uuid",
    "embedded_vector_dim", "free_after_bytes", "free_before_bytes", "measurement",
    "process_id", "schema", "total_bytes",
)
PRIVATE_KEYS = (
    "kind", "context_id", "pool_identity", "used_before_bytes", "used_after_bytes",
    "delta_bytes", "control_request_bytes", "control_before_bytes", "control_live_bytes",
    "control_released_bytes", "pool_exclusivity_precondition",
)
GLOBAL_KEYS = (
    "kind", "control_allocation_request_bytes", "control_block_count",
    "control_free_before_bytes", "control_free_after_bytes", "control_delta_bytes",
    "sole_gpu_consumer_precondition",
)


def measured(record: dict[str, object], **changes: object) -> dict[str, object]:
    return dict(record, measurement=dict(record["measurement"], **changes))


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
    assert set(valid) == set(REQUIRED_KEYS)
    assert set(valid["measurement"]) == set(PRIVATE_KEYS)
    assert valid["delta_bytes"] == valid["free_before_bytes"] - valid["free_after_bytes"] < 0
    proof = valid["measurement"]
    assert proof["used_after_bytes"] - proof["used_before_bytes"] == proof["delta_bytes"] == 132_848_640
    assert proof["control_live_bytes"] - proof["control_before_bytes"] == 67_108_864
    assert proof["control_released_bytes"] == proof["control_before_bytes"]
    assert proof["context_id"] == "18446744073709551615"
    global_record = json.loads(GLOBAL_FIXTURE.read_bytes())
    assert GLOBAL_FIXTURE.read_bytes() == canonical(global_record)
    assert set(global_record) == set(REQUIRED_KEYS)
    assert set(global_record["measurement"]) == set(GLOBAL_KEYS)
    assert global_record["delta_bytes"] == global_record["free_before_bytes"] - global_record["free_after_bytes"]
    diagnostics: dict[str, str] = {}

    with tempfile.TemporaryDirectory() as raw_root:
        root = Path(raw_root)

        # This current private proof passes even though its global delta is negative.
        result = run(FIXTURE)
        assert result.returncode == 0, f"rejected the committed fixture: {result.stderr}"
        assert "pass" in result.stdout, result.stdout
        accept(root, valid, "valid record with normalized nvidia-smi UUID", "--expect-uuid", NVIDIA_SMI_UUID)
        accept(root, valid, "valid record with prefixed UUID", "--expect-uuid", valid["device_uuid"])

        reject(root, json.loads(HISTORICAL.read_bytes()), "historical v1 on current verifier")
        reject(root, global_record, "discrete global proof on the Tegra route")
        accept(root, measured(valid, context_id="0"), "valid zero-valued uint64 context ID")

        # The private USED delta is authoritative; global arithmetic remains truthful.
        zero = measured(valid, used_after_bytes=0, delta_bytes=0)
        diagnostics["zero delta"] = reject(root, zero, "a zero delta")
        negative = measured(valid, used_before_bytes=4096, used_after_bytes=0, delta_bytes=-4096)
        diagnostics["negative delta"] = reject(root, negative, "a negative USED delta")
        below = measured(valid, used_after_bytes=1_048_576, delta_bytes=1_048_576)
        diagnostics["below floor"] = reject(root, below, "a below-floor delta")

        # A delta that does not follow from the retained samples (R80-13).
        diagnostics["unre-derivable delta"] = reject(
            root, measured(valid, delta_bytes=proof["delta_bytes"] + 1), "an unre-derivable USED delta"
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

        # One exact 64 MiB control must rise and return to the post-forward baseline.
        diagnostics["control not observed"] = reject(
            root, measured(valid, control_live_bytes=proof["control_before_bytes"] + 1024),
            "a private control the counter never saw",
        )
        diagnostics["control request"] = reject(
            root, measured(valid, control_request_bytes=1024), "a weakened control request",
        )
        diagnostics["oversized control"] = reject(
            root, measured(valid, control_request_bytes=134_217_728), "a nonexact 128 MiB control request",
        )
        diagnostics["context numeric"] = reject(
            root, measured(valid, context_id=18446744073709551615), "a numeric rather than lossless string context ID",
        )
        diagnostics["dimension nonzero wrong"] = reject(
            root, dict(valid, embedded_vector_dim=383), "a nonzero wrong forward dimension",
        )
        diagnostics["control release"] = reject(
            root, measured(valid, control_released_bytes=proof["control_before_bytes"] + 1),
            "an inexact control release",
        )
        diagnostics["global arithmetic"] = reject(
            root, dict(valid, delta_bytes=valid["delta_bytes"] + 1), "false global diagnostics",
        )
        diagnostics["method compatibility"] = reject(
            root, dict(valid, device_integrated=False), "private proof on a discrete GPU",
        )
        diagnostics["process"] = reject(root, dict(valid, process_id=0), "a missing process binding")
        diagnostics["context"] = reject(root, measured(valid, context_id="18446744073709551616"), "context ID outside uint64")
        diagnostics["pool"] = reject(root, measured(valid, pool_identity=""), "an empty pool identity")
        diagnostics["kind"] = reject(root, measured(valid, kind="reserved"), "a non-USED method")
        diagnostics["irrelevant field"] = reject(root, measured(valid, control_block_count=1), "a mixed variant")

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
            root, measured(valid, pool_exclusivity_precondition="fine"), "a softened pool precondition"
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

        for key in PRIVATE_KEYS:
            measurement = {name: value for name, value in proof.items() if name != key}
            diagnostics[f"missing measurement.{key}"] = reject(
                root, dict(valid, measurement=measurement), f"a private proof missing {key}",
            )

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
