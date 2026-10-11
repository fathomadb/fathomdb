#!/usr/bin/env python3
"""Focused non-vacuity contract for the 0.8.27 Slice 30 comparator."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
from pathlib import Path
import random
import shutil
import tempfile
from types import SimpleNamespace
from types import ModuleType
from typing import Any, Callable
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / "dev" / "tools" / "surface_comparator.py"
SOURCE_SHA = "1" * 40


def load_tool() -> ModuleType:
    spec = importlib.util.spec_from_file_location("surface_comparator", TOOL)
    if spec is None or spec.loader is None:
        raise AssertionError(f"cannot load Slice 30 comparator: {TOOL}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def metadata(tool: ModuleType) -> dict[str, Any]:
    return {
        "schema": "fathomdb.surface-baseline.v1",
        "capture_source_sha": SOURCE_SHA,
        "tools": {
            "cargo-public-api": "0.52.0",
            "node": "v25.9.0",
            "rust-toolchain": "nightly-2026-04-24",
            "rustc": "rustc 1.97.0-nightly (36ba2c771 2026-04-23)",
            "typescript": "6.0.3",
        },
        "target": "x86_64-unknown-linux-gnu",
        "row_identities": copy.deepcopy(tool.REQUIRED_ROW_IDENTITIES),
    }


def fixture(tool: ModuleType) -> dict[str, Any]:
    rust = {
        row[
            "id"
        ]: "pub struct fathomdb::Engine\npub fn fathomdb::open() -> fathomdb::Engine\n"
        for row in tool.REQUIRED_ROW_IDENTITIES
        if row["kind"] == "rust"
    }
    rust["rust-engine-operator-test-hooks"] += (
        "pub fn fathomdb_engine::combined_operator_test_hook()\n"
    )
    # cargo-public-api legitimately emits one associated path more than once
    # when blanket impls give it distinct signatures.
    rust["rust-engine-default"] += (
        "pub type fathomdb_engine::Thing::Error = core::convert::Infallible\n"
        "pub type fathomdb_engine::Thing::Error = <U as TryFrom<T>>::Error\n"
        "impl core::fmt::Debug for fathomdb_engine::Thing\n"
        "pub fn fathomdb_engine::Thing::fmt(&self, &mut Formatter) -> Result\n"
        "impl core::fmt::Display for fathomdb_engine::Thing\n"
        "pub fn fathomdb_engine::Thing::fmt(&self, &mut Formatter) -> Result\n"
        # rustdoc may repeat the same blanket impl/item pair while walking
        # different public types; it is one semantic entry.
        "impl core::fmt::Display for fathomdb_engine::Thing\n"
        "pub fn fathomdb_engine::Thing::fmt(&self, &mut Formatter) -> Result\n"
    )
    return {
        "rust": rust,
        "python_exports": """\
from fathomdb.engine import Engine
from fathomdb.types import SearchHit
__all__ = ["Engine", "SearchHit"]
""",
        "python_registrations": """\
fn _fathomdb(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add("StorageError", py.get_type::<StorageError>())?;
    Ok(())
}
""",
        "python_stub": """\
class EngineError(Exception): ...

class StorageError(EngineError): ...

class Engine:
    version: str
    def search(self, query: str, limit: int = ...) -> list[str]: ...

def rerank(query: str, passages: list[str]) -> list[float]: ...
""",
        "napi_declaration": """\
export interface NativeHit {
  body: string
}
export declare function nativeSearch(query: string): Array<NativeHit>
""",
        "typescript_declarations": {
            "index.d.ts": """\
export * from "./errors.js";
export { read } from "./read.js";
export interface SearchHit {
  body: string;
}
export declare function search(query: string): Promise<SearchHit[]>;
""",
            "errors.d.ts": """\
export declare class FathomDbError extends Error {
}
export declare class DatabaseLockedError extends FathomDbError {
}
export interface CorruptionErrorPayload {
  path: string;
}
""",
            "read.d.ts": """\
export interface ReadView {
  id: string;
}
export declare const read: {
    get(id: string): Promise<ReadView>;
    getMany(ids: string[]): Promise<ReadView[]>;
};
export declare const internalOnly = 1;
""",
        },
        "python_sources": {
            "__init__.py": """\
from fathomdb import read
from fathomdb.engine import Engine
__all__ = ["Engine", "read"]
""",
            "engine.py": """\
from typing import Any

class Engine:
    version: str
    def search(self, query: str, limit: int = 10) -> list[str]:
        return [query]
    def _private(self) -> None:
        pass
""",
            "read.py": """\
__all__ = ["get_many"]

def get_many(ids: list[int]) -> list[str]:
    return []
""",
        },
        "package_json": json.dumps(
            {
                "name": "fathomdb",
                "type": "module",
                "main": "dist/index.js",
                "types": "dist/index.d.ts",
                "files": ["dist", "!dist/tests", "!dist/src"],
            }
        ),
        "runtime_exports": ["Engine", "search"],
    }


def assert_row_diff(
    tool: ModuleType,
    base_inputs: dict[str, Any],
    mutate: Callable[[dict[str, Any]], None],
    expected_row: str,
    expected_bucket: str,
    expected_paths: set[str] | None = None,
) -> None:
    baseline = tool.capture_from_fixture(base_inputs, metadata(tool))
    changed_inputs = copy.deepcopy(base_inputs)
    mutate(changed_inputs)
    candidate = tool.capture_from_fixture(changed_inputs, metadata(tool))
    result = tool.compare_manifests(baseline, candidate)
    assert not result["equal"], f"mutation for {expected_row} compared equal"
    # One mutation must disturb exactly its own row.
    assert [item["row"] for item in result["row_diffs"]] == [expected_row], result[
        "row_diffs"
    ]
    row = result["row_diffs"][0]
    assert row[expected_bucket], (
        f"{expected_row} did not report {expected_bucket}: {row}"
    )
    if expected_paths is not None:
        actual = {item["path"] for item in row[expected_bucket]}
        assert actual == expected_paths, (
            f"{expected_row} {expected_bucket} paths {actual}"
        )


def main() -> None:
    tool = load_tool()
    inputs = fixture(tool)
    baseline = tool.capture_from_fixture(inputs, metadata(tool))
    repeat = tool.capture_from_fixture(copy.deepcopy(inputs), metadata(tool))
    assert tool.canonical_json(baseline) == tool.canonical_json(repeat)
    assert tool.compare_manifests(baseline, repeat)["equal"]

    current_metadata = metadata(tool)
    current_metadata["capture_source_sha"] = "2" * 40
    current = tool.capture_from_fixture(copy.deepcopy(inputs), current_metadata)
    current_comparison = tool.compare_manifests(baseline, current)
    assert current_comparison["equal"], current_comparison
    assert current_comparison["metadata_diffs"] == {}
    assert current_comparison["provenance"] == {
        "baseline_capture_source_sha": SOURCE_SHA,
        "candidate_capture_source_sha": "2" * 40,
        "same_source": False,
    }

    assert_row_diff(
        tool,
        inputs,
        lambda value: value["rust"].__setitem__(
            "rust-facade-default",
            value["rust"]["rust-facade-default"] + "pub struct fathomdb::Added\n",
        ),
        "rust-facade-default",
        "added",
        {"fathomdb::Added"},
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value["rust"].__setitem__(
            "rust-facade-default",
            value["rust"]["rust-facade-default"].replace(
                "pub struct fathomdb::Engine\n", ""
            ),
        ),
        "rust-facade-default",
        "removed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "python_registrations",
            value["python_registrations"].replace(
                '    m.add("StorageError", py.get_type::<StorageError>())?;\n', ""
            ),
        ),
        "python-native-registrations",
        "removed",
        {"StorageError"},
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value["rust"].__setitem__(
            "rust-facade-operator",
            value["rust"]["rust-facade-operator"].replace(
                "fathomdb::Engine", "fathomdb::RenamedEngine"
            ),
        ),
        "rust-facade-operator",
        "changed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value["rust"].__setitem__(
            "rust-engine-operator-test-hooks",
            value["rust"]["rust-engine-operator-test-hooks"].replace(
                "pub fn fathomdb_engine::combined_operator_test_hook()\n", ""
            ),
        ),
        "rust-engine-operator-test-hooks",
        "removed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "python_stub",
            value["python_stub"].replace(
                "class StorageError(EngineError): ...",
                "class StorageError(Exception): ...",
            ),
        ),
        "python-native-stub",
        "changed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "python_registrations",
            value["python_registrations"].replace(
                "    m.add_function(wrap_pyfunction!(search, m)?)?;\n", ""
            ),
        ),
        "python-native-registrations",
        "removed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "python_stub",
            value["python_stub"].replace(
                "    def search(self, query: str, limit: int = ...) -> list[str]: ...\n",
                "",
            ),
        ),
        "python-native-stub",
        "removed",
        {"Engine.search"},
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "napi_declaration",
            value["napi_declaration"].replace("body: string", "body: Uint8Array"),
        ),
        "napi-production",
        "changed",
        {"NativeHit"},
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "typescript_declarations",
            {
                **value["typescript_declarations"],
                "index.d.ts": value["typescript_declarations"]["index.d.ts"].replace(
                    "search(query: string)", "lookup(query: string)"
                ),
            },
        ),
        "typescript-declarations",
        "changed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__("runtime_exports", ["Engine", "lookup"]),
        "package-entrypoints",
        "changed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "package_json",
            value["package_json"].replace("dist/index.js", "dist/main.js"),
        ),
        "package-entrypoints",
        "changed",
    )

    wrong_metadata = metadata(tool)
    wrong_metadata["row_identities"] = copy.deepcopy(wrong_metadata["row_identities"])
    wrong_metadata["row_identities"][0]["args"] = ["--wrong-row"]
    mismatch = tool.compare_manifests(
        baseline,
        tool.capture_from_fixture(inputs, wrong_metadata),
    )
    assert not mismatch["equal"]
    assert "row_identities" in mismatch["metadata_diffs"]

    duplicate_manifest = copy.deepcopy(baseline)
    duplicate_manifest["rows"][0]["entries"].append(
        copy.deepcopy(duplicate_manifest["rows"][0]["entries"][0])
    )
    try:
        tool.compare_manifests(baseline, duplicate_manifest)
    except tool.SurfaceError as exc:
        assert "duplicate" in str(exc)
    else:
        raise AssertionError("duplicate manifest keys must fail closed")

    invalid_provenance = copy.deepcopy(current)
    invalid_provenance["metadata"]["capture_source_sha"] = "not-a-sha"
    try:
        tool.compare_manifests(baseline, invalid_provenance)
    except tool.ComparatorError as exc:
        assert "capture_source_sha" in str(exc)
    else:
        raise AssertionError("invalid provenance must fail closed")

    with mock.patch.object(
        tool.subprocess, "run", side_effect=FileNotFoundError("missing")
    ):
        try:
            tool._run(["missing-slice30-tool"], ROOT)
        except tool.ComparatorError as exc:
            assert "missing-slice30-tool" in str(exc)
        else:
            raise AssertionError("missing executable must raise ComparatorError")

    with mock.patch.object(tool, "_run", return_value="cargo-public-api 10.52.0\n"):
        try:
            tool._exact_version(
                ["cargo", "public-api", "--version"],
                "cargo-public-api 0.52.0",
            )
        except tool.ComparatorError:
            pass
        else:
            raise AssertionError("tool version substring must not satisfy an exact pin")

    baseline_path = ROOT / "dev/plans/0.8.27/features/slice-30/baseline.json"
    with tempfile.TemporaryDirectory(dir=baseline_path.parent) as directory:
        temporary = Path(directory)
        hardlink = temporary / "baseline-hardlink.json"
        os.link(baseline_path, hardlink)
        try:
            tool._guard_capture_output(hardlink)
        except tool.ComparatorError as exc:
            assert "reviewed baseline" in str(exc)
        else:
            raise AssertionError("hardlink alias must not permit baseline rewrite")

        output = temporary / "candidate.json"
        output.write_text("original\n")
        with mock.patch.object(
            tool.os, "replace", side_effect=OSError("replace refused")
        ):
            try:
                tool._atomic_write(output, "replacement\n")
            except tool.ComparatorError as exc:
                assert "atomic" in str(exc)
            else:
                raise AssertionError("atomic replacement failure must be typed")
        assert output.read_text() == "original\n"
        assert list(temporary.glob(f".{output.name}.*.tmp")) == []

    def with_module(value: dict[str, Any], key: str, module: str, text: str) -> None:
        value[key] = {**value[key], module: text}

    # Re-exported TypeScript declarations are part of the root surface.
    ts = inputs["typescript_declarations"]
    assert_row_diff(
        tool,
        inputs,
        lambda value: with_module(
            value,
            "typescript_declarations",
            "errors.d.ts",
            ts["errors.d.ts"].replace(
                "DatabaseLockedError extends FathomDbError",
                "DatabaseLockedError extends Error",
            ),
        ),
        "typescript-declarations",
        "changed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: with_module(
            value,
            "typescript_declarations",
            "errors.d.ts",
            ts["errors.d.ts"].replace(
                "export interface CorruptionErrorPayload {\n  path: string;\n}\n", ""
            ),
        ),
        "typescript-declarations",
        "removed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: with_module(
            value,
            "typescript_declarations",
            "read.d.ts",
            ts["read.d.ts"].replace(
                "    getMany(ids: string[]): Promise<ReadView[]>;\n", ""
            ),
        ),
        "typescript-declarations",
        "changed",
    )
    ts_entries = next(
        row["entries"]
        for row in baseline["rows"]
        if row["id"] == "typescript-declarations"
    )
    ts_paths = {entry["path"] for entry in ts_entries}
    assert {"DatabaseLockedError", "CorruptionErrorPayload", "read"} <= ts_paths, (
        ts_paths
    )
    assert "internalOnly" not in ts_paths and "ReadView" not in ts_paths, ts_paths
    assert not any(path.startswith("export:") for path in ts_paths), ts_paths
    moved_ts = copy.deepcopy(inputs)
    moved_ts["typescript_declarations"] = {
        "index.d.ts": 'export * from "./errors.js";\nexport * from "./search.js";\n'
        'export { read } from "./read.js";\n',
        "errors.d.ts": ts["errors.d.ts"],
        "read.d.ts": ts["read.d.ts"],
        "search.d.ts": "export interface SearchHit {\n  body: string;\n}\n"
        "export declare function search(query: string): Promise<SearchHit[]>;\n",
    }
    moved_ts_result = tool.compare_manifests(
        baseline, tool.capture_from_fixture(moved_ts, metadata(tool))
    )
    assert moved_ts_result["equal"], moved_ts_result
    missing_module = copy.deepcopy(inputs)
    del missing_module["typescript_declarations"]["read.d.ts"]
    try:
        tool.capture_from_fixture(missing_module, metadata(tool))
    except tool.ComparatorError as exc:
        assert "read" in str(exc)
    else:
        raise AssertionError("unresolved TypeScript re-export must fail closed")

    # Python wrapper modules: public names and callable signatures, not bodies
    # or file placement.
    py = inputs["python_sources"]
    assert_row_diff(
        tool,
        inputs,
        lambda value: with_module(
            value,
            "python_sources",
            "engine.py",
            py["engine.py"].replace("limit: int = 10", "max_hits: int = 10"),
        ),
        "python-wrapper-declarations",
        "changed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: with_module(
            value,
            "python_sources",
            "read.py",
            "__all__ = []\n",
        ),
        "python-wrapper-declarations",
        "removed",
    )
    body_only = copy.deepcopy(inputs)
    body_only["python_sources"]["engine.py"] = py["engine.py"].replace(
        "return [query]", "return [query, query]"
    )
    body_result = tool.compare_manifests(
        baseline, tool.capture_from_fixture(body_only, metadata(tool))
    )
    assert body_result["equal"], body_result
    moved_py = copy.deepcopy(inputs)
    moved_py["python_sources"]["_engine_impl.py"] = py["engine.py"]
    moved_py["python_sources"]["engine.py"] = (
        "from fathomdb._engine_impl import Engine\n"
    )
    moved_py_result = tool.compare_manifests(
        baseline, tool.capture_from_fixture(moved_py, metadata(tool))
    )
    assert moved_py_result["equal"], moved_py_result
    py_paths = {
        entry["path"]
        for row in baseline["rows"]
        if row["id"] == "python-wrapper-declarations"
        for entry in row["entries"]
    }
    assert {
        "fathomdb.Engine.search",
        "fathomdb.read.get_many",
        "fathomdb.engine.Engine",
    } <= py_paths, py_paths
    assert not any("_private" in path or path.endswith(".Any") for path in py_paths), (
        py_paths
    )

    # cfg-gated PyO3 registrations are distinguishable from shipped ones.
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "python_registrations",
            value["python_registrations"].replace(
                "    m.add_class::<PyEngine>()?;\n",
                '    #[cfg(feature = "test-hooks")]\n    m.add_class::<PyEngine>()?;\n',
            ),
        ),
        "python-native-registrations",
        "changed",
    )

    # The recorded NAPI build identity must match the script capture runs.
    napi_identity = next(
        row for row in tool.REQUIRED_ROW_IDENTITIES if row["id"] == "napi-production"
    )
    good_package = json.dumps({"scripts": {"build:native": napi_identity["script"]}})
    tool._validate_napi_build_script(good_package)
    drifted = json.dumps(
        {"scripts": {"build:native": napi_identity["script"] + " --unexpected"}}
    )
    try:
        tool._validate_napi_build_script(drifted)
    except tool.ComparatorError as exc:
        assert "build:native" in str(exc)
    else:
        raise AssertionError("drifted build:native script must fail closed")

    # cfg on an enclosing item (fn/mod) gates every registration inside it.
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "python_registrations",
            value["python_registrations"].replace("    Ok(())\n}\n", "    Ok(())\n}\n")
            + '#[cfg(feature = "test-hooks")]\n'
            "fn register_hooks(m: &Bound<'_, PyModule>) -> PyResult<()> {\n"
            "    m.add_class::<PyEngine>()?;\n"
            "    Ok(())\n"
            "}\n",
        ),
        "python-native-registrations",
        "added",
    )
    gated_fn = tool.parse_python_registrations(
        '#[cfg(feature = "test-hooks")]\n'
        "fn register_hooks(m: &Bound<'_, PyModule>) -> PyResult<()> {\n"
        "    m.add_class::<PyHook>()?;\n"
        "    Ok(())\n"
        "}\n"
        '#[cfg(feature = "test-hooks")]\n'
        "mod hooks {\n"
        "    fn more(m: &Bound<'_, PyModule>) {\n"
        '        m.add("HookError", py.get_type::<HookError>())?;\n'
        "    }\n"
        "}\n"
        "fn shipped(m: &Bound<'_, PyModule>) -> PyResult<()> {\n"
        "    m.add_class::<PyShipped>()?;\n"
        "    Ok(())\n"
        "}\n"
    )
    by_path = {entry["path"]: entry["signature"] for entry in gated_fn}
    assert by_path["PyHook"].startswith('#[cfg(feature = "test-hooks")]'), by_path
    assert by_path["HookError"].startswith('#[cfg(feature = "test-hooks")]'), by_path
    assert by_path["PyShipped"] == "add_class::<PyShipped>", by_path

    # Documentation is not surface: comment-only edits compare equal, and
    # braces inside comments cannot merge or split declarations.
    documented = copy.deepcopy(inputs)
    documented["typescript_declarations"]["index.d.ts"] = ts["index.d.ts"].replace(
        "export declare function search",
        "/** Search; see {@link SearchHit}. Unbalanced opener: `{`. */\n"
        "export declare function search",
    )
    documented["napi_declaration"] = inputs["napi_declaration"].replace(
        "export declare function nativeSearch",
        '/**\n * if x.is_some() { obj.set("x", x)?;\n * // not code\n */\n'
        "export declare function nativeSearch",
    )
    documented_result = tool.compare_manifests(
        baseline, tool.capture_from_fixture(documented, metadata(tool))
    )
    assert documented_result["equal"], documented_result
    literal = tool.parse_typescript_declarations(
        'export type Url = "https://example.test/*path";\nexport type Next = "b";\n',
        "literal",
    )
    assert [entry["path"] for entry in literal] == ["Next", "Url"], literal
    assert "https://example.test/*path" in literal[1]["signature"], literal

    # --- Rust impl context keeps same-named associated items distinct.
    thing = next(row for row in baseline["rows"] if row["id"] == "rust-engine-default")
    fmt = sorted(
        e["signature"]
        for e in thing["entries"]
        if e["path"] == "fathomdb_engine::Thing::fmt"
    )
    assert len(fmt) == 2, fmt
    assert fmt[0].startswith("impl core::fmt::Debug for fathomdb_engine::Thing =>"), fmt
    assert fmt[1].startswith("impl core::fmt::Display for fathomdb_engine::Thing =>"), (
        fmt
    )
    errors = [
        e for e in thing["entries"] if e["path"] == "fathomdb_engine::Thing::Error"
    ]
    assert len(errors) == 2, errors
    assert_row_diff(
        tool,
        inputs,
        lambda value: value["rust"].__setitem__(
            "rust-engine-default",
            value["rust"]["rust-engine-default"].replace(
                "impl core::fmt::Display for fathomdb_engine::Thing\n", ""
            ),
        ),
        "rust-engine-default",
        "removed",
    )

    # --- Tool and target identity are part of the comparison.
    for key, value in (
        ("tools", {**metadata(tool)["tools"], "node": "v26.8.2"}),
        ("target", "aarch64-unknown-linux-gnu"),
    ):
        other = metadata(tool)
        other[key] = value
        result = tool.compare_manifests(
            baseline, tool.capture_from_fixture(inputs, other)
        )
        assert not result["equal"] and key in result["metadata_diffs"], result[
            "metadata_diffs"
        ]
    wrong_schema = metadata(tool)
    wrong_schema["schema"] = "other"
    try:
        tool.capture_from_fixture(inputs, wrong_schema)
    except tool.SurfaceError as exc:
        assert "schema" in str(exc)
    else:
        raise AssertionError("wrong schema must fail closed")

    # --- Root package export row.
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "python_exports",
            value["python_exports"].replace('"Engine", "SearchHit"', '"Engine"'),
        ),
        "python-package-exports",
        "removed",
        {"SearchHit"},
    )
    for bad in (
        '__all__ = [name for name in ("a",)]\n',
        "x = 1\n",
        '__all__ = ["a", 1]\n',
    ):
        try:
            tool.parse_python_exports(bad)
        except tool.SurfaceError:
            pass
        else:
            raise AssertionError(f"invalid __all__ must fail closed: {bad!r}")

    # --- Capture-side provenance guards (AC27-30A).
    head = "a" * 40

    def fake_run(outputs: dict[tuple[str, ...], str]) -> Callable[..., str]:
        def run(command: Any, cwd: Any = None, env: Any = None) -> str:
            key = tuple(command[:3])
            for prefix, output in outputs.items():
                if key[: len(prefix)] == prefix:
                    return output
            raise AssertionError(f"unexpected command {command}")

        return run

    def expect_error(error: type, needle: str, call: Callable[[], Any]) -> None:
        try:
            call()
        except error as exc:
            assert needle in str(exc), str(exc)
        else:
            raise AssertionError(f"expected {error.__name__} containing {needle!r}")

    clean = {("git", "rev-parse"): head + "\n", ("git", "status"): ""}
    with mock.patch.object(tool, "_run", side_effect=fake_run(clean)):
        tool._clean_source(head)
        expect_error(
            tool.SurfaceError,
            "does not match HEAD",
            lambda: tool._clean_source("b" * 40),
        )
        expect_error(
            tool.SurfaceError, "full 40-character", lambda: tool._clean_source(head[:7])
        )
    dirty = {("git", "rev-parse"): head + "\n", ("git", "status"): " M src/lib.rs\n"}
    with mock.patch.object(tool, "_run", side_effect=fake_run(dirty)):
        expect_error(
            tool.SurfaceError, "clean worktree", lambda: tool._clean_source(head)
        )

    tools_ok = {
        ("cargo", "public-api", "--version"): "cargo-public-api 0.52.0\n",
        ("rustc", "+nightly-2026-04-24", "--version"): "rustc 1.97.0-nightly\n",
        ("rustc", "+nightly-2026-04-24", "-vV"): "host: x86_64-unknown-linux-gnu\n",
        ("node", "--version"): "v25.9.0\n",
        ("npm", "exec", "--"): "Version 6.0.3\n",
    }
    with mock.patch.object(tool, "_run", side_effect=fake_run(tools_ok)):
        produced = tool._tool_metadata(head)
        assert (
            produced["tools"]["typescript"] == "6.0.3"
            and produced["tools"]["node"] == "v25.9.0"
        )
        assert produced["target"] == "x86_64-unknown-linux-gnu"
    with mock.patch.object(
        tool,
        "_run",
        side_effect=fake_run({**tools_ok, ("node", "--version"): "v26.8.2\n"}),
    ):
        expect_error(tool.ComparatorError, "v25.9.0", lambda: tool._tool_metadata(head))
    with mock.patch.object(
        tool,
        "_run",
        side_effect=fake_run({**tools_ok, ("npm", "exec", "--"): "6.0.3\n"}),
    ):
        expect_error(
            tool.SurfaceError, "TypeScript version", lambda: tool._tool_metadata(head)
        )

    with tempfile.TemporaryDirectory() as directory:
        scratch = Path(directory) / "scratch"
        scratch.mkdir()
        # The capacity guard reads real disk usage; pin it so this ownership
        # check does not depend on the host's free space.
        with (
            mock.patch.object(tool, "OWNED_SCRATCH", scratch),
            mock.patch.object(tool, "OWNED_CACHE", Path(directory) / "cache"),
            mock.patch.object(
                tool.shutil,
                "disk_usage",
                return_value=SimpleNamespace(free=tool.MIN_FREE_BYTES * 10),
            ),
        ):
            expect_error(tool.SurfaceError, "ownership marker", tool._prepare_scratch)
            (scratch / tool.SCRATCH_MARKER).write_text(tool.SCRATCH_MARKER_CONTENT)
            (scratch / "leftover").write_text("x")
            tool._prepare_scratch()
            assert (
                not (scratch / "leftover").exists()
                and (scratch / tool.SCRATCH_MARKER).is_file()
            )
            with mock.patch.object(
                tool.shutil,
                "disk_usage",
                return_value=shutil._ntuple_diskusage(1, 1, 1),
            ):
                expect_error(tool.SurfaceError, "free bytes", tool._prepare_scratch)

    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        scratch = root / "scratch"
        scratch.mkdir()
        (scratch / tool.SCRATCH_MARKER).write_text("forged\n")
        with (
            mock.patch.object(tool, "OWNED_SCRATCH", scratch),
            mock.patch.object(tool, "OWNED_CACHE", root / "cache"),
            mock.patch.object(
                tool.shutil,
                "disk_usage",
                return_value=SimpleNamespace(free=tool.MIN_FREE_BYTES * 10),
            ),
        ):
            expect_error(tool.SurfaceError, "ownership marker", tool._prepare_scratch)
        target = root / "target"
        target.mkdir()
        shutil.rmtree(scratch)
        scratch.symlink_to(target, target_is_directory=True)
        with (
            mock.patch.object(tool, "OWNED_SCRATCH", scratch),
            mock.patch.object(tool, "OWNED_CACHE", root / "cache"),
            mock.patch.object(
                tool.shutil,
                "disk_usage",
                return_value=SimpleNamespace(free=tool.MIN_FREE_BYTES * 10),
            ),
        ):
            expect_error(tool.SurfaceError, "real directory", tool._prepare_scratch)

    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        scratch = root / "scratch"
        scratch.mkdir()
        external = root / "external-marker"
        external.write_text(tool.SCRATCH_MARKER_CONTENT)
        (scratch / tool.SCRATCH_MARKER).symlink_to(external)
        with mock.patch.object(tool, "OWNED_SCRATCH", scratch):
            expect_error(tool.SurfaceError, "ownership marker", tool._assert_owned_scratch)
        (scratch / tool.SCRATCH_MARKER).unlink()
        (scratch / tool.SCRATCH_MARKER).mkdir()
        with mock.patch.object(tool, "OWNED_SCRATCH", scratch):
            expect_error(tool.SurfaceError, "ownership marker", tool._assert_owned_scratch)
        shutil.rmtree(scratch / tool.SCRATCH_MARKER)
        (scratch / tool.SCRATCH_MARKER).write_text("changed during capture\n")
        with (
            mock.patch.object(tool, "OWNED_SCRATCH", scratch),
            mock.patch.object(tool.shutil, "rmtree") as remove,
        ):
            expect_error(tool.SurfaceError, "ownership marker", tool._cleanup_scratch)
            remove.assert_not_called()
            assert scratch.exists()

    # Exercise filesystem enumeration itself: the two roots are deduplicated
    # only when their nearest existing parents report the same device.
    with (
        mock.patch.object(
            tool,
            "_existing_parent",
            side_effect=[Path("/cache-parent"), Path("/scratch-parent")],
        ),
        mock.patch.object(
            Path,
            "stat",
            side_effect=[SimpleNamespace(st_dev=7), SimpleNamespace(st_dev=7)],
        ),
    ):
        assert tool._capture_filesystems() == [("cache/scratch", Path("/cache-parent"))]
    with (
        mock.patch.object(
            tool,
            "_existing_parent",
            side_effect=[Path("/cache-parent"), Path("/scratch-parent")],
        ),
        mock.patch.object(
            Path,
            "stat",
            side_effect=[SimpleNamespace(st_dev=7), SimpleNamespace(st_dev=8)],
        ),
    ):
        assert tool._capture_filesystems() == [
            ("cache", Path("/cache-parent")),
            ("scratch", Path("/scratch-parent")),
        ]

    # The cache and scratch may live on different filesystems. Both must pass
    # the heavy-route floor rather than inheriting the repository result.
    with (
        mock.patch.object(
            tool,
            "_capture_filesystems",
            return_value=[("cache", Path("/cache")), ("scratch", Path("/scratch"))],
        ),
        mock.patch.object(
            tool.shutil,
            "disk_usage",
            side_effect=[
                shutil._ntuple_diskusage(200_000_000_000, 0, 150_000_000_000),
                shutil._ntuple_diskusage(200_000_000_000, 0, 1),
            ],
        ),
    ):
        expect_error(
            tool.SurfaceError, "scratch filesystem", tool._check_capture_capacity
        )

    generation = {
        ("cargo",): "pub struct fathomdb::Engine\n",
        ("npm", "run"): "",
        ("npm", "exec", "--"): "",
        ("node", "--input-type=module"): '["Engine"]\n',
        ("git", "status"): " M src/ts/index.d.ts\n",
    }
    with (
        tempfile.TemporaryDirectory() as directory,
        mock.patch.object(tool, "OWNED_SCRATCH", Path(directory)),
        mock.patch.object(tool, "_clean_source"),
        mock.patch.object(tool, "_prepare_scratch"),
        mock.patch.object(tool, "_cleanup_scratch"),
        mock.patch.object(tool, "_tool_metadata", return_value=metadata(tool)),
        mock.patch.object(tool, "_validate_napi_build_script"),
        mock.patch.object(tool, "_run", side_effect=fake_run(generation)),
    ):
        expect_error(
            tool.SurfaceError,
            "generation changed",
            lambda: tool.capture_repository(head),
        )
    assert (
        not Path("/tmp/fathomdb-0.8.27-slice30").exists()
        or (Path("/tmp/fathomdb-0.8.27-slice30") / tool.SCRATCH_MARKER).is_file()
    ), "tests must never create the real scratch root without its marker"

    # --- Adapter paths the real baseline relies on.
    modules = {
        "index.d.ts": 'export type { ReadView } from "./read.js";\n'
        'export { search as find } from "./search.js";\n'
        'export * from "./defaults.js";\n'
        'export * from "./a.js";\n',
        "read.d.ts": "export interface ReadView {\n  id: string;\n}\n",
        "search.d.ts": "export declare function search(q: string): void;\n",
        "defaults.d.ts": "export default function hidden(): void;\nexport declare const kept = 1;\n",
        "a.d.ts": 'export * from "./b.js";\nexport declare const fromA = 1;\n',
        "b.d.ts": 'export * from "./a.js";\nexport declare const fromB = 1;\n',
    }
    surface = {e["path"]: e for e in tool.parse_typescript_surface(modules, "fixture")}
    assert set(surface) == {"ReadView", "find", "kept", "fromA", "fromB"}, surface
    assert surface["ReadView"]["kind"] == "interface"
    assert surface["find"]["signature"].startswith("export { search as find } =>"), (
        surface["find"]
    )
    expect_error(
        tool.ComparatorError,
        "unterminated block comment",
        lambda: tool.parse_typescript_declarations(
            "/* open\nexport declare const x = 1;\n", "c"
        ),
    )
    relative = tool.parse_python_wrappers(
        {
            "__init__.py": 'from .engine import Engine\n__all__ = ["Engine"]\n',
            "engine.py": "class Engine:\n    def close(self) -> None:\n        pass\n",
        }
    )
    assert {e["path"] for e in relative} >= {
        "fathomdb.Engine",
        "fathomdb.Engine.close",
    }, relative
    baseline_path = ROOT / "dev/plans/0.8.27/features/slice-30/baseline.json"
    with tempfile.TemporaryDirectory() as directory:
        alias = Path(directory) / "alias.json"
        alias.symlink_to(baseline_path)
        expect_error(
            tool.ComparatorError,
            "reviewed baseline",
            lambda: tool._guard_capture_output(alias),
        )

    # --- Seeded generative properties for the normalization layer.
    generator = random.Random(2708)
    rust_blocks = [
        ["pub struct fathomdb_engine::Thing"],
        [
            "impl core::fmt::Debug for fathomdb_engine::Thing",
            "pub fn fathomdb_engine::Thing::fmt(&self, &mut Formatter) -> Result",
        ],
        [
            "impl core::fmt::Display for fathomdb_engine::Thing",
            "pub fn fathomdb_engine::Thing::fmt(&self, &mut Formatter) -> Result",
        ],
        ["pub struct fathomdb_engine::Other"],
        ["pub fn fathomdb_engine::open(path: &Path) -> Result<Engine, EngineError>"],
    ]
    reference = tool.parse_rust_public_api(
        "\n".join(line for block in rust_blocks for line in block), "p"
    )
    for _ in range(200):
        blocks = [list(block) for block in rust_blocks]
        blocks += [
            list(generator.choice(rust_blocks)) for _ in range(generator.randrange(3))
        ]
        generator.shuffle(blocks)
        lines = []
        for block in blocks:
            for line in block:
                spaced = line.replace(" ", " " * generator.randrange(1, 3))
                lines.append(" " * generator.randrange(4) + spaced)
                if generator.random() < 0.2:
                    lines.append("")
        assert tool.parse_rust_public_api("\n".join(lines), "p") == reference, lines

    alphabet = "abcXYZ_:()<>&{}é漢 "
    for _ in range(100):
        rows = []
        for index in range(generator.randrange(1, 4)):
            entries = {}
            for _ in range(generator.randrange(1, 6)):
                path = "".join(
                    generator.choice(alphabet) for _ in range(generator.randrange(1, 8))
                )
                signature = "".join(
                    generator.choice(alphabet)
                    for _ in range(generator.randrange(0, 12))
                )
                entries[(path, signature)] = {
                    "path": path,
                    "kind": "k",
                    "signature": signature,
                }
            rows.append({"id": f"row-{index}", "entries": list(entries.values())})
        manifest = {"metadata": metadata(tool), "rows": rows}
        round_tripped = json.loads(tool.canonical_json(manifest))
        assert tool.canonical_json(round_tripped) == tool.canonical_json(manifest)
        assert tool.compare_manifests(manifest, round_tripped)["equal"]

    for _ in range(300):
        pieces, literals = [], []
        for _ in range(generator.randrange(1, 10)):
            choice = generator.randrange(4)
            if choice == 0:
                quote = generator.choice("\"'`")
                body = "".join(
                    generator.choice(["//", "/*", "*/", "{", "x", " "])
                    for _ in range(4)
                )
                literal = f"{quote}LIT{body}{quote}"
                literals.append(literal)
                pieces.append(literal)
            elif choice == 1:
                pieces.append("// CMT { " + generator.choice(["", "'", '"']) + "\n")
            elif choice == 2:
                pieces.append("/* CMT } { \n */")
            else:
                pieces.append(
                    generator.choice(["export ", "type ", "{", "}", ";", "\n", "x"])
                )
        text = "".join(pieces)
        stripped = tool._strip_comments(text)
        assert tool._strip_comments(stripped) == stripped, text
        assert "CMT" not in stripped, (text, stripped)
        assert all(literal in stripped for literal in literals), (text, stripped)
        assert stripped.count("\n") == text.count("\n") - sum(
            piece.count("\n") for piece in pieces if piece.startswith("// CMT")
        ) + sum(1 for piece in pieces if piece.startswith("// CMT")), (text, stripped)

    # --- Phase 3: locally declared shapes exported by a bare export list.
    local = {
        "index.d.ts": "declare class Engine {\n    close(): void;\n}\n"
        "declare const read: {\n    get(id: string): void;\n};\n"
        "interface Hidden {\n    id: string;\n}\n"
        "export { Engine, read as reader };\nexport type { Hidden };\n",
    }
    local_surface = {
        e["path"]: e for e in tool.parse_typescript_surface(local, "local")
    }
    assert set(local_surface) == {"Engine", "reader", "Hidden"}, local_surface
    assert "close(): void" in local_surface["Engine"]["signature"], local_surface[
        "Engine"
    ]
    assert "get(id: string)" in local_surface["reader"]["signature"], local_surface[
        "reader"
    ]
    changed_local = {"index.d.ts": local["index.d.ts"].replace("close(): void;\n", "")}
    assert tool.parse_typescript_surface(changed_local, "local") != list(
        local_surface.values()
    )
    expect_error(
        tool.ComparatorError,
        "Nope",
        lambda: tool.parse_typescript_surface(
            {"index.d.ts": "export { Nope };\n"}, "local"
        ),
    )

    # --- Phase 3: generic/unsafe impl headers own the items that follow them.
    owned = {
        e["path"]: e["signature"]
        for e in tool.parse_rust_public_api(
            "pub struct fathomdb::X\n"
            "impl core::panic::unwind_safe::UnwindSafe for fathomdb::X\n"
            "impl<T> core::convert::From<T> for T\n"
            "pub fn fathomdb::X::from(T) -> T\n"
            "unsafe impl core::marker::Send for fathomdb::X\n"
            "impl<T, U> core::convert::Into<U> for T where U: core::convert::From<T>\n"
            "pub fn fathomdb::X::into(self) -> U\n",
            "owned",
        )
    }
    assert owned["fathomdb::X::from"].startswith(
        "impl<T> core::convert::From<T> for T =>"
    ), owned
    assert owned["fathomdb::X::into"].startswith("impl<T, U> core::convert::Into<U>"), (
        owned
    )

    # --- Phase 3: cfg scanning ignores literal braces and gates single-line forms.
    literal_cfg = {
        e["path"]: e["signature"]
        for e in tool.parse_python_registrations(
            '#[cfg(feature = "x")] m.add_class::<PyInline>()?;\n'
            '#[cfg(feature = "test-hooks")]\n'
            "fn hooks(m: &Bound<'_, PyModule>) -> PyResult<()> {\n"
            "    let opener = \"{\"; let brace = '{'; // stray { in a comment\n"
            "    m.add_class::<PyHook>()?;\n"
            "    Ok(())\n"
            "}\n"
            "fn shipped(m: &Bound<'_, PyModule>) -> PyResult<()> {\n"
            "    m.add_class::<PyShipped>()?;\n"
            "    Ok(())\n"
            "}\n"
        )
    }
    assert literal_cfg["PyInline"].startswith('#[cfg(feature = "x")]'), literal_cfg
    assert literal_cfg["PyHook"].startswith('#[cfg(feature = "test-hooks")]'), (
        literal_cfg
    )
    assert literal_cfg["PyShipped"] == "add_class::<PyShipped>", literal_cfg
    expect_error(
        tool.ComparatorError,
        "unbalanced",
        lambda: tool.parse_python_registrations(
            "fn a() {\n    m.add_class::<PyA>()?;\n"
        ),
    )

    # --- Phase 3: changed pairing consumes each added entry once.
    def manifest_of(entries: list[tuple[str, str]]) -> dict[str, Any]:
        return {
            "metadata": metadata(tool),
            "rows": [
                {
                    "id": "r",
                    "entries": [
                        {"path": p, "kind": "k", "signature": g} for p, g in entries
                    ],
                }
            ],
        }

    paired = tool.compare_manifests(
        manifest_of([("a", "1"), ("b", "2")]), manifest_of([("c", "3"), ("d", "4")])
    )["row_diffs"][0]["changed"]
    assert sorted(item["after"]["path"] for item in paired) == ["c", "d"], paired
    same_path = tool.compare_manifests(
        manifest_of([("a", "1"), ("b", "2")]), manifest_of([("b", "3"), ("a", "4")])
    )["row_diffs"][0]["changed"]
    assert all(item["before"]["path"] == item["after"]["path"] for item in same_path), (
        same_path
    )

    # --- Phase 3: parsing edge cases.
    enum_entries = tool.parse_typescript_declarations(
        'export declare const enum Mode {\n  A = "a"\n}\nexport type Brace = "{";\n',
        "enum",
    )
    assert {e["path"]: e["kind"] for e in enum_entries} == {
        "Mode": "enum",
        "Brace": "type",
    }, enum_entries
    missing_input = copy.deepcopy(inputs)
    del missing_input["python_sources"]
    expect_error(
        tool.ComparatorError,
        "python_sources",
        lambda: tool.capture_from_fixture(missing_input, metadata(tool)),
    )

    # --- Phase 3: TypeScript declarations are emitted into the owned scratch root.
    commands: list[list[str]] = []
    environments: list[dict[str, str] | None] = []
    recording = fake_run({**generation, ("git", "status"): ""})

    def record(command: Any, cwd: Any = None, env: Any = None) -> str:
        commands.append(list(command))
        environments.append(env)
        if command[:3] == ["git", "status", "--porcelain"] and len(commands) > 1:
            raise tool.SurfaceError("stop after generation")
        return recording(command, cwd, env)

    with tempfile.TemporaryDirectory() as directory:
        owned = Path(directory) / "scratch"
        owned.mkdir()
        (owned / tool.SCRATCH_MARKER).write_text(tool.SCRATCH_MARKER_CONTENT)
        with (
            mock.patch.object(tool, "OWNED_SCRATCH", owned),
            mock.patch.object(tool, "_clean_source"),
            mock.patch.object(tool, "_prepare_scratch"),
            mock.patch.object(tool, "_tool_metadata", return_value=metadata(tool)),
            mock.patch.object(tool, "_validate_napi_build_script"),
            mock.patch.object(tool, "_run", side_effect=record),
        ):
            expect_error(
                tool.SurfaceError,
                "stop after generation",
                lambda: tool.capture_repository(head),
            )
        declaration = [c for c in commands if "--emitDeclarationOnly" in c]
        assert declaration and str(owned / "ts-declarations") in declaration[0], (
            commands
        )
        # The canonical wrapper owns the targeted clean and private temporary
        # variables. Capture supplies a scratch-owned directory to it.
        napi = commands.index(["npm", "run", "build:native"])
        napi_env = environments[napi] or {}
        requested = napi_env.get("FATHOMDB_NAPI_BUILD_TMPDIR", "")
        assert requested.startswith(str(owned)), requested
        target = napi_env.get("CARGO_TARGET_DIR", "")
        assert target.startswith(str(tool.OWNED_CACHE)), (commands[napi], target)

    # --- Phase 3 FIX-2: multi-line cfg text is recorded verbatim.
    multi = {
        e["path"]: e["signature"]
        for e in tool.parse_python_registrations(
            "#[cfg(any(\n"
            '    feature = "alpha",\n'
            '    feature = "other"\n'
            "))]\n"
            "m.add_class::<PyMulti>()?;\n"
            "#[cfg(\n"
            '    feature = "y"\n'
            ")]\n"
            "m.add_class::<PySingle>()?;\n"
        )
    }
    assert (
        multi["PyMulti"]
        == '#[cfg(any( feature = "alpha", feature = "other" ))] add_class::<PyMulti>'
    ), multi
    assert multi["PySingle"] == '#[cfg( feature = "y" )] add_class::<PySingle>', multi

    multiline_item = {
        e["path"]: e["signature"]
        for e in tool.parse_python_registrations(
            '#[cfg(feature = "test-hooks")]\n'
            "fn register_hooks(\n"
            "    module: &Bound<'_, PyModule>,\n"
            ") -> PyResult<()> {\n"
            "    module.add_class::<PyMultiline>()?;\n"
            "}\n"
        )
    }
    assert multiline_item["PyMultiline"].startswith('#[cfg(feature = "test-hooks")]'), (
        multiline_item
    )

    # --- Phase 3 FIX-2: same-path pairs win regardless of processing order.
    ordered = tool.compare_manifests(
        manifest_of([("a", "1"), ("b", "old")]), manifest_of([("b", "new"), ("c", "3")])
    )["row_diffs"][0]["changed"]
    pairs = sorted((item["before"]["path"], item["after"]["path"]) for item in ordered)
    assert pairs == [("a", "c"), ("b", "b")], pairs

    agent_test = (ROOT / "scripts/agent-test.sh").read_text()
    assert "test-slice30-surface-comparator" in agent_test
    assert "scripts/tests/test_slice30_surface_comparator.py" in agent_test

    print("ok    slice30-surface-comparator")


if __name__ == "__main__":
    main()
