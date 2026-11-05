"""Characterize the installed Python-facing boundary before SDK decomposition."""

from __future__ import annotations

import hashlib
import importlib.util
import inspect
import json
from pathlib import Path

import fathomdb
from fathomdb import admin, errors, graph, read
from fathomdb import engine as engine_module

BASELINE = (
    Path(__file__).resolve().parents[3]
    / "dev/plans/0.8.27/features/slice-130/pre-move-python-surface.json"
)
# Deliberate public additions after the frozen Slice 130 baseline. The
# baseline never changes; each later addition is declared and reviewed here.
ADDITIONS = Path(__file__).resolve().parent / "python_surface_additions.json"

# Approved later public surface (agent-access Block 1, 2026-10-18): EP-03
# `embedder_overloaded` degradations and EP-17 match-centred snippets.
EP03_EP17_EXPORTS = {
    "ArmDegradation",
    "DegradationReason",
    "SnippetAnchor",
    "SnippetOptions",
    "SnippetV1",
    "snippet_for",
}
# Declared in `fathomdb.types` and re-exported at the package root.
EP03_EP17_TYPE_DECLARATIONS = (
    ("ArmDegradation", "python-class", "@dataclass(frozen=True) class ArmDegradation"),
    ("ArmDegradation.arm", "python-attribute", "arm: SoftFallbackBranch"),
    ("ArmDegradation.reason", "python-attribute", "reason: DegradationReason"),
    (
        "DegradationReason",
        "python-value",
        "DegradationReason = Literal['embedder_overloaded']",
    ),
    (
        "SearchExpandResult.degradations",
        "python-attribute",
        "degradations: list[ArmDegradation] | None = None",
    ),
    (
        "SearchResult.degradations",
        "python-attribute",
        "degradations: list[ArmDegradation] | None = None",
    ),
    ("SearchResult.snippets", "python-attribute", "snippets: list[SnippetV1] | None = None"),
    ("SnippetAnchor", "python-value", "SnippetAnchor = Literal['whole', 'match', 'start']"),
    ("SnippetOptions", "python-class", "@dataclass(frozen=True) class SnippetOptions"),
    ("SnippetOptions.__post_init__", "python-method", "def __post_init__(self) -> None"),
    ("SnippetOptions.head_bytes", "python-attribute", "head_bytes: int = 0"),
    ("SnippetOptions.highlights", "python-attribute", "highlights: bool = False"),
    ("SnippetOptions.max_bytes", "python-attribute", "max_bytes: int = 300"),
    ("SnippetOptions.max_point_queries", "python-attribute", "max_point_queries: int = 20"),
    ("SnippetOptions.max_windows", "python-attribute", "max_windows: int = 1"),
    ("SnippetV1", "python-class", "@dataclass(frozen=True) class SnippetV1"),
    ("SnippetV1.anchor", "python-attribute", "anchor: SnippetAnchor"),
    ("SnippetV1.cut", "python-attribute", "cut: bool = False"),
    (
        "SnippetV1.highlights",
        "python-attribute",
        "highlights: list[tuple[int, int]] = field(default_factory=list)",
    ),
    (
        "SnippetV1.spans",
        "python-attribute",
        "spans: list[tuple[int, int]] = field(default_factory=list)",
    ),
    ("SnippetV1.text", "python-attribute", "text: str"),
    (
        "SnippetV1.windows",
        "python-attribute",
        "windows: list[tuple[int, int]] = field(default_factory=list)",
    ),
)
EP17_SNIPPET_FOR = (
    "def snippet_for(body: str, spans: Sequence[tuple[int, int]], "
    "options: SnippetOptions | None=None) -> SnippetV1"
)
EP17_SNIPPET_METHODS = ("search", "search_frozen", "search_text_only")
EP17_SNIPPET_PARAMETER = ", snippet: SnippetOptions | None=None) -> SearchResult"
# Each (current, historical) pair is an exact Block 1 edit to the native stub.
EP03_EP17_STUB_EDITS = (
    (
        "class ArmDegradation:\n    arm: str\n    reason: str\n\n",
        "",
    ),
    (
        "    explanation: Explanation | None\n"
        "    # None when every requested arm ran.\n"
        "    degradations: list[ArmDegradation] | None\n"
        "    # EP-17: one per hit when the search passed `snippet`; None otherwise.\n"
        "    snippets: list[SnippetV1] | None\n"
        "\n"
        "class SnippetV1:\n"
        "    # UTF-8 byte offsets: windows/spans into the body, highlights into text.\n"
        "    text: str\n"
        '    # "whole" | "match" | "start"\n'
        "    anchor: str\n"
        "    windows: list[tuple[int, int]]\n"
        "    spans: list[tuple[int, int]]\n"
        "    highlights: list[tuple[int, int]]\n"
        "    cut: bool\n",
        "    explanation: Explanation | None\n",
    ),
    (
        "        limit: int = ...,\n"
        "        snippet: object | None = ...,\n"
        "    ) -> SearchResult: ...\n"
        "    def search_with_evidence(\n",
        "        limit: int = ...,\n    ) -> SearchResult: ...\n    def search_with_evidence(\n",
    ),
    (
        "        limit: int = ...,\n"
        "        # EP-17: a `fathomdb.SnippetOptions` (read by attribute).\n"
        "        snippet: object | None = ...,\n",
        "        limit: int = ...,\n",
    ),
    (
        "    # EP-17: `snippet` is a `fathomdb.SnippetOptions` (read by attribute).\n"
        "    def search_text_only(\n"
        "        self, query: str, view: ReadView | None = ..., limit: int = ..., *,\n"
        "        source_type: str | None = ..., kind: str | None = ...,\n"
        "        created_after: int | None = ..., status: str | None = ...,\n"
        "        attributes: list[tuple[str, str]] | None = ...,\n"
        "        snippet: object | None = ...,\n"
        "    ) -> SearchResult: ...\n",
        "    def search_text_only(\n"
        "        self, query: str, view: ReadView | None = ..., limit: int = ...\n"
        "    ) -> SearchResult: ...\n",
    ),
    (
        "    all_logical_ids: list[str]\n"
        "    # None when every requested search arm ran.\n"
        "    degradations: list[ArmDegradation] | None\n",
        "    all_logical_ids: list[str]\n",
    ),
    (
        "def snippet_for(\n"
        "    body: str,\n"
        "    spans: list[tuple[int, int]],\n"
        "    options: object,\n"
        ") -> SnippetV1:\n"
        '    """EP-17 pure selector; `options` is a `fathomdb.SnippetOptions`.\n'
        "\n"
        "    Spans that do not fit u32 are ignored; option values are saturated into\n"
        "    their field types and clamped into range. Never raises for valid types.\n"
        '    """\n'
        "    ...\n"
        "\n",
        "",
    ),
)


def test_package_boundary_matches_pre_move_baseline() -> None:
    baseline = json.loads(BASELINE.read_text())
    baseline_exports = {row["path"] for row in baseline["python_package_exports"]}
    assert len(baseline_exports) == 119
    # Top-level additions since the frozen baseline come from the same
    # reviewed registry the declaration test uses: paths exactly `fathomdb.<name>`.
    additions = json.loads(ADDITIONS.read_text())
    added_exports = {
        path.removeprefix("fathomdb.")
        for path in (entry["declaration"]["path"] for entry in additions["additions"])
        if path.count(".") == 1 and path.startswith("fathomdb.")
    }
    assert not added_exports & baseline_exports
    assert not (baseline_exports | added_exports) & EP03_EP17_EXPORTS
    expected_exports = baseline_exports | added_exports | EP03_EP17_EXPORTS
    assert len(fathomdb.__all__) == len(expected_exports) == (
        119 + len(added_exports) + len(EP03_EP17_EXPORTS)
    )
    assert set(fathomdb.__all__) == expected_exports
    for name in expected_exports:
        assert getattr(fathomdb, name) is not None

    assert fathomdb.Engine is engine_module.Engine
    assert fathomdb.read is read
    assert fathomdb.graph is graph
    assert fathomdb.admin is admin
    assert fathomdb.errors is errors
    assert fathomdb.FrozenReadError is errors.FrozenReadError
    assert fathomdb.EvidenceError is errors.EvidenceError
    assert fathomdb.GraphExpansionError is errors.GraphExpansionError
    assert fathomdb.DependencyTraceError is errors.DependencyTraceError
    assert str(inspect.signature(fathomdb.Engine.search)) == (
        "(self, query: 'str', filter: 'SearchFilter | Filter | None' = None, *, "
        "rerank_depth: 'int' = 0, use_graph_arm: 'bool' = False, "
        "alpha: 'float | None' = None, pool_n: 'int | None' = None, "
        "explain: 'bool' = False, view: 'ReadView | None' = None, "
        "limit: 'int' = 10, snippet: 'SnippetOptions | None' = None) -> 'SearchResult'"
    )


def test_candidate_sources_are_from_this_checkout() -> None:
    checkout = Path(__file__).resolve().parents[3]
    assert Path(fathomdb.__file__).resolve().is_relative_to(checkout)
    assert Path(inspect.getfile(fathomdb.Engine)).resolve().is_relative_to(checkout)


def test_python_declarations_match_pre_move_baseline_with_approved_deltas() -> None:
    checkout = Path(__file__).resolve().parents[3]
    package = checkout / "src/python/fathomdb"
    spec = importlib.util.spec_from_file_location(
        "slice130_surface_comparator", checkout / "dev/tools/surface_comparator.py"
    )
    assert spec is not None and spec.loader is not None
    comparator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(comparator)

    sources = {
        path.relative_to(package).as_posix(): path.read_text() for path in package.rglob("*.py")
    }
    baseline = json.loads(BASELINE.read_text())
    additions = json.loads(ADDITIONS.read_text())
    declared = [entry["declaration"] for entry in additions["additions"]]
    declarations = comparator.parse_python_wrappers(sources)
    baseline_declarations = baseline["python_wrapper_declarations"]
    expected = {row["path"]: row for row in baseline_declarations}
    assert len(baseline_declarations) == len(expected) == 1131

    # The frozen baseline is historical; each release delta stays explicit.
    for path_name in ("fathomdb.Engine.search_frozen", "fathomdb.engine.Engine.search_frozen"):
        old = expected[path_name]
        assert "pool_n: int=0" in old["signature"]
        expected[path_name] = {
            **old,
            "signature": old["signature"].replace("pool_n: int=0", "pool_n: int | None=None"),
        }
    # D28-14 extends only the direct-body filter keyword; preserve both aliases.
    text_signature = (
        "def search_text_only(self, query: str, view: ReadView | None=None, *, "
        "limit: int=10) -> SearchResult"
    )
    filtered_text_signature = (
        "def search_text_only(self, query: str, view: ReadView | None=None, *, "
        "limit: int=10, filter: SearchFilter | None=None) -> SearchResult"
    )
    for namespace in ("fathomdb", "fathomdb.engine"):
        path_name = f"{namespace}.Engine.search_text_only"
        old = expected[path_name]
        assert old["signature"] == text_signature
        expected[path_name] = {**old, "signature": filtered_text_signature}
    expected["fathomdb.errors.DependencyTraceError"] = {
        "kind": "python-value",
        "path": "fathomdb.errors.DependencyTraceError",
        "signature": "DependencyTraceError = _DependencyTraceError",
    }
    version_row = expected["fathomdb.__version__"]
    assert version_row["signature"] == "__version__ = '0.8.26'"
    release_version = json.loads((checkout / "src/ts/package.json").read_text())["version"]
    expected["fathomdb.__version__"] = {
        **version_row,
        "signature": f"__version__ = {release_version!r}",
    }
    for declaration in declared:
        assert declaration["path"] not in expected
        expected[declaration["path"]] = declaration
    for prefix in ("fathomdb.Engine.", "fathomdb.engine.Engine."):
        for method in EP17_SNIPPET_METHODS:
            old = expected[prefix + method]
            assert old["signature"].count(") -> SearchResult") == 1
            expected[prefix + method] = {
                **old,
                "signature": old["signature"].replace(") -> SearchResult", EP17_SNIPPET_PARAMETER),
            }
    for suffix, kind, signature in EP03_EP17_TYPE_DECLARATIONS:
        for path in (f"fathomdb.{suffix}", f"fathomdb.types.{suffix}"):
            assert path not in expected
            expected[path] = {"kind": kind, "path": path, "signature": signature}
    assert "fathomdb.snippet_for" not in expected
    expected["fathomdb.snippet_for"] = {
        "kind": "python-function",
        "path": "fathomdb.snippet_for",
        "signature": EP17_SNIPPET_FOR,
    }

    # Owner seq-359 replaces attached-but-refused engines with typed failed opens.
    provider_removals = {
        f"{namespace}.Engine.{name}": {
            "kind": "python-method",
            "path": f"{namespace}.Engine.{name}",
            "signature": signature,
        }
        for namespace in ("fathomdb", "fathomdb.engine")
        for name, signature in (
            ("dense_disabled", "def dense_disabled(self) -> bool"),
            ("dense_disabled_reason", "def dense_disabled_reason(self) -> str | None"),
            ("vector_equivalence_refusal_count", "def vector_equivalence_refusal_count(self) -> int"),
        )
    }
    for name, signature in (
        ("dense_disabled", "dense_disabled: bool = False"),
        ("dense_disabled_reason", "dense_disabled_reason: str | None = None"),
    ):
        path_name = f"fathomdb.types.OpenReport.{name}"
        provider_removals[path_name] = {
            "kind": "python-attribute", "path": path_name, "signature": signature,
        }
    assert len(provider_removals) == 8
    for path_name, old_row in provider_removals.items():
        assert expected.pop(path_name) == old_row

    provider_literals = {
        "EmbeddingReadinessState": (
            "Literal['ready', 'processing', 'deferred', 'blocked']",
            "Literal['ready', 'processing', 'blocked']",
        ),
        "ProjectionReadinessV1": (
            "Literal['ready', 'processing', 'blocked', 'deferred', 'degraded']",
            "Literal['ready', 'processing', 'blocked', 'degraded']",
        ),
        "ProjectionRuntimeStateV1": (
            "Literal['absent', 'usable', 'refused']", "Literal['absent', 'usable']",
        ),
        "ProjectionRuntimeUnavailabilityReason": (
            "Literal['none', 'no_runtime', 'vector_equivalence_disabled']",
            "Literal['none', 'no_runtime']",
        ),
    }
    assert len(provider_literals) == 4
    for name, (old_literal, new_literal) in provider_literals.items():
        path_name = f"fathomdb.{name}"
        old_row = {
            "kind": "python-value", "path": path_name, "signature": f"{name} = {old_literal}",
        }
        assert expected[path_name] == old_row
        expected[path_name] = {**old_row, "signature": f"{name} = {new_literal}"}
    verification_error = {
        "kind": "python-value",
        "path": "fathomdb.errors.VectorVerificationError",
        "signature": "VectorVerificationError = _VectorVerificationError",
    }
    assert verification_error["path"] not in expected
    expected[verification_error["path"]] = verification_error
    observed = {row["path"]: row for row in declarations}
    assert len(declarations) == len(observed) == len(expected) == (
        1132 + len(declared) - 8 + 1 + 2 * len(EP03_EP17_TYPE_DECLARATIONS) + 1
    )
    assert observed == expected

    stub = (package / "_fathomdb.pyi").read_text()
    for current, historical in EP03_EP17_STUB_EDITS:
        assert stub.count(current) == 1, current
        stub = stub.replace(current, historical)
    verification_error_stub = (
        "class VectorVerificationError(EngineError):\n"
        '    """Open failed because vector verification could not complete; no engine exists."""\n'
        "    probe_ordinal: int | None\n"
        "    cause_code: str | None\n"
        "    cause_message: str | None\n"
        "    provider_kind: str | None\n"
        "    provider_error: EmbedderError | None\n"
        "    def __init__(self, *args: Any, probe_ordinal: int | None = ..., "
        "cause_code: str | None = ..., cause_message: str | None = ..., "
        "provider_kind: str | None = ..., provider_error: EmbedderError | None = ...) -> None: ...\n\n"
    )
    lifecycle_comment = (
        "# OPP-12 Phase-1 (0.8.19 Slice 10) — lifecycle-verb typed errors. Parity-safe\n"
    )
    provider_stub_deltas = (
        (
            "    embedder_mean_vec_pinned: bool\n"
            "    dense_disabled: bool\n"
            "    dense_disabled_reason: str | None\n"
            "    embedder_device_resolution: DeviceResolution | None\n",
            "    embedder_mean_vec_pinned: bool\n"
            "    embedder_device_resolution: DeviceResolution | None\n",
        ),
        (
            "    def dense_disabled(self) -> bool: ...\n"
            "    def dense_disabled_reason(self) -> str | None: ...\n"
            "    def vector_equivalence_refusal_count(self) -> int: ...\n"
            "    # 0.8.8 Slice 15 (OPP-9) — opt-in local telemetry capture.\n",
            "    # 0.8.8 Slice 15 (OPP-9) — opt-in local telemetry capture.\n",
        ),
        (
            "# 0.8.18 Slice 5 (#5 vector-equivalence probe) — query-time dense-refusal leaf.\n",
            "# Actual numeric verification refusal fails open with its original reason.\n",
        ),
        (lifecycle_comment, verification_error_stub + lifecycle_comment),
    )
    for old_stub, approved_stub in provider_stub_deltas:
        assert stub.count(approved_stub) == 1
        stub = stub.replace(approved_stub, old_stub, 1)
    # Exact owner-approved maintenance additions leave every other frozen byte
    # under the existing checksum; no historical baseline is regenerated.
    maintenance_class = (
        "class ErasureMaintenance:\n"
        "    def reattach_original_erasure_sink(self, ticket: str, path: str) -> None: ...\n"
        "    def complete_physical_erasure(self, ticket: str) -> str: ...\n"
        "    def queue_physical_erasure(self, selector_json: str, key: str) -> str: ...\n"
        "    def close(self) -> str: ...\n\n"
    )
    maintenance_engine_methods = (
        "    def queue_physical_erasure(self, selector_json: str, key: str) -> str: ...\n"
        "    def physical_erasure_status(self, ticket: str) -> str: ...\n"
        "    def withdraw_queued_erasure(self, ticket: str) -> str: ...\n"
        "    def physical_erasure_policy(self) -> str: ...\n"
        "    def begin_erasure_maintenance(self, acquisition_wait_ms: int) -> ErasureMaintenance: ...\n"
    )
    maintenance_error = (
        "\nclass MaintenanceError(EngineError):\n"
        "    reason: str\n"
        "    def __init__(self, *args: Any, reason: str = ...) -> None: ...\n"
    )
    for approved_stub in (
        maintenance_class + "class Engine:\n",
        "class Engine:\n" + maintenance_engine_methods,
        maintenance_error,
    ):
        assert stub.count(approved_stub) == 1
        replacement = "class Engine:\n" if "class Engine:\n" in approved_stub else ""
        stub = stub.replace(approved_stub, replacement, 1)
    # Slice 80 changes only this view-contract comment; preserve the declaration
    # checksum and every unapproved byte in the frozen baseline comparison.
    historical_view_comment = (
        "        # Optional lifecycle and validity view. Defaults to active,\n"
        "        # non-superseded rows valid at query time. The existence flags compose\n"
        "        # independently; unavailable dense history reports vector soft fallback.\n"
        "        # See `fathomdb.engine.Engine.search`.\n"
    )
    original_view_comment = (
        "        # 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — optional VALIDITY view.\n"
        "        # `None` (default) is the strict view: active-only, non-superseded, and\n"
        "        # valid AT QUERY TIME. The EXISTENCE flags (`include_superseded` /\n"
        "        # `include_inactive`) raise `InvalidArgumentError` on this path rather\n"
        "        # than being silently ignored. See `fathomdb.engine.Engine.search`.\n"
    )
    assert stub.count(historical_view_comment) == 1
    stub = stub.replace(historical_view_comment, original_view_comment, 1)
    prefix, marker, tail = stub.partition("    def search_frozen(\n")
    assert marker and "    def search_frozen(\n" not in tail
    frozen, suffix = tail.split("    ) -> SearchResult: ...\n", 1)
    approved = "pool_n: int | None = ..."
    assert frozen.count(approved) == 1
    pre_027_delta_stub = (
        prefix
        + marker
        + frozen.replace(approved, "pool_n: int = ...", 1)
        + "    ) -> SearchResult: ...\n"
        + suffix
    )
    assert (
        hashlib.sha256(pre_027_delta_stub.encode()).hexdigest() == additions["native_stub_sha256"]
    )
