#!/usr/bin/env python3
"""Measure installed-Python default-embedder open/close memory in one process."""

from __future__ import annotations

import argparse
from dataclasses import asdict
import gc
import hashlib
import json
import os
from pathlib import Path
import resource
import sys
from tempfile import TemporaryDirectory
import time
from zipfile import ZipFile


TEXT = "the central bank raised interest rates"
MEMORY_FIELDS = ("rss_kib", "pss_kib", "private_kib")


def sha(path: Path) -> str:
    """Hash exact file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_smaps_rollup(contents: str) -> dict[str, int]:
    """Read Linux resident, proportional and private memory in KiB."""
    values = {}
    for line in contents.splitlines():
        if ":" not in line:
            continue
        key, rest = line.split(":", 1)
        if key in ("Rss", "Pss", "Private_Clean", "Private_Dirty"):
            fields = rest.strip().split()
            if len(fields) != 2 or fields[1] != "kB":
                raise ValueError("invalid smaps_rollup unit")
            values[key] = int(fields[0])
    if set(values) != {"Rss", "Pss", "Private_Clean", "Private_Dirty"}:
        raise ValueError("missing smaps_rollup memory field")
    return {
        "rss_kib": values["Rss"],
        "pss_kib": values["Pss"],
        "private_kib": values["Private_Clean"] + values["Private_Dirty"],
    }


def memory() -> dict[str, int]:
    """Sample the current process's Linux smaps rollup."""
    return parse_smaps_rollup(Path("/proc/self/smaps_rollup").read_text())


def validate_observation(raw: dict) -> None:
    """Reject malformed state or a failed real-database lifecycle assertion."""
    if (
        raw.get("semantic_ok") is not True
        or raw.get("vector_dimension") != 384
        or raw.get("vector_nonzero") is not True
        or raw.get("reopen_ok", True) is not True
    ):
        raise ValueError("semantic lifecycle assertion failed")
    if type(raw.get("close_ns")) is not int or raw["close_ns"] <= 0:
        raise ValueError("close duration invalid")
    for state in ("before", "opened", "closed_idle", "dropped"):
        if state not in raw:
            raise ValueError(f"missing {state} memory snapshot")
        values = raw[state]
        if set(values) != set(MEMORY_FIELDS) or any(
            type(values[key]) is not int or values[key] <= 0 for key in MEMORY_FIELDS
        ):
            raise ValueError(f"invalid {state} memory snapshot")
    if "closed" in raw and (
        set(raw["closed"]) != set(MEMORY_FIELDS)
        or any(
            type(raw["closed"][key]) is not int or raw["closed"][key] <= 0
            for key in MEMORY_FIELDS
        )
    ):
        raise ValueError("invalid immediate-close memory snapshot")


def run_once(wheel: Path, wheel_sha256: str, source_sha: str) -> dict:
    """Run one fresh real-database default-model lifecycle and sample memory."""
    import fathomdb
    import fathomdb._fathomdb as native

    if sha(wheel) != wheel_sha256:
        raise ValueError("wheel SHA-256 changed")
    if len(source_sha) != 40 or any(
        char not in "0123456789abcdef" for char in source_sha
    ):
        raise ValueError("source SHA invalid")
    module = Path(fathomdb.__file__).resolve()
    native_path = Path(native.__file__).resolve()
    if (
        not module.is_relative_to(Path(sys.prefix).resolve())
        or "site-packages" not in module.parts
    ):
        raise ValueError("Python module escaped installed environment")
    with ZipFile(wheel) as archive:
        if module.read_bytes() != archive.read("fathomdb/__init__.py"):
            raise ValueError("installed Python module differs from wheel")
        native_sha256 = hashlib.sha256(
            archive.read("fathomdb/_fathomdb.abi3.so")
        ).hexdigest()
    if sha(native_path) != native_sha256:
        raise ValueError("installed native module differs from wheel")
    gc.collect()
    with TemporaryDirectory(prefix="slice135-close-memory-") as directory:
        database = str(Path(directory) / "lifecycle.sqlite")
        before = memory()
        open_start = time.perf_counter_ns()
        engine = fathomdb.Engine.open(database, use_default_embedder=True)
        vector = engine.embed(TEXT)
        open_embed_ns = time.perf_counter_ns() - open_start
        vector_dimension = len(vector)
        vector_nonzero = any(vector)
        embedder = asdict(engine.open_report().default_embedder)
        opened = memory()
        close_start = time.perf_counter_ns()
        engine.close()
        close_ns = time.perf_counter_ns() - close_start
        closed = memory()
        time.sleep(0.2)
        closed_idle = memory()
        engine.close()
        del engine
        gc.collect()
        dropped = memory()
        reopened = fathomdb.Engine.open(database, use_default_embedder=False)
        reopened.close()
        usage = resource.getrusage(resource.RUSAGE_SELF)
    raw = {
        "schema_version": 1,
        "status": "S02_LIFECYCLE_MEMORY_DIAGNOSTIC",
        "source_sha": source_sha,
        "runner_sha256": sha(Path(__file__)),
        "artifact": {
            "wheel_sha256": wheel_sha256,
            "native_sha256": native_sha256,
            "installed_module": str(module),
            "installed_native": str(native_path),
        },
        "host": {
            "uname": list(os.uname()),
            "python": sys.version,
            "embed_device": os.environ.get("FATHOMDB_EMBED_DEVICE"),
        },
        "embedder": embedder,
        "vector_dimension": vector_dimension,
        "vector_nonzero": vector_nonzero,
        "vector_sha256": hashlib.sha256(json.dumps(vector).encode()).hexdigest(),
        "semantic_ok": True,
        "reopen_ok": True,
        "before": before,
        "opened": opened,
        "closed": closed,
        "closed_idle": closed_idle,
        "dropped": dropped,
        "open_embed_ns": open_embed_ns,
        "close_ns": close_ns,
        "resource": {
            "peak_rss_kib": usage.ru_maxrss,
            "user_cpu_s": usage.ru_utime,
            "system_cpu_s": usage.ru_stime,
            "major_faults": usage.ru_majflt,
            "swap_events": usage.ru_nswap,
        },
    }
    validate_observation(raw)
    return raw


def main() -> None:
    """Write one source-bound lifecycle result without overwriting evidence."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("refusing to overwrite lifecycle receipt")
    raw = run_once(args.wheel, args.wheel_sha256, args.source_sha)
    args.output.write_text(json.dumps(raw, indent=2) + "\n")


if __name__ == "__main__":
    main()
