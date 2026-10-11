"""Python wrapper around the native PyO3 engine handle.

`Engine` mirrors the public five-verb surface owned by
`dev/interfaces/python.md`. The native PyO3 class
(`fathomdb._fathomdb.Engine`) holds the `Arc<fathomdb_engine::Engine>`
and runs every blocking call under `py.allow_threads`; this Python
wrapper converts native return values into the dataclasses in
`fathomdb.types` and rejects unknown `open()` kwargs.
"""

from __future__ import annotations

import logging
from typing import Any

from fathomdb import (
    _sdk_evidence,
    _sdk_graph,
    _sdk_instrumentation,
    _sdk_open,
    _sdk_projection,
    _sdk_search,
    _sdk_write,
)
from fathomdb._fathomdb import ConsolidateReceipt, EraseReport, IngestWithExtractorReceipt
from fathomdb._fathomdb import Engine as _NativeEngine
from fathomdb._sdk_evidence import (
    _decode_dependency_trace_response as _decode_dependency_trace_response,
)  # noqa: F401

# Existing tests import these private converters from the facade.
from fathomdb._sdk_evidence import (
    _map_native_evidence_search as _map_native_evidence_search,  # noqa: F401
)
from fathomdb._sdk_evidence import (
    _map_native_resolved_evidence as _map_native_resolved_evidence,  # noqa: F401
)
from fathomdb._sdk_graph import (
    _map_native_graph_expand_result as _map_native_graph_expand_result,  # noqa: F401
)
from fathomdb._sdk_open import _KWARG_FIELDS, _native_engine_config
from fathomdb._sdk_open import _map_open_report as _map_open_report  # noqa: F401
from fathomdb._sdk_search import (
    _map_native_search_result as _map_native_search_result,  # noqa: F401
)
from fathomdb._sdk_search import _map_per_hit_explain as _map_per_hit_explain  # noqa: F401
from fathomdb.config import EngineConfig
from fathomdb.filter import Filter
from fathomdb.types import (
    ActuationBatchV1,
    ActuationReceiptV1,
    ClosureLookupV1,
    ClosureStatusV1,
    CounterSnapshot,
    DependencyDerivedLookupV1,
    DependencyListV1,
    DependencySourceLookupV1,
    DependencyTraceRequestV1,
    DependencyTraceResultV1,
    EvidenceResolveRequestV1,
    EvidenceSearchRequestV1,
    EvidenceSearchResultV1,
    FrozenReadContextV1,
    GraphEvidenceResolveRequestV1,
    OpenReport,
    ProjectionDelta,
    ProjectionSpec,
    ReadContextV1,
    ReadView,
    ResolvedEvidenceV1,
    ResolvedGraphEvidenceV1,
    SearchExpandResult,
    SearchFilter,
    SearchResult,
    SourceDependencyRegistrationV1,
    SourceDependencyV1,
    WriteReceipt,
)


class Engine:
    """Python handle that wraps the native PyO3 engine."""

    __slots__ = ("_native", "_path", "_config")

    def __init__(
        self,
        native: _NativeEngine,
        *,
        path: str,
        config: EngineConfig,
    ) -> None:
        self._native = native
        self._path = path
        self._config = config

    @classmethod
    def open(
        cls,
        path: str,
        *,
        config: EngineConfig | None = None,
        use_default_embedder: bool = False,
        **engine_config: Any,
    ) -> "Engine":
        """Open the database at `path`.

        Either `config` or per-knob keyword arguments may be supplied,
        but not both. Unknown keyword arguments are rejected.

        EU-6: ``use_default_embedder`` opts into the engine's pinned
        default embedder (``fathomdb-bge-small-en-v1.5``). On first use,
        weights are downloaded from HuggingFace and cached under
        ``~/.cache/fathomdb/embedders/``. The default (``False``) opens
        without an embedder; subsequent vector writes fail with
        ``EmbedderNotConfiguredError``. Caller-supplied custom embedders
        are deferred to a later release (see ``dev/interfaces/python.md``).
        """

        if config is not None and engine_config:
            raise ValueError(
                "Engine.open accepts either config= or per-knob keyword arguments, not both",
            )

        unknown = set(engine_config) - _KWARG_FIELDS
        if unknown:
            raise TypeError(
                f"Engine.open got unexpected keyword arguments: {sorted(unknown)!r}",
            )

        resolved = config if config is not None else EngineConfig(**engine_config)
        native_config = _native_engine_config(resolved)
        native = _NativeEngine.open(
            path, use_default_embedder=use_default_embedder, config=native_config
        )
        return cls(native, path=path, config=resolved)

    @property
    def path(self) -> str:
        return self._path

    @property
    def config(self) -> EngineConfig:
        return self._config

    def write(self, batch: list[Any] | None = None) -> WriteReceipt:
        """Write a batch of items.

        ``source_id`` is MANDATORY on every canonical item (0.8.20 R-20-E3) —
        a row written without it can never be erased by :meth:`erase_source`.

        A node item is ``{"kind", "body", "source_id", "logical_id"?, "state"?,
        "reason"?, "valid_from"?, "valid_until"?}``; an edge item is
        ``{"edge": {"kind", "from", "to", "source_id", ...}}``.

        ``valid_from`` / ``valid_until`` (0.8.20 Slice 15b, TC-34) author the
        node's WORLD-TIME validity window as INTEGER epoch SECONDS. The window
        is HALF-OPEN — ``valid_from`` is inclusive, ``valid_until`` is exclusive
        — and an omitted (or ``None``) bound means unbounded on that side, so
        omitting both (the default) makes the node valid at every instant. Read
        it back through the ``valid_as_of`` field of a
        :class:`~fathomdb.types.ReadView`, or ask which nodes crossed a boundary
        with :func:`fathomdb.read.crossed_boundary_since`.

        Because the window is half-open, ``valid_from >= valid_until`` describes
        a window no instant can satisfy; that pair raises
        ``WriteValidationError`` rather than being silently stored. A
        non-integer bound raises ``WriteValidationError`` too — it is never
        coerced (``True`` is rejected too, even though ``bool`` subclasses
        ``int``). One family for the whole write-validation boundary.

        **BREAKING (0.8.20 Slice 22, decision #18).** The unsatisfiable-window
        pair used to raise ``InvalidArgumentError`` carrying both bounds. It is
        now ``WriteValidationError``, and that error is **message-less** — the
        offending bounds are no longer recoverable from it, so validate the
        pair before calling.
        """
        return _sdk_write.write(self, batch)

    def register_source_dependency(
        self, request: SourceDependencyRegistrationV1
    ) -> SourceDependencyV1:
        """Register one pinned dependency; exact replay is a no-op success."""
        return _sdk_write.register_source_dependency(self, request)

    def actuate(self, request: ActuationBatchV1) -> ActuationReceiptV1:
        """Atomically apply a bounded set of caller-decided memory operations."""
        return _sdk_write.actuate(self, request)

    def dependencies_for_source(self, request: DependencySourceLookupV1) -> DependencyListV1:
        """Return at most 100 dependencies in stable derived-revision order."""
        return _sdk_write.dependencies_for_source(self, request)

    def dependency_for_derived(
        self, request: DependencyDerivedLookupV1
    ) -> SourceDependencyV1 | None:
        """Return the dependency for one derived revision, or ``None``."""
        return _sdk_write.dependency_for_derived(self, request)

    def read_dependency_closure(self, request: ClosureLookupV1) -> ClosureStatusV1 | None:
        """Return current closure status, or ``None`` for an absent opaque ID."""
        return _sdk_write.read_dependency_closure(self, request)

    def transition(self, logical_id: str, to_state: str, reason: str | None = None) -> None:
        """OPP-12 Phase-1 (0.8.19 Slice 10) — the ``transition`` lifecycle verb.

        Move a governed node between existence states per the engine-enforced
        legal-transition table: promote ``pending``→``active``, reject
        ``pending``→``deleted``, soft-delete ``active``→``deleted``, undelete
        ``deleted``→``active``. Promote/undelete CLEAR ``reason``;
        reject/soft-delete SET it (``reason`` is advisory, never engine-
        interpreted). Keys on the bare ``logical_id`` (``l:`` space only) — a
        non-``l:`` id raises ``NotLifecycleAddressableError``; an illegal move
        (``purged``/``pending`` targets, self-loops, an absent node) raises
        ``IllegalTransitionError`` with ``from_state``/``to_state``/``legal``.
        Thin pass-through (no client-side logic)."""
        _sdk_write.transition(self, logical_id, to_state, reason)

    def purge(self, logical_id: str) -> None:
        """OPP-12 Phase-1 (0.8.19 Slice 10) — the ``purge`` lifecycle verb.

        Irreversibly hard-erase a governed node across every row-owned target
        (all versions + FTS/vector shadows + touching edges, cascade-removed).
        A SEPARATE verb from ``transition`` (NOT a recovery-denylist name).
        Precondition: DELETED-FIRST (legal only from ``deleted``; else
        ``IllegalTransitionError``); IDEMPOTENT (purging an absent/already-purged
        id is a no-op success). Keys on the bare ``logical_id`` (``l:`` only) — a
        non-``l:`` id raises ``NotLifecycleAddressableError``. Thin pass-through."""
        _sdk_write.purge(self, logical_id)

    def erase_source(self, source_id: str) -> EraseReport:
        """0.8.20 (R-20-E4) — the ``erase_source`` lifecycle verb.

        Erase every canonical row carrying ``source_id``, together with its
        row-owned projections (FTS5, vec0, ``search_index_v2``), and finish the
        erasure at rest (telemetry redaction + WAL truncation).

        The COMPANION to :meth:`purge`, not a duplicate of it. ``purge``
        addresses a *governed* node by ``logical_id``; ``erase_source``
        addresses *anonymous* content — rows written with no ``logical_id``,
        which ``purge`` cannot reach at all. Together they make every canonical
        row erasable from the SDK alone, with no CLI on ``PATH``.

        Idempotent: erasing an absent or already-erased source is a zero-count
        success, so an interrupted erasure obligation can be retried without a
        pre-check.

        Raises ``WriteValidationError`` for an empty, whitespace-only or
        reserved (``_``-prefixed) ``source_id``. The engine's reserved
        namespace (``_engine:*`` substrate and the ``_legacy:pre-0.8.20``
        migration cohort) is reachable ONLY through the CLI recovery seam
        ``fathomdb recover --excise-source``; a single governed call against it
        would erase every pre-0.8.20 anonymous row.

        NOT a recovery verb: ``erase_source`` carries no REQ-054
        recovery-denylist name, so AC-041 is unaffected. Thin pass-through."""
        return _sdk_write.erase_source(self, source_id)

    def configure_projections(
        self,
        specs: list[ProjectionSpec],
        drop: list[str] | None = None,
    ) -> ProjectionDelta:
        """0.8.20 Slice 15d (R-20-PR / C-1) — the ``configure_projections`` verb.

        Declaratively apply projection declarations. The engine is the SOLE
        projection authority: it diffs ``specs`` against the durable registry and
        backfills the difference in ONE transaction. Cheap projections
        (``filterable``, ``searchable→FTS``) build same-transaction; ``rankable``
        and the ``searchable→vector`` sub-target are persisted-but-deferred (F9 /
        Slice 20).

        ``drop`` is EXPLICIT: omitting a live projection from ``specs`` does NOT
        drop it; removal requires naming it in ``drop``. A destructive change to a
        live projection (a role removal or a tokenizer/embedder change) that is
        NOT in ``drop`` raises ``ProjectionDestructiveError`` with the destructive
        delta — never silent data loss. Re-applying an unchanged spec returns a
        ``ProjectionDelta`` with ``unchanged=True``.

        Pair with :func:`fathomdb.read.projections` to inspect current state
        first. Thin pass-through."""
        return _sdk_projection.configure_projections(self, specs, drop)

    def embed(self, text: str) -> list[float]:
        """Embed ``text`` with the engine's pinned default embedder
        (``fathomdb-bge-small-en-v1.5``) and return the raw vector.

        Read-path primitive for callers that need vectors under the engine's
        own embedder identity (e.g. coverage-index clustering) rather than a
        parallel, possibly-divergent embedder. Raises
        ``EmbedderNotConfiguredError`` if the engine was opened without an
        embedder (``use_default_embedder=False``)."""
        return list(self._native.embed(text))

    def search(
        self,
        query: str,
        filter: SearchFilter | Filter | None = None,
        *,
        rerank_depth: int = 0,
        use_graph_arm: bool = False,
        alpha: float | None = None,
        pool_n: int | None = None,
        explain: bool = False,
        view: ReadView | None = None,
        limit: int = 10,
    ) -> SearchResult:
        """Hybrid search with optional CE reranking and optional graph-BFS arm.

        Args:
            query: Free-text search query.
            filter: Optional closed metadata filter (``SearchFilter``).
            rerank_depth: 0 (default) = soft-fallback / identity (no CE).
                N > 0 = rerank the top-N fused hits with the cross-encoder.
                Must be a non-negative integer. Negative values raise
                ``ValueError``.
            use_graph_arm: When ``True``, seed a BFS over temporal fact-edges
                from the top-10 fused hits and fuse reachable nodes as a third
                RRF arm (Slice 30 R3). Default ``False`` → byte-identical to
                the pre-Slice-30 two-arm pipeline.
            alpha: 0.8.5 (EXP-0) CE-blend weight, clamped to ``[0, 1]`` in the
                engine. ``None`` (default) ⇒ 0.3, the C6 factoid-guard default;
                ``1.0`` is the measured Mem0-parity config. Opt-in for the
                agentic-answer/memory path — the default protects naive lookups.
            pool_n: 0.8.5 (EXP-0) reranked-pool size. ``None`` (default) ⇒
                ``rerank_depth`` (preserves today's pool == depth semantics).
            view: 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — optional validity
                view, the same keyword the five read verbs take. ``None``
                (default) is the STRICT view: active-only, non-superseded, and
                valid AT QUERY TIME. ``ReadView(include_out_of_window=True)``
                returns hits whatever their ``[valid_from, valid_until)``
                window; ``ReadView(valid_as_of=t)`` evaluates validity at the
                bound instant ``t``.

                Only the VALIDITY axis is honoured here. The existence flags
                (``include_superseded`` / ``include_inactive``) raise
                ``InvalidArgumentError`` on the search path rather than being
                silently ignored: search hydrates from projection indexes that
                are not version-complete, so they have no truthful answer.
                Use ``read.list`` to enumerate history.

        Returns:
            ``SearchResult`` with RRF-fused (and optionally CE-reranked) hits.
            Each hit carries ``ce_score`` (the CE score for in-pool reranked
            hits, ``None`` otherwise).
        """
        return _sdk_search.search(
            self,
            query,
            filter,
            rerank_depth=rerank_depth,
            use_graph_arm=use_graph_arm,
            alpha=alpha,
            pool_n=pool_n,
            explain=explain,
            view=view,
            limit=limit,
        )

    def freeze_read_context(self, context: ReadContextV1) -> FrozenReadContextV1:
        """Mint a restart-stable context bound to this database's read state."""
        return _sdk_search.freeze_read_context(self, context)

    def trace_dependency(self, request: DependencyTraceRequestV1) -> DependencyTraceResultV1:
        """Trace one reciprocal registered dependency under a frozen context."""
        return _sdk_evidence.trace_dependency(self, request)

    def search_frozen(
        self,
        query: str,
        context: FrozenReadContextV1,
        *,
        rerank_depth: int = 0,
        use_graph_arm: bool = False,
        alpha: float = 0.3,
        pool_n: int | None = None,
        explain: bool = False,
        limit: int = 10,
    ) -> SearchResult:
        """Search under an Engine-authenticated frozen context.

        ``pool_n=None`` uses ``rerank_depth``; this changes the former frozen
        default of zero to match ordinary search and the other SDKs.
        ``explain=True`` returns a finalized non-empty correlation identity.
        """
        return _sdk_search.search_frozen(
            self,
            query,
            context,
            rerank_depth=rerank_depth,
            use_graph_arm=use_graph_arm,
            alpha=alpha,
            pool_n=pool_n,
            explain=explain,
            limit=limit,
        )

    def search_with_evidence(
        self,
        request: EvidenceSearchRequestV1,
    ) -> EvidenceSearchResultV1:
        """Attach one evidence reference per frozen hit.

        ``include_explanation=True`` finalizes the nested correlation identity.
        """
        return _sdk_evidence.search_with_evidence(self, request)

    def resolve_evidence(self, request: EvidenceResolveRequestV1) -> ResolvedEvidenceV1:
        """Resolve exact source bytes under an equivalent frozen context."""
        return _sdk_evidence.resolve_evidence(self, request)

    def resolve_graph_evidence(
        self, request: GraphEvidenceResolveRequestV1
    ) -> ResolvedGraphEvidenceV1:
        """Resolve one exact artifact disclosed by frozen graph expansion."""
        return _sdk_graph.resolve_graph_evidence(self, request)

    def search_expand_frozen(
        self,
        query: str,
        context: FrozenReadContextV1,
        depth: int,
        *,
        limit: int = 10,
    ) -> SearchExpandResult:
        """Search and expand while enforcing one frozen read context."""
        return _sdk_graph.search_expand_frozen(self, query, context, depth, limit=limit)

    def search_projected_text(
        self,
        query: str,
        name: str,
        filter: SearchFilter | None = None,
        *,
        view: ReadView | None = None,
        limit: int = 10,
    ) -> SearchResult:
        """Search one declared ``searchable`` property-FTS projection.

        The projection ``name`` is the public query key; its nested source path
        is never accepted from a query caller. This path does not body-scan,
        invoke vector search, or fuse scores.
        """
        return _sdk_search.search_projected_text(self, query, name, filter, view=view, limit=limit)

    def search_text_only(
        self, query: str, view: ReadView | None = None, *, limit: int = 10
    ) -> SearchResult:
        """0.8.18 Slice 5 (#5 vector-equivalence probe) — text-only / FTS-only search.

        Does NOT embed the query and NEVER raises
        ``VectorEquivalenceMismatchError``, so it stays serviceable when the engine
        opened in the degraded ``dense_disabled`` state (the D2 "keep FTS servable"
        contract). It does not invoke vector recall, CE reranking, or the graph
        arm. Matching node- and edge-body FTS candidates are deterministically
        body-deduplicated and ranked before ``limit`` is applied. For one
        immutable selection and effective validity time, smaller accepted limits
        are prefixes of larger limits; this does not extend to hybrid search.
        """
        return _sdk_search.search_text_only(self, query, view, limit=limit)

    def dense_disabled(self) -> bool:
        """0.8.18 Slice 5 (R-VEQ-6) — ``True`` iff the engine opened degraded.

        The open-time #5 self-check found a vector-equivalence divergence and every
        vector-dependent arm now refuses at query time with
        ``VectorEquivalenceMismatchError``. Mirrors ``OpenReport.dense_disabled``.
        """
        return _sdk_instrumentation.dense_disabled(self)

    def dense_disabled_reason(self) -> str | None:
        """0.8.18 Slice 5 (R-VEQ-6) — reason for the degraded state, or ``None``."""
        return _sdk_instrumentation.dense_disabled_reason(self)

    def vector_equivalence_refusal_count(self) -> int:
        """0.8.18 Slice 5 (R-VEQ-6) — count of query-time dense-arm refusals."""
        return _sdk_instrumentation.vector_equivalence_refusal_count(self)

    def enable_telemetry(self, sink_path: str) -> None:
        """0.8.8 Slice 15 (OPP-9) — enable opt-in local telemetry capture to a
        JSONL ``sink_path``. Off by default; local file only (no egress). Once
        enabled, each ``search`` records a query→result event keyed on the
        stable id, and ``record_feedback`` appends correlated agent labels.
        The query text and ``source_id`` are NEVER written (privacy, ADR §C)."""
        _sdk_instrumentation.enable_telemetry(self, sink_path)

    def last_telemetry_query_id(self) -> str | None:
        """0.8.8 Slice 15 — the most-recent captured ``query_id`` (for
        ``record_feedback``), or ``None`` when telemetry is off / no query has
        been captured yet."""
        return _sdk_instrumentation.last_telemetry_query_id(self)

    def record_feedback(
        self,
        query_id: str,
        relevant_ids: list[int],
        irrelevant_ids: list[int],
        label_source: str,
    ) -> None:
        """0.8.8 Slice 15 — attach agent relevance labels for a previously
        captured ``query_id``. ``relevant_ids`` / ``irrelevant_ids`` are the
        telemetry ``result_ids`` / ``write_cursor`` keys (the pre-0.8.19
        ``SearchHit.id`` space), NOT the post-C-2 typed ``SearchHit.id``;
        ``label_source`` is the caller-declared label origin (e.g.
        ``"agent:hermes"``). Raises when telemetry is off."""
        _sdk_instrumentation.record_feedback(
            self, query_id, relevant_ids, irrelevant_ids, label_source
        )

    def close(self) -> None:
        self._native.close()

    def drain(self, *, timeout_s: float | int = 0) -> None:
        """Block until in-flight writes drain or `timeout_s` elapses."""

        self._native.drain(timeout_s=float(timeout_s))

    def ingest_with_extractor(
        self,
        cmd: list[str],
        documents: list[dict[str, str]],
    ) -> IngestWithExtractorReceipt:
        """G11 (Slice 15) — BYO-LLM ingest via the fathomdb.extract.v1 protocol.

        ``cmd`` is argv (first element = program, rest = args).
        ``documents`` is a list of dicts with ``source_doc_id`` and ``body`` keys.
        """
        return _sdk_write.ingest_with_extractor(self, cmd, documents)

    def consolidate_with_provider(
        self,
        cmd: list[str],
        axes: list[dict[str, str]],
    ) -> ConsolidateReceipt:
        """0.8.12 Slice 15 (OPP-2) — consolidation / recency via a BYO-LLM
        harness speaking the ``fathomdb.consolidate.v1`` protocol.

        ``cmd`` is argv (first element = program, rest = args).
        ``axes`` is a list of dicts with ``subject_logical_id`` and ``relation``
        keys; each names one (subject, relation) cluster to consolidate.
        """
        return _sdk_write.consolidate_with_provider(self, cmd, axes)

    def open_report(self) -> OpenReport:
        """Return the structured open-time report captured at `Engine.open`.

        Shape D (locked HITL 2026-05-24): the report is exposed as an
        engine-attached accessor, not a return-shape change on
        `Engine.open`. Idempotent — repeat calls return the same data;
        the report is a snapshot from open time, not live state.
        """
        return _sdk_open.open_report(self)

    def counters(self) -> CounterSnapshot:
        return _sdk_instrumentation.counters(self)

    def set_profiling(self, *, enabled: bool) -> None:
        _sdk_instrumentation.set_profiling(self, enabled=enabled)

    def set_slow_threshold_ms(self, *, value: int) -> None:
        _sdk_instrumentation.set_slow_threshold_ms(self, value=value)

    def attach_logging_subscriber(
        self,
        logger: logging.Logger,
    ) -> None:
        """Deliver best-effort engine diagnostics to a caller-owned logger.

        Delivery uses a bounded queue. Slow or failing handlers cannot block a
        database operation; overload may drop records. The logger must support
        weak references and remain alive while records are wanted. A handler
        cannot call back into database work on its delivery thread.
        """
        _sdk_instrumentation.attach_logging_subscriber(self, logger)


__all__ = ["Engine"]
