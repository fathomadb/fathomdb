"""Fork after ``import fathomdb`` on the Tegra wheel (0.8.28 Slice 31, D-15).

The Tegra wheel runs the CUDA driver's early ``cuInit`` at import, so a
``multiprocessing`` child created with ``fork`` afterwards cannot use CUDA
(hazard H-1). These cases characterize that contract on the Jetson AGX Orin
64 GB (test-plan TDN-06 to TDN-10, F1-F5): each runs a driver in a fresh
interpreter, because the import hook runs once per process, and every
forked or spawned child must finish within 120 s (a hang fails).

They need the Tegra wheel (an extension with the ``tegra-pool`` import hook)
on a Jetson and the cached default-embedder weights; elsewhere they skip.
With ``FATHOMDB_REQUIRE_LIVE=1`` (the Orin windows) a skip is a failure.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from fathomdb import _fathomdb

_ON_JETSON = Path("/etc/nv_tegra_release").exists()
_TEGRA_WHEEL = hasattr(_fathomdb, "_cuda_module_load_init") and _ON_JETSON

# On a Jetson, FATHOMDB_REQUIRE_LIVE=1 turns a missing Tegra wheel into a
# failure instead of a skip; other hosts always skip.
if _ON_JETSON and not _TEGRA_WHEEL and os.environ.get("FATHOMDB_REQUIRE_LIVE") == "1":
    raise RuntimeError(
        "FATHOMDB_REQUIRE_LIVE=1 on a Jetson, but this extension lacks the tegra-pool "
        "import hook: install the Tegra wheel"
    )

pytestmark = pytest.mark.skipif(
    not _TEGRA_WHEEL, reason="needs the Tegra wheel (tegra-pool import hook) on a Jetson"
)

# The driver: optional parent GPU work, then one child (fork or spawn) that
# opens an engine with the default embedder under its policy and embeds.
# Each step prints one JSON line the test reads.
_DRIVER = r"""
import json
import multiprocessing as mp
import os
import sys
import tempfile

from fathomdb import Engine
from fathomdb.errors import EmbedDevicePolicyError


def describe(engine):
    resolution = engine.open_report().embedder_device_resolution
    device = resolution.effective_device
    allocator = device.cuda_device.cuda_allocator if device.cuda_device is not None else None
    return {
        "device": device.kind,
        "path": allocator.path if allocator is not None else None,
        "allocator_reason": allocator.reason if allocator is not None else None,
        "resolution_reason": resolution.reason,
    }


def open_and_embed(policy, label):
    os.environ["FATHOMDB_EMBED_DEVICE"] = policy
    with tempfile.TemporaryDirectory() as directory:
        engine = Engine.open(os.path.join(directory, label + ".sqlite"), use_default_embedder=True)
        try:
            facts = describe(engine)
            facts["embedded"] = len(engine.embed(label + " embeds")) > 0
            return facts
        finally:
            engine.close()


def child(connection, policy):
    try:
        connection.send({"ok": True, **open_and_embed(policy, "child")})
    except EmbedDevicePolicyError as error:
        connection.send({"ok": False, "error": type(error).__name__, "kind": error.kind})
    except Exception as error:  # every other failure is reported, not hidden
        connection.send({"ok": False, "error": type(error).__name__, "message": str(error)})


def main():
    start_method, child_policy, parent_policy = sys.argv[1], sys.argv[2], sys.argv[3]
    if parent_policy != "none":
        print("parent " + json.dumps(open_and_embed(parent_policy, "parent-before")), flush=True)
    context = mp.get_context(start_method)
    receiver, sender = context.Pipe(duplex=False)
    process = context.Process(target=child, args=(sender, child_policy))
    process.start()
    result = receiver.recv() if receiver.poll(120) else {"ok": False, "error": "timeout"}
    process.join(120)
    if process.is_alive():
        process.kill()
        process.join()
    result["exitcode"] = process.exitcode
    print("child " + json.dumps(result), flush=True)
    if parent_policy != "none":
        print("parent-after " + json.dumps(open_and_embed(parent_policy, "parent-after")), flush=True)


if __name__ == "__main__":
    main()
"""


def _run(
    tmp_path: Path, start_method: str, child_policy: str, parent_policy: str = "none", **env: str
) -> dict:
    environment = {
        key: value
        for key, value in os.environ.items()
        if key
        not in ("FATHOMDB_CUDA_EARLY_INIT", "FATHOMDB_EMBED_DEVICE", "FATHOMDB_RERANK_DEVICE")
    }
    environment.update(env)
    # A file, not `-c`: a spawned child re-imports the driver as __main__.
    driver = tmp_path / "fork_driver.py"
    driver.write_text(_DRIVER, encoding="utf-8")
    completed = subprocess.run(
        [sys.executable, str(driver), start_method, child_policy, parent_policy],
        env=environment,
        capture_output=True,
        text=True,
        timeout=600,
        check=False,
    )
    report = f"rc={completed.returncode}\n{completed.stdout}\n{completed.stderr}"
    assert completed.returncode == 0, report
    lines = {}
    for line in completed.stdout.splitlines():
        label, _, payload = line.partition(" ")
        if label in ("parent", "child", "parent-after"):
            lines[label] = json.loads(payload)
    assert "child" in lines, report
    assert lines["child"]["exitcode"] == 0, f"the child must end without a signal or hang: {report}"
    return lines


def test_forked_child_after_import_forced_cuda_raises_cuda_probe_failed(tmp_path: Path) -> None:
    """F1: the hook ran in the parent; a forked child forcing cuda:0 gets a typed error."""

    child = _run(tmp_path, "fork", "cuda:0")["child"]
    assert child["ok"] is False, child
    assert child["error"] == "EmbedDevicePolicyError", child
    assert child["kind"] == "cuda_probe_failed", child


def test_forked_child_with_early_init_off_embeds_on_cuda(tmp_path: Path) -> None:
    """F2: with the hook opted out, a forked child opens CUDA on the 0.8.27 path."""

    child = _run(tmp_path, "fork", "cuda:0", FATHOMDB_CUDA_EARLY_INIT="off")["child"]
    assert child["ok"] is True, child
    assert child["device"] == "cuda", child
    assert child["path"] in ("default_pool", "synchronous"), child
    assert child["allocator_reason"] == "cuinit_opted_out", child
    assert child["embedded"] is True, child


def test_forked_child_after_import_auto_falls_back_to_cpu(tmp_path: Path) -> None:
    """F3: under auto, a forked child after import runs on CPU with the probe reason."""

    child = _run(tmp_path, "fork", "auto")["child"]
    assert child["ok"] is True, child
    assert child["device"] == "cpu", child
    assert child["resolution_reason"] == "cuda_probe_failed", child
    assert child["embedded"] is True, child


def test_forked_child_after_parent_gpu_use_gets_a_typed_error_and_the_parent_still_embeds(
    tmp_path: Path,
) -> None:
    """F4: the parent used the private pool; the forked child fails typed; the parent embeds again.

    The product exposes no pool counters (Slice 30 deviation D-1), so the
    parent's state is checked by its allocator path and a second embed.
    """

    lines = _run(tmp_path, "fork", "cuda:0", parent_policy="cuda:0")
    assert lines["parent"]["path"] == "private", lines
    child = lines["child"]
    assert child["ok"] is False, child
    assert child["error"] == "EmbedDevicePolicyError", child
    after = lines["parent-after"]
    assert after["device"] == "cuda", after
    assert after["path"] == "private", after
    assert after["embedded"] is True, after


def test_spawned_child_embeds_on_the_private_pool(tmp_path: Path) -> None:
    """F5: a spawned child (a fresh interpreter) opens on the private pool and embeds."""

    lines = _run(tmp_path, "spawn", "cuda:0", parent_policy="cuda:0")
    child = lines["child"]
    assert child["ok"] is True, child
    assert child["device"] == "cuda", child
    assert child["path"] == "private", child
    assert child["embedded"] is True, child
