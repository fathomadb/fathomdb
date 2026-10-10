#!/usr/bin/env python3
"""Check the small set of current public facts against their machine sources."""

import json
import os
import re
import subprocess
import sys
from datetime import date
from pathlib import Path


PUBLIC_DOCS = (
    Path("README.md"),
    Path("docs/index.md"),
    Path("docs/getting-started/index.md"),
    Path("docs/install/python.md"),
    Path("docs/install/typescript.md"),
    Path("docs/install/rust.md"),
    Path("docs/compatibility/index.md"),
    Path("docs/concepts/index.md"),
    Path("docs/operations/index.md"),
    Path("docs/reference/index.md"),
    Path("docs/reference/python-api.md"),
    Path("docs/reference/typescript-api.md"),
    Path("docs/reference/rust-api.md"),
    Path("docs/reference/cli.md"),
    Path("docs/embedder.md"),
    Path("src/ts/README.md"),
)
CURRENT_RELEASE_DOCS = PUBLIC_DOCS
PLATFORM_BOUNDARY_DOCS = (
    Path("README.md"),
    Path("docs/index.md"),
    Path("docs/getting-started/index.md"),
    Path("docs/install/python.md"),
    Path("docs/install/typescript.md"),
    Path("docs/install/rust.md"),
    Path("docs/compatibility/index.md"),
    Path("src/ts/README.md"),
)
NUMBER_WORDS = {
    "zero": 0,
    "one": 1,
    "two": 2,
    "three": 3,
    "four": 4,
    "five": 5,
    "six": 6,
    "seven": 7,
    "eight": 8,
    "nine": 9,
    "ten": 10,
    "eleven": 11,
    "twelve": 12,
}
STATE_RE = re.compile(r"dev/plans/release-state-(\d+)\.(\d+)\.(\d+)\.json$")
COMMIT_RE = re.compile(r"[0-9a-f]{40}$")
DATE_RE = re.compile(r"\d{4}-\d{2}-\d{2}$")


def repo_root() -> Path:
    override = os.environ.get("REPO_ROOT")
    if override:
        return Path(override).resolve()
    return Path(__file__).resolve().parent.parent


def fail(message: str) -> None:
    print(f"FAIL public-doc-truth: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_json(root: Path, relative: Path) -> dict:
    try:
        return json.loads((root / relative).read_text())
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot read {relative}: {exc}")


def tracked_release_states(root: Path) -> list[Path]:
    """Return release-state records from Git's tracked-file index only."""
    result = subprocess.run(
        ["git", "-C", str(root), "--no-optional-locks", "ls-files", "-z", "--", "dev/plans/release-state-*.json"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode:
        detail = result.stderr.decode("utf-8", "replace").strip()
        fail(f"cannot list tracked release-state records: {detail}")
    return [Path(path.decode("utf-8")) for path in result.stdout.split(b"\0") if path]


def published_record(state: dict, version: str, relative: Path) -> dict | None:
    """Validate the canonical nullable publication record for one state."""
    published = state.get("published")
    if published is None:
        release_kind = state.get("release_kind")
        if isinstance(release_kind, str) and "publication complete" in release_kind.casefold():
            fail(f"{relative} declares publication complete without a published receipt")
        return None
    if not isinstance(published, dict):
        fail(f"{relative} has a non-object published record")
    required = ("tag", "tag_commit", "published_on", "npm_dist_tag")
    if any(not isinstance(published.get(field), str) or not published[field] for field in required):
        fail(f"{relative} has an incomplete published record")
    if published["tag"] != f"v{version}":
        fail(f"{relative} published tag {published['tag']!r} does not match v{version}")
    if not COMMIT_RE.fullmatch(published["tag_commit"]):
        fail(f"{relative} published tag_commit is not a full lowercase commit SHA")
    try:
        parsed_date = date.fromisoformat(published["published_on"])
    except ValueError:
        parsed_date = None
    if not DATE_RE.fullmatch(published["published_on"]) or parsed_date is None:
        fail(f"{relative} published_on is not an ISO date")
    return published


def current_published_state(root: Path) -> dict:
    """Return the newest valid published state from tracked canonical records."""
    candidates: list[tuple[tuple[int, int, int], dict]] = []
    for relative in tracked_release_states(root):
        match = STATE_RE.fullmatch(relative.as_posix())
        if not match:
            fail(f"tracked release-state path has an invalid name: {relative}")
        version = ".".join(match.groups())
        state = load_json(root, relative)
        version = state.get("release")
        if version != ".".join(match.groups()):
            fail(f"{relative} release does not match its filename")
        published = published_record(state, version, relative)
        if published is not None:
            candidates.append((tuple(int(part) for part in match.groups()), state))
    if not candidates:
        fail("no release-state record declares publication complete")
    return max(candidates, key=lambda item: item[0])[1]


def documentation_target_state(root: Path, published_state: dict) -> dict:
    result = subprocess.run(
        [sys.executable, str(root / "scripts/release-current.py")],
        cwd=root,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode:
        fail(f"cannot resolve canonical active release: {result.stderr.strip()}")
    if not result.stdout.strip():
        return published_state
    fields = result.stdout.strip().split("\t")
    if len(fields) != 3:
        fail("canonical active release did not return a release/board/state tuple")
    version, _, state_path = fields
    state = load_json(root, Path(state_path))
    if "public_docs_target_release" not in state:
        return published_state
    target = state["public_docs_target_release"]
    if not isinstance(target, str) or target != version or target != state.get("release"):
        fail(f"{state_path} public_docs_target_release must equal its own release {version}")
    return state


def asserts_published_version(text: str, version: str) -> bool:
    normalized = text.replace("**", "").replace("`", "")
    v = rf"v?{re.escape(version)}\b"
    return bool(
        re.search(rf"(?i)\bpublished\s+(?:FathomDB\s+)?{v}", normalized)
        or re.search(
            rf"(?i)\b{v}\s+(?:(?:release|CLI|packages?|artifacts?)\s+)?"
            r"(?:is|was|has\s+been)\s+(?:already\s+)?published\b",
            normalized,
        )
    )


def workspace_member_count(cargo_toml: str) -> int:
    match = re.search(r"(?ms)^members\s*=\s*\[(.*?)^\]", cargo_toml)
    if not match:
        fail("Cargo.toml has no workspace members list")
    return len(re.findall(r'^\s*"[^"]+"\s*,?\s*$', match.group(1), re.MULTILINE))


def missing_platform_boundaries(text: str) -> list[str]:
    normalized = re.sub(r"\s+", " ", text.replace("`", ""))
    required = {
        "Linux x64": r"(?:Linux x86_64|x86_64-unknown-linux-gnu)",
        "Linux arm64": r"(?:Linux.{0,100}(?:AArch64|aarch64-unknown-linux-gnu))",
        "macOS x64": r"(?:macOS.{0,100}(?:x64|x86_64-apple-darwin))",
        "macOS arm64": r"(?:macOS.{0,100}(?:arm64|aarch64-apple-darwin))",
        "Windows x64": r"(?:Windows.{0,100}(?:x64|x86_64-pc-windows-msvc))",
    }
    return [label for label, pattern in required.items() if re.search(pattern, normalized) is None]


def main() -> None:
    root = repo_root()
    published_state = current_published_state(root)
    if published_state.get("schema_version") != 34:
        fail("current published release state must declare schema 34")
    state = documentation_target_state(root, published_state)
    version = state.get("release")
    if not isinstance(version, str):
        fail("documentation target state must declare release")

    manifest = load_json(root, Path("dev/platform-capabilities.json"))
    published_triples = [
        platform.get("triple")
        for platform in manifest.get("platforms", [])
        if platform.get("status") == "published"
    ]
    expected_published = [
        "linux-x64-gnu",
        "linux-arm64-gnu",
        "darwin-x64",
        "darwin-arm64",
        "win32-x64-msvc",
    ]
    if published_triples != expected_published:
        fail(
            "manifest must declare the five published native artifacts, "
            f"got {published_triples}"
        )
    if state.get("schema_version") != 34:
        fail("documentation target state must declare schema 34")

    docs: dict[Path, str] = {}
    for relative in PUBLIC_DOCS:
        try:
            docs[relative] = (root / relative).read_text()
        except OSError as exc:
            fail(f"cannot read public document {relative}: {exc}")

    published_version = published_state["release"]
    unpublished = re.compile(
        rf"(?is)\bv?{re.escape(published_version)}\b"
        r"(?:(?!\bv?\d+\.\d+\.\d+\b).){0,120}"
        r"\b(?:is\s+not|has\s+not|not\s+yet)\s+published\b"
    )
    pending = state.get("published") is None
    for relative, text in docs.items():
        normalized = text.replace("**", "").replace("`", "")
        if unpublished.search(normalized):
            fail(f"{relative} says published {published_version} is unpublished")
        if pending and asserts_published_version(text, version):
            fail(f"{relative} says unpublished documentation target {version} is published")

    for relative in CURRENT_RELEASE_DOCS:
        text = re.sub(r"\]\([^)]*\)", "]", docs[relative])
        text = re.sub(r"https?://\S+|<!--.*?-->", "", text, flags=re.DOTALL)
        if not re.search(rf"\bv?{re.escape(version)}\b", text):
            fail(f"{relative} lacks documentation target {version}")

    for relative in PLATFORM_BOUNDARY_DOCS:
        missing = missing_platform_boundaries(docs[relative])
        if missing:
            fail(f"{relative} lacks published-platform boundaries: {', '.join(missing)}")

    compatibility = docs[Path("docs/compatibility/index.md")]
    for target in (
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc",
    ):
        row = re.compile(rf"(?m)^\|[^\n]*`{re.escape(target)}`[^\n]*\|\s*\*\*yes\*\*")
        if row.search(compatibility) is None:
            fail(f"docs/compatibility/index.md lacks the published {target} table row")
    if "SCHEMA_VERSION` to **34**" not in compatibility or "no public automatic migration" not in compatibility.lower():
        fail("docs/compatibility/index.md lacks the schema-34 fresh-database boundary")

    ts_install = docs[Path("docs/install/typescript.md")]
    exact_commands = (f"npm install --save-exact fathomdb@{version}", f"npm install fathomdb@{version}")
    dist_tag = state.get("published", {}).get("npm_dist_tag") if not pending else None
    if not any(command in ts_install for command in exact_commands):
        if pending or not dist_tag or f"npm install fathomdb@{dist_tag}" not in ts_install:
            fail(f"docs/install/typescript.md lacks exact {version} install or recorded tag {dist_tag}")

    cargo_toml = (root / "Cargo.toml").read_text()
    expected_count = workspace_member_count(cargo_toml)
    members_match = re.search(r"(?ms)^members\s*=\s*\[(.*?)^\]", cargo_toml)
    assert members_match is not None
    expected_paths = {
        f"{member}/Cargo.toml"
        for member in re.findall(r'^\s*"([^"]+)"\s*,?\s*$', members_match.group(1), re.MULTILINE)
    }
    try:
        contributor_map = (root / "docs/generated/contributor-map.md").read_text()
    except OSError as exc:
        fail(f"cannot read docs/generated/contributor-map.md: {exc}")
    section = re.search(r"(?ms)^## Rust workspace\n(.*?)(?=^## |\Z)", contributor_map)
    paths = re.findall(r"\[([^\]]+/Cargo.toml)\]\(", section.group(1) if section else "")
    if len(paths) != expected_count or set(paths) != expected_paths:
        fail("docs/generated/contributor-map.md does not match Cargo.toml workspace members")

    readme = docs[Path("README.md")]
    for count_match in re.finditer(r"(?im)^[-*]\s+([A-Za-z]+|\d+)\s+Rust workspace members\b", readme):
        claimed = count_match.group(1).lower()
        claimed_count = NUMBER_WORDS.get(claimed, int(claimed) if claimed.isdigit() else None)
        if claimed_count != expected_count:
            fail(f"README.md claims {claimed_count} Rust workspace members; Cargo.toml has {expected_count}")

    print(
        f"ok    public-doc-truth: documentation target {version}; "
        f"newest published receipt {published_version}; "
        f"native artifacts are {', '.join(published_triples)}"
    )


if __name__ == "__main__":
    main()
