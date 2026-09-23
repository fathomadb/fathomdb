#!/usr/bin/env python3
"""Capture and compare the 0.8.27 hidden-inclusive Rust surface.

Sibling of `surface_comparator.py` (Slice 30), whose rows come from
`cargo public-api` and therefore omit `#[doc(hidden)]` items. This tool builds
rustdoc JSON with `--document-hidden-items` for each row, walks every item
reachable by public path, and records each entry's per-site doc-hidden flags
and cfg predicates. A release compile probe proves that debug-only items are
absent, and release-only items present, in a real `--release` build.

Design of record: `dev/plans/0.8.27/features/hidden-surface/design.md`.

Exit codes: 0 equal (or success), 1 unequal, 2 usage or capture error.
"""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import hashlib
import heapq
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from typing import Any, Iterable, Iterator, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))
import surface_comparator as _comparator  # noqa: E402

canonical_json = _comparator.canonical_json
compare_manifests = _comparator.compare_manifests
_unique = _comparator._unique


SCHEMA = "fathomdb.hidden-surface.v1"
PINNED_NIGHTLY = "nightly-2026-04-24"
FORMAT_VERSION = 57
REPO_ROOT = Path(__file__).resolve().parents[2]
OWNED_ROOT = Path("/tmp/fathomdb-0.8.27-hidden-surface")
ROOT_MARKER = ".fathomdb-hidden-surface-owned"
ROOT_MARKER_CONTENT = "fathomdb.hidden-surface-owned.v1\n"
EXPORT_MARKER = ".fathomdb-hidden-surface-export"
OWNED_CACHE = REPO_ROOT / ".cache" / "0.8.27-hidden-surface" / "target"
BASELINE_DIR = REPO_ROOT / "dev" / "plans" / "0.8.27" / "features" / "hidden-surface"
MIN_FREE_BYTES = 20_000_000_000
GIT_SUBCOMMANDS = frozenset({"archive", "cat-file"})

ROWS: list[dict[str, Any]] = [
    {"id": "engine-default", "crate": "fathomdb-engine", "features": []},
    {"id": "engine-test-hooks", "crate": "fathomdb-engine", "features": ["test-hooks"]},
    {
        "id": "engine-slice72-test-hooks",
        "crate": "fathomdb-engine",
        "features": ["slice72-test-hooks"],
    },
    {
        "id": "engine-operator-test-hooks",
        "crate": "fathomdb-engine",
        "features": ["operator", "test-hooks"],
    },
    {
        "id": "engine-migration-test-hooks",
        "crate": "fathomdb-engine",
        "features": ["migration-test-hooks"],
    },
    {
        "id": "engine-tc5-benchmark",
        "crate": "fathomdb-engine",
        "features": ["tc5-benchmark"],
    },
    {"id": "facade-default", "crate": "fathomdb", "features": []},
    {"id": "facade-operator", "crate": "fathomdb", "features": ["operator"]},
]
RELEASE_PROBE_ROW = "release-probe"
# (package, crate directory, rustdoc default row) for each probed crate.
PROBE_CRATES = [
    ("fathomdb-engine", "src/rust/crates/fathomdb-engine", "engine-default"),
    ("fathomdb", "src/rust/crates/fathomdb", "facade-default"),
]
# Release-only items are invisible to rustdoc (it always enables
# debug_assertions), so they are curated here; `check_release_only` fails the
# capture when the source gains one that is not listed.
RELEASE_ONLY: list[dict[str, str]] = [
    {
        "crate": "fathomdb",
        "path": "fathomdb::release_surface_raw_sql_absence_proof",
        "kind": "module",
    },
]
PROBE_ERROR_CODES = frozenset({"E0425", "E0432", "E0433", "E0412", "E0599", "E0277"})
SCRUBBED_ENV = (
    "RUSTFLAGS",
    "RUSTDOCFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_ENCODED_RUSTDOCFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "CARGO_BUILD_RUSTDOCFLAGS",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_TARGET_DIR",
    "RUSTUP_TOOLCHAIN",
)
STRIP_KEYS = frozenset({"id", "span", "links", "docs", "crate_id"})
USE_FORM_KINDS = frozenset(
    {
        "function",
        "constant",
        "static",
        "struct",
        "enum",
        "union",
        "type_alias",
        "trait",
        "trait_alias",
        "module",
        "macro",
        "proc_macro",
        "external",
    }
)


class HiddenSurfaceError(Exception):
    """A usage, validation, or capture failure (exit code 2)."""


def _run(command: Sequence[str], cwd: Path, env: dict[str, str] | None = None) -> str:
    try:
        return _comparator._run(command, cwd, env)
    except _comparator.ComparatorError as exc:
        raise HiddenSurfaceError(str(exc)) from exc


def _compact(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


# --------------------------------------------------------------------------
# cfg predicates


_SPAN = re.compile(r"[^\s,()]+:\d+:\d+: \d+:\d+ \(#\d+\)")
_STRING = re.compile(r'"((?:[^"\\]|\\.)*)"')
_CFG_PREFIX = "#[attr = CfgTrace(["
_CFG_SUFFIX = "])]"


class _CfgTraceParser:
    def __init__(self, text: str) -> None:
        self.text = text
        self.pos = 0

    def fail(self, what: str) -> HiddenSurfaceError:
        context = self.text[self.pos : self.pos + 60]
        return HiddenSurfaceError(f"unparseable cfg attribute: {what} at {context!r}")

    def skip_space(self) -> None:
        while self.pos < len(self.text) and self.text[self.pos] == " ":
            self.pos += 1

    def peek(self, token: str) -> bool:
        self.skip_space()
        return self.text.startswith(token, self.pos)

    def eat(self, token: str) -> None:
        if not self.peek(token):
            raise self.fail(f"expected {token!r}")
        self.pos += len(token)

    def span(self) -> None:
        self.skip_space()
        match = _SPAN.match(self.text, self.pos)
        if match is None:
            raise self.fail("expected span")
        self.pos = match.end()

    def string(self) -> str:
        self.skip_space()
        match = _STRING.match(self.text, self.pos)
        if match is None:
            raise self.fail("expected string")
        self.pos = match.end()
        return match.group(1)

    def predicate(self) -> str:
        for combinator in ("Any", "All"):
            if self.peek(combinator + "(["):
                self.eat(combinator + "([")
                operands: list[str] = []
                if not self.peek("]"):
                    operands.append(self.predicate())
                    while self.peek(","):
                        self.eat(",")
                        operands.append(self.predicate())
                self.eat("],")
                self.span()
                self.eat(")")
                return f"{combinator.lower()}({', '.join(sorted(operands))})"
        if self.peek("Not("):
            self.eat("Not(")
            inner = self.predicate()
            self.eat(",")
            self.span()
            self.eat(")")
            return f"not({inner})"
        if self.peek("NameValue {"):
            self.eat("NameValue {")
            self.eat("name:")
            name = self.string()
            self.eat(",")
            self.eat("value:")
            value: str | None
            if self.peek("None"):
                self.eat("None")
                value = None
            else:
                self.eat("Some(")
                value = self.string()
                self.eat(")")
            self.eat(",")
            self.eat("span:")
            self.span()
            self.eat("}")
            return name if value is None else f'{name} = "{value}"'
        raise self.fail("unknown predicate form")


def parse_cfg_attr(attr: str) -> str:
    """Parse one rustdoc `CfgTrace` attribute into a canonical predicate.

    Span text is discarded; `any`/`all` operands are sorted; several stacked
    predicates become one `all(...)`. Raises `HiddenSurfaceError` for any text
    that is not exactly this format, so a toolchain format change cannot drop
    a gate silently.
    """

    if not (attr.startswith(_CFG_PREFIX) and attr.endswith(_CFG_SUFFIX)):
        raise HiddenSurfaceError(f"unparseable cfg attribute: {attr[:80]!r}")
    body = attr[len(_CFG_PREFIX) : -len(_CFG_SUFFIX)]
    parser = _CfgTraceParser(body)
    predicates = [parser.predicate()]
    while parser.peek(","):
        parser.eat(",")
        predicates.append(parser.predicate())
    parser.skip_space()
    if parser.pos != len(body):
        raise parser.fail("trailing text")
    return _conjunction(predicates)


def _conjunction(predicates: list[str]) -> str:
    if len(predicates) == 1:
        return predicates[0]
    return f"all({', '.join(sorted(predicates))})"


def _attr_texts(item: dict[str, Any]) -> list[str]:
    texts = []
    for attr in item.get("attrs") or []:
        if isinstance(attr, dict) and isinstance(attr.get("other"), str):
            texts.append(attr["other"])
    return texts


def _is_hidden(item: dict[str, Any] | None) -> bool:
    return item is not None and "#[doc(hidden)]" in _attr_texts(item)


def _item_cfg(item: dict[str, Any] | None) -> str | None:
    if item is None:
        return None
    predicates = []
    for text in _attr_texts(item):
        if "CfgTrace" in text or text.startswith("#[cfg"):
            predicates.append(parse_cfg_attr(text))
    return _conjunction(predicates) if predicates else None


def _parse_canonical(text: str) -> Any:
    """Parse a canonical predicate string into a nested tuple form."""

    pos = 0

    def skip() -> None:
        nonlocal pos
        while pos < len(text) and text[pos] == " ":
            pos += 1

    def parse() -> Any:
        nonlocal pos
        skip()
        for combinator in ("any(", "all(", "not("):
            if text.startswith(combinator, pos):
                pos += len(combinator)
                operands = []
                skip()
                if not text.startswith(")", pos):
                    operands.append(parse())
                    skip()
                    while text.startswith(",", pos):
                        pos += 1
                        operands.append(parse())
                        skip()
                if not text.startswith(")", pos):
                    raise HiddenSurfaceError(f"bad cfg predicate: {text!r}")
                pos += 1
                return (combinator[:-1], operands)
        match = re.compile(
            r'([A-Za-z_][A-Za-z0-9_]*)(?: = "((?:[^"\\]|\\.)*)")?'
        ).match(text, pos)
        if match is None:
            raise HiddenSurfaceError(f"bad cfg predicate: {text!r}")
        pos = match.end()
        return ("atom", match.group(1), match.group(2))

    result = parse()
    skip()
    if pos != len(text):
        raise HiddenSurfaceError(f"bad cfg predicate: {text!r}")
    return result


def _eval_release(node: Any) -> bool:
    kind = node[0]
    if kind == "atom":
        name = node[1]
        if name in ("debug_assertions", "test", "feature"):
            return False
        raise HiddenSurfaceError(f"release probe cannot evaluate cfg name {name!r}")
    if kind == "not":
        (operand,) = node[1]
        return not _eval_release(operand)
    if kind == "any":
        return any(_eval_release(operand) for operand in node[1])
    return all(_eval_release(operand) for operand in node[1])


def release_enabled(preds: Iterable[str]) -> bool:
    """Evaluate a conjunction of predicates with debug_assertions, test, and
    every feature false."""

    return all(_eval_release(_parse_canonical(pred)) for pred in preds)


# --------------------------------------------------------------------------
# Reachability walk and normalization


class Record:
    """One reachable public path. `preds` is every site predicate on the path
    (ancestors, own, re-export, impl); `form` is how the release probe names
    the item, or None when it cannot."""

    __slots__ = (
        "crate",
        "path",
        "kind",
        "signature",
        "preds",
        "form",
        "trait_path",
        "owner_path",
        "owner_preds",
    )

    def __init__(
        self,
        crate: str,
        path: str,
        kind: str,
        signature: str,
        preds: tuple[str, ...],
        form: str | None,
        trait_path: str | None = None,
        owner_path: str | None = None,
        owner_preds: tuple[str, ...] = (),
    ) -> None:
        self.crate = crate
        self.path = path
        self.kind = kind
        self.signature = signature
        self.preds = preds
        self.form = form
        self.trait_path = trait_path
        self.owner_path = owner_path
        self.owner_preds = owner_preds


class _Node:
    __slots__ = (
        "item_id",
        "path",
        "use_hidden",
        "use_cfg",
        "impl_hidden",
        "impl_cfg",
        "ancestors",
        "chain",
        "assoc",
    )

    def __init__(
        self,
        item_id: str,
        path: str,
        use_hidden: bool,
        use_cfg: str | None,
        impl_hidden: bool,
        impl_cfg: str | None,
        ancestors: tuple[str, ...],
        chain: tuple[str, ...],
        assoc: bool,
    ) -> None:
        self.item_id = item_id
        self.path = path
        self.use_hidden = use_hidden
        self.use_cfg = use_cfg
        self.impl_hidden = impl_hidden
        self.impl_cfg = impl_cfg
        self.ancestors = ancestors
        self.chain = chain
        self.assoc = assoc


class _Raw:
    __slots__ = (
        "path",
        "kind",
        "item_id",
        "use_hidden",
        "use_cfg",
        "impl_hidden",
        "impl_cfg",
        "preds",
        "assoc",
        "external_source",
        "impl_owner",
    )

    def __init__(
        self,
        path: str,
        kind: str,
        item_id: str | None,
        use_hidden: bool,
        use_cfg: str | None,
        impl_hidden: bool,
        impl_cfg: str | None,
        preds: tuple[str, ...],
        assoc: bool,
        external_source: str | None = None,
        impl_owner: str | None = None,
    ) -> None:
        self.path = path
        self.kind = kind
        self.item_id = item_id
        self.use_hidden = use_hidden
        self.use_cfg = use_cfg
        self.impl_hidden = impl_hidden
        self.impl_cfg = impl_cfg
        self.preds = preds
        self.assoc = assoc
        self.external_source = external_source
        self.impl_owner = impl_owner


def _kind_of(item: dict[str, Any]) -> str:
    inner = item.get("inner")
    if not isinstance(inner, dict) or len(inner) != 1:
        raise HiddenSurfaceError(
            f"rustdoc item has no single inner kind: {item.get('id')}"
        )
    return next(iter(inner))


def _private_token(item_id: str) -> str:
    return f"\x00{item_id}\x00"


_TOKEN_IN_PATH = re.compile("\x00(\\d+)\x00")
_TOKEN_IN_JSON = re.compile(r"\\u0000(\d+)\\u0000")


def walk(doc: dict[str, Any]) -> list[Record]:
    """Return one record per public path reachable from the crate root."""

    index: dict[str, Any] = doc["index"]
    paths: dict[str, Any] = doc["paths"]
    root_id = str(doc["root"])
    crate = index[root_id]["name"]
    first_path: dict[str, str] = {}
    raws: list[_Raw] = []
    seen: set[tuple[str, str]] = set()
    heap: list[tuple[int, str, int, _Node]] = []
    counter = 0

    def push(node: _Node) -> None:
        nonlocal counter
        heapq.heappush(heap, (node.path.count("::"), node.path, counter, node))
        counter += 1

    def item(item_id: Any) -> dict[str, Any] | None:
        return index.get(str(item_id))

    def module_members(
        module_id: str, visited: frozenset[str]
    ) -> list[tuple[str, dict[str, Any], bool, str | None]]:
        """(name, member item, via-glob hidden, via-glob cfg) for public members,
        explicit names shadowing glob-imported ones."""

        module = index[module_id]["inner"]["module"]
        explicit: set[str] = set()
        for child_id in module["items"]:
            child = item(child_id)
            if child is None:
                continue
            kind = _kind_of(child)
            if kind == "use":
                if not child["inner"]["use"]["is_glob"]:
                    explicit.add(child["inner"]["use"]["name"])
            elif child.get("name"):
                explicit.add(child["name"])
        members: list[tuple[str, dict[str, Any], bool, str | None]] = []
        glob_names: set[str] = set()
        for child_id in module["items"]:
            child = item(child_id)
            if child is None or child.get("visibility") != "public":
                continue
            kind = _kind_of(child)
            if kind == "use" and child["inner"]["use"]["is_glob"]:
                target = item(child["inner"]["use"]["id"])
                if target is None or _kind_of(target) != "module":
                    members.append(("", child, False, None))
                    continue
                target_id = str(child["inner"]["use"]["id"])
                if target_id in visited:
                    continue
                for name, member, hidden, cfg in module_members(
                    target_id, visited | {target_id}
                ):
                    if not name or name in explicit or name in glob_names:
                        continue
                    glob_names.add(name)
                    glob_cfgs = [c for c in (_item_cfg(child), cfg) if c]
                    members.append(
                        (
                            name,
                            member,
                            hidden or _is_hidden(child),
                            _conjunction(glob_cfgs) if glob_cfgs else None,
                        )
                    )
                continue
            name = child["inner"]["use"]["name"] if kind == "use" else child.get("name")
            members.append((name or "", child, False, None))
        return members

    push(_Node(root_id, crate, False, None, False, None, (), (), False))
    while heap:
        _, _, _, node = heapq.heappop(heap)
        if (node.item_id, node.path) in seen:
            continue
        seen.add((node.item_id, node.path))
        current = index[node.item_id]
        kind = _kind_of(current)
        first_path.setdefault(node.item_id, node.path)
        own_cfg = _item_cfg(current)
        preds = node.ancestors + tuple(
            c for c in (own_cfg, node.use_cfg, node.impl_cfg) if c is not None
        )
        raws.append(
            _Raw(
                node.path,
                kind,
                node.item_id,
                node.use_hidden,
                node.use_cfg,
                node.impl_hidden,
                node.impl_cfg,
                preds,
                node.assoc,
            )
        )
        if node.item_id in node.chain:
            continue
        chain = node.chain + (node.item_id,)
        inner = current["inner"][kind]

        def child(child_id: Any, path: str, **flags: Any) -> None:
            push(
                _Node(
                    str(child_id),
                    path,
                    flags.get("use_hidden", False),
                    flags.get("use_cfg"),
                    flags.get("impl_hidden", False),
                    flags.get("impl_cfg"),
                    preds,
                    chain,
                    flags.get("assoc", False),
                )
            )

        if kind == "module":
            for name, member, glob_hidden, glob_cfg in module_members(
                node.item_id, frozenset({node.item_id})
            ):
                member_kind = _kind_of(member)
                if member_kind != "use":
                    child(
                        member["id"],
                        f"{node.path}::{name}",
                        use_hidden=glob_hidden,
                        use_cfg=glob_cfg,
                    )
                    continue
                use = member["inner"]["use"]
                use_hidden = glob_hidden or _is_hidden(member)
                use_cfgs = [c for c in (glob_cfg, _item_cfg(member)) if c]
                use_cfg = _conjunction(use_cfgs) if use_cfgs else None
                if use["is_glob"]:
                    raws.append(
                        _Raw(
                            f"{node.path}::external-glob:{use['source']}",
                            "external-glob",
                            None,
                            use_hidden,
                            use_cfg,
                            False,
                            None,
                            preds + ((use_cfg,) if use_cfg else ()),
                            False,
                            external_source=use["source"],
                        )
                    )
                    continue
                if use["id"] is not None and item(use["id"]) is not None:
                    child(
                        use["id"],
                        f"{node.path}::{use['name']}",
                        use_hidden=use_hidden,
                        use_cfg=use_cfg,
                    )
                    continue
                raws.append(
                    _Raw(
                        f"{node.path}::{use['name']}",
                        "external",
                        None,
                        use_hidden,
                        use_cfg,
                        False,
                        None,
                        preds + ((use_cfg,) if use_cfg else ()),
                        False,
                        external_source=use["source"],
                    )
                )
        elif kind in ("struct", "union", "enum"):
            if kind == "struct":
                shape = inner["kind"]
                if isinstance(shape, dict) and "plain" in shape:
                    fields = shape["plain"]["fields"]
                elif isinstance(shape, dict) and "tuple" in shape:
                    fields = [f for f in shape["tuple"] if f is not None]
                else:
                    fields = []
            elif kind == "union":
                fields = inner["fields"]
            else:
                fields = inner["variants"]
            for field_id in fields:
                field = index[str(field_id)]
                if kind != "enum" and field.get("visibility") != "public":
                    continue
                child(field_id, f"{node.path}::{field['name']}")
            for impl_id in inner.get("impls", []):
                impl_item = index[str(impl_id)]
                impl = impl_item["inner"]["impl"]
                if impl["is_synthetic"] or impl["blanket_impl"] is not None:
                    continue
                impl_hidden = _is_hidden(impl_item)
                impl_cfg = _item_cfg(impl_item)
                if impl["trait"] is None:
                    for assoc_id in impl["items"]:
                        assoc = index[str(assoc_id)]
                        if assoc.get("visibility") != "public":
                            continue
                        child(
                            assoc_id,
                            f"{node.path}::{assoc['name']}",
                            impl_hidden=impl_hidden,
                            impl_cfg=impl_cfg,
                            assoc=True,
                        )
                    continue
                raws.append(
                    _Raw(
                        node.path,
                        "impl",
                        str(impl_id),
                        False,
                        None,
                        impl_hidden,
                        impl_cfg,
                        preds + ((impl_cfg,) if impl_cfg else ()),
                        False,
                        impl_owner=node.path,
                    )
                )
        elif kind == "trait":
            for assoc_id in inner["items"]:
                assoc = index[str(assoc_id)]
                child(assoc_id, f"{node.path}::{assoc['name']}", assoc=True)

    for impl_id, impl_item in index.items():
        if impl_item.get("crate_id") != 0 or _kind_of(impl_item) != "impl":
            continue
        impl = impl_item["inner"]["impl"]
        if impl["is_synthetic"] or impl["blanket_impl"] is not None:
            continue
        if not (isinstance(impl["for"], dict) and "generic" in impl["for"]):
            continue
        impl_cfg = _item_cfg(impl_item)
        raws.append(
            _Raw(
                crate,
                "impl",
                impl_id,
                False,
                None,
                _is_hidden(impl_item),
                impl_cfg,
                (impl_cfg,) if impl_cfg else (),
                False,
            )
        )

    return _finish(crate, index, paths, first_path, raws)


def _finish(
    crate: str,
    index: dict[str, Any],
    paths: dict[str, Any],
    first_path: dict[str, str],
    raws: list[_Raw],
) -> list[Record]:
    private_names: dict[str, str] = {}

    def reference(item_id: Any) -> str:
        key = str(item_id)
        if key in first_path:
            return first_path[key]
        summary = paths.get(key)
        local = index.get(key)
        if summary is not None and summary.get("crate_id") != 0:
            return "::".join(summary["path"])
        if summary is None and local is None:
            raise HiddenSurfaceError(
                f"rustdoc reference {key} is in neither index nor paths"
            )
        if summary is not None:
            private_names[key] = f"private:{summary['kind']}:{summary['path'][-1]}"
        else:
            private_names[key] = f"private:{_kind_of(local)}:{local['name']}"
        return _private_token(key)

    def normalize(value: Any) -> Any:
        if isinstance(value, dict):
            result = {k: normalize(v) for k, v in value.items() if k not in STRIP_KEYS}
            if isinstance(value.get("id"), int) and isinstance(value.get("path"), str):
                result["path"] = reference(value["id"])
            return result
        if isinstance(value, list):
            return [normalize(v) for v in value]
        return value

    def field_list(field_ids: list[Any], with_type: bool) -> list[Any]:
        result = []
        for field_id in field_ids:
            field = index[str(field_id)]
            entry: list[Any] = [field["name"]]
            if with_type:
                entry.append(normalize(field["inner"]["struct_field"]))
            entry.extend([_is_hidden(field), _item_cfg(field)])
            result.append(entry)
        return result

    def tuple_types(field_ids: list[Any]) -> list[Any]:
        return [
            None if f is None else normalize(index[str(f)]["inner"]["struct_field"])
            for f in field_ids
        ]

    def names(ids: list[Any]) -> list[str]:
        return sorted(index[str(i)].get("name") or "" for i in ids)

    def normalized_inner(kind: str, inner: dict[str, Any]) -> Any:
        body = json.loads(json.dumps(inner[kind]))
        if kind in ("struct", "union", "enum", "primitive"):
            body.pop("impls", None)
        if kind == "struct":
            shape = body["kind"]
            if isinstance(shape, dict) and "plain" in shape:
                shape["plain"]["fields"] = field_list(shape["plain"]["fields"], False)
            elif isinstance(shape, dict) and "tuple" in shape:
                shape["tuple"] = tuple_types(shape["tuple"])
        elif kind == "union":
            body["fields"] = field_list(body["fields"], False)
        elif kind == "enum":
            body["variants"] = names(body["variants"])
        elif kind == "variant":
            shape = body["kind"]
            if isinstance(shape, dict) and "tuple" in shape:
                shape["tuple"] = tuple_types(shape["tuple"])
            elif isinstance(shape, dict) and "struct" in shape:
                shape["struct"]["fields"] = field_list(shape["struct"]["fields"], True)
        elif kind == "trait":
            body["items"] = names(body["items"])
            body.pop("implementations", None)
        elif kind == "impl":
            body["items"] = names(body["items"])
        elif kind == "module":
            body.pop("items", None)
        elif kind == "constant":
            body.get("const", {}).pop("expr", None)
        return {kind: normalize(body)}

    staged: list[tuple[_Raw, str, str, str | None]] = []
    for raw in raws:
        path = raw.path
        trait_path: str | None = None
        if raw.kind in ("external", "external-glob"):
            signature = _compact(
                {
                    "source": raw.external_source,
                    "use_cfg": raw.use_cfg,
                    "use_hidden": raw.use_hidden,
                }
            )
            staged.append((raw, path, signature, None))
            continue
        assert raw.item_id is not None
        current = index[raw.item_id]
        inner = normalized_inner(raw.kind, current["inner"])
        if raw.kind == "impl":
            impl = inner["impl"]
            trait_path = _render_path(impl["trait"]) if impl["trait"] else "?"
            if impl.get("is_negative"):
                trait_path = "!" + trait_path
            if raw.impl_owner is not None:
                path = f"{raw.impl_owner}::impl {trait_path}"
            else:
                path = f"{crate}::impl {trait_path} for {_render_type(impl['for'])}"
            own_hidden, own_cfg = False, None
        else:
            own_hidden, own_cfg = _is_hidden(current), _item_cfg(current)
        signature = _compact(
            {
                "visibility": current.get("visibility"),
                "own_hidden": own_hidden,
                "use_hidden": raw.use_hidden,
                "impl_hidden": raw.impl_hidden,
                "own_cfg": own_cfg,
                "use_cfg": raw.use_cfg,
                "impl_cfg": raw.impl_cfg,
                "inner": inner,
            }
        )
        staged.append((raw, path, signature, trait_path))

    def base_name(match: re.Match[str]) -> str:
        return private_names[match.group(1)]

    def plain(text: str, pattern: re.Pattern[str]) -> str:
        return pattern.sub(base_name, text)

    staged.sort(
        key=lambda s: (
            plain(s[1], _TOKEN_IN_PATH),
            s[0].kind,
            plain(s[2], _TOKEN_IN_JSON),
        )
    )
    encounter: list[str] = []
    for _, path, signature, _ in staged:
        for pattern, text in ((_TOKEN_IN_PATH, path), (_TOKEN_IN_JSON, signature)):
            for match in pattern.finditer(text):
                if match.group(1) not in encounter:
                    encounter.append(match.group(1))
    groups: dict[str, list[str]] = {}
    for item_id in encounter:
        groups.setdefault(private_names[item_id], []).append(item_id)
    final_names: dict[str, str] = {}
    for name, ids in groups.items():
        for number, item_id in enumerate(ids, start=1):
            final_names[item_id] = name if len(ids) == 1 else f"{name}#{number}"

    def final(match: re.Match[str]) -> str:
        return final_names[match.group(1)]

    records = []
    for raw, path, signature, trait_path in staged:
        form: str | None
        if raw.kind == "impl":
            bound_ok = (
                raw.impl_owner is not None
                and trait_path is not None
                and "\x00" not in trait_path
                and "json:" not in trait_path
                and not trait_path.startswith(("!", "?"))
                and not index[str(raw.item_id)]["inner"]["impl"]["generics"]["params"]
            )
            form = "bound" if bound_ok else None
        elif raw.assoc:
            form = (
                "value" if raw.kind in ("function", "constant", "assoc_const") else None
            )
        else:
            form = "use" if raw.kind in USE_FORM_KINDS else None
        records.append(
            Record(
                crate=crate,
                path=_TOKEN_IN_PATH.sub(final, path),
                kind=raw.kind,
                signature=_TOKEN_IN_JSON.sub(final, signature),
                preds=raw.preds,
                form=form,
                trait_path=None
                if trait_path is None
                else _TOKEN_IN_PATH.sub(final, trait_path),
                owner_path=raw.impl_owner,
                owner_preds=raw.preds[:-1] if raw.impl_cfg else raw.preds,
            )
        )
    return records


def _render_type(value: Any) -> str:
    if value is None:
        return "_"
    if not isinstance(value, dict) or len(value) != 1:
        return "json:" + _compact(value)
    kind, body = next(iter(value.items()))
    if kind == "resolved_path":
        return _render_path(body)
    if kind in ("primitive", "generic"):
        return str(body)
    if kind == "infer":
        return "_"
    if kind == "tuple":
        inner = ", ".join(_render_type(t) for t in body)
        return f"({inner},)" if len(body) == 1 else f"({inner})"
    if kind == "slice":
        return f"[{_render_type(body)}]"
    if kind == "array":
        return f"[{_render_type(body['type'])}; {body['len']}]"
    if kind == "borrowed_ref":
        lifetime = f"{body['lifetime']} " if body.get("lifetime") else ""
        mutable = "mut " if body.get("is_mutable") else ""
        return f"&{lifetime}{mutable}{_render_type(body['type'])}"
    if kind == "raw_pointer":
        pointer = "*mut " if body.get("is_mutable") else "*const "
        return pointer + _render_type(body["type"])
    return "json:" + _compact(value)


def _render_path(path: dict[str, Any]) -> str:
    base = str(path["path"])
    args = path.get("args")
    if not args:
        return base
    if "angle_bracketed" not in args:
        return base + "json:" + _compact(args)
    parts = []
    for arg in args["angle_bracketed"]["args"]:
        if isinstance(arg, dict) and "type" in arg:
            parts.append(_render_type(arg["type"]))
        elif isinstance(arg, dict) and "lifetime" in arg:
            parts.append(str(arg["lifetime"]))
        else:
            parts.append("json:" + _compact(arg))
    for constraint in args["angle_bracketed"].get("constraints", []):
        parts.append("json:" + _compact(constraint))
    return f"{base}<{', '.join(parts)}>" if parts else base


def rustdoc_entries(doc: dict[str, Any]) -> list[dict[str, str]]:
    """Return the sorted, de-duplicated `{path, kind, signature}` row entries."""

    return _unique(
        ({"path": r.path, "kind": r.kind, "signature": r.signature} for r in walk(doc)),
        "rustdoc",
    )


def check_format_version(doc: dict[str, Any]) -> None:
    version = doc.get("format_version")
    if version != FORMAT_VERSION:
        raise HiddenSurfaceError(
            f"rustdoc format_version {version!r} is not the pinned {FORMAT_VERSION}"
        )


def load_rustdoc(path: Path) -> dict[str, Any]:
    """Read a rustdoc JSON file and require the pinned format version."""

    try:
        doc = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise HiddenSurfaceError(f"cannot read rustdoc JSON {path}: {exc}") from exc
    if not isinstance(doc, dict):
        raise HiddenSurfaceError(f"rustdoc JSON root must be an object: {path}")
    check_format_version(doc)
    return doc


# --------------------------------------------------------------------------
# Release probe


class ProbeItem:
    """One item the release probe names, with its expected release status."""

    __slots__ = (
        "path",
        "kind",
        "form",
        "expected",
        "trait_path",
        "owner_path",
    )

    def __init__(
        self,
        path: str,
        kind: str,
        form: str | None,
        expected: str,
        trait_path: str | None = None,
        owner_path: str | None = None,
    ) -> None:
        self.path = path
        self.kind = kind
        self.form = form
        self.expected = expected
        self.trait_path = trait_path
        self.owner_path = owner_path


def select_probe_items(
    records: Iterable[Record], release_only: Iterable[dict[str, str]]
) -> list[ProbeItem]:
    """Debug-only items (expected unresolved) plus curated release-only items
    (expected resolved). Struct fields and enum variants are covered by their
    recorded predicates only."""

    items = []
    for r in records:
        if r.kind in ("struct_field", "variant") or release_enabled(r.preds):
            continue
        form = r.form
        if r.kind == "impl" and r.owner_path and not release_enabled(r.owner_preds):
            # The owner is itself absent in release, which proves the impl
            # absent; canonical trait paths can be private or unstable
            # (`core::ops::drop::Drop`, `core::marker::StructuralPartialEq`).
            form = "owner"
        items.append(
            ProbeItem(r.path, r.kind, form, "unresolved", r.trait_path, r.owner_path)
        )
    items.extend(
        ProbeItem(entry["path"], entry["kind"], "use", "resolved")
        for entry in release_only
    )
    return sorted(items, key=lambda item: (item.path, item.kind))


_PROBE_HEADER = (
    "// @generated by dev/tools/hidden_surface.py (release probe); do not edit.\n"
    "#![allow(dead_code, unused_imports, unused_variables)]\n"
    "\n"
    "fn main() {}\n"
)


def generate_probe(items: Sequence[ProbeItem]) -> tuple[str, list[tuple[int, int]]]:
    """Return the probe example source and each item's function line range."""

    lines = _PROBE_HEADER.splitlines()
    ranges: list[tuple[int, int]] = []
    for number, item in enumerate(items):
        if item.form == "use":
            body = [f"    use {item.path} as _;"]
        elif item.form == "value":
            body = [f"    let _ = {item.path};"]
        elif item.form == "owner" and item.owner_path:
            body = [f"    use {item.owner_path} as _;"]
        elif item.form == "bound" and item.trait_path and item.owner_path:
            body = [
                f"    fn hs_bound<T: ?Sized + {item.trait_path}>() {{}}",
                f"    hs_bound::<{item.owner_path}>();",
            ]
        else:
            raise HiddenSurfaceError(
                f"release probe cannot name {item.kind} {item.path}"
            )
        lines.extend(["", f"// hs_probe {number}: {item.kind} {item.path}"])
        start = len(lines) + 1
        lines.append(f"fn hs_probe_{number}() {{")
        lines.extend(body)
        lines.append("}")
        ranges.append((start, len(lines)))
    return "\n".join(lines) + "\n", ranges


def probe_statuses(messages: str, ranges: Sequence[tuple[int, int]]) -> list[str]:
    """Map `cargo check --message-format json` output onto probe functions.

    Every error must carry an expected code and a primary span inside one probe
    function; anything else fails closed.
    """

    unresolved: set[int] = set()
    for line in messages.splitlines():
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if record.get("reason") != "compiler-message":
            continue
        message = record.get("message") or {}
        if message.get("level") not in ("error", "error: internal compiler error"):
            continue
        code = (message.get("code") or {}).get("code")
        text = str(message.get("message", ""))
        spans = message.get("spans") or []
        if code is None and not spans and text.startswith("aborting due to"):
            continue
        if (record.get("target") or {}).get("name") != "hs_probe":
            raise HiddenSurfaceError(
                f"release probe build error outside the probe: {text}"
            )
        if code not in PROBE_ERROR_CODES:
            raise HiddenSurfaceError(
                f"release probe: unexpected diagnostic {code}: {text}"
            )
        primary = [
            span
            for span in spans
            if span.get("is_primary")
            and str(span.get("file_name", "")).endswith("examples/hs_probe.rs")
        ]
        if not primary:
            raise HiddenSurfaceError(
                f"release probe diagnostic has no probe span: {text}"
            )
        for span in primary:
            line_number = span["line_start"]
            owner = next(
                (
                    n
                    for n, (start, end) in enumerate(ranges)
                    if start <= line_number <= end
                ),
                None,
            )
            if owner is None:
                raise HiddenSurfaceError(
                    f"release probe diagnostic outside every probe function (line {line_number}): {text}"
                )
            unresolved.add(owner)
    return ["unresolved" if n in unresolved else "resolved" for n in range(len(ranges))]


def probe_row_entries(
    items: Sequence[ProbeItem], statuses: Sequence[str]
) -> list[dict[str, str]]:
    """Build release-probe entries; a status that contradicts its predicate
    fails the capture."""

    if len(items) != len(statuses):
        raise HiddenSurfaceError("release probe status count does not match its items")
    mismatches = [
        f"{item.path}: expected {item.expected}, got {status}"
        for item, status in zip(items, statuses)
        if item.expected != status
    ]
    if mismatches:
        raise HiddenSurfaceError(
            "release probe contradicts predicates:\n" + "\n".join(mismatches)
        )
    return [
        {"path": item.path, "kind": "release-probe", "signature": status}
        for item, status in zip(items, statuses)
    ]


_ITEM_AFTER_ATTRS = re.compile(
    r"pub\s+(?:(?:unsafe|async|const|extern(?:\s+\"[^\"]*\")?)\s+)*"
    r"(fn|mod|struct|enum|union|trait|type|const|static|use|macro)\s+"
    r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)"
)


def _skip_trivia(text: str, pos: int) -> int:
    while pos < len(text):
        if text[pos].isspace():
            pos += 1
        elif text.startswith("//", pos):
            end = text.find("\n", pos)
            pos = len(text) if end < 0 else end + 1
        elif text.startswith("/*", pos):
            end = text.find("*/", pos)
            pos = len(text) if end < 0 else end + 2
        elif text.startswith("#[", pos):
            pos = _balanced_end(text, pos + 1, "[", "]")
        else:
            break
    return pos


def _balanced_end(text: str, pos: int, opening: str, closing: str) -> int:
    depth = 0
    while pos < len(text):
        if text[pos] == opening:
            depth += 1
        elif text[pos] == closing:
            depth -= 1
            if depth == 0:
                return pos + 1
        pos += 1
    return pos


def release_only_names(sources: dict[str, str]) -> set[str]:
    """Names of `pub` items (not `pub(...)`) gated on `not(debug_assertions)`."""

    found: set[str] = set()
    for text in sources.values():
        for match in re.finditer(r"#\[cfg\(", text):
            end = _balanced_end(text, match.start() + 1, "[", "]")
            attribute = re.sub(r"\s+", "", text[match.start() : end])
            if "not(debug_assertions)" not in attribute:
                continue
            item = _ITEM_AFTER_ATTRS.match(text, _skip_trivia(text, end))
            if item is not None:
                found.add(item.group(2).rsplit("::", 1)[-1])
    return found


def check_release_only(
    sources: dict[str, str], curated: Iterable[dict[str, str]]
) -> None:
    """Fail when a public release-only item is missing from the curated list."""

    listed = {entry["path"].rsplit("::", 1)[-1] for entry in curated}
    missing = sorted(release_only_names(sources) - listed)
    if missing:
        raise HiddenSurfaceError(
            "public items gated on not(debug_assertions) are missing from "
            "RELEASE_ONLY: " + ", ".join(missing)
        )


# --------------------------------------------------------------------------
# Manifests


def build_manifest(
    metadata: dict[str, Any], rows: dict[str, list[dict[str, str]]]
) -> dict[str, Any]:
    """Assemble a manifest; rows keep the given order, entries are sorted."""

    return {
        "metadata": json.loads(json.dumps(metadata)),
        "rows": [
            {"id": row_id, "entries": _unique(entries, row_id)}
            for row_id, entries in rows.items()
        ],
    }


def compare(baseline: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any]:
    """Compare two hidden-surface manifests; other schemas are rejected."""

    for label, value in (("baseline", baseline), ("candidate", candidate)):
        metadata = value.get("metadata") if isinstance(value, dict) else None
        schema = metadata.get("schema") if isinstance(metadata, dict) else None
        if schema != SCHEMA:
            raise HiddenSurfaceError(f"{label} schema {schema!r} is not {SCHEMA}")
    try:
        return compare_manifests(baseline, candidate)
    except _comparator.ComparatorError as exc:
        raise HiddenSurfaceError(str(exc)) from exc


def _load_manifest(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise HiddenSurfaceError(f"cannot read manifest {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise HiddenSurfaceError(f"manifest {path} root must be an object")
    return value


def exclusive_write(output: Path, contents: str) -> None:
    """Create `output` with `contents`; never replace an existing file."""

    output.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=output.parent,
            prefix=f".{output.name}.",
            suffix=".tmp",
            delete=False,
        ) as handle:
            temporary = Path(handle.name)
            handle.write(contents)
            handle.flush()
            os.fsync(handle.fileno())
        umask = os.umask(0)
        os.umask(umask)
        os.chmod(temporary, 0o666 & ~umask)
        try:
            os.link(temporary, output)
        except FileExistsError as exc:
            raise HiddenSurfaceError(
                f"{output} already exists; captures never overwrite a file"
            ) from exc
        except OSError as exc:
            raise HiddenSurfaceError(f"cannot write {output}: {exc}") from exc
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def guard_output(output: Path, source_modified: bool) -> None:
    """A `--source-dir` capture may never be written under the baseline
    directory, so an edited tree cannot become a baseline."""

    if not source_modified:
        return
    resolved = Path(os.path.realpath(output))
    baseline_dir = Path(os.path.realpath(BASELINE_DIR))
    if resolved == baseline_dir or baseline_dir in resolved.parents:
        raise HiddenSurfaceError(
            f"a source_modified capture cannot be written under {BASELINE_DIR}"
        )


# --------------------------------------------------------------------------
# Source selection, ownership, and capture


def _git(arguments: Sequence[str]) -> str:
    if not arguments or arguments[0] not in GIT_SUBCOMMANDS:
        raise HiddenSurfaceError(f"git subcommand not permitted: {list(arguments)}")
    return _run(["git", *arguments], REPO_ROOT)


def validate_source_sha(sha: str) -> str:
    """Require a full lowercase 40-hex SHA that names a commit."""

    if re.fullmatch(r"[0-9a-f]{40}", sha) is None:
        raise HiddenSurfaceError(
            "--source-sha must be a full lowercase 40-character SHA"
        )
    try:
        object_type = _git(["cat-file", "-t", sha]).strip()
    except HiddenSurfaceError as exc:
        raise HiddenSurfaceError(f"{sha} is not a commit in this repository") from exc
    if object_type != "commit":
        raise HiddenSurfaceError(f"{sha} is a {object_type}, not a commit")
    return sha


_EXPORT_NAME = re.compile(r"([0-9a-f]{40})-([1-9][0-9]*)")


def validate_source_dir(path: Path, must_exist: bool = True) -> str:
    """Accept only `<root>/exports/<sha>-<n>`; return its SHA."""

    absolute = Path(os.path.abspath(path))
    exports = OWNED_ROOT / "exports"
    match = _EXPORT_NAME.fullmatch(absolute.name)
    if absolute.parent != exports or match is None:
        raise HiddenSurfaceError(
            f"--source-dir must be an export under {exports}: {path}"
        )
    if must_exist:
        if absolute.is_symlink() or not absolute.is_dir():
            raise HiddenSurfaceError(f"--source-dir is not an export directory: {path}")
        marker = absolute / EXPORT_MARKER
        if not marker.is_file() or marker.read_text() != match.group(1) + "\n":
            raise HiddenSurfaceError(
                f"--source-dir export marker is missing or wrong: {path}"
            )
    return match.group(1)


def _cargo_env() -> dict[str, str]:
    env = {k: v for k, v in os.environ.items() if k not in SCRUBBED_ENV}
    env["CARGO_TARGET_DIR"] = str(OWNED_CACHE)
    env["RUSTUP_AUTO_INSTALL"] = "0"
    return env


def check_toolchain() -> dict[str, str]:
    """Require the pinned nightly; return its rustc version and host target."""

    env = {**os.environ, "RUSTUP_AUTO_INSTALL": "0"}
    env.pop("RUSTUP_TOOLCHAIN", None)
    try:
        rustc = _run(
            ["rustc", f"+{PINNED_NIGHTLY}", "--version"], REPO_ROOT, env
        ).strip()
        verbose = _run(["rustc", f"+{PINNED_NIGHTLY}", "-vV"], REPO_ROOT, env)
        _run(["cargo", f"+{PINNED_NIGHTLY}", "--version"], REPO_ROOT, env)
    except HiddenSurfaceError as exc:
        raise HiddenSurfaceError(
            f"pinned toolchain {PINNED_NIGHTLY} is unavailable: {exc}"
        ) from exc
    host = next(
        (
            line.split(":", 1)[1].strip()
            for line in verbose.splitlines()
            if line.startswith("host:")
        ),
        None,
    )
    if not host:
        raise HiddenSurfaceError(
            f"cannot read the host target from rustc +{PINNED_NIGHTLY} -vV"
        )
    return {"rustc": rustc, "target": host}


def _existing_parent(path: Path) -> Path:
    while not path.exists():
        path = path.parent
    return path


def _check_capacity() -> None:
    for label, path in (("cache", OWNED_CACHE), ("scratch", OWNED_ROOT)):
        free = shutil.disk_usage(_existing_parent(path)).free
        if free < MIN_FREE_BYTES:
            raise HiddenSurfaceError(
                f"{label} filesystem has {free} free bytes; {MIN_FREE_BYTES} required"
            )


def _ensure_owned_root() -> None:
    if not os.path.lexists(OWNED_ROOT):
        OWNED_ROOT.mkdir(parents=True)
        (OWNED_ROOT / ROOT_MARKER).write_text(ROOT_MARKER_CONTENT)
        return
    if not stat.S_ISDIR(OWNED_ROOT.lstat().st_mode):
        raise HiddenSurfaceError(f"owned root must be a real directory: {OWNED_ROOT}")
    marker = OWNED_ROOT / ROOT_MARKER
    if (
        not os.path.lexists(marker)
        or not stat.S_ISREG(marker.lstat().st_mode)
        or marker.read_text() != ROOT_MARKER_CONTENT
    ):
        raise HiddenSurfaceError(f"owned root marker is missing or invalid: {marker}")


@contextlib.contextmanager
def _owned_lock() -> Iterator[None]:
    _ensure_owned_root()
    with open(OWNED_ROOT / ".lock", "w", encoding="utf-8") as handle:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def _export(sha: str, destination: Path) -> None:
    destination.mkdir(parents=True)
    archive_command = ["git", "archive", "--format=tar", sha]
    if archive_command[1] not in GIT_SUBCOMMANDS:
        raise HiddenSurfaceError("git archive is not permitted")
    with subprocess.Popen(
        archive_command, cwd=REPO_ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    ) as archive:
        extract = subprocess.run(
            ["tar", "-x", "-m", "-C", str(destination)],
            stdin=archive.stdout,
            capture_output=True,
            check=False,
        )
        assert archive.stdout is not None
        archive.stdout.close()
        archive_error = archive.stderr.read().decode() if archive.stderr else ""
        archive_status = archive.wait()
    if archive_status != 0 or extract.returncode != 0:
        raise HiddenSurfaceError(
            f"export of {sha} failed: git archive {archive_status}: {archive_error.strip()}; "
            f"tar {extract.returncode}: {extract.stderr.decode().strip()}"
        )


def _tree_sha256(tree: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(tree.rglob("*")):
        relative = path.relative_to(tree).as_posix()
        if relative == EXPORT_MARKER:
            continue
        if path.is_symlink():
            digest.update(f"L {relative}\0{os.readlink(path)}\0".encode())
        elif path.is_file():
            digest.update(f"F {relative}\0".encode())
            digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def rustdoc_arguments(row: dict[str, Any]) -> list[str]:
    """The exact cargo command for one rustdoc row (no absolute paths)."""

    command = [
        "cargo",
        f"+{PINNED_NIGHTLY}",
        "rustdoc",
        "--locked",
        "-p",
        row["crate"],
        "--lib",
        "--no-default-features",
    ]
    if row["features"]:
        command.extend(["--features", ",".join(row["features"])])
    command.extend(
        [
            "--",
            "-Z",
            "unstable-options",
            "--output-format",
            "json",
            "--document-hidden-items",
        ]
    )
    return command


def probe_arguments(package: str) -> list[str]:
    """The exact cargo command for one crate's release probe."""

    return [
        "cargo",
        f"+{PINNED_NIGHTLY}",
        "check",
        "--release",
        "--locked",
        "--offline",
        "-p",
        package,
        "--example",
        "hs_probe",
        "--message-format",
        "json",
    ]


def row_identities() -> list[dict[str, Any]]:
    identities: list[dict[str, Any]] = [
        {
            "id": row["id"],
            "crate": row["crate"],
            "features": list(row["features"]),
            "profile": "dev",
            "cargo_arguments": rustdoc_arguments(row),
        }
        for row in ROWS
    ]
    identities.append(
        {
            "id": RELEASE_PROBE_ROW,
            "crate": [package for package, _, _ in PROBE_CRATES],
            "features": [],
            "profile": "release",
            "cargo_arguments": [
                probe_arguments(package) for package, _, _ in PROBE_CRATES
            ],
        }
    )
    return identities


def _rust_sources(directory: Path) -> dict[str, str]:
    return {
        path.as_posix(): path.read_text(encoding="utf-8")
        for path in sorted(directory.rglob("*.rs"))
    }


def _run_probe(
    tree: Path, package: str, crate_dir: str, items: list[ProbeItem]
) -> list[dict[str, str]]:
    source, ranges = generate_probe(items)
    examples = tree / crate_dir / "examples"
    created_examples = not examples.exists()
    probe = examples / "hs_probe.rs"
    if os.path.lexists(probe):
        raise HiddenSurfaceError(f"refusing to overwrite an existing {probe}")
    examples.mkdir(parents=True, exist_ok=True)
    probe.write_text(source)
    try:
        completed = subprocess.run(
            probe_arguments(package),
            cwd=tree,
            env=_cargo_env(),
            check=False,
            text=True,
            capture_output=True,
        )
    finally:
        probe.unlink()
        if created_examples:
            examples.rmdir()
    statuses = probe_statuses(completed.stdout, ranges)
    if completed.returncode != 0 and "unresolved" not in statuses:
        raise HiddenSurfaceError(
            f"release probe for {package} failed ({completed.returncode}):\n{completed.stderr[-4000:]}"
        )
    if completed.returncode == 0 and "unresolved" in statuses:
        raise HiddenSurfaceError(
            f"release probe for {package} reported errors but exited 0"
        )
    return probe_row_entries(items, statuses)


def capture_tree(
    tree: Path,
    source_sha: str,
    toolchain: dict[str, str],
    source_modified: bool,
) -> dict[str, Any]:
    """Build every row from an exported tree and return the manifest."""

    metadata: dict[str, Any] = {
        "schema": SCHEMA,
        "capture_source_sha": source_sha,
        "source_modified": source_modified,
        "tools": {"rust-toolchain": PINNED_NIGHTLY, "rustc": toolchain["rustc"]},
        "rustdoc_format_version": FORMAT_VERSION,
        "target": toolchain["target"],
        "row_identities": row_identities(),
    }
    if source_modified:
        metadata["source_tree_sha256"] = _tree_sha256(tree)
    env = _cargo_env()
    rows: dict[str, list[dict[str, str]]] = {}
    default_records: dict[str, list[Record]] = {}
    for row in ROWS:
        output = OWNED_CACHE / "doc" / f"{row['crate'].replace('-', '_')}.json"
        output.unlink(missing_ok=True)
        _run(rustdoc_arguments(row), tree, env)
        doc = load_rustdoc(output)
        if doc.get("target", {}).get("triple") != toolchain["target"]:
            raise HiddenSurfaceError(
                f"row {row['id']} target {doc.get('target')} is not {toolchain['target']}"
            )
        records = walk(doc)
        rows[row["id"]] = [
            {"path": r.path, "kind": r.kind, "signature": r.signature} for r in records
        ]
        default_records[row["id"]] = records
        print(f"captured {row['id']}: {len(rows[row['id']])} entries", file=sys.stderr)
    sources: dict[str, str] = {}
    for _, crate_dir, _ in PROBE_CRATES:
        sources.update(_rust_sources(tree / crate_dir / "src"))
    check_release_only(sources, RELEASE_ONLY)
    probe_entries: list[dict[str, str]] = []
    for package, crate_dir, default_row in PROBE_CRATES:
        curated = [entry for entry in RELEASE_ONLY if entry["crate"] == package]
        items = select_probe_items(default_records[default_row], curated)
        probe_entries.extend(_run_probe(tree, package, crate_dir, items))
        print(f"probed {package}: {len(items)} items", file=sys.stderr)
    rows[RELEASE_PROBE_ROW] = probe_entries
    return build_manifest(metadata, rows)


def capture_source_sha(sha: str) -> dict[str, Any]:
    """Capture a commit from a fresh export at the stable `<root>/src`."""

    toolchain = check_toolchain()
    _check_capacity()
    OWNED_CACHE.mkdir(parents=True, exist_ok=True)
    with _owned_lock():
        tree = OWNED_ROOT / "src"
        if os.path.lexists(tree):
            shutil.rmtree(tree)
        _export(sha, tree)
        return capture_tree(tree, sha, toolchain, source_modified=False)


def capture_source_dir(directory: Path) -> dict[str, Any]:
    """Capture an owned (possibly edited) export; labelled source_modified."""

    sha = validate_source_dir(directory)
    toolchain = check_toolchain()
    _check_capacity()
    OWNED_CACHE.mkdir(parents=True, exist_ok=True)
    with _owned_lock():
        return capture_tree(
            Path(os.path.abspath(directory)), sha, toolchain, source_modified=True
        )


def export_source(sha: str) -> Path:
    """Export a commit to a new `<root>/exports/<sha>-<n>` directory."""

    with _owned_lock():
        exports = OWNED_ROOT / "exports"
        exports.mkdir(exist_ok=True)
        number = 1
        while os.path.lexists(exports / f"{sha}-{number}"):
            number += 1
        destination = exports / f"{sha}-{number}"
        _export(sha, destination)
        (destination / EXPORT_MARKER).write_text(sha + "\n")
        return destination


def discard_source(directory: Path) -> None:
    validate_source_dir(directory)
    with _owned_lock():
        shutil.rmtree(Path(os.path.abspath(directory)))


def prune() -> None:
    """Delete the build cache and every owned export."""

    cache = OWNED_CACHE.parent
    if os.path.lexists(cache):
        shutil.rmtree(cache)
    if os.path.lexists(OWNED_ROOT):
        with _owned_lock():
            for child in OWNED_ROOT.iterdir():
                if child.name in (ROOT_MARKER, ".lock"):
                    continue
                if child.is_dir() and not child.is_symlink():
                    shutil.rmtree(child)
                else:
                    child.unlink()


# --------------------------------------------------------------------------
# Command line


def _capture_command(args: argparse.Namespace) -> int:
    output = Path(os.path.abspath(args.output))
    if args.source_sha is not None:
        sha = validate_source_sha(args.source_sha)
        guard_output(output, source_modified=False)
        if os.path.lexists(output):
            raise HiddenSurfaceError(
                f"{output} already exists; captures never overwrite a file"
            )
        manifest = capture_source_sha(sha)
    else:
        validate_source_dir(args.source_dir)
        guard_output(output, source_modified=True)
        if os.path.lexists(output):
            raise HiddenSurfaceError(
                f"{output} already exists; captures never overwrite a file"
            )
        manifest = capture_source_dir(args.source_dir)
    exclusive_write(output, canonical_json(manifest))
    print(f"captured {len(manifest['rows'])} rows to {output}")
    return 0


def _export_command(args: argparse.Namespace) -> int:
    print(export_source(validate_source_sha(args.source_sha)))
    return 0


def _discard_command(args: argparse.Namespace) -> int:
    discard_source(args.source_dir)
    return 0


def _prune_command(args: argparse.Namespace) -> int:
    prune()
    return 0


def _compare_command(args: argparse.Namespace) -> int:
    result = compare(_load_manifest(args.baseline), _load_manifest(args.candidate))
    print(canonical_json(result), end="")
    return 0 if result["equal"] else 1


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)
    capture = commands.add_parser("capture", help="capture a commit or an owned export")
    source = capture.add_mutually_exclusive_group(required=True)
    source.add_argument("--source-sha")
    source.add_argument("--source-dir", type=Path)
    capture.add_argument("--output", required=True, type=Path)
    capture.set_defaults(run=_capture_command)
    export = commands.add_parser("export", help="export a commit for editing")
    export.add_argument("--source-sha", required=True)
    export.set_defaults(run=_export_command)
    discard = commands.add_parser("discard", help="remove an owned export")
    discard.add_argument("--source-dir", required=True, type=Path)
    discard.set_defaults(run=_discard_command)
    prune_parser = commands.add_parser(
        "prune", help="delete the cache and owned exports"
    )
    prune_parser.set_defaults(run=_prune_command)
    compare_parser = commands.add_parser("compare", help="compare two manifests")
    compare_parser.add_argument("--baseline", required=True, type=Path)
    compare_parser.add_argument("--candidate", required=True, type=Path)
    compare_parser.set_defaults(run=_compare_command)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    try:
        args = _parser().parse_args(argv)
    except SystemExit as exc:
        return 0 if exc.code == 0 else 2
    try:
        return args.run(args)
    except HiddenSurfaceError as exc:
        print(f"hidden-surface: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
