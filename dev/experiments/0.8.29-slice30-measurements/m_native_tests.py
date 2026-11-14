"""Owned actual CLI/Adapter/SDK boundaries; explicitly selected native source tests."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from m_client import RpcChild
from m_wire import decode_tool


class NativeCase(unittest.TestCase):
    """Create fresh real closed SDK stores; retain every native source-test attempt."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="m-source-", dir=os.environ["TMPDIR"]))
        self.cli = Path(os.environ["FATHOMDB_SLICE30_M_CLI"]).resolve(strict=True)
        self.helper = Path(os.environ["FATHOMDB_SLICE30_M_HELPER"]).resolve(strict=True)
        self.stores = self.root / "stores"
        seeded = subprocess.run([str(self.helper), "--seed-fixture", str(self.stores)],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=15)
        (self.root / "seed.stdout").write_bytes(seeded.stdout)
        (self.root / "seed.stderr").write_bytes(seeded.stderr)
        self.assertEqual(seeded.returncode, 0, seeded.stderr.decode())

    def config(self, result_format="text", **extra):
        """Write only existing provider-free configuration keys and selected literal stores."""
        value = {"stores": {name: {"path": str(self.stores / f"{name}.sqlite")}
                             for name in ("a", "b")}, "default_store": "a",
                 "use_default_embedder": False, "search_mode": "text",
                 "result_format": result_format, **extra}
        path = self.root / f"config-{result_format}.json"
        path.write_text(json.dumps(value))
        return path

    def initialize(self, client):
        """Use the existing protocol handshake and verify unchanged seven-tool discovery."""
        request_id = client.submit("initialize", {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "slice30-m-source-tests", "version": "0"}}, phase="control")
        self.assertIn("result", client.wait(request_id))
        client.notify("notifications/initialized", {})
        request_id = client.submit("tools/list", {}, phase="control")
        tools = client.wait(request_id)["result"]["tools"]
        self.assertEqual(sorted(tool["name"] for tool in tools),
                         ["expand", "feedback", "fetch", "list_stores", "orient", "search", "use_store"])

    def call(self, client, name, arguments):
        """Record one actual wire call and return its decoded outcome."""
        request_id = client.submit("tools/call", {"name": name, "arguments": arguments},
                                   phase="control", tool=name)
        return decode_tool(client.wait(request_id))


class CompactSearchTests(NativeCase):
    """The compact-default normal CLI must not become an empty successful result."""

    def test_actual_compact_cli_retains_literal_hit_and_envelope(self):
        with RpcChild([str(self.cli), "--config", str(self.config())],
                      self.root / "cli") as client:
            self.initialize(client)
            result = self.call(client, "search", {"query": "alpha", "limit": 10})
            payload = result["payload"]
            self.assertEqual([row["ref"] for row in payload.get("results", [])], ["l:alpha"],
                             f"actual compact hit was lost; raw={result['text']!r}; retained={self.root}")
            self.assertTrue(result["ok"])
            self.assertEqual(payload["store"], "a")
            self.assertEqual(payload["epoch"], 1)
            self.assertEqual(payload["mode"], "text")
            self.assertEqual(payload["degradation"], [])
            self.assertEqual(payload["budget"]["max_tokens"], 4000)
            self.assertTrue(payload["response_id"].startswith("r1-"))
        self.assertEqual(client.exit["code"], 0)
        self.assertTrue(client.exit["readers_joined"])
        self.assertEqual(client.failures, [])


class TypedErrorTests(NativeCase):
    """Both actual wire formats retain the typed error and its complete remedy."""

    def test_actual_unknown_store_keeps_error_words_candidates_and_label(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    result = self.call(client, "search", {"store": "aa", "query": "alpha"})
                    error = result["payload"].get("error", {})
                    self.assertEqual(error.get("class"), "invalid_request",
                                     f"actual typed error was lost: {result}; retained={self.root}")
                    self.assertEqual(error["code"], "unknown_store")
                    self.assertEqual(error["candidates"], ["a", "b"])
                    self.assertEqual(error["message"], 'store "aa" is not addressable')
                    self.assertEqual(error["remedy"], "call list_stores and use one of the listed names")
                    self.assertFalse(result["ok"])
                    self.assertEqual(result.get("error"), "invalid_request/unknown_store")
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])


class FetchTests(NativeCase):
    """Actual fetch preserves UTF8 byte frames, duplicate order and item errors."""

    def test_actual_fetch_retains_exact_body_and_each_ordered_outcome(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    result = self.call(client, "fetch", {"refs": ["l:alpha", "l:missing", "l:alpha"]})
                    items = result["payload"].get("items", [])
                    self.assertEqual(len(items), 3,
                                     f"actual fetch frames were lost: {result}; retained={self.root}")
                    self.assertEqual([item["ref"] for item in items],
                                     ["l:alpha", "l:missing", "l:alpha"])
                    self.assertEqual(items[0]["record"],
                                     {"kind": "note", "body": "alpha river crossing\né😀",
                                      "body_cut": False})
                    self.assertEqual(items[2], items[0])
                    self.assertEqual(items[1]["error"]["class"], "not_found")
                    self.assertEqual(items[1]["error"]["code"], "not_found")
                    self.assertIn("remedy", items[1]["error"])
                    self.assertTrue(result["ok"])
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])


class ListStateTests(NativeCase):
    """Lifecycle observations come from actual list_stores without touching engines."""

    def test_actual_list_retains_closed_and_ready_states(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    before = self.call(client, "list_stores", {})["payload"]
                    self.assertEqual(before.get("stores", []), [
                        {"name": "a", "open": False, "readiness": "closed", "default": True},
                        {"name": "b", "open": False, "readiness": "closed", "default": False}],
                        f"actual initial listing was lost; retained={self.root}")
                    self.call(client, "search", {"store": "a", "query": "alpha"})
                    after = self.call(client, "list_stores", {})["payload"]
                    self.assertEqual(after["stores"], [
                        {"name": "a", "open": True, "readiness": "ready", "default": True},
                        {"name": "b", "open": False, "readiness": "closed", "default": False}])
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])


class OrientTests(NativeCase):
    """Actual orient readiness is retained without inventing provider availability."""

    def test_actual_orient_retains_provider_free_runtime_and_embedding_facts(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    result = self.call(client, "orient", {})
                    payload = result["payload"]
                    status = payload.get("projection_status", {})
                    self.assertEqual(status.get("runtime_embedder_available"), False,
                                     f"actual orient readiness was lost: {result}; retained={self.root}")
                    self.assertEqual(status["runtime_unavailability_reason"], "no_runtime")
                    self.assertEqual(status["vector_unsupported_kinds"], [])
                    self.assertEqual(payload["embedding_readiness"]["usable_embedder"], False)
                    self.assertEqual(payload["embedding_readiness"]["pending_count"], 0)
                    self.assertEqual(payload["store"], "a")
                    self.assertEqual(payload["epoch"], 1)
                    self.assertEqual(payload["budget"]["max_tokens"], 4000)
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])


if __name__ == "__main__":
    unittest.main()
