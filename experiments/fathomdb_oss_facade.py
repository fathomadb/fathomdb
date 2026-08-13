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
import shutil
import threading
from collections.abc import Callable
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Protocol
from urllib.parse import parse_qs, urlparse


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


class FathomDBOssStore:
    """One FathomDB database per official Mem0 user ID."""

    def __init__(self, root: str | Path, *, engine_factory: Callable[[str], _Engine] | None = None) -> None:
        self.root = Path(root).resolve()
        self.root.mkdir(parents=True, exist_ok=True)
        if engine_factory is None:
            from fathomdb import Engine

            def default_engine_factory(path: str) -> _Engine:
                return Engine.open(path, use_default_embedder=False)

            engine_factory = default_engine_factory
        self._engine_factory = engine_factory
        self._engines: dict[str, _Engine] = {}
        self._chunk_counts: dict[str, int] = {}
        self._lock = threading.RLock()

    def _path(self, user_id: str) -> Path:
        return self.root / _user_token(user_id)

    def _engine(self, user_id: str) -> _Engine:
        if user_id not in self._engines:
            self._engines[user_id] = self._engine_factory(str(self._path(user_id)))
            self._chunk_counts[user_id] = 0
        return self._engines[user_id]

    def add(self, payload: object) -> dict[str, list]:
        """Implement the Mem0 OSS ``POST /memories`` shape."""
        if not isinstance(payload, dict):
            raise ValueError("request body must be an object")
        user_id = payload.get("user_id")
        _user_token(user_id)
        body = render_messages(payload.get("messages"))
        with self._lock:
            engine = self._engine(user_id)
            chunk = self._chunk_counts[user_id]
            token = _user_token(user_id)
            engine.write([{
                "kind": "locomo_message_chunk",
                "body": body,
                "source_id": f"mem0-oss:{token}",
                "logical_id": f"mem0-oss:{token}:{chunk}",
            }])
            engine.drain(timeout_s=30)
            self._chunk_counts[user_id] += 1
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
        with self._lock:
            engine = self._engines.get(user_id)
            if engine is None:
                return {"results": []}
            hits = engine.search_text_only(query).results[:limit]
        return {"results": [
            {"memory": hit.body, "score": hit.score, "id": hit.id.value}
            for hit in hits
        ]}

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
    args = parser.parse_args(argv)
    store = FathomDBOssStore(args.root)
    server = ThreadingHTTPServer((args.host, args.port), handler_for(store))
    try:
        server.serve_forever()
    finally:
        server.server_close()
        store.close()
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
