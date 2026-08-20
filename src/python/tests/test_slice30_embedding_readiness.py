"""Slice 30 public Python readiness/error contract through the real engine."""

from __future__ import annotations

from types import SimpleNamespace
from typing import Literal, cast

import pytest

from fathomdb import Engine, read
from fathomdb.errors import EmbedderRequiredError


def _body_edge(body: str) -> dict[str, object]:
    return {
        "edge": {
            "kind": "relates_to",
            "from": "memex-a",
            "to": "memex-b",
            "logical_id": "memex-edge-a-b",
            "source_id": "py-test:slice30-readiness",
            "body": body,
        }
    }


def test_embedding_readiness_and_immediate_error_are_typed_and_body_private(tmp_path) -> None:
    """Absent configuration is typed feedback, not a scheduler timeout or body leak."""

    secret = "slice30-private-edge-body-must-not-cross-readiness"
    engine = Engine.open(str(tmp_path / "readiness.sqlite"), use_default_embedder=False)
    try:
        engine.write([_body_edge(secret)])

        readiness = read.embedding_readiness(engine)
        assert readiness.state == "blocked"
        assert readiness.usable_embedder is False
        assert readiness.pending_count == 1
        assert readiness.affected_kinds == ("edge_fact",)
        assert readiness.code == "FDB_EMBEDDER_REQUIRED"
        assert readiness.operation == "graph_edge_body_projection"
        assert readiness.remediations == (
            "configure_default_embedder",
            "configure_caller_embedder",
            "submit_non_embedding_input",
        )
        assert readiness.documentation_url == "https://fathomdb.dev/errors/FDB_EMBEDDER_REQUIRED"
        assert secret not in repr(readiness)

        with pytest.raises(EmbedderRequiredError) as raised:
            engine.drain(timeout_s=30)
        error = raised.value
        _code: Literal["FDB_EMBEDDER_REQUIRED"] = error.code
        _operation: Literal["graph_edge_body_projection", "vector_projection"] = error.operation
        _state: Literal["blocked"] = error.state
        _remediations: list[str] = error.remediations
        _documentation_url: str = error.documentation_url
        assert error.code == "FDB_EMBEDDER_REQUIRED"
        assert error.operation == "graph_edge_body_projection"
        assert error.state == "blocked"
        assert error.remediations == list(readiness.remediations)
        assert error.documentation_url == readiness.documentation_url
        assert secret not in str(error)
        assert secret not in repr(error)
    finally:
        engine.close()


@pytest.mark.parametrize(
    ("payload", "message"),
    [
        ({"remediations": []}, "omitted its required payload"),
        ({"documentation_url": None}, "omitted its required payload"),
        ({"documentation_url": ""}, "omitted its required payload"),
        (
            {
                "state": "ready",
                "code": None,
                "operation": None,
                "remediations": [],
                "documentation_url": "",
            },
            "included a blocked payload",
        ),
    ],
)
def test_embedding_readiness_rejects_incomplete_or_spurious_native_payload(
    monkeypatch: pytest.MonkeyPatch, payload: dict[str, object], message: str
) -> None:
    """The native transport cannot smuggle a partial blocked payload into the SDK."""

    status = {
        "state": "blocked",
        "usable_embedder": False,
        "pending_count": 1,
        "affected_kinds": ["edge_fact"],
        "code": "FDB_EMBEDDER_REQUIRED",
        "operation": "graph_edge_body_projection",
        "remediations": ["configure_default_embedder"],
        "documentation_url": "https://fathomdb.dev/errors/FDB_EMBEDDER_REQUIRED",
    }
    status.update(payload)
    monkeypatch.setattr(
        read,
        "_native_read_embedding_readiness",
        lambda _native: SimpleNamespace(**status),
    )

    with pytest.raises(RuntimeError, match=message):
        read.embedding_readiness(cast(Engine, SimpleNamespace(_native=object())))
