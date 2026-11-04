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

