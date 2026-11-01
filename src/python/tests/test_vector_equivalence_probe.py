"""Failed-open verification classes and report surface parity with TypeScript."""

from __future__ import annotations

from fathomdb import Engine
from fathomdb.errors import CudaPoolExhaustedError, EngineError, VectorEquivalenceMismatchError, VectorVerificationError

# 0.8.20 (R-20-E3): `source_id` is mandatory on every canonical write.
_SOURCE_ID = "py-test:vector-equivalence-probe"



def test_vector_equivalence_mismatch_is_engine_error_leaf() -> None:
    assert issubclass(VectorEquivalenceMismatchError, EngineError)
    # Typed-init carries the divergence reason (parity with the identity-mismatch leaf).
    err = VectorEquivalenceMismatchError("boom", reason="P1 flips=3")
    assert err.reason == "P1 flips=3"


def test_open_report_and_engine_have_no_refused_open_state(db_path: str) -> None:
    engine = Engine.open(db_path)
    try:
        report = engine.open_report()
        assert not hasattr(report, "dense_disabled")
        assert not hasattr(report, "dense_disabled_reason")
        assert not hasattr(engine, "dense_disabled")
        assert not hasattr(engine, "dense_disabled_reason")
        assert not hasattr(engine, "vector_equivalence_refusal_count")
    finally:
        engine.close()


def test_search_text_only_serves(db_path: str) -> None:
    engine = Engine.open(db_path)
    try:
        engine.write(
            [{"kind": "note", "body": "alpha bravo charlie", "source_id": _SOURCE_ID}]
        )
        engine.drain(timeout_s=30)
        result = engine.search_text_only("alpha")
        # FTS-only path returns a SearchResult (may or may not hit; must not raise).
        assert hasattr(result, "results")
    finally:
        engine.close()


def test_vector_verification_constructor_preserves_typed_provider_payload() -> None:
    provider = CudaPoolExhaustedError("oom", ordinal=2, max_size_bytes=12345, message="original detail")
    error = VectorVerificationError("cannot verify", probe_ordinal=7, cause_code="provider", cause_message="original detail", provider_kind="cuda_pool_exhausted", provider_error=provider)
    assert isinstance(error, EngineError)
    assert error.probe_ordinal == 7
    assert error.cause_code == "provider"
    assert error.cause_message == "original detail"
    assert error.provider_kind == "cuda_pool_exhausted"
    assert error.provider_error is provider
    assert error.provider_error.ordinal == 2
    assert error.provider_error.max_size_bytes == 12345
