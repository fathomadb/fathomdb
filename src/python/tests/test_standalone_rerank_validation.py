"""Standalone rerank argument validation at the Python SDK boundary."""

from __future__ import annotations

import pytest

from fathomdb import rerank


@pytest.mark.parametrize("alpha", [float("nan"), float("inf"), float("-inf")])
def test_rerank_rejects_nonfinite_alpha_on_identity_path(alpha: float) -> None:
    with pytest.raises(ValueError, match="alpha must be finite"):
        rerank("query", [], 0, alpha=alpha)
