"""AC80-25 / D-80.6-3 — refuse a locally hand-built sibling distribution.

The shipped Tegra build keeps the plain ``fathomdb`` distribution name and uses
a ``+tegra`` local version, so it is not a second distribution. This guard
retains the only remaining collision case: a locally hand-built sibling that
ships the same top-level ``fathomdb`` **import package**
(``src/python/pyproject.toml`` pins ``module-name = "fathomdb._fathomdb"`` and
``python-source = "."``).

Two distributions providing one import package are **not co-installable**.
pip does not detect the overlap; the second install simply overwrites the
first's ``fathomdb/`` directory, and whichever landed last silently shadows
the other — including its native ``_fathomdb`` extension, which is exactly
the artifact whose device support differs.

**This check is the enforcement; packaging metadata is not.**  A design review
established that no PyPI/pip mechanism can express it: PEP 621's ``[project]``
table, which maturin builds this wheel from, has no conflicts field, and pip
ignores ``Provides-Dist``/``Obsoletes-Dist`` even where a backend can emit
them.  Anything the published metadata says about mutual exclusivity is
advisory documentation around this module, and must not be described as
enforcement.

Importing this module performs the check and raises ``ImportError`` on a
collision; ``fathomdb/__init__.py`` imports it before anything else.
"""

from __future__ import annotations

import os
import sys

#: The top-level import package every FathomDB distribution ships.
IMPORT_PACKAGE = "fathomdb"

#: Known distribution names that may ship ``IMPORT_PACKAGE``.
#:
#: A declared list, not a scan of every installed distribution's file manifest.
#: A file-manifest scan is both slower and *less* accurate here: an editable
#: maturin install — the shape this repo's own ``.venv`` uses — records only
#: ``fathomdb.pth`` in its ``RECORD`` and writes no ``top_level.txt``, so
#: ``importlib.metadata.packages_distributions()`` reports no ``fathomdb``
#: top-level package for it at all (verified against this repo's ``.venv``).
#: It would therefore miss one side of the very collision this module exists
#: to catch.
#:
#: A distribution outside this list is not the hazard D-80.6-3 names; adding a
#: new FathomDB distribution means adding its name here.
#:
#: ``fathomdb-tegra`` is NOT a distribution this project ships, and under the
#: revised D-80.6-3 it never will be: the Tegra build carries the plain
#: ``fathomdb`` name and is distinguished by a ``+tegra`` PEP 440 local version.
#: It stays listed as a **tripwire for a locally hand-built sibling** — the one
#: way D-1's source-build path can still put two distributions over the same
#: ``fathomdb/`` import package. That residual case is the sole remaining reason
#: this guard exists, so removing the entry would empty it of purpose.
#:
#: Do not read this entry as evidence that a ``fathomdb-tegra`` release exists,
#: and do not cite this guard as what makes the naming decision safe — that role
#: belongs to the displaced-build check (AC80-27), which detects a
#: wrong-but-intact environment rather than a corrupted one.
PROVIDER_DISTRIBUTIONS: tuple[str, ...] = (
    "fathomdb",
    "fathomdb-tegra",
)

_METADATA_SUFFIXES = (".dist-info", ".egg-info")


def _escape(name: str) -> str:
    """Normalize a distribution name to its on-disk metadata-directory prefix.

    PEP 503 normalization (lower-case, ``-``/``_``/``.`` runs unified) followed
    by the wheel-spec escaping that replaces the separator with ``_``.  Written
    with ``str`` operations rather than ``re`` so this module imports nothing a
    CPython startup has not already loaded.
    """

    escaped = name.lower()
    for separator in "-.":
        escaped = escaped.replace(separator, "_")
    while "__" in escaped:
        escaped = escaped.replace("__", "_")
    return escaped


_PROVIDER_DIRECTORY_PREFIXES = {_escape(name): name for name in PROVIDER_DISTRIBUTIONS}


def installed_provider_distributions(
    candidates: tuple[str, ...] = PROVIDER_DISTRIBUTIONS,
) -> tuple[str, ...]:
    """Return the ``candidates`` whose installed metadata is on ``sys.path``.

    **Cost discipline — this runs on every ``import fathomdb``.**  Measured on
    CPython 3.13, once in this repo's ``.venv`` (18 distributions) and once in
    a synthetic environment carrying 318:

    ==================================================  ========  =========
    approach                                            18 dists  318 dists
    ==================================================  ========  =========
    ``importlib.metadata.packages_distributions()``        206 ms     262 ms
    ``distributions()`` + a ``top_level.txt`` read each   1.8 ms      23 ms
    one ``importlib.metadata.distribution(name)`` call   0.39 ms    0.39 ms
    this scan                                            0.42 ms    0.87 ms
    ==================================================  ========  =========

    The per-name ``importlib.metadata`` lookup is itself cheap, but *importing*
    ``importlib.metadata`` is not: ``python -X importtime -c 'import fathomdb'``
    attributed **48 ms cumulative** to it against a 91 ms total, because it
    drags in ``email``, ``zipfile`` and friends that nothing else in this
    package needs.  Adding 48 ms to every user's import to check two names is
    not a trade worth making, so this reads the same ``*.dist-info`` directory
    names ``importlib.metadata`` would, using only ``os`` and ``sys`` — both
    already loaded before any user code runs.  Measured contribution: **0.42 ms**
    here, **0.87 ms** in the 318-distribution environment.

    The narrowing this accepts is that a distribution installed somewhere
    ``sys.path`` does not name as a directory (a zipimport, a custom finder)
    is invisible.  Both FathomDB wheels are ordinary pip installs into
    ``site-packages``, which is exactly what this walks.
    """

    prefixes = (
        _PROVIDER_DIRECTORY_PREFIXES
        if candidates is PROVIDER_DISTRIBUTIONS
        else {_escape(name): name for name in candidates}
    )
    seen: set[str] = set()
    for entry in sys.path:
        if not entry:
            continue
        try:
            names = os.listdir(entry)
        except OSError:
            continue
        for name in names:
            for suffix in _METADATA_SUFFIXES:
                if not name.endswith(suffix):
                    continue
                # `{escaped_name}-{version}{suffix}`; the escaped name never
                # contains `-`, so the first `-` is the version separator.
                prefix = _escape(name[: -len(suffix)].split("-", 1)[0])
                if prefix in prefixes:
                    seen.add(prefixes[prefix])
                break
    return tuple(name for name in candidates if name in seen)


def reject_co_installed_distributions(providers: tuple[str, ...] | None = None) -> None:
    """Raise ``ImportError`` when two distributions claim ``IMPORT_PACKAGE``.

    Zero or one provider is the ordinary case and returns silently.  Two or
    more means the ``fathomdb/`` directory on disk belongs to whichever
    distribution was installed last, and nothing here can say which one the
    caller wanted — so this fails closed rather than guessing.
    """

    installed = installed_provider_distributions() if providers is None else providers
    if len(installed) < 2:
        return
    names = ", ".join(installed)
    raise ImportError(
        f"FathomDB distributions {names} are all installed, and they are not "
        f"co-installable: each ships the same top-level '{IMPORT_PACKAGE}' import "
        "package, so the one installed last has silently overwritten the others "
        "and you are importing an artifact you may not have chosen. Keep exactly "
        f"one — `pip uninstall {installed[-1]}` (or uninstall all of them and "
        "reinstall only the one that matches this machine), then import again."
    )


reject_co_installed_distributions()
