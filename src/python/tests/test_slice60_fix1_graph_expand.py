"""Slice 60 FIX-1 RED for binding-local recursive graph request validation.

This is source-contract coverage because the durable worktree intentionally
retains a stale native extension; importing the package would fail before the
Python wrapper can be exercised. GREEN replaces this with a fresh artifact
consumer route once the binding-local validator exists.
"""

from pathlib import Path


GRAPH_MODULE = Path(__file__).parents[1] / "fathomdb" / "graph.py"


def test_graph_expand_has_a_recursive_pretransport_string_and_eligibility_validator() -> None:
    source = GRAPH_MODULE.read_text(encoding="utf-8")
    assert "def _validate_graph_expand_request(" in source
    assert "embedded NUL" in source
    assert "lone surrogate" in source
    assert "graph_edge_kinds_invalid" in source
    assert "graph_context_invalid" in source
    assert "_validate_graph_expand_request(request)" in source
