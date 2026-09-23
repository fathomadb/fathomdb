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
# Known fail-closed false positives — both cost a manual CHANGELOG entry,
# never a silent miss, so they're accepted rather than special-cased:
#   - `#[cfg_attr(pred, doc(...))]` is treated as gating even when the
#     cfg_attr's inner attribute (e.g. `doc(...)`) never affects whether the
#     item is actually compiled in.
#   - `#[cfg(...)] pub use m::Foo;` written as ONE physical added line (attr
#     and `pub use` sharing a line) is never credited as a re-export at all:
#     the whole line is classified as an attribute line, so the trailing
#     `pub use` text on it is never fed into the re-export accumulator.
RUST_ATTR_OPEN = re.compile(r"^#!?\[")


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


_PUB_USE_LINE_START = re.compile(r"^\s*pub\s+use\b")
_CHANGED_MARK = "\x00"


def _mark_line(content: str, changed: bool) -> str:
    """Prefix `content` (one diff line's text, +/-/space marker stripped)
    with `_CHANGED_MARK` at the start and after every `,`/`{` that has more
    text on the same line, so a flat multi-name line (`A, B, C,` — the common rustfmt shape
    once several re-exported names share a line) attributes the mark to
    EACH name it carries, not just the first. A no-op for context lines.
    """
    if not changed:
        return content
    # Only a separator followed by more text on this line starts a name on
    # this line; marking after a trailing `,`/`{` would leak the mark onto
    # the first name of the next (possibly unchanged context) line.
    return _CHANGED_MARK + re.sub(r"([,{])(?=\s*\S)", r"\1" + _CHANGED_MARK, content)


def _rust_reexport_marked_names(marked_statement: str) -> set[str]:
    """Like `_rust_reexport_names`, but `marked_statement` is a full
    ``pub use ...;`` statement reconstructed line-by-line via `_mark_line`;
    only names whose contributing text carries `_CHANGED_MARK` — i.e. came
    from at least one changed (`+`/`-`) diff line rather than purely
    unchanged context — are returned.
    """

    # A changed opening line carries a leading mark; names on it are marked
    # after their `{`/`,` separators, so the leading one is not needed.
    match = RUST_PUBLIC_USE.match(marked_statement.lstrip(_CHANGED_MARK))
    if match is None:
        return set()

    def collect(tree: str, prefix: str = "") -> set[str]:
        tree = tree.strip()
        bare = tree.replace(_CHANGED_MARK, "")
        if not bare or bare == "*":
            return set()
        if "{" in bare:
            open_index = tree.find("{")
            trimmed = tree.rstrip(_CHANGED_MARK)
            if not trimmed.endswith("}"):
                return set()
            close_index = len(trimmed) - 1
            nested_prefix = tree[:open_index].replace(_CHANGED_MARK, "").strip().removesuffix("::")
            joined_prefix = "::".join(part for part in (prefix, nested_prefix) if part)
            names: set[str] = set()
            for item in _split_use_tree(tree[open_index + 1 : close_index]):
                item = item.strip()
                marked_item = _CHANGED_MARK in item
                clean_item = item.replace(_CHANGED_MARK, "").strip()
                if clean_item == "self":
                    name = joined_prefix.rsplit("::", 1)[-1]
                    if marked_item and name and name not in {"crate", "self", "super"}:
                        names.add(name)
                else:
                    names.update(collect(item, joined_prefix))
            return names

        marked = _CHANGED_MARK in tree
        clean = tree.replace(_CHANGED_MARK, "").strip()
        alias_parts = re.split(r"\s+as\s+", clean, maxsplit=1)
        if len(alias_parts) == 2:
            alias = alias_parts[1].strip()
            return {alias} if marked and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", alias) else set()
        full_path = "::".join(part for part in (prefix, clean) if part)
        name = full_path.rsplit("::", 1)[-1]
        if name in {"*", "crate", "self", "super"}:
            return set()
        return {name} if marked and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name) else set()

    return collect(match.group(1))


def _side_pub_use_step(
    content: str,
    changed: bool,
    active: bool,
    lines: list[str],
    start_on_changed: bool = False,
) -> tuple[bool, list[str], str | None]:
    """Advance a ONE-SIDED (old: context+removed, or new: context+added)
    ``pub use { ... }`` reconstruction (fix-2 / V-2) by one line of that
    side's view, and return the completed marked statement text once a
    terminating ``;`` is seen (else None).

    Starts on a CONTEXT opening line, or also on a changed one when
    ``start_on_changed`` (old side only). The added side must not start on
    an added opening line: the cfg-aware `rust_public_use` accumulator owns
    that case, and double-processing it would bypass V-1's cfg-gate
    fail-closed semantics. The old side has no cfg concern, and starting on
    a removed opening line covers a removed block whose inner lines mix
    removed and context lines, which the contiguous removed-side
    accumulator resets on.
    """
    if not active:
        if (changed and not start_on_changed) or not _PUB_USE_LINE_START.match(content):
            return active, lines, None
        active = True
        lines = []
    marked = _mark_line(content, changed)
    lines = lines + [marked]
    if ";" in content:
        return False, [], "\n".join(lines)
    return active, lines, None


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
    # removal of the same name). `rust_attr_*` track a possibly multi-line
    # attribute (fix-1 / V-1): bracket depth stays open across rustfmt's
    # wrapped `#[cfg(any(\n ...\n))]` form, and `///`/`//!`/`//`/blank added
    # lines between a cfg attribute and the `pub use` it guards keep
    # `rust_cfg_pending` alive instead of resetting it.
    rust_public_use: list[str] = []
    rust_public_use_cfg_gated = False
    rust_cfg_pending = False
    rust_attr_open = False
    rust_attr_text = ""
    rust_attr_depth = 0
    # Removed-side `pub use` accumulator (C-2): a removed re-export is itself
    # a removal of its exported names, subject to the same cancellation via
    # `real_removals` if the name re-appears (unconditionally) in `additions`.
    rust_public_use_removed: list[str] = []
    # One-sided (context + removed, or context + added) `pub use { ... }`
    # reconstructions (fix-2 / V-2), for the gap the accumulators above
    # can't see: a block whose OPENING line is unchanged context. See
    # `_side_pub_use_step`. Kept independent per side; each only starts on
    # a context opening line, so neither overlaps the cfg-aware `rust_*`
    # accumulators above (whose opening line is on their own +/- side).
    old_pub_use_active = False
    old_pub_use_lines: list[str] = []
    new_pub_use_active = False
    new_pub_use_lines: list[str] = []
    for raw in diff_text.splitlines():
        if raw.startswith("+++ "):
            rust_public_use = []
            rust_public_use_cfg_gated = False
            rust_cfg_pending = False
            rust_attr_open = False
            rust_attr_text = ""
            rust_attr_depth = 0
            rust_public_use_removed = []
            old_pub_use_active = False
            old_pub_use_lines = []
            new_pub_use_active = False
            new_pub_use_lines = []
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
            rust_attr_open = False
            rust_attr_text = ""
            rust_attr_depth = 0
            rust_public_use_removed = []
            old_pub_use_active = False
            old_pub_use_lines = []
            new_pub_use_active = False
            new_pub_use_lines = []
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
            rust_attr_open = False
            rust_attr_text = ""
            rust_attr_depth = 0
            rust_public_use_removed = []
            old_pub_use_active = False
            old_pub_use_lines = []
            new_pub_use_active = False
            new_pub_use_lines = []
            continue

        is_added_line = raw.startswith("+") and not raw.startswith("+++")
        is_removed_line = raw.startswith("-") and not raw.startswith("--")
        # Contiguity resets: both accumulators only span an unbroken run of
        # their own +/- lines respectively.
        if not is_added_line:
            rust_public_use = []
            rust_public_use_cfg_gated = False
            rust_cfg_pending = False
            rust_attr_open = False
            rust_attr_text = ""
            rust_attr_depth = 0
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
                old_pub_use_active, old_pub_use_lines, old_statement = _side_pub_use_step(
                    line, True, old_pub_use_active, old_pub_use_lines, start_on_changed=True
                )
                if old_statement is not None:
                    for name in _rust_reexport_marked_names(old_statement):
                        removals.add(
                            Removal(
                                path=current_path,
                                kind=current_kind,
                                symbol_kind="use",
                                name=name,
                            )
                        )
        elif is_added_line:
            line = raw[1:]
            match = _scan_line(current_kind, line)
            if match:
                additions.add((current_path, current_kind, match[1]))
            if current_kind == "rust":
                stripped = line.strip()
                if rust_attr_open:
                    # Continuing a multi-line attribute (rustfmt wraps long
                    # ones, e.g. `#[cfg(any(\n    a,\n    b\n))]`): stay in
                    # attribute mode until bracket depth returns to 0, then
                    # classify the WHOLE joined attribute text at once.
                    rust_attr_text += " " + stripped
                    rust_attr_depth += stripped.count("[") - stripped.count("]")
                    if rust_attr_depth <= 0:
                        rust_attr_open = False
                        if RUST_CFG_ATTR.match(rust_attr_text.strip()):
                            rust_cfg_pending = True
                        rust_attr_text = ""
                        rust_attr_depth = 0
                elif RUST_ATTR_OPEN.match(stripped):
                    depth = stripped.count("[") - stripped.count("]")
                    if depth <= 0:
                        if RUST_CFG_ATTR.match(stripped):
                            rust_cfg_pending = True
                        # Non-cfg single-line attribute: leave any pending
                        # cfg gate untouched — stacked attributes above a
                        # `pub use` must not clear it.
                    else:
                        rust_attr_open = True
                        rust_attr_text = stripped
                        rust_attr_depth = depth
                elif stripped == "" or stripped.startswith(("///", "//!", "//")):
                    # Blank added lines and doc/line comments between a cfg
                    # attribute and the `pub use` it guards keep the pending
                    # gate alive — only a real item/statement line below
                    # resolves or clears it.
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
                new_pub_use_active, new_pub_use_lines, new_statement = _side_pub_use_step(
                    line, True, new_pub_use_active, new_pub_use_lines
                )
                if new_statement is not None:
                    for name in _rust_reexport_marked_names(new_statement):
                        additions.add((current_path, current_kind, name))
        else:
            # Context line (unchanged): not part of the diff's +/- surface,
            # but still part of BOTH the old and new file's view, so it
            # feeds both one-sided `pub use` reconstructions (fix-2 / V-2).
            # It never itself yields a removal or addition — only a
            # REMOVED/ADDED line inside an active block does.
            if current_kind == "rust" and (raw.startswith(" ") or raw == ""):
                ctx_line = raw[1:] if raw else ""
                old_pub_use_active, old_pub_use_lines, old_statement = _side_pub_use_step(
                    ctx_line, False, old_pub_use_active, old_pub_use_lines
                )
                if old_statement is not None:
                    for name in _rust_reexport_marked_names(old_statement):
                        removals.add(
                            Removal(
                                path=current_path,
                                kind=current_kind,
                                symbol_kind="use",
                                name=name,
                            )
                        )
                new_pub_use_active, new_pub_use_lines, new_statement = _side_pub_use_step(
                    ctx_line, False, new_pub_use_active, new_pub_use_lines
                )
                if new_statement is not None:
                    for name in _rust_reexport_marked_names(new_statement):
                        additions.add((current_path, current_kind, name))

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
