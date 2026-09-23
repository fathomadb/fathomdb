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
from dataclasses import dataclass, field
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
# Rust `pub use` handling reconstructs each side of a changed file (old =
# context + removed lines, new = context + added lines) and parses the set of
# names each side exports UNCONDITIONALLY; a name in the old set but not the
# new one is a removal. `load_diff` asks git for whole-file context, so the
# reconstruction is the complete old and new file and the result does not
# depend on which lines happen to fall inside a hunk.
#
# A `pub use` (or public item) is conditional when an attribute stacked
# immediately above it (multi-line attributes and interleaved doc/line
# comments and blank lines allowed) is `#[cfg(...)]` or `#[cfg_attr(...)]`,
# when it sits inside a block (inline `mod`, `impl`, ...) whose item carries
# such an attribute, or when an enclosing scope has an inner `#![cfg(...)]`.
# `pub(crate)`/`pub(super)`/`pub(in ...)` uses are not public; glob exports
# name nothing and stay fail-closed. A public item that loses its last
# unconditional definition in a file (e.g. a cfg attribute added above an
# unchanged `pub fn`) is also a removal.
#
# Known fail-closed false positives — each costs a manual CHANGELOG entry,
# never a silent miss, so they are accepted rather than special-cased:
#   - `#[cfg_attr(pred, doc(...))]` / `#[cfg_attr(pred, allow(...))]` is
#     treated as gating even though it never affects whether the item is
#     compiled in.
#   - Exports are compared per file by bare name: an inline `pub mod x {
#     pub use ... }` contributes to the same set as the crate root.
# Known limits:
#   - `--diff-file` inputs are parsed as given. A hunk without full context
#     (e.g. a `-U3` patch that starts inside a long `pub use { ... }` block,
#     or whose cfg attribute lies outside the hunk) is reconstructed only as
#     far as the hunk reaches; each hunk is parsed independently.
#   - Comment/string stripping is lexical: nested block comments and a
#     lifetime immediately followed by a quote can confuse brace tracking.
#   - Macro-generated exports (`macro_rules!` expanding to `pub use`) are
#     invisible.
_RUST_CFG_ATTR = re.compile(r"#!?\s*\[\s*cfg(?:_attr)?\s*\(")
_RUST_ATTR_START = re.compile(r"#!?\s*\[")
_RUST_USE_START = re.compile(r"pub\s+use\b")
# A line starting one of these at the gated item's own depth begins a new
# item, so a cfg gate still waiting for its item's `{`/`;` does not leak
# onto it.
_RUST_ITEM_START = re.compile(
    r"(?:pub\b|use\b|mod\b|fn\b|struct\b|enum\b|union\b|impl\b|trait\b|type\b"
    r"|const\b|static\b|extern\b|unsafe\b|async\b|macro_rules!)"
)
# Comments and string/char literals, replaced by blanks (newlines kept) before
# parsing so their braces, `;`, and `pub use` text never count as code.
_RUST_NON_CODE = re.compile(
    r"//[^\n]*"
    r"|/\*.*?\*/"
    r"|\b(?:b|c|br|cr)?r(#*)\".*?\"\1"
    r"|\"(?:\\.|[^\"\\])*\""
    r"|'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'",
    re.DOTALL,
)
_NON_NEWLINE = re.compile(r"[^\n]")
_RUST_STRUCTURE = re.compile(r"[{}()\[\];]")


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


@dataclass
class _RustExports:
    """Names one side of one hunk exports, split by cfg-gating."""

    uncond_uses: set[str] = field(default_factory=set)
    # Unrestricted `pub` items only (name -> kind); `pub(crate)` etc. are
    # not public API, so they never make an item "become conditional".
    uncond_items: dict[str, str] = field(default_factory=dict)
    cond_items: set[str] = field(default_factory=set)
    # Items on `+` lines, any visibility, matching the line-scan removals.
    added_uncond_items: set[str] = field(default_factory=set)
    added_cond_items: set[str] = field(default_factory=set)


def _strip_rust_non_code(lines: list[str]) -> list[str]:
    text = "\n".join(lines)
    blanked = _RUST_NON_CODE.sub(lambda m: _NON_NEWLINE.sub(" ", m.group(0)), text)
    return blanked.split("\n")


def _parse_rust_side(lines: list[str], added: list[bool]) -> _RustExports:
    """Parse one reconstructed side of a Rust file into its exports.

    ``added[i]`` marks lines that are ``+`` lines of the diff (new side
    only); an unconditional public item on such a line is an addition.
    """

    out = _RustExports()
    depth = 0
    paren = 0
    # Each entry d gates everything while `depth > d` (a cfg-gated block
    # opened at depth d, or -1 for a file-level `#![cfg(...)]`).
    gates: list[int] = []
    # Depth of a cfg-gated item whose opening `{` (or terminating `;`) has
    # not been seen yet — a multi-line item header.
    carry: int | None = None
    attrs_gated = False
    attr_text: str | None = None
    attr_depth = 0
    use_text: str | None = None
    use_gated = False

    for index, line in enumerate(_strip_rust_non_code(lines)):
        rest = line.strip()
        while rest:
            if use_text is not None:
                end = rest.find(";")
                if end < 0:
                    use_text += "\n" + rest
                    break
                use_text += "\n" + rest[: end + 1]
                if not use_gated:
                    out.uncond_uses.update(_rust_reexport_names(use_text))
                use_text = None
                rest = rest[end + 1 :].strip()
                continue
            if attr_text is not None or _RUST_ATTR_START.match(rest):
                if attr_text is None:
                    attr_text = ""
                    attr_depth = 0
                consumed = len(rest)
                for pos, char in enumerate(rest):
                    if char == "[":
                        attr_depth += 1
                    elif char == "]":
                        attr_depth -= 1
                        if attr_depth == 0:
                            consumed = pos + 1
                            break
                attr_text += rest[:consumed]
                rest = rest[consumed:].strip()
                if attr_depth > 0:
                    break
                if _RUST_CFG_ATTR.match(attr_text):
                    if attr_text.startswith("#!"):
                        gates.append(depth - 1)
                    else:
                        attrs_gated = True
                attr_text = None
                continue

            if carry is not None and depth == carry and _RUST_ITEM_START.match(rest):
                carry = None
            if _RUST_ITEM_START.match(rest):
                paren = 0
            gated = attrs_gated or carry is not None or bool(gates)
            if attrs_gated:
                carry = depth
                attrs_gated = False
            if _RUST_USE_START.match(rest):
                use_text = ""
                use_gated = gated
                # A single-statement `pub use` never opens a gated block.
                if carry == depth:
                    carry = None
                continue
            match = RUST_PUBLIC.match(rest)
            if match:
                kind, name = match.group(1), match.group(2)
                if gated:
                    out.cond_items.add(name)
                    if added[index]:
                        out.added_cond_items.add(name)
                else:
                    if not rest.startswith("pub("):
                        out.uncond_items.setdefault(name, kind)
                    if added[index]:
                        out.added_uncond_items.add(name)
            for token in _RUST_STRUCTURE.findall(rest):
                if token in "([":
                    paren += 1
                elif token in ")]":
                    paren = max(0, paren - 1)
                elif token == "{":
                    if carry is not None and depth == carry:
                        gates.append(depth)
                        carry = None
                    depth += 1
                elif token == "}":
                    depth = max(0, depth - 1)
                    while gates and depth <= gates[-1]:
                        gates.pop()
                    if carry is not None and depth < carry:
                        carry = None
                elif paren == 0 and carry is not None and depth == carry:
                    carry = None
            break
    return out


def parse_diff(diff_text: str) -> tuple[set[Removal], set[tuple[str, str, str]]]:
    """Returns (removals, additions) keyed for cancellation matching."""
    removals: set[Removal] = set()
    # Additions keyed (path, kind, name) so a rename WITHIN the same file
    # at the same path cancels out a removal (true delete only when no
    # corresponding + line in the same path).
    additions: set[tuple[str, str, str]] = set()

    current_path: str | None = None
    current_kind: str | None = None
    # Per-file Rust exports, unioned over that file's hunks (one hunk under
    # `load_diff`'s whole-file context).
    old_exports: dict[str, _RustExports] = {}
    new_exports: dict[str, _RustExports] = {}
    hunk_old: list[str] = []
    hunk_new: list[str] = []
    hunk_new_added: list[bool] = []

    def flush_hunk() -> None:
        if current_kind == "rust" and current_path is not None and (hunk_old or hunk_new):
            old = _parse_rust_side(hunk_old, [False] * len(hunk_old))
            new = _parse_rust_side(hunk_new, hunk_new_added)
            for side, exports in ((old_exports, old), (new_exports, new)):
                acc = side.setdefault(current_path, _RustExports())
                acc.uncond_uses |= exports.uncond_uses
                for name, kind in exports.uncond_items.items():
                    acc.uncond_items.setdefault(name, kind)
                acc.cond_items |= exports.cond_items
                acc.added_uncond_items |= exports.added_uncond_items
                acc.added_cond_items |= exports.added_cond_items
        hunk_old.clear()
        hunk_new.clear()
        hunk_new_added.clear()

    for raw in diff_text.splitlines():
        if raw.startswith("+++ "):
            flush_hunk()
            # New-path marker is more reliable than the diff header for
            # in-file renames; both old and new paths are the same in
            # normal removals.
            spec = raw[4:].strip()
            spec = spec.removeprefix("b/")
            current_path = spec if spec != "/dev/null" else current_path
            current_kind = _classify(current_path) if current_path else None
            continue
        if raw.startswith("--- "):
            flush_hunk()
            spec = raw[4:].strip()
            spec = spec.removeprefix("a/")
            if spec != "/dev/null":
                current_path = spec
                current_kind = _classify(current_path) if current_path else None
            continue
        if raw.startswith("@@") or raw.startswith("diff --git "):
            flush_hunk()
            continue

        if current_kind is None or current_path is None:
            continue
        if current_kind == "rust":
            if raw.startswith("+"):
                hunk_new.append(raw[1:])
                hunk_new_added.append(True)
            elif raw.startswith("-"):
                hunk_old.append(raw[1:])
            elif raw.startswith(" ") or raw == "":
                hunk_old.append(raw[1:])
                hunk_new.append(raw[1:])
                hunk_new_added.append(False)

        is_added_line = raw.startswith("+") and not raw.startswith("+++")
        is_removed_line = raw.startswith("-") and not raw.startswith("--")
        if is_removed_line:
            match = _scan_line(current_kind, raw[1:])
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
        elif is_added_line and current_kind != "rust":
            # Rust additions come from the per-side parse, which excludes
            # cfg-gated items.
            match = _scan_line(current_kind, raw[1:])
            if match:
                additions.add((current_path, current_kind, match[1]))
    flush_hunk()

    for path in old_exports.keys() | new_exports.keys():
        old = old_exports.get(path, _RustExports())
        new = new_exports.get(path, _RustExports())
        for name in new.added_uncond_items:
            additions.add((path, "rust", name))
        # A cfg-gated re-add cancels a removed line only when the name was
        # never an unconditional public item: gated -> gated is a move, but
        # unconditional -> gated removes it from the default build.
        for name in new.added_cond_items - old.uncond_items.keys():
            additions.add((path, "rust", name))
        # Only NEWLY unconditional re-exports cancel item removals: a
        # re-export that already existed does not replace a removed item.
        for name in new.uncond_uses - old.uncond_uses:
            additions.add((path, "rust", name))
        for name in old.uncond_uses - new.uncond_uses:
            removals.add(Removal(path=path, kind="rust", symbol_kind="use", name=name))
        for name, kind in old.uncond_items.items():
            if name in new.cond_items and name not in new.uncond_items and name not in new.uncond_uses:
                removals.add(Removal(path=path, kind="rust", symbol_kind=kind, name=name))

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


_HUNK_HEADER = re.compile(r"^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@")


def _assert_whole_file_hunks(diff_text: str) -> str | None:
    """Return an error unless every file diff is at most one hunk starting at
    line 1 (or 0 for an empty side) — i.e. the whole-file context the Rust
    per-side reconstruction relies on."""

    path = None
    hunks = 0
    for raw in diff_text.splitlines():
        if raw.startswith("diff --git "):
            path = raw
            hunks = 0
            continue
        match = _HUNK_HEADER.match(raw)
        if match is None:
            continue
        hunks += 1
        if hunks > 1 or int(match.group(1)) > 1 or int(match.group(2)) > 1:
            return f"git diff did not produce whole-file context for {path}"
    return None


def load_diff(args: argparse.Namespace) -> str:
    if args.diff_file:
        return Path(args.diff_file).read_text(encoding="utf-8")
    # Whole-file context (`-U` larger than any source file) makes each
    # changed file a single hunk holding the complete old and new text, so
    # the Rust per-side export parse sees every `pub use` block and cfg
    # attribute. `--function-context` was rejected: it widens hunks only to
    # the enclosing funcname match, which does not cover a crate-root
    # `pub use` block or the attributes above it. The result is checked by
    # `_assert_whole_file_hunks` rather than trusted.
    cmd = [
        "git",
        "-C",
        str(args.repo_root),
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "--unified=1000000000",
        f"{args.base}..{args.head}",
        "--",
    ]
    for prefix in SCAN_PREFIXES:
        cmd.append(f"{prefix}**")
    completed = subprocess.run(cmd, check=False, capture_output=True, text=True)
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr)
        sys.exit(2)
    error = _assert_whole_file_hunks(completed.stdout)
    if error is not None:
        sys.stderr.write(error + "\n")
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
