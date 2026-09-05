"""The governed ``read.*`` namespace (Slice 30 — G2 / G3; Slice 35 — G4).

Per ``dev/adr/ADR-0.8.0-supersede-five-verb-surface-cap.md`` (B1 ``read.*``), this
module exposes the governed read verbs beside ``admin``:

* ``read.get`` / ``read.get_many`` — active-only point lookup by ``logical_id``
  (active = ``superseded_at IS NULL``). Not-found is a normal ``None`` (a typed
  ``NotFound`` class is reserved for a later slice), never an exception.
* ``read.collection`` / ``read.mutations`` — paginated op-store read-back over
  ``operational_mutations`` with a MANDATORY ``limit`` + ``after_id`` cursor.
* ``read.list`` (G4 / Slice 35) — list active ``canonical_nodes`` of a given
  ``kind``, optionally filtered by a list of ``Predicate`` dicts (AND-combined),
  up to ``limit`` rows. Compiles to parameterized ``json_extract`` over the
  allowlisted path set (injection-safe per ADR D-F4).
* ``read.projection_status`` (0.8.22 Slice 22) — pure current dense-runtime
  status for every declared projection; distinct from the configuration-facing
  ``read.projections`` result.

The retrieval verbs use the native binding's ReaderWorkerPool DEFERRED-tx
path. ``read.projections`` and ``read.projection_status`` are instead pure
introspection queries through the ordinarily opened engine and may briefly take
its connection lock. Neither introspection query writes, configures, or
schedules work, but neither promises a separately opened read-only SQLite mode.
This module exposes the typed Python signatures and converts native rows to the
public dataclasses in ``fathomdb.types``.
"""

from __future__ import annotations

import builtins
from typing import TYPE_CHECKING, Any, Literal, cast

from fathomdb._fathomdb import NodeRecord as _NativeNodeRecord
from fathomdb._fathomdb import OpStoreRow as _NativeOpStoreRow
from fathomdb._fathomdb import read_collection as _native_collection
from fathomdb._fathomdb import read_get as _native_get
from fathomdb._fathomdb import read_get_many as _native_get_many
from fathomdb._fathomdb import read_list as _native_list
from fathomdb._fathomdb import read_list_filter as _native_list_filter
from fathomdb._fathomdb import read_canonical_page as _native_canonical_page
from fathomdb._fathomdb import read_operational_state as _native_operational_state
from fathomdb._fathomdb import (
    read_operational_state_page as _native_operational_state_page,
)
from fathomdb._fathomdb import read_mutations as _native_mutations
from fathomdb._fathomdb import crossed_boundary_since as _native_crossed_boundary_since
from fathomdb._fathomdb import read_projections as _native_read_projections
from fathomdb._fathomdb import read_projection_status as _native_read_projection_status
from fathomdb._fathomdb import (
    read_mutation_projection_status as _native_read_mutation_projection_status,
)
from fathomdb._fathomdb import (
    read_projection_generation_status as _native_read_projection_generation_status,
)
from fathomdb._fathomdb import read_embedding_readiness as _native_read_embedding_readiness
from fathomdb._fathomdb import ReadView as _NativeReadView
from fathomdb._fathomdb import ReadContextV1 as _NativeReadContextV1
from fathomdb._fathomdb import FrozenReadContextV1 as _NativeFrozenReadContextV1
from fathomdb._fathomdb import BoundaryCrossing as _NativeBoundaryCrossing
from fathomdb.errors import PageError, ProjectionGenerationError
from fathomdb.types import (
    BoundaryCrossing,
    EmbeddingOperation,
    FrozenReadContextV1,
    NodeRecord,
    OperationalStateRecordV1,
    OpStoreRow,
    PageRequestV1,
    PageV1,
    ProjectionRuntimeStatus,
    EmbeddingReadiness,
    EmbeddingReadinessState,
    ProjectionRuntimeStatusEntry,
    MutationProjectionStatusRequestV1,
    MutationProjectionStatusV1,
    ProjectionGenerationStatusV1,
    ProjectionGenerationOriginV1,
    ProjectionReadinessV1,
    ProjectionRuntimeStateV1,
    ProjectionSpec,
    ReadView,
)

_PROJECTION_ORIGINS = frozenset({"fresh", "legacy_unverified", "configuration", "rebuild"})
_PROJECTION_READINESS = frozenset({"ready", "processing", "blocked", "deferred", "degraded"})
_PROJECTION_RUNTIME_STATES = frozenset({"absent", "usable", "refused"})


def _projection_enum(value: str, allowed: frozenset[str], field_path: str) -> str:
    if value not in allowed:
        raise ProjectionGenerationError(
            f"projection generation projection_generation_corrupt at {field_path}",
            reason="projection_generation_corrupt",
            field_path=field_path,
        )
    return value


def projection_generation_status(engine: "Engine") -> ProjectionGenerationStatusV1:
    """Return the current serving projection generation and exact completeness."""

    value = _native_read_projection_generation_status(engine._native)
    if value.schema_version != 1:
        raise ProjectionGenerationError(
            "projection generation unsupported_schema_version at /schemaVersion",
            reason="unsupported_schema_version",
            field_path="/schemaVersion",
        )
    return ProjectionGenerationStatusV1(
        schema_version=value.schema_version,
        generation_id=value.generation_id,
        declaration_sha256=value.declaration_sha256,
        origin=cast(
            ProjectionGenerationOriginV1,
            _projection_enum(value.origin, _PROJECTION_ORIGINS, "/origin"),
        ),
        transition_boundary=value.transition_boundary,
        effective_at_epoch_s=value.effective_at_epoch_s,
        observed_boundary=value.observed_boundary,
        ready_through=value.ready_through,
        readiness=cast(
            ProjectionReadinessV1,
            _projection_enum(value.readiness, _PROJECTION_READINESS, "/readiness"),
        ),
        runtime_state=cast(
            ProjectionRuntimeStateV1,
            _projection_enum(value.runtime_state, _PROJECTION_RUNTIME_STATES, "/runtimeState"),
        ),
        pending_count=value.pending_count,
        failed_count=value.failed_count,
    )


def mutation_projection_status(
    engine: "Engine", request: MutationProjectionStatusRequestV1
) -> MutationProjectionStatusV1:
    """Return completion for one pending cursor from an actuation receipt."""

    value = _native_read_mutation_projection_status(
        engine._native, cast(dict[str, object], request)
    )
    if value.schema_version != 1:
        raise ProjectionGenerationError(
            "projection generation unsupported_schema_version at /schemaVersion",
            reason="unsupported_schema_version",
            field_path="/schemaVersion",
        )
    return MutationProjectionStatusV1(
        schema_version=value.schema_version,
        operation_id=value.operation_id,
        write_cursor=value.write_cursor,
        generation_id=value.generation_id,
        effective_at_epoch_s=value.effective_at_epoch_s,
        observed_boundary=value.observed_boundary,
        ready_through=value.ready_through,
        readiness=cast(
            ProjectionReadinessV1,
            _projection_enum(value.readiness, _PROJECTION_READINESS, "/readiness"),
        ),
        runtime_state=cast(
            ProjectionRuntimeStateV1,
            _projection_enum(value.runtime_state, _PROJECTION_RUNTIME_STATES, "/runtimeState"),
        ),
        pending_count=value.pending_count,
        failed_count=value.failed_count,
    )

if TYPE_CHECKING:
    from fathomdb.engine import Engine
    from fathomdb.filter import Filter


def _to_node_record(native: _NativeNodeRecord) -> NodeRecord:
    return NodeRecord(
        logical_id=native.logical_id,
        kind=native.kind,
        body=native.body,
        write_cursor=native.write_cursor,
    )


def _to_native_view(view: ReadView | None) -> _NativeReadView | None:
    """Translate the public dataclass to the native ``ReadView``.

    ``None`` stays ``None`` — the native layer then applies the strict default
    view, so an omitted ``view=`` is exactly the shipped behaviour.
    """

    if view is None:
        return None
    return _NativeReadView(
        include_superseded=view.include_superseded,
        include_inactive=view.include_inactive,
        include_out_of_window=view.include_out_of_window,
        valid_as_of=view.valid_as_of,
    )


def _to_native_frozen_context(context: FrozenReadContextV1) -> _NativeFrozenReadContextV1:
    eligibility = context.context.eligibility
    native_context = _NativeReadContextV1(
        view=_to_native_view(context.context.view),
        source_type=eligibility.source_type,
        kind=eligibility.kind,
        created_after=eligibility.created_after,
        status=eligibility.status,
        attributes=builtins.list(eligibility.attributes),
        schema_version=context.context.schema_version,
    )
    return _NativeFrozenReadContextV1(
        context.effective_valid_at,
        native_context,
        context.token,
        schema_version=context.schema_version,
    )


def _to_boundary_crossing(native: _NativeBoundaryCrossing) -> BoundaryCrossing:
    return BoundaryCrossing(
        node=_to_node_record(native.node),
        became_valid_at=native.became_valid_at,
        became_invalid_at=native.became_invalid_at,
    )


def _to_op_store_row(native: _NativeOpStoreRow) -> OpStoreRow:
    return OpStoreRow(
        id=native.id,
        collection=native.collection,
        record_key=native.record_key,
        op_kind=native.op_kind,
        payload=native.payload,
        schema_id=native.schema_id,
        write_cursor=native.write_cursor,
    )


def _to_operational_state_record(native: Any) -> OperationalStateRecordV1:
    _require_page_response_v1(native.schema_version)
    return OperationalStateRecordV1(
        schema_version=native.schema_version,
        collection=native.collection,
        record_key=native.record_key,
        payload=native.payload,
        schema_id=native.schema_id,
        write_cursor=native.write_cursor,
    )


def _require_page_response_v1(schema_version: int) -> None:
    if schema_version != 1:
        raise PageError(
            "unsupported_schema_version at /schemaVersion",
            reason="unsupported_schema_version",
            field_path="/schemaVersion",
        )


def get(engine: "Engine", logical_id: str, *, view: ReadView | None = None) -> NodeRecord | None:
    """Return the ACTIVE node carrying ``logical_id``, or ``None`` if absent.

    Active-only (``superseded_at IS NULL``): a superseded version is never
    returned. A missing/superseded id is a normal ``None``, not an exception.
    """

    if not logical_id:
        raise ValueError("read.get requires a non-empty logical_id")
    native = _native_get(engine._native, logical_id, _to_native_view(view))
    return _to_node_record(native) if native is not None else None


def get_many(
    engine: "Engine",
    logical_ids: builtins.list[str],
    *,
    view: ReadView | None = None,
) -> builtins.list[NodeRecord | None]:
    """Return one slot per requested id, in REQUEST ORDER.

    A missing/superseded id yields ``None`` in its slot (partial result, never
    all-or-nothing). Order is preserved 1:1 with ``logical_ids``.
    """

    natives = _native_get_many(engine._native, builtins.list(logical_ids), _to_native_view(view))
    return [_to_node_record(n) if n is not None else None for n in natives]


def collection(
    engine: "Engine",
    collection: str,
    *,
    after_id: int | None = None,
    limit: int,
) -> builtins.list[OpStoreRow]:
    """Paginated op-store read-back over ``operational_mutations``, ``ORDER BY id``.

    ``limit`` is MANDATORY (the engine clamps it to a ~1M cap, so no call yields
    an unbounded read); ``after_id`` is the exclusive cursor for the next page.
    """

    _validate_limit(limit)
    return [
        _to_op_store_row(row)
        for row in _native_collection(engine._native, collection, after_id, limit)
    ]


def mutations(
    engine: "Engine",
    collection: str,
    *,
    after_id: int | None = None,
    limit: int,
) -> builtins.list[OpStoreRow]:
    """Mutation-log-oriented alias surface over the same op-store read-back as
    :func:`collection` (identical args + semantics)."""

    _validate_limit(limit)
    return [
        _to_op_store_row(row)
        for row in _native_mutations(engine._native, collection, after_id, limit)
    ]


def list(  # noqa: A001 — shadows builtin; public API requires this name
    engine: "Engine",
    kind: str,
    predicates: builtins.list[builtins.dict[str, Any]] | None = None,
    *,
    limit: int = 100,
    filter: "Filter | None" = None,
    view: ReadView | None = None,
) -> builtins.list[NodeRecord]:
    """G4 (Slice 35) — list active ``canonical_nodes`` of the given ``kind``.

    ``predicates`` is an optional list of filter dicts (AND-combined). Each dict:
    ``{"type": "eq"|"gt"|"gte"|"lt"|"lte", "path": str, "value": str|int|bool}``.
    The ``path`` must be from the engine's allowlist (``$.status``, ``$.priority``,
    ``$.tags``, ``$.kind``, ``$.created_at``); non-allowlisted paths raise
    ``InvalidFilterError``. Values are always bound as parameterized SQL — never
    interpolated (injection-safe per ADR D-F4).

    Empty ``predicates`` (or ``None``) returns all active nodes of the kind up to
    ``limit`` (unfiltered path). ``limit`` defaults to 100.

    0.8.11 Slice 40 (#17): pass ``filter=`` (a unified :class:`fathomdb.Filter`)
    for the additive unified grammar instead of ``predicates``. The engine then
    performs the authoritative total dispatch (``Json`` → allowlisted
    ``json_extract``; ``Status``/``CreatedAfter`` → allowlisted json-paths;
    ``Kind``/``SourceType`` constant-fold vs the partition ``kind`` — a
    contradicting fold returns ``[]`` without touching SQL). ``predicates`` and
    ``filter`` are mutually exclusive. This stays the **same** governed
    ``read.list`` verb (no new surface member).
    """

    if limit < 0:
        raise ValueError("read.list limit must be non-negative")
    if filter is not None:
        if predicates:
            raise ValueError("read.list: pass either `predicates` or `filter`, not both")
        terms = filter.to_native_terms()
        rows = _native_list_filter(engine._native, kind, terms or None, limit, _to_native_view(view))
    else:
        rows = _native_list(engine._native, kind, predicates or None, limit, _to_native_view(view))
    return [_to_node_record(row) for row in rows]


def canonical_page(
    engine: "Engine",
    kind: str,
    context: FrozenReadContextV1,
    page: PageRequestV1,
) -> PageV1[NodeRecord]:
    """Read one stable page of canonical logical nodes under ``context``."""

    native = _native_canonical_page(
        engine._native,
        kind,
        _to_native_frozen_context(context),
        page.limit,
        page.cursor,
        page.schema_version,
    )
    _require_page_response_v1(native.schema_version)
    return PageV1(
        schema_version=native.schema_version,
        items=[_to_node_record(item) for item in native.items],
        next_cursor=native.next_cursor,
    )


def operational_state(
    engine: "Engine",
    collection: str,
    record_key: str,
    context: FrozenReadContextV1 | None = None,
) -> OperationalStateRecordV1 | None:
    """Read one current record from a governed ``latest_state`` collection."""

    native = _native_operational_state(
        engine._native,
        collection,
        record_key,
        None if context is None else _to_native_frozen_context(context),
    )
    return None if native is None else _to_operational_state_record(native)


def operational_state_page(
    engine: "Engine",
    collection: str,
    context: FrozenReadContextV1,
    page: PageRequestV1,
) -> PageV1[OperationalStateRecordV1]:
    """Read one stable page from a governed ``latest_state`` collection."""

    native = _native_operational_state_page(
        engine._native,
        collection,
        _to_native_frozen_context(context),
        page.limit,
        page.cursor,
        page.schema_version,
    )
    _require_page_response_v1(native.schema_version)
    return PageV1(
        schema_version=native.schema_version,
        items=[_to_operational_state_record(item) for item in native.items],
        next_cursor=native.next_cursor,
    )


def crossed_boundary_since(
    engine: "Engine",
    since: int,
    *,
    view: ReadView | None = None,
) -> builtins.list[BoundaryCrossing]:
    """R-20-NV — nodes that crossed a validity boundary in ``(since, as_of]``.

    ``since`` is an INTEGER epoch-second instant, and the upper bound is the
    view's own ``valid_as_of`` (defaulting to now). Both are bound parameters,
    so the answer is deterministic for a fixed pair.

    A node whose window opened AND closed inside the interval reports both
    boundaries. Rows with no window (every row predating schema step 22) can
    never cross one, so they never appear.

    World-time only — this asks what was true in the world, never what the
    database believed.
    """

    if not isinstance(since, int) or isinstance(since, bool):
        raise ValueError("read.crossed_boundary_since requires an integer `since`")
    rows = _native_crossed_boundary_since(engine._native, since, _to_native_view(view))
    return [_to_boundary_crossing(row) for row in rows]


def projections(engine: "Engine") -> builtins.list[ProjectionSpec]:
    """0.8.20 Slice 15d (R-20-PR) — ``read.projections`` introspection.

    Returns every declared :class:`ProjectionSpec` (sorted by name), so a caller
    can inspect current registry state — and the destructive delta a change would
    cause — BEFORE calling ``Engine.configure_projections``. This pure
    introspection query may briefly take the ordinarily opened engine connection
    lock; it is not a ReaderWorkerPool request and does not promise a separate
    read-only SQLite connection.
    """

    return [
        ProjectionSpec(
            name=s.name,
            roles=frozenset(s.roles),
            fts=s.fts,
            fts_tokenizer=s.fts_tokenizer,
            vector=s.vector,
            vector_embedder=s.vector_embedder,
            # 0.8.20 Slice 20 (R-20-DR) — engine-set readiness read metadata
            # (`"unavailable"` / `"embedding"` / `"ready"`; `None` when no
            # vector sub-object).
            vector_dense_readiness=s.vector_dense_readiness,
            source=tuple(s.source) if s.source is not None else None,
        )
        for s in _native_read_projections(engine._native)
    ]


def projection_status(engine: "Engine") -> ProjectionRuntimeStatus:
    """Return current dense-runtime facts for declared projections.

    This pure read is distinct from :func:`projections`: it reports the current
    session's usable-runtime state, per-declaration dense readiness, and the
    current declaration-scoped unsupported-kind report without re-applying any
    caller-owned configuration. It uses the ordinarily opened engine connection
    and may briefly take its connection lock; it is not a ReaderWorkerPool
    request and does not promise a separate read-only SQLite connection.
    """

    status = _native_read_projection_status(engine._native)
    return ProjectionRuntimeStatus(
        runtime_embedder_available=status.runtime_embedder_available,
        runtime_unavailability_reason=status.runtime_unavailability_reason,
        projections=tuple(
            ProjectionRuntimeStatusEntry(
                name=entry.name,
                dense_readiness=entry.dense_readiness,
            )
            for entry in status.projections
        ),
        vector_unsupported_kinds=tuple(status.vector_unsupported_kinds),
    )


def _embedding_readiness_state(value: str) -> EmbeddingReadinessState:
    if value == "ready":
        return "ready"
    if value == "processing":
        return "processing"
    if value == "deferred":
        return "deferred"
    if value == "blocked":
        return "blocked"
    raise RuntimeError(f"native embedding readiness returned unknown state {value!r}")


def _embedding_required_code(value: str | None) -> Literal["FDB_EMBEDDER_REQUIRED"] | None:
    if value is None or value == "FDB_EMBEDDER_REQUIRED":
        return value
    raise RuntimeError(f"native embedding readiness returned unknown code {value!r}")


def _embedding_operation(value: str | None) -> EmbeddingOperation | None:
    if value is None:
        return None
    if value == "graph_edge_body_projection":
        return value
    if value == "vector_projection":
        return value
    raise RuntimeError(f"native embedding readiness returned unknown operation {value!r}")


def embedding_readiness(engine: "Engine") -> EmbeddingReadiness:
    """Return pure typed readiness for pending embedding projection work."""
    status = _native_read_embedding_readiness(engine._native)
    state = _embedding_readiness_state(status.state)
    code = _embedding_required_code(status.code)
    operation = _embedding_operation(status.operation)
    if state == "blocked":
        if (
            code is None
            or operation is None
            or not status.remediations
            or not status.documentation_url
        ):
            raise RuntimeError("native blocked embedding readiness omitted its required payload")
    elif (
        code is not None
        or operation is not None
        or status.remediations
        or status.documentation_url is not None
    ):
        raise RuntimeError("native non-blocked embedding readiness included a blocked payload")
    return EmbeddingReadiness(
        state=state, usable_embedder=status.usable_embedder,
        pending_count=status.pending_count, affected_kinds=tuple(status.affected_kinds),
        code=code, operation=operation, remediations=tuple(status.remediations),
        documentation_url=status.documentation_url,
    )


def _validate_limit(limit: int) -> None:
    if not isinstance(limit, int) or isinstance(limit, bool):
        raise ValueError("read.collection/read.mutations require an integer limit")
    if limit < 0:
        raise ValueError("read.collection/read.mutations limit must be non-negative")


__all__ = [
    "get",
    "get_many",
    "collection",
    "mutations",
    "list",
    "canonical_page",
    "operational_state",
    "operational_state_page",
    "crossed_boundary_since",
    "projections",
    "projection_status",
    "projection_generation_status",
    "mutation_projection_status",
    "embedding_readiness",
]
