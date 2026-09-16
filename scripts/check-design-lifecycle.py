#!/usr/bin/env python3
"""Validate exact lifecycle coverage and ownership for technical design docs."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


CLASSES = {
    "maintained",
    "reference",
    "experiment",
    "historical",
    "proposal",
    "deferred",
    "superseded",
}
FIELDS = {"path", "class", "topic", "role", "owner", "release", "successor"}
KEY_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
RELEASE_RE = re.compile(
    r"^(?:cross-release|(?:current|historical|future):[A-Za-z0-9][A-Za-z0-9._+-]*)$"
)


def fail(message: str) -> None:
    print(f"FAIL design-lifecycle: {message}", file=sys.stderr)


def existing_repo_path(root: Path, value: object, field: str, path: str) -> bool:
    if not isinstance(value, str) or not value:
        fail(f"{path}: {field} must be a nonempty repository-relative path")
        return False
    candidate = Path(value)
    if candidate.is_absolute() or ".." in candidate.parts:
        fail(f"{path}: {field} must stay within the repository: {value!r}")
        return False
    if not (root / candidate).is_file():
        fail(f"{path}: {field} is not an existing regular file: {value}")
        return False
    return True


def active_lines(path: Path) -> list[str]:
    return [
        line.strip()
        for line in path.read_text().splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def ci_job(lines: list[str], name: str) -> list[str]:
    header = f"  {name}:"
    try:
        start = lines.index(header)
    except ValueError:
        return []
    end = len(lines)
    for index in range(start + 1, len(lines)):
        if re.fullmatch(r"  [A-Za-z0-9_-]+:", lines[index]):
            end = index
            break
    return lines[start:end]


def check_wiring(root: Path) -> bool:
    ok = True
    local = root / "scripts/agent-lint-md.sh"
    ci = root / ".github/workflows/ci.yml"
    local_call = (
        'run_capped check-design-lifecycle "$SCRIPT_DIR/check-design-lifecycle.py"'
    )
    if not local.is_file() or local_call not in active_lines(local):
        fail("scripts/agent-lint-md.sh does not run check-design-lifecycle.py")
        ok = False
    ci_lines = ci.read_text().splitlines() if ci.is_file() else []
    markdownlint = ci_job(ci_lines, "markdownlint")
    docs_only = "if: needs.changes.outputs.docs_only == 'true'"
    ci_call = "run: python3 scripts/check-design-lifecycle.py"
    active_markdownlint = [
        line.strip()
        for line in markdownlint
        if line.strip() and not line.lstrip().startswith("#")
    ]
    if docs_only not in active_markdownlint or ci_call not in active_markdownlint:
        fail(
            ".github/workflows/ci.yml markdownlint docs-only job does not run "
            "check-design-lifecycle.py"
        )
        ok = False
    return ok


def check_successor_graph(records: dict[str, dict[str, object]]) -> bool:
    ok = True
    for path, record in records.items():
        if record.get("class") != "superseded":
            continue
        seen = {path}
        current = record
        while current.get("class") == "superseded":
            successor = current.get("successor")
            if not isinstance(successor, str):
                break
            if successor in seen:
                fail(f"{path}: supersession cycle reaches {successor}")
                ok = False
                break
            seen.add(successor)
            next_record = records.get(successor)
            if next_record is None:
                break
            current = next_record
    return ok


def validate(root: Path) -> bool:
    catalog_path = root / "dev/design/document-lifecycle.json"
    if not catalog_path.is_file():
        fail("missing dev/design/document-lifecycle.json")
        return False

    try:
        catalog = json.loads(catalog_path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot parse catalog: {exc}")
        return False

    if not isinstance(catalog, dict) or catalog.get("schema_version") != 1:
        fail("catalog schema_version must be 1")
        return False
    documents = catalog.get("documents")
    if not isinstance(documents, list):
        fail("catalog documents must be an array")
        return False

    ok = True
    paths: list[str] = []
    records: dict[str, dict[str, object]] = {}
    maintained_keys: set[tuple[str, str]] = set()
    for index, record in enumerate(documents):
        label = f"documents[{index}]"
        if not isinstance(record, dict):
            fail(f"{label}: record must be an object")
            ok = False
            continue
        missing_fields = FIELDS - set(record)
        if missing_fields:
            fail(f"{label}: missing fields: {', '.join(sorted(missing_fields))}")
            ok = False
            continue

        path = record["path"]
        if not isinstance(path, str):
            fail(f"{label}: path must be a string")
            ok = False
            continue
        paths.append(path)
        records[path] = record
        if not path.startswith("dev/design/") or not path.endswith(".md"):
            fail(f"{path}: catalog path must match dev/design/**/*.md")
            ok = False
        if not existing_repo_path(root, path, "path", path):
            ok = False

        lifecycle_class = record["class"]
        if lifecycle_class not in CLASSES:
            fail(f"{path}: unknown class {lifecycle_class!r}")
            ok = False
        for field in ("topic", "role"):
            value = record[field]
            if not isinstance(value, str) or not KEY_RE.fullmatch(value):
                fail(f"{path}: {field} must be lower-kebab-case")
                ok = False
        if not existing_repo_path(root, record["owner"], "owner", path):
            ok = False

        release = record["release"]
        if not isinstance(release, str) or not RELEASE_RE.fullmatch(release):
            fail(f"{path}: invalid release relevance {release!r}")
            ok = False

        successor = record["successor"]
        if successor is not None and not existing_repo_path(
            root, successor, "successor", path
        ):
            ok = False
        if lifecycle_class == "superseded":
            if successor is None:
                fail(f"{path}: superseded records require a successor")
                ok = False
            elif record["owner"] != successor:
                fail(f"{path}: superseded owner must equal successor")
                ok = False

        if lifecycle_class == "maintained":
            key = (record["topic"], record["role"])
            if key in maintained_keys:
                fail(f"{path}: duplicate maintained topic/role {key[0]}/{key[1]}")
                ok = False
            maintained_keys.add(key)

    if paths != sorted(paths):
        fail("catalog documents must be sorted by path")
        ok = False
    duplicates = sorted({path for path in paths if paths.count(path) > 1})
    if duplicates:
        fail(f"duplicate catalog paths: {', '.join(duplicates)}")
        ok = False

    expected = {
        path.relative_to(root).as_posix()
        for path in (root / "dev/design").rglob("*.md")
    }
    actual = set(paths)
    missing = sorted(expected - actual)
    extra = sorted(actual - expected)
    if missing:
        fail(f"uncataloged design paths: {', '.join(missing)}")
        ok = False
    if extra:
        fail(f"catalog paths without documents: {', '.join(extra)}")
        ok = False

    return check_successor_graph(records) and check_wiring(root) and ok


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
    )
    args = parser.parse_args()
    root = args.repo_root.resolve()
    if validate(root):
        count = len(list((root / "dev/design").rglob("*.md")))
        print(f"ok    design-lifecycle: {count} documents")
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
