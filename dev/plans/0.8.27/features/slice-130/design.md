---
title: FathomDB 0.8.27 Slice 130 - Python SDK decomposition design
status: REVIEWED
target_release: 0.8.27
---

# Slice 130 - Python SDK decomposition design

## Boundary

`fathomdb.__init__` retains its literal `__all__`, import order, and one
`Engine` class identity. Standalone `rerank` and `embed_batch_cls` remain
native package-root exports; the already-thin `Engine.embed` delegate stays
on the facade. The public `read`, `graph`, and `admin` modules keep
their existing namespace functions. `engine.py` retains every public method's
name, signature, annotation, and docstring, plus open/close/drain, handle,
path, and configuration lifecycle. It delegates substantial operations to
private, package-internal domain functions. These functions receive the
`Engine` or native handle explicitly and create no second facade, registry,
or mutable global state. The native PyO3 binding remains the sole database
semantic authority.

## Ownership

| Owner | Remaining `engine.py` responsibility |
| --- | --- |
| `_sdk_write.py` | Write/ingest/consolidate, source dependencies, actuation, closure, lifecycle, and erasure adapters. |
| `_sdk_search.py` | Ranked, text-only, projected-text, and frozen search; hit/result and explanation mapping; limits/filter validation and frozen context conversion. |
| `_sdk_graph.py` | Graph expansion and graph evidence validation/mapping and frozen graph search. Existing public `graph.py` keeps its namespace. |
| `_sdk_evidence.py` | Evidence search/resolve and dependency trace validation/mapping. |
| `_sdk_projection.py` | Projection configuration and projection source conversion. Existing public `read.py` keeps projection read functions. |
| `_sdk_open.py` | Open-report, GPU witness, and structural/frozen trace mapping plus runtime configuration conversion. |
| `_sdk_instrumentation.py` | Telemetry, feedback, counters, profiling, slow threshold, subscriber forwarding, and dense-disabled status. |
| Existing modules | `read.py`, `graph.py`, `admin.py`, `filter.py`, `config.py`, `types.py`, and `errors.py` retain current owners and public paths. |

The names are private implementation paths, not supported imports or package
subpaths. Runtime imports flow from the facade into domain owners, and from
graph/evidence owners into canonical search converters where needed. Domain
owners import `Engine` only under `TYPE_CHECKING`. The public `graph.expand`
calls its graph owner directly instead of dynamically importing the mapper
from `engine.py`; this closes the current back edge. The known direct
private test imports (`_map_native_search_result`, `_map_per_hit_explain`,
`_map_native_graph_expand_result`, `_map_native_evidence_search`,
`_map_native_resolved_evidence`, and `_map_open_report`) remain aliases in
`engine.py` for existing test reachability. They are not added to `__all__`.

## Compatibility proof

Capture the pre-move public comparator rows at the exact release tip and
compare them to the final candidate. A focused runtime fixture checks root and
documented namespace imports, `__all__`, signatures, and exact exception
objects. A temporary removed export or altered delegation demonstrates its
non-vacuity. Existing wire/error fixtures cover mappings and precedence;
functional smoke checks use candidate Python sources with a separately built
or previously qualified native binary, without an editable installation from
the worktree. Each runtime receipt records `fathomdb.__file__`, the loaded
`Engine` source path, and the native binary path, and asserts that Python
sources resolve inside this slice worktree. The native `.pyi` and
PyO3 source remain untouched. `pyright`, Ruff, and the source-change gate
cover static and repository integration. The hidden-surface and test inventory
comparison records expected private movement without weakening the public
oracle. Slice 150 owns the later full release matrix.
