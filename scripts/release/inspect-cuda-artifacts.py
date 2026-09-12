#!/usr/bin/env python3
"""Fail closed on CUDA/NVIDIA runtime linkage in shipped CUDA-capable archives."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path, PurePosixPath


FORBIDDEN_LIBRARY = re.compile(
    r"^lib(?:cuda|cudart|cublas(?:lt)?|curand|cufft|cusolver|cusparse|nvrtc|nvjitlink|nvidia-ml|nvml)(?:[._-]|$)",
    re.IGNORECASE,
)
NEEDED = re.compile(r"Shared library: \[([^]]+)\]")
SONAME = re.compile(r"Library soname: \[([^]]+)\]")


def fail(message: str) -> None:
    raise SystemExit(f"cuda-artifact-linkage: {message}")


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as opened:
        for chunk in iter(lambda: opened.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def safe_name(name: str) -> PurePosixPath:
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts or not name:
        fail(f"unsafe archive member path: {name!r}")
    return path


def is_elf(path: Path) -> bool:
    with path.open("rb") as opened:
        return opened.read(4) == b"\x7fELF"


def forbidden_name(name: str) -> bool:
    return bool(FORBIDDEN_LIBRARY.match(PurePosixPath(name).name))


def inspect_member(path: Path, label: str, output_dir: Path, ordinal: int) -> dict[str, object]:
    completed = subprocess.run(["readelf", "-d", str(path)], check=False, capture_output=True, text=True)
    if completed.returncode != 0:
        fail(f"readelf failed for {label}: {completed.stderr.strip()}")
    needed = NEEDED.findall(completed.stdout)
    sonames = SONAME.findall(completed.stdout)
    forbidden = [library for library in [*needed, *sonames] if forbidden_name(library)]
    if forbidden:
        fail(f"forbidden CUDA/NVIDIA ELF dependency or SONAME in {label}: {', '.join(forbidden)}")
    output_name = f"{ordinal:02d}-{label.replace('/', '__')}.readelf.txt"
    output_path = output_dir / output_name
    output_path.write_text(completed.stdout, encoding="utf-8")
    return {
        "path": label,
        "sha256": digest(path),
        "needed": needed,
        "readelf": completed.stdout,
        "readelf_filename": output_name,
        "readelf_sha256": digest(output_path),
    }


def inspect_zip(archive: Path, root: Path, output_dir: Path, artifact_label: str, ordinal: int) -> tuple[dict[str, object], int]:
    members: list[dict[str, object]] = []
    with zipfile.ZipFile(archive) as opened:
        for info in sorted(opened.infolist(), key=lambda item: item.filename):
            path = safe_name(info.filename)
            if info.is_dir():
                continue
            mode = info.external_attr >> 16
            if mode and (mode & 0o170000) == 0o120000:
                fail(f"symlinked archive member is forbidden: {artifact_label}:{path}")
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(opened.read(info))
            if forbidden_name(str(path)):
                fail(f"forbidden CUDA/NVIDIA shared-library payload in {artifact_label}:{path}")
            if is_elf(target):
                members.append(inspect_member(target, f"{artifact_label}:{path}", output_dir, ordinal))
                ordinal += 1
    if not members:
        fail(f"archive has no ELF member to inspect: {artifact_label}")
    return {"sha256": digest(archive), "members": members}, ordinal


def inspect_tar(archive: Path, root: Path, output_dir: Path, artifact_label: str, ordinal: int) -> tuple[dict[str, object], int]:
    members: list[dict[str, object]] = []
    with tarfile.open(archive, "r:gz") as opened:
        for info in sorted(opened.getmembers(), key=lambda item: item.name):
            path = safe_name(info.name)
            if info.isdir():
                continue
            if not info.isreg():
                fail(f"non-regular archive member is forbidden: {artifact_label}:{path}")
            extracted = opened.extractfile(info)
            if extracted is None:
                fail(f"cannot read archive member: {artifact_label}:{path}")
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(extracted.read())
            if forbidden_name(str(path)):
                fail(f"forbidden CUDA/NVIDIA shared-library payload in {artifact_label}:{path}")
            if is_elf(target):
                members.append(inspect_member(target, f"{artifact_label}:{path}", output_dir, ordinal))
                ordinal += 1
    if not members:
        fail(f"archive has no ELF member to inspect: {artifact_label}")
    return {"sha256": digest(archive), "members": members}, ordinal


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--python-wheel", type=Path, required=True)
    parser.add_argument("--napi-tarball", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    for artifact in (args.python_wheel, args.napi_tarball):
        if not artifact.is_file() or artifact.is_symlink():
            fail(f"artifact must be a regular non-symlink file: {artifact}")
    if args.output_dir.exists():
        fail(f"output directory must be new: {args.output_dir}")
    args.output_dir.mkdir(parents=True)

    with tempfile.TemporaryDirectory(prefix="fathomdb-cuda-linkage-") as temporary:
        root = Path(temporary)
        wheel, ordinal = inspect_zip(args.python_wheel, root / "wheel", args.output_dir, "python-wheel", 1)
        napi, _ = inspect_tar(args.napi_tarball, root / "napi", args.output_dir, "napi-tarball", ordinal)

    manifest = {
        "schema_version": "fathomdb.cuda-artifact-linkage/v1",
        "artifacts": {"napi_tarball": napi, "python_wheel": wheel},
    }
    (args.output_dir / "artifact-linkage.json").write_text(
        json.dumps(manifest, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n",
        encoding="ascii",
    )
    print("cuda-artifact-linkage: inspected all packaged ELF members")


if __name__ == "__main__":
    main()
