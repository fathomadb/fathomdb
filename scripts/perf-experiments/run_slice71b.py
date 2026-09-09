#!/usr/bin/env python3
"""Build and execute the sealed Slice 71B attribution campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from experiments import scale_02  # noqa: E402
from experiments.release_0825_slice71b import (  # noqa: E402
    attribution_classification,
    validate_attribution_manifest,
    validate_attribution_receipt,
)


PROBE = ROOT / "experiments" / "slice71b_write_probe.rs"
DEFAULT_MANIFEST = (
    ROOT
    / "experiments"
    / "configs"
    / "release-0825-slice71b-attribution-manifest.v1.json"
)
DEFAULT_SCALE02_CONFIG = (
    ROOT / "experiments" / "configs" / "scale-02" / "a0-envelope.v2.json"
)


class CampaignAbort(RuntimeError):
    """Stop a campaign without replacing or discarding an attempted cell."""

    def __init__(self, state: str, message: str, failure: dict[str, Any]) -> None:
        super().__init__(message)
        self.state = state
        self.failure = failure


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _timestamp() -> str:
    return datetime.now(UTC).isoformat()


def _artifact_path(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path.resolve())


def generate_scale02_input(config_path: Path, output: Path) -> None:
    """Materialize the exact frozen Scale-02 10k input prefix as content JSONL."""

    config = scale_02.load_config(config_path)
    fixture = scale_02.load_fixture(config)
    rows = scale_02.build_rows(fixture.documents, 10_000, seed=config.growth_seed)
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(
                json.dumps(
                    {
                        "body": row.body,
                        "logical_id": row.logical_id,
                        "source_id": row.source_id,
                    },
                    sort_keys=True,
                    separators=(",", ":"),
                )
                + "\n"
            )
    print(
        json.dumps(
            {
                "input_jsonl": str(output.resolve()),
                "input_sha256": sha256(output),
                "fixture_sha256": fixture.fixture_digest,
                "rows": len(rows),
            },
            sort_keys=True,
        )
    )


def _read_first_number(path: Path, key: str) -> int:
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.split()
        if fields and fields[0] == key:
            return int(fields[1])
    raise RuntimeError(f"{key} is unavailable in {path}")


def _temperature() -> float:
    for hwmon in Path("/sys/class/hwmon").glob("hwmon*"):
        try:
            if (hwmon / "name").read_text(encoding="utf-8").strip() != "k10temp":
                continue
            for label in hwmon.glob("temp*_label"):
                if label.read_text(encoding="utf-8").strip() == "Tctl":
                    value = Path(str(label).replace("_label", "_input"))
                    return int(value.read_text(encoding="utf-8").strip()) / 1000.0
        except OSError:
            continue
    raise RuntimeError("required k10temp:Tctl signal is unavailable")


def _competing_processes() -> list[str]:
    completed = subprocess.run(
        ["ps", "-eo", "pid=,comm=,args="],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        timeout=10,
    )
    own_pid = os.getpid()
    blocked = ("cargo", "rustc", "slice71b-probe", "run-ac013.sh", "perf_gates")
    return [
        line.strip()
        for line in completed.stdout.splitlines()
        if line.strip()
        and not line.lstrip().startswith(str(own_pid) + " ")
        and any(token in line for token in blocked)
    ]


def environment_snapshot() -> dict[str, Any]:
    memory_total = _read_first_number(Path("/proc/meminfo"), "MemTotal:")
    memory_available = _read_first_number(Path("/proc/meminfo"), "MemAvailable:")
    return {
        "load_1m": os.getloadavg()[0],
        "available_memory_percent": memory_available * 100.0 / memory_total,
        "swap_in": _read_first_number(Path("/proc/vmstat"), "pswpin"),
        "swap_out": _read_first_number(Path("/proc/vmstat"), "pswpout"),
        "cpu_temp_c": _temperature(),
        "online_cpus": os.cpu_count() or 1,
        "competing_processes": _competing_processes(),
    }


def environment_valid(
    start: dict[str, Any], end: dict[str, Any], policy: dict[str, Any]
) -> bool:
    cpu_limit = float(policy["max_load_per_online_cpu"]) * int(start["online_cpus"])
    return (
        max(float(start["load_1m"]), float(end["load_1m"])) <= cpu_limit
        and min(
            float(start["available_memory_percent"]),
            float(end["available_memory_percent"]),
        )
        >= float(policy["min_available_memory_percent"])
        and int(end["swap_in"]) - int(start["swap_in"])
        <= int(policy["max_swap_io_delta"])
        and int(end["swap_out"]) - int(start["swap_out"])
        <= int(policy["max_swap_io_delta"])
        and max(float(start["cpu_temp_c"]), float(end["cpu_temp_c"]))
        <= float(policy["max_cpu_temp_c"])
        and not start["competing_processes"]
        and not end["competing_processes"]
    )


def _source_identity(source_root: Path) -> tuple[str, bool]:
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=source_root,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        timeout=10,
    ).stdout.strip()
    dirty = bool(
        subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=source_root,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            timeout=10,
        ).stdout.strip()
    )
    return commit, dirty


def _write_probe_manifest(directory: Path, source_root: Path) -> Path:
    manifest = directory / "Cargo.toml"
    engine = source_root / "src" / "rust" / "crates" / "fathomdb-engine"
    embedder_api = source_root / "src" / "rust" / "crates" / "fathomdb-embedder-api"
    manifest.write_text(
        "\n".join(
            [
                "[package]",
                'name = "slice71b-probe"',
                'version = "0.0.0"',
                'edition = "2021"',
                "",
                "[[bin]]",
                'name = "slice71b-probe"',
                f'path = "{PROBE}"',
                "",
                "[dependencies]",
                f'fathomdb-engine = {{ path = "{engine}" }}',
                f'fathomdb-embedder-api = {{ path = "{embedder_api}" }}',
                'rusqlite = { version = "0.40", features = ["bundled"] }',
                'serde_json = "1"',
                'libc = "0.2"',
                "",
                "[workspace]",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return manifest


def _command_output(command: list[str]) -> str:
    return subprocess.run(
        command,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=30,
    ).stdout.strip()


def _build_probe(
    source_root: Path,
    target_root: Path,
    lock_path: Path,
    build_log: Path,
    timeout_s: int,
) -> tuple[Path, dict[str, str]]:
    temporary = Path(tempfile.mkdtemp(prefix="fathomdb-slice71b-probe-"))
    try:
        manifest = _write_probe_manifest(temporary, source_root)
        shutil.copy2(lock_path, temporary / "Cargo.lock")
        with build_log.open("w", encoding="utf-8") as log:
            subprocess.run(
                [
                    "cargo",
                    "build",
                    "--offline",
                    "--locked",
                    "--release",
                    "--manifest-path",
                    str(manifest),
                    "--target-dir",
                    str(target_root),
                ],
                check=True,
                cwd=source_root,
                stdout=log,
                stderr=subprocess.STDOUT,
                timeout=timeout_s,
            )
    finally:
        shutil.rmtree(temporary)
    rustc_verbose = _command_output(["rustc", "-Vv"]).encode()
    identity = {
        "cargo": _command_output(["cargo", "--version"]),
        "rustc": _command_output(["rustc", "--version"]),
        "rustc_verbose_sha256": hashlib.sha256(rustc_verbose).hexdigest(),
        "profile": "release",
    }
    return target_root / "release" / "slice71b-probe", identity


def _libsqlite3_sys_version(lock_path: Path) -> str:
    document = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    versions = {
        package["version"]
        for package in document["package"]
        if package["name"] == "libsqlite3-sys"
    }
    if len(versions) != 1:
        raise RuntimeError("sealed probe lock must contain one libsqlite3-sys package")
    return versions.pop()


def _write_json(path: Path, document: object) -> None:
    path.write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def _artifact_binding(kind: str, path: Path) -> dict[str, str]:
    return {"kind": kind, "path": _artifact_path(path), "sha256": sha256(path)}


def _failure_disposition(
    *,
    state: str,
    message: str,
    root: Path,
    fixture: str | None,
    treatment: str | None,
    ordinal: int | None,
    extra_artifacts: tuple[tuple[str, Path], ...] = (),
) -> dict[str, Any]:
    candidates = [
        ("environment", root / "environment.json"),
        ("raw_log", root / "raw.log"),
        ("cell_disposition", root / "cell.json"),
        *extra_artifacts,
    ]
    artifacts = [
        _artifact_binding(kind, path) for kind, path in candidates if path.is_file()
    ]
    occurred_at = _timestamp()
    disposition_path = root / "attempt.json"
    _write_json(
        disposition_path,
        {
            "schema_version": "slice71b-failed-attempt.v1",
            "state": state,
            "fixture": fixture,
            "treatment": treatment,
            "ordinal": ordinal,
            "occurred_at": occurred_at,
            "message": message,
            "artifacts": artifacts,
        },
    )
    artifacts.append(_artifact_binding("attempt_disposition", disposition_path))
    return {
        "state": state,
        "fixture": fixture,
        "treatment": treatment,
        "ordinal": ordinal,
        "occurred_at": occurred_at,
        "message": message,
        "artifacts": artifacts,
    }


def _abort(state: str, message: str, failure: dict[str, Any]) -> NoReturn:
    raise CampaignAbort(state, message, failure)


def _treatment_signature_valid(treatment: str, metrics: dict[str, Any]) -> bool:
    if metrics.get("trigger_inventory") != 54:
        return False
    generations = [
        metrics.get("generation_before"),
        metrics.get("generation_after_ack"),
        metrics.get("generation_after_drain"),
    ]
    nonces = [
        metrics.get("nonce_before"),
        metrics.get("nonce_after_ack"),
        metrics.get("nonce_after_drain"),
    ]
    if any(value is None for value in generations + nonces):
        return False
    before, after_ack, after_drain = (int(value) for value in generations)
    nonce_before, nonce_ack, nonce_drain = (str(value) for value in nonces)
    if treatment == "production":
        drain_generation_changed = after_drain > after_ack
        drain_nonce_changed = nonce_drain != nonce_ack
        return (
            after_ack > before
            and after_drain >= after_ack
            and nonce_ack != nonce_before
            and drain_generation_changed == drain_nonce_changed
        )
    if treatment == "generation_only":
        return (
            after_ack > before
            and after_drain >= after_ack
            and len({nonce_before, nonce_ack, nonce_drain}) == 1
        )
    return (
        before == after_ack == after_drain and nonce_before == nonce_ack == nonce_drain
    )


def _run_cell(
    *,
    document: dict[str, Any],
    source_ref: str,
    executable: Path,
    build_identity: dict[str, str],
    libsqlite3_sys: str,
    fixture: str,
    treatment: str,
    ordinal: int,
    cell_root: Path,
) -> dict[str, Any]:
    fixture_config = document["fixtures"][fixture]
    cell_root.mkdir(parents=True, exist_ok=False)
    raw_log = cell_root / "raw.log"
    environment_path = cell_root / "environment.json"
    started_at = _timestamp()

    def fail(state: str, message: str) -> NoReturn:
        _abort(
            state,
            message,
            _failure_disposition(
                state=state,
                message=message,
                root=cell_root,
                fixture=fixture,
                treatment=treatment,
                ordinal=ordinal,
            ),
        )

    try:
        start = environment_snapshot()
    except Exception as exc:
        _write_json(environment_path, {"start_error": str(exc)})
        fail("environment_invalid", f"{fixture}/{treatment}-{ordinal}: {exc}")
    if not environment_valid(start, start, document["environment_policy"]):
        _write_json(environment_path, {"start": start, "end": start})
        fail(
            "environment_invalid",
            f"{fixture}/{treatment}-{ordinal}: pre-cell environment rejected; "
            f"see {_artifact_path(environment_path)}",
        )

    command = [
        str(executable),
        "--fixture",
        fixture,
        "--treatment",
        treatment,
        "--records",
        "10000",
        "--batch-size",
        str(fixture_config["batch_size_10k"]),
        "--database",
        str(cell_root / "cell.fathom"),
    ]
    if fixture == "scale02":
        command.extend(["--input-jsonl", str(ROOT / fixture_config["input_jsonl"])])
    try:
        completed = subprocess.run(
            command,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=int(document["timeouts_s"]["cell"]),
        )
        raw_log.write_text(completed.stdout, encoding="utf-8")
    except subprocess.TimeoutExpired as exc:
        output = (
            exc.stdout.decode() if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        )
        raw_log.write_text(output, encoding="utf-8")
        _write_json(environment_path, {"start": start, "end_error": "probe timeout"})
        fail("probe_failed", f"{fixture}/{treatment}-{ordinal}: probe timed out")
    try:
        end = environment_snapshot()
    except Exception as exc:
        _write_json(environment_path, {"start": start, "end_error": str(exc)})
        fail("environment_invalid", f"{fixture}/{treatment}-{ordinal}: {exc}")
    _write_json(environment_path, {"start": start, "end": end})
    finished_at = _timestamp()
    if completed.returncode != 0:
        fail(
            "probe_failed",
            f"{fixture}/{treatment}-{ordinal}: probe failed; see {_artifact_path(raw_log)}",
        )
    lines = [line for line in completed.stdout.splitlines() if line.startswith("{")]
    if len(lines) != 1:
        fail("probe_failed", f"{fixture}/{treatment}-{ordinal}: malformed probe output")
    try:
        probe_result = json.loads(lines[0])
    except json.JSONDecodeError as exc:
        fail("probe_failed", f"{fixture}/{treatment}-{ordinal}: invalid JSON: {exc}")
    if not isinstance(probe_result, dict):
        fail(
            "probe_failed",
            f"{fixture}/{treatment}-{ordinal}: probe JSON is not an object",
        )
    if probe_result.pop("schema_version", None) != "slice71b-write-probe.v1":
        fail("probe_failed", f"{fixture}/{treatment}-{ordinal}: probe schema drifted")
    if probe_result.pop("fixture", None) != fixture:
        fail(
            "probe_failed", f"{fixture}/{treatment}-{ordinal}: fixture identity drifted"
        )
    if probe_result.pop("treatment", None) != treatment:
        fail(
            "probe_failed",
            f"{fixture}/{treatment}-{ordinal}: treatment identity drifted",
        )
    sqlite_version = probe_result.pop("sqlite_version", "")
    sqlite_source_id = probe_result.pop("sqlite_source_id", "")
    if not _treatment_signature_valid(treatment, probe_result):
        fail(
            "probe_failed",
            f"{fixture}/{treatment}-{ordinal}: treatment signature failed; "
            f"see {_artifact_path(raw_log)}",
        )
    cell = {
        "fixture": fixture,
        "treatment": treatment,
        "ordinal": ordinal,
        "source_ref": source_ref,
        "source_tree_dirty": False,
        "probe_sha256": document["probe"]["sha256"],
        "probe_lock_sha256": document["probe"]["lock_sha256"],
        "runner_sha256": document["runner"]["sha256"],
        "executable_sha256": sha256(executable),
        "executable_path": _artifact_path(executable),
        "fixture_sha256": fixture_config["fixture_sha256"],
        "started_at": started_at,
        "finished_at": finished_at,
        "environment_valid": environment_valid(
            start, end, document["environment_policy"]
        ),
        "environment_sha256": sha256(environment_path),
        "environment_path": _artifact_path(environment_path),
        "raw_log_sha256": sha256(raw_log),
        "raw_log_path": _artifact_path(raw_log),
        "build_identity": build_identity,
        "runtime_identity": {
            "sqlite_version": sqlite_version,
            "sqlite_source_id": sqlite_source_id,
            "libsqlite3_sys": libsqlite3_sys,
        },
        "metrics": probe_result,
    }
    _write_json(cell_root / "cell.json", cell)
    if not cell["environment_valid"]:
        fail(
            "environment_invalid",
            f"{fixture}/{treatment}-{ordinal}: post-cell environment rejected; "
            f"see {_artifact_path(cell_root / 'cell.json')}",
        )
    return cell


def _arm_spread(cells: list[dict[str, Any]], fixture: str, treatment: str) -> float:
    arm = [
        cell
        for cell in cells
        if cell["fixture"] == fixture and cell["treatment"] == treatment
    ]
    if len(arm) < 3:
        return 0.0
    return max(
        (
            max(float(cell["metrics"][metric]) for cell in arm)
            / min(float(cell["metrics"][metric]) for cell in arm)
            - 1.0
        )
        * 100.0
        for metric in ("ingest_ack_ms", "total_ms")
    )


def _receipt(
    *,
    manifest_path: Path,
    started_at: str,
    cells: list[dict[str, Any]],
    state: str,
    failure: dict[str, Any] | None,
    errors: list[str],
) -> dict[str, Any]:
    classification = (
        attribution_classification(cells)
        if state == "complete"
        else {
            "state": state,
            "supported_causes": [],
            "conditional_preparation_factorial_required": False,
        }
    )
    return {
        "schema_version": "slice71b-attribution-receipt.v1",
        "manifest_sha256": sha256(manifest_path),
        "started_at": started_at,
        "finished_at": _timestamp(),
        "cells": cells,
        "classification": classification,
        "failure": failure,
        "errors": errors,
    }


def attribution_sequence(document: dict[str, Any]) -> list[tuple[str, str, int]]:
    """Return the one permitted fixture/treatment/ordinal sequence."""

    sequence = []
    ordinals = {
        fixture: {treatment: 0 for treatment in document["treatments"]}
        for fixture in ("scale02", "ac013")
    }
    for fixture in ("scale02", "ac013"):
        for treatment in document["attribution_order"]:
            ordinals[fixture][treatment] += 1
            sequence.append((fixture, treatment, ordinals[fixture][treatment]))
    return sequence


def _unexpected_failure_state(build_complete: bool) -> str:
    return "harness_failed" if build_complete else "build_failed"


def run_attribution(manifest_path: Path, source_root: Path) -> None:
    document = json.loads(manifest_path.read_text(encoding="utf-8"))
    validate_attribution_manifest(document)
    source_ref, dirty = _source_identity(source_root)
    if source_ref != document["product_sources"]["unchanged_current"] or dirty:
        raise RuntimeError(
            "campaign requires the sealed clean unchanged-current source"
        )
    output_root = ROOT / document["raw_root"] / "attribution"
    output_root.mkdir(parents=True, exist_ok=False)
    started_at = _timestamp()
    cells: list[dict[str, Any]] = []
    receipt_path = output_root / "attribution-receipt.json"
    state = "complete"
    failure: dict[str, Any] | None = None
    errors: list[str] = []
    build_complete = False
    active_cell: tuple[str, str, int, Path] | None = None
    build_root = output_root / "build"
    build_root.mkdir()
    build_log = build_root / "build.log"
    build_log.write_text("", encoding="utf-8")
    try:
        lock_path = ROOT / document["probe"]["lock_path"]
        target_root = Path(tempfile.mkdtemp(prefix="fathomdb-slice71b-target-"))
        executable, build_identity = _build_probe(
            source_root,
            target_root,
            lock_path,
            build_log,
            int(document["timeouts_s"]["build"]),
        )
        if not executable.is_file():
            message = "probe build did not produce the expected executable"
            _abort(
                "build_failed",
                message,
                _failure_disposition(
                    state="build_failed",
                    message=message,
                    root=build_root,
                    fixture=None,
                    treatment=None,
                    ordinal=None,
                    extra_artifacts=(("build_log", build_log),),
                ),
            )
        libsqlite3_sys = _libsqlite3_sys_version(lock_path)
        build_complete = True
        for fixture, treatment, ordinal in attribution_sequence(document):
            cell_root = output_root / fixture / f"{treatment}-{ordinal}"
            active_cell = (fixture, treatment, ordinal, cell_root)
            cell = _run_cell(
                document=document,
                source_ref=source_ref,
                executable=executable,
                build_identity=build_identity,
                libsqlite3_sys=libsqlite3_sys,
                fixture=fixture,
                treatment=treatment,
                ordinal=ordinal,
                cell_root=cell_root,
            )
            cells.append(cell)
            active_cell = None
            spread = _arm_spread(cells, fixture, treatment)
            if spread > float(document["policy"]["max_within_arm_spread_percent"]):
                message = (
                    f"{fixture}/{treatment}: within-arm spread "
                    f"{spread:.3f}% exceeded 25%"
                )
                _abort(
                    "spread_invalid",
                    message,
                    _failure_disposition(
                        state="spread_invalid",
                        message=message,
                        root=output_root / fixture / f"{treatment}-{ordinal}",
                        fixture=fixture,
                        treatment=treatment,
                        ordinal=ordinal,
                    ),
                )
    except CampaignAbort as exc:
        state = exc.state
        failure = exc.failure
        errors.append(str(exc))
    except (OSError, subprocess.SubprocessError, RuntimeError) as exc:
        state = _unexpected_failure_state(build_complete)
        errors.append(str(exc))
        if active_cell is None:
            failure_root = build_root
            failure_fixture = None
            failure_treatment = None
            failure_ordinal = None
        else:
            failure_fixture, failure_treatment, failure_ordinal, failure_root = (
                active_cell
            )
            failure_root.mkdir(parents=True, exist_ok=True)
        failure = _failure_disposition(
            state=state,
            message=str(exc),
            root=failure_root,
            fixture=failure_fixture,
            treatment=failure_treatment,
            ordinal=failure_ordinal,
            extra_artifacts=(("build_log", build_log),),
        )
    receipt = _receipt(
        manifest_path=manifest_path,
        started_at=started_at,
        cells=cells,
        state=state,
        failure=failure,
        errors=errors,
    )
    _write_json(receipt_path, receipt)
    validate_attribution_receipt(receipt, document)
    print(json.dumps(receipt, sort_keys=True))
    if state != "complete":
        raise RuntimeError(f"campaign stopped with {state}; retained {receipt_path}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    generate = subparsers.add_parser("generate-scale02-input")
    generate.add_argument("--config", type=Path, default=DEFAULT_SCALE02_CONFIG)
    generate.add_argument("--output", type=Path, required=True)
    campaign = subparsers.add_parser("run-attribution")
    campaign.add_argument("--source-root", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command == "generate-scale02-input":
        generate_scale02_input(args.config, args.output)
    else:
        run_attribution(DEFAULT_MANIFEST, args.source_root.resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
