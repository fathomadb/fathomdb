"""Contract tests for the fixed Python SDK benchmark driver."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

from experiments.fathomdb_test_setup import prepare_test_database
from experiments.sdk_drivers import python_driver


def _fake_sdk(tmp_path: Path):
    native_file = tmp_path / "_fathomdb.so"
    native_file.write_bytes(b"native fixture")
    package_file = tmp_path / "__init__.py"
    package_file.write_text("# fixture\n")
    calls: list[object] = []

    class Engine:
        @classmethod
        def open(cls, path):
            calls.append(("open", path))
            return cls()

        def search_text_only(self, query, view, *, limit):
            calls.append(("text", query, view.valid_as_of, limit))
            return SimpleNamespace(
                results=[
                    SimpleNamespace(
                        id=SimpleNamespace(space="logical", value="node-a")
                    ),
                    SimpleNamespace(
                        id=SimpleNamespace(space="content", value="digest-b")
                    ),
                ]
            )

        def search(self, *args, **kwargs):
            raise AssertionError("hybrid search must not be used")

        def close(self):
            calls.append("close")

    sdk = SimpleNamespace(
        __file__=str(package_file),
        __version__="0.8.27",
        _fathomdb=SimpleNamespace(__file__=str(native_file)),
        Engine=Engine,
        ReadView=lambda **kwargs: SimpleNamespace(**kwargs),
        admin=SimpleNamespace(
            configure_runtime=lambda **kwargs: calls.append(("runtime", kwargs))
        ),
        errors=SimpleNamespace(EngineError=type("EngineError", (Exception,), {})),
    )
    return sdk, calls, hashlib.sha256(native_file.read_bytes()).hexdigest()


def _request(tmp_path: Path, native_sha: str) -> dict[str, object]:
    return {
        "schema_version": "sdk.driver.v1",
        "mode": "run-repetition",
        "sdk": "python",
        "operation_id": "engine.search_text_only",
        "operation_version": 1,
        "database_path": str(tmp_path / "prepared.sqlite"),
        "queries": [{"id": "q1", "text": "needle"}],
        "arguments": {"limit": 2, "view": {"valid_as_of": 1700000000}},
        "warmup_per_query": 1,
        "timeout_ms": 1001,
        "expected_native_sha256": native_sha,
    }


def test_fts_uses_public_method_and_normalizes_after_timing(tmp_path, monkeypatch):
    sdk, calls, native_sha = _fake_sdk(tmp_path)
    ticks = iter([10, 19])
    monkeypatch.setattr(python_driver.time, "perf_counter_ns", lambda: next(ticks))

    result = python_driver.execute(_request(tmp_path, native_sha), sdk=sdk)

    assert result["status"] == "complete"
    assert result["samples"] == [
        {"query_id": "q1", "elapsed_ns": 9, "ids": ["l:node-a", "h:digest-b"]}
    ]
    assert result["identity"]["native_sha256"] == native_sha
    if sys.platform.startswith("linux"):
        assert result["resources"]["driver_peak_rss_bytes"] > 0
    assert [item[0] for item in calls if isinstance(item, tuple)] == [
        "runtime",
        "open",
        "text",
        "text",
    ]
    assert calls[-1] == "close"
    assert calls[2] == ("text", "needle", 1700000000, 2)


@pytest.mark.skipif(not sys.platform.startswith("linux"), reason="Linux procfs")
def test_peak_rss_uses_child_process_not_inherited_parent_high_watermark():
    ballast = bytearray(128 * 1024 * 1024)
    for offset in range(0, len(ballast), 4096):
        ballast[offset] = 1
    code = """
from experiments.sdk_drivers.python_driver import _peak_rss_bytes
from pathlib import Path
actual = _peak_rss_bytes()
reported = int(next(line for line in Path('/proc/self/status').read_text().splitlines()
                    if line.startswith('VmHWM:')).split()[1]) * 1024
print(actual, reported)
"""
    child = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True, check=True)
    actual, reported = map(int, child.stdout.split())
    assert abs(actual - reported) < 1024 * 1024
    assert actual < len(ballast)


def test_wrong_extension_hash_blocks_before_open(tmp_path):
    sdk, calls, _ = _fake_sdk(tmp_path)
    result = python_driver.execute(_request(tmp_path, "0" * 64), sdk=sdk)
    assert result["status"] == "blocked"
    assert result["samples"] == []
    assert result["error"]["kind"] == "native_identity_mismatch"
    assert calls == []


def test_public_error_blocks_without_samples_and_closes(tmp_path):
    sdk, calls, native_sha = _fake_sdk(tmp_path)

    def fail(*args, **kwargs):
        raise sdk.errors.EngineError("secret query payload")

    sdk.Engine.search_text_only = fail
    result = python_driver.execute(_request(tmp_path, native_sha), sdk=sdk)
    assert result["status"] == "blocked"
    assert result["samples"] == []
    assert result["error"]["kind"] == "EngineError"
    assert "secret query payload" not in json.dumps(result)
    assert calls[-1] == "close"


def test_public_error_preserves_class_and_typed_kind_payload(tmp_path):
    sdk, _, native_sha = _fake_sdk(tmp_path)
    storage_error = type("StorageError", (sdk.errors.EngineError,), {})

    def fail(*args, **kwargs):
        error = storage_error()
        error.kind = "corruption"
        raise error

    sdk.Engine.search_text_only = fail
    result = python_driver.execute(_request(tmp_path, native_sha), sdk=sdk)
    assert result["error"] == {
        "kind": "StorageError",
        "fields": {"kind": "corruption"},
    }


def test_timeout_1001_roundtrips_to_exact_native_milliseconds():
    seconds = python_driver.exact_timeout_seconds(1001)
    assert int(seconds * 1000) == 1001


def test_huge_timeout_refuses_as_invalid_deadline():
    import pytest

    with pytest.raises(ValueError):
        python_driver.exact_timeout_seconds(10**1000)


def test_close_failure_blocks_without_latency_samples(tmp_path):
    sdk, _, native_sha = _fake_sdk(tmp_path)

    def fail_close(self):
        raise RuntimeError("private close detail")

    sdk.Engine.close = fail_close
    result = python_driver.execute(_request(tmp_path, native_sha), sdk=sdk)
    assert result["status"] == "blocked"
    assert result["samples"] == []
    assert result["error"]["kind"] == "close_failure"
    assert "private close detail" not in json.dumps(result)


def test_cli_emits_one_json_response_for_bad_request(tmp_path):
    request_file = tmp_path / "request.json"
    request_file.write_text("{broken", encoding="utf-8")
    completed = subprocess.run(
        [
            sys.executable,
            "-m",
            "experiments.sdk_drivers.python_driver",
            str(request_file),
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    assert len(completed.stdout.splitlines()) == 1
    assert json.loads(completed.stdout)["status"] == "blocked"


def test_capabilities_advertise_only_implemented_operations():
    result = python_driver.execute(
        {"schema_version": "sdk.driver.v1", "mode": "capabilities"}
    )
    assert result["status"] == "complete"
    assert {entry["operation_id"] for entry in result["operations"]} == {
        "engine.search_text_only",
        "read.get",
        "graph.neighbors",
    }


def test_read_get_calls_public_binding_with_fixed_view(tmp_path, monkeypatch):
    sdk, calls, native_sha = _fake_sdk(tmp_path)
    request = _request(tmp_path, native_sha)
    request["operation_id"] = "read.get"
    request["arguments"] = {"view": {"valid_as_of": 1700000000}}
    request["warmup_per_query"] = 0
    sdk.read = SimpleNamespace(
        get=lambda engine, logical_id, *, view: (
            calls.append(("read.get", logical_id, view.valid_as_of))
            or SimpleNamespace(logical_id="node-a")
        )
    )
    ticks = iter([10, 21])
    monkeypatch.setattr(python_driver.time, "perf_counter_ns", lambda: next(ticks))

    result = python_driver.execute(request, sdk=sdk)

    assert result["status"] == "complete"
    assert result["samples"] == [
        {"query_id": "q1", "elapsed_ns": 11, "ids": ["l:node-a"]}
    ]
    assert ("read.get", "needle", 1700000000) in calls
    assert calls[-1] == "close"


def test_graph_neighbors_calls_public_binding_with_bound(tmp_path, monkeypatch):
    sdk, calls, native_sha = _fake_sdk(tmp_path)
    request = _request(tmp_path, native_sha)
    request["operation_id"] = "graph.neighbors"
    request["arguments"] = {"depth": 1, "view": {"valid_as_of": 1700000000}}
    request["warmup_per_query"] = 0
    sdk.graph = SimpleNamespace(
        neighbors=lambda engine, logical_id, depth, *, view: (
            calls.append(("graph.neighbors", logical_id, depth, view.valid_as_of))
            or [SimpleNamespace(logical_id="node-b")]
        )
    )
    ticks = iter([100, 123])
    monkeypatch.setattr(python_driver.time, "perf_counter_ns", lambda: next(ticks))

    result = python_driver.execute(request, sdk=sdk)

    assert result["status"] == "complete"
    assert result["samples"] == [
        {"query_id": "q1", "elapsed_ns": 23, "ids": ["l:node-b"]}
    ]
    assert ("graph.neighbors", "needle", 1, 1700000000) in calls
    assert calls[-1] == "close"


@pytest.mark.parametrize(
    ("operation_id", "arguments", "expected_ids"),
    [
        ("read.get", {"view": {"valid_as_of": 1700000000}}, ["l:root"]),
        (
            "graph.neighbors",
            {"depth": 1, "view": {"valid_as_of": 1700000000}},
            ["l:child"],
        ),
    ],
)
def test_real_fresh_database_read_and_graph(
    tmp_path, operation_id, arguments, expected_ids
):
    sdk = pytest.importorskip("fathomdb")
    fathomdb_bin = shutil.which("fathomdb") or str(
        Path(sys.executable).with_name("fathomdb")
    )
    if not Path(fathomdb_bin).is_file():
        pytest.skip("FathomDB CLI is unavailable for real doctor evidence")
    prepared = prepare_test_database(
        tmp_path,
        test_id=operation_id.replace(".", "-"),
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=fathomdb_bin,
    )
    engine = sdk.Engine.open(str(prepared.database_path))
    try:
        engine.write(
            [
                {
                    "kind": "doc",
                    "body": "root",
                    "logical_id": "root",
                    "source_id": "sdk-test",
                },
                {
                    "kind": "doc",
                    "body": "child",
                    "logical_id": "child",
                    "source_id": "sdk-test",
                },
                {
                    "edge": {
                        "kind": "link",
                        "from": "root",
                        "to": "child",
                        "logical_id": "edge-root-child",
                        "source_id": "sdk-test",
                    }
                },
            ]
        )
        engine.drain(timeout_s=30)
    finally:
        engine.close()
    native_sha = hashlib.sha256(Path(sdk._fathomdb.__file__).read_bytes()).hexdigest()
    request = _request(tmp_path, native_sha)
    request.update(
        operation_id=operation_id,
        database_path=str(prepared.database_path),
        queries=[{"id": "q1", "text": "root"}],
        arguments=arguments,
        warmup_per_query=0,
    )

    result = python_driver.execute(request)

    assert result["status"] == "complete", result["error"]
    assert result["samples"][0]["ids"] == expected_ids
