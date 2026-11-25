#!/usr/bin/env python3
"""Hermetic contract for exact PyPI/npm visibility polling."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/release/wait-for-registry-version.py"
RELEASE_VERSION = json.loads((ROOT / "src/ts/package.json").read_text(encoding="utf-8"))["version"]


WHEEL_TAGS = (
    "cp310-abi3-manylinux_2_28_x86_64.whl",
    "cp310-abi3-manylinux_2_28_aarch64.whl",
    "cp310-abi3-macosx_11_0_x86_64.whl",
    "cp310-abi3-macosx_11_0_arm64.whl",
    "cp310-abi3-win_amd64.whl",
)


class Handler(BaseHTTPRequestHandler):
    counts: dict[str, int] = {}
    simple_counts: dict[str, int] = {}
    simple_accepts: list[str] = []

    def do_GET(self) -> None:  # noqa: N802
        prefix = self.path.split("/", 2)[1]
        is_simple = "/simple/" in self.path
        pypi = "/pypi/" in self.path or is_simple
        if is_simple:
            Handler.simple_counts[prefix] = Handler.simple_counts.get(prefix, 0) + 1
            Handler.simple_accepts.append(self.headers.get("Accept", ""))
            if prefix == "simple_auth":
                self.send_response(401)
                self.end_headers()
                return
            if prefix == "simple_server":
                self.send_response(503)
                self.end_headers()
                return
            if prefix == "simple_malformed":
                self.send_response(200)
                self.send_header("Content-Type", "application/vnd.pypi.simple.v1+json")
                self.end_headers()
                self.wfile.write(b"{")
                return
            self.send_response(200)
            self.send_header("Content-Type", "application/vnd.pypi.simple.v1+json")
            self.end_headers()
            simple_pkg = self.path.rstrip("/").split("/")[-1]
            dist_prefix = simple_pkg.replace("-", "_")
            ver = "0.8.31rc1" if prefix == "prerelease" else (
                RELEASE_VERSION if prefix == "delayed" else "0.8.26"
            )
            tags = WHEEL_TAGS
            if prefix == "wheel_missing" or (
                prefix == "wheel_lag" and Handler.simple_counts[prefix] == 1
            ):
                tags = WHEEL_TAGS[:-1]
            files = [{"filename": f"{dist_prefix}-{ver}-{tag}"} for tag in tags]
            body = {
                "meta": {"api-version": "1.1"},
                "name": simple_pkg,
                "files": files,
            }
            self.wfile.write(json.dumps(body).encode("utf-8"))
            return
        parts = self.path.rstrip("/").split("/")
        requested_version = parts[-2 if pypi else -1]
        requested_package = parts[-3 if pypi else -2]
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
                if pypi
                else f"version not found: {requested_version}"
            )
            self.wfile.write(json.dumps(body).encode("utf-8"))
            return
        if prefix == "truncated":
            self.protocol_version = "HTTP/1.1"
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Transfer-Encoding", "chunked")
            self.end_headers()
            self.wfile.write(b'8\r\n{"info":\r\n')
            self.wfile.flush()
            self.close_connection = True
            return
        if prefix == "slow_body":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", "100")
            self.end_headers()
            self.wfile.write(b'{"info":')
            self.wfile.flush()
            time.sleep(0.25)
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        if prefix == "malformed":
            self.wfile.write(b"{")
            return
        package = "wrong" if prefix == "mismatch" else requested_package
        version = "0.8.25" if prefix == "wrongversion" else requested_version
        body = (
            {"info": {"name": package, "version": version}}
            if pypi
            else {"name": package, "version": version}
        )
        self.wfile.write(json.dumps(body).encode("utf-8"))

    def log_message(self, *_args: object) -> None:
        return


def invoke(base: str, registry: str = "pypi", version: str = "0.8.26") -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "wait",
            "--registry", registry,
            "--package", "fathomdb",
            "--version", version,
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
            "--version", RELEASE_VERSION,
            "--pypi-base-url", f"{root}/delayed",
            "--npm-base-url", f"{root}/missing",
            "--timeout-seconds", "0.5",
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
            Handler.simple_counts["present"] = 0
            Handler.simple_accepts.clear()
            present = invoke(f"{root}/present", registry)
            assert present.returncode == 0, present.stderr
            assert "attempts=1" in present.stdout and "timeout_seconds=0.15" in present.stdout
            if registry == "pypi":
                assert Handler.simple_counts.get("present") == 1, Handler.simple_counts
                assert Handler.simple_accepts == ["application/vnd.pypi.simple.v1+json"], Handler.simple_accepts
            Handler.counts["eventual"] = 0
            Handler.simple_counts["eventual"] = 0
            eventual = invoke(f"{root}/eventual", registry)
            assert eventual.returncode == 0, eventual.stderr
            assert Handler.counts["eventual"] == 2
            if registry == "pypi":
                assert Handler.simple_counts.get("eventual") == 1, Handler.simple_counts
            assert "attempts=2" in eventual.stdout
        Handler.counts["wheel_lag"] = 0
        Handler.simple_counts["wheel_lag"] = 0
        wheel_lag = invoke(f"{root}/wheel_lag", "pypi")
        assert wheel_lag.returncode == 0, wheel_lag.stderr
        assert Handler.simple_counts["wheel_lag"] == 2
        assert "attempts=2" in wheel_lag.stdout
        Handler.counts["wheel_missing"] = 0
        Handler.simple_counts["wheel_missing"] = 0
        wheel_missing = invoke(f"{root}/wheel_missing", "pypi")
        assert wheel_missing.returncode == 1 and "simple_index_wheels_unavailable" in wheel_missing.stderr, wheel_missing.stderr
        Handler.counts["prerelease"] = 0
        Handler.simple_counts["prerelease"] = 0
        prerelease = invoke(f"{root}/prerelease", "pypi", version="0.8.31-rc.1")
        assert prerelease.returncode == 0, prerelease.stderr
        assert Handler.simple_counts["prerelease"] == 1
        for prefix, reason in (
            ("simple_auth", "authentication"),
            ("simple_server", "http_503"),
            ("simple_malformed", "malformed_metadata"),
        ):
            Handler.counts[prefix] = 0
            Handler.simple_counts[prefix] = 0
            result = invoke(f"{root}/{prefix}", "pypi")
            assert result.returncode == 1 and reason in result.stderr, result.stderr
            assert Handler.simple_counts[prefix] == 1
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
            ("truncated", "transport_error"),
            ("slow_body", "transport_error"),
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
        assert "registry_visibility_pass registry=pypi package=fathomdb " in shared_deadline.stdout
        assert "registry_visibility_pass registry=pypi package=fathomdb-cli " in shared_deadline.stdout
        assert "registry_visibility_pass registry=pypi package=fathomdb-mcp " in shared_deadline.stdout
        assert "registry=npm" in shared_deadline.stderr
        assert Handler.counts["delayed"] == 6
        remaining_bound = float(
            shared_deadline.stderr.split("timeout_seconds=", 1)[1].split()[0]
        )
        assert 0 < remaining_bound < 0.5, shared_deadline.stderr
    finally:
        server.shutdown()
        thread.join()
        server.server_close()
    transport = invoke(root)
    assert transport.returncode == 1 and "transport_error" in transport.stderr

    module = load_module()
    packages = module.release_packages(ROOT, RELEASE_VERSION, include_npm=True)
    assert [(item.registry, item.name) for item in packages[:4]] == [
        ("pypi", "fathomdb"),
        ("pypi", "fathomdb-cli"),
        ("pypi", "fathomdb-mcp"),
        ("npm", "fathomdb"),
    ]
    expected_npm = {
        json.loads(path.read_text(encoding="utf-8"))["name"]
        for path in (ROOT / "src/ts/npm").glob("*/package.json")
    } | {"fathomdb"}
    assert {item.name for item in packages if item.registry == "npm"} == expected_npm
    assert module.release_packages(ROOT, RELEASE_VERSION, include_npm=False) == (
        module.RegistryPackage("pypi", "fathomdb", RELEASE_VERSION),
        module.RegistryPackage("pypi", "fathomdb-cli", RELEASE_VERSION),
        module.RegistryPackage("pypi", "fathomdb-mcp", RELEASE_VERSION),
    )

    # Historical tags (e.g. v0.8.20) lack src/python-cli and src/python-mcp
    # manifests and must omit them rather than failing visibility checks.
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        historical_root = Path(td)
        assert module.release_packages(historical_root, "0.8.20", include_npm=False) == (
            module.RegistryPackage("pypi", "fathomdb", "0.8.20"),
        )
        # Mismatched src/python-cli/pyproject.toml version must fail closed.
        cli_dir = historical_root / "src/python-cli"
        cli_dir.mkdir(parents=True)
        (cli_dir / "pyproject.toml").write_text(
            '[project]\nname = "fathomdb-cli"\nversion = "0.8.99"\n',
            encoding="utf-8",
        )
        try:
            module.release_packages(historical_root, "0.8.30", include_npm=False)
        except ValueError as err:
            assert "python-cli manifest version" in str(err)
        else:
            raise AssertionError("expected ValueError on mismatched python-cli version")
        # Mismatched src/python-mcp/pyproject.toml version must fail closed.
        (cli_dir / "pyproject.toml").write_text(
            '[project]\nname = "fathomdb-cli"\nversion = "0.8.32"\n',
            encoding="utf-8",
        )
        mcp_dir = historical_root / "src/python-mcp"
        mcp_dir.mkdir(parents=True)
        (mcp_dir / "pyproject.toml").write_text(
            '[project]\nname = "fathomdb-mcp"\nversion = "0.8.99"\n',
            encoding="utf-8",
        )
        try:
            module.release_packages(historical_root, "0.8.32", include_npm=False)
        except ValueError as err:
            assert "python-mcp manifest version" in str(err)
        else:
            raise AssertionError("expected ValueError on mismatched python-mcp version")
    print("PASS test-wait-for-registry-visibility")


def test_wait_for_registry_visibility() -> None:
    main()


if __name__ == "__main__":
    main()
