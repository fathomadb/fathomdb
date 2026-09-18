#!/usr/bin/env python3
"""Validate exact lifecycle coverage and ownership for technical design docs."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
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
PROFILE_FIELDS = {
    "path",
    "profile",
    "semantic_authority",
    "implementation_witness",
    "evidence_only",
}
RELATIONSHIP_FIELDS = (
    "semantic_authority",
    "implementation_witness",
    "evidence_only",
)
WITNESS_ROOTS = ("src/", "scripts/", ".github/workflows/")
KEY_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
RELEASE_RE = re.compile(
    r"^(?:cross-release|(?:current|historical|future):[A-Za-z0-9][A-Za-z0-9._+-]*)$"
)
STATUS_RE = re.compile(r"^status\s*:\s*(.*)$")


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


def frontmatter_status(root: Path, path: str) -> str | None:
    lines = (root / path).read_text(encoding="utf-8").splitlines()
    if not lines or lines[0].strip() != "---":
        fail(f"{path}: authority requires opening YAML front matter")
        return None
    try:
        end = lines.index("---", 1)
    except ValueError:
        fail(f"{path}: authority has unterminated YAML front matter")
        return None
    status_matches = [
        match for line in lines[1:end] if (match := STATUS_RE.fullmatch(line))
    ]
    if len(status_matches) != 1:
        fail(f"{path}: authority requires exactly one top-level status")
        return None
    raw = status_matches[0].group(1).strip()
    if not raw or raw[0] in "[{|>" or raw.lower() in {"null", "true", "false", "~"}:
        fail(f"{path}: authority status must be a scalar string")
        return None
    if raw[0] in "\"'":
        if len(raw) < 2 or raw[-1] != raw[0]:
            fail(f"{path}: authority status has malformed quoting")
            return None
        raw = raw[1:-1].strip()
    return raw.lower()


def check_external_authority(root: Path, path: str, role: object) -> bool:
    if path == "dev/requirements.md":
        return True
    if path == "AGENTS.md":
        if role in {"index", "method"}:
            return True
        fail(f"{path}: repository invariants cannot authorize role {role!r}")
        return False
    if path.startswith("dev/adr/") and path.endswith(".md"):
        status = frontmatter_status(root, path)
        if status is not None and status.startswith(("accepted", "locked")):
            return True
        fail(f"{path}: ADR authority must have accepted or locked status")
        return False
    if path.startswith("dev/interfaces/") and path.endswith(".md"):
        status = frontmatter_status(root, path)
        if status == "locked":
            return True
        fail(f"{path}: interface authority must have exactly locked status")
        return False
    fail(f"{path}: unsupported external semantic authority")
    return False


def check_authority_graph(root: Path, records: dict[str, dict[str, object]]) -> bool:
    catalog_path = root / "dev/design/current-owner-authority.json"
    if not catalog_path.is_file():
        fail("missing dev/design/current-owner-authority.json")
        return False
    try:
        catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot parse current-owner authority catalog: {exc}")
        return False
    if not isinstance(catalog, dict) or set(catalog) != {"schema_version", "profiles"}:
        fail("authority catalog must contain exactly schema_version and profiles")
        return False
    if catalog["schema_version"] != 1 or not isinstance(catalog["profiles"], list):
        fail("authority catalog schema_version must be 1 and profiles must be an array")
        return False

    ok = True
    profiles: dict[str, dict[str, object]] = {}
    paths: list[str] = []
    for index, profile in enumerate(catalog["profiles"]):
        label = f"profiles[{index}]"
        if not isinstance(profile, dict) or set(profile) != PROFILE_FIELDS:
            fail(
                f"{label}: profile must contain exactly {', '.join(sorted(PROFILE_FIELDS))}"
            )
            ok = False
            continue
        path = profile["path"]
        if not isinstance(path, str):
            fail(f"{label}: path must be a string")
            ok = False
            continue
        paths.append(path)
        if path in profiles:
            fail(f"duplicate authority profile: {path}")
            ok = False
        profiles[path] = profile
        if profile["profile"] != "current":
            fail(f"{path}: profile must be current")
            ok = False

        relationships: dict[str, list[str]] = {}
        for field in RELATIONSHIP_FIELDS:
            values = profile[field]
            if not isinstance(values, list) or any(
                not isinstance(value, str) or not value for value in values
            ):
                fail(f"{path}: {field} must be an array of nonempty paths")
                ok = False
                relationships[field] = []
                continue
            relationships[field] = values
            if values != sorted(values) or len(values) != len(set(values)):
                fail(f"{path}: {field} must be sorted and duplicate-free")
                ok = False
            for value in values:
                if not existing_repo_path(root, value, field, path):
                    ok = False
        if not relationships.get("semantic_authority"):
            fail(f"{path}: semantic_authority must not be empty")
            ok = False
        if not relationships.get("implementation_witness"):
            fail(f"{path}: implementation_witness must not be empty")
            ok = False
        seen: set[str] = set()
        for field in RELATIONSHIP_FIELDS:
            overlap = seen.intersection(relationships.get(field, []))
            if overlap:
                fail(
                    f"{path}: relationship classes overlap at {', '.join(sorted(overlap))}"
                )
                ok = False
            seen.update(relationships.get(field, []))
        for witness in relationships.get("implementation_witness", []):
            if not witness.startswith(WITNESS_ROOTS):
                fail(f"{path}: implementation witness has unsupported root: {witness}")
                ok = False

    if paths != sorted(paths):
        fail("authority profiles must be sorted by path")
        ok = False
    maintained = {
        path for path, record in records.items() if record.get("class") == "maintained"
    }
    actual = set(paths)
    missing = sorted(maintained - actual)
    extra = sorted(actual - maintained)
    if missing:
        fail(f"maintained designs without current profiles: {', '.join(missing)}")
        ok = False
    if extra:
        fail(f"current profiles for non-maintained designs: {', '.join(extra)}")
        ok = False

    adjacency: dict[str, list[str]] = {}
    for path, profile in profiles.items():
        role = records.get(path, {}).get("role")
        adjacency[path] = []
        authorities = profile.get("semantic_authority")
        if not isinstance(authorities, list):
            continue
        for authority in authorities:
            if not isinstance(authority, str):
                continue
            if authority.startswith("dev/design/"):
                record = records.get(authority)
                if record is None or record.get("class") != "maintained":
                    fail(f"{path}: design authority is not maintained: {authority}")
                    ok = False
                elif authority not in profiles:
                    fail(
                        f"{path}: maintained authority lacks current profile: {authority}"
                    )
                    ok = False
                else:
                    adjacency[path].append(authority)
            elif existing_repo_path(root, authority, "semantic_authority", path):
                if not check_external_authority(root, authority, role):
                    ok = False

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(path: str) -> None:
        nonlocal ok
        if path in visiting:
            fail(f"semantic-authority cycle reaches {path}")
            ok = False
            return
        if path in visited:
            return
        visiting.add(path)
        for target in adjacency.get(path, []):
            visit(target)
        visiting.remove(path)
        visited.add(path)

    for path in sorted(adjacency):
        visit(path)
    return ok


def tracked_design_paths(root: Path) -> set[str] | None:
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--", "dev/design"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or f"git exited {result.returncode}"
        fail(f"cannot enumerate tracked design documents: {detail}")
        return None
    return {
        path
        for path in result.stdout.split("\0")
        if path.startswith("dev/design/") and path.endswith(".md")
    }


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

    expected = tracked_design_paths(root)
    if expected is None:
        return False
    actual = set(paths)
    missing = sorted(expected - actual)
    extra = sorted(actual - expected)
    if missing:
        fail(f"uncataloged design paths: {', '.join(missing)}")
        ok = False
    if extra:
        fail(f"catalog paths without documents: {', '.join(extra)}")
        ok = False

    successor_ok = check_successor_graph(records)
    authority_ok = check_authority_graph(root, records)
    wiring_ok = check_wiring(root)
    return successor_ok and authority_ok and wiring_ok and ok


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
        count = len(tracked_design_paths(root) or ())
        print(f"ok    design-lifecycle: {count} documents")
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
