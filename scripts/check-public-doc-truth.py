#!/usr/bin/env python3
"""Check the small set of current public facts against their machine sources."""

import json
import os
import re
import subprocess
import sys
from pathlib import Path


PUBLIC_DOCS = (
    Path("README.md"),
    Path("docs/index.md"),
    Path("docs/getting-started/index.md"),
    Path("docs/install/python.md"),
    Path("docs/install/typescript.md"),
    Path("docs/install/rust.md"),
    Path("docs/compatibility/index.md"),
)
PLATFORM_BOUNDARY_DOCS = PUBLIC_DOCS[:-1] + (Path("docs/compatibility/index.md"),)
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
    if not DATE_RE.fullmatch(published["published_on"]):
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


def workspace_member_count(cargo_toml: str) -> int:
    match = re.search(r"(?ms)^members\s*=\s*\[(.*?)^\]", cargo_toml)
    if not match:
        fail("Cargo.toml has no workspace members list")
    return len(re.findall(r'^\s*"[^"]+"\s*,?\s*$', match.group(1), re.MULTILINE))


def mentions_published_version(text: str, version: str) -> bool:
    return bool(re.search(rf"(?is)\bv?{re.escape(version)}\b.{{0,100}}\bpublished\b", text))


def has_platform_boundary(text: str) -> bool:
    return "Linux x86_64" in text or "x86_64-unknown-linux-gnu" in text


def main() -> None:
    root = repo_root()
    state = current_published_state(root)
    version = state.get("release")
    if not isinstance(version, str):
        fail("current published release-state must declare release")

    manifest = load_json(root, Path("dev/platform-capabilities.json"))
    published_triples = [
        platform.get("triple")
        for platform in manifest.get("platforms", [])
        if platform.get("status") == "published"
    ]
    if published_triples != ["linux-x64-gnu", "linux-arm64-gnu"]:
        fail(
            "manifest must declare the published Linux x64 and ARM64 artifacts, "
            f"got {published_triples}"
        )

    docs: dict[Path, str] = {}
    for relative in PUBLIC_DOCS:
        try:
            docs[relative] = (root / relative).read_text()
        except OSError as exc:
            fail(f"cannot read public document {relative}: {exc}")

    unpublished = re.compile(
        rf"(?is)\bv?{re.escape(version)}\b.{{0,120}}\b(?:is\s+not|has\s+not|not\s+yet)\s+published\b"
    )
    for relative, text in docs.items():
        if unpublished.search(text.replace("**", "").replace("`", "")):
            fail(f"{relative} says published {version} is unpublished")

    for relative in (Path("README.md"), Path("docs/index.md"), Path("docs/getting-started/index.md")):
        if not mentions_published_version(docs[relative], version):
            fail(f"{relative} lacks a current published {version} statement")

    for relative in PLATFORM_BOUNDARY_DOCS:
        if not has_platform_boundary(docs[relative]):
            fail(f"{relative} lacks the linux-x64 published-platform boundary")

    arm64_positive = re.compile(
        r"(?is)\b(?:linux\s+)?(?:aarch64|arm64)(?:-unknown-linux-gnu)?\b"
        r".{0,80}\b(?:is|are|currently|now)\s+(?:published|available|supported)\b"
    )
    compatibility = docs[Path("docs/compatibility/index.md")]
    if not arm64_positive.search(compatibility):
        fail("docs/compatibility/index.md lacks the published ARM64 native-artifact fact")

    ts_install = docs[Path("docs/install/typescript.md")]
    if "npm install fathomdb@next" not in ts_install:
        fail("docs/install/typescript.md lacks `npm install fathomdb@next`")

    expected_count = workspace_member_count((root / "Cargo.toml").read_text())
    readme = docs[Path("README.md")]
    count_match = re.search(
        r"(?im)^[-*]\s+([A-Za-z]+|\d+)\s+Rust workspace members\b", readme
    )
    if not count_match:
        fail("README.md lacks a Rust workspace member count")
    claimed = count_match.group(1).lower()
    claimed_count = NUMBER_WORDS.get(claimed, int(claimed) if claimed.isdigit() else None)
    if claimed_count != expected_count:
        fail(f"README.md claims {claimed_count} Rust workspace members; Cargo.toml has {expected_count}")

    print(
        f"ok    public-doc-truth: {version} published; "
        f"published native artifacts are {', '.join(published_triples)}"
    )


if __name__ == "__main__":
    main()
