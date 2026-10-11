"""Private instrumentation operations for the Python Engine facade."""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from fathomdb._sdk_search import _validate_id_list
from fathomdb.types import (
    CounterSnapshot,
)

if TYPE_CHECKING:
    from fathomdb.engine import Engine


def dense_disabled(self: Engine) -> bool:
    return self._native.dense_disabled()


def dense_disabled_reason(self: Engine) -> str | None:
    return self._native.dense_disabled_reason()


def vector_equivalence_refusal_count(self: Engine) -> int:
    return self._native.vector_equivalence_refusal_count()


def enable_telemetry(self: Engine, sink_path: str) -> None:
    if not isinstance(sink_path, str):
        raise TypeError(f"sink_path must be a str, got {type(sink_path).__name__!r}")
    self._native.enable_telemetry(sink_path)


def last_telemetry_query_id(self: Engine) -> str | None:
    return self._native.last_telemetry_query_id()


def record_feedback(
    self: Engine,
    query_id: str,
    relevant_ids: list[int],
    irrelevant_ids: list[int],
    label_source: str,
) -> None:
    if not isinstance(query_id, str):
        raise TypeError(f"query_id must be a str, got {type(query_id).__name__!r}")
    if not isinstance(label_source, str):
        raise TypeError(f"label_source must be a str, got {type(label_source).__name__!r}")
    relevant = _validate_id_list("relevant_ids", relevant_ids)
    irrelevant = _validate_id_list("irrelevant_ids", irrelevant_ids)
    self._native.record_feedback(query_id, relevant, irrelevant, label_source)


def counters(self: Engine) -> CounterSnapshot:
    snap = self._native.counters()
    return CounterSnapshot(
        queries=snap.queries,
        writes=snap.writes,
        write_rows=snap.write_rows,
        admin_ops=snap.admin_ops,
        cache_hit=snap.cache_hit,
        cache_miss=snap.cache_miss,
    )


def set_profiling(self: Engine, *, enabled: bool) -> None:
    self._native.set_profiling(enabled)


def set_slow_threshold_ms(self: Engine, *, value: int) -> None:
    self._native.set_slow_threshold_ms(value)


def attach_logging_subscriber(
    self: Engine,
    logger: logging.Logger,
) -> None:

    self._native.attach_logging_subscriber(logger)
