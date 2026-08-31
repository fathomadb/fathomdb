#!/usr/bin/env python3
"""Regression tests for CUDA-capable archive linkage inspection."""

from __future__ import annotations

import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
INSPECTOR = REPO_ROOT / "scripts/release/inspect-cuda-artifacts.py"


def package_artifacts(root: Path, node_source: Path, *, bundled_runtime: bool = False) -> tuple[Path, Path]:
    wheel = root / "fathomdb-0.8.25-cp311-abi3-manylinux_2_28_x86_64.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.write(node_source, "fathomdb/_fathomdb.abi3.so")
        if bundled_runtime:
            archive.write(node_source, "fathomdb/libcudart.so.12")
    napi = root / "fathomdb-linux-x64-gnu-0.8.25.tgz"
    package_root = root / "package"
    package_root.mkdir()
    shutil.copy2(node_source, package_root / "fathomdb.linux-x64-gnu.node")
    with tarfile.open(napi, "w:gz") as archive:
        archive.add(package_root, arcname="package")
    return wheel, napi


def inspect(wheel: Path, napi: Path, output: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["python3", str(INSPECTOR), "--python-wheel", str(wheel), "--napi-tarball", str(napi), "--output-dir", str(output)],
        check=False,
        text=True,
        capture_output=True,
    )


def main() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        wheel, napi = package_artifacts(root, Path("/bin/true"))
        result = inspect(wheel, napi, root / "valid")
        assert result.returncode == 0, result.stderr
        manifest = (root / "valid/artifact-linkage.json").read_text(encoding="utf-8")
        assert '"schema_version":"fathomdb.cuda-artifact-linkage/v1"' in manifest
        assert '"python-wheel:fathomdb/_fathomdb.abi3.so"' in manifest
        assert '"napi-tarball:package/fathomdb.linux-x64-gnu.node"' in manifest

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        wheel, napi = package_artifacts(root, Path("/bin/true"), bundled_runtime=True)
        result = inspect(wheel, napi, root / "bundled-runtime")
        assert result.returncode != 0
        assert "forbidden CUDA/NVIDIA shared-library payload" in result.stderr

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        (root / "runtime.c").write_text("void cuda_fixture(void) {}\n", encoding="utf-8")
        (root / "consumer.c").write_text("void cuda_fixture(void); int main(void) { cuda_fixture(); }\n", encoding="utf-8")
        subprocess.run(
            ["gcc", "-shared", "-fPIC", "-Wl,-soname,libcudart.so.12", "-o", str(root / "libcudart.so.12"), str(root / "runtime.c")],
            check=True,
        )
        subprocess.run(
            ["gcc", "-o", str(root / "consumer"), str(root / "consumer.c"), f"-L{root}", "-Wl,-rpath,$ORIGIN", "-l:libcudart.so.12"],
            check=True,
        )
        wheel, napi = package_artifacts(root, root / "consumer")
        result = inspect(wheel, napi, root / "needed-runtime")
        assert result.returncode != 0
        assert "forbidden CUDA/NVIDIA ELF dependency or SONAME" in result.stderr

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        (root / "runtime.c").write_text("void harmless_fixture(void) {}\n", encoding="utf-8")
        disguised = root / "native-helper.so"
        subprocess.run(
            ["gcc", "-shared", "-fPIC", "-Wl,-soname,libcudart.so.12", "-o", str(disguised), str(root / "runtime.c")],
            check=True,
        )
        wheel, napi = package_artifacts(root, disguised)
        result = inspect(wheel, napi, root / "renamed-runtime")
        assert result.returncode != 0
        assert "forbidden CUDA/NVIDIA ELF dependency or SONAME" in result.stderr

    print("PASS  CUDA artifact linkage inspector rejects bundled and dynamic CUDA runtime dependencies")


if __name__ == "__main__":
    main()
