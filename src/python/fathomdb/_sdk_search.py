"""Private search operations for the Python Engine facade."""

from __future__ import annotations

import math
import re
from collections.abc import Sequence
from dataclasses import replace
from typing import TYPE_CHECKING, Any, NoReturn, cast

from fathomdb._fathomdb import FrozenReadContextV1 as _NativeFrozenReadContextV1
from fathomdb._fathomdb import ReadContextV1 as _NativeReadContextV1
from fathomdb._fathomdb import snippet_for as _native_snippet_for
from fathomdb.errors import (
    FrozenReadError,
    InvalidArgumentError,
)
from fathomdb.filter import Filter

# 0.8.20 Slice 15b fix-2 — reuse the read namespace's dataclass -> native
# ReadView translator rather than duplicating it here, so the two search entry
# points can never drift from the five read verbs. `fathomdb.read` imports
# `fathomdb.engine` only under TYPE_CHECKING, so this is not circular at runtime.
from fathomdb.read import _to_native_view
from fathomdb.types import (
    ArmDegradation,
    DegradationReason,
    Explanation,
    FrozenReadContextV1,
    IdSpace,
    NodeRecord,
    PerHitExplain,
    QueryTrace,
    ReadContextV1,
    ReadView,
    SearchFilter,
    SearchHit,
    SearchResult,
    SnippetOptions,
    SnippetV1,
    SoftFallback,
    SoftFallbackBranch,
    StructuralInclusionV1,
)

if TYPE_CHECKING:
    from fathomdb.engine import Engine


def _map_native_search_result(result: Any) -> SearchResult:
    fallback = result.soft_fallback
    soft = (
        SoftFallback(branch=cast(SoftFallbackBranch, fallback.branch))
        if fallback is not None
        else None
    )
    native_exp = result.explanation
    explanation = (
        _map_candidate_native_explanation(native_exp, result.results)
        if native_exp is not None
        else None
    )
    degradations = _map_native_degradations(result)
    return SearchResult(
        projection_cursor=result.projection_cursor,
        soft_fallback=soft,
        results=[
            SearchHit(
                id=IdSpace(space=hit.id.space, value=hit.id.value),
                kind=hit.kind,
                body=hit.body,
                score=hit.score,
                branch=cast(SoftFallbackBranch, hit.branch),
                source_id=hit.source_id,
                ce_score=hit.ce_score,
            )
            for hit in result.results
        ],
        explanation=explanation,
        degradations=degradations,
        snippets=_map_native_snippets(result),
    )


def snippet_for(
    body: str,
    spans: Sequence[tuple[int, int]],
    options: SnippetOptions | None = None,
) -> SnippetV1:
    """EP-17 pure snippet selector, the same function search uses.

    ``spans`` are match spans as UTF-8 byte ranges ``(start, end)`` into
    ``body``, so ``body.encode("utf-8")[start:end].decode("utf-8")`` is the
    matched text; spans that are empty, run past the body or split a
    character are ignored. ``options`` defaults to ``SnippetOptions()``; values out of
    range are clamped, so this never raises for a well-typed call. No
    database is involved.

    Raises:
        TypeError: ``body`` is not a str, a span is not a pair of ints, or
            ``options`` is not a ``SnippetOptions``.
    """
    if not isinstance(body, str):
        raise TypeError(f"body must be a str, got {type(body).__name__!r}")
    native_spans: list[tuple[int, int]] = []
    for span in spans:
        if (
            not isinstance(span, (tuple, list))
            or len(span) != 2
            or not all(isinstance(v, int) and not isinstance(v, bool) for v in span)
        ):
            raise TypeError(f"each span must be a (start, end) pair of ints, got {span!r}")
        # A span past u32 cannot lie inside a body; it is ignored like any
        # span that runs past the body.
        if all(0 <= v <= _U32_MAX for v in span):
            native_spans.append((span[0], span[1]))
    if options is None:
        options = SnippetOptions()
    _check_snippet_type(options)
    # The native layer reads i64; saturating there keeps clamping exact.
    clamped = replace(
        options,
        **{name: _clamp_i64(getattr(options, name)) for name in _SNIPPET_INT_FIELDS},
    )
    return _map_native_snippet(_native_snippet_for(body, native_spans, clamped))


_U32_MAX = 2**32 - 1
_I64_MIN = -(2**63)
_I64_MAX = 2**63 - 1
_SNIPPET_INT_FIELDS = ("max_bytes", "max_windows", "head_bytes", "max_point_queries")


def _clamp_i64(value: int) -> int:
    return max(_I64_MIN, min(_I64_MAX, value))


def _check_snippet_type(snippet: object) -> None:
    if not isinstance(snippet, (SnippetOptions, type(None))):
        raise TypeError(f"snippet must be a SnippetOptions or None, got {type(snippet).__name__!r}")


def _validate_snippet(snippet: object) -> None:
    """Search-side check: the type, then a refusal for ints the native
    layer cannot carry (every field range lies far inside i64)."""
    _check_snippet_type(snippet)
    if snippet is None:
        return
    for name in _SNIPPET_INT_FIELDS:
        value = getattr(snippet, name)
        if not _I64_MIN <= value <= _I64_MAX:
            raise InvalidArgumentError(f"snippet {name} is out of range, got {value}")


def _map_native_snippets(result: Any) -> list[SnippetV1] | None:
    """A native result's EP-17 snippet sidecar, or ``None`` when absent."""
    native_snippets = getattr(result, "snippets", None)
    if native_snippets is None:
        return None
    return [_map_native_snippet(entry) for entry in native_snippets]


def _map_native_snippet(entry: Any) -> SnippetV1:
    return SnippetV1(
        text=entry.text,
        anchor=entry.anchor,
        windows=[(int(a), int(b)) for a, b in entry.windows],
        spans=[(int(a), int(b)) for a, b in entry.spans],
        highlights=[(int(a), int(b)) for a, b in entry.highlights],
        cut=entry.cut,
    )


def _map_native_degradations(result: Any) -> list[ArmDegradation] | None:
    """A native result's degradation list, or ``None`` when absent or empty."""
    native_degradations = getattr(result, "degradations", None)
    if not native_degradations:
        return None
    return [
        ArmDegradation(
            arm=cast(SoftFallbackBranch, entry.arm),
            reason=cast(DegradationReason, entry.reason),
        )
        for entry in native_degradations
    ]


def _to_native_read_context(context: ReadContextV1) -> Any:
    eligibility = context.eligibility
    return _NativeReadContextV1(
        view=_to_native_view(context.view),
        source_type=eligibility.source_type,
        kind=eligibility.kind,
        created_after=eligibility.created_after,
        status=eligibility.status,
        attributes=list(eligibility.attributes),
        schema_version=context.schema_version,
    )


def _to_native_frozen_context(context: FrozenReadContextV1) -> Any:
    return _NativeFrozenReadContextV1(
        context.effective_valid_at,
        _to_native_read_context(context.context),
        context.token,
        schema_version=context.schema_version,
    )


def _map_native_search_hit(hit: Any) -> SearchHit:
    return SearchHit(
        id=IdSpace(space=hit.id.space, value=hit.id.value),
        kind=hit.kind,
        body=hit.body,
        score=hit.score,
        branch=cast(SoftFallbackBranch, hit.branch),
        source_id=hit.source_id,
        ce_score=hit.ce_score,
    )


def _map_native_node(node: Any) -> NodeRecord:
    return NodeRecord(
        logical_id=node.logical_id,
        kind=node.kind,
        body=node.body,
        write_cursor=node.write_cursor,
    )


def _validate_ranked_result_limit(name: str, limit: object) -> int:
    """Return a public ranked-result limit or raise the SDK's typed error."""
    if not isinstance(limit, int) or isinstance(limit, bool):
        raise TypeError(f"{name} must be an integer in 1..=100, got {type(limit).__name__!r}")
    if not 1 <= limit <= 100:
        raise InvalidArgumentError(f"{name} must be an integer in 1..=100, got {limit!r}")
    return limit


def _validate_id_list(name: str, value: object) -> list[int]:
    """0.8.8 Slice 15 — validate a relevance-label id list before the native
    call (mirrors the TS ``validateIdArray`` guard for cross-SDK parity). Ids
    are non-negative ints — the telemetry ``result_ids`` / ``write_cursor`` key
    space (the pre-0.8.19 ``SearchHit.id``), NOT the post-C-2 typed
    ``SearchHit.id``. ``bool`` is rejected explicitly (it is an int subclass that
    PyO3 would otherwise coerce silently)."""
    if not isinstance(value, list):
        raise TypeError(f"{name} must be a list of non-negative ints, got {type(value).__name__!r}")
    for item in value:
        if not isinstance(item, int) or isinstance(item, bool):
            raise TypeError(
                f"{name} must contain only non-negative ints, got {type(item).__name__!r}"
            )
        if item < 0:
            raise ValueError(f"{name} must contain only non-negative ints, got {item!r}")
    return value


def _map_per_hit_explain(p: Any) -> PerHitExplain:
    """Map one native per-hit explain object into the public
    :class:`fathomdb.types.PerHitExplain` dataclass.

    Factored out of :meth:`Engine.search` so the mapping is unit-testable
    against a fake native per-hit object without the compiled ``_fathomdb``
    extension (0.8.16 Slice 5 / F9, codex §9 fix-2). ``importance``/``confidence``
    are the additive F9 fields (node importance / edge confidence applied to this
    hit's contribution; ``None`` = graceful-absent / neutral), symmetric with the
    TypeScript ``perHit`` mapping.
    """

    def require_u64(value: object, path: str, *, optional: bool = False) -> None:
        if optional and value is None:
            return
        if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 2**64 - 1:
            _invalid_explanation(path)

    def require_u32(value: object, path: str) -> None:
        if value is None:
            return
        if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 2**32 - 1:
            _invalid_explanation(path)

    def require_finite(value: object, path: str, *, optional: bool = False) -> None:
        if optional and value is None:
            return
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
        ):
            _invalid_explanation(path)

    require_u64(getattr(p, "id", None), "/id")
    arm = getattr(p, "arm", None)
    if arm not in ("vector", "text", "text_edge", "graph_arm"):
        _invalid_explanation("/arm")
    require_u32(getattr(p, "vector_rank", None), "/vectorRank")
    require_u32(getattr(p, "text_rank", None), "/textRank")
    require_u32(getattr(p, "graph_rank", None), "/graphRank")
    require_finite(getattr(p, "fused_score", None), "/fusedScore")
    require_finite(getattr(p, "ce_score", None), "/ceScore", optional=True)
    require_finite(getattr(p, "blended", None), "/blended")
    require_finite(getattr(p, "importance", None), "/importance", optional=True)
    require_finite(getattr(p, "confidence", None), "/confidence", optional=True)
    native_structural = getattr(p, "structural", None)
    structural = (
        _map_structural_explanation(native_structural) if native_structural is not None else None
    )
    return PerHitExplain(
        id=p.id,
        arm=cast(SoftFallbackBranch, p.arm),
        vector_rank=p.vector_rank,
        text_rank=p.text_rank,
        graph_rank=p.graph_rank,
        fused_score=p.fused_score,
        ce_score=p.ce_score,
        blended=p.blended,
        importance=p.importance,
        confidence=p.confidence,
        structural=structural,
    )


def _map_structural_explanation(value: Any) -> StructuralInclusionV1:
    if getattr(value, "schema_version", None) != 1:
        _invalid_explanation("/structural/schemaVersion")
    inclusion = getattr(value, "inclusion_state", None)
    if inclusion not in ("included", "degraded"):
        _invalid_explanation("/structural/inclusionState")
    projection = getattr(value, "projection_origin", None)
    if projection not in (
        "synchronous_body_fts",
        "current_dense_generation",
        "graph_traversal",
    ):
        _invalid_explanation("/structural/projectionOrigin")
    dependency = getattr(value, "dependency_state", None)
    if dependency not in ("not_applicable", "not_registered", "registered"):
        _invalid_explanation("/structural/dependencyState")
    lifecycle = getattr(value, "lifecycle_state", None)
    if lifecycle not in ("node_pending", "node_active", "node_deleted", "edge_valid"):
        _invalid_explanation("/structural/lifecycleState")
    codes = getattr(value, "degradation_codes", None)
    if not isinstance(codes, (list, tuple)):
        _invalid_explanation("/structural/degradationCodes")
    order = (
        "soft_fallback_text",
        "soft_fallback_text_edge",
        "projection_legacy_unverified",
        "projection_blocked",
        "projection_deferred",
        "graph_bound_reached",
    )
    previous = -1
    for index, code in enumerate(codes):
        if code not in order:
            _invalid_explanation(f"/structural/degradationCodes/{index}")
        ordinal = order.index(code)
        if ordinal <= previous:
            _invalid_explanation(f"/structural/degradationCodes/{index}")
        previous = ordinal
    if (inclusion == "included") != (len(codes) == 0):
        _invalid_explanation("/structural/inclusionState")
    return StructuralInclusionV1(
        schema_version=1,
        inclusion_state=inclusion,
        projection_origin=projection,
        dependency_state=dependency,
        lifecycle_state=lifecycle,
        degradation_codes=tuple(codes),
    )


def _invalid_explanation(path: str) -> NoReturn:
    raise ValueError(f"invalid explanation response at {path}")


def _map_candidate_native_explanation(value: Any, native_results: Sequence[Any]) -> Explanation:
    def require_u32(candidate: object, path: str, *, positive: bool = False) -> int:
        if (
            not isinstance(candidate, int)
            or isinstance(candidate, bool)
            or not 0 <= candidate <= 2**32 - 1
            or (positive and candidate == 0)
        ):
            _invalid_explanation(path)
        return candidate

    trace = getattr(value, "trace", None)
    if trace is None:
        _invalid_explanation("/trace")
    query_chars = require_u32(getattr(trace, "query_chars", None), "/trace/queryChars")
    k = require_u32(getattr(trace, "k", None), "/trace/k", positive=True)
    if k > 100:
        _invalid_explanation("/trace/k")
    rerank_depth = require_u32(getattr(trace, "rerank_depth", None), "/trace/rerankDepth")
    pool_n = require_u32(getattr(trace, "pool_n", None), "/trace/poolN")
    alpha = getattr(trace, "alpha", None)
    if isinstance(alpha, bool) or not isinstance(alpha, (int, float)) or not math.isfinite(alpha):
        _invalid_explanation("/trace/alpha")
    for name, path in (
        ("use_graph_arm", "/trace/useGraphArm"),
        ("recency", "/trace/recency"),
        ("ce_active", "/trace/ceActive"),
    ):
        if not isinstance(getattr(trace, name, None), bool):
            _invalid_explanation(path)
    embedder_id = getattr(trace, "embedder_id", None)
    if not isinstance(embedder_id, str):
        _invalid_explanation("/trace/embedderId")
    # Additive 0.8.28 field: an older native trace without it maps to "".
    stop_words_sha256 = getattr(trace, "stop_words_sha256", "")
    if not isinstance(stop_words_sha256, str):
        _invalid_explanation("/trace/stopWordsSha256")
    counts = [
        require_u32(getattr(trace, name, None), path)
        for name, path in (
            ("vector_hits", "/trace/vectorHits"),
            ("text_hits", "/trace/textHits"),
            ("graph_hits", "/trace/graphHits"),
            ("dropped_edge_hits", "/trace/droppedEdgeHits"),
        )
    ]
    correlation = getattr(value, "correlation_id", "")
    if hasattr(value, "correlation_id") and (
        not isinstance(correlation, str)
        or not re.fullmatch(r"(?:q[0-9]+|x[0-9a-f]{32})-(?:0|[1-9][0-9]*)", correlation)
    ):
        _invalid_explanation("/correlationId")
    native_per_hit = getattr(value, "per_hit", None)
    if not isinstance(native_per_hit, (list, tuple)) or len(native_per_hit) != len(native_results):
        _invalid_explanation("/perHit")
    mapped: list[PerHitExplain] = []
    seen_ids: set[int] = set()
    allowed_arms = ("vector", "text", "text_edge", "graph_arm")
    for index, (native_explain, native_hit) in enumerate(zip(native_per_hit, native_results)):
        prefix = f"/perHit/{index}"
        try:
            item = _map_per_hit_explain(native_explain)
        except ValueError as error:
            marker = "invalid explanation response at "
            suffix = str(error).removeprefix(marker)
            _invalid_explanation(f"{prefix}{suffix}")
        if item.id in seen_ids:
            _invalid_explanation(f"{prefix}/id")
        seen_ids.add(item.id)
        hit_branch = getattr(native_hit, "branch", None)
        if hit_branch not in allowed_arms:
            _invalid_explanation(f"/results/{index}/branch")
        hit_score = getattr(native_hit, "score", None)
        if (
            isinstance(hit_score, bool)
            or not isinstance(hit_score, (int, float))
            or not math.isfinite(hit_score)
        ):
            _invalid_explanation(f"/results/{index}/score")
        hit_ce_score = getattr(native_hit, "ce_score", None)
        if hit_ce_score is not None and (
            isinstance(hit_ce_score, bool)
            or not isinstance(hit_ce_score, (int, float))
            or not math.isfinite(hit_ce_score)
            or not 0 <= hit_ce_score <= 1
        ):
            _invalid_explanation(f"/results/{index}/ceScore")
        if item.arm != hit_branch:
            _invalid_explanation(f"{prefix}/arm")
        if item.blended != hit_score:
            _invalid_explanation(f"{prefix}/blended")
        if item.ce_score != hit_ce_score:
            _invalid_explanation(f"{prefix}/ceScore")
        mapped.append(item)
    return Explanation(
        trace=QueryTrace(
            query_chars=query_chars,
            k=k,
            rerank_depth=rerank_depth,
            pool_n=pool_n,
            alpha=float(alpha),
            use_graph_arm=trace.use_graph_arm,
            recency=trace.recency,
            embedder_id=embedder_id,
            ce_active=trace.ce_active,
            vector_hits=counts[0],
            text_hits=counts[1],
            graph_hits=counts[2],
            dropped_edge_hits=counts[3],
            stop_words_sha256=stop_words_sha256,
        ),
        per_hit=mapped,
        correlation_id=correlation,
    )


def search(
    self: Engine,
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
    snippet: SnippetOptions | None = None,
) -> SearchResult:
    # FIX-3: reject bool and non-int before the negative check.
    # bool is a subclass of int in Python so it passes isinstance(x, int);
    # we reject it explicitly for X1 parity with TypeScript.
    if not isinstance(rerank_depth, int) or isinstance(rerank_depth, bool):
        raise TypeError(
            f"rerank_depth must be a non-negative integer, got {type(rerank_depth).__name__!r}"
        )
    if rerank_depth < 0:
        raise ValueError(f"rerank_depth must be >= 0, got {rerank_depth!r}")
    if not isinstance(use_graph_arm, bool):
        raise TypeError(f"use_graph_arm must be a bool, got {type(use_graph_arm).__name__!r}")
    # 0.8.8 EXP-OBS (Slice 10) — validate `explain` before the native call,
    # mirroring use_graph_arm + the TS `search` guard (cross-SDK parity).
    if not isinstance(explain, bool):
        raise TypeError(f"explain must be a bool, got {type(explain).__name__!r}")
    # 0.8.5 (codex §9 P2-2) — validate the new α/pool_n knobs before the native
    # call, mirroring the rerank_depth guard and the TS `search` validation
    # (cross-SDK parity). bool is rejected explicitly (it is an int/float
    # subclass that PyO3 would otherwise coerce silently).
    if alpha is not None:
        if isinstance(alpha, bool) or not isinstance(alpha, (int, float)):
            raise TypeError(f"alpha must be a finite number, got {type(alpha).__name__!r}")
        if not math.isfinite(alpha):
            raise ValueError(f"alpha must be a finite number, got {alpha!r}")
    if pool_n is not None:
        if not isinstance(pool_n, int) or isinstance(pool_n, bool):
            raise TypeError(f"pool_n must be a non-negative integer, got {type(pool_n).__name__!r}")
        if pool_n < 0:
            raise ValueError(f"pool_n must be >= 0, got {pool_n!r}")
    if not isinstance(view, (ReadView, type(None))):
        raise TypeError(f"view must be a ReadView or None, got {type(view).__name__!r}")
    _validate_snippet(snippet)
    limit = _validate_ranked_result_limit("limit", limit)
    native_view = _to_native_view(view)
    # 0.8.11 Slice 40 (#17) — accept the unified Filter on the vec0 search
    # path; lower to the SearchFilter sugar (typed-rejects a Json term, D3).
    if isinstance(filter, Filter):
        filter = filter.to_search_filter()
    if filter is None:
        result = self._native.search(
            query,
            rerank_depth=rerank_depth,
            use_graph_arm=use_graph_arm,
            alpha=alpha,
            pool_n=pool_n,
            explain=explain,
            view=native_view,
            limit=limit,
            snippet=snippet,
        )
    else:
        result = self._native.search(
            query,
            source_type=filter.source_type,
            kind=filter.kind,
            created_after=filter.created_after,
            status=filter.status,
            attributes=list(filter.attributes),
            rerank_depth=rerank_depth,
            use_graph_arm=use_graph_arm,
            alpha=alpha,
            pool_n=pool_n,
            explain=explain,
            view=native_view,
            limit=limit,
            snippet=snippet,
        )
    fallback = result.soft_fallback
    soft = (
        SoftFallback(branch=cast(SoftFallbackBranch, fallback.branch))
        if fallback is not None
        else None
    )
    # 0.8.8 EXP-OBS (Slice 10) — convert the opt-in native explanation sidecar
    # into dataclasses; `None` (default explain=False) stays `None`.
    native_exp = result.explanation
    explanation = (
        _map_candidate_native_explanation(native_exp, result.results)
        if native_exp is not None
        else None
    )
    return SearchResult(
        projection_cursor=result.projection_cursor,
        soft_fallback=soft,
        results=[
            SearchHit(
                id=IdSpace(space=hit.id.space, value=hit.id.value),
                kind=hit.kind,
                body=hit.body,
                score=hit.score,
                branch=cast(SoftFallbackBranch, hit.branch),
                source_id=hit.source_id,
                ce_score=hit.ce_score,
            )
            for hit in result.results
        ],
        explanation=explanation,
        degradations=_map_native_degradations(result),
        snippets=_map_native_snippets(result),
    )


def freeze_read_context(self: Engine, context: ReadContextV1) -> FrozenReadContextV1:
    if not isinstance(context, ReadContextV1):
        raise TypeError(f"context must be a ReadContextV1, got {type(context).__name__!r}")
    native = self._native.freeze_read_context(_to_native_read_context(context))
    if native.schema_version != 1 or native.context.schema_version != 1:
        raise FrozenReadError(
            "unsupported_schema_version at /schemaVersion",
            reason="unsupported_schema_version",
            field_path="/schemaVersion",
        )
    resolved_context = ReadContextV1(
        view=ReadView(
            include_superseded=context.view.include_superseded,
            include_inactive=context.view.include_inactive,
            include_out_of_window=context.view.include_out_of_window,
            valid_as_of=native.effective_valid_at,
        ),
        eligibility=context.eligibility,
        schema_version=context.schema_version,
    )
    return FrozenReadContextV1(
        effective_valid_at=native.effective_valid_at,
        context=resolved_context,
        token=native.token,
        schema_version=native.schema_version,
    )


def search_frozen(
    self: Engine,
    query: str,
    context: FrozenReadContextV1,
    *,
    rerank_depth: int = 0,
    use_graph_arm: bool = False,
    alpha: float = 0.3,
    pool_n: int | None = None,
    explain: bool = False,
    limit: int = 10,
    snippet: SnippetOptions | None = None,
) -> SearchResult:
    if not isinstance(context, FrozenReadContextV1):
        raise TypeError(f"context must be a FrozenReadContextV1, got {type(context).__name__!r}")
    native_context = _to_native_frozen_context(context)
    self._native.validate_frozen_read_context(native_context)
    if not isinstance(rerank_depth, int) or isinstance(rerank_depth, bool):
        raise TypeError("rerank_depth must be a non-negative integer")
    if rerank_depth < 0:
        raise InvalidArgumentError(f"rerank_depth must be >= 0, got {rerank_depth!r}")
    if not isinstance(use_graph_arm, bool):
        raise TypeError("use_graph_arm must be a bool")
    if isinstance(alpha, bool) or not isinstance(alpha, (int, float)):
        raise TypeError("alpha must be a finite number")
    if not math.isfinite(alpha):
        raise ValueError(f"alpha must be a finite number, got {alpha!r}")
    if pool_n is not None:
        if not isinstance(pool_n, int) or isinstance(pool_n, bool):
            raise TypeError("pool_n must be a non-negative integer")
        if pool_n < 0:
            raise InvalidArgumentError(f"pool_n must be >= 0, got {pool_n!r}")
    if not isinstance(explain, bool):
        raise TypeError("explain must be a bool")
    _validate_snippet(snippet)
    _validate_ranked_result_limit("limit", limit)
    native = self._native.search_frozen(
        query,
        native_context,
        rerank_depth=rerank_depth,
        use_graph_arm=use_graph_arm,
        alpha=alpha,
        pool_n=rerank_depth if pool_n is None else pool_n,
        explain=explain,
        limit=limit,
        snippet=snippet,
    )
    return _map_native_search_result(native)


def search_projected_text(
    self: Engine,
    query: str,
    name: str,
    filter: SearchFilter | None = None,
    *,
    view: ReadView | None = None,
    limit: int = 10,
) -> SearchResult:
    if not isinstance(filter, (SearchFilter, type(None))):
        raise TypeError(f"filter must be a SearchFilter or None, got {type(filter).__name__!r}")
    if not isinstance(view, (ReadView, type(None))):
        raise TypeError(f"view must be a ReadView or None, got {type(view).__name__!r}")
    limit = _validate_ranked_result_limit("limit", limit)
    kwargs: dict[str, Any] = {"view": _to_native_view(view), "limit": limit}
    if filter is not None:
        kwargs.update(
            source_type=filter.source_type,
            kind=filter.kind,
            created_after=filter.created_after,
            status=filter.status,
            attributes=list(filter.attributes),
        )
    result = self._native.search_projected_text(query, name, **kwargs)
    return SearchResult(
        projection_cursor=result.projection_cursor,
        soft_fallback=None,
        results=[
            SearchHit(
                id=IdSpace(space=hit.id.space, value=hit.id.value),
                kind=hit.kind,
                body=hit.body,
                score=hit.score,
                branch=cast(SoftFallbackBranch, hit.branch),
                source_id=hit.source_id,
                ce_score=hit.ce_score,
            )
            for hit in result.results
        ],
        explanation=None,
    )


def search_text_only(
    self: Engine,
    query: str,
    view: ReadView | None = None,
    *,
    limit: int = 10,
    snippet: SnippetOptions | None = None,
) -> SearchResult:
    if not isinstance(view, (ReadView, type(None))):
        raise TypeError(f"view must be a ReadView or None, got {type(view).__name__!r}")
    _validate_snippet(snippet)
    limit = _validate_ranked_result_limit("limit", limit)
    result = self._native.search_text_only(
        query, view=_to_native_view(view), limit=limit, snippet=snippet
    )
    fallback = result.soft_fallback
    soft = (
        SoftFallback(branch=cast(SoftFallbackBranch, fallback.branch))
        if fallback is not None
        else None
    )
    return SearchResult(
        projection_cursor=result.projection_cursor,
        soft_fallback=soft,
        results=[
            SearchHit(
                id=IdSpace(space=hit.id.space, value=hit.id.value),
                kind=hit.kind,
                body=hit.body,
                score=hit.score,
                branch=cast(SoftFallbackBranch, hit.branch),
                source_id=hit.source_id,
                ce_score=hit.ce_score,
            )
            for hit in result.results
        ],
        explanation=None,
        degradations=_map_native_degradations(result),
        snippets=_map_native_snippets(result),
    )
