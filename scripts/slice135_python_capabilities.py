#!/usr/bin/env python3
"""Run inspected real-database contract cases through a verified installed wheel.

The operation map is exhaustive; successful cases are narrow behavioral evidence,
not a claim of complete contract, platform, contention or timing qualification.
Existing assertions and fixtures are reused without generating new gold values.
Requires pytest in the selected wheel environment. Never imports checkout SDK code.
"""
from __future__ import annotations

import argparse
from contextlib import redirect_stdout, redirect_stderr
from datetime import datetime, timezone
import hashlib
import importlib.util
import inspect
import io
import json
from pathlib import Path
import platform
import sqlite3
import subprocess
import sys
from tempfile import TemporaryDirectory
import traceback
from typing import Any, Callable
import zipfile


# Cases accept only temporary paths: no mock-native or monkeypatch fixture is allowed.
CASES = {
    "retrieve": ("test_functional_retrieve.py", "test_read_get_returns_active_node_by_id"),
    "many": ("test_functional_retrieve.py", "test_read_get_many_preserves_order_with_none"),
    "log": ("test_functional_retrieve.py", "test_read_collection_and_mutations_honor_cursor_and_limit"),
    "admin": ("test_functional_retrieve.py", "test_admin_configure_path_exercised"),
    "list": ("test_read_list.py", "test_read_list_and_composition"),
    "list_error": ("test_read_list.py", "test_read_list_non_allowlisted_path_raises_invalid_filter"),
    "lifecycle": ("test_opp12_lifecycle_verbs.py", "test_legal_transitions_and_reason_semantics"),
    "lifecycle_error": ("test_opp12_lifecycle_verbs.py", "test_illegal_transition_is_typed_with_fields"),
    "purge": ("test_opp12_lifecycle_verbs.py", "test_purge_requires_deleted_first_and_is_idempotent"),
    "dependency": ("test_slice20_source_dependencies.py", "test_dependency_round_trip_and_decimal_generation"),
    "actuation": ("test_slice25_actuation.py", "test_actuation_round_trip_replay_and_decimal_boundaries"),
    "actuation_error": ("test_slice25_actuation.py", "test_actuation_rejects_unknown_top_level_field_before_engine_call"),
    "projection": ("test_slice15d_projection_registry.py", "test_configure_and_read_projections_round_trip"),
    "projection_error": ("test_slice15d_projection_registry.py", "test_destructive_change_requires_explicit_drop"),
    "projected": ("test_slice45_nested_source_projections.py", "test_nested_projection_type_collapsed_equality_and_projected_search"),
    "projection_status": ("test_slice22_projection_status.py", "test_projection_status_is_a_frozen_typed_current_read"),
    "generation": ("test_slice40_projection_generation.py", "test_fresh_generation_status_is_typed_and_stable"),
    "mutation_status": ("test_slice40_projection_generation.py", "test_receipt_keyed_mutation_status_round_trips_through_python"),
    "readiness": ("test_slice30_embedding_readiness.py", "test_embedding_readiness_and_immediate_error_are_typed_and_body_private"),
    "frozen": ("test_slice35_frozen_read.py", "test_frozen_search_preserves_context_and_filters_before_ranking"),
    "frozen_error": ("test_slice35_frozen_read.py", "test_frozen_authentication_precedes_query_and_range_validation"),
    "frozen_expand": ("test_slice35_frozen_read.py", "test_frozen_search_expand_uses_the_same_context"),
    "frozen_drift": ("test_slice35_frozen_read.py", "test_frozen_context_rejects_state_drift"),
    "pages": ("test_slice45_pagination.py", "test_canonical_and_operational_pages_share_frozen_authority"),
    "pages_error": ("test_slice45_pagination.py", "test_page_request_refusals_retain_page_error_reason_and_path"),
    "boundary": ("test_slice15b_node_validity_write.py", "test_crossed_boundary_since_works_on_authored_windows"),
    "neighbors": ("test_functional_graph.py", "test_graph_neighbors_depth1_outgoing"),
    "neighbors_error": ("test_functional_graph.py", "test_graph_neighbors_depth_gt3_raises_invalid_argument"),
    "graph_search": ("test_functional_graph.py", "test_search_expand_expanded_contains_neighbor"),
    "graph_evidence": ("test_slice20_graph_evidence.py", "test_real_engine_resolves_exact_target_and_terminal_edge"),
    "evidence": ("test_slice50_evidence.py", "test_search_and_resolve_exact_source_evidence"),
    "evidence_error": ("test_slice50_evidence.py", "test_unsupported_evidence_schema_is_typed"),
    "embed": ("test_embed.py", "test_embed_returns_fixed_dim_float_vector"),
    "embed_determinism": ("test_embed.py", "test_embed_is_deterministic"),
    "embed_error": ("test_embed.py", "test_embed_without_embedder_raises"),
    "closure_committed": ("test_slice30_dependency_closure.py", "test_committed_closure_status_survives_reopen"),
}

# The second member names an independently executed error case, if available.
PLANNED = {
    "engine.open": ("retrieve", "boundary_errors"),
    "admin.configure": ("admin", "boundary_errors"),
    "engine.write": ("retrieve", "boundary_errors"),
    "engine.actuate": ("actuation", "actuation_error"),
    "engine.register_source_dependency": ("dependency", "dependency_errors"),
    "engine.dependencies_for_source": ("dependency", "dependency_errors"),
    "engine.dependency_for_derived": ("dependency", "dependency_errors"),
    "engine.read_dependency_closure": ("closure_committed", "closure"),
    "engine.transition": ("lifecycle", "lifecycle_error"),
    "engine.purge": ("purge", "purge"),
    "engine.erase_source": ("erasure", "erasure"),
    "engine.search": ("projected", "boundary_errors"),
    "engine.freeze_read_context": ("frozen", "boundary_errors"),
    "engine.search_frozen": ("frozen", "frozen_error"),
    "engine.search_expand_frozen": ("frozen_expand", "frozen_error"),
    "engine.search_text_only": ("search_text", "boundary_errors"),
    "engine.search_projected_text": ("projected", "boundary_errors"),
    "engine.search_with_evidence": ("evidence", "evidence_error"),
    "engine.resolve_evidence": ("evidence", "evidence_refusals"),
    "engine.resolve_graph_evidence": ("graph_evidence", "evidence_refusals"),
    "engine.trace_dependency": ("trace", "trace"),
    "engine.close": ("retrieve", "boundary_errors"),
    "read.get": ("retrieve", "boundary_errors"),
    "read.get_many": ("many", "boundary_errors"),
    "read.collection": ("log", "boundary_errors"),
    "read.mutations": ("log", "boundary_errors"),
    "read.list": ("list", "list_error"),
    "graph.expand": ("graph_evidence", "graph_refusals"),
    "graph.neighbors": ("neighbors", "neighbors_error"),
    "graph.search_expand": ("graph_search", "graph_refusals"),
    "engine.embed": ("embed", "embed_error"),
    "read.crossed_boundary_since": ("boundary", "boundary_errors"),
    "engine.configure_projections": ("projection", "projection_error"),
    "read.projections": ("projection", "closed_reads"),
    "read.projection_status": ("projection_status", "closed_reads"),
    "read.embedding_readiness": ("readiness", "closed_reads"),
    "read.projection_generation_status": ("generation", "closed_reads"),
    "read.mutation_projection_status": ("mutation_status", "projection_status_error"),
    "read.canonical_page": ("pages", "pages_error"),
    "read.operational_state": ("pages", "operational_errors"),
    "read.operational_state_page": ("pages", "operational_errors"),
}
UNAVAILABLE = {
    "engine.ingest_with_extractor": {"boundary": "provider", "reason": "No independently qualified extraction provider/command or credentials supplied; fake provider output is not a provider qualification.", "owner": "Slice 135 provider exercise"},
    "engine.consolidate_with_provider": {"boundary": "provider", "reason": "No independently qualified consolidation provider/command or credentials supplied; deterministic fabricated LLM output would not qualify real-provider behavior.", "owner": "Slice 135 provider exercise"},
    "rerank": {"boundary": "standalone_model", "reason": "Standalone cross-encoder model identity and offline availability are not qualified by this wheel exercise; no database boundary exists.", "owner": "Slice 135 standalone reranker qualification"},
}


def refusal(call: Callable[[], Any], error_type: type[Exception], *, reason: str | None = None, field_path: str | None = None) -> dict[str, Any]:
    """Assert a typed refusal and optional contract fields; success is a failure."""
    try:
        call()
    except error_type as error:
        if reason is not None and getattr(error, "reason", None) != reason:
            raise AssertionError(f"refusal reason differs: expected {reason!r}, got {getattr(error, 'reason', None)!r}") from error
        if field_path is not None and getattr(error, "field_path", None) != field_path:
            raise AssertionError("refusal field path differs") from error
        return {"type": type(error).__name__, "reason": getattr(error, "reason", None), "field_path": getattr(error, "field_path", None), "message": str(error)}
    raise AssertionError("invalid call was accepted")


def summarize(operations: list[str], rows: dict[str, Any]) -> dict[str, int]:
    """Validate exhaustive outcomes and return disjoint coverage counts."""
    if len(operations) != len(set(operations)) or set(operations) != set(rows):
        raise ValueError("operation set differs from canonical map")
    counts = {"supported": len(operations), "executed": 0, "failed": 0, "unavailable": 0, "gap": 0}
    for row in rows.values():
        status = row["status"]
        if status not in counts or status == "supported":
            raise ValueError("unknown operation outcome")
        if status == "executed" and not all(row.get(key) for key in ("positive", "negative", "reopen")):
            raise ValueError("executed operation omitted asserted evidence")
        if status in ("gap", "unavailable") and not (row.get("reason") and row.get("owner")):
            raise ValueError("unexplained operation gap")
        if status == "failed" and not row.get("error"):
            raise ValueError("failed operation omitted error")
        counts[status] += 1
    counts["unexecuted"] = counts["unavailable"] + counts["gap"]
    return counts


def operation_result(operation: str, positive: str, negative: str, cases: dict[str, Any]) -> dict[str, Any]:
    """Attribute case and per-operation failures without contaminating sibling calls."""
    for case in (positive, negative):
        value = cases[case]
        if operation in value.get("operation_errors", {}):
            return {"status": "failed", "error": value["operation_errors"][operation], "case": case}
        if value["status"] == "failed":
            return {"status": "failed", "error": value["error"], "case": case}
    return {"status": "executed", "positive": {"case": positive}, "negative": {"case": negative}, "reopen": {"case": positive, "checks": cases[positive]["reopen"]}}


def _hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _identity(wheel: Path, digest: str) -> dict[str, Any]:
    import fathomdb
    import fathomdb._fathomdb as native
    from importlib.metadata import distribution

    if _hash(wheel) != digest:
        raise ValueError("wheel SHA-256 mismatch")
    if sys.flags.optimize:
        raise ValueError("optimized Python disables contract assertions")
    root = Path(fathomdb.__file__).resolve().parent
    native_path = Path(native.__file__).resolve()
    if "site-packages" not in root.parts or not root.is_relative_to(Path(sys.prefix).resolve()):
        raise ValueError("SDK import escaped selected installed environment")
    direct = json.loads(distribution("fathomdb").read_text("direct_url.json") or "{}")
    if direct.get("dir_info", {}).get("editable"):
        raise ValueError("editable SDK is not installed-wheel evidence")
    checked = []
    with zipfile.ZipFile(wheel) as archive:
        for member in archive.namelist():
            if member.startswith("fathomdb/") and not member.endswith("/"):
                installed = root.parent / member
                if not installed.is_file() or installed.read_bytes() != archive.read(member):
                    raise ValueError(f"installed file differs from wheel: {member}")
                checked.append(member)
    if not native_path.is_relative_to(root):
        raise ValueError("native import escaped verified wheel package")
    return {"wheel": str(wheel.resolve()), "wheel_sha256": digest, "native": str(native_path), "native_sha256": _hash(native_path), "module": str(root / "__init__.py"), "verified_members": checked, "distribution_version": distribution("fathomdb").version, "direct_url": direct, "python_executable": sys.executable, "python_version": sys.version, "platform": platform.platform()}


def _snapshot(database: Path) -> dict[str, Any]:
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        tables = [item[0] for item in connection.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name") if item[0].startswith(("canonical_", "operational_")) or item[0] in {"_fathomdb_projection_registry", "_fathomdb_source_dependencies", "_fathomdb_source_versions", "_fathomdb_source_links", "_fathomdb_actuation_receipts"}]
        return {name: connection.execute('SELECT * FROM "' + name + '" ORDER BY rowid').fetchall() for name in tables}


def _reopen(database: Path) -> dict[str, Any]:
    import fathomdb
    before = _snapshot(database)
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        assert _snapshot(database) == before, "reopen changed canonical/operational state"
        with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
            active = connection.execute("SELECT logical_id, body FROM canonical_nodes WHERE logical_id IS NOT NULL AND state='active' AND superseded_at IS NULL ORDER BY logical_id").fetchall()
        for logical_id, body in active:
            row = fathomdb.read.get(engine, logical_id, view=fathomdb.ReadView(include_out_of_window=True))
            assert row is not None and row.body == body, "reopen SDK read differs from canonical bytes"
        assert fathomdb.read.get(engine, "slice135-guaranteed-absent") is None
        return {"canonical_operational_sha256": hashlib.sha256(repr(before).encode()).hexdigest(), "table_counts": {name: len(rows) for name, rows in before.items()}, "active_bodies_checked": len(active), "active_bodies": active, "persisted_rows": before, "missing_id": None}
    finally:
        engine.close()


def _existing_case(repo: Path, case: str, temporary: Path) -> dict[str, Any]:
    filename, name = CASES[case]
    path = repo / "src/python/tests" / filename
    spec = importlib.util.spec_from_file_location("capability_" + case, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    function = getattr(module, name)
    parameters = inspect.signature(function).parameters
    if set(parameters) - {"db_path", "tmp_path"}:
        raise ValueError("case requests forbidden fixtures")
    arguments = {key: str(temporary / "contract.sqlite") if key == "db_path" else temporary for key in parameters}
    function(**arguments)
    databases = sorted(path for path in temporary.rglob("*") if path.is_file() and path.read_bytes()[:16] == b"SQLite format 3\0")
    assert databases, "case produced no real SQLite database"
    return {"test": str(path.relative_to(repo)) + "::" + name, "test_sha256": _hash(path), "assertions": "unchanged inspected source assertions passed", "reopen": [_reopen(path) for path in databases]}


def _extra_case(name: str, temporary: Path) -> dict[str, Any]:
    import fathomdb as f
    from dataclasses import asdict, replace
    import slice135_python_s02 as s02

    database = temporary / "extra.sqlite"
    engine = f.Engine.open(str(database), use_default_embedder=False)
    observed: dict[str, Any] = {}
    try:
        operation_errors: dict[str, str] = {}
        if name == "embed_restart":
            engine.close()
            engine = f.Engine.open(str(database), use_default_embedder=True)
            identity = asdict(engine.open_report().default_embedder)
            vector = engine.embed("the central bank raised interest rates")
            assert len(vector) == 384 and any(vector)
            engine.close()
            engine = f.Engine.open(str(database), use_default_embedder=True)
            assert asdict(engine.open_report().default_embedder) == identity
            assert engine.embed("the central bank raised interest rates") == vector
            observed["engine.embed"] = {"identity": identity, "dimension": len(vector), "vector_sha256": hashlib.sha256(json.dumps(vector).encode()).hexdigest(), "reopened_vector_identical": True}
        elif name == "boundary_errors":
            calls = {
                "engine.open": lambda: refusal(lambda: f.Engine.open(str(temporary / "refused.sqlite"), config=f.EngineConfig(), slow_threshold_ms=10), ValueError),
                "admin.configure": lambda: refusal(lambda: f.admin.configure(engine, name="", body="{}"), ValueError),
                "engine.write": lambda: refusal(lambda: engine.write([{"kind": "doc", "body": "body", "source_id": ""}]), f.errors.WriteValidationError),
                "read.get": lambda: refusal(lambda: f.read.get(engine, ""), ValueError),
                "read.get_many": lambda: {"missing": f.read.get_many(engine, ["missing", "missing"])},
                "read.collection": lambda: refusal(lambda: f.read.collection(engine, "log", limit=-1), ValueError),
                "read.mutations": lambda: refusal(lambda: f.read.mutations(engine, "log", limit=-1), ValueError),
                "engine.search": lambda: refusal(lambda: engine.search("bad\0query"), f.errors.WriteValidationError),
                "engine.search_text_only": lambda: refusal(lambda: engine.search_text_only("bad\0query"), f.errors.WriteValidationError),
                "engine.search_projected_text": lambda: refusal(lambda: engine.search_projected_text("bad\0query", "missing"), f.errors.WriteValidationError),
                "engine.freeze_read_context": lambda: refusal(lambda: engine.freeze_read_context(f.ReadContextV1(schema_version=2)), f.errors.FrozenReadError, reason="unsupported_schema_version", field_path="/schemaVersion"),
                "read.crossed_boundary_since": lambda: refusal(lambda: f.read.crossed_boundary_since(engine, True), ValueError),
            }
            for operation, call in calls.items():
                try:
                    observed[operation] = call()
                    if operation == "read.get_many":
                        assert observed[operation]["missing"] == [None, None]
                except Exception:
                    operation_errors[operation] = traceback.format_exc()
            assert not (temporary / "refused.sqlite").exists()
            engine.close()
            engine.close()
            observed["engine.close"] = {"idempotent_close": True}
        elif name == "closed_reads":
            engine.close()
            for op in ("projections", "projection_status", "embedding_readiness", "projection_generation_status"):
                observed["read." + op] = refusal(lambda op=op: getattr(f.read, op)(engine), f.errors.ClosingError)
        elif name in ("dependency_errors", "trace", "evidence_refusals", "graph_refusals", "search_text"):
            engine.write(s02.make_graph_records())
            if name == "dependency_errors":
                for op, request in (("register_source_dependency", {"schema_version": 2, "dependency_id": "x", "source_revision_id": "s02-source-r1", "derived_revision_id": "s02-claim-r1"}), ("dependencies_for_source", {"schema_version": 2, "source_revision_id": "s02-source-r1"}), ("dependency_for_derived", {"schema_version": 2, "derived_revision_id": "s02-claim-r1"})):
                    observed["engine." + op] = refusal(lambda op=op, request=request: getattr(engine, op)(request), f.errors.DependencyError, reason="unsupported_schema_version", field_path="/schemaVersion")
            elif name == "trace":
                registration = engine.register_source_dependency({"schema_version": 1, "dependency_id": "trace-dependency", "source_revision_id": "s02-source-r1", "derived_revision_id": "s02-claim-r1"})
                context = engine.freeze_read_context(f.ReadContextV1())
                request = f.DependencyTraceRequestV1(root_revision_id="s02-source-r1", direction="to_dependents", context=context)
                result = engine.trace_dependency(request)
                assert result.complete is True
                assert [(edge.dependency_id, edge.source_revision_id, edge.derived_revision_id) for edge in result.dependency_edges] == [(registration.dependency_id, "s02-source-r1", "s02-claim-r1")]
                observed["engine.trace_dependency"] = {"dependency_edges": [registration.dependency_id], "complete": True, "refusal": refusal(lambda: engine.trace_dependency(replace(request, max_relations=0)), f.errors.DependencyTraceError)}
            elif name == "search_text":
                result = engine.search_text_only(s02.CLAIM_TOKEN)
                assert [hit.id.value for hit in result.results] == ["s02-claim"]
                observed["engine.search_text_only"] = {"ids": [hit.id.value for hit in result.results], "branches": [hit.branch for hit in result.results]}
            elif name == "graph_refusals":
                observed["graph.search_expand"] = refusal(lambda: f.graph.search_expand(engine, s02.CLAIM_TOKEN, depth=4), f.errors.InvalidArgumentError)
                context = engine.freeze_read_context(f.ReadContextV1())
                request = f.GraphExpandRequestV1(schema_version=2, seed=f.GraphExplicitSeedV1(schema_version=1, type="explicit", logical_ids=(f.IdSpace(space="logical", value="s02-root"),)), context=f.FrozenGraphReadContextV1(schema_version=1, type="frozen", context=context), max_depth=1, direction="outgoing", edge_kinds=("supports",), target_kinds=("doc",), result_limit=1, max_work_units="10", include_explanation=False)
                observed["graph.expand"] = refusal(lambda: f.graph.expand(engine, request), f.errors.GraphExpansionError)
            else:
                context = engine.freeze_read_context(f.ReadContextV1())
                observed["engine.resolve_evidence"] = refusal(lambda: engine.resolve_evidence(f.EvidenceResolveRequestV1(evidence_ref="invalid", context=context)), f.errors.EvidenceError, reason="evidence_unavailable")
                observed["engine.resolve_graph_evidence"] = refusal(lambda: engine.resolve_graph_evidence(f.GraphEvidenceResolveRequestV1(evidence_ref="invalid", context=context)), f.errors.EvidenceError, reason="evidence_unavailable")
        elif name == "closure":
            request = {"schema_version": 1, "closure_operation_id": "_fdb:c:" + "a" * 64}
            assert engine.read_dependency_closure(request) is None
            observed["engine.read_dependency_closure"] = {"absent_closure": None, "refusal": refusal(lambda: engine.read_dependency_closure({**request, "schema_version": 2}), f.errors.DependencyClosureError, reason="unsupported_schema_version", field_path="/schemaVersion")}
        elif name == "erasure":
            engine.write(s02.make_graph_records())
            report = engine.erase_source(s02.GRAPH_SOURCE)
            assert (report.nodes_excised, report.edges_excised) == (3, 1)
            retry = engine.erase_source(s02.GRAPH_SOURCE)
            assert (retry.nodes_excised, retry.edges_excised) == (0, 0)
            assert f.read.get(engine, "s02-claim") is None
            observed["engine.erase_source"] = {"nodes_excised": 3, "edges_excised": 1, "retry_counts": [0, 0], "refusal": refusal(lambda: engine.erase_source(""), f.errors.WriteValidationError)}
        elif name == "projection_status_error":
            observed["read.mutation_projection_status"] = refusal(lambda: f.read.mutation_projection_status(engine, {"schemaVersion": 2, "operationId": "missing", "writeCursor": "1", "expectedGenerationId": "missing"}), f.errors.ProjectionGenerationError, reason="unsupported_schema_version")
        elif name == "operational_errors":
            frozen = engine.freeze_read_context(f.ReadContextV1())
            observed["read.operational_state"] = refusal(lambda: f.read.operational_state(engine, "unregistered", "key", frozen), f.errors.PageError)
            observed["read.operational_state_page"] = refusal(lambda: f.read.operational_state_page(engine, "unregistered", frozen, f.PageRequestV1(limit=0)), f.errors.PageError, reason="invalid_page_limit")
        else:
            raise ValueError("unknown extra case")
    finally:
        engine.close()
    return {"assertions": observed, "operation_errors": operation_errors, "reopen": [_reopen(database)]}


def run(repo: Path, wheel: Path, wheel_sha256: str, source_sha: str) -> dict[str, Any]:
    """Execute every planned operation and retain failures without losing coverage."""
    import pytest
    if not Path(pytest.__file__).resolve().is_relative_to(Path(sys.prefix).resolve()):
        raise ValueError("pytest must be installed in selected wheel environment")
    identity = _identity(wheel, wheel_sha256)
    subprocess.run(["git", "-C", str(repo), "cat-file", "-e", source_sha + "^{commit}"], check=True)
    for scope in ("src/rust", "src/python/fathomdb", "Cargo.lock"):
        if subprocess.check_output(["git", "-C", str(repo), "diff", source_sha, "--", scope]):
            raise ValueError("product source differs from declared build commit: " + scope)
    product_tree = subprocess.check_output(["git", "-C", str(repo), "rev-parse", source_sha + ":src/rust"], text=True).strip()
    map_path = repo / "src/conformance/governed-operation-parity.json"
    operations = [row["id"] for row in json.loads(map_path.read_text())["operations"] if row["state"] == "live"]
    cases: dict[str, Any] = {}
    required = sorted({case for pair in PLANNED.values() for case in pair} | {"embed_determinism", "embed_restart", "frozen_drift"})
    for name in required:
        output = io.StringIO()
        with TemporaryDirectory(prefix="slice135-wheel-capability-" + name + "-") as directory:
            try:
                with redirect_stdout(output), redirect_stderr(output):
                    value = _existing_case(repo, name, Path(directory)) if name in CASES else _extra_case(name, Path(directory))
                cases[name] = {"status": "partial_failed" if value.get("operation_errors") else "passed", **value, "output": output.getvalue()}
            except BaseException as error:
                if isinstance(error, (KeyboardInterrupt, SystemExit)):
                    raise
                cases[name] = {"status": "failed", "error": traceback.format_exc(), "output": output.getvalue()}
        print(json.dumps({"case": name, "status": cases[name]["status"]}), flush=True)
    rows: dict[str, Any] = {}
    for op, (positive, negative) in PLANNED.items():
        rows[op] = operation_result(op, positive, negative, cases)
    for op, reason in UNAVAILABLE.items():
        rows[op] = {"status": "unavailable", **reason}
    counts = summarize(operations, rows)
    fixture_paths = subprocess.check_output(["git", "-C", str(repo), "ls-files", "dev/fixtures/*.json", "src/conformance/*.json", "src/python/tests/*.json"], text=True).splitlines()
    fixture_hashes = {path: _hash(repo / path) for path in fixture_paths}
    return {"schema_version": "fathomdb.slice135-python-capabilities/v1", "status": "INTERIM_INSTALLED_WHEEL_CAPABILITY_EXERCISE", "finished_utc": datetime.now(timezone.utc).isoformat(), "product_source_sha": source_sha, "product_rust_tree": product_tree, "product_python_tree": subprocess.check_output(["git", "-C", str(repo), "rev-parse", source_sha + ":src/python/fathomdb"], text=True).strip(), "cargo_lock_sha256": _hash(repo / "Cargo.lock"), "runner_sha256": _hash(Path(__file__)), "oracle_fixtures_sha256": fixture_hashes, "s02_helper_sha256": _hash(Path(__file__).with_name("slice135_python_s02.py")), "s01_helper_sha256": _hash(Path(__file__).with_name("slice135_python_s01.py")), "operation_map_sha256": _hash(map_path), "artifact": identity, "counts": counts, "operations": rows, "cases": cases, "contract_gaps": ["This is a selected operation exercise, not complete contract qualification.", "Generic reopen checks compare canonical/operational SQLite rows and active SDK read bodies; they do not replay every original operation after restart.", "No platform beyond this recorded Linux wheel, no provider success or standalone reranker, no concurrent/cancellation matrix.", "Search positive cases exercise lexical and projected paths; qualified vector/hybrid S01 and S02 receipts remain separate."]}


def main() -> None:
    """Write a new receipt and exit nonzero if any operation case failed."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("refusing to overwrite receipt")
    if len(args.source_sha) != 40 or any(c not in "0123456789abcdef" for c in args.source_sha):
        raise ValueError("source SHA must be forty lowercase hex digits")
    result = run(args.repo.resolve(), args.wheel.resolve(), args.wheel_sha256, args.source_sha)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["counts"], sort_keys=True))
    if result["counts"]["failed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
