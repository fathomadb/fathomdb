"""A minimal local Mem0-OSS façade backed by FathomDB FTS.

The official ``memory-benchmarks`` client needs only ``/health``,
``POST /memories``, ``POST /search``, and ``DELETE /memories``.  This module
implements that narrow seam without importing or reusing Mem0-derived facts.
It is deliberately limited to FathomDB's current public FTS top-10 surface.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
import sys
import threading
import time
from collections.abc import Callable
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Protocol
from urllib.parse import parse_qs, urlparse

from experiments.locomo_provenance import ProvenanceEntry, ProvenanceMap, load_manifest, search_request_fingerprint
from experiments.fathomdb_test_setup import prepare_test_database


MAX_FTS_RESULTS = 10


class _Engine(Protocol):
    def write(self, batch: list[dict[str, str]]) -> Any: ...
    def drain(self, *, timeout_s: int) -> None: ...
    def search_text_only(self, query: str) -> Any: ...
    def close(self) -> None: ...


def render_messages(messages: object) -> str:
    """Render one official Mem0 message chunk deterministically."""
    if not isinstance(messages, list) or not messages:
        raise ValueError("messages must be a non-empty list")
    rendered: list[str] = []
    for message in messages:
        if not isinstance(message, dict):
            raise ValueError("each message must be an object")
        role, content = message.get("role"), message.get("content")
        if not isinstance(role, str) or not role or not isinstance(content, str) or not content.strip():
            raise ValueError("each message needs non-empty string role and content")
        rendered.append(f"{role}: {content}")
    return "\n".join(rendered)


def _user_token(user_id: str) -> str:
    if not isinstance(user_id, str) or not user_id:
        raise ValueError("user_id must be a non-empty string")
    return hashlib.sha256(user_id.encode("utf-8")).hexdigest()[:24]


def _timing_summary(values: list[float]) -> dict[str, float | int | None]:
    if not values:
        return {"n": 0, "p50": None, "p95": None, "p99": None}
    ordered = sorted(values)

    def percentile(q: float) -> float:
        return ordered[max(0, math.ceil(len(ordered) * q) - 1)]

    return {"n": len(ordered), "p50": percentile(0.50), "p95": percentile(0.95), "p99": percentile(0.99)}


class FathomDBOssStore:
    """One FathomDB database per official Mem0 user ID."""

    def __init__(self, root: str | Path, *, engine_factory: Callable[[str], _Engine] | None = None,
                 provenance: ProvenanceMap | None = None,
                 fathomdb_bin: str | None = None) -> None:
        self.root = Path(root).resolve()
        self.root.mkdir(parents=True, exist_ok=True)
        self._prepare_databases = engine_factory is None
        self._fathomdb_bin = fathomdb_bin or str(Path(sys.executable).parent / "fathomdb")
        if engine_factory is None:
            from fathomdb import Engine

            def default_engine_factory(path: str) -> _Engine:
                return Engine.open(path, use_default_embedder=False)

            engine_factory = default_engine_factory
        self._engine_factory = engine_factory
        self._engines: dict[str, _Engine] = {}
        self._chunk_counts: dict[str, int] = {}
        self._provenance = provenance
        self._provenance_by_logical_id: dict[str, ProvenanceEntry] = {}
        self._query_provenance: dict[str, list[dict[str, object]]] = {}
        self._timings: dict[str, list[float]] = {
            "facade_query_ms": [], "engine_query_ms": [], "ingest_ack_ms": [], "ready_to_search_ms": [],
        }
        self._lock = threading.RLock()

    def _path(self, user_id: str) -> Path:
        return self.root / f"user-{_user_token(user_id)}"

    def _engine(self, user_id: str) -> _Engine:
        if user_id not in self._engines:
            path = self._path(user_id)
            if self._prepare_databases:
                prepared = prepare_test_database(
                    self.root,
                    test_id=path.name,
                    embed_device="cpu",
                    rerank_device="cpu",
                    embedder="none",
                    check_reranker=False,
                    fathomdb_bin=self._fathomdb_bin,
                )
                path = prepared.database_path
            self._engines[user_id] = self._engine_factory(str(path))
            self._chunk_counts[user_id] = 0
        return self._engines[user_id]

    def add(self, payload: object) -> dict[str, list]:
        """Implement the Mem0 OSS ``POST /memories`` shape."""
        if not isinstance(payload, dict):
            raise ValueError("request body must be an object")
        user_id = payload.get("user_id")
        _user_token(user_id)
        body = render_messages(payload.get("messages"))
        provenance = self._provenance.resolve(payload) if self._provenance is not None else None
        with self._lock:
            engine = self._engine(user_id)
            chunk = self._chunk_counts[user_id]
            token = _user_token(user_id)
            logical_id = f"mem0-oss:{token}:{chunk}"
            started = time.monotonic()
            engine.write([{
                "kind": "locomo_message_chunk",
                "body": body,
                "source_id": f"mem0-oss:{token}",
                "logical_id": logical_id,
            }])
            acked = time.monotonic()
            engine.drain(timeout_s=30)
            ready = time.monotonic()
            self._chunk_counts[user_id] += 1
            self._timings["ingest_ack_ms"].append((acked - started) * 1000)
            self._timings["ready_to_search_ms"].append((ready - started) * 1000)
            if provenance is not None:
                self._provenance_by_logical_id[logical_id] = provenance
        return {"results": []}

    def search(self, payload: object) -> dict[str, list[dict[str, object]]]:
        """Implement the Mem0 OSS ``POST /search`` shape."""
        if not isinstance(payload, dict):
            raise ValueError("request body must be an object")
        user_id, query, limit = payload.get("user_id"), payload.get("query"), payload.get("limit", MAX_FTS_RESULTS)
        _user_token(user_id)
        if not isinstance(query, str) or not query:
            raise ValueError("query must be a non-empty string")
        if not isinstance(limit, int) or isinstance(limit, bool) or not 1 <= limit <= MAX_FTS_RESULTS:
            raise ValueError(f"limit must be an integer in [1, {MAX_FTS_RESULTS}]")
        facade_started = time.monotonic()
        with self._lock:
            engine = self._engines.get(user_id)
            if engine is None:
                return {"results": []}
            engine_started = time.monotonic()
            hits = engine.search_text_only(query).results[:limit]
            engine_ended = time.monotonic()
            self._timings["engine_query_ms"].append((engine_ended - engine_started) * 1000)
            results: list[dict[str, object]] = []
            for hit in hits:
                result: dict[str, object] = {"memory": hit.body, "score": hit.score, "id": hit.id.value}
                if self._provenance is not None:
                    try:
                        result["evaluation_provenance"] = self._provenance_by_logical_id[hit.id.value].safe_metadata()
                    except KeyError as exc:
                        raise ValueError("search hit has no safe evaluation provenance") from exc
                results.append(result)
            if self._provenance is not None:
                request_fingerprint = search_request_fingerprint(user_id, query)
                safe_results = [result["evaluation_provenance"] for result in results]
                existing = self._query_provenance.get(request_fingerprint)
                if existing is not None and existing != safe_results:
                    raise ValueError("search request provenance changed during one campaign run")
                self._query_provenance[request_fingerprint] = safe_results
        self._timings["facade_query_ms"].append((time.monotonic() - facade_started) * 1000)
        return {"results": results}

    def metrics_snapshot(self) -> dict[str, dict[str, float | int | None]]:
        """Return deterministic aggregate timings without query or corpus text."""
        with self._lock:
            return {name: _timing_summary(values) for name, values in self._timings.items()}

    def provenance_snapshot(self) -> dict[str, object]:
        """Return hashed-query retrieval provenance without text or raw payloads."""
        with self._lock:
            return {
                "schema_version": "locomo-facade-provenance.v1",
                "requests": {key: list(value) for key, value in sorted(self._query_provenance.items())},
            }

    def delete_user(self, user_id: str) -> None:
        """Close and remove the isolated store for one official user ID."""
        with self._lock:
            engine = self._engines.pop(user_id, None)
            self._chunk_counts.pop(user_id, None)
            if engine is not None:
                engine.close()
            path = self._path(user_id)
            if path.is_dir():
                shutil.rmtree(path)
            else:
                for candidate in (path, Path(f"{path}-wal"), Path(f"{path}-shm")):
                    if candidate.exists():
                        candidate.unlink()

    def close(self) -> None:
        """Close every private engine without deleting campaign artifacts."""
        with self._lock:
            for engine in self._engines.values():
                engine.close()
            self._engines.clear()
            self._chunk_counts.clear()


def handler_for(store: FathomDBOssStore) -> type[BaseHTTPRequestHandler]:
    """Create an HTTP handler bound to one campaign store."""

    class Handler(BaseHTTPRequestHandler):
        def _send(self, status: HTTPStatus, body: dict[str, object]) -> None:
            encoded = json.dumps(body).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(encoded)))
            self.end_headers()
            self.wfile.write(encoded)

        def _payload(self) -> object:
            length = int(self.headers.get("Content-Length", "0"))
            return json.loads(self.rfile.read(length))

        def do_GET(self) -> None:  # noqa: N802
            if self.path == "/health":
                self._send(HTTPStatus.OK, {"status": "ok"})
            elif self.path == "/metrics":
                self._send(HTTPStatus.OK, store.metrics_snapshot())
            elif self.path == "/provenance":
                self._send(HTTPStatus.OK, store.provenance_snapshot())
            else:
                self._send(HTTPStatus.NOT_FOUND, {"detail": "not found"})

        def do_POST(self) -> None:  # noqa: N802
            try:
                payload = self._payload()
                if self.path == "/memories":
                    self._send(HTTPStatus.OK, store.add(payload))
                elif self.path == "/search":
                    self._send(HTTPStatus.OK, store.search(payload))
                else:
                    self._send(HTTPStatus.NOT_FOUND, {"detail": "not found"})
            except (ValueError, json.JSONDecodeError) as exc:
                self._send(HTTPStatus.BAD_REQUEST, {"detail": str(exc)})

        def do_DELETE(self) -> None:  # noqa: N802
            parsed = urlparse(self.path)
            if parsed.path != "/memories":
                self._send(HTTPStatus.NOT_FOUND, {"detail": "not found"})
                return
            try:
                user_id = parse_qs(parsed.query).get("user_id", [None])[0]
                _user_token(user_id)
                store.delete_user(user_id)
                self._send(HTTPStatus.OK, {"status": "deleted"})
            except ValueError as exc:
                self._send(HTTPStatus.BAD_REQUEST, {"detail": str(exc)})

        def log_message(self, _format: str, *_args: object) -> None:
            """Do not emit request payloads into stdout/stderr logs."""

    return Handler


def main(argv: list[str] | None = None) -> int:
    """Serve the narrow OSS façade for one external comparator campaign."""
    parser = argparse.ArgumentParser(description="FathomDB Mem0-OSS façade")
    parser.add_argument("--root", type=Path, required=True, help="external campaign database root")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8889)
    parser.add_argument("--fathomdb-bin", required=True)
    parser.add_argument("--provenance-manifest", type=Path, required=True,
                        help="content-free LOCOMO payload-to-ID manifest outside the repository")
    args = parser.parse_args(argv)
    store = FathomDBOssStore(
        args.root,
        provenance=load_manifest(args.provenance_manifest),
        fathomdb_bin=args.fathomdb_bin,
    )
    server = ThreadingHTTPServer((args.host, args.port), handler_for(store))
    try:
        server.serve_forever()
    finally:
        server.server_close()
        store.close()
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
