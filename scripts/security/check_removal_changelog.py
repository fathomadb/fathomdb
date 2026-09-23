#!/usr/bin/env python3
"""AC-050c removal-detect linter.

Scans a unified diff for removed public API symbols across the Rust,
Python, and TypeScript surfaces and asserts that every removed symbol
is announced in CHANGELOG.md under a heading containing the word
``Removed`` (case-insensitive).

Exit codes:
    0 — every removal documented (or none found).
    1 — one or more removals missing from CHANGELOG.
    2 — invocation error (bad args, missing CHANGELOG).
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


SCAN_PREFIXES = ("src/rust/crates/", "src/python/", "src/ts/")

RUST_PUBLIC = re.compile(
    r"^\s*pub(?:\([^)]+\))?\s+(?:unsafe\s+|async\s+|const\s+|extern\s+(?:\"[^\"]+\"\s+)?)*"
    r"(fn|struct|enum|trait|const|type|static|mod)\s+([A-Za-z_][A-Za-z0-9_]*)"
)
# Python: top-level def/class. Indented members are not part of the
# public surface (their owning class is what counts).
PY_PUBLIC = re.compile(r"^(def|class)\s+([A-Za-z_][A-Za-z0-9_]*)")
TS_PUBLIC = re.compile(
    r"^\s*export\s+(?:default\s+)?(?:async\s+)?"
    r"(function|class|const|let|var|type|interface|enum)\s+([A-Za-z_$][A-Za-z0-9_$]*)"
)
RUST_PUBLIC_USE = re.compile(r"^\s*pub\s+use\s+(.+);\s*$", re.DOTALL)
# A cfg-gated `pub use` does not unconditionally restore a removed public
# name — the re-export only exists when the cfg predicate holds, so it must
# not cancel the removal (fail closed). Matched against attribute lines that
# immediately precede a `pub use` in the added-lines run.
RUST_CFG_ATTR = re.compile(r"^#\s*\[\s*cfg(?:_attr)?\s*\(")
RUST_ATTR_LINE = re.compile(r"^#!?\s*\[.*\]\s*$")


@dataclass(frozen=True)
class Removal:
    path: str
    kind: str  # rust|python|ts
    symbol_kind: str  # fn|class|...
    name: str


def _classify(path: str) -> str | None:
    # Test files are NOT public API — a renamed/removed test function is not a
    # consumer-visible removal. Excluding any `tests/` directory keeps the
    # removal-detect gate scoped to the shipped public surface and stops false
    # positives on test churn (e.g. the Slice-25 `test_surface.py` rewrite).
    if "/tests/" in path:
        return None
    if path.startswith("src/rust/crates/") and path.endswith(".rs"):
        return "rust"
    if path.startswith("src/python/") and path.endswith(".py"):
        return "python"
    if path.startswith("src/ts/") and (path.endswith(".ts") or path.endswith(".tsx")):
        return "ts"
    return None


def _scan_line(kind: str, line: str) -> tuple[str, str] | None:
    if kind == "rust":
        m = RUST_PUBLIC.match(line)
    elif kind == "python":
        m = PY_PUBLIC.match(line)
    elif kind == "ts":
        m = TS_PUBLIC.match(line)
    else:
        return None
    return (m.group(1), m.group(2)) if m else None


def _split_use_tree(value: str) -> list[str]:
    parts: list[str] = []
    depth = 0
    start = 0
    for index, char in enumerate(value):
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(value[start:index])
            start = index + 1
    parts.append(value[start:])
    return parts


def _rust_reexport_names(statement: str) -> set[str]:
    """Return explicit crate-root names exported by one ``pub use``.

    Glob exports stay fail-closed because their names cannot be established
    from the diff alone. Aliases contribute only the exported alias.
    """

    match = RUST_PUBLIC_USE.match(statement)
    if match is None:
        return set()

    def collect(tree: str, prefix: str = "") -> set[str]:
        tree = tree.strip()
        if not tree or tree == "*":
            return set()
        if "{" in tree:
            open_index = tree.find("{")
            if not tree.endswith("}"):
                return set()
            nested_prefix = tree[:open_index].strip().removesuffix("::")
            joined_prefix = "::".join(part for part in (prefix, nested_prefix) if part)
            names: set[str] = set()
            for item in _split_use_tree(tree[open_index + 1 : -1]):
                item = item.strip()
                if item == "self":
                    name = joined_prefix.rsplit("::", 1)[-1]
                    if name and name not in {"crate", "self", "super"}:
                        names.add(name)
                else:
                    names.update(collect(item, joined_prefix))
            return names

        alias_parts = re.split(r"\s+as\s+", tree, maxsplit=1)
        if len(alias_parts) == 2:
            alias = alias_parts[1].strip()
            return {alias} if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", alias) else set()
        full_path = "::".join(part for part in (prefix, tree) if part)
        name = full_path.rsplit("::", 1)[-1]
        if name in {"*", "crate", "self", "super"}:
            return set()
        return {name} if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name) else set()

    return collect(match.group(1))


def parse_diff(diff_text: str) -> tuple[set[Removal], set[tuple[str, str, str]]]:
    """Returns (removals, additions) keyed for cancellation matching."""
    removals: set[Removal] = set()
    # Additions keyed (path, kind, name) so a rename WITHIN the same file
    # at the same path cancels out a removal (true delete only when no
    # corresponding + line in the same path).
    additions: set[tuple[str, str, str]] = set()

    current_path: str | None = None
    current_kind: str | None = None
    # Added-side `pub use` accumulator (C-1 adds cfg-gate tracking: a name
    # exported only under `#[cfg(...)]`/`#[cfg_attr(...)]` must not cancel a
    # removal of the same name).
    rust_public_use: list[str] = []
    rust_public_use_cfg_gated = False
    rust_cfg_pending = False
    # Removed-side `pub use` accumulator (C-2): a removed re-export is itself
    # a removal of its exported names, subject to the same cancellation via
    # `real_removals` if the name re-appears (unconditionally) in `additions`.
    rust_public_use_removed: list[str] = []
    for raw in diff_text.splitlines():
        if raw.startswith("+++ "):
            rust_public_use = []
            rust_public_use_cfg_gated = False
            rust_cfg_pending = False
            rust_public_use_removed = []
            # New-path marker is more reliable than the diff header for
            # in-file renames; both old and new paths are the same in
            # normal removals.
            spec = raw[4:].strip()
            spec = spec.removeprefix("b/")
            current_path = spec if spec != "/dev/null" else current_path
            current_kind = _classify(current_path) if current_path else None
            continue
        if raw.startswith("--- "):
            rust_public_use = []
            rust_public_use_cfg_gated = False
            rust_cfg_pending = False
            rust_public_use_removed = []
            spec = raw[4:].strip()
            spec = spec.removeprefix("a/")
            if spec != "/dev/null":
                current_path = spec
                current_kind = _classify(current_path) if current_path else None
            continue
        if raw.startswith("@@"):
            rust_public_use = []
            rust_public_use_cfg_gated = False
            rust_cfg_pending = False
            rust_public_use_removed = []
            continue

        is_added_line = raw.startswith("+") and not raw.startswith("+++")
        is_removed_line = raw.startswith("-") and not raw.startswith("--")
        # Contiguity resets: both accumulators only span an unbroken run of
        # their own +/- lines respectively.
        if not is_added_line:
            rust_public_use = []
            rust_public_use_cfg_gated = False
            rust_cfg_pending = False
        if not is_removed_line:
            rust_public_use_removed = []

        if current_kind is None or current_path is None:
            continue
        # Skip hunk headers and diff metadata.
        if raw.startswith("+++") or raw.startswith("---"):
            continue
        if is_removed_line:
            line = raw[1:]
            match = _scan_line(current_kind, line)
            if match:
                symbol_kind, name = match
                removals.add(
                    Removal(
                        path=current_path,
                        kind=current_kind,
                        symbol_kind=symbol_kind,
                        name=name,
                    )
                )
            if current_kind == "rust":
                if rust_public_use_removed:
                    rust_public_use_removed.append(line)
                elif re.match(r"^\s*pub\s+use\b", line):
                    rust_public_use_removed = [line]
                if rust_public_use_removed and ";" in line:
                    statement = "\n".join(rust_public_use_removed)
                    for name in _rust_reexport_names(statement):
                        removals.add(
                            Removal(
                                path=current_path,
                                kind=current_kind,
                                symbol_kind="use",
                                name=name,
                            )
                        )
                    rust_public_use_removed = []
        elif is_added_line:
            line = raw[1:]
            match = _scan_line(current_kind, line)
            if match:
                additions.add((current_path, current_kind, match[1]))
            if current_kind == "rust":
                stripped = line.strip()
                if RUST_CFG_ATTR.match(stripped):
                    rust_cfg_pending = True
                elif RUST_ATTR_LINE.match(stripped):
                    # Non-cfg attribute: keep any pending cfg gate alive
                    # through stacked attributes above the `pub use`.
                    pass
                else:
                    if rust_public_use:
                        rust_public_use.append(line)
                    elif re.match(r"^\s*pub\s+use\b", line):
                        rust_public_use = [line]
                        rust_public_use_cfg_gated = rust_cfg_pending
                    if rust_public_use and ";" in line:
                        statement = "\n".join(rust_public_use)
                        if not rust_public_use_cfg_gated:
                            for name in _rust_reexport_names(statement):
                                additions.add((current_path, current_kind, name))
                        rust_public_use = []
                        rust_public_use_cfg_gated = False
                    rust_cfg_pending = False

    return removals, additions


def real_removals(removals: set[Removal], additions: set[tuple[str, str, str]]) -> list[Removal]:
    """A removal cancels if the same symbol name re-appears in the same
    file, including through an explicit Rust ``pub use``. Cross-file moves
    without that same-path re-export still count as removals because consumers
    may import by full path.
    """
    out = []
    for r in removals:
        if (r.path, r.kind, r.name) in additions:
            continue
        out.append(r)
    return sorted(out, key=lambda r: (r.path, r.name))


def changelog_documents(changelog: Path, removed: Iterable[Removal]) -> list[Removal]:
    text = changelog.read_text(encoding="utf-8")
    # Split CHANGELOG into sections per `##` heading and only consider
    # text under a heading containing "Removed" (case-insensitive). This
    # bounds the match to the announcement region.
    sections = re.split(r"^(#{1,6}\s+.*)$", text, flags=re.MULTILINE)
    removed_text_parts: list[str] = []
    # `sections` is [pre, heading1, body1, heading2, body2, ...]
    for i in range(1, len(sections), 2):
        heading = sections[i]
        body = sections[i + 1] if i + 1 < len(sections) else ""
        if re.search(r"removed", heading, flags=re.IGNORECASE):
            removed_text_parts.append(body)
    haystack = "\n".join(removed_text_parts)
    undocumented = []
    for r in removed:
        if not re.search(r"\b" + re.escape(r.name) + r"\b", haystack):
            undocumented.append(r)
    return undocumented


def load_diff(args: argparse.Namespace) -> str:
    if args.diff_file:
        return Path(args.diff_file).read_text(encoding="utf-8")
    cmd = ["git", "-C", str(args.repo_root), "diff", f"{args.base}..{args.head}", "--"]
    for prefix in SCAN_PREFIXES:
        cmd.append(f"{prefix}**")
    completed = subprocess.run(cmd, check=False, capture_output=True, text=True)
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr)
        sys.exit(2)
    return completed.stdout


def main(argv: list[str]) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--base", default="v0.6.1", help="base git ref")
    p.add_argument("--head", default="HEAD", help="head git ref")
    p.add_argument("--diff-file", default=None, help="read diff from a file instead of git")
    p.add_argument("--changelog", default=None, help="CHANGELOG.md path (default: <repo>/CHANGELOG.md)")
    p.add_argument("--repo-root", default=None, help="repository root (default: git rev-parse)")
    args = p.parse_args(argv)

    if args.repo_root is None:
        try:
            args.repo_root = subprocess.check_output(
                ["git", "rev-parse", "--show-toplevel"], text=True
            ).strip()
        except subprocess.CalledProcessError:
            sys.stderr.write("not inside a git repo; pass --repo-root\n")
            return 2

    changelog_path = Path(args.changelog) if args.changelog else Path(args.repo_root) / "CHANGELOG.md"
    if not changelog_path.exists():
        sys.stderr.write(f"CHANGELOG.md missing at {changelog_path}\n")
        return 2

    diff_text = load_diff(args)
    removals, additions = parse_diff(diff_text)
    truly_removed = real_removals(removals, additions)
    undocumented = changelog_documents(changelog_path, truly_removed)

    if undocumented:
        sys.stderr.write("AC-050c: removed public symbols missing from CHANGELOG Removed section:\n")
        for r in undocumented:
            sys.stderr.write(f"  {r.path}: {r.symbol_kind} {r.name}\n")
        return 1

    if truly_removed:
        sys.stdout.write(f"AC-050c OK: {len(truly_removed)} removals all documented.\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
