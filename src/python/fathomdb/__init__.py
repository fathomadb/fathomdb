"""FathomDB Python SDK public surface.

Top-level five-verb surface and exception hierarchy owned by
`dev/interfaces/python.md` + `dev/design/errors.md`. The package delegates
to the native PyO3 extension `fathomdb._fathomdb` which binds to
`fathomdb-engine`.
"""

from __future__ import annotations

# AC80-25 / D-80.6-3 — FIRST, before the native extension is loaded: refuse to
# run when two FathomDB distributions (e.g. the generic and the Tegra wheels)
# are installed into one environment. They ship the same top-level `fathomdb`
# import package, so the second install silently shadows the first's native
# extension. No pip/PEP-621 metadata can express that conflict, so this
# import-time check is the enforcement; see `fathomdb/_coinstall.py`.
from fathomdb import _coinstall  # noqa: F401 — import-time co-installation guard
from fathomdb import _fathomdb as _native  # noqa: F401 — load native extension
from fathomdb import admin, errors, graph, read
from fathomdb._fathomdb import ConsolidateReceipt
from fathomdb._fathomdb import IngestWithExtractorReceipt
from fathomdb._fathomdb import embed_batch_cls
from fathomdb._fathomdb import rerank
from fathomdb.config import EngineConfig
from fathomdb.engine import Engine
from fathomdb.filter import Filter
from fathomdb.types import (
    CounterSnapshot,
    CudaDeviceInfo,
    CudaVisibleDevice,
    DenseReadiness,
    DeviceResolution,
    EmbeddingOperation,
    EmbeddingReadiness,
    EmbeddingReadinessState,
    EffectiveEmbedDevice,
    ExpandedNode,
    Explanation,
    GpuAllocationWitness,
    IdSpace,
    NodeRecord,
    OpStoreRow,
    PerHitExplain,
    ProjectionDelta,
    ProjectionRole,
    ProjectionRuntimeStatus,
    ProjectionRuntimeStatusEntry,
    ProjectionRuntimeUnavailabilityReason,
    ProjectionSpec,
    ProjectionStatusDenseReadiness,
    QueryTrace,
    SearchExpandResult,
    SearchFilter,
    SearchHit,
    SearchResult,
    SoftFallback,
    SoftFallbackBranch,
    WriteReceipt,
)

__all__ = [
    "ConsolidateReceipt",
    "CounterSnapshot",
    "CudaDeviceInfo",
    "CudaVisibleDevice",
    "DenseReadiness",
    "DeviceResolution",
    "EmbeddingOperation",
    "EmbeddingReadiness",
    "EmbeddingReadinessState",
    "EffectiveEmbedDevice",
    "Engine",
    "EngineConfig",
    "Filter",
    "IngestWithExtractorReceipt",
    "ExpandedNode",
    "Explanation",
    "GpuAllocationWitness",
    "IdSpace",
    "NodeRecord",
    "OpStoreRow",
    "PerHitExplain",
    "ProjectionDelta",
    "ProjectionRole",
    "ProjectionRuntimeStatus",
    "ProjectionRuntimeStatusEntry",
    "ProjectionRuntimeUnavailabilityReason",
    "ProjectionSpec",
    "ProjectionStatusDenseReadiness",
    "QueryTrace",
    "SearchExpandResult",
    "SearchFilter",
    "SearchHit",
    "SearchResult",
    "SoftFallback",
    "SoftFallbackBranch",
    "WriteReceipt",
    "__version__",
    "admin",
    "embed_batch_cls",
    "errors",
    "graph",
    "read",
    "rerank",
]
__version__ = "0.8.22"
