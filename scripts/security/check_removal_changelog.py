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
from typing import Callable, Iterable


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
TS_NAMED_REEXPORT = re.compile(
    r'^export\s+(type\s+)?\{([^{}]*)\}\s+from\s+["\'](\./[^"\']+\.js)["\']\s*;',
    re.MULTILINE,
)
TS_REEXPORT_NAME = re.compile(
    r"^(?:type\s+)?([A-Za-z_$][A-Za-z0-9_$]*)(?:\s+as\s+([A-Za-z_$][A-Za-z0-9_$]*))?$"
)
RUST_PUBLIC_USE = re.compile(r"^\s*pub\s+use\s+(.+);\s*$", re.DOTALL)
# Rust removals come only from reconstructing each side of a changed file
# (old = context + removed lines, new = context + added lines) and parsing
# what each side exports UNCONDITIONALLY: `pub use` names, and unrestricted
# `pub` items keyed by owner — ("impl", SelfType, name) for associated items
# (self type = last path segment, generics stripped; trait impls key by the
# `for` type) and ("mod", inline-module path, name) for free items. An old
# key missing from the new side is a removal unless it moved (see
# `_rust_removals`). `load_diff` asks git for whole-file context without
# rename detection, so the reconstruction is the complete old and new file.
#
# A `pub use` (or public item) is conditional when an attribute stacked
# immediately above it (multi-line attributes and interleaved doc/line
# comments and blank lines allowed) is `#[cfg(...)]` or `#[cfg_attr(...)]`,
# when it sits inside a block (inline `mod`, `impl`, ...) whose item carries
# such an attribute, or when an enclosing scope has an inner `#![cfg(...)]`.
# Conditional, `pub(crate)`/`pub(super)`/`pub(in ...)` items and uses, and
# `pub` text inside fn/struct/trait/macro bodies are not public API, so
# their removal is never reported; an unconditional item that becomes
# conditional (e.g. a cfg attribute added above an unchanged `pub fn`) is.
# A glob export names nothing by itself; only a single-segment glob in the
# crate root (`pub use m::*;`) is resolved, one level deep, when checking
# whether the root still exports a name (see `_rust_root_names`).
#
# Known fail-closed false positives — each costs a manual CHANGELOG entry,
# never a silent miss, so they are accepted rather than special-cased:
#   - `#[cfg_attr(pred, doc(...))]` / `#[cfg_attr(pred, allow(...))]` is
#     treated as gating even though it never affects whether the item is
#     compiled in.
#   - A free item moved between inline modules of the same file is reported
#     (its key changes and same-file moves only cancel by exact key).
#   - A free item moved between files, or into a `pub mod`, is reported
#     unless the crate root re-exports its name (the module path changed).
#   - A free item (or `pub use`) under any unrestricted `pub mod` segment —
#     cfg-gated or not, file or inline, at any depth, even below a restricted
#     ancestor where the path is not actually reachable (fails closed) — or
#     under a module the
#     crate root re-exports (`pub use a::b;`, `pub use a::b as api;`) has
#     its own path (`crate::a::X`, `crate::api::X`); losing it is reported
#     even when the crate root (by name or glob) still exports `X`, e.g. an
#     item moved out of a `pub mod` into a private module with the root
#     re-export updated. An undeclared or unseen module segment also fails
#     closed.
#   - Root globs are followed one level only: a name exported through
#     `pub use m::*;` whose module `m` itself only globs a submodule
#     (`pub use rd::*;` in `m/mod.rs`) is not seen, so moving it there is
#     reported. Explicit `pub use` names in the intermediate `mod.rs` avoid
#     it.
# Only library sources (`src/rust/crates/<c>/src/`, minus `src/main.rs` and
# `src/bin/`) are scanned; examples, benches, build scripts, and binaries
# neither report removals nor cancel them.
# Known limits:
#   - Free-item cancellation approximates the public path: the same file
#     still exports the name at the same level (a module never stands in for
#     a non-module item), or the item was public only through the crate root
#     — at the root's own top level, or in a module every segment of which
#     is declared private or `pub(...)`-restricted in its parent (either
#     side seen) and not re-exported by the root — and the head-side crate
#     root (`src/lib.rs`) exports the name
#     unconditionally as a non-module item or `pub use`. The root is read
#     from the diff, else from the head ref in live-git mode; with
#     `--diff-file` a root outside the diff is unseen and nothing cancels
#     against it. The match is by name: a different item re-exported under
#     the removed name (e.g. an unrelated `sqlite::open` behind `pub use`)
#     cancels. A glob module's `mod` declaration is not checked for cfg gates
#     or `#[path]`, so a cfg-gated `mod m;` behind an ungated `pub use m::*;`
#     still cancels; module segments are mapped to files by name, so a
#     `#[path = "x.rs"] pub mod api;` beside a private `mod x;` is unseen.
#   - A dropped `pub use x::open` (target kind unknown) cancels against a
#     same-named `pub mod open` in the same file or crate root.
#   - The root re-export check matches the last segment of every root
#     `pub use` leaf, items included, so a root re-export of an item that
#     shares a private module's name (`mod open; pub use open::{open, ..};`)
#     blocks cancellation for moves out of that module (fails closed).
#   - An associated item cancels on any same-crate `impl` of a type with the
#     same last path segment, so two distinct same-named types in different
#     modules can cancel each other's method removals.
#   - The path-level oracle is the surface comparator
#     (`dev/tools/surface_comparator.py`).
#   - `pub use` names are compared per file by bare name: an inline `pub mod
#     x { pub use ... }` contributes to the same set as the crate root.
#   - Changelog matching is by bare name, so documenting `new` covers every
#     removed `Type::new`.
#   - `--diff-file` inputs are parsed as given. A hunk without full context
#     (e.g. a `-U3` patch that starts inside an `impl` or a long
#     `pub use { ... }` block, or whose cfg attribute lies outside the hunk)
#     is reconstructed only as far as the hunk reaches; each hunk is parsed
#     independently.
#   - Comment/string stripping is lexical: nested block comments and a
#     lifetime immediately followed by a quote can confuse brace tracking.
#   - Macro-generated items and exports are invisible.
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
    # Rust associated items: the owning `impl` self type ("" otherwise).
    owner: str = ""

    def display(self) -> str:
        return f"{self.owner}::{self.name}" if self.owner else self.name


_RUST_LIBRARY_SOURCE = re.compile(r"^src/rust/crates/[^/]+/src/(.+)$")


def _classify(path: str) -> str | None:
    # Test files are NOT public API — a renamed/removed test function is not a
    # consumer-visible removal. Excluding any `tests/` directory keeps the
    # removal-detect gate scoped to the shipped public surface and stops false
    # positives on test churn (e.g. the Slice-25 `test_surface.py` rewrite).
    if "/tests/" in path:
        return None
    if path.startswith("src/rust/crates/") and path.endswith(".rs"):
        # Only a crate's library sources are public API: `examples/`,
        # `benches/`, `build.rs`, `src/main.rs`, and `src/bin/` neither
        # report removals nor cancel them.
        match = _RUST_LIBRARY_SOURCE.match(path)
        if (
            match is None
            or match.group(1) == "main.rs"
            or match.group(1).startswith("bin/")
        ):
            return None
        return "rust"
    if path.startswith("src/python/") and path.endswith(".py"):
        return "python"
    if path.startswith("src/ts/") and (path.endswith(".ts") or path.endswith(".tsx")):
        return "ts"
    return None


def _scan_line(kind: str, line: str) -> tuple[str, str] | None:
    if kind == "python":
        m = PY_PUBLIC.match(line)
    elif kind == "ts":
        m = TS_PUBLIC.match(line)
    else:
        return None
    return (m.group(1), m.group(2)) if m else None


def _typescript_root_reexports(
    new_files: dict[str, list[str]], read_head: HeadReader | None
) -> set[tuple[str, str, str]]:
    """Recognize explicit package-root exports backed by a sibling declaration.

    A missing target, a glob, or a type-only export of a runtime value cannot
    cancel a removal. TypeScript compilation remains the full module oracle.
    """
    root_path = "src/ts/src/index.ts"
    root = new_files.get(root_path)
    if root is None:
        return set()
    additions: set[tuple[str, str, str]] = set()
    root_text = re.sub(r"/\*.*?\*/|//[^\n]*", "", "\n".join(root), flags=re.DOTALL)
    for match in TS_NAMED_REEXPORT.finditer(root_text):
        statement_type_only, members, ref = match.groups()
        # The package root for this check uses private sibling modules.
        # Other paths stay fail-closed until explicitly supported.
        if "/" in ref[2:] or ".." in ref or not ref.endswith(".js"):
            continue
        target_path = f"src/ts/src/{ref[2:-3]}.ts"
        target = new_files.get(target_path)
        if target is None and read_head is not None:
            target = read_head(target_path)
        if target is None:
            continue
        declared = {
            name: kind
            for line in target
            if (scanned := _scan_line("ts", line)) is not None
            for kind, name in (scanned,)
        }
        for member in members.split(","):
            member = member.strip()
            if not member:
                continue
            item = TS_REEXPORT_NAME.fullmatch(member)
            if item is None:
                continue
            source, alias = item.groups()
            kind = declared.get(source)
            if kind is None:
                continue
            type_only = bool(statement_type_only or member.startswith("type "))
            if type_only and kind not in {"interface", "type"}:
                continue
            additions.add((root_path, "ts", alias or source))
    return additions


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
    """Return the names one ``pub use`` exports explicitly.

    Globs contribute nothing here (`_rust_root_names` resolves simple root
    globs separately). Aliases contribute only the exported alias.
    """

    return {name for name, _ in _rust_use_leaves(statement)}


def _rust_use_leaves(statement: str) -> set[tuple[str, str]]:
    """``(exported name, last segment of the source path)`` per explicit leaf
    of one ``pub use`` (they differ only for aliases)."""

    match = RUST_PUBLIC_USE.match(statement)
    if match is None:
        return set()
    ident = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")

    def collect(tree: str, prefix: str = "") -> set[tuple[str, str]]:
        tree = tree.strip()
        if not tree or tree == "*":
            return set()
        if "{" in tree:
            open_index = tree.find("{")
            if not tree.endswith("}"):
                return set()
            nested_prefix = tree[:open_index].strip().removesuffix("::")
            joined_prefix = "::".join(part for part in (prefix, nested_prefix) if part)
            leaves: set[tuple[str, str]] = set()
            for item in _split_use_tree(tree[open_index + 1 : -1]):
                item = item.strip()
                if item == "self":
                    name = joined_prefix.rsplit("::", 1)[-1]
                    if name and name not in {"crate", "self", "super"}:
                        leaves.add((name, name))
                else:
                    leaves.update(collect(item, joined_prefix))
            return leaves

        alias_parts = re.split(r"\s+as\s+", tree, maxsplit=1)
        if len(alias_parts) == 2:
            alias = alias_parts[1].strip()
            source_path = "::".join(
                part for part in (prefix, alias_parts[0].strip()) if part
            )
            if source_path.endswith("::self"):
                source_path = source_path.removesuffix("::self")
            source = source_path.rsplit("::", 1)[-1].strip()
            return {(alias, source)} if ident.fullmatch(alias) else set()
        full_path = "::".join(part for part in (prefix, tree) if part)
        name = full_path.rsplit("::", 1)[-1]
        if name in {"*", "crate", "self", "super"}:
            return set()
        return {(name, name)} if ident.fullmatch(name) else set()

    return collect(match.group(1))


# An item key: ("impl", SelfType, name) for associated items,
# ("mod", "a::b", name) for free items ("" = file level).
_ItemKey = tuple[str, str, str]


@dataclass
class _RustExports:
    """Names one side of one Rust file exports unconditionally."""

    uncond_uses: set[str] = field(default_factory=set)
    # The file's own top level only: explicit `pub use` names and the
    # module named by each single-segment glob (`pub use m::*;`).
    top_uses: set[str] = field(default_factory=set)
    top_globs: set[str] = field(default_factory=set)
    # Source-path last segments of the top-level `pub use` leaves (the
    # module or item behind each export, alias or not).
    top_use_sources: set[str] = field(default_factory=set)
    # Module declarations (`mod m;` / `mod m { .. }`) keyed by (enclosing
    # inline-module path, name), cfg-gated or not: unrestricted `pub mod`
    # vs private or `pub(...)`-restricted.
    pub_mods: set[tuple[str, str]] = field(default_factory=set)
    private_mods: set[tuple[str, str]] = field(default_factory=set)
    # Unrestricted `pub` items only (key -> kinds; a module and a value item
    # can share a name); `pub(crate)` etc. and items inside fn/struct/trait
    # bodies are not public API.
    uncond_items: dict[_ItemKey, set[str]] = field(default_factory=dict)


_RUST_SIMPLE_GLOB = re.compile(
    r"^\s*pub\s+use\s+(?:(?:self|crate)\s*::\s*)?([A-Za-z_][A-Za-z0-9_]*)\s*::\s*\*\s*;\s*$"
)
_RUST_MOD_DECL = re.compile(
    r"^(pub\s*(\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\b"
)
_RUST_VIS_PREFIX = re.compile(r"^(?:pub\s*(?:\([^)]*\))?\s*|unsafe\s+|default\s+)*")
_RUST_MOD_HEADER = re.compile(r"^mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*$")


def _impl_self_type(header: str) -> str | None:
    """Last path segment of an ``impl`` header's self type, generics stripped."""

    rest = header[len("impl") :].lstrip()
    if rest.startswith("<"):
        depth = 0
        for index, char in enumerate(rest):
            depth += char == "<"
            depth -= char == ">"
            if depth == 0:
                rest = rest[index + 1 :]
                break
    rest = re.split(r"\bwhere\b", rest, maxsplit=1)[0]
    # The trait path may itself carry generics; split on the top-level `for`.
    depth = 0
    for match in re.finditer(r"[<>]|\bfor\b", rest):
        token = match.group(0)
        if token == "<":
            depth += 1
        elif token == ">":
            depth -= 1
        elif depth == 0:
            rest = rest[match.end() :]
            break
    rest = re.sub(
        r"^(?:\s*(?:&\s*(?:'[A-Za-z_]+\s*)?|mut\b|dyn\b|!))*", "", rest.strip()
    )
    rest = rest.split("<", 1)[0].strip()
    name = rest.rsplit("::", 1)[-1].strip()
    return name if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name) else None


def _block_frame(header: str, parent: tuple[str, str]) -> tuple[str, str]:
    """Classify the block a ``{`` opens: an impl (owner type), an inline
    module (path), a transparent ``extern`` block, or opaque (fn/struct/
    trait/... bodies, whose `pub` text is not a public item)."""

    if parent[0] == "opaque":
        return parent
    header = _RUST_VIS_PREFIX.sub("", header.strip())
    if re.match(r"impl\b", header):
        owner = _impl_self_type(header)
        return ("impl", owner) if owner else ("opaque", "")
    mod = _RUST_MOD_HEADER.match(header)
    if mod and parent[0] == "mod":
        return ("mod", "::".join(part for part in (parent[1], mod.group(1)) if part))
    if re.match(r"extern\b", header) and not re.search(r"\bfn\b", header):
        return parent
    return ("opaque", "")


def _strip_rust_non_code(lines: list[str]) -> list[str]:
    text = "\n".join(lines)
    blanked = _RUST_NON_CODE.sub(lambda m: _NON_NEWLINE.sub(" ", m.group(0)), text)
    return blanked.split("\n")


def _parse_rust_side(lines: list[str]) -> _RustExports:
    """Parse one reconstructed side of a Rust file into its exports."""

    out = _RustExports()
    depth = 0
    # One frame per open `{`, as classified by `_block_frame`; the file
    # itself is the top-level module.
    frames: list[tuple[str, str]] = []
    header = ""
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

    for line in _strip_rust_non_code(lines):
        rest = line.strip()
        while rest:
            if use_text is not None:
                end = rest.find(";")
                if end < 0:
                    use_text += "\n" + rest
                    break
                use_text += "\n" + rest[: end + 1]
                if not use_gated and (not frames or frames[-1][0] == "mod"):
                    leaves = _rust_use_leaves(use_text)
                    out.uncond_uses.update(name for name, _ in leaves)
                    if not frames:
                        out.top_uses.update(name for name, _ in leaves)
                        out.top_use_sources.update(source for _, source in leaves)
                        glob = _RUST_SIMPLE_GLOB.match(use_text)
                        if glob:
                            out.top_globs.add(glob.group(1))
                use_text = None
                header = ""
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
            if not frames or frames[-1][0] == "mod":
                mod_decl = _RUST_MOD_DECL.match(rest)
                if mod_decl:
                    restricted = mod_decl.group(1) is None or mod_decl.group(2)
                    target = out.private_mods if restricted else out.pub_mods
                    target.add((frames[-1][1] if frames else "", mod_decl.group(3)))
            match = RUST_PUBLIC.match(rest)
            frame = frames[-1] if frames else ("mod", "")
            if (
                match
                and not gated
                and not rest.startswith("pub(")
                and frame[0] != "opaque"
            ):
                kind, name = match.group(1), match.group(2)
                out.uncond_items.setdefault((frame[0], frame[1], name), set()).add(kind)
            last = 0
            for token_match in _RUST_STRUCTURE.finditer(rest):
                token = token_match.group(0)
                header += rest[last : token_match.start()]
                last = token_match.end()
                if token in "([":
                    paren += 1
                    header += token
                elif token in ")]":
                    paren = max(0, paren - 1)
                    header += token
                elif token == "{":
                    if carry is not None and depth == carry:
                        gates.append(depth)
                        carry = None
                    frames.append(
                        _block_frame(header, frames[-1] if frames else ("mod", ""))
                    )
                    header = ""
                    depth += 1
                elif token == "}":
                    depth = max(0, depth - 1)
                    if frames:
                        frames.pop()
                    header = ""
                    while gates and depth <= gates[-1]:
                        gates.pop()
                    if carry is not None and depth < carry:
                        carry = None
                else:
                    if paren == 0:
                        header = ""
                    else:
                        header += token
                    if paren == 0 and carry is not None and depth == carry:
                        carry = None
            header += rest[last:] + " "
            break
    return out


_RUST_CRATE = re.compile(r"^(src/rust/crates/[^/]+/)")

# Reads a file's text at the head ref when it is not part of the diff
# (None: absent or unavailable).
HeadReader = Callable[[str], "list[str] | None"]


def _rust_crate(path: str) -> str:
    match = _RUST_CRATE.match(path)
    return match.group(1) if match else path


def _rust_removals(
    old_exports: dict[str, _RustExports],
    new_exports: dict[str, _RustExports],
    read_head: HeadReader | None = None,
) -> set[Removal]:
    """Compare per-file unconditional exports, cancelling path-preserving moves.

    An item is removed from a file when its old-side key is unconditional
    and the new side lacks it. The removal cancels as a move only when:
      - associated item: an unconditional `pub` item of that name sits in an
        `impl` of the same self type in any library file of the same crate
        (methods are addressed through their type, so the path survives);
      - free item or `pub use` name: the same file still exports it at the
        same level (a file-level item or a newly added top-level `pub use`),
        or — only when the crate root itself is the old path, or the item's
        module is private (`_root_path_only`) — the crate root re-exports
        the name (`_rust_root_names`). A root `pub mod` of that name stands
        in only for a removed module or `pub use`.
    """

    in_diff = old_exports.keys() | new_exports.keys()
    head_cache: dict[str, _RustExports | None] = {}

    def head_exports(path: str) -> _RustExports | None:
        if path in in_diff:
            return new_exports.get(path, _RustExports())
        if path not in head_cache:
            lines = read_head(path) if read_head is not None else None
            head_cache[path] = None if lines is None else _parse_rust_side(lines)
        return head_cache[path]

    root_cache: dict[str, dict[str, set[str]]] = {}

    def root_names(crate: str) -> dict[str, set[str]]:
        if crate not in root_cache:
            root_cache[crate] = _rust_root_names(crate, head_exports)
        return root_cache[crate]

    def both_sides(path: str) -> list[_RustExports]:
        head = head_exports(path)
        base = old_exports.get(path, _RustExports()) if path in in_diff else head
        return [side for side in (head, base) if side is not None]

    def root_cancels(path: str, owner: str | None, kind: str, name: str) -> bool:
        crate = _rust_crate(path)
        if not _root_path_only(crate, path[len(crate + "src/") :], owner, both_sides):
            return False
        kinds = root_names(crate).get(name, set())
        if kind not in ("mod", "use"):
            kinds = kinds - {"mod"}
        return bool(kinds)

    new_impl: dict[str, set[tuple[str, str]]] = {}
    for path, exports in new_exports.items():
        crate = _rust_crate(path)
        for scope, owner, name in exports.uncond_items:
            if scope == "impl":
                new_impl.setdefault(crate, set()).add((owner, name))

    removals: set[Removal] = set()
    for path in in_diff:
        crate = _rust_crate(path)
        old = old_exports.get(path, _RustExports())
        new = new_exports.get(path, _RustExports())
        for key, kinds in old.uncond_items.items():
            survivors = new.uncond_items.get(key, set())
            scope, owner, name = key
            for kind in sorted(kinds):
                # A module of the same name does not stand in for a
                # non-module item (or vice versa).
                if any((other == "mod") == (kind == "mod") for other in survivors):
                    continue
                if scope == "impl":
                    if (owner, name) in new_impl.get(crate, set()):
                        continue
                    removals.add(Removal(path, "rust", kind, name, owner))
                    continue
                if owner == "" and name in new.top_uses - old.top_uses:
                    continue
                if root_cancels(path, owner, kind, name):
                    continue
                removals.add(Removal(path, "rust", kind, name))
        for name in old.uncond_uses - new.uncond_uses:
            # A use below the file's top level has an unknown module path.
            owner = "" if name in old.top_uses else None
            if ("mod", "", name) in new.uncond_items or root_cancels(
                path, owner, "use", name
            ):
                continue
            removals.add(Removal(path, "rust", "use", name))
    return removals


def _root_path_only(
    crate: str,
    rel: str,
    owner: str | None,
    sides: Callable[[str], list[_RustExports]],
) -> bool:
    """Whether an item at ``rel`` (path under the crate's `src/`), inline
    module ``owner`` (None: unknown), was public only through the crate root.

    True for the root's own top level. Otherwise every module segment on the
    way — the file's (`a/b.rs` -> `a`, `b`) then the inline owner's — must
    be declared, in its parent file (either side seen), private or
    `pub(...)`-restricted and never unrestricted `pub`, and the crate root
    must not re-export any of them by name or alias (`pub use a::b as api`
    makes `crate::api::X` public). An unrestricted `pub mod` is treated as
    giving the item its own path (`crate::a::X`) that a root name does not
    keep, even below a restricted ancestor; an
    undeclared or unseen module, or an unknown inline owner, fails closed.
    """

    if owner is None:
        return False
    parts = [] if rel == "lib.rs" else rel.removesuffix(".rs").split("/")
    if parts and parts[-1] == "mod":
        parts.pop()
    inline = owner.split("::") if owner else []
    if not parts and not inline:
        return True

    def module_files(segments: list[str]) -> list[str]:
        if not segments:
            return [crate + "src/lib.rs"]
        stem = crate + "src/" + "/".join(segments)
        return [stem + ".rs", stem + "/mod.rs"]

    # (files that may declare the segment, enclosing inline path, name)
    decls = [
        (module_files(parts[:index]), "", segment)
        for index, segment in enumerate(parts)
    ]
    own_file = [crate + "src/" + rel]
    decls += [
        (own_file, "::".join(inline[:index]), segment)
        for index, segment in enumerate(inline)
    ]
    for files, parent, segment in decls:
        seen = [side for file in files for side in sides(file)]
        if any((parent, segment) in side.pub_mods for side in seen):
            return False
        if not any((parent, segment) in side.private_mods for side in seen):
            return False
    exposed = {segment for _, _, segment in decls}
    root_sides = sides(crate + "src/lib.rs")
    return not any(
        exposed & (side.top_uses | side.top_use_sources) for side in root_sides
    )


def _rust_root_names(
    crate: str, head_exports: Callable[[str], _RustExports | None]
) -> dict[str, set[str]]:
    """Names the head-side crate root (`src/lib.rs`) exports at its top level.

    Counts unconditional top-level `pub` items and explicit `pub use` names.
    A single-segment glob `pub use m::*;` contributes the unconditional
    `pub` items of inline module `m` in the root, else the top-level
    unconditional items and explicit `pub use` names of `src/m.rs` or
    `src/m/mod.rs`, one level deep. A root (or glob module file) that is
    neither in the diff nor readable at the head ref contributes nothing.
    Maps each name to the item kinds (or "use") that export it.
    """

    names: dict[str, set[str]] = {}

    def add(exports: _RustExports, module: str, uses: bool) -> bool:
        found = False
        for (scope, owner, name), kinds in exports.uncond_items.items():
            if (scope, owner) == ("mod", module):
                names.setdefault(name, set()).update(kinds)
                found = True
        if uses:
            for name in exports.top_uses:
                names.setdefault(name, set()).add("use")
        return found

    root = head_exports(crate + "src/lib.rs")
    if root is None:
        return names
    add(root, "", True)
    for module in root.top_globs:
        if add(root, module, False):
            continue
        for candidate in (f"{crate}src/{module}.rs", f"{crate}src/{module}/mod.rs"):
            target = head_exports(candidate)
            if target is not None:
                add(target, "", True)
    return names


def parse_diff(
    diff_text: str, read_head: HeadReader | None = None
) -> tuple[set[Removal], set[tuple[str, str, str]]]:
    """Returns (removals, additions) keyed for cancellation matching.

    Rust removals are computed from the per-side export sets and are
    final; ``additions`` only cancels Python/TypeScript line-scan removals.
    ``read_head`` supplies head-side files outside the diff (a crate root
    or glob module file); without it only files in the diff are seen.
    """
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
    new_ts_files: dict[str, list[str]] = {}
    hunk_old: list[str] = []
    hunk_new: list[str] = []

    def flush_hunk() -> None:
        if (
            current_kind == "rust"
            and current_path is not None
            and (hunk_old or hunk_new)
        ):
            for side, lines in ((old_exports, hunk_old), (new_exports, hunk_new)):
                exports = _parse_rust_side(lines)
                acc = side.setdefault(current_path, _RustExports())
                acc.uncond_uses |= exports.uncond_uses
                acc.top_uses |= exports.top_uses
                acc.top_globs |= exports.top_globs
                acc.top_use_sources |= exports.top_use_sources
                acc.pub_mods |= exports.pub_mods
                acc.private_mods |= exports.private_mods
                for key, kinds in exports.uncond_items.items():
                    acc.uncond_items.setdefault(key, set()).update(kinds)
        if current_kind == "ts" and current_path is not None and hunk_new:
            new_ts_files.setdefault(current_path, []).extend(hunk_new)
        hunk_old.clear()
        hunk_new.clear()

    for raw in diff_text.splitlines():
        if raw.startswith("+++ "):
            flush_hunk()
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
            elif raw.startswith("-"):
                hunk_old.append(raw[1:])
            elif raw.startswith(" ") or raw == "":
                hunk_old.append(raw[1:])
                hunk_new.append(raw[1:])
            continue
        if current_kind == "ts":
            if raw.startswith("+"):
                hunk_new.append(raw[1:])
            elif raw.startswith(" ") or raw == "":
                hunk_new.append(raw[1:])

        match = _scan_line(current_kind, raw[1:])
        if match is None:
            continue
        symbol_kind, name = match
        if raw.startswith("-"):
            removals.add(Removal(current_path, current_kind, symbol_kind, name))
        elif raw.startswith("+"):
            additions.add((current_path, current_kind, name))
    flush_hunk()

    removals |= _rust_removals(old_exports, new_exports, read_head)
    additions |= _typescript_root_reexports(new_ts_files, read_head)
    return removals, additions


def real_removals(
    removals: set[Removal], additions: set[tuple[str, str, str]]
) -> list[Removal]:
    """A Python/TypeScript removal cancels if the same symbol name re-appears
    in the same file; Rust removals arrive already move-cancelled."""
    out = []
    for r in removals:
        if r.kind != "rust" and (r.path, r.kind, r.name) in additions:
            continue
        out.append(r)
    return sorted(out, key=lambda r: (r.path, r.owner, r.name))


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


def _git_head_reader(args: argparse.Namespace) -> HeadReader:
    def read(path: str) -> list[str] | None:
        completed = subprocess.run(
            ["git", "-C", str(args.repo_root), "show", f"{args.head}:{path}"],
            check=False,
            capture_output=True,
            text=True,
        )
        return completed.stdout.splitlines() if completed.returncode == 0 else None

    return read


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
        # A rename is diffed as delete + add so the renamed file's items
        # are compared (a pure rename would otherwise produce no hunk).
        "--no-renames",
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
    p.add_argument(
        "--diff-file", default=None, help="read diff from a file instead of git"
    )
    p.add_argument(
        "--changelog",
        default=None,
        help="CHANGELOG.md path (default: <repo>/CHANGELOG.md)",
    )
    p.add_argument(
        "--repo-root", default=None, help="repository root (default: git rev-parse)"
    )
    args = p.parse_args(argv)

    if args.repo_root is None:
        try:
            args.repo_root = subprocess.check_output(
                ["git", "rev-parse", "--show-toplevel"], text=True
            ).strip()
        except subprocess.CalledProcessError:
            sys.stderr.write("not inside a git repo; pass --repo-root\n")
            return 2

    changelog_path = (
        Path(args.changelog)
        if args.changelog
        else Path(args.repo_root) / "CHANGELOG.md"
    )
    if not changelog_path.exists():
        sys.stderr.write(f"CHANGELOG.md missing at {changelog_path}\n")
        return 2

    diff_text = load_diff(args)
    read_head = None if args.diff_file else _git_head_reader(args)
    removals, additions = parse_diff(diff_text, read_head)
    truly_removed = real_removals(removals, additions)
    undocumented = changelog_documents(changelog_path, truly_removed)

    if undocumented:
        sys.stderr.write(
            "AC-050c: removed public symbols missing from CHANGELOG Removed section:\n"
        )
        for r in undocumented:
            sys.stderr.write(f"  {r.path}: {r.symbol_kind} {r.display()}\n")
        return 1

    if truly_removed:
        sys.stdout.write(f"AC-050c OK: {len(truly_removed)} removals all documented.\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
