#!/usr/bin/env python3
"""Hermetic contract for exact PyPI/npm visibility polling."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/release/wait-for-registry-version.py"


class Handler(BaseHTTPRequestHandler):
    counts: dict[str, int] = {}

    def do_GET(self) -> None:  # noqa: N802
        prefix = self.path.split("/", 2)[1]
        Handler.counts[prefix] = Handler.counts.get(prefix, 0) + 1
        if prefix == "server":
            self.send_response(503)
            self.end_headers()
            return
        if prefix == "auth":
            self.send_response(401)
            self.end_headers()
            return
        if prefix == "missing" or (
            prefix == "eventual" and Handler.counts[prefix] == 1
        ):
            self.send_response(404)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        if prefix == "malformed":
            self.wfile.write(b"{")
            return
        package = "wrong" if prefix == "mismatch" else "fathomdb"
        body = (
            {"info": {"name": package, "version": "0.8.26"}}
            if "/pypi/" in self.path
            else {"name": package, "version": "0.8.26"}
        )
        self.wfile.write(json.dumps(body).encode("utf-8"))

    def log_message(self, *_args: object) -> None:
        return


def invoke(base: str, registry: str = "pypi") -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "wait",
            "--registry", registry,
            "--package", "fathomdb",
            "--version", "0.8.26",
            "--base-url", base,
            "--timeout-seconds", "0.15",
            "--interval-seconds", "0.01",
        ],
        text=True,
        capture_output=True,
        check=False,
    )


def load_module():
    spec = importlib.util.spec_from_file_location("wait_for_registry_version", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> None:
    assert SCRIPT.is_file(), f"missing visibility helper: {SCRIPT}"
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    root = f"http://127.0.0.1:{server.server_port}"
    try:
        for registry in ("pypi", "npm"):
            present = invoke(f"{root}/present", registry)
            assert present.returncode == 0, present.stderr
            Handler.counts["eventual"] = 0
            eventual = invoke(f"{root}/eventual", registry)
            assert eventual.returncode == 0, eventual.stderr
            assert Handler.counts["eventual"] == 2
        missing = invoke(f"{root}/missing")
        assert missing.returncode == 1 and "exact_version_unavailable" in missing.stderr
        for prefix, reason in (
            ("auth", "authentication"),
            ("server", "http_503"),
            ("malformed", "malformed_metadata"),
            ("mismatch", "package_mismatch"),
        ):
            Handler.counts[prefix] = 0
            result = invoke(f"{root}/{prefix}")
            assert result.returncode == 1 and reason in result.stderr, result.stderr
            assert Handler.counts[prefix] == 1
    finally:
        server.shutdown()
        thread.join()

    module = load_module()
    packages = module.release_packages(ROOT, "0.8.26", include_npm=True)
    assert [(item.registry, item.name) for item in packages[:2]] == [
        ("pypi", "fathomdb"),
        ("npm", "fathomdb"),
    ]
    expected_npm = {
        json.loads(path.read_text(encoding="utf-8"))["name"]
        for path in (ROOT / "src/ts/npm").glob("*/package.json")
    } | {"fathomdb"}
    assert {item.name for item in packages if item.registry == "npm"} == expected_npm
    assert module.release_packages(ROOT, "0.8.26", include_npm=False) == (
        module.RegistryPackage("pypi", "fathomdb", "0.8.26"),
    )
    print("PASS test-wait-for-registry-visibility")


if __name__ == "__main__":
    main()
