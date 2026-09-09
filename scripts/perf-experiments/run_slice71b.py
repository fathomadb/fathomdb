#!/usr/bin/env python3
"""Build and execute one isolated Slice 71B write-measurement cell."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from experiments import scale_02  # noqa: E402
from experiments.release_0825_slice71b import validate_attribution_manifest  # noqa: E402


PROBE = ROOT / "experiments" / "slice71b_write_probe.rs"
DEFAULT_SCALE02_CONFIG = ROOT / "experiments" / "configs" / "scale-02" / "a0-envelope.v2.json"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


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
    for label in Path("/sys/class/hwmon").glob("hwmon*/temp*_label"):
        try:
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
        and int(end["swap_in"]) - int(start["swap_in"]) <= int(policy["max_swap_io_delta"])
        and int(end["swap_out"]) - int(start["swap_out"]) <= int(policy["max_swap_io_delta"])
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
    ).stdout.strip()
    dirty = bool(
        subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=no"],
            cwd=source_root,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
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


def _build_probe(source_root: Path, target_root: Path) -> Path:
    temporary = Path(tempfile.mkdtemp(prefix="fathomdb-slice71b-probe-"))
    try:
        manifest = _write_probe_manifest(temporary, source_root)
        subprocess.run(
            [
                "cargo",
                "build",
                "--offline",
                "--release",
                "--manifest-path",
                str(manifest),
                "--target-dir",
                str(target_root),
            ],
            check=True,
            cwd=source_root,
        )
    finally:
        shutil.rmtree(temporary)
    return target_root / "release" / "slice71b-probe"


def run_cell(
    manifest_path: Path,
    source_root: Path,
    fixture: str,
    treatment: str,
    ordinal: int,
    output_root: Path,
) -> None:
    document = json.loads(manifest_path.read_text(encoding="utf-8"))
    validate_attribution_manifest(document)
    source_ref, dirty = _source_identity(source_root)
    expected = document["product_sources"]["unchanged_current"]
    if source_ref != expected or dirty:
        raise RuntimeError("attribution cells require the sealed clean unchanged-current source")
    fixture_config = document["fixtures"][fixture]
    records = 10_000
    batch_size = int(fixture_config["batch_size_10k"])
    probe_sha = sha256(PROBE)
    if probe_sha != document["probe"]["sha256"]:
        raise RuntimeError("probe digest drifted")

    cell_root = output_root / fixture / f"{treatment}-{ordinal}"
    cell_root.mkdir(parents=True, exist_ok=False)
    target_root = output_root / "targets" / source_ref
    executable = _build_probe(source_root, target_root)
    database = cell_root / "cell.fathom"
    command = [
        str(executable),
        "--fixture",
        fixture,
        "--treatment",
        treatment,
        "--records",
        str(records),
        "--batch-size",
        str(batch_size),
        "--database",
        str(database),
    ]
    if fixture == "scale02":
        command.extend(["--input-jsonl", str(ROOT / fixture_config["input_jsonl"])])
    started_at = datetime.now(UTC).isoformat()
    start = environment_snapshot()
    completed = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    end = environment_snapshot()
    finished_at = datetime.now(UTC).isoformat()
    raw_log = cell_root / "raw.log"
    raw_log.write_text(completed.stdout, encoding="utf-8")
    if completed.returncode != 0:
        raise RuntimeError(f"probe failed; retained output at {raw_log}")
    lines = [line for line in completed.stdout.splitlines() if line.startswith("{")]
    if len(lines) != 1:
        raise RuntimeError("probe output did not contain exactly one JSON record")
    metrics = json.loads(lines[0])
    if metrics.pop("schema_version") != "slice71b-write-probe.v1":
        raise RuntimeError("probe schema drifted")
    if metrics.pop("fixture") != fixture or metrics.pop("treatment") != treatment:
        raise RuntimeError("probe identity drifted")
    cell = {
        "fixture": fixture,
        "treatment": treatment,
        "ordinal": ordinal,
        "source_ref": source_ref,
        "source_tree_dirty": dirty,
        "probe_sha256": probe_sha,
        "fixture_sha256": fixture_config["fixture_sha256"],
        "started_at": started_at,
        "finished_at": finished_at,
        "environment_valid": environment_valid(start, end, document["environment_policy"]),
        "raw_log_sha256": sha256(raw_log),
        "metrics": metrics,
    }
    (cell_root / "cell.json").write_text(
        json.dumps(cell, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (cell_root / "environment.json").write_text(
        json.dumps({"start": start, "end": end}, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(cell, sort_keys=True))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    generate = subparsers.add_parser("generate-scale02-input")
    generate.add_argument("--config", type=Path, default=DEFAULT_SCALE02_CONFIG)
    generate.add_argument("--output", type=Path, required=True)
    cell = subparsers.add_parser("run-cell")
    cell.add_argument("--manifest", type=Path, required=True)
    cell.add_argument("--source-root", type=Path, required=True)
    cell.add_argument("--fixture", choices=("scale02", "ac013"), required=True)
    cell.add_argument(
        "--treatment",
        choices=("production", "generation_only", "no_op"),
        required=True,
    )
    cell.add_argument("--ordinal", type=int, choices=(1, 2, 3), required=True)
    cell.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command == "generate-scale02-input":
        generate_scale02_input(args.config, args.output)
    else:
        run_cell(
            args.manifest,
            args.source_root.resolve(),
            args.fixture,
            args.treatment,
            args.ordinal,
            args.output_root.resolve(),
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
