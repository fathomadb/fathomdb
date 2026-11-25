#!/usr/bin/env python3
"""Wait for exact PyPI/npm package versions without masking hard failures."""

from __future__ import annotations

import argparse
import http.client
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path

TRANSPORT_ERRORS = (
    urllib.error.URLError,
    TimeoutError,
    http.client.HTTPException,
    OSError,
)

PEP691_ACCEPT = "application/vnd.pypi.simple.v1+json"
PYPI_GENERIC_WHEEL_PLATFORM_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("linux_x86_64", re.compile(r"manylinux[^-]*x86_64\.whl$")),
    ("linux_aarch64", re.compile(r"manylinux[^-]*aarch64\.whl$")),
    ("macosx_x86_64", re.compile(r"macosx_[^-]*x86_64\.whl$")),
    ("macosx_arm64", re.compile(r"macosx_[^-]*arm64\.whl$")),
    ("win_amd64", re.compile(r"win_amd64\.whl$")),
)


def pep440_normalize(version: str) -> str:
    """Normalize SemVer pre-release tags (-rc.N/-alpha.N/-beta.N) to PEP 440."""

    version = re.sub(r"-rc\.(\d+)$", r"rc\1", version)
    version = re.sub(r"-alpha\.(\d+)$", r"a\1", version)
    version = re.sub(r"-beta\.(\d+)$", r"b\1", version)
    return version


def _normalize_pypi_dist_prefix(package: str) -> str:
    return re.sub(r"[-_.]+", "_", package.lower())


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


def _load_toml(path: Path) -> dict[str, object]:
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
        import tomli as tomllib  # type: ignore[import-not-found,no-redef]
    with path.open("rb") as fh:
        data = tomllib.load(fh)
    if not isinstance(data, dict):
        raise ValueError(f"manifest must be a TOML table: {path}")
    return data


def release_packages(root: Path, version: str, *, include_npm: bool) -> tuple[RegistryPackage, ...]:
    """Return the manifest-derived PyPI/npm release package set."""

    packages = [RegistryPackage("pypi", "fathomdb", version)]
    for subdir, expected_name in (
        ("src/python-cli", "fathomdb-cli"),
        ("src/python-mcp", "fathomdb-mcp"),
    ):
        extra_manifest = root / subdir / "pyproject.toml"
        if extra_manifest.is_file():
            extra_project = _load_toml(extra_manifest).get("project")
            label = Path(subdir).name
            if not isinstance(extra_project, dict):
                raise ValueError(f"{label} manifest lacks [project]: {extra_manifest}")
            if extra_project.get("version") != version:
                raise ValueError(f"{label} manifest version does not match requested release")
            extra_name = extra_project.get("name")
            if extra_name != expected_name:
                raise ValueError(f"unexpected {label} package name: {extra_name!r}")
            packages.append(RegistryPackage("pypi", expected_name, version))
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
    effective_version = pep440_normalize(version) if registry == "pypi" else version
    quoted_version = urllib.parse.quote(effective_version, safe="")
    if registry == "pypi":
        return f"{base}/pypi/{quoted_name}/{quoted_version}/json"
    return f"{base}/{quoted_name}/{quoted_version}"


def _pypi_simple_url(base_url: str, package: str) -> str:
    base = base_url.rstrip("/")
    quoted_name = urllib.parse.quote(package, safe="")
    return f"{base}/simple/{quoted_name}/"


def _validate_metadata(registry: str, payload: object, package: str, version: str) -> None:
    if not isinstance(payload, dict):
        raise ValueError("malformed_metadata")
    metadata = payload.get("info") if registry == "pypi" else payload
    if not isinstance(metadata, dict):
        raise ValueError("malformed_metadata")
    if metadata.get("name") != package:
        raise ValueError("package_mismatch")
    expected_version = pep440_normalize(version) if registry == "pypi" else version
    if metadata.get("version") != expected_version:
        raise ValueError("version_mismatch")


def _validate_pypi_simple_wheels(payload: object, package: str, version: str) -> bool:
    """Return True iff PEP 691 /simple/<pkg>/ advertises all 5 generic wheels."""

    if not isinstance(payload, dict):
        raise ValueError("malformed_metadata")
    name = payload.get("name")
    if not isinstance(name, str) or _normalize_pypi_dist_prefix(name) != _normalize_pypi_dist_prefix(package):
        raise ValueError("package_mismatch")
    files = payload.get("files")
    if not isinstance(files, list):
        raise ValueError("malformed_metadata")
    prefix = f"{_normalize_pypi_dist_prefix(package)}-{pep440_normalize(version)}-"
    filenames: list[str] = []
    for entry in files:
        if not isinstance(entry, dict):
            raise ValueError("malformed_metadata")
        filename = entry.get("filename")
        if not isinstance(filename, str):
            raise ValueError("malformed_metadata")
        if entry.get("yanked"):
            continue
        if filename.startswith(prefix) and filename.endswith(".whl"):
            filenames.append(filename)
    for _label, pattern in PYPI_GENERIC_WHEEL_PLATFORM_PATTERNS:
        if not any(pattern.search(fname) for fname in filenames):
            return False
    return True


def _validate_absence_response(
    registry: str, error: urllib.error.HTTPError, version: str
) -> None:
    content_type = error.headers.get_content_type()
    if content_type != "application/json":
        raise ValueError("malformed_metadata")
    try:
        payload = json.load(error)
    except (json.JSONDecodeError, UnicodeDecodeError) as decode_error:
        raise ValueError("malformed_metadata") from decode_error
    expected: object = (
        {"message": "Not Found"}
        if registry == "pypi"
        else f"version not found: {version}"
    )
    if payload != expected:
        raise ValueError("absence_mismatch")


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
    last_pending_reason = "exact_version_unavailable"
    url = _metadata_url(package.registry, base_url, package.name, package.version)
    simple_url = (
        _pypi_simple_url(base_url, package.name) if package.registry == "pypi" else None
    )
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise VisibilityFailure(
                last_pending_reason, attempts, time.monotonic() - started
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
            if simple_url is None:
                return VisibilityResult(attempts, time.monotonic() - started)
            simple_remaining = deadline - time.monotonic()
            if simple_remaining <= 0:
                raise VisibilityFailure(
                    "simple_index_wheels_unavailable", attempts, time.monotonic() - started
                )
            simple_request = urllib.request.Request(
                simple_url, headers={"Accept": PEP691_ACCEPT}
            )
            try:
                with urllib.request.urlopen(
                    simple_request, timeout=max(0.001, min(30.0, simple_remaining))
                ) as simple_response:
                    try:
                        simple_payload = json.load(simple_response)
                    except (json.JSONDecodeError, UnicodeDecodeError) as error:
                        raise VisibilityFailure(
                            "malformed_metadata", attempts, time.monotonic() - started
                        ) from error
                    try:
                        wheels_ready = _validate_pypi_simple_wheels(
                            simple_payload, package.name, package.version
                        )
                    except ValueError as error:
                        raise VisibilityFailure(
                            str(error), attempts, time.monotonic() - started
                        ) from error
                    if wheels_ready:
                        return VisibilityResult(attempts, time.monotonic() - started)
                    last_pending_reason = "simple_index_wheels_unavailable"
            except urllib.error.HTTPError as error:
                if error.code == 404:
                    last_pending_reason = "simple_index_wheels_unavailable"
                else:
                    if error.code in (401, 403):
                        reason = "authentication"
                    elif error.code == 429:
                        reason = "rate_limit"
                    else:
                        reason = f"http_{error.code}"
                    raise VisibilityFailure(
                        reason, attempts, time.monotonic() - started
                    ) from error
        except urllib.error.HTTPError as error:
            if error.code == 404:
                try:
                    _validate_absence_response(package.registry, error, package.version)
                except TRANSPORT_ERRORS as transport_error:
                    raise VisibilityFailure(
                        "transport_error", attempts, time.monotonic() - started
                    ) from transport_error
                except ValueError as absence_error:
                    raise VisibilityFailure(
                        str(absence_error), attempts, time.monotonic() - started
                    ) from absence_error
                last_pending_reason = "exact_version_unavailable"
            else:
                if error.code in (401, 403):
                    reason = "authentication"
                elif error.code == 429:
                    reason = "rate_limit"
                else:
                    reason = f"http_{error.code}"
                raise VisibilityFailure(reason, attempts, time.monotonic() - started) from error
        except TRANSPORT_ERRORS as error:
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
