"""AC80-25 / D-80.6-3 — the Tegra and generic distributions are not co-installable.

Both distributions ship the same top-level ``fathomdb`` import package
(``src/python/pyproject.toml``: ``module-name = "fathomdb._fathomdb"``,
``python-source = "."``), so installing both leaves whichever landed last
silently shadowing the other.  No packaging metadata can prevent that — PEP
621's ``[project]`` table has no conflicts field and pip ignores
``Provides-Dist``/``Obsoletes-Dist`` — so the enforcement is an import-time
check in ``fathomdb/__init__.py``.

These tests construct the two-distribution situation **for real**: a temporary
directory carrying genuine ``.dist-info`` metadata for both distributions is
prepended to ``PYTHONPATH`` and a fresh interpreter is asked to
``import fathomdb``.  Nothing here mocks the detector; the assertions are on
what a real interpreter does with real installed-distribution metadata.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

_VERSION = "0.0.0"


def _write_dist_info(root: Path, distribution: str) -> Path:
    """Create a real ``.dist-info`` directory ``importlib.metadata`` will find."""

    directory = root / f"{distribution.replace('-', '_')}-{_VERSION}.dist-info"
    directory.mkdir(parents=True)
    (directory / "METADATA").write_text(
        f"Metadata-Version: 2.1\nName: {distribution}\nVersion: {_VERSION}\n",
        encoding="utf-8",
    )
    (directory / "WHEEL").write_text(
        "Wheel-Version: 1.0\nGenerator: fathomdb-test\nRoot-Is-Purelib: false\n",
        encoding="utf-8",
    )
    (directory / "RECORD").write_text("", encoding="utf-8")
    return directory


def _import_fathomdb(extra_path: Path) -> subprocess.CompletedProcess[str]:
    """Import ``fathomdb`` in a fresh interpreter that sees ``extra_path`` first."""

    env = dict(os.environ)
    existing = env.get("PYTHONPATH")
    env["PYTHONPATH"] = str(extra_path) if not existing else f"{extra_path}{os.pathsep}{existing}"
    return subprocess.run(
        [sys.executable, "-c", "import fathomdb; print(fathomdb.__version__)"],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )


def test_two_distributions_claiming_fathomdb_are_detected(tmp_path: Path) -> None:
    """Both providers installed -> import fails loudly, naming both (AC80-25)."""

    _write_dist_info(tmp_path, "fathomdb")
    _write_dist_info(tmp_path, "fathomdb-tegra")

    result = _import_fathomdb(tmp_path)

    assert result.returncode != 0, (
        "importing fathomdb with two distributions providing the same top-level "
        f"package must fail; got stdout={result.stdout!r}"
    )
    assert "fathomdb" in result.stderr
    assert "fathomdb-tegra" in result.stderr
    assert "not co-installable" in result.stderr


def test_single_distribution_imports_cleanly(tmp_path: Path) -> None:
    """The control arm: one provider is the normal case and must not raise."""

    _write_dist_info(tmp_path, "fathomdb")

    result = _import_fathomdb(tmp_path)

    assert result.returncode == 0, result.stderr
    assert result.stdout.strip()


def test_an_unrelated_distribution_is_not_a_collision(tmp_path: Path) -> None:
    """A distribution that does not ship the `fathomdb` package is not the hazard."""

    _write_dist_info(tmp_path, "fathomdb")
    _write_dist_info(tmp_path, "unrelated-project")

    result = _import_fathomdb(tmp_path)

    assert result.returncode == 0, result.stderr


def test_guard_reports_the_installed_providers_it_found() -> None:
    """The detector is inspectable, and this environment carries exactly one."""

    from fathomdb import _coinstall

    found = _coinstall.installed_provider_distributions()

    assert isinstance(found, tuple)
    assert len(found) <= 1, f"this test environment already collides: {found}"
    assert set(found) <= set(_coinstall.PROVIDER_DISTRIBUTIONS)


@pytest.mark.parametrize("providers", [(), ("fathomdb",)])
def test_fewer_than_two_providers_never_raises(providers: tuple[str, ...]) -> None:
    """The refusal is exactly the >=2 case; zero and one are ordinary."""

    from fathomdb import _coinstall

    _coinstall.reject_co_installed_distributions(providers)


def test_two_providers_raise_import_error_with_a_remedy() -> None:
    """The refusal is an ImportError and tells the operator what to do."""

    from fathomdb import _coinstall

    with pytest.raises(ImportError) as excinfo:
        _coinstall.reject_co_installed_distributions(("fathomdb", "fathomdb-tegra"))

    message = str(excinfo.value)
    assert "fathomdb" in message
    assert "fathomdb-tegra" in message
    assert "pip uninstall" in message


def test_generic_build_on_confirmed_classic_tegra_warns_under_default_filters() -> None:
    """AC80-27: a default-filtered child visibly emits the named warning."""

    coinstall_path = Path(__file__).parents[1] / "fathomdb" / "_coinstall.py"
    child = "\n".join(
        [
            "import importlib.util",
            f"spec = importlib.util.spec_from_file_location('coinstall_child', {str(coinstall_path)!r})",
            "module = importlib.util.module_from_spec(spec)",
            "spec.loader.exec_module(module)",
            "module.warn_if_generic_build_on_classic_tegra(",
            "    version='0.8.22', platform=module.PlatformProbeResult.classic_tegra()",
            ")",
        ]
    )
    result = subprocess.run([sys.executable, "-c", child], capture_output=True, text=True, check=False)

    assert result.returncode == 0, result.stderr
    assert "FathomDbPlatformWarning" in result.stderr
    assert "Generic FathomDB build detected on confirmed classic Tegra" in result.stderr


@pytest.mark.parametrize(
    "platform",
    ["thor", "tier_two_missing", "generic_aarch64", "non_aarch64"],
)
def test_only_confirmed_classic_tegra_warns(platform: str) -> None:
    """Thor and indeterminate hosts fail closed toward silence."""

    coinstall_path = Path(__file__).parents[1] / "fathomdb" / "_coinstall.py"
    child = "\n".join(
        [
            "import importlib.util",
            f"spec = importlib.util.spec_from_file_location('coinstall_child', {str(coinstall_path)!r})",
            "module = importlib.util.module_from_spec(spec)",
            "spec.loader.exec_module(module)",
            "module.warn_if_generic_build_on_classic_tegra(",
            f"    version='0.8.22', platform=module.PlatformProbeResult.named({platform!r})",
            ")",
        ]
    )
    result = subprocess.run([sys.executable, "-c", child], capture_output=True, text=True, check=False)

    assert result.returncode == 0, result.stderr
    assert result.stderr == ""


def test_tegra_local_version_is_not_warned() -> None:
    """The temporary +tegra metadata stamp is the only accepted build identity."""

    from fathomdb import _coinstall

    import warnings

    with warnings.catch_warnings(record=True) as captured:
        warnings.simplefilter("default")
        _coinstall.warn_if_generic_build_on_classic_tegra(
            version="0.8.22+tegra",
            platform=_coinstall.PlatformProbeResult.classic_tegra(),
        )
    assert not captured


def test_editable_maturin_metadata_is_found_on_the_active_sys_path(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """PEP 660's source .pth and site-packages metadata identify this build."""

    from fathomdb import _coinstall

    source_root = tmp_path / "source"
    package_root = source_root / "fathomdb"
    package_root.mkdir(parents=True)
    (package_root / "_coinstall.py").write_text("# editable source marker\n", encoding="utf-8")
    site_packages = tmp_path / "site-packages"
    metadata = _write_dist_info(site_packages, "fathomdb")
    (metadata / "METADATA").write_text(
        "Metadata-Version: 2.1\nName: fathomdb\nVersion: 0.8.22+tegra\n",
        encoding="utf-8",
    )

    monkeypatch.setattr(_coinstall, "__file__", str(package_root / "_coinstall.py"))
    monkeypatch.setattr(_coinstall.sys, "path", [str(source_root), str(site_packages)])

    assert _coinstall.installed_fathomdb_version() == "0.8.22+tegra"
