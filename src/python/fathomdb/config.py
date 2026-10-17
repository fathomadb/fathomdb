"""Engine config dataclass mirrored across SDK bindings.

Knob set owned by `dev/design/engine.md` and pinned for symmetry across
SDK bindings by `dev/design/bindings.md` § 6. Field names follow Python
snake_case per `dev/interfaces/python.md`.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal


@dataclass(frozen=True)
class QueryStopWords:
    """Query stop words for one opened engine: ``(base ∪ extra) \\ keep``.

    ``base`` is ``"fathomdb"`` (the built-in list) or ``"none"``. Each word is
    one lowercase alphanumeric token, at most 64 characters, at most 256 per
    list, matched exactly against the query's tokens before stemming and
    without diacritic folding (``suggest`` does not drop ``suggests``). Every
    ``keep`` word must be in ``base`` and no ``extra`` word may be. The engine
    checks the words at open and raises ``InvalidArgumentError`` naming the
    field; the setting is not stored in the database.
    """

    base: Literal["fathomdb", "none"] = "fathomdb"
    extra: tuple[str, ...] = ()
    keep: tuple[str, ...] = ()


@dataclass(frozen=True)
class EngineConfig:
    """Engine-owned runtime knobs.

    Equivalent to the keyword form on `Engine.open`; both forms accepted, but
    not in the same call.

    The three GPU memory pool settings (`cuda_pool`, `cuda_pool_max_size`,
    `cuda_pool_release_threshold`) are fixed once per process: by the first
    open that passes any of them, or else by the `FATHOMDB_POOL_*`
    environment variables at the first GPU use. A later open that sets one
    to a different value raises `InvalidArgumentError`; closing every engine
    does not reset them.
    """

    embedder_pool_size: int | None = None
    scheduler_runtime_threads: int | None = None
    provenance_row_cap: int | None = None
    embedder_call_timeout_ms: int | None = None
    slow_threshold_ms: int | None = None
    cuda_pool: Literal["auto", "on", "off"] | None = None
    cuda_pool_max_size: int | None = None
    cuda_pool_release_threshold: Literal["0", "max"] | None = None
    #: Query stop words; ``None`` is the built-in list. Fixed at open and
    #: recorded as ``stop_words_sha256`` on the open report and on traces.
    query_stop_words: QueryStopWords | None = None


__all__ = ["EngineConfig", "QueryStopWords"]
