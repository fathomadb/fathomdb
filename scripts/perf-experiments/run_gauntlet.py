#!/usr/bin/env python3
"""Coordinate the existing Performance Gauntlet v1.2 runners."""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import time
from collections.abc import Sequence
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


PLAN_SCHEMA = "fathomdb.performance-gauntlet.plan/v1"
CONFIG_SCHEMA = "fathomdb.performance-gauntlet.config/v1"
RESOLVED_CONFIG_SCHEMA = "fathomdb.performance-gauntlet.resolved-config/v1"
DEFAULT_CELLS = (
    "ac076",
    "ac072",
    "ac081",
    "ac073",
    "ac075",
    "scale02",
    "protected-writes",
    "ce-profile",
    "search01",
    "locomo",
)
OPTIONAL_CELLS = ("ac013-scale-matrix",)
GRAPH_CELLS = (
    "graph-evidence01",
    "graph-expand01",
    "graph-retrieval01",
)
ALL_CELLS = DEFAULT_CELLS + OPTIONAL_CELLS + GRAPH_CELLS
TOP_LEVEL_KEYS = (
    "schema_version",
    "release",
    "source",
    "output_root",
    "runtime",
    "configs",
    "assets",
    "gpu",
    "cells",
    "timeouts_s",
)
RUNTIME_KEYS = (
    "fathomdb_cli",
    "python",
    "wheel",
    "native_extension",
    "virtualenv",
)
BASE_CONFIG_KEYS = (
    "ac072",
    "ac073",
    "ac075",
    "scale02",
    "protected_writes",
    "ce_profile",
    "search01",
    "locomo",
)
GRAPH_CONFIG_KEYS = ("graph_evidence", "graph_expand", "graph_retrieval")
CONFIG_KEYS = BASE_CONFIG_KEYS + GRAPH_CONFIG_KEYS
BASE_ASSET_KEYS = ("ir_c", "ac073", "tc5", "locomo", "ce_profile")
GRAPH_ASSET_KEYS = ("graph_output", "musique", "stark")
ASSET_KEYS = BASE_ASSET_KEYS + GRAPH_ASSET_KEYS
ASSET_FIELDS = {
    "ir_c": ("data_root", "snapshot", "manifest", "gold"),
    "ac073": ("corpus_root",),
    "tc5": ("corpus_root", "qualified_manifest", "embedder_model_cache"),
    "locomo": (
        "harness_checkout",
        "harness_python",
        "dataset",
        "provenance_manifest",
        "external_output_root",
    ),
    "ce_profile": ("reranker_model_cache",),
    "graph_output": ("external_output_root",),
    "musique": ("dataset", "extractions", "cohort", "seed_manifest"),
    "stark": ("checkout", "dataset_root", "manifest", "evaluator"),
}
DIRECTORY_RUNTIME_FIELDS = frozenset({"virtualenv"})
DIRECTORY_ASSET_FIELDS = frozenset(
    {
        ("ir_c", "data_root"),
        ("ac073", "corpus_root"),
        ("tc5", "corpus_root"),
        ("tc5", "embedder_model_cache"),
        ("locomo", "harness_checkout"),
        ("locomo", "external_output_root"),
        ("ce_profile", "reranker_model_cache"),
        ("graph_output", "external_output_root"),
        ("stark", "checkout"),
        ("stark", "dataset_root"),
    }
)
RELEASE_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
CUDA_UUID_RE = re.compile(
    r"^GPU-[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-"
    r"[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
)


class GauntletArgumentError(ValueError):
    """Report a caller-supplied gauntlet argument that is not valid."""


class GauntletConfigError(ValueError):
    """Report a gauntlet configuration contract violation."""


def _object(value: object, location: str) -> dict[str, Any]:
    """Return a JSON object or raise a path-specific configuration error."""
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        raise GauntletConfigError(f"{location} must be an object")
    return value


def _exact_keys(
    document: dict[str, Any], expected: Sequence[str], location: str
) -> None:
    """Require a JSON object to contain exactly the named keys."""
    expected_set = set(expected)
    missing = expected_set.difference(document)
    unknown = set(document).difference(expected_set)
    if missing:
        raise GauntletConfigError(
            f"{location} has missing fields: {', '.join(sorted(missing))}"
        )
    if unknown:
        raise GauntletConfigError(
            f"{location} has unknown fields: {', '.join(sorted(unknown))}"
        )


def _absolute_path(value: object, location: str) -> str:
    """Validate and return one absolute path string."""
    if not isinstance(value, str) or not value or not Path(value).is_absolute():
        raise GauntletConfigError(f"{location} must be an absolute path")
    return value


def _nullable_absolute_path(value: object, location: str) -> str | None:
    """Validate an optional absolute path string."""
    if value is None:
        return None
    return _absolute_path(value, location)


def _positive_integer(value: object, location: str) -> int:
    """Validate one strictly positive, non-boolean integer."""
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise GauntletConfigError(f"{location} must be a positive integer")
    return value


def _validate_cells(value: object) -> tuple[str, ...]:
    """Validate the configured canonical non-empty cell subsequence."""
    if (
        not isinstance(value, list)
        or not value
        or any(not isinstance(cell, str) for cell in value)
    ):
        raise GauntletConfigError("cells must be a non-empty list of cell identifiers")
    cells = tuple(value)
    if len(set(cells)) != len(cells):
        raise GauntletConfigError("cells must not contain duplicates")
    if set(cells).difference(ALL_CELLS):
        raise GauntletConfigError("cells contains an unknown cell identifier")
    canonical = tuple(cell for cell in ALL_CELLS if cell in cells)
    if cells != canonical:
        raise GauntletConfigError("cells must be in canonical gauntlet order")
    return cells


def validate_configuration(document: object) -> None:
    """Validate the closed Performance Gauntlet v1.2 input schema."""
    root = _object(document, "configuration")
    _exact_keys(root, TOP_LEVEL_KEYS, "configuration")
    if root["schema_version"] != CONFIG_SCHEMA:
        raise GauntletConfigError(f"schema_version must equal {CONFIG_SCHEMA}")
    if not isinstance(root["release"], str) or not RELEASE_RE.fullmatch(
        root["release"]
    ):
        raise GauntletConfigError("release must be a semantic version such as 0.8.26")

    source = _object(root["source"], "source")
    _exact_keys(source, ("root", "commit"), "source")
    _absolute_path(source["root"], "source.root")
    if not isinstance(source["commit"], str) or not COMMIT_RE.fullmatch(
        source["commit"]
    ):
        raise GauntletConfigError(
            "source.commit must be 40 lowercase hexadecimal characters"
        )
    _absolute_path(root["output_root"], "output_root")

    runtime = _object(root["runtime"], "runtime")
    _exact_keys(runtime, RUNTIME_KEYS, "runtime")
    for key in RUNTIME_KEYS:
        _nullable_absolute_path(runtime[key], f"runtime.{key}")

    configs = _object(root["configs"], "configs")
    missing_configs = set(BASE_CONFIG_KEYS).difference(configs)
    unknown_configs = set(configs).difference(CONFIG_KEYS)
    if missing_configs or unknown_configs:
        raise GauntletConfigError(
            "configs has invalid fields: missing="
            + ",".join(sorted(missing_configs))
            + " unknown="
            + ",".join(sorted(unknown_configs))
        )
    for key in CONFIG_KEYS:
        _nullable_absolute_path(configs.get(key), f"configs.{key}")

    assets = _object(root["assets"], "assets")
    missing_assets = set(BASE_ASSET_KEYS).difference(assets)
    unknown_assets = set(assets).difference(ASSET_KEYS)
    if missing_assets or unknown_assets:
        raise GauntletConfigError(
            "assets has invalid fields: missing="
            + ",".join(sorted(missing_assets))
            + " unknown="
            + ",".join(sorted(unknown_assets))
        )
    for group_name, fields in ASSET_FIELDS.items():
        group_value = assets.get(group_name)
        if group_value is None:
            continue
        group = _object(group_value, f"assets.{group_name}")
        _exact_keys(group, fields, f"assets.{group_name}")
        for field in fields:
            _absolute_path(group[field], f"assets.{group_name}.{field}")

    gpu = _object(root["gpu"], "gpu")
    _exact_keys(gpu, ("enabled", "cuda_uuid"), "gpu")
    if not isinstance(gpu["enabled"], bool):
        raise GauntletConfigError("gpu.enabled must be a boolean")
    cuda_uuid = gpu["cuda_uuid"]
    if gpu["enabled"]:
        if not isinstance(cuda_uuid, str) or not CUDA_UUID_RE.fullmatch(cuda_uuid):
            raise GauntletConfigError(
                "gpu.cuda_uuid must be a full GPU UUID when enabled"
            )
    elif cuda_uuid is not None:
        raise GauntletConfigError("gpu.cuda_uuid must be null when GPU is disabled")

    _validate_cells(root["cells"])
    timeouts = _object(root["timeouts_s"], "timeouts_s")
    _exact_keys(timeouts, ("default", "cells"), "timeouts_s")
    _positive_integer(timeouts["default"], "timeouts_s.default")
    cell_timeouts = _object(timeouts["cells"], "timeouts_s.cells")
    unknown_timeout_cells = set(cell_timeouts).difference(ALL_CELLS)
    if unknown_timeout_cells:
        raise GauntletConfigError(
            "timeouts_s.cells has unknown fields: "
            + ", ".join(sorted(unknown_timeout_cells))
        )
    for cell, timeout in cell_timeouts.items():
        _positive_integer(timeout, f"timeouts_s.cells.{cell}")


def _unique_json_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    """Reject duplicate JSON object keys instead of silently taking the last."""
    document: dict[str, object] = {}
    for key, value in pairs:
        if key in document:
            raise GauntletConfigError(f"duplicate JSON key: {key}")
        document[key] = value
    return document


def _read_configuration_snapshot(
    path: str | Path,
) -> tuple[dict[str, Any], dict[str, str]]:
    """Read, hash, parse, and validate one immutable byte snapshot."""
    requested_path = Path(path)
    try:
        config_path = requested_path.resolve(strict=True)
        raw = config_path.read_bytes()
        document = json.loads(
            raw.decode("utf-8"), object_pairs_hook=_unique_json_object
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise GauntletConfigError(
            f"cannot load configuration {requested_path}: {error}"
        ) from error
    validate_configuration(document)
    return _object(document, "configuration"), {
        "path": str(config_path),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def load_configuration(path: str | Path) -> dict[str, Any]:
    """Load and strictly validate one gauntlet configuration file."""
    document, _ = _read_configuration_snapshot(path)
    return document


def _sha256(path: Path) -> str:
    """Return the SHA-256 identity of one regular file."""
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise GauntletConfigError(f"cannot read input file {path}: {error}") from error
    return digest.hexdigest()


def _file_identity(value: str, location: str) -> dict[str, str]:
    """Resolve and identify a required regular-file binding."""
    try:
        path = Path(value).resolve(strict=True)
    except OSError as error:
        raise GauntletConfigError(f"{location} is missing: {value}") from error
    if not path.is_file():
        raise GauntletConfigError(f"{location} must be a regular file: {path}")
    return {"path": str(path), "sha256": _sha256(path)}


def _launcher_identity(value: str, location: str) -> dict[str, str]:
    """Identify a venv launcher without resolving it to the system interpreter."""
    path = Path(value).absolute()
    if not path.is_file():
        raise GauntletConfigError(f"{location} must be a regular file: {path}")
    return {"path": str(path), "sha256": _sha256(path)}


def _directory_identity(value: str, location: str) -> dict[str, str]:
    """Resolve a required directory binding without inventing a tree hash."""
    try:
        path = Path(value).resolve(strict=True)
    except OSError as error:
        raise GauntletConfigError(f"{location} is missing: {value}") from error
    if not path.is_dir():
        raise GauntletConfigError(f"{location} must be a directory: {path}")
    return {"path": str(path)}


def _resolve_optional_binding(
    value: str | None, location: str, *, directory: bool
) -> dict[str, str] | None:
    """Resolve one nullable file or directory binding."""
    if value is None:
        return None
    if directory:
        return _directory_identity(value, location)
    return _file_identity(value, location)


def _is_relative_to(path: Path, parent: Path) -> bool:
    """Return whether path is equal to or nested beneath parent."""
    try:
        path.relative_to(parent)
    except ValueError:
        return False
    return True


def _ensure_safe_output(
    output_root: Path,
    input_directories: Sequence[Path],
    input_files: Sequence[Path],
) -> None:
    """Reject output paths that could overwrite or pollute named inputs."""
    for directory in input_directories:
        if _is_relative_to(output_root, directory):
            raise GauntletConfigError(
                f"output_root must be outside input directory {directory}"
            )
    if output_root in input_files:
        raise GauntletConfigError(
            f"output_root must not equal input file {output_root}"
        )


def resolve_configuration(
    config_path: str | Path, cli_selected: tuple[str, ...] | None = None
) -> dict[str, Any]:
    """Resolve one path-owned config snapshot without creating output."""
    root, config_identity = _read_configuration_snapshot(config_path)
    source = _object(root["source"], "source")
    runtime = _object(root["runtime"], "runtime")
    configs = _object(root["configs"], "configs")
    assets = _object(root["assets"], "assets")

    source_identity = _directory_identity(source["root"], "source.root")
    resolved_runtime = {}
    for key in RUNTIME_KEYS:
        if key == "python" and runtime[key] is not None:
            resolved_runtime[key] = _launcher_identity(runtime[key], "runtime.python")
        else:
            resolved_runtime[key] = _resolve_optional_binding(
                runtime[key],
                f"runtime.{key}",
                directory=key in DIRECTORY_RUNTIME_FIELDS,
            )
    resolved_configs = {
        key: _resolve_optional_binding(
            configs.get(key), f"configs.{key}", directory=False
        )
        for key in CONFIG_KEYS
    }
    resolved_assets: dict[str, Any] = {}
    for group_name, fields in ASSET_FIELDS.items():
        group_value = assets.get(group_name)
        if group_value is None:
            resolved_assets[group_name] = None
            continue
        group = _object(group_value, f"assets.{group_name}")
        resolved_group = {}
        for field in fields:
            location = f"assets.{group_name}.{field}"
            if group_name == "locomo" and field == "harness_python":
                resolved_group[field] = _launcher_identity(group[field], location)
            else:
                resolved_group[field] = _resolve_optional_binding(
                    group[field],
                    location,
                    directory=(group_name, field) in DIRECTORY_ASSET_FIELDS,
                )
        resolved_assets[group_name] = resolved_group

    input_directories = [Path(source_identity["path"])]
    input_files = [Path(config_identity["path"])]
    for identity in resolved_runtime.values():
        if identity is None:
            continue
        target = Path(identity["path"])
        (input_files if "sha256" in identity else input_directories).append(target)
    for identity in resolved_configs.values():
        if identity is not None:
            input_files.append(Path(identity["path"]))
    for group in resolved_assets.values():
        if group is None:
            continue
        for identity in group.values():
            target = Path(identity["path"])
            (input_files if "sha256" in identity else input_directories).append(target)

    output_root = Path(root["output_root"]).resolve(strict=False)
    _ensure_safe_output(output_root, input_directories, input_files)
    cells = select_configured_cells(root["cells"], cli_selected)
    timeouts = _object(root["timeouts_s"], "timeouts_s")
    overrides = _object(timeouts["cells"], "timeouts_s.cells")
    effective_timeouts = {
        cell: overrides.get(cell, timeouts["default"]) for cell in cells
    }
    return {
        "schema_version": RESOLVED_CONFIG_SCHEMA,
        "release": root["release"],
        "source": {"root": source_identity["path"], "commit": source["commit"]},
        "source_config": config_identity,
        "output_root": str(output_root),
        "runtime": resolved_runtime,
        "configs": resolved_configs,
        "assets": resolved_assets,
        "gpu": root["gpu"],
        "cells": list(cells),
        "timeouts_s": effective_timeouts,
    }


def write_resolved_configuration(
    config_path: str | Path, cli_selected: tuple[str, ...] | None = None
) -> Path:
    """Atomically write a resolved snapshot beneath one new output root."""
    resolved = resolve_configuration(config_path, cli_selected)
    output_root = Path(resolved["output_root"])
    output_path = output_root / "gauntlet-plan.resolved.json"
    temporary_path = output_root / ".gauntlet-plan.resolved.json.tmp"
    created_directories: list[Path] = []
    candidate = output_root
    while not candidate.exists():
        created_directories.append(candidate)
        if candidate.parent == candidate:
            break
        candidate = candidate.parent
    created_root = False
    completed = False
    try:
        output_root.mkdir(parents=True, exist_ok=False)
        created_root = True
        with temporary_path.open("x", encoding="utf-8") as handle:
            json.dump(resolved, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_path, output_path)
        completed = True
    except FileExistsError as error:
        raise GauntletConfigError(
            f"output_root already exists: {output_root}"
        ) from error
    except OSError as error:
        raise GauntletConfigError(
            f"cannot write resolved configuration: {error}"
        ) from error
    finally:
        if created_root and not completed:
            temporary_path.unlink(missing_ok=True)
            for directory in created_directories:
                try:
                    directory.rmdir()
                except OSError:
                    break
    return output_path


def assert_cli_configuration(
    document: object, *, release: str, source_root: str, output_root: str
) -> None:
    """Require later CLI execution assertions to match loaded configuration."""
    validate_configuration(document)
    root = _object(document, "configuration")
    source = _object(root["source"], "source")
    if release != root["release"]:
        raise GauntletConfigError("CLI release does not match configuration release")
    if Path(source_root).resolve(strict=False) != Path(source["root"]).resolve(
        strict=False
    ):
        raise GauntletConfigError(
            "CLI source_root does not match configuration source.root"
        )
    if Path(output_root).resolve(strict=False) != Path(root["output_root"]).resolve(
        strict=False
    ):
        raise GauntletConfigError(
            "CLI output_root does not match configuration output_root"
        )


def select_configured_cells(
    configured: object, cli_selected: tuple[str, ...] | None
) -> tuple[str, ...]:
    """Select configured cells while forbidding CLI expansion of the run."""
    configured_cells = _validate_cells(configured)
    if cli_selected is None:
        return configured_cells
    unavailable = set(cli_selected).difference(configured_cells)
    if unavailable:
        raise GauntletConfigError(
            "CLI cells are not enabled by configuration: "
            + ", ".join(sorted(unavailable))
        )
    canonical = tuple(cell for cell in configured_cells if cell in cli_selected)
    if tuple(cli_selected) != canonical:
        raise GauntletConfigError("CLI cells must retain canonical configured order")
    return canonical


def parse_cell_selection(value: str | None) -> tuple[str, ...]:
    """Return a validated cell selection in canonical gauntlet order."""
    if value is None:
        return DEFAULT_CELLS

    requested = [part.strip() for part in value.split(",")]
    if not requested or any(not cell for cell in requested):
        raise GauntletArgumentError("--cells must not contain empty cell names")

    duplicates = {cell for cell in requested if requested.count(cell) > 1}
    if duplicates:
        names = ", ".join(sorted(duplicates))
        raise GauntletArgumentError(f"--cells contains duplicate cell names: {names}")

    unknown = set(requested).difference(ALL_CELLS)
    if unknown:
        names = ", ".join(sorted(unknown))
        raise GauntletArgumentError(f"--cells contains unknown cell names: {names}")

    selected = set(requested)
    return tuple(cell for cell in ALL_CELLS if cell in selected)


def parse_suite_selection(suite: str | None, cells: str | None) -> tuple[str, ...]:
    """Resolve one named suite or an explicit cell selection."""
    if suite is not None and cells is not None:
        raise GauntletArgumentError("--suite and --cells are mutually exclusive")
    if suite is None:
        return parse_cell_selection(cells)
    if suite == "graph":
        return GRAPH_CELLS
    raise GauntletArgumentError(f"unknown suite: {suite}")


def build_parser() -> argparse.ArgumentParser:
    """Build the public command-line parser without touching external inputs."""
    parser = argparse.ArgumentParser(
        description="Run the Performance Gauntlet v1.2 in canonical cell order."
    )
    parser.add_argument("--release", required=True, help="FathomDB release under test")
    parser.add_argument(
        "--source-root", required=True, help="Source tree for the release under test"
    )
    parser.add_argument("--config", required=True, help="Gauntlet configuration file")
    parser.add_argument(
        "--output-root", required=True, help="Directory that will receive run output"
    )
    parser.add_argument(
        "--cells",
        help="Comma-separated subset of cells; execution keeps canonical order",
    )
    parser.add_argument(
        "--suite",
        choices=("graph",),
        help="Run one optional named suite",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the requested run without reading inputs or creating output",
    )
    parser.add_argument(
        "--preflight-only",
        action="store_true",
        help="Validate prerequisites without running cells (available in Slice 50)",
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="Resume eligible cells from an existing output directory",
    )
    parser.add_argument(
        "--fail-fast",
        action="store_true",
        help="Stop after the first cell that does not pass",
    )
    return parser


def _dry_run_projection(
    args: argparse.Namespace, cells: tuple[str, ...]
) -> dict[str, object]:
    """Build the stable metadata projection emitted by dry-run mode."""
    return {
        "cells": list(cells),
        "config": args.config,
        "fail_fast": args.fail_fast,
        "mode": "dry-run",
        "output_root": args.output_root,
        "release": args.release,
        "resume": args.resume,
        "schema_version": PLAN_SCHEMA,
        "source_root": args.source_root,
    }


def _load_cells_module():
    """Load the sibling cell planner without making this directory a package."""
    path = Path(__file__).with_name("gauntlet_cells.py")
    spec = importlib.util.spec_from_file_location("fathomdb_gauntlet_cells", path)
    if spec is None or spec.loader is None:
        raise GauntletConfigError("cannot load gauntlet cell planner")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _run_checked(
    command: list[str],
    cwd: Path,
    log: Path,
    timeout: int,
    *,
    environment: dict[str, str] | None = None,
) -> None:
    """Run one prerequisite command and retain its complete combined output."""
    log.parent.mkdir(parents=True, exist_ok=True)
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        log.write_text((error.stdout or "") + "\nTIMEOUT\n", encoding="utf-8")
        raise GauntletConfigError(f"prerequisite timed out; see {log}") from error
    log.write_text(result.stdout, encoding="utf-8")
    if result.returncode:
        raise GauntletConfigError(f"prerequisite failed; see {log}")


def _cuda_environment() -> dict[str, str]:
    """Return the pinned CUDA toolkit environment; never permit CPU fallback."""
    cuda_root = Path("/usr/local/cuda")
    nvcc = cuda_root / "bin" / "nvcc"
    library = cuda_root / "lib64"
    if not nvcc.is_file() or not library.is_dir():
        raise GauntletConfigError("CUDA toolkit is unavailable; refusing CPU fallback")
    environment = os.environ.copy()
    environment.update(
        {
            "CUDA_HOME": str(cuda_root),
            "CUDA_PATH": str(cuda_root),
            "LIBRARY_PATH": str(library),
            "PATH": f"{cuda_root / 'bin'}:{environment.get('PATH', '')}",
        }
    )
    return environment


def _git_output(source: Path, *arguments: str) -> str:
    """Return one successful git query for the target checkout."""
    result = subprocess.run(
        ["git", *arguments],
        cwd=source,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode:
        raise GauntletConfigError(result.stderr.strip() or "git query failed")
    return result.stdout.strip()


def _prepare_artifacts(
    resolved: dict[str, Any], plans: list[dict[str, object]], repo_root: Path
) -> dict[str, str]:
    """Build shared test artifacts once and return placeholder substitutions."""
    source = Path(resolved["source"]["root"])
    output = Path(resolved["output_root"])
    build_logs = output / "preflight"
    selected = {str(plan["cell"]) for plan in plans}
    replacements: dict[str, str] = {}
    if selected.intersection({"ac072", "ac081"}):
        _run_checked(
            [
                "cargo",
                "test",
                "--release",
                "--no-run",
                "-p",
                "fathomdb-engine",
                "--test",
                "perf_gates",
            ],
            source,
            build_logs / "perf-gates-build.log",
            3600,
        )
        candidates = [
            path
            for path in (source / "target" / "release" / "deps").glob("perf_gates-*")
            if path.is_file() and os.access(path, os.X_OK) and path.suffix != ".d"
        ]
        if not candidates:
            raise GauntletConfigError("perf_gates executable was not produced")
        binary = max(candidates, key=lambda path: path.stat().st_mtime_ns).resolve()
        paths = [
            "Cargo.toml",
            "Cargo.lock",
            ".cargo/config.toml",
            "src/rust/crates/fathomdb-engine/Cargo.toml",
            "src/rust/crates/fathomdb-engine/src",
            "src/rust/crates/fathomdb-engine/tests/perf_gates.rs",
            "src/rust/crates/fathomdb-engine/tests/reader_pool.rs",
            "src/rust/crates/fathomdb-query",
            "src/rust/crates/fathomdb-schema",
            "src/rust/crates/fathomdb-embedder",
            "src/rust/crates/fathomdb-embedder-api",
        ]
        tree = subprocess.run(
            ["git", "ls-tree", "-r", resolved["source"]["commit"], "--", *paths],
            cwd=source,
            check=True,
            stdout=subprocess.PIPE,
        ).stdout
        replacements.update(
            {
                "artifact:perf_gates.path": str(binary),
                "artifact:perf_gates.sha256": _sha256(binary),
                "artifact:perf_gates.input_sha256": hashlib.sha256(tree).hexdigest(),
            }
        )
    if "ac075" in selected:
        binary = (source / "target" / "release" / "fathomdb-tc5-benchmark").resolve()
        _run_checked(
            [
                "cargo",
                "build",
                "--release",
                "-p",
                "fathomdb-tc5-benchmark",
                "--features",
                "tc5-benchmark-cuda",
            ],
            source,
            build_logs / "tc5-build.log",
            3600,
            environment=_cuda_environment(),
        )
        if not binary.is_file():
            raise GauntletConfigError("TC-5 benchmark executable was not produced")
        replacements.update(
            {
                "artifact:tc5_benchmark.path": str(binary),
                "artifact:tc5_benchmark.sha256": _sha256(binary),
            }
        )
    del repo_root
    return replacements


def _verify_required_gpu(
    resolved: dict[str, Any], plans: list[dict[str, object]]
) -> None:
    """Fail closed before any cell when a GPU-bound workload cannot use its pin."""
    gpu_cells = {"ac073", "ac075", "ce-profile"}
    selected = {str(plan["cell"]) for plan in plans}
    if not selected.intersection(gpu_cells):
        return
    _cuda_environment()
    gpu = _object(resolved.get("gpu"), "gpu")
    expected = gpu.get("cuda_uuid")
    if gpu.get("enabled") is not True or not isinstance(expected, str):
        raise GauntletConfigError(
            "encoding/GPU cells require gpu.enabled=true and a pinned CUDA UUID"
        )
    result = subprocess.run(
        ["nvidia-smi", "--query-gpu=uuid,name", "--format=csv,noheader"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    visible = {}
    for line in result.stdout.splitlines():
        uuid, separator, name = line.partition(",")
        if separator:
            visible[uuid.strip()] = name.strip()
    if result.returncode or expected not in visible:
        raise GauntletConfigError(
            f"pinned CUDA device is unavailable; refusing CPU fallback: {expected}"
        )
    if "RTX 3090" not in visible[expected]:
        raise GauntletConfigError(
            f"pinned CUDA device must be an RTX 3090: {visible[expected]}"
        )


def _replace(value: object, replacements: dict[str, str]) -> object:
    """Recursively replace exact artifact placeholders."""
    if isinstance(value, str):
        return replacements.get(value, value)
    if isinstance(value, list):
        return [_replace(item, replacements) for item in value]
    if isinstance(value, dict):
        return {key: _replace(item, replacements) for key, item in value.items()}
    return value


def _canonical_json(value: object) -> bytes:
    """Serialize one identity-bearing object deterministically."""
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def _plan_identity(plan: object, resolved: dict[str, Any]) -> str:
    """Bind a cell plan to the target release, source, and input config."""
    identity = {
        "plan": plan,
        "release": resolved["release"],
        "source_commit": resolved["source"]["commit"],
        "source_config_sha256": resolved["source_config"]["sha256"],
    }
    return hashlib.sha256(_canonical_json(identity)).hexdigest()


def _resume_result_matches(
    result: object, plan: object, resolved: dict[str, Any]
) -> bool:
    """Return whether a passed result is safe to reuse for this exact plan."""
    if not isinstance(result, dict) or result.get("state") != "passed":
        return False
    identities_match = (
        result.get("release") == resolved["release"]
        and result.get("source_commit") == resolved["source"]["commit"]
        and result.get("source_config_sha256") == resolved["source_config"]["sha256"]
        and result.get("plan_sha256") == _plan_identity(plan, resolved)
    )
    if not identities_match or result.get("runner") != _runner_identity(plan):
        return False
    artifacts = result.get("artifacts")
    if not isinstance(artifacts, list):
        return False
    for artifact in artifacts:
        if not isinstance(artifact, dict):
            return False
        path = artifact.get("path")
        expected = artifact.get("sha256")
        if not isinstance(path, str) or not isinstance(expected, str):
            return False
        candidate = Path(path)
        if not candidate.is_file() or _sha256(candidate) != expected:
            return False
    return True


def _now() -> str:
    """Return an unambiguous UTC timestamp for result records."""
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def _result_base(cell: str, resolved: dict[str, Any]) -> dict[str, object]:
    """Return target identity fields shared by every typed cell outcome."""
    return {
        "schema_version": "fathomdb.performance-gauntlet.cell-result/v1",
        "cell": cell,
        "release": resolved["release"],
        "source_commit": resolved["source"]["commit"],
        "source_config_sha256": resolved["source_config"]["sha256"],
    }


def _unavailable_result(
    cell: str, reason: str, resolved: dict[str, Any], *, phase: str
) -> dict[str, object]:
    """Build one terminal, typed missing-prerequisite result."""
    return {
        **_result_base(cell, resolved),
        "state": "unavailable",
        "phase": phase,
        "reason": reason,
        "finished_at": _now(),
        "artifacts": [],
    }


def _not_run_result(
    cell: str, reason: str, resolved: dict[str, Any]
) -> dict[str, object]:
    """Build one terminal result for a cell intentionally not started."""
    return {
        **_result_base(cell, resolved),
        "state": "not-run",
        "phase": "execution",
        "reason": reason,
        "finished_at": _now(),
        "artifacts": [],
    }


def _ready_result(plan: dict[str, Any], resolved: dict[str, Any]) -> dict[str, object]:
    """Build one successful preflight result without executing the workload."""
    return {
        **_result_base(str(plan["cell"]), resolved),
        "state": "ready",
        "phase": "preflight",
        "plan_sha256": _plan_identity(plan, resolved),
        "runner": _runner_identity(plan),
        "finished_at": _now(),
        "artifacts": [],
    }


def _write_json_atomic(path: Path, document: object) -> None:
    """Write one JSON document atomically within its destination directory."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp")
    try:
        with temporary.open("w", encoding="utf-8") as handle:
            json.dump(document, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def _cell_result_path(output_root: Path, cell: str) -> Path:
    """Return the canonical typed-result path for one cell."""
    return output_root / "cells" / cell / "cell-result.json"


def _artifact_manifest(
    output_root: Path, cell: str, plan: dict[str, Any] | None = None
) -> list[dict[str, object]]:
    """Identify retained files produced beneath one cell directory."""
    cell_root = output_root / "cells" / cell
    roots = [cell_root]
    if plan is not None:
        for declared in _planned_output_paths(plan):
            if declared.exists() and not _is_relative_to(declared, cell_root):
                roots.append(declared)
    artifacts = []
    seen: set[Path] = set()
    candidates: list[Path] = []
    for root in roots:
        if root.is_file():
            candidates.append(root)
        elif root.is_dir():
            candidates.extend(sorted(root.rglob("*")))
    for path in candidates:
        if not path.is_file() or path.name == "cell-result.json":
            continue
        canonical = path.resolve()
        if canonical in seen:
            continue
        seen.add(canonical)
        relative = (
            str(path.relative_to(output_root))
            if _is_relative_to(path, output_root)
            else None
        )
        artifacts.append(
            {
                "path": str(path),
                "relative_path": relative,
                "sha256": _sha256(path),
                "size_bytes": path.stat().st_size,
            }
        )
    return artifacts


def _runner_identity(plan: dict[str, Any]) -> dict[str, str]:
    """Identify the concrete file backing a runner or manifest fragment."""
    declared = str(plan["adapter"])
    backing = Path(declared.split("#", 1)[0])
    return {
        "path": declared,
        "backing_path": str(backing),
        "sha256": _sha256(backing),
    }


def _finalize_executed_result(
    raw: dict[str, object], plan: dict[str, Any], resolved: dict[str, Any]
) -> dict[str, object]:
    """Add required target, runner, timing, and artifact identity to a result."""
    finished_unix = time.time()
    invocations = raw.get("invocations", [])
    exit_codes = [
        item.get("returncode")
        for item in invocations
        if isinstance(item, dict) and isinstance(item.get("returncode"), int)
    ]
    exit_code = next((code for code in exit_codes if code), 0)
    return {
        **_result_base(str(plan["cell"]), resolved),
        "state": raw["state"],
        "phase": "execution",
        "reason": raw.get("completion_error"),
        "plan_sha256": _plan_identity(plan, resolved),
        "runner": _runner_identity(plan),
        "exit_code": exit_code,
        "started_unix": raw["started_unix"],
        "finished_unix": finished_unix,
        "started_at": datetime.fromtimestamp(float(raw["started_unix"]), UTC)
        .isoformat()
        .replace("+00:00", "Z"),
        "finished_at": datetime.fromtimestamp(finished_unix, UTC)
        .isoformat()
        .replace("+00:00", "Z"),
        "elapsed_s": raw["elapsed_s"],
        "invocations": invocations,
        "artifacts": _artifact_manifest(
            Path(resolved["output_root"]), str(plan["cell"]), plan
        ),
    }


def _overall_state(results: Sequence[dict[str, object]], mode: str) -> str:
    """Reduce cell states to one campaign state without hiding incompleteness."""
    states = {str(item.get("state")) for item in results}
    if mode == "preflight" and states == {"ready"}:
        return "ready"
    if mode == "run" and states == {"passed"}:
        return "passed"
    if "failed" in states:
        return "failed"
    return "incomplete"


def _render_summary_markdown(summary: dict[str, object]) -> str:
    """Render the human view solely from the machine-readable summary."""
    lines = [
        f"# Performance Gauntlet v1.2 — FathomDB {summary['release']}",
        "",
        f"- State: `{summary['state']}`",
        "- Comparison: `directional`",
        f"- Source commit: `{summary['source']['commit']}`",
        "",
        "| Cell | State | Detail |",
        "| --- | --- | --- |",
    ]
    for item in summary["cells"]:
        reason = str(item.get("reason") or "")
        reason = reason.replace("|", "\\|").replace("\n", " ")
        lines.append(f"| {item['cell']} | {item['state']} | {reason} |")
    lines.extend(
        [
            "",
            "Results are informative and directional; they are not a release gate.",
            "",
        ]
    )
    return "\n".join(lines)


def _write_campaign_documents(
    resolved: dict[str, Any],
    results: Sequence[dict[str, object]],
    *,
    mode: str,
) -> dict[str, Path]:
    """Write status, JSON summary, and the JSON-derived Markdown view."""
    output_root = Path(resolved["output_root"])
    ordered = list(results)
    expected_cells = [str(cell) for cell in resolved["cells"]]
    actual_cells = [str(item.get("cell")) for item in ordered]
    if actual_cells != expected_cells:
        raise GauntletConfigError(
            "campaign summary must cover every requested cell in canonical order"
        )
    summary = {
        "schema_version": "fathomdb.performance-gauntlet.summary/v1",
        "release": resolved["release"],
        "source": resolved["source"],
        "source_config": resolved["source_config"],
        "mode": mode,
        "comparison": "directional",
        "state": _overall_state(ordered, mode),
        "cells": ordered,
    }
    status = {
        "schema_version": "fathomdb.performance-gauntlet.status/v1",
        "release": resolved["release"],
        "source_commit": resolved["source"]["commit"],
        "mode": mode,
        "state": summary["state"],
        "updated_at": _now(),
        "cells": {
            str(item["cell"]): str(item["state"])
            for item in sorted(ordered, key=lambda value: str(value["cell"]))
        },
    }
    prefix = "gauntlet-preflight" if mode == "preflight" else "gauntlet"
    paths = {
        "status": output_root / f"{prefix}-status.json",
        "summary_json": output_root / f"{prefix}-summary.json",
        "summary_markdown": output_root / f"{prefix}-summary.md",
    }
    _write_json_atomic(paths["status"], status)
    _write_json_atomic(paths["summary_json"], summary)
    paths["summary_markdown"].write_text(
        _render_summary_markdown(summary), encoding="utf-8"
    )
    if mode == "run":
        compatibility = output_root / "gauntlet-result.json"
        _write_json_atomic(compatibility, summary)
        paths["compatibility_result"] = compatibility
    return paths


def _write_plan_documents(plan: dict[str, Any]) -> None:
    """Materialize planner-owned JSON overlays beneath the run output."""

    def visit(value: object) -> None:
        if isinstance(value, dict):
            path = value.get("path")
            if isinstance(path, str) and "document" in value:
                destination = Path(path)
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(_canonical_json(value["document"]))
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(plan.get("inputs"))
    if plan["cell"] == "ac075":
        template = plan["inputs"]["overlay_template"]["document"]
        final_path = Path(plan["outputs"]["final_overlay"])
        final_path.parent.mkdir(parents=True, exist_ok=True)
        final_path.write_bytes(_canonical_json(template))


def _execute_plan(plan: dict[str, Any], fail_fast: bool) -> dict[str, object]:
    """Execute one cell's existing invocations in order."""
    del fail_fast
    cell = str(plan["cell"])
    results = []
    state = "passed"
    started = time.time()
    for invocation in plan["invocations"]:
        label = str(invocation["label"])
        log_root = Path(plan["cwd"])
        pending = list(plan["outputs"].values())
        while pending:
            output = pending.pop(0)
            if isinstance(output, list):
                pending.extend(output)
            elif isinstance(output, dict):
                pending.extend(output.values())
            elif isinstance(output, str) and Path(output).is_absolute():
                log_root = Path(output)
                while log_root.name and log_root.name != "cells":
                    log_root = log_root.parent
                if log_root.name == "cells":
                    log_root = log_root / cell / "logs"
                break
        if log_root == Path(plan["cwd"]):
            raise GauntletConfigError(f"cannot determine log root for {cell}")
        log_root.mkdir(parents=True, exist_ok=True)
        log = log_root / f"{label}.log"
        environment = os.environ.copy()
        environment.update({str(k): str(v) for k, v in plan["env"].items()})
        if cell in {"ac073", "ac075", "ce-profile"}:
            environment.update(_cuda_environment())
        for key in plan["unset_env"]:
            environment.pop(str(key), None)
        try:
            result = subprocess.run(
                [str(item) for item in invocation["argv"]],
                cwd=plan["cwd"],
                env=environment,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=int(plan["timeout_s"]),
            )
            log.write_text(result.stdout, encoding="utf-8")
            returncode = result.returncode
            timed_out = False
        except subprocess.TimeoutExpired as error:
            output = error.stdout or ""
            if isinstance(output, bytes):
                output = output.decode(errors="replace")
            log.write_text(output + "\nTIMEOUT\n", encoding="utf-8")
            returncode = 124
            timed_out = True
        except OSError as error:
            log.write_text(f"EXECUTION ERROR: {error}\n", encoding="utf-8")
            returncode = 127
            timed_out = False
        results.append(
            {
                "label": label,
                "returncode": returncode,
                "timed_out": timed_out,
                "log": str(log),
            }
        )
        if returncode:
            state = "failed"
            break
    return {
        "cell": cell,
        "state": state,
        "started_unix": started,
        "elapsed_s": time.time() - started,
        "invocations": results,
    }


def _planned_output_paths(plan: dict[str, Any]) -> list[Path]:
    """Return absolute artifact paths declared by one plan."""
    paths: list[Path] = []

    def visit(value: object) -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                if key == "completion_requirements":
                    continue
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)
        elif isinstance(value, str):
            path = Path(value)
            if path.is_absolute():
                paths.append(path)

    visit(plan.get("outputs", {}))
    return paths


def _enforce_completion_contract(
    raw: dict[str, object], plan: dict[str, Any]
) -> dict[str, object]:
    """Turn a zero-exit false pass into a failed cell with concrete evidence."""
    if raw.get("state") != "passed":
        cell = str(plan.get("cell"))
        if cell == "ac072":
            campaign = Path(str(plan["outputs"]["raw_root"]))
            verdicts = []
            for path in sorted(campaign.glob("*.verdict.json")):
                try:
                    verdicts.append(json.loads(path.read_text(encoding="utf-8")))
                except (OSError, json.JSONDecodeError):
                    continue
            if verdicts and any(
                item.get("status") == "ENVIRONMENT_INVALID" for item in verdicts
            ):
                raw = dict(raw)
                raw["state"] = "unavailable"
                raw["completion_error"] = (
                    "existing AC-072 runner rejected the measurement environment"
                )
                return raw
        if cell == "protected-writes":
            receipt = Path(str(plan["outputs"]["receipt"]))
            if receipt.is_file():
                try:
                    document = json.loads(receipt.read_text(encoding="utf-8"))
                except (OSError, json.JSONDecodeError):
                    document = {}
                failures = document.get("failures", [])
                if failures and all(
                    isinstance(item, dict)
                    and item.get("state") == "environment_invalid"
                    for item in failures
                ):
                    raw = dict(raw)
                    raw["state"] = "unavailable"
                    raw["completion_error"] = (
                        "existing protected-write runner rejected the measurement "
                        "environment"
                    )
                    return raw
        invocations = raw.get("invocations", [])
        if plan.get("cell") != "ac073" or len(invocations) != 1:
            return raw
        invocation = invocations[0]
        if not isinstance(invocation, dict):
            return raw
        log_value = invocation.get("log")
        log = Path(log_value) if isinstance(log_value, str) else None
        text = (
            log.read_text(encoding="utf-8", errors="replace")
            if log is not None and log.is_file()
            else ""
        )
        outputs_exist = all(path.is_file() for path in _planned_output_paths(plan))
        if not (
            invocation.get("returncode") == 101
            and outputs_exist
            and "EU7_WROTE " in text
            and "AC-075 recall verdict" in text
        ):
            return raw
        raw = dict(raw)
        raw["state"] = "passed"
        raw["informational_outcome"] = (
            "measurement complete; inherited recall floor failed directionally"
        )
    missing = [str(path) for path in _planned_output_paths(plan) if not path.exists()]
    requirements = plan.get("outputs", {}).get("completion_requirements", [])
    violations = []
    if "no SKIP:" in requirements:
        for invocation in raw.get("invocations", []):
            if not isinstance(invocation, dict):
                continue
            log = invocation.get("log")
            if isinstance(log, str) and Path(log).is_file():
                if "SKIP:" in Path(log).read_text(encoding="utf-8", errors="replace"):
                    violations.append(f"runner reported SKIP in {log}")
    if missing:
        violations.append("missing declared output: " + ", ".join(missing))
    if violations:
        raw = dict(raw)
        raw["state"] = "failed"
        raw["completion_error"] = "; ".join(violations)
    return raw


def _write_live_status(
    resolved: dict[str, Any], states: dict[str, str], *, mode: str = "run"
) -> None:
    """Persist current per-cell lifecycle state during sequential execution."""
    _write_json_atomic(
        Path(resolved["output_root"]) / "gauntlet-status.json",
        {
            "schema_version": "fathomdb.performance-gauntlet.status/v1",
            "release": resolved["release"],
            "source_commit": resolved["source"]["commit"],
            "mode": mode,
            "state": "running",
            "updated_at": _now(),
            "cells": states,
        },
    )


def _run_ready_plans(
    plans: Sequence[dict[str, Any]],
    resolved: dict[str, Any],
    *,
    resume: bool,
    fail_fast: bool,
) -> list[dict[str, object]]:
    """Execute preflight-ready plans sequentially with identity-safe resume."""
    output_root = Path(resolved["output_root"])
    results: list[dict[str, object]] = []
    states = {str(plan["cell"]): "pending" for plan in plans}
    for index, plan in enumerate(plans):
        cell = str(plan["cell"])
        result_path = _cell_result_path(output_root, cell)
        if resume and result_path.is_file():
            existing = json.loads(result_path.read_text(encoding="utf-8"))
            if existing.get("state") == "passed":
                if not _resume_result_matches(existing, plan, resolved):
                    raise GauntletConfigError(
                        f"cannot resume {cell}: passed result identity does not match"
                    )
                results.append(existing)
                states[cell] = "passed"
                _write_live_status(resolved, states)
                continue
            cell_root = result_path.parent
            attempt_root = (
                output_root
                / "attempts"
                / cell
                / datetime.now(UTC).strftime("%Y%m%dT%H%M%S.%fZ")
            )
            attempt_root.parent.mkdir(parents=True, exist_ok=True)
            os.replace(cell_root, attempt_root)

        _write_plan_documents(plan)
        plan_path = output_root / "cells" / cell / "cell-plan.json"
        _write_json_atomic(plan_path, plan)
        states[cell] = "running"
        _write_live_status(resolved, states)
        raw = _enforce_completion_contract(_execute_plan(plan, fail_fast), plan)
        result = _finalize_executed_result(raw, plan, resolved)
        _write_json_atomic(result_path, result)
        results.append(result)
        states[cell] = str(result["state"])
        _write_live_status(resolved, states)
        if result["state"] != "passed" and fail_fast:
            reason = "not run after fail-fast stopped the campaign"
            for remaining in plans[index + 1 :]:
                remaining_cell = str(remaining["cell"])
                skipped = _not_run_result(remaining_cell, reason, resolved)
                _write_json_atomic(
                    _cell_result_path(output_root, remaining_cell), skipped
                )
                results.append(skipped)
                states[remaining_cell] = "not-run"
            _write_live_status(resolved, states)
            break
    return results


def _verify_source_checkout(resolved: dict[str, Any]) -> None:
    """Require the target checkout to be clean, exact, and release-tagged."""
    source = Path(resolved["source"]["root"])
    expected_commit = resolved["source"]["commit"]
    if _git_output(source, "rev-parse", "HEAD") != expected_commit:
        raise GauntletConfigError(
            "target checkout HEAD does not match configured commit"
        )
    development_graph_smoke = (
        os.environ.get("FATHOMDB_GRAPH_SMOKE_ALLOW_DIRTY") == "1"
        and tuple(resolved["cells"]) == GRAPH_CELLS
    )
    dirty = _git_output(source, "status", "--porcelain", "--untracked-files=all")
    if dirty and not development_graph_smoke:
        raise GauntletConfigError("target checkout must be clean")
    tags = set(_git_output(source, "tag", "--points-at", "HEAD").splitlines())
    expected_tag = f"v{resolved['release']}"
    if expected_tag not in tags and not development_graph_smoke:
        raise GauntletConfigError(
            f"target checkout is not tagged as requested release {expected_tag}"
        )


def _health_check_plan(plan: dict[str, Any], resolved: dict[str, Any]) -> None:
    """Perform cheap syntax/help/import checks without running a measurement."""
    cell = str(plan["cell"])
    declared = str(plan["adapter"])
    adapter = Path(declared.split("#", 1)[0])
    if not adapter.is_file():
        raise GauntletConfigError(f"runner is unavailable for {cell}: {adapter}")
    log = Path(resolved["output_root"]) / "preflight" / f"{cell}-runner-health.log"
    if adapter.suffix == ".sh":
        _run_checked(["bash", "-n", str(adapter)], Path(plan["cwd"]), log, 60)
    elif adapter.suffix == ".py":
        invocation = plan["invocations"][0]["argv"]
        candidate = Path(str(invocation[0]))
        python = candidate if candidate.is_file() else Path(sys.executable)
        if len(invocation) >= 3 and invocation[1] == "-m":
            health_command = [str(python), "-m", str(invocation[2]), "--help"]
        else:
            health_command = [str(python), str(adapter), "--help"]
        environment = os.environ.copy()
        environment.update({str(k): str(v) for k, v in plan["env"].items()})
        _run_checked(
            health_command,
            Path(plan["cwd"]),
            log,
            120,
            environment=environment,
        )

    python_binding = resolved.get("runtime", {}).get("python")
    if isinstance(python_binding, dict):
        python_path = str(python_binding.get("path", ""))
        if python_path and python_path in json.dumps(plan, sort_keys=True):
            import_log = (
                Path(resolved["output_root"])
                / "preflight"
                / f"{cell}-target-import.log"
            )
            statement = (
                "import fathomdb; "
                f"assert fathomdb.__version__ == {resolved['release']!r}, "
                "fathomdb.__version__"
            )
            _run_checked(
                [python_path, "-c", statement],
                Path(plan["cwd"]),
                import_log,
                120,
            )
    if cell == "locomo":
        harness = plan["inputs"]["harness"]
        _run_checked(
            [
                harness["python"],
                "-c",
                "from benchmarks.locomo import run as _run",
            ],
            Path(harness["checkout"]),
            Path(resolved["output_root"]) / "preflight" / "locomo-harness-import.log",
            120,
        )


def _bind_locomo_harness_identity(
    resolved: dict[str, Any], module: object, repo_root: Path
) -> None:
    """Verify the external patched harness and expose its identity to planning."""
    assets = resolved.get("assets", {}).get("locomo")
    if not isinstance(assets, dict):
        raise GauntletConfigError("required LOCOMO assets are unavailable")
    checkout_binding = assets.get("harness_checkout")
    if not isinstance(checkout_binding, dict):
        raise GauntletConfigError("required LOCOMO harness checkout is unavailable")
    checkout = Path(str(checkout_binding.get("path", "")))
    if _git_output(checkout, "status", "--porcelain", "--untracked-files=all"):
        raise GauntletConfigError("required LOCOMO harness checkout is not clean")
    head = _git_output(checkout, "rev-parse", "HEAD")
    parent = _git_output(checkout, "rev-parse", "HEAD^")
    upstream = str(module.LOCOMO_UPSTREAM_SHA)
    if parent != upstream:
        raise GauntletConfigError("required LOCOMO harness upstream parent drifted")
    tree = _git_output(checkout, "rev-parse", "HEAD^{tree}")
    if tree != module.LOCOMO_PATCHED_TREE_SHA:
        raise GauntletConfigError("required LOCOMO harness patched tree drifted")
    patch = (
        repo_root / "experiments/configs/mem0-oss/memory01-resilience.patch"
    ).resolve()
    if _sha256(patch) != module.LOCOMO_PATCH_SHA256:
        raise GauntletConfigError("required LOCOMO harness patch identity drifted")
    reverse = subprocess.run(
        ["git", "apply", "--reverse", "--check", str(patch)],
        cwd=checkout,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if reverse.returncode:
        raise GauntletConfigError("required LOCOMO harness patch is not applied")
    resolved["locomo_harness_identity"] = {
        "checkout": str(checkout.resolve()),
        "git_sha": head,
        "upstream_git_sha": upstream,
        "patch_path": str(patch),
        "patch_sha256": module.LOCOMO_PATCH_SHA256,
        "tree_sha": tree,
    }


def _prepare_one_plan(
    resolved: dict[str, Any],
    module: object,
    repo_root: Path,
    artifact_cache: dict[str, str],
) -> tuple[dict[str, Any], dict[str, str]]:
    """Plan, bind generated artifacts, and health-check one selected cell."""
    plans = module.build_execution_map(resolved, repo_root)
    if len(plans) != 1:
        raise GauntletConfigError("single-cell planner returned the wrong plan count")
    plan = plans[0]
    _verify_required_gpu(resolved, [plan])
    cell = str(plan["cell"])
    required = {
        "ac072": {
            "artifact:perf_gates.path",
            "artifact:perf_gates.sha256",
            "artifact:perf_gates.input_sha256",
        },
        "ac081": {
            "artifact:perf_gates.path",
            "artifact:perf_gates.sha256",
            "artifact:perf_gates.input_sha256",
        },
        "ac075": {
            "artifact:tc5_benchmark.path",
            "artifact:tc5_benchmark.sha256",
        },
    }.get(cell, set())
    if required.issubset(artifact_cache):
        replacements = {key: artifact_cache[key] for key in required}
    else:
        replacements = _prepare_artifacts(resolved, [plan], repo_root)
        artifact_cache.update(replacements)
    if cell == "ac075":
        replacements["artifact:ac075_overlay.path"] = str(
            Path(resolved["output_root"])
            / "cells"
            / "ac075"
            / "tc5-0.8.26.overlay.json"
        )
    plan = _replace(plan, replacements)
    if plan["cell"] == "ac075":
        overlay = plan["inputs"]["overlay_template"]["document"]
        replacements.update(
            {
                "artifact:ac075_overlay.sha256": hashlib.sha256(
                    _canonical_json(overlay)
                ).hexdigest(),
            }
        )
        plan = _replace(plan, replacements)
    _write_plan_documents(plan)
    _health_check_plan(plan, resolved)
    return plan, replacements


def _preflight_cells(
    resolved: dict[str, Any], module: object, repo_root: Path
) -> tuple[list[dict[str, Any]], list[dict[str, object]], dict[str, str]]:
    """Classify each selected cell independently as ready or unavailable."""
    ready: list[dict[str, Any]] = []
    outcomes: list[dict[str, object]] = []
    artifacts: dict[str, str] = {}
    artifact_cache: dict[str, str] = {}
    try:
        _verify_source_checkout(resolved)
    except (GauntletConfigError, OSError, ValueError) as error:
        return (
            [],
            [
                _unavailable_result(cell, str(error), resolved, phase="preflight")
                for cell in resolved["cells"]
            ],
            {},
        )

    for cell in resolved["cells"]:
        one = copy.deepcopy(resolved)
        one["cells"] = [cell]
        one["timeouts_s"] = {cell: resolved["timeouts_s"][cell]}
        try:
            if cell == "locomo":
                _bind_locomo_harness_identity(one, module, repo_root)
            plan, replacements = _prepare_one_plan(
                one, module, repo_root, artifact_cache
            )
            ready.append(plan)
            outcomes.append(_ready_result(plan, resolved))
            artifacts.update(
                {f"{cell}:{key}": value for key, value in replacements.items()}
            )
        except (
            GauntletConfigError,
            OSError,
            ValueError,
            subprocess.SubprocessError,
        ) as error:
            outcomes.append(
                _unavailable_result(cell, str(error), resolved, phase="preflight")
            )
    return ready, outcomes, artifacts


def _write_runtime_manifest(
    resolved: dict[str, Any], artifacts: dict[str, str]
) -> Path:
    """Persist generated binary and overlay bindings from preflight."""
    path = Path(resolved["output_root"]) / "runtime" / "artifact-manifest.json"
    _write_json_atomic(
        path,
        {
            "schema_version": "fathomdb.performance-gauntlet.artifacts/v1",
            "release": resolved["release"],
            "source_commit": resolved["source"]["commit"],
            "artifacts": artifacts,
        },
    )
    return path


def _write_preflight_cell_results(
    resolved: dict[str, Any], outcomes: Sequence[dict[str, object]]
) -> None:
    """Retain each preflight outcome without overwriting a completed run result."""
    output_root = Path(resolved["output_root"])
    for outcome in outcomes:
        cell = str(outcome["cell"])
        path = output_root / "cells" / cell / "preflight-result.json"
        _write_json_atomic(path, outcome)
        result_path = _cell_result_path(output_root, cell)
        if outcome["state"] == "unavailable" and not result_path.exists():
            _write_json_atomic(result_path, outcome)


def _merge_campaign_results(
    campaign: dict[str, Any], current: Sequence[dict[str, object]]
) -> list[dict[str, object]]:
    """Merge a subset resume without shrinking or falsifying campaign scope."""
    by_cell = {str(item["cell"]): item for item in current}
    output_root = Path(campaign["output_root"])
    merged = []
    for cell in campaign["cells"]:
        if cell in by_cell:
            merged.append(by_cell[cell])
            continue
        path = _cell_result_path(output_root, cell)
        if path.is_file():
            merged.append(json.loads(path.read_text(encoding="utf-8")))
        else:
            merged.append(
                _not_run_result(
                    cell,
                    "not selected or not reached in the current campaign",
                    campaign,
                )
            )
    return merged


def _execute(args: argparse.Namespace, cells: tuple[str, ...]) -> int:
    """Resolve, preflight, and execute the selected gauntlet cells."""
    config, current_config_identity = _read_configuration_snapshot(args.config)
    assert_cli_configuration(
        config,
        release=args.release,
        source_root=args.source_root,
        output_root=args.output_root,
    )
    if args.resume:
        resolved_path = Path(args.output_root) / "gauntlet-plan.resolved.json"
        if not resolved_path.is_file():
            raise GauntletConfigError("--resume requires an existing resolved plan")
        campaign = json.loads(resolved_path.read_text(encoding="utf-8"))
        if campaign.get("source_config") != current_config_identity:
            raise GauntletConfigError(
                "--resume configuration identity does not match the original campaign"
            )
        resolved = copy.deepcopy(campaign)
        if args.cells is not None or getattr(args, "suite", None) is not None:
            resolved["cells"] = list(select_configured_cells(resolved["cells"], cells))
            resolved["timeouts_s"] = {
                cell: resolved["timeouts_s"][cell] for cell in resolved["cells"]
            }
    else:
        selected = (
            None
            if args.cells is None and getattr(args, "suite", None) is None
            else cells
        )
        resolved_path = write_resolved_configuration(args.config, selected)
        resolved = json.loads(resolved_path.read_text(encoding="utf-8"))
        campaign = copy.deepcopy(resolved)

    module = _load_cells_module()
    repo_root = Path(__file__).resolve().parents[2]
    plans, preflight, artifacts = _preflight_cells(resolved, module, repo_root)
    plan_path = Path(resolved["output_root"]) / "execution-plan.json"
    _write_json_atomic(plan_path, plans)
    _write_runtime_manifest(resolved, artifacts)
    _write_preflight_cell_results(resolved, preflight)
    if args.preflight_only:
        paths = _write_campaign_documents(resolved, preflight, mode="preflight")
        print(paths["summary_json"])
        return 0 if all(item["state"] == "ready" for item in preflight) else 1

    unavailable = {
        str(item["cell"]): item for item in preflight if item["state"] == "unavailable"
    }
    for item in unavailable.values():
        _write_json_atomic(
            _cell_result_path(Path(resolved["output_root"]), str(item["cell"])),
            item,
        )

    if args.fail_fast and unavailable:
        first_blocked = min(resolved["cells"].index(cell) for cell in unavailable)
        executable = [
            plan
            for plan in plans
            if resolved["cells"].index(str(plan["cell"])) < first_blocked
        ]
        stopped = [
            plan
            for plan in plans
            if resolved["cells"].index(str(plan["cell"])) > first_blocked
        ]
    else:
        executable = plans
        stopped = []

    executed = _run_ready_plans(
        executable,
        resolved,
        resume=args.resume,
        fail_fast=args.fail_fast,
    )
    stopped_results = []
    for plan in stopped:
        item = _not_run_result(
            str(plan["cell"]),
            "not run after fail-fast stopped at unavailable preflight",
            resolved,
        )
        stopped_results.append(item)
        _write_json_atomic(
            _cell_result_path(Path(resolved["output_root"]), str(plan["cell"])), item
        )

    current_by_cell = {
        str(item["cell"]): item
        for item in [*executed, *unavailable.values(), *stopped_results]
    }
    selected_results = [
        current_by_cell[cell] for cell in resolved["cells"] if cell in current_by_cell
    ]
    if args.resume and campaign["cells"] != resolved["cells"]:
        results = _merge_campaign_results(campaign, selected_results)
        summary_scope = campaign
    else:
        results = selected_results
        summary_scope = resolved
    paths = _write_campaign_documents(summary_scope, results, mode="run")
    summary = json.loads(paths["summary_json"].read_text(encoding="utf-8"))
    print(paths["summary_json"])
    return 0 if summary["state"] == "passed" else 1


def main(argv: Sequence[str] | None = None) -> int:
    """Parse the CLI and execute the currently supported gauntlet mode."""
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        cells = parse_suite_selection(args.suite, args.cells)
    except GauntletArgumentError as error:
        parser.error(str(error))

    if args.dry_run and args.preflight_only:
        parser.error("--dry-run and --preflight-only cannot be combined")
    if args.dry_run:
        print(json.dumps(_dry_run_projection(args, cells), indent=2, sort_keys=True))
        return 0
    try:
        return _execute(args, cells)
    except (GauntletConfigError, OSError, ValueError) as error:
        print(f"gauntlet: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
