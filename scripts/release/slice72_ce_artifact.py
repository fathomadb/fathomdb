#!/usr/bin/env python3
"""Build and verify an installed Slice 72 Python artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from typing import Any
import zipfile


SCHEMA = "fathomdb.slice72.ce-artifact/v1"


class ArtifactError(ValueError):
    """Artifact provenance is incomplete or inconsistent."""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run(
    command: list[str], *, cwd: Path, environment: dict[str, str] | None = None
) -> str:
    process_environment = os.environ.copy()
    if environment:
        process_environment.update(environment)
        if "PATH_prefix" in environment:
            process_environment["PATH"] = (
                environment["PATH_prefix"] + os.pathsep + process_environment["PATH"]
            )
            del process_environment["PATH_prefix"]
    result = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=process_environment,
    )
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise ArtifactError(f"command failed: {' '.join(command)}\n{detail}")
    return result.stdout.strip()


def native_member(wheel: Path) -> tuple[str, str]:
    with zipfile.ZipFile(wheel) as archive:
        members = [
            name
            for name in archive.namelist()
            if name.startswith("fathomdb/_fathomdb") and name.endswith((".so", ".pyd"))
        ]
        if len(members) != 1:
            raise ArtifactError("wheel must contain exactly one fathomdb native extension")
        payload = archive.read(members[0])
    return members[0], hashlib.sha256(payload).hexdigest()


def validate_receipt(
    receipt: dict[str, Any],
    manifest: dict[str, Any],
    role: str,
    device: str,
    *,
    verify_files: bool = True,
) -> dict[str, Any]:
    expected_keys = {
        "schema_version",
        "role",
        "device",
        "commit_sha",
        "features",
        "source_root",
        "source_clean",
        "build_command",
        "build_environment",
        "wheel_path",
        "wheel_size",
        "wheel_sha256",
        "wheel_native_member",
        "wheel_native_sha256",
        "venv_root",
        "python_path",
        "install_command",
        "module_path",
        "native_path",
        "native_sha256",
    }
    if set(receipt) != expected_keys:
        raise ArtifactError("artifact receipt keys are incomplete or unknown")
    if receipt["schema_version"] != SCHEMA:
        raise ArtifactError("unsupported artifact receipt schema")
    if receipt["role"] != role or receipt["device"] != device:
        raise ArtifactError("artifact role/device does not match the requested cell")
    if receipt["commit_sha"] != manifest[f"{role}_sha"]:
        raise ArtifactError("artifact commit does not match the manifest")
    if receipt["features"] != manifest["features"][device]:
        raise ArtifactError("artifact features do not match the manifest")
    if receipt["source_clean"] is not True:
        raise ArtifactError("artifact source checkout was not clean")
    expected_build_environment = (
        {
            "CUDA_HOME": "/usr/local/cuda",
            "LIBRARY_PATH": "/usr/local/cuda/lib64",
            "PATH_prefix": "/usr/local/cuda/bin",
        }
        if device == "cuda"
        else {}
    )
    if receipt["build_environment"] != expected_build_environment:
        raise ArtifactError("artifact build environment is not the registered environment")

    if not isinstance(receipt["wheel_size"], int) or receipt["wheel_size"] <= 0:
        raise ArtifactError("artifact wheel size is invalid")
    for key in ["wheel_sha256", "wheel_native_sha256", "native_sha256"]:
        value = receipt[key]
        if not isinstance(value, str) or len(value) != 64 or any(
            char not in "0123456789abcdef" for char in value
        ):
            raise ArtifactError(f"artifact {key} is malformed")
    if receipt["native_sha256"] != receipt["wheel_native_sha256"]:
        raise ArtifactError("installed native extension does not match the wheel")

    wheel = Path(receipt["wheel_path"]).resolve()
    venv = Path(receipt["venv_root"]).resolve()
    # Keep the venv launcher path lexical: resolving its symlink would turn it
    # into the system interpreter and defeat the isolated-install contract.
    python = Path(receipt["python_path"]).absolute()
    module = Path(receipt["module_path"]).resolve()
    native = Path(receipt["native_path"]).resolve()
    if not python.is_relative_to(venv) or not module.is_relative_to(venv) or not native.is_relative_to(venv):
        raise ArtifactError("artifact installed paths escape the recorded venv")
    build_command = receipt["build_command"]
    if (
        not isinstance(build_command, list)
        or len(build_command) != 11
        or not all(isinstance(item, str) and item for item in build_command)
    ):
        raise ArtifactError("artifact build command is malformed")
    expected_build = [
        "maturin",
        "build",
        "--release",
        "--auditwheel",
        "skip",
        "--out",
        str(wheel.parent),
        "--features",
        ",".join(receipt["features"]),
        "-i",
        build_command[-1],
    ]
    if build_command != expected_build:
        raise ArtifactError("artifact build command is not the registered command")
    expected_install = [
        str(python),
        "-m",
        "pip",
        "install",
        "--no-index",
        "--no-deps",
        str(wheel),
    ]
    if receipt["install_command"] != expected_install:
        raise ArtifactError("artifact install command is not the registered command")
    if verify_files:
        for path, label in [
            (wheel, "wheel"),
            (python, "python"),
            (module, "module"),
            (native, "native"),
        ]:
            if not path.is_file():
                raise ArtifactError(f"artifact {label} file is missing: {path}")
        if wheel.stat().st_size != receipt["wheel_size"] or sha256(wheel) != receipt["wheel_sha256"]:
            raise ArtifactError("installed artifact wheel identity changed")
        member, member_hash = native_member(wheel)
        if member != receipt["wheel_native_member"] or member_hash != receipt["wheel_native_sha256"]:
            raise ArtifactError("wheel native member identity changed")
        if sha256(native) != receipt["native_sha256"] or receipt["native_sha256"] != member_hash:
            raise ArtifactError("installed native extension does not match the wheel")
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--role", choices=["baseline", "candidate"], required=True)
    parser.add_argument("--device", choices=["cpu", "cuda"], required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--python", type=Path, default=Path(sys.executable))
    args = parser.parse_args()

    manifest = json.loads(args.manifest.resolve().read_text(encoding="utf-8"))
    source = args.source_root.resolve()
    output = args.output_dir.resolve()
    if output.exists() and any(output.iterdir()):
        raise ArtifactError(f"artifact output directory is not empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    commit = run(["git", "rev-parse", "HEAD"], cwd=source)
    if commit != manifest[f"{args.role}_sha"]:
        raise ArtifactError("source HEAD does not match the manifest")
    if run(["git", "status", "--porcelain", "--untracked-files=normal"], cwd=source):
        raise ArtifactError("source checkout has tracked or non-ignored untracked changes")

    wheel_dir = output / "wheel"
    venv = output / "venv"
    wheel_dir.mkdir()
    build_command = [
        "maturin",
        "build",
        "--release",
        "--auditwheel",
        "skip",
        "--out",
        str(wheel_dir),
        "--features",
        ",".join(manifest["features"][args.device]),
        "-i",
        str(args.python.resolve()),
    ]
    build_environment = (
        {
            "CUDA_HOME": "/usr/local/cuda",
            "LIBRARY_PATH": "/usr/local/cuda/lib64",
            "PATH_prefix": "/usr/local/cuda/bin",
        }
        if args.device == "cuda"
        else {}
    )
    run(
        build_command,
        cwd=source / "src" / "python",
        environment=build_environment,
    )
    wheels = list(wheel_dir.glob("*.whl"))
    if len(wheels) != 1:
        raise ArtifactError("build must produce exactly one wheel")
    wheel = wheels[0].resolve()
    member, member_hash = native_member(wheel)

    run([str(args.python.resolve()), "-m", "venv", str(venv)], cwd=source)
    installed_python = (venv / "bin" / "python").absolute()
    install_command = [
        str(installed_python),
        "-m",
        "pip",
        "install",
        "--no-index",
        "--no-deps",
        str(wheel),
    ]
    run(install_command, cwd=output)
    probe = run(
        [
            str(installed_python),
            "-c",
            "import json,fathomdb,fathomdb._fathomdb as n; print(json.dumps({'module':fathomdb.__file__,'native':n.__file__}))",
        ],
        cwd=output,
    )
    paths = json.loads(probe)
    module = Path(paths["module"]).resolve()
    native = Path(paths["native"]).resolve()
    receipt = {
        "schema_version": SCHEMA,
        "role": args.role,
        "device": args.device,
        "commit_sha": commit,
        "features": manifest["features"][args.device],
        "source_root": str(source),
        "source_clean": True,
        "build_command": build_command,
        "build_environment": build_environment,
        "wheel_path": str(wheel),
        "wheel_size": wheel.stat().st_size,
        "wheel_sha256": sha256(wheel),
        "wheel_native_member": member,
        "wheel_native_sha256": member_hash,
        "venv_root": str(venv),
        "python_path": str(installed_python),
        "install_command": install_command,
        "module_path": str(module),
        "native_path": str(native),
        "native_sha256": sha256(native),
    }
    validate_receipt(receipt, manifest, args.role, args.device)
    receipt_path = output / "artifact-receipt.json"
    receipt_path.write_text(json.dumps(receipt, sort_keys=True, separators=(",", ":")) + "\n", encoding="utf-8")
    print(receipt_path)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ArtifactError, OSError, json.JSONDecodeError) as exc:
        print(f"FAIL Slice 72 artifact build: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
