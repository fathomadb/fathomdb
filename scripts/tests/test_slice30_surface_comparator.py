#!/usr/bin/env python3
"""Focused non-vacuity contract for the 0.8.27 Slice 30 comparator."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
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
            "typescript": "6.0.3",
        },
        "target": "x86_64-unknown-linux-gnu",
        "row_identities": copy.deepcopy(tool.REQUIRED_ROW_IDENTITIES),
    }


def fixture(tool: ModuleType) -> dict[str, Any]:
    rust = {
        row["id"]: "pub struct fathomdb::Engine\npub fn fathomdb::open() -> fathomdb::Engine\n"
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
        "python_exports": '''\
from fathomdb.engine import Engine
from fathomdb.types import SearchHit
__all__ = ["Engine", "SearchHit"]
''',
        "python_registrations": '''\
fn _fathomdb(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add("StorageError", py.get_type::<StorageError>())?;
    Ok(())
}
''',
        "python_stub": '''\
class EngineError(Exception): ...

class StorageError(EngineError): ...

class Engine:
    version: str
    def search(self, query: str, limit: int = ...) -> list[str]: ...

def rerank(query: str, passages: list[str]) -> list[float]: ...
''',
        "napi_declaration": '''\
export interface NativeHit {
  body: string
}
export declare function nativeSearch(query: string): Array<NativeHit>
''',
        "typescript_declarations": {
            "index.d.ts": '''\
export * from "./errors.js";
export { read } from "./read.js";
export interface SearchHit {
  body: string;
}
export declare function search(query: string): Promise<SearchHit[]>;
''',
            "errors.d.ts": '''\
export declare class FathomDbError extends Error {
}
export declare class DatabaseLockedError extends FathomDbError {
}
export interface CorruptionErrorPayload {
  path: string;
}
''',
            "read.d.ts": '''\
export interface ReadView {
  id: string;
}
export declare const read: {
    get(id: string): Promise<ReadView>;
    getMany(ids: string[]): Promise<ReadView[]>;
};
export declare const internalOnly = 1;
''',
        },
        "python_sources": {
            "__init__.py": '''\
from fathomdb import read
from fathomdb.engine import Engine
__all__ = ["Engine", "read"]
''',
            "engine.py": '''\
from typing import Any

class Engine:
    version: str
    def search(self, query: str, limit: int = 10) -> list[str]:
        return [query]
    def _private(self) -> None:
        pass
''',
            "read.py": '''\
__all__ = ["get_many"]

def get_many(ids: list[int]) -> list[str]:
    return []
''',
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
) -> None:
    baseline = tool.capture_from_fixture(base_inputs, metadata(tool))
    changed_inputs = copy.deepcopy(base_inputs)
    mutate(changed_inputs)
    candidate = tool.capture_from_fixture(changed_inputs, metadata(tool))
    result = tool.compare_manifests(baseline, candidate)
    assert not result["equal"], f"mutation for {expected_row} compared equal"
    row = next((item for item in result["row_diffs"] if item["row"] == expected_row), None)
    assert row is not None, f"missing row diff for {expected_row}: {result}"
    assert row[expected_bucket], f"{expected_row} did not report {expected_bucket}: {row}"


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
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value["rust"].__setitem__(
            "rust-facade-default",
            value["rust"]["rust-facade-default"].replace("pub struct fathomdb::Engine\n", ""),
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
                "    def search(self, query: str, limit: int = ...) -> list[str]: ...\n", ""
            ),
        ),
        "python-native-stub",
        "removed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "napi_declaration", value["napi_declaration"].replace("body: string", "body: Uint8Array")
        ),
        "napi-production",
        "changed",
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
        lambda value: value.__setitem__(
            "runtime_exports", ["Engine", "lookup"]
        ),
        "package-entrypoints",
        "changed",
    )
    assert_row_diff(
        tool,
        inputs,
        lambda value: value.__setitem__(
            "package_json", value["package_json"].replace("dist/index.js", "dist/main.js")
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

    with mock.patch.object(tool.subprocess, "run", side_effect=FileNotFoundError("missing")):
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
        with mock.patch.object(tool.os, "replace", side_effect=OSError("replace refused")):
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
                "DatabaseLockedError extends FathomDbError", "DatabaseLockedError extends Error"
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
            ts["errors.d.ts"].replace("export interface CorruptionErrorPayload {\n  path: string;\n}\n", ""),
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
            ts["read.d.ts"].replace("    getMany(ids: string[]): Promise<ReadView[]>;\n", ""),
        ),
        "typescript-declarations",
        "changed",
    )
    ts_entries = next(
        row["entries"] for row in baseline["rows"] if row["id"] == "typescript-declarations"
    )
    ts_paths = {entry["path"] for entry in ts_entries}
    assert {"DatabaseLockedError", "CorruptionErrorPayload", "read"} <= ts_paths, ts_paths
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
            '__all__ = []\n',
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
    moved_py["python_sources"]["engine.py"] = "from fathomdb._engine_impl import Engine\n"
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
    assert {"fathomdb.Engine.search", "fathomdb.read.get_many", "fathomdb.engine.Engine"} <= py_paths, py_paths
    assert not any("_private" in path or path.endswith(".Any") for path in py_paths), py_paths

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
    good_package = json.dumps({"scripts": {"build:native": napi_identity["expanded_command"]}})
    tool._validate_napi_build_script(good_package)
    drifted = json.dumps(
        {"scripts": {"build:native": napi_identity["expanded_command"] + ",default-reranker"}}
    )
    try:
        tool._validate_napi_build_script(drifted)
    except tool.ComparatorError as exc:
        assert "build:native" in str(exc)
    else:
        raise AssertionError("drifted build:native script must fail closed")

    agent_test = (ROOT / "scripts/agent-test.sh").read_text()
    assert "test-slice30-surface-comparator" in agent_test
    assert "scripts/tests/test_slice30_surface_comparator.py" in agent_test

    print("ok    slice30-surface-comparator")


if __name__ == "__main__":
    main()
