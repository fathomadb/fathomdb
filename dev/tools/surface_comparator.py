#!/usr/bin/env python3
"""Capture and compare the 0.8.27 cross-language public surface.

The capture command is intentionally explicit and heavy. It invokes the pinned
Rust API extractor and the production NAPI/TypeScript build, then writes one
canonical JSON manifest to a caller-selected path. Comparison never writes or
replaces either input.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import posixpath
import re
import shutil
import subprocess
import sys
import tempfile
from typing import Any, Iterable, Sequence


SCHEMA = "fathomdb.surface-baseline.v1"
REPO_ROOT = Path(__file__).resolve().parents[2]
OWNED_CACHE = REPO_ROOT / ".cache" / "0.8.27-slice30"
OWNED_SCRATCH = Path("/tmp/fathomdb-0.8.27-slice30")
SCRATCH_MARKER = ".fathomdb-slice30-owned"
MIN_FREE_BYTES = 100_000_000_000
PINNED_CARGO_PUBLIC_API = "0.52.0"
PINNED_NIGHTLY = "nightly-2026-04-24"
PINNED_NODE = "v25.9.0"

RUST_ROWS = [
    {
        "id": "rust-facade-default",
        "kind": "rust",
        "command": "cargo public-api",
        "args": ["-p", "fathomdb", "--no-default-features"],
    },
    {
        "id": "rust-facade-operator",
        "kind": "rust",
        "command": "cargo public-api",
        "args": ["-p", "fathomdb", "--no-default-features", "--features", "operator"],
    },
    {
        "id": "rust-engine-default",
        "kind": "rust",
        "command": "cargo public-api",
        "args": ["-p", "fathomdb-engine", "--no-default-features"],
    },
    {
        "id": "rust-engine-operator",
        "kind": "rust",
        "command": "cargo public-api",
        "args": [
            "-p",
            "fathomdb-engine",
            "--no-default-features",
            "--features",
            "operator",
        ],
    },
    {
        "id": "rust-engine-test-hooks",
        "kind": "rust",
        "command": "cargo public-api",
        "args": [
            "-p",
            "fathomdb-engine",
            "--no-default-features",
            "--features",
            "test-hooks",
        ],
    },
    {
        "id": "rust-engine-operator-test-hooks",
        "kind": "rust",
        "command": "cargo public-api",
        "args": [
            "-p",
            "fathomdb-engine",
            "--no-default-features",
            "--features",
            "operator,test-hooks",
        ],
    },
]

REQUIRED_ROW_IDENTITIES = RUST_ROWS + [
    {
        "id": "python-package-exports",
        "kind": "python",
        "source": "src/python/fathomdb/__init__.py:__all__",
    },
    {
        "id": "python-wrapper-declarations",
        "kind": "python",
        "source": "src/python/fathomdb/**/*.py:resolved public namespace signatures",
    },
    {
        "id": "python-native-registrations",
        "kind": "python",
        "source": "src/rust/crates/fathomdb-py/src/lib.rs:pymodule registrations with cfg",
    },
    {
        "id": "python-native-stub",
        "kind": "python",
        "source": "src/python/fathomdb/_fathomdb.pyi",
    },
    {
        "id": "napi-production",
        "kind": "napi",
        "command": "npm run build:native",
        "features": ["default-embedder"],
        "expanded_command": (
            "napi build --platform --release --cargo-cwd "
            "../rust/crates/fathomdb-napi --features default-embedder --js false"
        ),
    },
    {
        "id": "typescript-declarations",
        "kind": "typescript",
        "command": (
            "npm exec -- tsc -p tsconfig.build.json --declaration --emitDeclarationOnly "
            "--outDir /tmp/fathomdb-0.8.27-slice30/ts-declarations"
        ),
        "source": "emitted index.d.ts with relative and local export lists resolved",
    },
    {
        "id": "package-entrypoints",
        "kind": "package",
        "source": "src/ts/package.json and generated dist/index.js exports",
    },
]


class ComparatorError(RuntimeError):
    """Capture or comparison input is ambiguous or invalid."""


# Backward-compatible name retained for the initial Slice 30 fixture contract.
SurfaceError = ComparatorError


def canonical_json(value: Any) -> str:
    """Return the stable on-disk representation used by the baseline."""

    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def _normalize_space(value: str) -> str:
    return " ".join(value.split())


def _entry(path: str, kind: str, signature: str) -> dict[str, str]:
    return {"path": path, "kind": kind, "signature": _normalize_space(signature)}


def _unique(entries: Iterable[dict[str, str]], row: str) -> list[dict[str, str]]:
    ordered = sorted(entries, key=lambda item: (item["path"], item["kind"], item["signature"]))
    seen: set[tuple[str, str, str]] = set()
    unique: list[dict[str, str]] = []
    for item in ordered:
        key = (item["path"], item["kind"], item["signature"])
        if key in seen:
            continue
        seen.add(key)
        unique.append(item)
    return unique


def parse_rust_public_api(text: str, row: str) -> list[dict[str, str]]:
    """Normalize complete cargo-public-api paths and signatures."""

    entries: list[dict[str, str]] = []
    current_impl: str | None = None
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        if re.match(r"(?:unsafe\s+)?impl\b", line):
            current_impl = _normalize_space(line)
        elif re.match(r"pub\s+(?:struct|enum|trait|union|mod)\s+", line):
            current_impl = None
        match = re.search(
            r"\b(struct|enum|trait|union|type|constant|static|macro|mod|fn)\s+"
            r"([A-Za-z_][A-Za-z0-9_:]*(?:![A-Za-z0-9_]*)?)",
            line,
        )
        if match is None:
            path = f"line:{hashlib.sha256(_normalize_space(line).encode()).hexdigest()[:16]}"
            kind = "rust-item"
        else:
            kind = match.group(1)
            path = match.group(2)
        signature = line
        if current_impl is not None and path.count("::") >= 2 and kind in {"fn", "type", "constant"}:
            signature = f"{current_impl} => {line}"
        entries.append(_entry(path, kind, signature))
    if not entries:
        raise SurfaceError(f"{row} produced no public API entries")
    return _unique(entries, row)


def parse_python_exports(text: str) -> list[dict[str, str]]:
    """Read the literal package ``__all__`` declaration."""

    values = _literal_all(ast.parse(text))
    if values is None:
        raise SurfaceError("python package has no literal __all__ declaration")
    return _unique((_entry(name, "python-export", name) for name in values), "python-package-exports")


def _literal_all(tree: ast.Module) -> list[str] | None:
    values: list[str] | None = None
    for node in tree.body:
        if not isinstance(node, (ast.Assign, ast.AnnAssign)):
            continue
        targets = node.targets if isinstance(node, ast.Assign) else [node.target]
        if not any(isinstance(target, ast.Name) and target.id == "__all__" for target in targets):
            continue
        value_node = node.value
        try:
            literal = ast.literal_eval(value_node)
        except (ValueError, SyntaxError) as exc:
            raise SurfaceError("python __all__ must be a literal sequence") from exc
        if not isinstance(literal, (list, tuple)) or not all(isinstance(item, str) for item in literal):
            raise SurfaceError("python __all__ must contain only strings")
        values = list(literal)
    return values


def _statements(body: list[ast.stmt]) -> Iterable[ast.stmt]:
    # Conditional and guarded definitions (TYPE_CHECKING, try/except import
    # fallbacks) bind module names just like unconditional ones.
    for node in body:
        if isinstance(node, ast.If):
            yield from _statements(node.body)
            yield from _statements(node.orelse)
        elif isinstance(node, ast.Try):
            yield from _statements(node.body)
            for handler in node.handlers:
                yield from _statements(handler.body)
            yield from _statements(node.orelse)
            yield from _statements(node.finalbody)
        else:
            yield node


def _is_public(name: str) -> bool:
    return not name.startswith("_") or (name.startswith("__") and name.endswith("__"))


def _python_callable_signature(node: ast.FunctionDef | ast.AsyncFunctionDef) -> str:
    decorators = "".join(f"@{_python_signature(item)} " for item in node.decorator_list)
    prefix = "async def" if isinstance(node, ast.AsyncFunctionDef) else "def"
    returns = f" -> {_python_signature(node.returns)}" if node.returns is not None else ""
    return f"{decorators}{prefix} {node.name}({_python_signature(node.args)}){returns}"


def _python_class_header(node: ast.ClassDef) -> str:
    decorators = "".join(f"@{_python_signature(item)} " for item in node.decorator_list)
    arguments = [_python_signature(base) for base in node.bases]
    arguments.extend(
        f"{keyword.arg}={_python_signature(keyword.value)}"
        if keyword.arg is not None
        else f"**{_python_signature(keyword.value)}"
        for keyword in node.keywords
    )
    suffix = f"({', '.join(arguments)})" if arguments else ""
    return f"{decorators}class {node.name}{suffix}"


class _PythonPackage:
    """Resolve public names through intra-package imports to their definitions."""

    def __init__(self, sources: dict[str, str]) -> None:
        self.modules: dict[str, ast.Module] = {}
        self.packages: set[str] = set()
        for relative, text in sources.items():
            if not relative.endswith(".py"):
                continue
            parts = relative[: -len(".py")].split("/")
            is_package = parts[-1] == "__init__"
            if is_package:
                parts = parts[:-1]
            module = ".".join(["fathomdb", *parts])
            try:
                self.modules[module] = ast.parse(text, filename=relative)
            except SyntaxError as exc:
                raise ComparatorError(f"cannot parse Python source {relative}: {exc}") from exc
            if is_package:
                self.packages.add(module)
        if "fathomdb" not in self.modules:
            raise ComparatorError("python sources have no fathomdb/__init__.py")
        self.bindings = {module: self._bindings(module) for module in self.modules}

    def _absolute(self, module: str, node: ast.ImportFrom) -> str:
        if node.level == 0:
            return node.module or ""
        base = module.split(".") if module in self.packages else module.split(".")[:-1]
        base = base[: len(base) - (node.level - 1)]
        return ".".join([*base, node.module] if node.module else base)

    def _bindings(self, module: str) -> dict[str, tuple[Any, ...]]:
        bindings: dict[str, tuple[Any, ...]] = {}
        for node in _statements(self.modules[module].body):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                bindings[node.name] = ("def", node)
            elif isinstance(node, ast.ImportFrom):
                source = self._absolute(module, node)
                for alias in node.names:
                    if alias.name != "*":
                        bindings[alias.asname or alias.name] = ("import", source, alias.name)
            elif isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name):
                        bindings[target.id] = ("def", node)
            elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
                bindings[node.target.id] = ("def", node)
        return bindings

    def resolve(self, module: str, name: str, seen: frozenset[tuple[str, str]] = frozenset()) -> tuple[Any, ...]:
        if (module, name) in seen:
            raise ComparatorError(f"circular Python import for {module}.{name}")
        binding = self.bindings.get(module, {}).get(name)
        if binding is None:
            if f"{module}.{name}" in self.modules:
                return ("module", f"{module}.{name}")
            if module not in self.modules:
                return ("external", module, name)
            raise ComparatorError(f"python name {module}.{name} is exported but never bound")
        if binding[0] == "def":
            return binding
        _, source, original = binding
        submodule = f"{source}.{original}"
        if submodule in self.modules and (
            source == module or self.bindings[source].get(original, ("",))[0] != "def"
        ):
            return ("module", submodule)
        if source not in self.modules:
            return ("external", source, original)
        return self.resolve(source, original, seen | {(module, name)})

    def namespaces(self) -> list[str]:
        return sorted(
            module
            for module in self.modules
            if all(_is_public(part) and not part.startswith("__") for part in module.split("."))
        )

    def exported(self, module: str) -> list[str]:
        declared = _literal_all(self.modules[module])
        if declared is not None:
            return declared
        # Without __all__, a public module exposes its own public definitions
        # and package definitions it re-imports, not third-party imports.
        names = []
        for name in self.bindings[module]:
            if _is_public(name) and self.resolve(module, name)[0] == "def":
                names.append(name)
        return names


def parse_python_wrappers(sources: dict[str, str]) -> list[dict[str, str]]:
    """Capture public Python namespaces by resolved declaration signature.

    Entries are keyed by public dotted path, never by defining file, and carry
    signatures without bodies, so moving an implementation behind a re-export
    or editing a body compares equal while a public name or signature change
    does not.
    """

    package = _PythonPackage(sources)
    entries: list[dict[str, str]] = []
    for namespace in package.namespaces():
        for name in package.exported(namespace):
            path = f"{namespace}.{name}"
            resolved = package.resolve(namespace, name)
            if resolved[0] == "module":
                entries.append(_entry(path, "python-module", f"module {resolved[1]}"))
                continue
            if resolved[0] == "external":
                entries.append(
                    _entry(path, "python-reexport", f"from {resolved[1]} import {resolved[2]}")
                )
                continue
            node = resolved[1]
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                entries.append(_entry(path, "python-function", _python_callable_signature(node)))
            elif isinstance(node, ast.ClassDef):
                entries.append(_entry(path, "python-class", _python_class_header(node)))
                for member in node.body:
                    if isinstance(member, (ast.FunctionDef, ast.AsyncFunctionDef)):
                        if _is_public(member.name):
                            entries.append(
                                _entry(
                                    f"{path}.{member.name}",
                                    "python-method",
                                    _python_callable_signature(member),
                                )
                            )
                    elif isinstance(member, ast.AnnAssign) and isinstance(member.target, ast.Name):
                        if _is_public(member.target.id):
                            entries.append(
                                _entry(
                                    f"{path}.{member.target.id}",
                                    "python-attribute",
                                    _python_signature(member),
                                )
                            )
                    elif isinstance(member, ast.Assign):
                        for target in member.targets:
                            if isinstance(target, ast.Name) and _is_public(target.id):
                                entries.append(
                                    _entry(
                                        f"{path}.{target.id}",
                                        "python-attribute",
                                        _python_signature(member),
                                    )
                                )
            else:
                entries.append(_entry(path, "python-value", _python_signature(node)))
    if not entries:
        raise SurfaceError("python wrapper sources expose no public declarations")
    return _unique(entries, "python-wrapper-declarations")


def parse_python_registrations(text: str) -> list[dict[str, str]]:
    """Read PyO3 registrations independently of package exports and stubs."""

    line_cfgs = _cfg_by_line(text)

    def gated(match: re.Match[str], signature: str) -> str:
        cfgs = line_cfgs[text.count("\n", 0, match.start())]
        return "".join(f"#[{cfg}] " for cfg in cfgs) + signature

    entries: list[dict[str, str]] = []
    for match in re.finditer(r"\.add_class::<([A-Za-z_][A-Za-z0-9_:]*)>\(\)\?", text):
        name = match.group(1)
        entries.append(_entry(name, "pyclass-registration", gated(match, f"add_class::<{name}>")))
    for match in re.finditer(r"wrap_pyfunction!\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*,", text):
        name = match.group(1)
        entries.append(
            _entry(name, "pyfunction-registration", gated(match, f"wrap_pyfunction!({name})"))
        )
    for match in re.finditer(
        r'\bm\.add\(\s*"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"\s*,\s*'
        r'(?P<value>.*?)\)\?;',
        text,
        re.DOTALL,
    ):
        name = match.group("name")
        value = _normalize_space(match.group("value"))
        entries.append(
            _entry(name, "py-alias-registration", gated(match, f'm.add("{name}", {value})'))
        )
    if not entries:
        raise SurfaceError("PyO3 source contains no class or function registrations")
    return _unique(entries, "python-native-registrations")


def _cfg_by_line(text: str) -> list[tuple[str, ...]]:
    """Return the ``#[cfg(...)]`` gates active for each source line.

    Covers attribute-on-statement and attribute-on-block forms, which are the
    two ways a PyO3 registration can be compiled out of a shipped wheel.
    """

    result: list[tuple[str, ...]] = []
    pending: list[str] = []
    blocks: list[tuple[int, tuple[str, ...]]] = []
    depth = 0
    # Literals and comments are blanked to same-length spaces so their braces
    # never count while line indexes and attribute offsets still match `text`.
    blanked = _RUST_LITERAL.sub(lambda m: re.sub(r"[^\n]", " ", m.group(0)), text)
    carried = ("", "")
    for original, line in zip(text.split("\n"), blanked.split("\n")):
        # Both views are sliced identically (never rstrip the blanked view:
        # trailing blanks may stand for a literal still present in `source`).
        offset = len(line) - len(line.lstrip())
        stripped = carried[0] + line[offset:]
        source = carried[1] + original[offset:]
        carried = ("", "")
        while stripped.startswith("#["):
            end = _attribute_end(stripped)
            if end < 0:
                # A multi-line attribute continues on the next line.
                carried = (stripped + " ", source + " ")
                stripped = ""
                break
            content = source[2:end]
            if content.startswith("cfg("):
                pending.append(content)
            rest = len(stripped) - len(stripped[end + 1 :].lstrip())
            stripped, source = stripped[rest:], source[rest:]
        if not stripped.strip():
            result.append(())
            continue
        result.append(tuple(cfg for _, cfgs in blocks for cfg in cfgs) + tuple(pending))
        opened = stripped.count("{") - stripped.count("}")
        # A gated item (`fn`, `mod`, `impl`, bare block) whose body opens on
        # this line gates everything until that body closes.
        if pending and opened > 0:
            blocks.append((depth, tuple(pending)))
        pending = []
        depth += opened
        while blocks and depth <= blocks[-1][0]:
            blocks.pop()
    if depth != 0 or carried != ("", ""):
        raise ComparatorError("unbalanced braces in PyO3 registration source")
    return result


_RUST_LITERAL = re.compile(
    r"//[^\n]*|/\*.*?\*/|\br(#*)\".*?\"\1|\bb?\"(?:\\.|[^\"\\])*\"|\bb?'(?:\\.|[^\\'\n])'"
    r"|(?<![A-Za-z0-9_])\"(?:\\.|[^\"\\])*\"|(?<![A-Za-z0-9_])'(?:\\.|[^\\'\n])'",
    re.DOTALL,
)


def _attribute_end(line: str) -> int:
    depth = 0
    for index, char in enumerate(line):
        if char == "[":
            depth += 1
        elif char == "]":
            depth -= 1
            if depth == 0:
                return index
    return -1


def _python_signature(node: ast.AST) -> str:
    try:
        return ast.unparse(node)
    except AttributeError as exc:  # pragma: no cover - Python 3.9 and older are unsupported
        raise SurfaceError("Python ast.unparse is required") from exc


def parse_python_stub(text: str) -> list[dict[str, str]]:
    """Normalize top-level native stub declarations and class members."""

    tree = ast.parse(text)
    entries: list[dict[str, str]] = []
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            entries.append(_entry(node.name, "stub-function", _python_signature(node)))
        elif isinstance(node, ast.ClassDef):
            class_arguments = [_python_signature(base) for base in node.bases]
            class_arguments.extend(
                f"{keyword.arg}={_python_signature(keyword.value)}"
                if keyword.arg is not None
                else f"**{_python_signature(keyword.value)}"
                for keyword in node.keywords
            )
            suffix = f"({', '.join(class_arguments)})" if class_arguments else ""
            entries.append(_entry(node.name, "stub-class", f"class {node.name}{suffix}"))
            for member in node.body:
                if isinstance(member, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    entries.append(
                        _entry(
                            f"{node.name}.{member.name}",
                            "stub-method",
                            _python_signature(member),
                        )
                    )
                elif isinstance(member, ast.AnnAssign) and isinstance(member.target, ast.Name):
                    entries.append(
                        _entry(
                            f"{node.name}.{member.target.id}",
                            "stub-attribute",
                            _python_signature(member),
                        )
                    )
    if not entries:
        raise SurfaceError("Python native stub contains no declarations")
    return _unique(entries, "python-native-stub")


def _declaration_name(statement: str) -> tuple[str, str]:
    const_enum = re.search(r"\bconst\s+enum\s+([A-Za-z_$][A-Za-z0-9_$]*)", statement)
    if const_enum is not None:
        return const_enum.group(1), "enum"
    match = re.search(
        r"\b(class|function|interface|type|const|let|var|enum|namespace)\s+"
        r"([A-Za-z_$][A-Za-z0-9_$]*)",
        statement,
    )
    if match is not None:
        return match.group(2), match.group(1)
    exported = re.search(r"\bexport\s+(?:type\s+)?\{([^}]+)\}", statement)
    if exported is not None:
        names = sorted(
            part.strip().split(" as ")[-1].strip()
            for part in exported.group(1).split(",")
            if part.strip()
        )
        return "export-list:" + ",".join(names), "export-list"
    digest = hashlib.sha256(_normalize_space(statement).encode()).hexdigest()[:16]
    return f"export:{digest}", "export"


def parse_typescript_declarations(text: str, row: str) -> list[dict[str, str]]:
    """Split an emitted declaration file into complete exported statements."""

    entries = []
    for exported, statement in _typescript_statements(text, row):
        if not exported:
            continue
        path, kind = _declaration_name(statement)
        entries.append(_entry(path, kind, statement))
    if not entries:
        raise SurfaceError(f"{row} contains no exported declarations")
    return _unique(entries, row)


_TS_STAR_REEXPORT = re.compile(r"""export\s+(?:type\s+)?\*\s+from\s+["']([^"']+)["']\s*;?""")
_TS_NAMED_REEXPORT = re.compile(
    r"""export\s+(?:type\s+)?\{([^}]*)\}\s+from\s+["']([^"']+)["']\s*;?"""
)


def _resolve_typescript_module(
    importer: str, specifier: str, modules: dict[str, str], row: str
) -> str:
    base = posixpath.normpath(posixpath.join(posixpath.dirname(importer), specifier))
    stem = re.sub(r"\.(?:js|mjs|cjs)$", "", base)
    for candidate in (f"{stem}.d.ts", f"{stem}.d.mts", f"{stem}/index.d.ts"):
        if candidate in modules:
            return candidate
    raise ComparatorError(f"{row} cannot resolve re-export {specifier!r} from {importer}")


def _flatten_typescript(
    modules: dict[str, str], module: str, row: str, stack: tuple[str, ...] = ()
) -> list[tuple[str, str, str]]:
    if module in stack:
        return []
    stack = (*stack, module)
    flattened: list[tuple[str, str, str]] = []
    statements = _typescript_statements(modules[module], row)
    local = [
        (*_declaration_name(statement), statement)
        for exported, statement in statements
        if not _TS_LOCAL_EXPORT_LIST.fullmatch(statement)
    ]
    for exported, statement in statements:
        if not exported:
            continue
        local_list = _TS_LOCAL_EXPORT_LIST.fullmatch(statement)
        if local_list is not None:
            flattened.extend(_resolve_export_list(local_list.group(1), local, module, row))
            continue
        star = _TS_STAR_REEXPORT.fullmatch(statement)
        named = _TS_NAMED_REEXPORT.fullmatch(statement)
        target_specifier = (star or named).group(1 if star else 2) if (star or named) else None
        if target_specifier is None or not target_specifier.startswith("."):
            path, kind = _declaration_name(statement)
            flattened.append((path, kind, statement))
            continue
        target = _resolve_typescript_module(module, target_specifier, modules, row)
        exported = _flatten_typescript(modules, target, row, stack)
        if star is not None:
            flattened.extend(item for item in exported if not item[2].startswith("export default"))
            continue
        flattened.extend(_resolve_export_list(named.group(1), exported, target, row))
    return flattened


_TS_LOCAL_EXPORT_LIST = re.compile(r"export\s+(?:type\s+)?\{([^}]*)\}\s*;?")


def _resolve_export_list(
    names: str, candidates: list[tuple[str, str, str]], module: str, row: str
) -> list[tuple[str, str, str]]:
    resolved: list[tuple[str, str, str]] = []
    for part in names.split(","):
        part = re.sub(r"^type\s+", "", part.strip())
        if not part:
            continue
        original, _, alias = (piece.strip() for piece in part.partition(" as "))
        alias = alias or original
        matches = [item for item in candidates if item[0] == original]
        if not matches:
            raise ComparatorError(f"{row} export {original!r} from {module} has no declaration")
        for _, kind, target_statement in matches:
            signature = (
                target_statement
                if alias == original
                else f"export {{ {original} as {alias} }} => {target_statement}"
            )
            resolved.append((alias, kind, signature))
    return resolved


def parse_typescript_surface(modules: dict[str, str], row: str) -> list[dict[str, str]]:
    """Capture ``index.d.ts`` with relative re-exports resolved to declarations.

    Entries are keyed by exported name, so moving a declaration into a module
    that the root re-exports compares equal.
    """

    if "index.d.ts" not in modules:
        raise ComparatorError(f"{row} requires index.d.ts")
    entries = [
        _entry(path, kind, statement)
        for path, kind, statement in _flatten_typescript(modules, "index.d.ts", row)
    ]
    if not entries:
        raise SurfaceError(f"{row} contains no exported declarations")
    return _unique(entries, row)


def _strip_comments(text: str) -> str:
    """Remove ``//`` and ``/* */`` comments outside string literals.

    Documentation is not surface, and braces inside comments must not affect
    statement splitting. Newlines inside block comments are kept so line
    structure is preserved.
    """

    out: list[str] = []
    index = 0
    quote: str | None = None
    while index < len(text):
        char = text[index]
        if quote is not None:
            out.append(char)
            if char == "\\" and index + 1 < len(text):
                out.append(text[index + 1])
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
        elif char in "\"'`":
            quote = char
            out.append(char)
            index += 1
        elif text.startswith("//", index):
            end = text.find("\n", index)
            index = len(text) if end == -1 else end
        elif text.startswith("/*", index):
            end = text.find("*/", index + 2)
            if end == -1:
                raise ComparatorError("unterminated block comment in declaration input")
            out.append("\n" * text.count("\n", index, end))
            index = end + 2
        else:
            out.append(char)
            index += 1
    return "".join(out)


_TS_DECLARATION_START = re.compile(
    r"(?:export\s|declare\s|interface\s|type\s|class\s|abstract\s|function\s|const\s|enum\s|namespace\s)"
)


def _brace_delta(line: str) -> int:
    unquoted = re.sub(r"\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`", "", line)
    return unquoted.count("{") - unquoted.count("}")


def _typescript_statements(text: str, row: str) -> list[tuple[bool, str]]:
    """Return top-level ``(exported, statement)`` pairs, comments removed."""

    text = _strip_comments(text)
    statements: list[tuple[bool, str]] = []
    current: list[str] = []
    exported = False
    depth = 0
    for raw in text.splitlines():
        stripped = raw.strip()
        if not current:
            if not _TS_DECLARATION_START.match(stripped):
                continue
            current = [stripped]
            exported = stripped.startswith("export ")
            depth = _brace_delta(stripped)
        else:
            current.append(stripped)
            depth += _brace_delta(stripped)
        single_line_function = (
            len(current) == 1
            and re.match(r"(?:export\s+)?declare\s+function\s", stripped) is not None
            and ")" in stripped
        )
        if depth == 0 and (
            stripped.endswith(";")
            or stripped.endswith("}")
            or " from " in stripped
            or single_line_function
        ):
            statements.append((exported, " ".join(current)))
            current = []
    if current:
        raise SurfaceError(f"unterminated exported declaration in {row}")
    return statements


def parse_package_entrypoints(package_text: str, runtime_exports: Sequence[str]) -> list[dict[str, str]]:
    """Capture supported package roots/subpaths and generated runtime keys."""

    try:
        package = json.loads(package_text)
    except json.JSONDecodeError as exc:
        raise SurfaceError("package.json is not valid JSON") from exc
    if not isinstance(package, dict):
        raise SurfaceError("package.json root must be an object")
    entries: list[dict[str, str]] = []
    for field in ("name", "type", "main", "module", "types", "exports", "files"):
        if field in package:
            entries.append(
                _entry(
                    f"package.{field}",
                    "package-metadata",
                    json.dumps(package[field], sort_keys=True, separators=(",", ":")),
                )
            )
    if "main" not in package or "types" not in package:
        raise SurfaceError("package root must declare main and types entrypoints")
    for name in runtime_exports:
        if not isinstance(name, str):
            raise SurfaceError("runtime export keys must be strings")
        entries.append(_entry(f"runtime.{name}", "runtime-export", name))
    if not runtime_exports:
        raise SurfaceError("generated runtime has no export keys")
    return _unique(entries, "package-entrypoints")


def capture_from_fixture(inputs: dict[str, Any], metadata: dict[str, Any]) -> dict[str, Any]:
    """Normalize already-produced, real-shaped adapter inputs.

    Tests use this seam to mutate one compiler/registration/declaration input at
    a time. Production capture supplies outputs generated by the pinned tools.
    """

    if metadata.get("schema") != SCHEMA:
        raise SurfaceError(f"metadata schema must be {SCHEMA}")
    if metadata.get("row_identities") != REQUIRED_ROW_IDENTITIES:
        # A mismatched identity is retained in a candidate manifest so compare
        # can diagnose it, but it cannot change which adapters execute.
        identities = metadata.get("row_identities")
        if not isinstance(identities, list):
            raise SurfaceError("row_identities must be a list")
    for key in (
        "rust",
        "python_exports",
        "python_sources",
        "python_registrations",
        "python_stub",
        "napi_declaration",
        "typescript_declarations",
        "package_json",
        "runtime_exports",
    ):
        if key not in inputs:
            raise ComparatorError(f"missing adapter input: {key}")
    rust = inputs.get("rust")
    if not isinstance(rust, dict):
        raise SurfaceError("rust adapter inputs must be a row map")
    rows: list[dict[str, Any]] = []
    for identity in RUST_ROWS:
        row_id = identity["id"]
        text = rust.get(row_id)
        if not isinstance(text, str):
            raise SurfaceError(f"missing Rust adapter input: {row_id}")
        rows.append({"id": row_id, "entries": parse_rust_public_api(text, row_id)})
    rows.extend(
        [
            {
                "id": "python-package-exports",
                "entries": parse_python_exports(inputs["python_exports"]),
            },
            {
                "id": "python-wrapper-declarations",
                "entries": parse_python_wrappers(inputs["python_sources"]),
            },
            {
                "id": "python-native-registrations",
                "entries": parse_python_registrations(inputs["python_registrations"]),
            },
            {
                "id": "python-native-stub",
                "entries": parse_python_stub(inputs["python_stub"]),
            },
            {
                "id": "napi-production",
                "entries": parse_typescript_declarations(inputs["napi_declaration"], "napi-production"),
            },
            {
                "id": "typescript-declarations",
                "entries": parse_typescript_surface(
                    inputs["typescript_declarations"], "typescript-declarations"
                ),
            },
            {
                "id": "package-entrypoints",
                "entries": parse_package_entrypoints(
                    inputs["package_json"], inputs["runtime_exports"]
                ),
            },
        ]
    )
    return {"metadata": json.loads(json.dumps(metadata)), "rows": rows}


def _row_map(manifest: dict[str, Any]) -> dict[str, dict[str, dict[str, str]]]:
    rows = manifest.get("rows")
    if not isinstance(rows, list):
        raise SurfaceError("manifest rows must be a list")
    result: dict[str, dict[str, dict[str, str]]] = {}
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("id"), str):
            raise SurfaceError("manifest row has no string id")
        row_id = row["id"]
        if row_id in result:
            raise SurfaceError(f"duplicate manifest row: {row_id}")
        entries = row.get("entries")
        if not isinstance(entries, list):
            raise SurfaceError(f"manifest row {row_id} entries must be a list")
        mapped: dict[str, dict[str, str]] = {}
        for item in entries:
            if not isinstance(item, dict) or not all(
                isinstance(item.get(field), str) for field in ("path", "kind", "signature")
            ):
                raise SurfaceError(f"malformed entry in row {row_id}")
            signature_digest = hashlib.sha256(item["signature"].encode()).hexdigest()
            key = f"{item['kind']}:{item['path']}:{signature_digest}"
            if key in mapped:
                raise SurfaceError(f"duplicate manifest key in {row_id}: {key}")
            mapped[key] = item
        result[row_id] = mapped
    return result


def compare_manifests(baseline: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any]:
    """Return deterministic metadata and row-level added/removed/changed diffs."""

    baseline_metadata = baseline.get("metadata")
    candidate_metadata = candidate.get("metadata")
    if not isinstance(baseline_metadata, dict) or not isinstance(candidate_metadata, dict):
        raise SurfaceError("both manifests require metadata objects")
    baseline_source_sha = _validated_source_sha(baseline_metadata, "baseline")
    candidate_source_sha = _validated_source_sha(candidate_metadata, "candidate")
    comparable_metadata_keys = (
        set(baseline_metadata) | set(candidate_metadata)
    ) - {"capture_source_sha"}
    metadata_diffs = {
        key: {"baseline": baseline_metadata.get(key), "candidate": candidate_metadata.get(key)}
        for key in sorted(comparable_metadata_keys)
        if baseline_metadata.get(key) != candidate_metadata.get(key)
    }
    baseline_rows = _row_map(baseline)
    candidate_rows = _row_map(candidate)
    row_diffs: list[dict[str, Any]] = []
    for row_id in sorted(set(baseline_rows) | set(candidate_rows)):
        before = baseline_rows.get(row_id, {})
        after = candidate_rows.get(row_id, {})
        removed_keys = sorted(set(before) - set(after))
        added_keys = sorted(set(after) - set(before))
        changed = [
            {"path": before[key]["path"], "before": before[key], "after": after[key]}
            for key in sorted(set(before) & set(after))
            if before[key] != after[key]
        ]
        # A declaration rename or re-export path change naturally appears as
        # one removal plus one addition. Pair compatible kinds as a changed
        # declaration as well, while retaining the exact added/removed facts.
        # Each added entry pairs at most once: same-path pairs first, then
        # remaining same-kind entries in key order.
        unpaired_added = list(added_keys)
        unpaired_removed = list(removed_keys)
        for same_path in (True, False):
            for removed_key in list(unpaired_removed):
                old = before[removed_key]
                chosen = next(
                    (
                        key
                        for key in unpaired_added
                        if after[key]["kind"] == old["kind"]
                        and (not same_path or after[key]["path"] == old["path"])
                    ),
                    None,
                )
                if chosen is None:
                    continue
                unpaired_added.remove(chosen)
                unpaired_removed.remove(removed_key)
                changed.append({"path": old["path"], "before": old, "after": after[chosen]})
        if removed_keys or added_keys or changed:
            row_diffs.append(
                {
                    "row": row_id,
                    "added": [after[key] for key in added_keys],
                    "removed": [before[key] for key in removed_keys],
                    "changed": changed,
                }
            )
    return {
        "equal": not metadata_diffs and not row_diffs,
        "metadata_diffs": metadata_diffs,
        "provenance": {
            "baseline_capture_source_sha": baseline_source_sha,
            "candidate_capture_source_sha": candidate_source_sha,
            "same_source": baseline_source_sha == candidate_source_sha,
        },
        "row_diffs": row_diffs,
    }


def _validated_source_sha(metadata: dict[str, Any], label: str) -> str:
    value = metadata.get("capture_source_sha")
    if not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{40}", value) is None:
        raise ComparatorError(f"{label} capture_source_sha must be a full lowercase Git SHA")
    return value


def _run(command: Sequence[str], cwd: Path, env: dict[str, str] | None = None) -> str:
    try:
        completed = subprocess.run(
            list(command),
            cwd=cwd,
            env=env,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as exc:
        raise ComparatorError(f"cannot execute {' '.join(command)}: {exc}") from exc
    if completed.returncode != 0:
        diagnostic = "\n".join(
            part.rstrip() for part in (completed.stdout, completed.stderr) if part.strip()
        )
        raise SurfaceError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n{diagnostic}"
        )
    return completed.stdout


def _exact_version(command: Sequence[str], expected_output: str) -> str:
    output = _run(command, REPO_ROOT).strip()
    if output != expected_output:
        raise ComparatorError(
            f"expected exact output {expected_output!r} from {' '.join(command)}, got {output!r}"
        )
    return output


def _clean_source(source_sha: str) -> None:
    head = _run(["git", "rev-parse", "HEAD"], REPO_ROOT).strip()
    if not re.fullmatch(r"[0-9a-f]{40}", source_sha):
        raise SurfaceError("capture source SHA must be a full 40-character Git SHA")
    if source_sha != head:
        raise SurfaceError(f"capture source SHA {source_sha} does not match HEAD {head}")
    status = _run(["git", "status", "--porcelain", "--untracked-files=all"], REPO_ROOT)
    if status:
        raise SurfaceError("capture requires a clean worktree:\n" + status)


def _prepare_scratch() -> None:
    if shutil.disk_usage(REPO_ROOT).free < MIN_FREE_BYTES:
        raise SurfaceError(f"heavy capture requires at least {MIN_FREE_BYTES} free bytes")
    if OWNED_SCRATCH.exists():
        marker = OWNED_SCRATCH / SCRATCH_MARKER
        if not marker.is_file():
            raise SurfaceError(f"refusing to reuse unowned scratch root: {OWNED_SCRATCH}")
        shutil.rmtree(OWNED_SCRATCH)
    OWNED_SCRATCH.mkdir(parents=True)
    (OWNED_SCRATCH / SCRATCH_MARKER).write_text("owned by 0.8.27 Slice 30\n")
    OWNED_CACHE.mkdir(parents=True, exist_ok=True)


def _tool_metadata(source_sha: str) -> dict[str, Any]:
    _exact_version(
        ["cargo", "public-api", "--version"],
        f"cargo-public-api {PINNED_CARGO_PUBLIC_API}",
    )
    rustc = _run(["rustc", f"+{PINNED_NIGHTLY}", "--version"], REPO_ROOT).strip()
    target_report = _run(["rustc", f"+{PINNED_NIGHTLY}", "-vV"], REPO_ROOT)
    target_match = re.search(r"^host:\s*(\S+)$", target_report, re.MULTILINE)
    if target_match is None:
        raise SurfaceError("nightly rustc did not report a host target")
    node = _exact_version(["node", "--version"], PINNED_NODE)
    typescript = _run(["npm", "exec", "--", "tsc", "--version"], REPO_ROOT / "src/ts").strip()
    match = re.fullmatch(r"Version\s+(\S+)", typescript)
    if match is None:
        raise SurfaceError(f"unexpected TypeScript version output: {typescript!r}")
    return {
        "schema": SCHEMA,
        "capture_source_sha": source_sha,
        "tools": {
            "cargo-public-api": PINNED_CARGO_PUBLIC_API,
            "node": node,
            "rust-toolchain": PINNED_NIGHTLY,
            "rustc": rustc,
            "typescript": match.group(1),
        },
        "target": target_match.group(1),
        "row_identities": REQUIRED_ROW_IDENTITIES,
    }


def _validate_napi_build_script(package_text: str) -> None:
    identity = next(row for row in REQUIRED_ROW_IDENTITIES if row["id"] == "napi-production")
    try:
        script = json.loads(package_text).get("scripts", {}).get("build:native")
    except (json.JSONDecodeError, AttributeError) as exc:
        raise ComparatorError(f"cannot read build:native from package.json: {exc}") from exc
    if script != identity["expanded_command"]:
        raise ComparatorError(
            f"package.json build:native {script!r} does not match recorded napi-production "
            f"identity {identity['expanded_command']!r}"
        )


def capture_repository(source_sha: str) -> dict[str, Any]:
    """Generate every required surface row from a clean repository checkout."""

    _clean_source(source_sha)
    _prepare_scratch()
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(OWNED_CACHE / "cargo-target")
    rust_outputs: dict[str, str] = {}
    try:
        metadata = _tool_metadata(source_sha)
        for identity in RUST_ROWS:
            rust_outputs[identity["id"]] = _run(
                [
                    "cargo",
                    f"+{PINNED_NIGHTLY}",
                    "public-api",
                    "--manifest-path",
                    str(REPO_ROOT / "Cargo.toml"),
                    *identity["args"],
                ],
                REPO_ROOT,
                env,
            )

        ts_root = REPO_ROOT / "src/ts"
        _validate_napi_build_script((ts_root / "package.json").read_text())
        # napi-rs writes `index.d.ts` from a type-definition file under TMPDIR
        # that its proc macros only rewrite when the crate recompiles. A cached
        # build would otherwise reuse whatever the last build for this checkout
        # left there (for example a test-hooks debug build). Force the NAPI
        # crate to recompile and give it a private TMPDIR.
        _run(
            [
                "cargo",
                "clean",
                "--manifest-path",
                str(REPO_ROOT / "Cargo.toml"),
                "-p",
                "fathomdb-napi",
                "--release",
            ],
            REPO_ROOT,
            env,
        )
        napi_tmp = OWNED_SCRATCH / "napi-tmp"
        napi_tmp.mkdir(parents=True, exist_ok=True)
        _run(["npm", "run", "build:native"], ts_root, {**env, "TMPDIR": str(napi_tmp)})
        _run(["npm", "exec", "--", "tsc", "-p", "tsconfig.build.json"], ts_root, env)
        # Declarations are emitted into the owned scratch root so only files
        # produced by this capture are read, never stale `dist/` output.
        declaration_root = OWNED_SCRATCH / "ts-declarations"
        _run(
            [
                "npm",
                "exec",
                "--",
                "tsc",
                "-p",
                "tsconfig.build.json",
                "--declaration",
                "--emitDeclarationOnly",
                "--outDir",
                str(declaration_root),
            ],
            ts_root,
            env,
        )
        runtime_json = _run(
            [
                "node",
                "--input-type=module",
                "--eval",
                "import('./dist/index.js').then(m => console.log(JSON.stringify(Object.keys(m).sort())))",
            ],
            ts_root,
            env,
        ).strip()
        runtime_exports = json.loads(runtime_json)
        if not isinstance(runtime_exports, list):
            raise SurfaceError("generated TypeScript runtime export probe returned no list")
        tracked_status = _run(["git", "status", "--porcelain", "--untracked-files=all"], REPO_ROOT)
        if tracked_status:
            raise SurfaceError("generation changed tracked or visible files:\n" + tracked_status)
        python_root = REPO_ROOT / "src/python/fathomdb"
        inputs = {
            "rust": rust_outputs,
            "python_exports": (REPO_ROOT / "src/python/fathomdb/__init__.py").read_text(),
            "python_sources": {
                path.relative_to(python_root).as_posix(): path.read_text()
                for path in sorted(python_root.rglob("*.py"))
                if "__pycache__" not in path.parts
            },
            "python_registrations": (
                REPO_ROOT / "src/rust/crates/fathomdb-py/src/lib.rs"
            ).read_text(),
            "python_stub": (REPO_ROOT / "src/python/fathomdb/_fathomdb.pyi").read_text(),
            "napi_declaration": (ts_root / "index.d.ts").read_text(),
            "typescript_declarations": {
                path.relative_to(declaration_root).as_posix(): path.read_text()
                for path in sorted(declaration_root.rglob("*.d.ts"))
            },
            "package_json": (ts_root / "package.json").read_text(),
            "runtime_exports": runtime_exports,
        }
        return capture_from_fixture(inputs, metadata)
    finally:
        marker = OWNED_SCRATCH / SCRATCH_MARKER
        if marker.is_file():
            shutil.rmtree(OWNED_SCRATCH)


def _load_manifest(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise SurfaceError(f"cannot read manifest {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise SurfaceError(f"manifest {path} root must be an object")
    return value


def _guard_capture_output(output: Path) -> None:
    baseline = REPO_ROOT / "dev/plans/0.8.27/features/slice-30/baseline.json"
    if output.resolve() == baseline.resolve():
        raise ComparatorError("capture cannot directly rewrite the tracked reviewed baseline")
    if output.exists() and baseline.exists():
        try:
            aliases_baseline = output.samefile(baseline)
        except OSError as exc:
            raise ComparatorError(f"cannot validate capture output identity: {exc}") from exc
        if aliases_baseline:
            raise ComparatorError("capture cannot rewrite a hardlink alias of the reviewed baseline")


def _atomic_write(output: Path, contents: str) -> None:
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=output.parent,
            prefix=f".{output.name}.",
            suffix=".tmp",
            delete=False,
        ) as handle:
            temporary_path = Path(handle.name)
            handle.write(contents)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_path, output)
        temporary_path = None
    except OSError as exc:
        raise ComparatorError(f"atomic manifest write failed for {output}: {exc}") from exc
    finally:
        if temporary_path is not None:
            try:
                temporary_path.unlink(missing_ok=True)
            except OSError:
                pass


def _capture_command(args: argparse.Namespace) -> int:
    output = args.output.resolve()
    _guard_capture_output(output)
    manifest = capture_repository(args.source_sha)
    output.parent.mkdir(parents=True, exist_ok=True)
    _atomic_write(output, canonical_json(manifest))
    print(f"captured {len(manifest['rows'])} rows to {output}")
    return 0


def _compare_command(args: argparse.Namespace) -> int:
    result = compare_manifests(_load_manifest(args.baseline), _load_manifest(args.candidate))
    print(canonical_json(result), end="")
    return 0 if result["equal"] else 1


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    capture = subparsers.add_parser("capture", help="capture from a clean exact-source checkout")
    capture.add_argument("--source-sha", required=True)
    capture.add_argument("--output", required=True, type=Path)
    capture.set_defaults(run=_capture_command)
    compare = subparsers.add_parser("compare", help="compare two immutable manifests")
    compare.add_argument("--baseline", required=True, type=Path)
    compare.add_argument("--candidate", required=True, type=Path)
    compare.set_defaults(run=_compare_command)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    try:
        args = _parser().parse_args(argv)
        return args.run(args)
    except ComparatorError as exc:
        print(f"surface-comparator: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
