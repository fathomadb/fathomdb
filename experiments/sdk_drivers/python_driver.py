"""One-repetition benchmark driver through the public FathomDB Python SDK.

Run with ``python -m experiments.sdk_drivers.python_driver <request.json>``.
The coordinator owns process isolation, fixture creation, and result receipts.
"""

from __future__ import annotations

import contextlib
import hashlib
import importlib
import json
import math
import platform
import sys
import time
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "sdk.driver.v1"
_OPERATIONS = (
    ("engine.search_text_only", 1),
    ("engine.search_projected_text", 1),
    ("engine.search", 1),
    ("read.get", 1),
    ("graph.neighbors", 1),
)
_ID_PREFIX = {"logical": "l:", "content": "h:", "passage": "p:"}
_SAFE_ERROR_FIELDS = (
    "kind",
    "stage",
    "recovery_hint_code",
    "stored",
    "supplied",
    "reason",
    "requested_mode",
    "effective_mode",
    "sqlite_code",
    "field_path",
)


class DriverRefusal(Exception):
    """A typed preflight or protocol refusal before a successful repetition."""

    def __init__(self, kind: str) -> None:
        self.kind = kind
        super().__init__(kind)


def exact_timeout_seconds(timeout_ms: int) -> float:
    """Translate an integer deadline through native ``int(seconds * 1000)``.

    Raises ``ValueError`` for invalid or unrepresentable millisecond values.
    """

    if (
        isinstance(timeout_ms, bool)
        or not isinstance(timeout_ms, int)
        or timeout_ms < 0
    ):
        raise ValueError("timeout_ms must be a non-negative integer")
    try:
        seconds = timeout_ms / 1000
    except OverflowError as exc:
        raise ValueError("timeout_ms is not representable") from exc
    if not math.isfinite(seconds):
        raise ValueError("timeout_ms is not representable")
    for _ in range(16):
        actual = int(seconds * 1000)
        if actual == timeout_ms:
            return seconds
        seconds = math.nextafter(
            seconds, math.inf if actual < timeout_ms else -math.inf
        )
    raise ValueError("timeout_ms is not representable")


def _response(request: object) -> dict[str, Any]:
    raw = request if isinstance(request, dict) else {}
    return {
        "schema_version": SCHEMA_VERSION,
        "status": "blocked",
        "sdk": "python",
        "operation_id": raw.get("operation_id"),
        "operation_version": raw.get("operation_version"),
        "identity": None,
        "effective_options": {},
        "samples": [],
        "error": None,
    }


def _require_dict(value: object) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise DriverRefusal("invalid_request")
    return value


def _require_int(value: object, *, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise DriverRefusal("invalid_request")
    return value


def _decode(request: object) -> dict[str, Any]:
    raw = _require_dict(request)
    if (
        raw.get("schema_version") != SCHEMA_VERSION
        or raw.get("mode") != "run-repetition"
    ):
        raise DriverRefusal("invalid_request")
    if raw.get("sdk") != "python":
        raise DriverRefusal("invalid_request")
    if (raw.get("operation_id"), raw.get("operation_version")) not in _OPERATIONS:
        raise DriverRefusal("unsupported_operation")
    path = raw.get("database_path")
    if not isinstance(path, str) or not path or not Path(path).is_absolute():
        raise DriverRefusal("invalid_request")
    expected_sha = raw.get("expected_native_sha256")
    if (
        not isinstance(expected_sha, str)
        or len(expected_sha) != 64
        or any(char not in "0123456789abcdef" for char in expected_sha)
    ):
        raise DriverRefusal("invalid_request")
    queries = raw.get("queries")
    if not isinstance(queries, list) or not queries:
        raise DriverRefusal("invalid_request")
    seen: set[str] = set()
    for query in queries:
        item = _require_dict(query)
        query_id, query_text = item.get("id"), item.get("text")
        if (
            not isinstance(query_id, str)
            or not query_id
            or query_id in seen
            or not isinstance(query_text, str)
            or not query_text
        ):
            raise DriverRefusal("invalid_request")
        seen.add(query_id)
    args = _require_dict(raw.get("arguments"))
    operation_id = raw["operation_id"]
    expected_args = {
        "engine.search_text_only": {"limit", "view"},
        "engine.search_projected_text": {"limit", "view", "name", "filter"},
        "engine.search": {"limit", "view", "use_default_embedder"},
        "read.get": {"view"},
        "graph.neighbors": {"depth", "view"},
    }[operation_id]
    if set(args) != expected_args:
        raise DriverRefusal("invalid_request")
    normalizer = raw.get("normalizer", "ordered_ids")
    if normalizer not in {"ordered_ids", "ordered_hits"}:
        raise DriverRefusal("invalid_request")
    name = args.get("name")
    if "name" in args and (not isinstance(name, str) or not name):
        raise DriverRefusal("invalid_request")
    filter_attributes = None
    if args.get("filter") is not None:
        flt = _require_dict(args["filter"])
        pairs = flt.get("attributes")
        if set(flt) != {"attributes"} or not isinstance(pairs, list) or any(
            not isinstance(pair, list) or len(pair) != 2 or not all(isinstance(v, str) for v in pair)
            for pair in pairs
        ):
            raise DriverRefusal("invalid_request")
        filter_attributes = tuple(tuple(pair) for pair in pairs)
    use_default_embedder = args.get("use_default_embedder", False)
    if not isinstance(use_default_embedder, bool):
        raise DriverRefusal("invalid_request")
    limit = _require_int(args["limit"], minimum=1) if "limit" in args else None
    depth = _require_int(args["depth"], minimum=1) if "depth" in args else None
    if depth is not None and depth > 3:
        raise DriverRefusal("invalid_request")
    view = _require_dict(args["view"])
    if set(view) != {"valid_as_of"}:
        raise DriverRefusal("invalid_request")
    valid_as_of = _require_int(view["valid_as_of"])
    warmup = _require_int(raw.get("warmup_per_query"))
    timeout_ms = _require_int(raw.get("timeout_ms"))
    try:
        timeout_s = exact_timeout_seconds(timeout_ms)
    except ValueError as exc:
        raise DriverRefusal("unrepresentable_timeout") from exc
    return {
        "operation_id": operation_id,
        "database_path": path,
        "queries": queries,
        "limit": limit,
        "depth": depth,
        "valid_as_of": valid_as_of,
        "warmup_per_query": warmup,
        "timeout_ms": timeout_ms,
        "timeout_s": timeout_s,
        "expected_native_sha256": expected_sha,
        "normalizer": normalizer,
        "name": name,
        "filter": args.get("filter"),
        "filter_attributes": filter_attributes,
        "use_default_embedder": use_default_embedder,
    }


def _sdk_identity(sdk: Any, expected_sha: str) -> dict[str, str]:
    package_file = Path(sdk.__file__).resolve(strict=True)
    native_file = Path(sdk._fathomdb.__file__).resolve(strict=True)
    if package_file.name != "__init__.py" or native_file.parent != package_file.parent:
        raise DriverRefusal("sdk_import_origin_mismatch")
    digest = hashlib.sha256(native_file.read_bytes()).hexdigest()
    if digest != expected_sha:
        raise DriverRefusal("native_identity_mismatch")
    version = sdk.__version__
    if not isinstance(version, str) or not version:
        raise DriverRefusal("sdk_version_missing")
    return {
        "sdk_version": version,
        "runtime": f"Python {sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
        "native_sha256": digest,
    }


def _normalized_hits(result: Any) -> list[dict[str, Any]]:
    hits: list[dict[str, Any]] = []
    for hit in result:
        prefix = _ID_PREFIX.get(hit.id.space)
        if prefix is None or not isinstance(hit.id.value, str):
            raise DriverRefusal("unsupported_id_space")
        hits.append({"id": prefix + hit.id.value, "score": float(hit.score), "branch": str(hit.branch)})
    return hits


def _normalized_ids(result: Any, operation_id: str) -> list[str]:
    ids: list[str] = []
    if operation_id in {"engine.search_text_only", "engine.search_projected_text", "engine.search"}:
        for hit in result:
            id_space = hit.id
            prefix = _ID_PREFIX.get(id_space.space)
            if prefix is None or not isinstance(id_space.value, str):
                raise DriverRefusal("unsupported_id_space")
            ids.append(prefix + id_space.value)
    else:
        for node in result:
            logical_id = node.logical_id
            if not isinstance(logical_id, str):
                raise DriverRefusal("unsupported_id_space")
            ids.append("l:" + logical_id)
    return ids


def _search_text_only(
    sdk: Any, engine: Any, text: str, view: Any, options: dict[str, Any]
) -> tuple[Any, ...]:
    return tuple(
        engine.search_text_only(text, view=view, limit=options["limit"]).results
    )


def _search_projected_text(
    sdk: Any, engine: Any, text: str, view: Any, options: dict[str, Any]
) -> tuple[Any, ...]:
    attributes = options["filter_attributes"]
    flt = None if attributes is None else sdk.SearchFilter(attributes=attributes)
    return tuple(
        engine.search_projected_text(text, options["name"], flt, view=view, limit=options["limit"]).results
    )


def _search(
    sdk: Any, engine: Any, text: str, view: Any, options: dict[str, Any]
) -> tuple[Any, ...]:
    return tuple(engine.search(text, view=view, limit=options["limit"]).results)


def _read_get(
    sdk: Any, engine: Any, text: str, view: Any, options: dict[str, Any]
) -> tuple[Any, ...]:
    node = sdk.read.get(engine, text, view=view)
    return () if node is None else (node,)


def _graph_neighbors(
    sdk: Any, engine: Any, text: str, view: Any, options: dict[str, Any]
) -> tuple[Any, ...]:
    return tuple(sdk.graph.neighbors(engine, text, options["depth"], view=view))


_BINDINGS = {
    "engine.search_text_only": _search_text_only,
    "engine.search_projected_text": _search_projected_text,
    "engine.search": _search,
    "read.get": _read_get,
    "graph.neighbors": _graph_neighbors,
}


def _typed_error(exc: Exception, sdk: Any | None) -> dict[str, Any]:
    if isinstance(exc, DriverRefusal):
        return {"kind": exc.kind}
    if isinstance(exc, (TypeError, ValueError)):
        return {"kind": "invalid_request"}
    if sdk is not None and isinstance(exc, sdk.errors.EngineError):
        payload: dict[str, Any] = {"kind": type(exc).__name__}
        fields: dict[str, Any] = {}
        for field in _SAFE_ERROR_FIELDS:
            value = getattr(exc, field, None)
            if isinstance(value, (bool, int, float, str)) and field != "reason":
                fields[field] = value
        if fields:
            payload["fields"] = fields
        return payload
    return {"kind": "driver_failure"}


def _peak_rss_bytes() -> int | None:
    if platform.system() != "Linux":
        return None
    try:
        status = Path("/proc/self/status").read_text(encoding="ascii")
    except OSError:
        return None
    for line in status.splitlines():
        if line.startswith("VmHWM:"):
            fields = line.split()
            if len(fields) == 3 and fields[2] == "kB" and fields[1].isdigit():
                return int(fields[1]) * 1024
    return None


def execute(request: object, *, sdk: Any | None = None) -> dict[str, Any]:
    """Execute one fixed protocol request; dependency injection is for tests only."""

    response = _response(request)
    try:
        raw = _require_dict(request)
        if raw.get("schema_version") != SCHEMA_VERSION:
            raise DriverRefusal("invalid_request")
        if raw.get("mode") == "capabilities":
            response.update(
                status="complete",
                operations=[
                    {"operation_id": operation_id, "operation_version": version}
                    for operation_id, version in _OPERATIONS
                ],
            )
            return response
        decoded = _decode(raw)
        if sdk is None:
            try:
                sdk = importlib.import_module("fathomdb")
            except Exception as exc:
                raise DriverRefusal("sdk_import_failure") from exc
        response["identity"] = _sdk_identity(sdk, decoded["expected_native_sha256"])
        response["effective_options"] = {
            "view": {"valid_as_of": decoded["valid_as_of"]},
            "warmup_per_query": decoded["warmup_per_query"],
            "requested_timeout_ms": decoded["timeout_ms"],
            "timeout_s": decoded["timeout_s"],
            "effective_native_timeout_ms": int(decoded["timeout_s"] * 1000),
            "sqlite_mode": "performance",
        }
        if decoded["limit"] is not None:
            response["effective_options"]["limit"] = decoded["limit"]
        if decoded["depth"] is not None:
            response["effective_options"]["depth"] = decoded["depth"]
        if decoded["operation_id"] == "engine.search_projected_text":
            response["effective_options"].update(name=decoded["name"], filter=decoded["filter"])
        if decoded["operation_id"] == "engine.search":
            response["effective_options"]["use_default_embedder"] = decoded["use_default_embedder"]
        sdk.admin.configure_runtime(sqlite_mode="performance")
        engine = (
            sdk.Engine.open(decoded["database_path"], use_default_embedder=True)
            if decoded["use_default_embedder"]
            else sdk.Engine.open(decoded["database_path"])
        )
        run_error: Exception | None = None
        samples: list[dict[str, Any]] = []
        try:
            if hasattr(engine, "open_report"):
                report = engine.open_report()
                response["effective_options"]["query_backend"] = report.query_backend
                response["effective_options"]["dense_disabled"] = report.dense_disabled
            if hasattr(engine, "drain"):
                engine.drain(timeout_s=decoded["timeout_s"])
            binding = _BINDINGS[decoded["operation_id"]]
            for query in decoded["queries"]:
                view = sdk.ReadView(valid_as_of=decoded["valid_as_of"])
                for _ in range(decoded["warmup_per_query"]):
                    binding(sdk, engine, query["text"], view, decoded)
                start_ns = time.perf_counter_ns()
                materialized = binding(sdk, engine, query["text"], view, decoded)
                elapsed_ns = time.perf_counter_ns() - start_ns
                sample = {
                    "query_id": query["id"],
                    "elapsed_ns": elapsed_ns,
                    "ids": _normalized_ids(materialized, decoded["operation_id"]),
                }
                if decoded["normalizer"] == "ordered_hits":
                    sample["hits"] = _normalized_hits(materialized)
                samples.append(sample)
        except Exception as exc:
            run_error = exc
        finally:
            try:
                engine.close()
            except Exception:
                run_error = DriverRefusal("close_failure")
        if run_error is not None:
            raise run_error
        response.update(status="complete", samples=samples)
        peak_rss = _peak_rss_bytes()
        if peak_rss is not None:
            response["resources"] = {"driver_peak_rss_bytes": peak_rss}
    except Exception as exc:
        response["status"] = "blocked"
        response["samples"] = []
        response["error"] = _typed_error(exc, sdk)
    return response


def main(argv: list[str] | None = None) -> int:
    """Read one request file and emit exactly one JSON protocol response."""

    args = sys.argv[1:] if argv is None else argv
    try:
        if len(args) != 1:
            raise DriverRefusal("invalid_request")
        request = json.loads(Path(args[0]).read_text(encoding="utf-8"))
        with contextlib.redirect_stdout(sys.stderr):
            response = execute(request)
    except (OSError, UnicodeError, json.JSONDecodeError, DriverRefusal) as exc:
        response = _response(None)
        response["error"] = _typed_error(exc, None)
    print(json.dumps(response, separators=(",", ":"), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
