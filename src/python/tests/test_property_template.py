"""Bounded property checks for the installed Python Engine contract."""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

try:
    from hypothesis import given, settings, strategies as st  # pyright: ignore[reportMissingImports]
except ImportError:  # pragma: no cover
    pytest.skip("hypothesis not installed; install the test extra", allow_module_level=True)

from fathomdb import Engine


@settings(max_examples=12, deadline=None)
@given(
    token=st.text(alphabet="abcdefghijklmnopqrstuvwxyz", min_size=4, max_size=12),
    logical_suffix=st.text(
        alphabet="abcdefghijklmnopqrstuvwxyz0123456789", min_size=4, max_size=12
    ),
)
def test_written_record_identity_survives_reopen(token: str, logical_suffix: str) -> None:
    body = f"property durable {token}"
    logical_id = f"property-{logical_suffix}"
    source_id = f"property:{logical_suffix}"
    with tempfile.TemporaryDirectory() as root:
        path = str(Path(root) / "property.sqlite")
        engine = Engine.open(path, use_default_embedder=False)
        try:
            engine.write(
                [
                    {
                        "kind": "doc",
                        "body": body,
                        "logical_id": logical_id,
                        "source_id": source_id,
                    }
                ]
            )
            engine.drain(timeout_s=5)
            before = next(hit for hit in engine.search(token).results if hit.body == body)
        finally:
            engine.close()

        reopened = Engine.open(path, use_default_embedder=False)
        try:
            after = next(hit for hit in reopened.search(token).results if hit.body == body)
            assert after.id == before.id
            assert after.body == body
            assert after.source_id == source_id
        finally:
            reopened.close()
