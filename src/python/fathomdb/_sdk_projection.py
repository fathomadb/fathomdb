"""Private projection operations for the Python Engine facade."""

from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING

from fathomdb._fathomdb import ProjectionSpec as _NativeProjectionSpec
from fathomdb._fathomdb import ProjectionTarget as _NativeProjectionTarget
from fathomdb._fathomdb import configure_projections as _native_configure_projections
from fathomdb.types import (
    ProjectionDelta,
    ProjectionSpec,
    ProjectionTarget,
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


def _native_drop_targets(drop: object) -> list[_NativeProjectionTarget] | None:
    """Validate strictly typed ``ProjectionTarget`` entries (never bare ``str``)."""
    if drop is None:
        return None
    if isinstance(drop, (str, bytes)) or not isinstance(drop, Sequence):
        raise TypeError("drop must be a sequence of ProjectionTarget instances")
    native_targets: list[_NativeProjectionTarget] = []
    for entry in drop:
        if not isinstance(entry, ProjectionTarget):
            raise TypeError(
                "drop entries must be ProjectionTarget instances, "
                f"got {type(entry).__name__}"
            )
        if not isinstance(entry.name, str):
            raise TypeError(
                f"ProjectionTarget.name must be str, got {type(entry.name).__name__}"
            )
        if not isinstance(entry.entity_class, str):
            raise TypeError(
                "ProjectionTarget.entity_class must be str, "
                f"got {type(entry.entity_class).__name__}"
            )
        native_targets.append(_NativeProjectionTarget(entry.name, entry.entity_class))
    return native_targets


def configure_projections(
    self: Engine,
    specs: list[ProjectionSpec],
    drop: list[ProjectionTarget] | None = None,
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
            s.entity_class,
            list(s.vector_kinds) if s.vector_kinds is not None else None,
        )
        for s in specs
    ]
    native_drop = _native_drop_targets(drop)
    delta = _native_configure_projections(self._native, native_specs, native_drop)
    return ProjectionDelta(
        built=[
            ProjectionTarget(name=t.name, entity_class=t.entity_class)
            for t in delta.built
        ],
        dropped=[
            ProjectionTarget(name=t.name, entity_class=t.entity_class)
            for t in delta.dropped
        ],
        deferred=[
            ProjectionTarget(name=t.name, entity_class=t.entity_class)
            for t in delta.deferred
        ],
        unchanged=delta.unchanged,
        # 0.8.20 Slice 22 (R-20-VC / TC-67) — the typed report that replaces
        # the silent drop of a kind the vector writer can never commit.
        vector_unsupported_kinds=list(delta.vector_unsupported_kinds),
    )
