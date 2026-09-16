#!/usr/bin/env python3
"""Wait for exact PyPI/npm package versions without masking hard failures."""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class RegistryPackage:
    """One exact registry coordinate required by a release."""

    registry: str
    name: str
    version: str


@dataclass(frozen=True)
class VisibilityResult:
    """Bounded polling evidence for one exact package coordinate."""

    attempts: int
    elapsed_seconds: float


class VisibilityFailure(RuntimeError):
    """Terminal visibility classification with bounded-attempt evidence."""

    def __init__(self, reason: str, attempts: int, elapsed_seconds: float) -> None:
        super().__init__(reason)
        self.reason = reason
        self.attempts = attempts
        self.elapsed_seconds = elapsed_seconds


def release_packages(root: Path, version: str, *, include_npm: bool) -> tuple[RegistryPackage, ...]:
    """Return the manifest-derived PyPI/npm release package set."""

    packages = [RegistryPackage("pypi", "fathomdb", version)]
    if not include_npm:
        return tuple(packages)
    main = json.loads((root / "src/ts/package.json").read_text(encoding="utf-8"))
    if main.get("version") != version:
        raise ValueError("main npm manifest version does not match requested release")
    packages.append(RegistryPackage("npm", main["name"], version))
    platform_names = []
    for path in sorted((root / "src/ts/npm").glob("*/package.json")):
        manifest = json.loads(path.read_text(encoding="utf-8"))
        if manifest.get("version") != version:
            raise ValueError(f"platform npm manifest version does not match requested release: {path}")
        platform_names.append(manifest["name"])
    platform_names.sort()
    packages.extend(RegistryPackage("npm", name, version) for name in platform_names)
    return tuple(packages)


def _metadata_url(registry: str, base_url: str, package: str, version: str) -> str:
    base = base_url.rstrip("/")
    quoted_name = urllib.parse.quote(package, safe="")
    quoted_version = urllib.parse.quote(version, safe="")
    if registry == "pypi":
        return f"{base}/pypi/{quoted_name}/{quoted_version}/json"
    return f"{base}/{quoted_name}/{quoted_version}"


def _validate_metadata(registry: str, payload: object, package: str, version: str) -> None:
    if not isinstance(payload, dict):
        raise ValueError("malformed_metadata")
    metadata = payload.get("info") if registry == "pypi" else payload
    if not isinstance(metadata, dict):
        raise ValueError("malformed_metadata")
    if metadata.get("name") != package:
        raise ValueError("package_mismatch")
    if metadata.get("version") != version:
        raise ValueError("version_mismatch")


def wait_for_version(
    package: RegistryPackage,
    *,
    base_url: str,
    timeout_seconds: float,
    interval_seconds: float,
) -> VisibilityResult:
    """Wait through exact absence only; every other failure is terminal."""

    started = time.monotonic()
    deadline = started + timeout_seconds
    attempts = 0
    url = _metadata_url(package.registry, base_url, package.name, package.version)
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise VisibilityFailure(
                "exact_version_unavailable", attempts, time.monotonic() - started
            )
        attempts += 1
        try:
            with urllib.request.urlopen(url, timeout=max(0.001, min(30.0, remaining))) as response:
                try:
                    payload = json.load(response)
                except (json.JSONDecodeError, UnicodeDecodeError) as error:
                    raise VisibilityFailure(
                        "malformed_metadata", attempts, time.monotonic() - started
                    ) from error
                try:
                    _validate_metadata(package.registry, payload, package.name, package.version)
                except ValueError as error:
                    raise VisibilityFailure(
                        str(error), attempts, time.monotonic() - started
                    ) from error
                return VisibilityResult(attempts, time.monotonic() - started)
        except urllib.error.HTTPError as error:
            if error.code != 404:
                if error.code in (401, 403):
                    reason = "authentication"
                elif error.code == 429:
                    reason = "rate_limit"
                else:
                    reason = f"http_{error.code}"
                raise VisibilityFailure(reason, attempts, time.monotonic() - started) from error
        except urllib.error.URLError as error:
            raise VisibilityFailure(
                "transport_error", attempts, time.monotonic() - started
            ) from error
        time.sleep(min(interval_seconds, max(0.0, deadline - time.monotonic())))


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    wait = subparsers.add_parser("wait")
    wait.add_argument("--registry", choices=("pypi", "npm"), required=True)
    wait.add_argument("--package", required=True)
    wait.add_argument("--version", required=True)
    wait.add_argument("--base-url")
    wait.add_argument("--timeout-seconds", type=float, default=900.0)
    wait.add_argument("--interval-seconds", type=float, default=15.0)
    release = subparsers.add_parser("wait-release")
    release.add_argument("--repo-root", type=Path, required=True)
    release.add_argument("--version", required=True)
    release.add_argument("--exclude-npm", action="store_true")
    release.add_argument("--pypi-base-url", default="https://pypi.org")
    release.add_argument("--npm-base-url", default="https://registry.npmjs.org")
    release.add_argument("--timeout-seconds", type=float, default=900.0)
    release.add_argument("--interval-seconds", type=float, default=15.0)
    return parser


def main() -> int:
    args = _parser().parse_args()
    try:
        if args.command == "wait":
            packages = (RegistryPackage(args.registry, args.package, args.version),)
            base_urls = {
                args.registry: args.base_url
                or (
                    "https://pypi.org"
                    if args.registry == "pypi"
                    else "https://registry.npmjs.org"
                )
            }
            release_deadline = None
        else:
            packages = release_packages(
                args.repo_root,
                args.version,
                include_npm=not args.exclude_npm,
            )
            base_urls = {"pypi": args.pypi_base_url, "npm": args.npm_base_url}
            release_deadline = time.monotonic() + args.timeout_seconds
        for package in packages:
            package_timeout = (
                args.timeout_seconds
                if release_deadline is None
                else max(0.0, release_deadline - time.monotonic())
            )
            result = wait_for_version(
                package,
                base_url=base_urls[package.registry],
                timeout_seconds=package_timeout,
                interval_seconds=args.interval_seconds,
            )
            print(
                f"registry_visibility_pass registry={package.registry} "
                f"package={package.name} version={package.version} attempts={result.attempts} "
                f"elapsed_seconds={result.elapsed_seconds:.3f} "
                f"timeout_seconds={package_timeout} interval_seconds={args.interval_seconds}"
            )
    except VisibilityFailure as error:
        print(
            f"registry_visibility_failed registry={package.registry} package={package.name} "
            f"version={package.version} attempts={error.attempts} "
            f"elapsed_seconds={error.elapsed_seconds:.3f} "
            f"timeout_seconds={package_timeout} interval_seconds={args.interval_seconds} "
            f"reason={error.reason}",
            file=sys.stderr,
        )
        return 1
    except ValueError as error:
        print(f"registry_visibility_failed reason={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
