"""Engine config dataclass mirrored across SDK bindings.

Knob set owned by `dev/design/engine.md` and pinned for symmetry across
SDK bindings by `dev/design/bindings.md` § 6. Field names follow Python
snake_case per `dev/interfaces/python.md`.
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
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


def _coerce_scope_sequence(scopes: Sequence[str] | Iterable[str], field: str) -> tuple[str, ...]:
    if isinstance(scopes, (str, bytes)):
        raise TypeError(f"ScopePolicy {field} must be a sequence of str, not {type(scopes).__name__}")
    if not isinstance(scopes, Iterable):
        raise TypeError(f"ScopePolicy {field} must be a sequence of str, got {type(scopes).__name__}")
    items = tuple(scopes)
    for item in items:
        if not isinstance(item, str):
            raise TypeError(f"ScopePolicy {field} entries must be str, got {type(item).__name__}")
    return items


@dataclass(frozen=True)
class ScopePolicy:
    """Caller-scope isolation policy bound at engine open (0.8.32 Slice 20 / Pin A)."""

    read_scopes: tuple[str, ...] = ()
    write_scope: str | None = None
    scoped: bool = False

    @classmethod
    def unscoped(cls) -> ScopePolicy:
        """Return an unscoped policy permitting full read/write visibility."""
        return cls(read_scopes=(), write_scope=None, scoped=False)

    @classmethod
    def single(cls, scope: str) -> ScopePolicy:
        """Construct a single-scope read/write policy."""
        if not isinstance(scope, str):
            raise TypeError(f"ScopePolicy.single scope must be a str, got {type(scope).__name__}")
        return cls(read_scopes=(scope,), write_scope=scope, scoped=True)

    @classmethod
    def read_only(cls, read_scopes: Sequence[str] | Iterable[str]) -> ScopePolicy:
        """Construct a read-only policy granting ``read_scopes`` (`write_scope = None`)."""
        return cls(
            read_scopes=_coerce_scope_sequence(read_scopes, "read_scopes"),
            write_scope=None,
            scoped=True,
        )

    @classmethod
    def read_write(
        cls, read_scopes: Sequence[str] | Iterable[str], write_scope: str
    ) -> ScopePolicy:
        """Construct a multi-scope read policy with an explicit write grant."""
        scopes = _coerce_scope_sequence(read_scopes, "read_scopes")
        if not isinstance(write_scope, str):
            raise TypeError(
                f"ScopePolicy.read_write write_scope must be a str, got {type(write_scope).__name__}"
            )
        return cls(read_scopes=scopes, write_scope=write_scope, scoped=True)

    @property
    def is_unscoped(self) -> bool:
        """Return ``True`` when this policy runs in unscoped mode."""
        return not self.scoped


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

    #: Persisted erasure scheduling; None honors reopen policy and defaults new
    #: stores to autonomous. Explicit maintenance never means immediate revocation.
    physical_erasure_policy: Literal["autonomous", "explicit_maintenance"] | None = None
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
    #: Caller-scope isolation policy bound at engine open; ``None`` = unscoped.
    scope_policy: ScopePolicy | None = None


__all__ = ["EngineConfig", "QueryStopWords", "ScopePolicy"]
