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
import re
import shutil
import subprocess
import sys
from typing import Any, Iterable, Sequence


SCHEMA = "fathomdb.surface-baseline.v1"
REPO_ROOT = Path(__file__).resolve().parents[2]
OWNED_CACHE = REPO_ROOT / ".cache" / "0.8.27-slice30"
OWNED_SCRATCH = Path("/tmp/fathomdb-0.8.27-slice30")
SCRATCH_MARKER = ".fathomdb-slice30-owned"
MIN_FREE_BYTES = 100_000_000_000
PINNED_CARGO_PUBLIC_API = "0.52.0"
PINNED_NIGHTLY = "nightly-2026-04-24"

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
        "id": "python-native-registrations",
        "kind": "python",
        "source": "src/rust/crates/fathomdb-py/src/lib.rs:pymodule registrations",
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
        "command": "npm exec -- tsc -p tsconfig.build.json",
        "source": "src/ts/dist/index.d.ts",
    },
    {
        "id": "package-entrypoints",
        "kind": "package",
        "source": "src/ts/package.json and generated dist/index.js exports",
    },
]


class SurfaceError(RuntimeError):
    """Capture or comparison input is ambiguous or invalid."""


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
    for item in ordered:
        key = (item["path"], item["kind"], item["signature"])
        if key in seen:
            raise SurfaceError(
                f"duplicate normalized entry in {row}: {item['kind']} "
                f"{item['path']} {item['signature']}"
            )
        seen.add(key)
    return ordered


def parse_rust_public_api(text: str, row: str) -> list[dict[str, str]]:
    """Normalize complete cargo-public-api paths and signatures."""

    entries: list[dict[str, str]] = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
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
        entries.append(_entry(path, kind, line))
    if not entries:
        raise SurfaceError(f"{row} produced no public API entries")
    return _unique(entries, row)


def parse_python_exports(text: str) -> list[dict[str, str]]:
    """Read the literal package ``__all__`` declaration."""

    tree = ast.parse(text)
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
    if values is None:
        raise SurfaceError("python package has no literal __all__ declaration")
    return _unique((_entry(name, "python-export", name) for name in values), "python-package-exports")


def parse_python_registrations(text: str) -> list[dict[str, str]]:
    """Read PyO3 registrations independently of package exports and stubs."""

    entries: list[dict[str, str]] = []
    for name in re.findall(r"\.add_class::<([A-Za-z_][A-Za-z0-9_:]*)>\(\)\?", text):
        entries.append(_entry(name, "pyclass-registration", f"add_class::<{name}>"))
    for name in re.findall(r"wrap_pyfunction!\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*,", text):
        entries.append(_entry(name, "pyfunction-registration", f"wrap_pyfunction!({name})"))
    if not entries:
        raise SurfaceError("PyO3 source contains no class or function registrations")
    return _unique(entries, "python-native-registrations")


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
            entries.append(_entry(node.name, "stub-class", f"class {node.name}"))
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

    statements: list[str] = []
    current: list[str] = []
    depth = 0
    for raw in text.splitlines():
        stripped = raw.strip()
        if not current:
            if not stripped.startswith("export "):
                continue
            current = [stripped]
            depth = stripped.count("{") - stripped.count("}")
        else:
            current.append(stripped)
            depth += stripped.count("{") - stripped.count("}")
        single_line_function = (
            len(current) == 1
            and stripped.startswith("export declare function ")
            and ")" in stripped
        )
        if depth == 0 and (
            stripped.endswith(";")
            or stripped.endswith("}")
            or " from " in stripped
            or single_line_function
        ):
            statements.append(" ".join(current))
            current = []
    if current:
        raise SurfaceError(f"unterminated exported declaration in {row}")
    entries = []
    for statement in statements:
        path, kind = _declaration_name(statement)
        entries.append(_entry(path, kind, statement))
    if not entries:
        raise SurfaceError(f"{row} contains no exported declarations")
    return _unique(entries, row)


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
                "entries": parse_typescript_declarations(
                    inputs["typescript_declaration"], "typescript-declarations"
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
    metadata_diffs = {
        key: {"baseline": baseline_metadata.get(key), "candidate": candidate_metadata.get(key)}
        for key in sorted(set(baseline_metadata) | set(candidate_metadata))
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
        for removed_key in removed_keys:
            for added_key in added_keys:
                if before[removed_key]["kind"] == after[added_key]["kind"]:
                    changed.append(
                        {
                            "path": before[removed_key]["path"],
                            "before": before[removed_key],
                            "after": after[added_key],
                        }
                    )
                    break
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
        "row_diffs": row_diffs,
    }


def _run(command: Sequence[str], cwd: Path, env: dict[str, str] | None = None) -> str:
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.returncode != 0:
        diagnostic = "\n".join(
            part.rstrip() for part in (completed.stdout, completed.stderr) if part.strip()
        )
        raise SurfaceError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n{diagnostic}"
        )
    return completed.stdout


def _exact_version(command: Sequence[str], expected: str) -> str:
    output = _run(command, REPO_ROOT).strip()
    if expected not in output:
        raise SurfaceError(f"expected {expected!r} from {' '.join(command)}, got {output!r}")
    return expected


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
    _exact_version(["cargo", "public-api", "--version"], PINNED_CARGO_PUBLIC_API)
    rustc = _run(["rustc", f"+{PINNED_NIGHTLY}", "--version"], REPO_ROOT).strip()
    target_report = _run(["rustc", f"+{PINNED_NIGHTLY}", "-vV"], REPO_ROOT)
    target_match = re.search(r"^host:\s*(\S+)$", target_report, re.MULTILINE)
    if target_match is None:
        raise SurfaceError("nightly rustc did not report a host target")
    node = _run(["node", "--version"], REPO_ROOT).strip()
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
        _run(["npm", "run", "build:native"], ts_root, env)
        _run(["npm", "exec", "--", "tsc", "-p", "tsconfig.build.json"], ts_root, env)
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
        inputs = {
            "rust": rust_outputs,
            "python_exports": (REPO_ROOT / "src/python/fathomdb/__init__.py").read_text(),
            "python_registrations": (
                REPO_ROOT / "src/rust/crates/fathomdb-py/src/lib.rs"
            ).read_text(),
            "python_stub": (REPO_ROOT / "src/python/fathomdb/_fathomdb.pyi").read_text(),
            "napi_declaration": (ts_root / "index.d.ts").read_text(),
            "typescript_declaration": (ts_root / "dist/index.d.ts").read_text(),
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


def _capture_command(args: argparse.Namespace) -> int:
    output = args.output.resolve()
    if output == (REPO_ROOT / "dev/plans/0.8.27/features/slice-30/baseline.json").resolve():
        raise SurfaceError("capture cannot directly rewrite the tracked reviewed baseline")
    manifest = capture_repository(args.source_sha)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(canonical_json(manifest))
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
    except SurfaceError as exc:
        print(f"surface-comparator: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
