#!/usr/bin/env python3
"""Close Engine-managed SQLite opener categories across current Rust owners."""

import argparse
from collections import Counter
from pathlib import Path
import re


ROLES = ("Writer", "ReaderWorker", "ProjectionDispatcher", "ProjectionWorker", "RuntimeProbe")
EXPECTED_CALLS = Counter({
    ("open.rs", "open_managed_connection", "Writer"): 1,
    ("open.rs", "open_managed_connection", "ReaderWorker"): 1,
    ("projection_worker.rs", "open_runtime_connection", "ProjectionDispatcher"): 1,
    ("projection_worker.rs", "open_runtime_connection", "ProjectionWorker"): 1,
    ("projection_worker.rs", "open_runtime_connection", "RuntimeProbe"): 1,
    ("projection_commit.rs", "open_runtime_connection", "ProjectionWorker"): 1,
    ("wal_runtime.rs", "open_managed_connection", "RuntimeProbe"): 2,
    ("read_api.rs", "open_managed_connection", "RuntimeProbe"): 1,
    ("connection_runtime.rs", "open_runtime_connection", "RuntimeProbe"): 1,
    ("connection_runtime.rs", "open_managed_connection", "category"): 1,
})
CALL = re.compile(r"\b(open_managed_connection|open_runtime_connection)\s*\(")
CLASSIFIED_CALL = re.compile(
    r"\b(open_managed_connection|open_runtime_connection)\s*\(\s*[^,]+,\s*"
    r"(?:#\[cfg[^\]]*\]\s*)?(?:ManagedConnectionCategory::(\w+)|(category))\s*,",
    re.S,
)
RAW_OPEN = re.compile(r"(?<![A-Za-z0-9_])Connection::(open\w*)\s*\(")
FUNCTION = re.compile(r"(?m)^(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)\s*\(")
RO_FLAGS = (
    "rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY|"
    "rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX|"
    "rusqlite::OpenFlags::SQLITE_OPEN_URI,"
)
EXPECTED_RAW_OPENS = Counter({
    ("connection_runtime.rs", "open_managed_connection", "open", "path"): 1,
    ("open.rs", "read_effective_schema_version", "open_with_flags", "read_only_sqlite_uri(path)," + RO_FLAGS): 1,
    # Slice 103 shares WAL admission across truncate and owed-erasure recovery.
    ("operator/data_plane.rs", "open_recovery_connection", "open_with_flags", "immutable_sqlite_uri(&canonical_path)," + RO_FLAGS): 1,
    ("operator/data_plane.rs", "open_recovery_connection", "open_with_flags", "sqlite_uri(&canonical_path,\"mode=rw\"),rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE|rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX|rusqlite::OpenFlags::SQLITE_OPEN_URI,"): 1,
    ("operator/data_plane.rs", "validate_effective_recovery_schema", "open_with_flags", "read_only_sqlite_uri(path)," + RO_FLAGS): 1,
    ("operator/data_plane.rs", "inspect_data_plane_integrity", "open_with_flags", "immutable_sqlite_uri(&canonical_path),OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_NO_MUTEX|OpenFlags::SQLITE_OPEN_URI,"): 1,
})


def production(source: str) -> str:
    source = re.sub(r"(?ms)^#\[cfg\(test\)\]\s*\nmod \w+ \{.*?^}\s*\n?", "", source)
    return re.sub(r"(?m)//[^\n]*", "", source)


def source_inventory(src: Path, overrides: dict[str, Path] | None = None) -> dict[str, str]:
    overrides = overrides or {}
    root = overrides.get("lib.rs", src / "lib.rs").read_text()
    test_modules = set(re.findall(r"#\[cfg\(test\)\]\s*\nmod\s+(\w+);", root))
    return {
        path.relative_to(src).as_posix(): production(overrides.get(path.relative_to(src).as_posix(), path).read_text())
        for path in sorted(src.rglob("*.rs"))
        if path.relative_to(src).as_posix() not in {f"{name}.rs" for name in test_modules}
    }


def raw_call_args(source: str, start: int) -> str:
    depth = 1
    end = start
    while depth:
        if source[end] == "(":
            depth += 1
        elif source[end] == ")":
            depth -= 1
        end += 1
    return re.sub(r"\s+", "", source[start:end - 1])


def owner_errors(sources: dict[str, str]) -> list[str]:
    errors = []
    wal = sources.get("wal_runtime.rs", "")
    enum = re.search(r"(?ms)^pub\(crate\) enum ManagedConnectionCategory \{(.*?)^}", wal)
    variants = tuple(re.findall(r"(?m)^\s+(\w+),\s*$", enum.group(1))) if enum else ()
    if variants != ROLES:
        errors.append(f"WAL runtime category enum differs: {variants!r}")
    if "ManagedConnectionCategory" in sources.get("lib.rs", ""):
        errors.append("root carries managed connection category")

    connection = sources.get("connection_runtime.rs", "")
    factory = re.search(r"(?ms)^pub\(crate\) fn open_managed_connection\(.*?^}", connection)
    factory_body = factory.group() if factory else ""
    registration = factory_body.find("managed_connections.record_open(category);")
    raw_open = factory_body.find("Connection::open(path)")
    if ("category: ManagedConnectionCategory" not in factory_body or registration < 0
            or raw_open < 0 or registration > raw_open):
        errors.append("managed factory lacks typed category registration before SQLite open")
    runtime = re.search(r"(?ms)^pub\(crate\) fn open_runtime_connection\(.*?^}", connection)
    runtime_body = runtime.group() if runtime else ""
    if ("category: ManagedConnectionCategory" not in runtime_body
            or not re.search(r"open_managed_connection\(\s*path,\s*(?:#\[cfg[^\]]*\]\s*)?category,\s*"
                             r"(?:#\[cfg[^\]]*\]\s*)?managed_connections,", runtime_body, re.S)):
        errors.append("runtime opener does not forward typed category to managed factory")

    calls = Counter()
    raw_sites = Counter()
    for path, source in sources.items():
        matched_starts = set()
        for match in CLASSIFIED_CALL.finditer(source):
            prefix = source[max(0, match.start() - 16):match.start()]
            if re.search(r"\bfn\s*$", prefix):
                continue
            role = match.group(2) or match.group(3)
            calls[path, match.group(1), role] += 1
            matched_starts.add(match.start())
        for match in CALL.finditer(source):
            prefix = source[max(0, match.start() - 16):match.start()]
            if re.search(r"\bfn\s*$", prefix):
                continue
            if match.start() not in matched_starts:
                errors.append(f"unclassified managed opener call in {path}")
        for match in RAW_OPEN.finditer(source):
            functions = list(FUNCTION.finditer(source, 0, match.start()))
            function = functions[-1].group(1) if functions else "<module>"
            raw_sites[path, function, match.group(1), raw_call_args(source, match.end())] += 1
    if calls != EXPECTED_CALLS:
        missing = EXPECTED_CALLS - calls
        unexpected = calls - EXPECTED_CALLS
        errors.append(f"managed call inventory differs: missing={dict(missing)!r} unexpected={dict(unexpected)!r}")
    if raw_sites != EXPECTED_RAW_OPENS:
        missing = EXPECTED_RAW_OPENS - raw_sites
        unexpected = raw_sites - EXPECTED_RAW_OPENS
        errors.append(f"direct SQLite open outside sole audited factory or scoped standalone probes: missing={dict(missing)!r} unexpected={dict(unexpected)!r}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("src", type=Path)
    parser.add_argument("--override", action="append", default=[], metavar="RELATIVE=PATH")
    args = parser.parse_args()
    overrides = {}
    for item in args.override:
        name, sep, path = item.partition("=")
        if not sep:
            parser.error(f"invalid override {item!r}")
        overrides[name] = Path(path)
    errors = owner_errors(source_inventory(args.src, overrides))
    for error in errors:
        print(error)
    return bool(errors)


if __name__ == "__main__":
    raise SystemExit(main())
