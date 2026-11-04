"""Explicit erasure policy, queued intent and window parity on real SQLite files."""
from __future__ import annotations

import pytest

from fathomdb import Engine, EngineConfig


def test_persisted_policy_and_maintenance_handle(tmp_path):
    path = str(tmp_path / "policy.sqlite")
    engine = Engine.open(path, config=EngineConfig(physical_erasure_policy="explicit_maintenance"))
    try:
        assert engine.physical_erasure_policy() == "explicit_maintenance"
        handle = engine.begin_erasure_maintenance(100)
        try:
            with pytest.raises(Exception) as captured:
                engine.erase_source("test-source")
            assert captured.value.reason == "maintenance_busy"
        finally:
            assert handle.close() == "normal_with_queued_restrictions"
    finally:
        engine.close()
    reopened = Engine.open(path)
    try:
        assert reopened.physical_erasure_policy() == "explicit_maintenance"
        with pytest.raises(Exception) as captured:
            reopened.erase_source("test-source")
        assert captured.value.reason == "maintenance_required"
    finally:
        reopened.close()


def test_queued_intent_retry_status_and_withdrawal(tmp_path):
    engine = Engine.open(str(tmp_path / "queue.sqlite"), config=EngineConfig(physical_erasure_policy="explicit_maintenance"))
    selector = {"kind": "source_all_at_cut", "source_id": "test-source"}
    opaque = "0123456789abcdef0123456789abcdef"
    try:
        assert engine.queue_physical_erasure(selector, opaque) == {"outcome": "needs_maintenance_admission"}
        with engine.begin_erasure_maintenance(100) as handle:
            accepted = handle.queue_physical_erasure(selector, opaque)
            assert accepted["outcome"] == "accepted"
            assert accepted["logical_state"] == "unchanged_by_this_request"
        ticket = accepted["ticket"]
        assert engine.queue_physical_erasure(selector, opaque) == accepted
        status = engine.physical_erasure_status(ticket)
        assert status["phase"] == "queued"
        assert "source_id" not in status
        assert engine.withdraw_queued_erasure(ticket) == "withdrawn_before_removal"
        assert engine.physical_erasure_status(ticket)["phase"] == "withdrawn_before_removal"
    finally:
        engine.close()


def test_physical_complete_returns_receipt_after_actual_bytes_are_gone(tmp_path):
    import sqlite3

    path = tmp_path / "physical.sqlite"
    engine = Engine.open(str(path), config=EngineConfig(physical_erasure_policy="explicit_maintenance"))
    marker = "pythonphysicalmarker85"
    engine.write([
        {"kind": "doc", "body": marker, "source_id": "target-source"},
        {"kind": "doc", "body": "retained survivor", "source_id": "kept-source"},
    ])
    assert marker.encode() in (path.read_bytes() + path.with_name(path.name + "-wal").read_bytes())
    try:
        with engine.begin_erasure_maintenance(1000) as handle:
            accepted = handle.queue_physical_erasure({"kind": "source_all_at_cut", "source_id": "target-source"}, "f23e2130b50242e3b8d545d4fa3d8015")
            result = handle.complete_physical_erasure(accepted["ticket"])
            assert result["outcome"] == "complete"
            assert result["receipt"]["logical_state"] == "removed"
            assert result["receipt"]["phase"] == "complete"
        with sqlite3.connect(path) as reader:
            assert reader.execute("SELECT source_id, body FROM canonical_nodes").fetchall() == [("kept-source", "retained survivor")]
        for artifact in [path, path.with_name(path.name + "-wal")]:
            if artifact.exists():
                assert marker.encode() not in artifact.read_bytes()
        assert engine.physical_erasure_status(accepted["ticket"])["phase"] == "complete"
    finally:
        engine.close()


@pytest.mark.requires_test_hooks
def test_cancelled_caller_keeps_native_ownership_until_durable_completion(tmp_path):
    import subprocess
    import sys
    from fathomdb import _fathomdb

    if not hasattr(_fathomdb.ErasureMaintenance, "_arm_erasure_pause_for_test"):
        pytest.skip("requires dev-only native erasure rendezvous")
    script = r'''
import asyncio, contextlib, gc, sys, time
from fathomdb import Engine, EngineConfig
from fathomdb.errors import MaintenanceError

async def main():
    engine = Engine.open(sys.argv[1], config=EngineConfig(physical_erasure_policy="explicit_maintenance"))
    engine.write([{"kind":"doc", "body":"cancelledcallermarker85", "source_id":"target-source"}])
    handle = engine.begin_erasure_maintenance(1000)
    accepted = handle.queue_physical_erasure({"kind":"source_all_at_cut", "source_id":"target-source"}, "f23e2130b50242e3b8d545d4fa3d8015")
    control = handle._native._arm_erasure_pause_for_test()
    task = asyncio.create_task(asyncio.to_thread(handle.complete_physical_erasure, accepted["ticket"]))
    try:
        await asyncio.to_thread(control.wait_entered)
        task.cancel()
        with contextlib.suppress(asyncio.CancelledError):
            await task
        del handle
        gc.collect()
    finally:
        control.release()
    deadline = time.monotonic() + 15
    while True:
        try:
            status = engine.physical_erasure_status(accepted["ticket"])
            if status["phase"] == "complete":
                assert status["logical_state"] == "removed"
                break
        except MaintenanceError as error:
            assert error.reason == "maintenance_busy"
        assert time.monotonic() < deadline, "native operation lost after caller cancellation"
        await asyncio.sleep(0.01)
    engine.close()
    print("cancelled-caller-complete")
asyncio.run(main())
'''
    result = subprocess.run([sys.executable, "-c", script, str(tmp_path / "drop.sqlite")], capture_output=True, text=True, timeout=25)
    assert result.returncode == 0, result.stdout + result.stderr
    assert "cancelled-caller-complete" in result.stdout
