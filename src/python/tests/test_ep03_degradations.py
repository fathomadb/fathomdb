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


def _native_expand(**fields: object) -> SimpleNamespace:
    return SimpleNamespace(search_hits=[], expanded=[], all_logical_ids=[], **fields)


def test_python_search_expand_results_report_embedder_overloaded() -> None:
    from fathomdb._sdk_graph import _map_native_search_expand_result
    from fathomdb.graph import _to_search_expand_result

    native = _native_expand(
        degradations=[SimpleNamespace(arm="vector", reason="embedder_overloaded")]
    )
    expected = [ArmDegradation(arm="vector", reason="embedder_overloaded")]
    assert _to_search_expand_result(native).degradations == expected
    assert _map_native_search_expand_result(native).degradations == expected
    for absent in (_native_expand(), _native_expand(degradations=None)):
        assert _to_search_expand_result(absent).degradations is None
        assert _map_native_search_expand_result(absent).degradations is None
