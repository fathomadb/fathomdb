"""Protocol and public-call tests for the TypeScript SDK repetition driver."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest


def _node_25() -> Path | None:
    candidate = os.environ.get("FATHOMDB_BENCH_NODE") or shutil.which("node")
    if candidate is None:
        return None
    node = Path(candidate).resolve()
    version = subprocess.run(
        [str(node), "--version"], capture_output=True, text=True, check=False
    )
    return (
        node if version.returncode == 0 and version.stdout.startswith("v25.") else None
    )


NODE = _node_25()
pytestmark = pytest.mark.skipif(NODE is None, reason="Node 25 is required")
DRIVER = (
    Path(__file__).resolve().parents[2]
    / "experiments/sdk_drivers/typescript_driver.mjs"
)


@pytest.fixture
def sandbox(tmp_path: Path) -> tuple[Path, str]:
    driver = tmp_path / "experiments/sdk_drivers/typescript_driver.mjs"
    driver.parent.mkdir(parents=True)
    shutil.copyfile(DRIVER, driver)
    package = tmp_path / "src/ts"
    (package / "dist").mkdir(parents=True)
    (package / "package.json").write_text(
        '{"name":"fathomdb","version":"0.8.27","type":"module"}'
    )
    native = package / "fathomdb.linux-x64-gnu.node"
    native.write_bytes(b"selected native fixture")
    (package / "dist/index.js").write_text(
        """
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
require.extensions['.node'] = (module) => { module.exports = {}; };
require('../fathomdb.linux-x64-gnu.node');
export const admin = { configureRuntime: (options) => {
  if (options.sqliteMode !== 'performance') throw Error('wrong runtime');
}};
export const read = { async get(engine, logicalId, view) {
  if (!(engine instanceof Engine) || view.validAsOf !== 1700000000)
    throw Error('wrong read view');
  await new Promise((resolve) => setTimeout(resolve, 25));
  return logicalId === 'missing' ? null : { logicalId };
}};
export const graph = { async neighbors(engine, logicalId, depth, direction, view) {
  if (!(engine instanceof Engine) || depth !== 1 || direction !== 'both' ||
      view.validAsOf !== 1700000000) throw Error('wrong graph options');
  await new Promise((resolve) => setTimeout(resolve, 25));
  return [{ logicalId: logicalId + '-child' }];
}};
export class Engine {
  static async open(path, options) {
    if (options.useDefaultEmbedder !== false) throw Error('wrong open options');
    const engine = new Engine();
    engine.closeFails = path.endsWith('close-fail.sqlite');
    return engine;
  }
  openReport() { return { queryBackend: 'sqlite', denseDisabled: false }; }
  async searchTextOnly(query, options) {
    if ('view' in options || options.validAsOf !== 1700000000 || options.limit !== 3)
      throw Error('wrong flat search options');
    if (query === 'reject') throw new TypeError('query rejected');
    await new Promise((resolve) => setTimeout(resolve, query === 'slow' ? 100 : 25));
    return { results: [{ id: { space: 'logical', value: 'alpha' } }] };
  }
  async search() { throw Error('general search called'); }
  async close() { if (this.closeFails) throw new RangeError('close rejected'); }
}
"""
    )
    return driver, hashlib.sha256(native.read_bytes()).hexdigest()


def invoke(tmp_path: Path, driver: Path, payload: dict) -> dict:
    request = tmp_path / "request.json"
    request.write_text(json.dumps(payload))
    result = subprocess.run(
        [str(NODE), str(driver), str(request)],
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert result.stderr == ""
    assert len(result.stdout.splitlines()) == 1
    return json.loads(result.stdout)


def request(native_sha256: str, *, text: str = "find alpha") -> dict:
    return {
        "schema_version": "sdk.driver.v1",
        "mode": "run-repetition",
        "sdk": "typescript",
        "operation_id": "engine.search_text_only",
        "operation_version": 1,
        "database_path": "/tmp/prepared.sqlite",
        "queries": [{"id": "q1", "text": text}],
        "arguments": {"limit": 3, "view": {"valid_as_of": 1700000000}},
        "warmup_per_query": 1,
        "timeout_ms": 2000,
        "expected_native_sha256": native_sha256,
    }


def test_awaited_public_fts_and_loaded_binary_attestation(sandbox, tmp_path):
    driver, digest = sandbox
    response = invoke(tmp_path, driver, request(digest))
    assert response["status"] == "complete"
    assert response["identity"]["native_sha256"] == digest
    assert response["effective_options"] == {
        "limit": 3,
        "view": {"valid_as_of": 1700000000},
    }
    assert response["samples"] == [
        {
            "query_id": "q1",
            "elapsed_ns": pytest.approx(25_000_000, abs=50_000_000),
            "ids": ["l:alpha"],
        }
    ]
    assert response["samples"][0]["elapsed_ns"] >= 20_000_000
    assert response["error"] is None


def test_digest_mismatch_blocks_before_query(sandbox, tmp_path):
    driver, _ = sandbox
    response = invoke(tmp_path, driver, request("0" * 64))
    assert response["status"] == "blocked"
    assert response["samples"] == []
    assert response["error"]["code"] == "native_digest_mismatch"


def test_rejected_promise_blocks_without_success_sample(sandbox, tmp_path):
    driver, digest = sandbox
    response = invoke(tmp_path, driver, request(digest, text="reject"))
    assert response["status"] == "blocked"
    assert response["samples"] == []
    assert response["error"]["code"] == "TypeError"


def test_missing_fixed_validity_blocks_before_import(sandbox, tmp_path):
    driver, digest = sandbox
    payload = request(digest)
    payload["arguments"]["view"] = {}
    response = invoke(tmp_path, driver, payload)
    assert response["status"] == "blocked"
    assert response["samples"] == []
    assert response["error"]["code"] == "invalid_request"


def test_timeout_blocks_without_success_sample(sandbox, tmp_path):
    driver, digest = sandbox
    payload = request(digest, text="slow")
    payload["warmup_per_query"] = 0
    payload["timeout_ms"] = 5
    response = invoke(tmp_path, driver, payload)
    assert response["status"] == "blocked"
    assert response["samples"] == []
    assert response["error"]["code"] == "timeout"


def test_close_failure_blocks_without_success_sample(sandbox, tmp_path):
    driver, digest = sandbox
    payload = request(digest)
    payload["database_path"] = "/tmp/close-fail.sqlite"
    response = invoke(tmp_path, driver, payload)
    assert response["status"] == "blocked"
    assert response["samples"] == []
    assert response["error"]["code"] == "close_failed"


def test_capabilities_is_one_protocol_response(sandbox, tmp_path):
    driver, _ = sandbox
    response = invoke(
        tmp_path, driver, {"schema_version": "sdk.driver.v1", "mode": "capabilities"}
    )
    assert response["status"] == "complete"
    assert response["operations"] == [
        {"operation_id": "engine.search_text_only", "operation_version": 1},
        {"operation_id": "read.get", "operation_version": 1},
        {"operation_id": "graph.neighbors", "operation_version": 1},
    ]


def test_read_get_awaits_public_method_and_normalizes_missing(sandbox, tmp_path):
    driver, digest = sandbox
    payload = request(digest, text="alpha")
    payload["operation_id"] = "read.get"
    payload["queries"].append({"id": "q2", "text": "missing"})
    payload["arguments"] = {"view": {"valid_as_of": 1700000000}}
    payload["warmup_per_query"] = 0
    response = invoke(tmp_path, driver, payload)
    assert response["status"] == "complete"
    assert response["effective_options"] == payload["arguments"]
    assert [(sample["query_id"], sample["ids"]) for sample in response["samples"]] == [
        ("q1", ["l:alpha"]),
        ("q2", []),
    ]
    assert all(sample["elapsed_ns"] >= 20_000_000 for sample in response["samples"])


def test_graph_neighbors_awaits_public_method_with_fixed_view(sandbox, tmp_path):
    driver, digest = sandbox
    payload = request(digest, text="root")
    payload["operation_id"] = "graph.neighbors"
    payload["arguments"] = {"depth": 1, "view": {"valid_as_of": 1700000000}}
    payload["warmup_per_query"] = 0
    response = invoke(tmp_path, driver, payload)
    assert response["status"] == "complete"
    assert response["effective_options"] == payload["arguments"]
    assert response["samples"][0]["ids"] == ["l:root-child"]
    assert response["samples"][0]["elapsed_ns"] >= 20_000_000


def test_graph_depth_out_of_range_blocks_before_sdk_call(sandbox, tmp_path):
    driver, digest = sandbox
    payload = request(digest)
    payload["operation_id"] = "graph.neighbors"
    payload["arguments"] = {"depth": 4, "view": {"valid_as_of": 1700000000}}
    response = invoke(tmp_path, driver, payload)
    assert response["status"] == "blocked"
    assert response["samples"] == []
    assert response["error"]["code"] == "invalid_request"


def test_real_public_sdk_fts_read_and_graph_fixture(tmp_path):
    root = DRIVER.parents[2]
    native = root / "src/ts/fathomdb.linux-x64-gnu.node"
    package_entry = root / "src/ts/dist/index.js"
    if not native.is_file() or not package_entry.is_file():
        pytest.skip("the checkout has no local TypeScript SDK native build")
    database = tmp_path / "real-sdk-fixture.sqlite"
    setup = """
const { Engine, admin } = await import(process.argv[2]);
admin.configureRuntime({ sqliteMode: 'performance' });
const engine = await Engine.open(process.argv[1], { useDefaultEmbedder: false });
try {
  await engine.write([
    { kind: 'doc', body: 'benchmarkalpha lexical fixture', logicalId: 'alpha', sourceId: 'sdk-driver-test' },
    { kind: 'doc', body: 'benchmarkbeta graph fixture', logicalId: 'beta', sourceId: 'sdk-driver-test' },
    { edge: { kind: 'link', from: 'alpha', to: 'beta', logicalId: 'edge-ab', sourceId: 'sdk-driver-test' } },
  ]);
  await engine.drain(30000);
} finally { await engine.close(); }
"""
    prepared = subprocess.run(
        [
            str(NODE),
            "--input-type=module",
            "-e",
            setup,
            str(database),
            package_entry.as_uri(),
        ],
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    assert prepared.returncode == 0, prepared.stderr
    digest = hashlib.sha256(native.read_bytes()).hexdigest()
    expected = {
        "engine.search_text_only": (
            "benchmarkalpha",
            {"limit": 3, "view": {"valid_as_of": 1700000000}},
            ["l:alpha"],
        ),
        "read.get": ("alpha", {"view": {"valid_as_of": 1700000000}}, ["l:alpha"]),
        "graph.neighbors": (
            "alpha",
            {"depth": 1, "view": {"valid_as_of": 1700000000}},
            ["l:beta"],
        ),
    }
    for operation_id, (query, arguments, ids) in expected.items():
        payload = request(digest, text=query)
        payload["operation_id"] = operation_id
        payload["arguments"] = arguments
        payload["database_path"] = str(database)
        payload["warmup_per_query"] = 0
        response = invoke(tmp_path, DRIVER, payload)
        assert response["status"] == "complete", (operation_id, response)
        assert response["samples"][0]["ids"] == ids
