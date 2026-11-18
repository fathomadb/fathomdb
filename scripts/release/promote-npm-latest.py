#!/usr/bin/env python3
"""Promote the main npm package with OIDC and wait for public readback."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
import time


NPM_PIN = "11.21.0"
NPM_REGISTRY = "https://registry.npmjs.org"
READBACK_SECONDS = 60.0
READBACK_ATTEMPTS = 6
VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?")
OIDC_VARIABLES = ("ACTIONS_ID_TOKEN_REQUEST_URL", "ACTIONS_ID_TOKEN_REQUEST_TOKEN")


def _version(value: object) -> bool:
    return isinstance(value, str) and len(value) <= 64 and VERSION.fullmatch(value) is not None


def _npm(args: list[str], *, cwd: str, env: dict[str, str], timeout: float) -> subprocess.CompletedProcess:
    return subprocess.run(
        [
            "npm", f"--registry={NPM_REGISTRY}", f"--userconfig={cwd}/user.npmrc",
            f"--globalconfig={cwd}/global.npmrc", "--loglevel=error", "--fetch-retries=0", *args,
        ],
        cwd=cwd, env=env, timeout=timeout, check=False, capture_output=True,
    )


def _observe_latest(cwd: str, env: dict[str, str], timeout: float) -> str:
    try:
        result = _npm(
            ["--prefer-online", "--json", "view", "fathomdb", "dist-tags.latest"],
            cwd=cwd, env=env, timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return "lookup-timeout"
    except OSError:
        return "lookup-unavailable"
    if result.returncode != 0:
        return f"lookup-error(exit={result.returncode})"
    if not result.stdout.strip():
        return "missing"
    try:
        observed = json.loads(result.stdout)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return "invalid-metadata"
    if observed is None:
        return "missing"
    return observed if _version(observed) else "invalid-metadata"


def _verify_latest(expected: str, cwd: str, env: dict[str, str]) -> int:
    deadline = time.monotonic() + READBACK_SECONDS
    observed = "not-observed"
    for attempt in range(1, READBACK_ATTEMPTS + 1):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        observed = _observe_latest(cwd, env, min(10.0, remaining))
        print(f"npm latest readback {attempt}/{READBACK_ATTEMPTS}: expected={expected} observed={observed}", flush=True)
        if observed == expected and time.monotonic() <= deadline:
            return 0
        if attempt < READBACK_ATTEMPTS:
            time.sleep(min(2 ** (attempt - 1), max(0.0, deadline - time.monotonic())))
    print(f"npm latest did not become visible: expected={expected} observed={observed}", flush=True)
    return 1


def main() -> int:
    """Promote one exact fathomdb version, returning failure on unknown visibility."""
    if len(sys.argv) != 2 or not _version(sys.argv[1]):
        print("usage: promote-npm-latest.py <exact-version>", file=sys.stderr)
        return 2
    version = sys.argv[1]
    if not all(os.environ.get(name) for name in OIDC_VARIABLES):
        print("GitHub OIDC request credentials are unavailable", file=sys.stderr)
        return 1
    env = {
        name: value for name, value in os.environ.items()
        if name.upper() not in {"NODE_AUTH_TOKEN", "NPM_TOKEN"}
        and not name.lower().startswith("npm_config_")
    }
    with tempfile.TemporaryDirectory(prefix="fathomdb-npm-promote-") as working:
        try:
            npm_version = _npm(["--version"], cwd=working, env=env, timeout=10)
            if npm_version.returncode != 0 or npm_version.stdout.strip() != NPM_PIN.encode():
                print(f"OIDC dist-tag requires npm {NPM_PIN}", file=sys.stderr)
                return 1
            print(f"Promoting fathomdb@{version} to latest with npm {NPM_PIN} OIDC", flush=True)
            result = _npm(["dist-tag", "add", f"fathomdb@{version}", "latest"], cwd=working, env=env, timeout=60)
        except (OSError, subprocess.TimeoutExpired) as error:
            print(f"npm promotion command failed: {type(error).__name__}", file=sys.stderr)
            return 1
        if result.returncode != 0:
            match = re.search(rb"(?im)^npm (?:error|err!) code ([A-Z0-9_]{1,32})\s*$", result.stderr)
            code = f", {match[1].decode()}" if match else ""
            print(f"npm dist-tag add failed (exit={result.returncode}{code})", file=sys.stderr)
            return 1

        public_env = {name: value for name, value in env.items() if name not in OIDC_VARIABLES}
        return _verify_latest(version, working, public_env)


if __name__ == "__main__":
    raise SystemExit(main())
