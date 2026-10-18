"""EP-03 ``embedder_overloaded`` at the Python wrapper boundary.

A Python test cannot drive the engine's embed queue to saturation, so the
native conversion of a real overloaded engine result is tested in Rust
(``fathomdb-py`` ``read_search::degradation_tests``) and this file tests the
wrapper's mapping of the native value.
"""

from types import SimpleNamespace

from fathomdb import ArmDegradation
from fathomdb.engine import _map_native_search_result


def _native(**fields: object) -> SimpleNamespace:
    return SimpleNamespace(
        projection_cursor=3,
        soft_fallback=None,
        results=[],
        explanation=None,
        **fields,
    )


def test_python_search_result_reports_embedder_overloaded() -> None:
    native = _native(degradations=[SimpleNamespace(arm="vector", reason="embedder_overloaded")])
    mapped = _map_native_search_result(native)
    assert mapped.degradations == [ArmDegradation(arm="vector", reason="embedder_overloaded")]


def test_python_search_result_without_degradations_is_none() -> None:
    assert _map_native_search_result(_native(degradations=None)).degradations is None
    assert _map_native_search_result(_native()).degradations is None
