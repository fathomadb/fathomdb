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
RAW_OPEN = re.compile(r"(?<![A-Za-z0-9_])Connection::open(?:_[A-Za-z0-9]+)?\s*\(")


def production(source: str) -> str:
    source = re.split(r"(?m)^mod tests \{", source, maxsplit=1)[0]
    return re.sub(r"(?m)//[^\n]*", "", source)


def source_inventory(src: Path, overrides: dict[str, Path] | None = None) -> dict[str, str]:
    overrides = overrides or {}
    return {
        path.relative_to(src).as_posix(): production(overrides.get(path.relative_to(src).as_posix(), path).read_text())
        for path in sorted(src.rglob("*.rs"))
    }


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
    raw_sites = []
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
            line = source.count("\n", 0, match.start()) + 1
            raw_sites.append((path, line))
    if calls != EXPECTED_CALLS:
        missing = EXPECTED_CALLS - calls
        unexpected = calls - EXPECTED_CALLS
        errors.append(f"managed call inventory differs: missing={dict(missing)!r} unexpected={dict(unexpected)!r}")
    if len(raw_sites) != 1 or raw_sites[0][0] != "connection_runtime.rs":
        errors.append(f"direct SQLite open outside sole audited factory: {raw_sites!r}")
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
