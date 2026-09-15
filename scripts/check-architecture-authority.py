#!/usr/bin/env python3
"""Validate the current architecture authority and its navigation links."""

from __future__ import annotations

import os
import sys
from pathlib import Path


ARCHITECTURE_GLOB = "fathomdb-data-plane-architecture-v*.md"
SUPERSESSION_BANNER = (
    "**Superseded architecture:** This 0.6.0 snapshot is historical and is "
    "not current architecture authority."
)


def repo_root() -> Path:
    override = os.environ.get("REPO_ROOT")
    if override:
        return Path(override).resolve()
    return Path(__file__).resolve().parent.parent


def fail(message: str) -> None:
    print(f"FAIL architecture-authority: {message}", file=sys.stderr)
    raise SystemExit(1)


def read(path: Path, relative: str) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"cannot read {relative}: {exc}")


def front_matter(text: str, relative: str) -> dict[str, str]:
    lines = text.splitlines()
    if not lines or lines[0] != "---":
        fail(f"{relative} has no YAML front matter")
    values: dict[str, str] = {}
    for line in lines[1:]:
        if line == "---":
            return values
        if ":" in line:
            key, value = line.split(":", 1)
            values[key.strip()] = value.strip().strip('"\'')
    fail(f"{relative} has unterminated YAML front matter")


def main() -> None:
    root = repo_root()
    historical_relative = "dev/architecture.md"
    historical_text = read(root / historical_relative, historical_relative)
    historical = front_matter(historical_text, historical_relative)

    if historical.get("status") != "SUPERSEDED":
        fail(f"{historical_relative} must have status SUPERSEDED")
    successor_relative = historical.get("superseded_by")
    if not successor_relative:
        fail(f"{historical_relative} must declare superseded_by")
    heading = historical_text.find("# Architecture")
    banner = historical_text.find("> **Superseded architecture:**")
    normalized = " ".join(
        line.removeprefix("> ").strip() for line in historical_text.splitlines()
    )
    if (
        banner < 0
        or heading < 0
        or banner > heading
        or SUPERSESSION_BANNER not in normalized
    ):
        fail(f"{historical_relative} must carry the exact supersession banner before its body")

    successor_path = root / successor_relative
    successor_text = read(successor_path, successor_relative)
    successor = front_matter(successor_text, successor_relative)
    if successor.get("status") != "ACTIVE":
        fail(f"{successor_relative} must have status ACTIVE")

    design_dir = root / "dev/design"
    active: list[str] = []
    for path in sorted(design_dir.glob(ARCHITECTURE_GLOB)):
        relative = path.relative_to(root).as_posix()
        if front_matter(read(path, relative), relative).get("status") == "ACTIVE":
            active.append(relative)
    if active != [successor_relative]:
        fail(
            "exactly the declared successor must be ACTIVE; found "
            + (", ".join(active) if active else "none")
        )

    successor_name = Path(successor_relative).name
    for index_relative in ("dev/README.md", "dev/design/README.md"):
        if successor_name not in read(root / index_relative, index_relative):
            fail(f"{index_relative} must link {successor_name}")

    print(f"ok    architecture-authority: {successor_relative}")


if __name__ == "__main__":
    main()
