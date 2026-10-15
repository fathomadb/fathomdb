"""Engine config dataclass mirrored across SDK bindings.

Knob set owned by `dev/design/engine.md` and pinned for symmetry across
SDK bindings by `dev/design/bindings.md` § 6. Field names follow Python
snake_case per `dev/interfaces/python.md`.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal


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


__all__ = ["EngineConfig"]
