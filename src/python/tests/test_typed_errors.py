"""AC-060a — typed error payload coverage.

The engine returns enum variants with typed fields; the binding
translator (`engine_error_to_py` / `engine_open_error_to_py`) attaches
those fields as Python attributes on the raised exception so callers
can dispatch and inspect without parsing message text.
"""

from __future__ import annotations

import pytest

from fathomdb import Engine, ScopePolicy
from fathomdb.errors import (
    CorruptionError,
    CudaContextLostError,
    CudaPoolExhaustedError,
    CudaPrivateBuildRefusedError,
    DatabaseLockedError,
    EmbedDevicePolicyError,
    EmbedderDimensionMismatchError,
    EmbedderError,
    EmbedderNotConfiguredError,
    EngineError,
    KindNotVectorIndexedError,
    ScopeViolationError,
    VectorError,
)


def test_database_locked_error_attr_round_trip() -> None:
    err = DatabaseLockedError(holder_pid=12345)
    assert err.holder_pid == 12345


def test_database_locked_engine_triggered(db_path: str) -> None:
    """Opening the same database twice in one process must surface
    `DatabaseLockedError` with the `holder_pid` attribute populated
    (or `None` when the lockfile is unparseable)."""

    a = Engine.open(db_path)
    try:
        with pytest.raises(DatabaseLockedError) as excinfo:
            Engine.open(db_path)
        assert hasattr(excinfo.value, "holder_pid")
        assert excinfo.value.holder_pid is None or isinstance(
            excinfo.value.holder_pid, int
        )
    finally:
        a.close()


def test_corruption_error_attrs_round_trip() -> None:
    err = CorruptionError(
        kind="HeaderMalformed",
        stage="HeaderProbe",
        recovery_hint_code="E_CORRUPT_HEADER",
        doc_anchor="design/recovery.md#header-malformed",
    )
    assert err.kind == "HeaderMalformed"
    assert err.stage == "HeaderProbe"
    assert err.recovery_hint_code == "E_CORRUPT_HEADER"
    assert err.doc_anchor == "design/recovery.md#header-malformed"


def test_embedder_not_configured_is_distinct_leaf_under_embedder_error() -> None:
    err = EmbedderNotConfiguredError("no embedder")
    assert isinstance(err, EmbedderNotConfiguredError)
    assert isinstance(err, EmbedderError)
    assert isinstance(err, EngineError)
    assert EmbedderNotConfiguredError is not EmbedderError


def test_embed_device_policy_error_has_typed_policy_fields() -> None:
    err = EmbedDevicePolicyError(kind="cuda_not_compiled", ordinal=2)
    assert err.kind == "cuda_not_compiled"
    assert err.ordinal == 2
    assert isinstance(err, EmbedderError)


def test_kind_not_vector_indexed_is_distinct_leaf_under_vector_error() -> None:
    err = KindNotVectorIndexedError("kind X not vector indexed")
    assert isinstance(err, KindNotVectorIndexedError)
    assert isinstance(err, VectorError)
    assert isinstance(err, EngineError)
    assert KindNotVectorIndexedError is not VectorError


def test_embedder_dimension_mismatch_attrs_round_trip() -> None:
    err = EmbedderDimensionMismatchError(stored=384, supplied=768)
    assert err.stored == 384
    assert err.supplied == 768
    assert isinstance(err.stored, int)
    assert isinstance(err.supplied, int)


# ---------------------------------------------------------------------------
# 0.8.28 Slice 30 (R30-04) — the three CUDA pool kinds
# ---------------------------------------------------------------------------


def test_cuda_pool_exhausted_carries_its_payload() -> None:
    err = CudaPoolExhaustedError(
        "rerank forward: out of memory",
        ordinal=0,
        max_size_bytes=3 << 30,
        message="rerank forward: out of memory",
    )
    assert isinstance(err, EmbedderError)
    assert isinstance(err, EngineError)
    assert err.ordinal == 0
    assert err.max_size_bytes == 3 << 30
    assert err.message == "rerank forward: out of memory"


def test_cuda_context_lost_carries_integer_context_ids() -> None:
    err = CudaContextLostError(
        "CUDA context lost",
        recorded_context_id=2**64 - 1,
        current_context_id=None,
        driver_error="CUDA_ERROR_CONTEXT_IS_DESTROYED",
        operation="embed forward",
    )
    assert isinstance(err, EmbedderError)
    assert err.recorded_context_id == 2**64 - 1
    assert err.current_context_id is None
    assert err.driver_error == "CUDA_ERROR_CONTEXT_IS_DESTROYED"
    assert err.operation == "embed forward"


def test_cuda_private_build_refused_carries_its_payload() -> None:
    err = CudaPrivateBuildRefusedError("refused", ordinal=1, message="refused")
    assert isinstance(err, EmbedderError)
    assert err.ordinal == 1
    assert err.message == "refused"


def test_the_cuda_pool_classes_are_distinct_leaves() -> None:
    classes = (CudaPoolExhaustedError, CudaContextLostError, CudaPrivateBuildRefusedError)
    for cls in classes:
        assert issubclass(cls, EmbedderError)
        assert cls is not EmbedderError
    assert not issubclass(CudaPoolExhaustedError, CudaContextLostError)
    assert not issubclass(CudaPrivateBuildRefusedError, CudaPoolExhaustedError)


def test_read_only_scope_write_raises_scope_violation_error(db_path: str) -> None:
    engine = Engine.open(
        db_path,
        use_default_embedder=False,
        scope_policy=ScopePolicy.read_only(["agent-a"]),
    )
    try:
        with pytest.raises(ScopeViolationError) as excinfo:
            engine.write([{"kind": "note", "body": "denied", "source_id": "typed-scope"}])
        assert isinstance(excinfo.value, EngineError)
        assert str(excinfo.value)
    finally:
        engine.close()


@pytest.mark.parametrize("update", [
    "UPDATE _fathomdb_embedder_profiles SET name=X'00' WHERE profile='default'",
    "UPDATE _fathomdb_embedder_profiles SET revision=X'00' WHERE profile='default'",
    "UPDATE _fathomdb_embedder_profiles SET dimension=-1 WHERE profile='default'",
], ids=["name", "revision", "dimension"])
def test_installed_profile_corruption_preserves_typed_admission_and_reopen(tmp_path, update):
    """Installed packages retain the established profile-corruption payload and release the lock."""
    import sqlite3

    path = str(tmp_path / "profile.sqlite")
    engine = Engine.open(path, use_default_embedder=False)
    try:
        engine.write([{"kind": "note", "logical_id": "persisted", "body": "typedopen persisted", "source_id": "qualification:profile"}])
    finally:
        engine.close()
    with sqlite3.connect(path) as connection:
        original = connection.execute("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'").fetchone()
        assert original is not None
        connection.execute(update)
    for _ in range(2):
        with pytest.raises(CorruptionError) as caught:
            Engine.open(path, use_default_embedder=False)
        error = caught.value
        assert error.kind == "EmbedderIdentityDrift"
        assert error.stage == "EmbedderIdentity"
        assert error.recovery_hint_code == "E_CORRUPT_EMBEDDER_IDENTITY"
        assert error.doc_anchor == "design/recovery.md#embedder-identity-drift"
    with sqlite3.connect(path) as connection:
        connection.execute("UPDATE _fathomdb_embedder_profiles SET name=?,revision=?,dimension=? WHERE profile='default'", original)
    engine = Engine.open(path, use_default_embedder=False)
    try:
        assert engine.search_text_only("typedopen").results[0].body == "typedopen persisted"
    finally:
        engine.close()
