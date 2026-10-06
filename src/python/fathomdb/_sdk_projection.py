"""Private projection operations for the Python Engine facade."""

from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING

from fathomdb._fathomdb import ProjectionSpec as _NativeProjectionSpec
from fathomdb._fathomdb import configure_projections as _native_configure_projections
from fathomdb.types import (
    ProjectionDelta,
    ProjectionSpec,
)

if TYPE_CHECKING:
    from fathomdb.engine import Engine


def _projection_source_segments(source: object) -> list[str] | None:
    """Validate and normalize a nested projection's literal member path."""
    if source is None:
        return None
    if isinstance(source, str) or not isinstance(source, Sequence):
        raise TypeError("ProjectionSpec.source must be a non-string sequence of strings")
    if not all(isinstance(segment, str) for segment in source):
        raise TypeError("ProjectionSpec.source must be a non-string sequence of strings")
    return list(source)


def configure_projections(
    self: Engine,
    specs: list[ProjectionSpec],
    drop: list[str] | None = None,
) -> ProjectionDelta:
    native_specs = [
        _NativeProjectionSpec(
            s.name,
            list(s.roles),
            s.fts,
            s.fts_tokenizer,
            s.vector,
            s.vector_embedder,
            # 0.8.20 Slice 20 (R-20-DR) — the engine-set readiness field is
            # carried ACROSS rather than dropped here, so the binding's
            # round-trip gate sees what the caller actually sent (a readiness
            # with ``vector=False``, or an unknown spelling, is refused).
            # Its VALUE is inert engine-side, which is what keeps
            # ``read.projections`` output re-appliable as a no-op.
            s.vector_dense_readiness,
            _projection_source_segments(s.source),
        )
        for s in specs
    ]
    delta = _native_configure_projections(self._native, native_specs, drop)
    return ProjectionDelta(
        built=list(delta.built),
        dropped=list(delta.dropped),
        deferred=list(delta.deferred),
        unchanged=delta.unchanged,
        # 0.8.20 Slice 22 (R-20-VC / TC-67) — the typed report that replaces
        # the silent drop of a kind the vector writer can never commit.
        vector_unsupported_kinds=list(delta.vector_unsupported_kinds),
    )
