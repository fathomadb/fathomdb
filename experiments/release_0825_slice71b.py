"""Strict contracts and classifiers for the bounded Slice 71B write work."""

from __future__ import annotations

import hashlib
import json
import math
import re
import statistics
from collections.abc import Mapping
from datetime import datetime
from pathlib import Path
from typing import NoReturn


class Slice71BContractError(ValueError):
    """Raised when a Slice 71B manifest or receipt drifts from its sealed shape."""


_SHA1 = re.compile(r"[0-9a-f]{40}")
_SHA256 = re.compile(r"[0-9a-f]{64}")
_TREATMENTS = ["production", "generation_only", "no_op"]
_ATTRIBUTION_ORDER = [
    "production",
    "generation_only",
    "no_op",
    "no_op",
    "generation_only",
    "production",
    "generation_only",
    "production",
    "no_op",
]
_SIZES = [1, 10, 100, 1_000, 10_000]
_TOP_KEYS = {
    "schema_version",
    "release",
    "approval",
    "product_sources",
    "probe",
    "runner",
    "fixtures",
    "treatments",
    "attribution_order",
    "policy",
    "environment_policy",
    "timeouts_s",
    "raw_root",
}
_APPROVAL_KEYS = {"state", "approved_by", "approved_at"}
_SOURCE_KEYS = {"scale02_baseline", "ac013_baseline", "unchanged_current"}
_FILE_BINDING_KEYS = {"path", "sha256"}
_PROBE_KEYS = {"path", "sha256", "lock_path", "lock_sha256"}
_FIXTURE_KEYS = {"scale02", "ac013"}
_SCALE02_KEYS = {
    "input_jsonl",
    "input_sha256",
    "fixture_sha256",
    "sizes",
    "batch_size_small",
    "batch_size_10k",
    "embedder",
}
_AC013_KEYS = {
    "generator",
    "generator_sha256",
    "fixture_sha256",
    "sizes",
    "batch_size_small",
    "batch_size_10k",
    "vector_dim",
    "embedder",
}
_POLICY_KEYS = {
    "repetitions_per_arm",
    "max_within_arm_spread_percent",
    "ten_k_limit_percent",
    "small_limit_percent",
    "small_absolute_floor_ms",
    "conditional_preparation_factorial",
}
_ENV_KEYS = {
    "max_load_per_online_cpu",
    "min_available_memory_percent",
    "max_swap_io_delta",
    "max_cpu_temp_c",
    "thermal_signal",
    "forbid_competing_processes",
}
_TIMEOUT_KEYS = {"build", "cell"}
_RECEIPT_KEYS = {
    "schema_version",
    "manifest_sha256",
    "started_at",
    "finished_at",
    "cells",
    "classification",
    "failure",
    "errors",
}
_CLASSIFICATION_KEYS = {
    "state",
    "supported_causes",
    "conditional_preparation_factorial_required",
}
_CELL_KEYS = {
    "fixture",
    "treatment",
    "ordinal",
    "source_ref",
    "source_tree_dirty",
    "probe_sha256",
    "probe_lock_sha256",
    "runner_sha256",
    "executable_sha256",
    "executable_path",
    "fixture_sha256",
    "started_at",
    "finished_at",
    "environment_valid",
    "environment_sha256",
    "environment_path",
    "raw_log_sha256",
    "raw_log_path",
    "build_identity",
    "runtime_identity",
    "metrics",
}
_BUILD_KEYS = {"cargo", "rustc", "rustc_verbose_sha256", "profile"}
_RUNTIME_KEYS = {"sqlite_version", "sqlite_source_id", "libsqlite3_sys"}
_FAILURE_KEYS = {
    "state",
    "fixture",
    "treatment",
    "ordinal",
    "occurred_at",
    "message",
    "artifacts",
}
_FAILURE_ARTIFACT_KEYS = {"kind", "path", "sha256"}
_FAILURE_STATES = {
    "environment_invalid",
    "probe_failed",
    "build_failed",
    "harness_failed",
    "spread_invalid",
}
_FAILURE_ARTIFACT_KINDS = {
    "attempt_disposition",
    "build_log",
    "environment",
    "raw_log",
    "cell_disposition",
}
_METRIC_KEYS = {
    "records",
    "batch_size",
    "transactions",
    "ingest_ack_ms",
    "projection_drain_ms",
    "total_ms",
    "generation_before",
    "generation_after_ack",
    "generation_after_drain",
    "nonce_before",
    "nonce_after_ack",
    "nonce_after_drain",
    "trigger_inventory",
    "database_bytes",
    "wal_bytes",
    "process_cpu_seconds",
    "peak_rss_bytes",
}


def _fail(message: str) -> NoReturn:
    raise Slice71BContractError(message)


def _mapping(value: object, path: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        _fail(f"{path} must be an object")
    return value


def _exact_keys(value: Mapping[str, object], keys: set[str], path: str) -> None:
    if set(value) != keys:
        _fail(
            f"{path} keys differ: missing={sorted(keys - set(value))} "
            f"extra={sorted(set(value) - keys)}"
        )


def _expect(value: object, expected: object, path: str) -> None:
    if value != expected or type(value) is not type(expected):
        _fail(f"{path} must be {expected!r}")


def _digest(value: object, path: str, pattern: re.Pattern[str] = _SHA256) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        _fail(f"{path} has an invalid digest")
    return value


def _nonempty(value: object, path: str) -> str:
    if not isinstance(value, str) or not value:
        _fail(f"{path} must be a non-empty string")
    return value


def _timestamp(value: object, path: str) -> datetime:
    text = _nonempty(value, path)
    try:
        parsed = datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError as exc:
        _fail(f"{path} must be an ISO-8601 timestamp: {exc}")
    if parsed.tzinfo is None:
        _fail(f"{path} must include a timezone")
    return parsed


def _positive(value: object, path: str) -> float:
    if (
        isinstance(value, bool)
        or not isinstance(value, int | float)
        or not math.isfinite(float(value))
        or value <= 0
    ):
        _fail(f"{path} must be a positive finite number")
    return float(value)


def _nullable_generation(value: object, path: str) -> None:
    if value is not None and (
        isinstance(value, bool) or not isinstance(value, int) or value < 0
    ):
        _fail(f"{path} must be a non-negative integer or null")


def attribution_classification(cells: object) -> dict[str, object]:
    """Derive only causes measured by the three trigger-body arms."""

    if not isinstance(cells, list):
        _fail("classification cells must be an array")
    grouped: dict[tuple[str, str], list[float]] = {}
    for value in cells:
        cell = _mapping(value, "/classification/cell")
        metrics = _mapping(cell["metrics"], "/classification/cell/metrics")
        grouped.setdefault((str(cell["fixture"]), str(cell["treatment"])), []).append(
            float(metrics["total_ms"])
        )
    causes: list[str] = []
    generation_supported = True
    nonce_supported = True
    try:
        for fixture in ("scale02", "ac013"):
            medians = {
                treatment: statistics.median(grouped[(fixture, treatment)])
                for treatment in _TREATMENTS
            }
            generation_delta = medians["generation_only"] - medians["no_op"]
            nonce_delta = medians["production"] - medians["generation_only"]
            generation_supported &= (
                generation_delta > 0.25 and generation_delta / medians["no_op"] > 0.10
            )
            nonce_supported &= (
                nonce_delta > 0.25 and nonce_delta / medians["generation_only"] > 0.10
            )
    except (KeyError, statistics.StatisticsError, ZeroDivisionError):
        _fail("classification requires every sealed treatment repetition")
    if generation_supported:
        causes.append("per_row_visibility_update")
    if nonce_supported:
        causes.append("per_fire_nonce")
    unresolved = not causes
    return {
        "state": "unresolved" if unresolved else "supported",
        "supported_causes": causes,
        "conditional_preparation_factorial_required": unresolved,
    }


def validate_attribution_manifest(
    document: object, *, verify_files: bool = True
) -> None:
    """Reject drift in the sealed Slice 71B attribution protocol."""

    root = _mapping(document, "/")
    _exact_keys(root, _TOP_KEYS, "/")
    _expect(
        root["schema_version"], "slice71b-attribution-manifest.v1", "/schema_version"
    )
    _expect(root["release"], "0.8.25", "/release")

    approval = _mapping(root["approval"], "/approval")
    _exact_keys(approval, _APPROVAL_KEYS, "/approval")
    _expect(approval["state"], "approved", "/approval/state")
    _expect(approval["approved_by"], "HITL", "/approval/approved_by")
    _nonempty(approval["approved_at"], "/approval/approved_at")

    sources = _mapping(root["product_sources"], "/product_sources")
    _exact_keys(sources, _SOURCE_KEYS, "/product_sources")
    _expect(
        sources["scale02_baseline"],
        "b2bfb1f318f58041144acb2356a6a4c9624068b9",
        "/product_sources/scale02_baseline",
    )
    _expect(
        sources["ac013_baseline"],
        "4fc1b890a11ebfaa8f11b15823656e856002807a",
        "/product_sources/ac013_baseline",
    )
    _digest(sources["unchanged_current"], "/product_sources/unchanged_current", _SHA1)

    probe = _mapping(root["probe"], "/probe")
    _exact_keys(probe, _PROBE_KEYS, "/probe")
    probe_path = Path(_nonempty(probe["path"], "/probe/path"))
    probe_sha = _digest(probe["sha256"], "/probe/sha256")
    lock_path = Path(_nonempty(probe["lock_path"], "/probe/lock_path"))
    lock_sha = _digest(probe["lock_sha256"], "/probe/lock_sha256")
    runner = _mapping(root["runner"], "/runner")
    _exact_keys(runner, _FILE_BINDING_KEYS, "/runner")
    runner_path = Path(_nonempty(runner["path"], "/runner/path"))
    runner_sha = _digest(runner["sha256"], "/runner/sha256")

    fixtures = _mapping(root["fixtures"], "/fixtures")
    _exact_keys(fixtures, _FIXTURE_KEYS, "/fixtures")
    scale02 = _mapping(fixtures["scale02"], "/fixtures/scale02")
    _exact_keys(scale02, _SCALE02_KEYS, "/fixtures/scale02")
    _expect(scale02["sizes"], _SIZES, "/fixtures/scale02/sizes")
    _expect(
        scale02["batch_size_small"],
        "one_transaction",
        "/fixtures/scale02/batch_size_small",
    )
    _expect(scale02["batch_size_10k"], 256, "/fixtures/scale02/batch_size_10k")
    _expect(scale02["embedder"], "none", "/fixtures/scale02/embedder")
    input_path = Path(
        _nonempty(scale02["input_jsonl"], "/fixtures/scale02/input_jsonl")
    )
    input_sha = _digest(scale02["input_sha256"], "/fixtures/scale02/input_sha256")
    _digest(scale02["fixture_sha256"], "/fixtures/scale02/fixture_sha256")

    ac013 = _mapping(fixtures["ac013"], "/fixtures/ac013")
    _exact_keys(ac013, _AC013_KEYS, "/fixtures/ac013")
    _expect(ac013["sizes"], _SIZES, "/fixtures/ac013/sizes")
    _expect(
        ac013["batch_size_small"], "one_transaction", "/fixtures/ac013/batch_size_small"
    )
    _expect(ac013["batch_size_10k"], 1_024, "/fixtures/ac013/batch_size_10k")
    _expect(ac013["vector_dim"], 384, "/fixtures/ac013/vector_dim")
    _expect(ac013["embedder"], "varying/perf-gates-dense", "/fixtures/ac013/embedder")
    generator_path = Path(_nonempty(ac013["generator"], "/fixtures/ac013/generator"))
    generator_sha = _digest(
        ac013["generator_sha256"], "/fixtures/ac013/generator_sha256"
    )
    _digest(ac013["fixture_sha256"], "/fixtures/ac013/fixture_sha256")

    _expect(root["treatments"], _TREATMENTS, "/treatments")
    _expect(root["attribution_order"], _ATTRIBUTION_ORDER, "/attribution_order")

    policy = _mapping(root["policy"], "/policy")
    _exact_keys(policy, _POLICY_KEYS, "/policy")
    for key, expected in {
        "repetitions_per_arm": 3,
        "max_within_arm_spread_percent": 25,
        "ten_k_limit_percent": 20,
        "small_limit_percent": 10,
        "small_absolute_floor_ms": 0.25,
        "conditional_preparation_factorial": "only_if_three_arm_attribution_is_unresolved",
    }.items():
        _expect(policy[key], expected, f"/policy/{key}")

    environment = _mapping(root["environment_policy"], "/environment_policy")
    _exact_keys(environment, _ENV_KEYS, "/environment_policy")
    for key, expected in {
        "max_load_per_online_cpu": 0.5,
        "min_available_memory_percent": 25,
        "max_swap_io_delta": 0,
        "max_cpu_temp_c": 90,
        "thermal_signal": "k10temp:Tctl",
        "forbid_competing_processes": True,
    }.items():
        _expect(environment[key], expected, f"/environment_policy/{key}")
    timeouts = _mapping(root["timeouts_s"], "/timeouts_s")
    _exact_keys(timeouts, _TIMEOUT_KEYS, "/timeouts_s")
    _expect(timeouts["build"], 900, "/timeouts_s/build")
    _expect(timeouts["cell"], 1_800, "/timeouts_s/cell")
    _expect(root["raw_root"], "dev/plans/runs/0.8.25-slice-71/71b", "/raw_root")

    if verify_files:
        repo = Path(__file__).resolve().parent.parent
        for path, expected, label in (
            (repo / probe_path, probe_sha, "probe"),
            (repo / lock_path, lock_sha, "probe lock"),
            (repo / runner_path, runner_sha, "runner"),
            (repo / generator_path, generator_sha, "generator"),
            (repo / input_path, input_sha, "scale02 input"),
        ):
            try:
                actual = hashlib.sha256(path.read_bytes()).hexdigest()
            except OSError as exc:
                _fail(f"{label} is unavailable: {exc}")
            if actual != expected:
                _fail(f"{label} digest drifted")


def validate_attribution_receipt(
    document: object,
    manifest_document: object,
    *,
    verify_hashes: bool = True,
) -> None:
    """Reject incomplete, reordered, unstable, or internally inconsistent evidence."""

    validate_attribution_manifest(manifest_document, verify_files=verify_hashes)
    manifest = _mapping(manifest_document, "/manifest")
    root = _mapping(document, "/")
    _exact_keys(root, _RECEIPT_KEYS, "/")
    _expect(
        root["schema_version"], "slice71b-attribution-receipt.v1", "/schema_version"
    )
    manifest_sha = _digest(root["manifest_sha256"], "/manifest_sha256")
    receipt_started = _timestamp(root["started_at"], "/started_at")
    receipt_finished = _timestamp(root["finished_at"], "/finished_at")
    if receipt_finished < receipt_started:
        _fail("/finished_at precedes /started_at")
    if verify_hashes:
        manifest_path = (
            Path(__file__).resolve().parent
            / "configs"
            / "release-0825-slice71b-attribution-manifest.v1.json"
        )
        if hashlib.sha256(manifest_path.read_bytes()).hexdigest() != manifest_sha:
            _fail("/manifest_sha256 does not bind the checked-in manifest")

    cells = root["cells"]
    if not isinstance(cells, list):
        _fail("/cells must be an array")
    expected = [
        (fixture, treatment)
        for fixture in ("scale02", "ac013")
        for treatment in manifest["attribution_order"]  # type: ignore[union-attr]
    ]
    if len(cells) > len(expected):
        _fail("/cells exceeds the sealed 18-cell attribution matrix")

    sources = _mapping(manifest["product_sources"], "/manifest/product_sources")
    probe = _mapping(manifest["probe"], "/manifest/probe")
    fixtures = _mapping(manifest["fixtures"], "/manifest/fixtures")
    ordinals = {
        fixture: {treatment: 0 for treatment in _TREATMENTS}
        for fixture in ("scale02", "ac013")
    }
    grouped: dict[tuple[str, str], list[Mapping[str, object]]] = {}
    prior_finished = receipt_started
    for index, (value, (expected_fixture, expected_treatment)) in enumerate(
        zip(cells, expected)
    ):
        path = f"/cells/{index}"
        cell = _mapping(value, path)
        _exact_keys(cell, _CELL_KEYS, path)
        _expect(cell["fixture"], expected_fixture, f"{path}/fixture")
        _expect(cell["treatment"], expected_treatment, f"{path}/treatment")
        ordinals[expected_fixture][expected_treatment] += 1
        _expect(
            cell["ordinal"],
            ordinals[expected_fixture][expected_treatment],
            f"{path}/ordinal",
        )
        _digest(cell["source_ref"], f"{path}/source_ref", _SHA1)
        _expect(cell["source_tree_dirty"], False, f"{path}/source_tree_dirty")
        _digest(cell["probe_sha256"], f"{path}/probe_sha256")
        _digest(cell["probe_lock_sha256"], f"{path}/probe_lock_sha256")
        _digest(cell["runner_sha256"], f"{path}/runner_sha256")
        _digest(cell["executable_sha256"], f"{path}/executable_sha256")
        _nonempty(cell["executable_path"], f"{path}/executable_path")
        _digest(cell["fixture_sha256"], f"{path}/fixture_sha256")
        if verify_hashes:
            _expect(
                cell["source_ref"], sources["unchanged_current"], f"{path}/source_ref"
            )
            _expect(cell["probe_sha256"], probe["sha256"], f"{path}/probe_sha256")
            _expect(
                cell["probe_lock_sha256"],
                probe["lock_sha256"],
                f"{path}/probe_lock_sha256",
            )
            runner = _mapping(manifest["runner"], "/manifest/runner")
            _expect(cell["runner_sha256"], runner["sha256"], f"{path}/runner_sha256")
            fixture = _mapping(
                fixtures[expected_fixture], f"/manifest/fixtures/{expected_fixture}"
            )
            _expect(
                cell["fixture_sha256"],
                fixture["fixture_sha256"],
                f"{path}/fixture_sha256",
            )
        cell_started = _timestamp(cell["started_at"], f"{path}/started_at")
        cell_finished = _timestamp(cell["finished_at"], f"{path}/finished_at")
        if cell_started < prior_finished or cell_finished < cell_started:
            _fail(f"{path} timestamps violate sealed execution order")
        prior_finished = cell_finished
        _expect(cell["environment_valid"], True, f"{path}/environment_valid")
        _digest(cell["environment_sha256"], f"{path}/environment_sha256")
        _nonempty(cell["environment_path"], f"{path}/environment_path")
        _digest(cell["raw_log_sha256"], f"{path}/raw_log_sha256")
        _nonempty(cell["raw_log_path"], f"{path}/raw_log_path")
        if verify_hashes:
            repo = Path(__file__).resolve().parent.parent
            for file_key, digest_key in (
                ("executable_path", "executable_sha256"),
                ("environment_path", "environment_sha256"),
                ("raw_log_path", "raw_log_sha256"),
            ):
                artifact = Path(str(cell[file_key]))
                artifact = artifact if artifact.is_absolute() else repo / artifact
                try:
                    actual = hashlib.sha256(artifact.read_bytes()).hexdigest()
                except OSError as exc:
                    _fail(f"{path}/{file_key} is unavailable: {exc}")
                _expect(
                    actual, cell[digest_key], f"{path}/{digest_key} (artifact bytes)"
                )
        build = _mapping(cell["build_identity"], f"{path}/build_identity")
        _exact_keys(build, _BUILD_KEYS, f"{path}/build_identity")
        _nonempty(build["cargo"], f"{path}/build_identity/cargo")
        _nonempty(build["rustc"], f"{path}/build_identity/rustc")
        _digest(
            build["rustc_verbose_sha256"], f"{path}/build_identity/rustc_verbose_sha256"
        )
        _expect(build["profile"], "release", f"{path}/build_identity/profile")
        runtime = _mapping(cell["runtime_identity"], f"{path}/runtime_identity")
        _exact_keys(runtime, _RUNTIME_KEYS, f"{path}/runtime_identity")
        for key in _RUNTIME_KEYS:
            _nonempty(runtime[key], f"{path}/runtime_identity/{key}")

        metrics = _mapping(cell["metrics"], f"{path}/metrics")
        _exact_keys(metrics, _METRIC_KEYS, f"{path}/metrics")
        _expect(metrics["records"], 10_000, f"{path}/metrics/records")
        batch_size = 256 if expected_fixture == "scale02" else 1_024
        _expect(metrics["batch_size"], batch_size, f"{path}/metrics/batch_size")
        _expect(
            metrics["transactions"],
            math.ceil(10_000 / batch_size),
            f"{path}/metrics/transactions",
        )
        ack = _positive(metrics["ingest_ack_ms"], f"{path}/metrics/ingest_ack_ms")
        drain = _positive(
            metrics["projection_drain_ms"], f"{path}/metrics/projection_drain_ms"
        )
        total = _positive(metrics["total_ms"], f"{path}/metrics/total_ms")
        if not math.isclose(total, ack + drain, rel_tol=1e-6, abs_tol=0.05):
            _fail(f"{path}/metrics/total_ms must equal acknowledgement plus drain")
        for key in (
            "generation_before",
            "generation_after_ack",
            "generation_after_drain",
        ):
            _nullable_generation(metrics[key], f"{path}/metrics/{key}")
        for key in ("nonce_before", "nonce_after_ack", "nonce_after_drain"):
            if metrics[key] is not None:
                _digest(metrics[key], f"{path}/metrics/{key}")
        for key in (
            "trigger_inventory",
            "database_bytes",
            "wal_bytes",
            "peak_rss_bytes",
        ):
            if (
                isinstance(metrics[key], bool)
                or not isinstance(metrics[key], int)
                or metrics[key] < 0
            ):
                _fail(f"{path}/metrics/{key} must be a non-negative integer")
        _positive(metrics["process_cpu_seconds"], f"{path}/metrics/process_cpu_seconds")
        _expect(metrics["trigger_inventory"], 54, f"{path}/metrics/trigger_inventory")
        generations = [
            metrics[key]
            for key in (
                "generation_before",
                "generation_after_ack",
                "generation_after_drain",
            )
        ]
        nonces = [
            metrics[key]
            for key in ("nonce_before", "nonce_after_ack", "nonce_after_drain")
        ]
        if any(value is None for value in generations + nonces):
            _fail(f"{path} current-source visibility observations must be present")
        before, after_ack, after_drain = (int(value) for value in generations)
        nonce_before, nonce_ack, nonce_drain = (str(value) for value in nonces)
        if expected_treatment == "production":
            ack_generation_changed = after_ack > before
            ack_nonce_changed = nonce_ack != nonce_before
            drain_generation_changed = after_drain > after_ack
            drain_nonce_changed = nonce_drain != nonce_ack
            if not (
                ack_generation_changed
                and ack_nonce_changed
                and after_drain >= after_ack
                and drain_generation_changed == drain_nonce_changed
            ):
                _fail(f"{path} production treatment did not advance visibility")
        elif expected_treatment == "generation_only":
            if not (after_ack > before and after_drain >= after_ack):
                _fail(f"{path} generation-only treatment did not advance generation")
            if len({nonce_before, nonce_ack, nonce_drain}) != 1:
                _fail(f"{path} generation-only treatment rotated the nonce")
        elif not (
            before == after_ack == after_drain
            and nonce_before == nonce_ack == nonce_drain
        ):
            _fail(f"{path} no-op treatment changed visibility state")
        grouped.setdefault((expected_fixture, expected_treatment), []).append(cell)

    if receipt_finished < prior_finished:
        _fail("/finished_at precedes the final cell")

    spread_limit = float(
        _mapping(manifest["policy"], "/manifest/policy")[
            "max_within_arm_spread_percent"
        ]
    )
    spread_violations: list[str] = []
    for (fixture, treatment), arm_cells in grouped.items():
        for metric in ("ingest_ack_ms", "total_ms"):
            values = [
                float(_mapping(cell["metrics"], "/metrics")[metric])
                for cell in arm_cells
            ]
            spread = (max(values) / min(values) - 1.0) * 100.0
            if spread > spread_limit:
                spread_violations.append(
                    f"{fixture}/{treatment}/{metric} spread {spread:.3f}% exceeds "
                    f"{spread_limit}%"
                )

    classification = _mapping(root["classification"], "/classification")
    _exact_keys(classification, _CLASSIFICATION_KEYS, "/classification")
    errors = root["errors"]
    if not isinstance(errors, list) or any(
        not isinstance(error, str) or not error for error in errors
    ):
        _fail("/errors must contain only non-empty strings")
    state = classification["state"]
    if state in {"supported", "unresolved"}:
        if len(cells) != len(expected):
            _fail("a completed receipt must contain the exact 18-cell matrix")
        _expect(classification, attribution_classification(cells), "/classification")
        if errors:
            _fail("/errors must be empty for a complete attribution receipt")
        if spread_violations:
            _fail(spread_violations[0])
        _expect(root["failure"], None, "/failure")
    else:
        if state not in _FAILURE_STATES:
            _fail("an incomplete receipt must carry a canonical abort state")
        _expect(
            classification["supported_causes"], [], "/classification/supported_causes"
        )
        _expect(
            classification["conditional_preparation_factorial_required"],
            False,
            "/classification/conditional_preparation_factorial_required",
        )
        if not errors:
            _fail("an incomplete receipt must retain its stop reason")
        if state == "spread_invalid" and not spread_violations:
            _fail("spread_invalid requires a measured spread violation")
        failure = _mapping(root["failure"], "/failure")
        _exact_keys(failure, _FAILURE_KEYS, "/failure")
        _expect(failure["state"], state, "/failure/state")
        message = _nonempty(failure["message"], "/failure/message")
        _expect(errors, [message], "/errors")
        occurred_at = _timestamp(failure["occurred_at"], "/failure/occurred_at")
        if occurred_at < receipt_started or occurred_at > receipt_finished:
            _fail("/failure/occurred_at lies outside the receipt interval")
        if state == "build_failed" or (
            state == "harness_failed" and failure["fixture"] is None
        ):
            for key in ("fixture", "treatment", "ordinal"):
                _expect(failure[key], None, f"/failure/{key}")
        else:
            failed_index = len(cells) - 1 if state == "spread_invalid" else len(cells)
            if failed_index >= len(expected):
                _fail("/failure does not identify a position in the sealed matrix")
            failed_fixture, failed_treatment = expected[failed_index]
            _expect(failure["fixture"], failed_fixture, "/failure/fixture")
            _expect(failure["treatment"], failed_treatment, "/failure/treatment")
            prior_same_arm = sum(
                1
                for prior_fixture, prior_treatment in expected[:failed_index]
                if prior_fixture == failed_fixture
                and prior_treatment == failed_treatment
            )
            _expect(failure["ordinal"], prior_same_arm + 1, "/failure/ordinal")
        artifacts = failure["artifacts"]
        if not isinstance(artifacts, list) or not artifacts:
            _fail("/failure/artifacts must bind retained failure evidence")
        kinds: set[str] = set()
        disposition_path: Path | None = None
        for index, value in enumerate(artifacts):
            path = f"/failure/artifacts/{index}"
            artifact = _mapping(value, path)
            _exact_keys(artifact, _FAILURE_ARTIFACT_KEYS, path)
            kind = _nonempty(artifact["kind"], f"{path}/kind")
            if kind not in _FAILURE_ARTIFACT_KINDS or kind in kinds:
                _fail(f"{path}/kind is invalid or duplicated")
            kinds.add(kind)
            artifact_path = Path(_nonempty(artifact["path"], f"{path}/path"))
            if kind == "attempt_disposition":
                disposition_path = artifact_path
            digest = _digest(artifact["sha256"], f"{path}/sha256")
            if verify_hashes:
                artifact_path = (
                    artifact_path
                    if artifact_path.is_absolute()
                    else Path(__file__).resolve().parent.parent / artifact_path
                )
                try:
                    actual = hashlib.sha256(artifact_path.read_bytes()).hexdigest()
                except OSError as exc:
                    _fail(f"{path}/path is unavailable: {exc}")
                _expect(actual, digest, f"{path}/sha256 (artifact bytes)")
        if "attempt_disposition" not in kinds:
            _fail("/failure/artifacts must bind an attempt disposition")
        if state == "build_failed" and "build_log" not in kinds:
            _fail("a build failure must bind its build log")
        if (
            state in {"environment_invalid", "probe_failed"}
            and "environment" not in kinds
        ):
            _fail(f"{state} must bind its environment disposition")
        if state == "probe_failed" and "raw_log" not in kinds:
            _fail("probe_failed must bind its raw log")
        if state == "spread_invalid" and "cell_disposition" not in kinds:
            _fail("spread_invalid must bind its triggering cell")
        if verify_hashes:
            assert disposition_path is not None
            disposition_path = (
                disposition_path
                if disposition_path.is_absolute()
                else Path(__file__).resolve().parent.parent / disposition_path
            )
            disposition = json.loads(disposition_path.read_text(encoding="utf-8"))
            expected_disposition = {
                "schema_version": "slice71b-failed-attempt.v1",
                "state": state,
                "fixture": failure["fixture"],
                "treatment": failure["treatment"],
                "ordinal": failure["ordinal"],
                "occurred_at": failure["occurred_at"],
                "message": message,
                "artifacts": [
                    artifact
                    for artifact in artifacts
                    if artifact["kind"] != "attempt_disposition"
                ],
            }
            _expect(disposition, expected_disposition, "/failure attempt disposition")


def classify_recovery(
    medians: Mapping[str, Mapping[int, Mapping[str, Mapping[str, float]]]],
) -> str:
    """Apply the preregistered 10k and joint small-write recovery boundaries."""

    for fixture in ("scale02", "ac013"):
        try:
            ten_k = medians[fixture][10_000]
            baseline = ten_k["baseline"]
            fixed = ten_k["fixed"]
        except KeyError:
            return "incomplete"
        for metric in ("ingest_ack_ms", "total_ms"):
            if baseline[metric] <= 0 or fixed[metric] > baseline[metric] * 1.20:
                return "missed_10k_boundary"
        for size, arms in medians[fixture].items():
            if size >= 10_000:
                continue
            try:
                baseline = arms["baseline"]
                fixed = arms["fixed"]
            except KeyError:
                return "incomplete"
            for metric in ("ingest_ack_ms", "total_ms"):
                absolute = fixed[metric] - baseline[metric]
                relative = fixed[metric] / baseline[metric] - 1.0
                if relative > 0.10 and absolute > 0.25:
                    return "small_write_regression"
    return "pass"


def main() -> int:
    """Validate the checked-in attribution manifest."""

    manifest_path = (
        Path(__file__).resolve().parent
        / "configs"
        / "release-0825-slice71b-attribution-manifest.v1.json"
    )
    validate_attribution_manifest(json.loads(manifest_path.read_text(encoding="utf-8")))
    print("ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
