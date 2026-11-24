#!/usr/bin/env python3
"""Verify that repo-relative paths cited in AGENTS.md exist on disk (§ 12)."""

import argparse
from pathlib import Path
import re
import sys

REPO_PREFIXES = (
    "dev/",
    "docs/",
    "scripts/",
    "src/",
    ".github/",
    ".claude/",
    ".codex/",
)
TOP_LEVEL_FILES = {
    "AGENTS.md",
    "Cargo.toml",
    "Cargo.lock",
    "CHANGELOG.md",
    "LICENSE",
    "README.md",
    "mkdocs.yml",
    ".gitignore",
    ".markdownlint-cli2.jsonc",
}
BACKTICK_RE = re.compile(r"`([^`\n]+)`")
BRACE_RE = re.compile(r"\{([^{}]+)\}")
# Paths intentionally gitignored or non-repo (local harness settings, build/tmp dirs).
EXEMPT_EXACT = {
    ".claude/settings.json",
    ".claude/settings.local.json",
    ".git/worktrees/",
}


def expand_braces(token: str) -> list[str]:
    match = BRACE_RE.search(token)
    if not match:
        return [token]
    prefix = token[: match.start()]
    suffix = token[match.end() :]
    out: list[str] = []
    for part in match.group(1).split(","):
        out.extend(expand_braces(f"{prefix}{part.strip()}{suffix}"))
    return out


def extract_repo_paths(markdown: str) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    for lineno, line in enumerate(markdown.splitlines(), 1):
        for raw in BACKTICK_RE.findall(line):
            token = raw.strip()
            if not token or " " in token:
                # Handle `./scripts/foo.sh` style invocations or leading command tokens
                parts = token.split()
                candidates = []
                for p in parts:
                    cleaned = p.lstrip("./") if p.startswith("./") else p
                    if cleaned.startswith(REPO_PREFIXES) or cleaned in TOP_LEVEL_FILES:
                        candidates.append(cleaned)
            else:
                cleaned = token[2:] if token.startswith("./") else token
                candidates = [cleaned] if (cleaned.startswith(REPO_PREFIXES) or cleaned in TOP_LEVEL_FILES) else []

            for cand in candidates:
                cand = cand.rstrip(".,;:)")
                if cand in EXEMPT_EXACT:
                    continue
                if any(ch in cand for ch in ("<", ">", "*")):
                    continue
                for expanded in expand_braces(cand):
                    if expanded in EXEMPT_EXACT:
                        continue
                    found.append((lineno, expanded))
    return found


def check_root(root: Path) -> list[str]:
    agents = root / "AGENTS.md"
    if not agents.is_file():
        return [f"AGENTS.md not found at {agents}"]
    text = agents.read_text(encoding="utf-8")
    lines = text.splitlines()
    errors: list[str] = []
    if len(lines) > 300:
        errors.append(f"AGENTS.md has {len(lines)} lines (limit is 300)")
    for lineno, rel in extract_repo_paths(text):
        target = root / rel.rstrip("/")
        if not target.exists():
            errors.append(f"AGENTS.md:{lineno}: cited path does not resolve: {rel}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="Repository root directory")
    args = parser.parse_args()
    root = Path(args.root).resolve()
    errors = check_root(root)
    if errors:
        for err in errors:
            print(f"FAIL check-agents-md-paths: {err}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
