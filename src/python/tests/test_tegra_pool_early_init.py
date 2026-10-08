"""The Tegra wheel runs the CUDA driver's early ``cuInit`` when it is imported.

Only an extension built with the ``tegra-pool`` feature on aarch64 Linux with
CUDA carries the import hook and its private probe
``_cuda_module_load_init``; every other build skips these tests. Each case
imports fathomdb in a fresh interpreter, because the hook runs once per
process at import.
"""

from __future__ import annotations

import os
import subprocess
import sys

import pytest

from fathomdb import _fathomdb

pytestmark = pytest.mark.skipif(
    not hasattr(_fathomdb, "_cuda_module_load_init"),
    reason="extension built without the tegra-pool import hook",
)

_PROBE = "from fathomdb import _fathomdb; print(_fathomdb._cuda_module_load_init())"


def _module_load_init(**overrides: str) -> str:
    env = {
        key: value
        for key, value in os.environ.items()
        if key not in ("FATHOMDB_CUDA_EARLY_INIT", "FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE")
    }
    env.update(overrides)
    completed = subprocess.run(
        [sys.executable, "-c", _PROBE],
        env=env,
        capture_output=True,
        text=True,
        timeout=120,
        check=True,
    )
    return completed.stdout.strip()


def test_import_runs_early_cuinit() -> None:
    assert _module_load_init() == "ran"


def test_the_opt_out_keeps_cuinit_from_running_at_import() -> None:
    assert _module_load_init(FATHOMDB_CUDA_EARLY_INIT="off") == "opted_out"


def test_only_the_exact_value_off_opts_out() -> None:
    assert _module_load_init(FATHOMDB_CUDA_EARLY_INIT="OFF") == "ran"


def test_every_cuda_component_on_cpu_skips_cuinit_at_import() -> None:
    assert (
        _module_load_init(FATHOMDB_EMBED_DEVICE="cpu", FATHOMDB_RERANK_DEVICE="cpu")
        == "skipped_cpu_only"
    )
