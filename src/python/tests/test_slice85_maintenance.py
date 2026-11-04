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
