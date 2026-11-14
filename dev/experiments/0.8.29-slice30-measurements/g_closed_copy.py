"""Byte-preserving preparation from an explicitly released CLOSED SQLite subject."""

from pathlib import Path
import time

from g_freeze import identify
from g_span_coverage import stored_coverage


def copy_subject(subject, destination, deadline_ns):
    """Copy valid closed packaging to fresh paths and verify before any workload starts.

    The caller must execute this inside the monitored owned worker, never the
    controller's synchronous prepare callback. Errors leave partials retained.
    """
    destination = Path(destination)
    files = subject["files"]
    if "" not in files or set(files) - {"", "-wal"}:
        raise ValueError("closed subject packaging must name its main file and any required WAL")
    source = Path(files[""]["path"])
    if source.with_name(source.name + "-wal").exists() and "-wal" not in files:
        raise ValueError("closed subject has an unaccounted WAL companion")
    if destination.exists() or any(destination.with_name(destination.name + suffix).exists() for suffix in files):
        raise ValueError("closed copy destination must be fresh")
    destination.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic_ns()
    copied = 0
    for suffix, record in files.items():
        target = destination.with_name(destination.name + suffix)
        with Path(record["path"]).open("rb") as reader, target.open("xb") as writer:
            while True:
                if time.monotonic_ns() >= deadline_ns:
                    raise TimeoutError("closed copy preparation exhausted its absolute cell deadline")
                block = reader.read(1024 * 1024)
                if not block:
                    break
                writer.write(block)
                copied += len(block)
    copied_at = time.monotonic_ns()
    for suffix, record in files.items():
        actual = identify(destination.with_name(destination.name + suffix))
        if (actual["bytes"], actual["sha256"]) != (record["bytes"], record["sha256"]):
            raise ValueError("closed copy does not preserve its released subject bytes")
    coverage = stored_coverage(destination)
    if coverage != subject["coverage"]:
        raise ValueError("closed copy stored ranges/body bytes differ from released coverage")
    finished = time.monotonic_ns()
    if finished >= deadline_ns:
        raise TimeoutError("closed copy validation exhausted its absolute cell deadline")
    return str(destination), {"validated": True, "source": str(source), "copied_bytes": copied,
        "copy_ns": copied_at - started, "validation_ns": finished - copied_at,
        "coverage": {key: value for key, value in coverage.items() if key != "spans"}}
