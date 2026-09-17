"""Executable canonical-operation parity oracle for the Python SDK."""

from __future__ import annotations

import importlib.util
import inspect
import json
from pathlib import Path

import fathomdb
from fathomdb import Engine, admin, graph, read


ROOT = Path(__file__).resolve().parents[3]
COMPANION = ROOT / "src" / "conformance" / "governed-operation-parity.json"
SIGNED = ROOT / "src" / "conformance" / "governed-surface-allowlist.json"
CHECKER = ROOT / "scripts" / "check-sdk-surface-parity.py"

ENGINE_NON_COMMAND = {
    "attach_logging_subscriber",
    "config",
    "counters",
    "dense_disabled",
    "dense_disabled_reason",
    "drain",
    "enable_telemetry",
    "last_telemetry_query_id",
    "open_report",
    "path",
    "record_feedback",
    "set_profiling",
    "set_slow_threshold_ms",
    "vector_equivalence_refusal_count",
}
PACKAGE_NON_COMMAND = {"embed_batch_cls"}
ADMIN_NON_COMMAND = {"configure_runtime"}


def load_checker():
    spec = importlib.util.spec_from_file_location("check_sdk_surface_parity", CHECKER)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def observed_python_surface() -> set[str]:
    observed: set[str] = set()
    for name in getattr(fathomdb, "__all__", ()):
        if name not in PACKAGE_NON_COMMAND and inspect.isroutine(getattr(fathomdb, name, None)):
            observed.add(f"package:{name}")
    for name in dir(Engine):
        if name.startswith("_") or name in ENGINE_NON_COMMAND:
            continue
        if not callable(getattr(Engine, name, None)):
            continue
        locator = "engine_static" if name == "open" else "engine_instance"
        observed.add(f"{locator}:{name}")
    for namespace, locator, exclusions in (
        (admin, "admin", ADMIN_NON_COMMAND),
        (read, "read", set()),
        (graph, "graph", set()),
    ):
        for name in getattr(namespace, "__all__", ()):
            if name in exclusions:
                continue
            if inspect.isfunction(getattr(namespace, name, None)):
                observed.add(f"{locator}:{name}")
    return observed


def test_python_surface_equals_live_canonical_operations() -> None:
    checker = load_checker()
    with SIGNED.open(encoding="utf-8") as fh:
        signed = json.load(fh)
    with COMPANION.open(encoding="utf-8") as fh:
        companion = json.load(fh)
    checker.validate_contract(signed, companion)
    canonical = checker.validate_observed(companion, "python", observed_python_surface())
    assert canonical == checker.live_canonical_ids(companion)
