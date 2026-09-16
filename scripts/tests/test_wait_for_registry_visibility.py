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
        if prefix == "rate":
            self.send_response(429)
            self.end_headers()
            return
        if prefix in {"bad404", "html404", "mismatch404"}:
            self.send_response(404)
            self.send_header(
                "Content-Type",
                "text/html" if prefix == "html404" else "application/json",
            )
            self.end_headers()
            if prefix == "bad404":
                self.wfile.write(b"{")
            elif prefix == "html404":
                self.wfile.write(b"<html>proxy failure</html>")
            else:
                self.wfile.write(json.dumps({"error": "unrelated"}).encode("utf-8"))
            return
        if prefix == "missing" or (
            prefix == "eventual" and Handler.counts[prefix] == 1
        ) or (
            prefix == "delayed" and Handler.counts[prefix] <= 3
        ):
            self.send_response(404)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            body: object = (
                {"message": "Not Found"}
                if "/pypi/" in self.path
                else "version not found: 0.8.26"
            )
            self.wfile.write(json.dumps(body).encode("utf-8"))
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        if prefix == "malformed":
            self.wfile.write(b"{")
            return
        package = "wrong" if prefix == "mismatch" else "fathomdb"
        version = "0.8.25" if prefix == "wrongversion" else "0.8.26"
        body = (
            {"info": {"name": package, "version": version}}
            if "/pypi/" in self.path
            else {"name": package, "version": version}
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


def invoke_release(root: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "wait-release",
            "--repo-root", str(ROOT),
            "--version", "0.8.26",
            "--pypi-base-url", f"{root}/delayed",
            "--npm-base-url", f"{root}/missing",
            "--timeout-seconds", "0.08",
            "--interval-seconds", "0.01",
        ],
        text=True,
        capture_output=True,
        check=False,
    )


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
            assert "attempts=1" in present.stdout and "timeout_seconds=0.15" in present.stdout
            Handler.counts["eventual"] = 0
            eventual = invoke(f"{root}/eventual", registry)
            assert eventual.returncode == 0, eventual.stderr
            assert Handler.counts["eventual"] == 2
            assert "attempts=2" in eventual.stdout
        missing = invoke(f"{root}/missing")
        assert missing.returncode == 1 and "exact_version_unavailable" in missing.stderr
        assert "registry=pypi" in missing.stderr
        assert "package=fathomdb" in missing.stderr
        assert "version=0.8.26" in missing.stderr
        assert "attempts=" in missing.stderr and "timeout_seconds=0.15" in missing.stderr
        elapsed = float(missing.stderr.split("elapsed_seconds=", 1)[1].split()[0])
        assert elapsed <= 0.25, missing.stderr
        for prefix, reason in (
            ("auth", "authentication"),
            ("rate", "rate_limit"),
            ("server", "http_503"),
            ("malformed", "malformed_metadata"),
            ("mismatch", "package_mismatch"),
            ("wrongversion", "version_mismatch"),
        ):
            Handler.counts[prefix] = 0
            result = invoke(f"{root}/{prefix}")
            assert result.returncode == 1 and reason in result.stderr, result.stderr
            assert Handler.counts[prefix] == 1
        for registry in ("pypi", "npm"):
            for prefix, reason in (
                ("bad404", "malformed_metadata"),
                ("html404", "malformed_metadata"),
                ("mismatch404", "absence_mismatch"),
            ):
                Handler.counts[prefix] = 0
                result = invoke(f"{root}/{prefix}", registry)
                assert result.returncode == 1 and reason in result.stderr, result.stderr
                assert Handler.counts[prefix] == 1
        Handler.counts["delayed"] = 0
        shared_deadline = invoke_release(root)
        assert shared_deadline.returncode == 1, shared_deadline.stderr
        assert Handler.counts["delayed"] == 4
        remaining_bound = float(
            shared_deadline.stderr.split("timeout_seconds=", 1)[1].split()[0]
        )
        assert 0 < remaining_bound < 0.06, shared_deadline.stderr
    finally:
        server.shutdown()
        thread.join()
        server.server_close()
    transport = invoke(root)
    assert transport.returncode == 1 and "transport_error" in transport.stderr

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
