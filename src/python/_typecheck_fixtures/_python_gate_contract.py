"""Compile-only contract for the Python clean-clone quality gate."""

from fathomdb._fathomdb import Engine as NativeEngine
from fathomdb._fathomdb import OpenReport as NativeOpenReport
from fathomdb.engine import Engine
from fathomdb.errors import VectorEquivalenceMismatchError
from fathomdb.graph import _to_search_hit


def native_observability_contract(engine: Engine) -> tuple[bool, str]:
    """Successful-open observations retain their public types."""
    report = engine.open_report()
    return report.embedder_mean_vec_pinned, report.query_backend


def native_stub_contract(engine: NativeEngine, report: NativeOpenReport) -> tuple[bool, str]:
    """The wrapper reaches native members that the release stub declares."""
    return report.embedder_mean_vec_pinned, engine.open_report().query_backend


def graph_contract() -> object:
    """Keep the graph wrapper's native-to-public hit conversion type-checked."""
    return _to_search_hit


def vector_refusal_contract() -> str | None:
    """The typed Python-only constructor accepts the documented reason payload."""
    return VectorEquivalenceMismatchError("vector divergence", reason="P1 flips=3").reason


def omitted_vector_refusal_contract() -> str | None:
    """The native error may omit its optional divergence payload."""
    return VectorEquivalenceMismatchError("vector divergence").reason
