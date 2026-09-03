"""Executable measurement-layer classifications for experiment receipts.

This module is intentionally evaluation-side. It validates source-bound,
closed classification documents and never changes FathomDB product behavior.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import subprocess
import sys
import uuid
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Iterable


SCHEMA_VERSION = "measurement.classification.v1"
PLAN_VERSION = "measurement.plan.v1"
CLASSIFIER_VERSION = "1"
SIDECAR_NAME = "measurement-classification.v1.json"
POLICY_NAME = "measurement-classification-policy.v1.json"
POLICY_VERSION = "measurement.classification-policy.v1"
HISTORICAL_VERSION = "measurement.classification-historical.v1"

LAYERS = ("data_plane", "semantic_control_plane", "end_to_end")
LAYER_RANK = {name: rank for rank, name in enumerate(LAYERS)}
COMPONENT_LAYERS = {
    "corpus_gold": "data_plane",
    "fathomdb_storage": "data_plane",
    "fathomdb_engine_search": "data_plane",
    "deterministic_metric": "data_plane",
    "external_retrieval": "data_plane",
    "query_planner": "semantic_control_plane",
    "candidate_planner": "semantic_control_plane",
    "context_packer": "semantic_control_plane",
    "answer_generator": "end_to_end",
    "semantic_judge": "end_to_end",
}
ARTIFACT_ROLES = {
    "record",
    "metrics_payload",
    "checkpoint",
    "configuration",
    "implementation",
    "derivation_spec",
}
LOCATOR_KINDS = {"repository_path", "external_path", "git_blob"}
OPERATIONS = {"Engine.search", "external_retrieval"}
EVIDENCE_KINDS = {
    "instrumented_call",
    "source_result",
    "coverage_trace",
    "static_path_audit",
    "immutable_receipt",
}
BLOCKED_CODES = {
    "config_invalid_value",
    "fixture_invalid",
    "run_id_collision",
    "database_setup_failed",
    "engine_open_failed",
    "engine_search_failed",
    "search_call_count_mismatch",
    "expected_hit_missing",
    "record_write_failed",
    "source_artifact_unavailable",
}
EXCLUSION_REASONS = {"identifier_or_hash", "run_control", "cost_budget"}

INITIAL_RUN_ID = "global-01-native-comparison-20260829T1613Z-40685e82"
HELDOUT_RUN_ID = "global-01-lazy-coverage-20260829T2159Z-60b3642c"
HELDOUT_COMMIT = "f5c5715236bc4827f1753d8a1b3b95334590e677"
HISTORICAL_MANIFEST_NAME = "measurement-classification-global-01.v1.json"
HISTORICAL_AUDIT_PATH = Path(
    "experiments/audits/global-01-measurement-classification-source.v1.json"
)

TOP_KEYS = {
    "schema_version",
    "classifier_version",
    "classification_id",
    "run_id",
    "outcome",
    "blocked_reason",
    "measurement_plan_id",
    "source_artifacts",
    "components",
    "call_paths",
    "execution_witnesses",
    "metrics",
    "metric_exclusions",
    "comparisons",
    "claims",
    "migration",
}
PLAN_KEYS = {
    "schema_version",
    "plan_id",
    "components",
    "call_paths",
    "comparisons",
    "metric_bindings",
    "claims",
}


class ClassificationError(ValueError):
    """A classification is structurally invalid or contradicts its authority."""


def canonical_json(value: Any) -> str:
    """Return the canonical JSON representation used by Slice 10 hashes."""
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True)


def canonical_sha256(value: Any) -> str:
    """Hash canonical JSON with SHA-256."""
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def classification_id(document: dict[str, Any]) -> str:
    """Hash the complete classification body, excluding only its ID field."""
    body = copy.deepcopy(document)
    body.pop("classification_id", None)
    return canonical_sha256(body)


def _closed(mapping: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(mapping, dict):
        raise ClassificationError(f"{label} must be an object")
    actual = set(mapping)
    if actual != keys:
        raise ClassificationError(
            f"{label} unknown keys or missing keys: "
            f"unknown={sorted(actual - keys)}, missing={sorted(keys - actual)}"
        )
    return mapping


def _list(value: Any, label: str) -> list[Any]:
    if not isinstance(value, list):
        raise ClassificationError(f"{label} must be a list")
    return value


def _nonempty_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ClassificationError(f"{label} must be a nonempty string")
    return value


def _unique(rows: Iterable[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for row in rows:
        identity = _nonempty_string(row.get("id"), f"{label}.id")
        if identity in result:
            raise ClassificationError(f"duplicate {label} id: {identity}")
        result[identity] = row
    return result


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _artifact_bytes(row: dict[str, Any], repository_root: Path) -> bytes:
    kind = row["locator_kind"]
    locator = row["locator"]
    if kind == "git_blob":
        if ":" not in locator:
            raise ClassificationError("git_blob locator must be <commit>:<path>")
        completed = subprocess.run(
            ["git", "show", locator],
            cwd=repository_root,
            check=False,
            capture_output=True,
        )
        if completed.returncode:
            raise ClassificationError(f"source artifact unavailable: {locator}")
        return completed.stdout

    path = Path(locator)
    if kind == "repository_path":
        if path.is_absolute():
            raise ClassificationError("repository_path must be relative")
        root = repository_root.resolve()
        path = (root / path).resolve()
        if path != root and root not in path.parents:
            raise ClassificationError("repository_path escapes repository")
    else:
        path = path.resolve() if path.is_absolute() else (repository_root / path).resolve()
    try:
        return path.read_bytes()
    except OSError as exc:
        raise ClassificationError(f"source artifact unavailable: {locator}") from exc


def _json_pointer(value: Any, pointer: str) -> Any:
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        raise ClassificationError(f"invalid JSON Pointer: {pointer!r}")
    current = value
    for raw in pointer[1:].split("/"):
        token = raw.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict) and token in current:
            current = current[token]
        elif isinstance(current, list) and token.isdigit() and int(token) < len(current):
            current = current[int(token)]
        else:
            raise ClassificationError(f"JSON Pointer does not resolve: {pointer}")
    return current


def _leaf_pointers(value: Any, pointer: str) -> list[str]:
    if isinstance(value, dict):
        result: list[str] = []
        for key in sorted(value):
            escaped = key.replace("~", "~0").replace("/", "~1")
            result.extend(_leaf_pointers(value[key], f"{pointer}/{escaped}"))
        return result
    if isinstance(value, list):
        result = []
        for index, child in enumerate(value):
            result.extend(_leaf_pointers(child, f"{pointer}/{index}"))
        return result
    return [pointer]


def _maximum_layer(component_ids: list[str], components: dict[str, dict[str, Any]]) -> str:
    if not component_ids:
        raise ClassificationError("metric must have contributing components")
    return max(
        (COMPONENT_LAYERS[components[item]["kind"]] for item in component_ids),
        key=LAYER_RANK.__getitem__,
    )


def _validate_authority(authority: dict[str, Any]) -> None:
    _closed(authority, PLAN_KEYS, "measurement plan")
    if authority["schema_version"] != PLAN_VERSION:
        raise ClassificationError("unsupported measurement plan version")
    _nonempty_string(authority["plan_id"], "measurement plan id")
    _list(authority["components"], "measurement plan components")
    _list(authority["call_paths"], "measurement plan call_paths")
    _list(authority["comparisons"], "measurement plan comparisons")
    _list(authority["metric_bindings"], "measurement plan metric_bindings")
    _list(authority["claims"], "measurement plan claims")


def _validate_artifacts(
    rows: list[Any], repository_root: Path
) -> tuple[dict[str, dict[str, Any]], dict[str, Any]]:
    artifacts: list[dict[str, Any]] = []
    decoded: dict[str, Any] = {}
    for index, value in enumerate(rows):
        row = _closed(
            value,
            {
                "id",
                "locator_kind",
                "locator",
                "role",
                "sha256",
                "measurement_root_json_pointers",
            },
            f"source_artifacts[{index}]",
        )
        if row["locator_kind"] not in LOCATOR_KINDS:
            raise ClassificationError("unknown source artifact locator_kind")
        if row["role"] not in ARTIFACT_ROLES:
            raise ClassificationError("unknown source artifact role")
        roots = _list(
            row["measurement_root_json_pointers"], "measurement_root_json_pointers"
        )
        if row["role"] == "metrics_payload":
            if not roots:
                raise ClassificationError("metrics payload requires measurement roots")
        elif roots:
            raise ClassificationError("evidence-only artifact must have empty roots")
        payload = _artifact_bytes(row, repository_root)
        if _sha256_bytes(payload) != row["sha256"]:
            raise ClassificationError("source artifact SHA-256 mismatch")
        if row["role"] == "metrics_payload":
            try:
                decoded[row["id"]] = json.loads(payload)
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise ClassificationError("metrics payload must be JSON") from exc
        artifacts.append(row)
    return _unique(artifacts, "source artifact"), decoded


def _validate_components(rows: list[Any]) -> dict[str, dict[str, Any]]:
    components = []
    for index, value in enumerate(rows):
        row = _closed(value, {"id", "name", "kind"}, f"components[{index}]")
        if row["kind"] not in COMPONENT_LAYERS:
            raise ClassificationError("unknown component kind")
        _nonempty_string(row["name"], "component name")
        components.append(row)
    return _unique(components, "component")


def _validate_call_paths(
    rows: list[Any], artifact_ids: set[str]
) -> dict[str, dict[str, Any]]:
    paths = []
    for index, value in enumerate(rows):
        row = _closed(
            value, {"id", "operation", "source_artifact_ids"}, f"call_paths[{index}]"
        )
        if row["operation"] not in OPERATIONS:
            raise ClassificationError("unknown call path operation")
        sources = _list(row["source_artifact_ids"], "call path source artifacts")
        if not sources or not set(sources) <= artifact_ids:
            raise ClassificationError("call path source artifact reference invalid")
        paths.append(row)
    return _unique(paths, "call path")


def _validate_comparisons(
    rows: list[Any], component_ids: set[str]
) -> tuple[dict[str, dict[str, Any]], set[str]]:
    comparisons = []
    all_arms: set[str] = set()
    for index, value in enumerate(rows):
        row = _closed(
            value,
            {"id", "arms", "shared_component_ids", "differing_component_ids"},
            f"comparisons[{index}]",
        )
        arms = []
        for arm_index, arm_value in enumerate(_list(row["arms"], "comparison arms")):
            arm = _closed(
                arm_value,
                {"id", "component_ids", "execution_witness_ids"},
                f"comparison arm[{arm_index}]",
            )
            ids = _list(arm["component_ids"], "arm component_ids")
            if not set(ids) <= component_ids:
                raise ClassificationError("comparison component reference invalid")
            arms.append(arm)
        arm_map = _unique(arms, "comparison arm")
        if all_arms & set(arm_map):
            raise ClassificationError("comparison arm IDs must be globally unique")
        all_arms |= set(arm_map)
        if len(arms) < 2:
            raise ClassificationError("comparison requires at least two arms")
        component_sets = [set(arm["component_ids"]) for arm in arms]
        shared = set.intersection(*component_sets)
        differing = set.union(*component_sets) - shared
        if set(row["shared_component_ids"]) != shared:
            raise ClassificationError("shared component set is not derived")
        if set(row["differing_component_ids"]) != differing:
            raise ClassificationError("differing component set is not derived")
        comparisons.append(row)
    return _unique(comparisons, "comparison"), all_arms


def _validate_witnesses(
    rows: list[Any],
    paths: dict[str, dict[str, Any]],
    components: dict[str, dict[str, Any]],
    artifact_ids: set[str],
    arm_ids: set[str],
) -> dict[str, dict[str, Any]]:
    witnesses = []
    keys = {
        "id",
        "arm_id",
        "call_path_id",
        "component_id",
        "engine_search_state",
        "call_count",
        "count_semantics",
        "evidence_kind",
        "source_artifact_ids",
    }
    for index, value in enumerate(rows):
        row = _closed(value, keys, f"execution_witnesses[{index}]")
        if row["call_path_id"] not in paths or row["component_id"] not in components:
            raise ClassificationError("execution witness reference invalid")
        if row["arm_id"] is not None and row["arm_id"] not in arm_ids:
            raise ClassificationError("execution witness arm reference invalid")
        if row["evidence_kind"] not in EVIDENCE_KINDS:
            raise ClassificationError("unknown execution evidence kind")
        sources = _list(row["source_artifact_ids"], "witness source_artifact_ids")
        if not sources or not set(sources) <= artifact_ids:
            raise ClassificationError("execution witness source reference invalid")
        state = row["engine_search_state"]
        count = row["call_count"]
        semantics = row["count_semantics"]
        operation = paths[row["call_path_id"]]["operation"]
        if state == "executed":
            if (
                operation != "Engine.search"
                or not isinstance(count, int)
                or isinstance(count, bool)
                or count <= 0
                or semantics not in {"exact", "lower_bound"}
                or row["evidence_kind"] not in {"instrumented_call", "source_result"}
            ):
                raise ClassificationError("executed witness contract invalid")
        elif state == "not_executed":
            if (
                count != 0
                or semantics != "exact"
                or row["evidence_kind"]
                not in {"coverage_trace", "static_path_audit"}
            ):
                raise ClassificationError("not_executed witness contract invalid")
        elif state == "unknown_historical":
            if count is not None or semantics != "unknown":
                raise ClassificationError("unknown_historical witness contract invalid")
        else:
            raise ClassificationError("unknown engine_search_state")
        witnesses.append(row)
    return _unique(witnesses, "execution witness")


def _validate_metrics(
    rows: list[Any],
    exclusions: list[Any],
    artifacts: dict[str, dict[str, Any]],
    decoded: dict[str, Any],
    components: dict[str, dict[str, Any]],
    witnesses: dict[str, dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    metrics = []
    keys = {
        "id",
        "source_artifact_id",
        "json_pointer",
        "value_type",
        "allowed_values",
        "layer",
        "contributing_component_ids",
        "execution_witness_ids",
    }
    classified: set[tuple[str, str]] = set()
    for index, value in enumerate(rows):
        row = _closed(value, keys, f"metrics[{index}]")
        artifact_id = row["source_artifact_id"]
        if artifact_id not in decoded:
            raise ClassificationError("metric source artifact is not a metrics payload")
        pointer = row["json_pointer"]
        leaf = _json_pointer(decoded[artifact_id], pointer)
        value_type = row["value_type"]
        allowed = _list(row["allowed_values"], "metric allowed_values")
        if value_type == "number":
            if not isinstance(leaf, (int, float)) or isinstance(leaf, bool) or allowed:
                raise ClassificationError("number metric value contract invalid")
        elif value_type == "boolean":
            if not isinstance(leaf, bool) or allowed:
                raise ClassificationError("boolean metric value contract invalid")
        elif value_type == "enum":
            if not isinstance(leaf, str) or not allowed or leaf not in allowed:
                raise ClassificationError("enum metric value contract invalid")
        else:
            raise ClassificationError("unknown metric value_type")
        component_ids = _list(
            row["contributing_component_ids"], "metric contributing_component_ids"
        )
        if not set(component_ids) <= set(components):
            raise ClassificationError("metric component reference invalid")
        effective = _maximum_layer(component_ids, components)
        if row["layer"] != effective:
            raise ClassificationError("metric layer does not match contributors")
        witness_ids = _list(row["execution_witness_ids"], "metric witness ids")
        if not set(witness_ids) <= set(witnesses):
            raise ClassificationError("metric witness reference invalid")
        key = (artifact_id, pointer)
        if key in classified:
            raise ClassificationError("metric leaf classified more than once")
        classified.add(key)
        metrics.append(row)

    excluded: set[tuple[str, str]] = set()
    for index, value in enumerate(exclusions):
        row = _closed(
            value,
            {"source_artifact_id", "json_pointer", "reason"},
            f"metric_exclusions[{index}]",
        )
        if row["reason"] not in EXCLUSION_REASONS:
            raise ClassificationError("unknown metric exclusion reason")
        artifact_id = row["source_artifact_id"]
        if artifact_id not in decoded:
            raise ClassificationError("metric exclusion source invalid")
        _json_pointer(decoded[artifact_id], row["json_pointer"])
        key = (artifact_id, row["json_pointer"])
        if key in excluded or key in classified:
            raise ClassificationError("metric leaf classification overlaps")
        excluded.add(key)

    expected: set[tuple[str, str]] = set()
    for artifact_id, payload in decoded.items():
        for root in artifacts[artifact_id]["measurement_root_json_pointers"]:
            subtree = _json_pointer(payload, root)
            for pointer in _leaf_pointers(subtree, root):
                expected.add((artifact_id, pointer))
    if classified | excluded != expected:
        raise ClassificationError(
            "measurement plan roots are not exhaustively classified"
        )
    return _unique(metrics, "metric")


def _validate_claims(
    rows: list[Any], metrics: dict[str, dict[str, Any]]
) -> dict[str, dict[str, Any]]:
    claims = []
    for index, value in enumerate(rows):
        row = _closed(value, {"id", "layer", "metric_ids"}, f"claims[{index}]")
        metric_ids = _list(row["metric_ids"], "claim metric_ids")
        if not metric_ids or not set(metric_ids) <= set(metrics):
            raise ClassificationError("claim metric reference invalid")
        effective = max(
            (metrics[item]["layer"] for item in metric_ids), key=LAYER_RANK.__getitem__
        )
        if row["layer"] != effective:
            raise ClassificationError("claim layer does not match metrics")
        claims.append(row)
    return _unique(claims, "claim")


def _validate_blocked_reason(reason: Any) -> None:
    row = _closed(reason, {"code", "stage", "message", "detail"}, "blocked_reason")
    if row["code"] not in BLOCKED_CODES:
        raise ClassificationError("blocked_reason code is unsupported")
    _nonempty_string(row["stage"], "blocked_reason stage")
    _nonempty_string(row["message"], "blocked_reason message")
    if not isinstance(row["detail"], dict):
        raise ClassificationError("blocked_reason detail must be an object")


def validate_classification(
    document: dict[str, Any],
    *,
    repository_root: str | Path,
    authority: dict[str, Any],
) -> dict[str, Any]:
    """Validate a classification against its source-bound measurement plan."""
    row = _closed(document, TOP_KEYS, "classification")
    if row["schema_version"] != SCHEMA_VERSION:
        raise ClassificationError("unsupported classification version")
    if row["classifier_version"] != CLASSIFIER_VERSION:
        raise ClassificationError("unsupported classifier version")
    if row["classification_id"] != classification_id(row):
        raise ClassificationError("classification_id does not cover full body")
    _nonempty_string(row["run_id"], "run_id")
    if row["outcome"] not in {"complete", "blocked"}:
        raise ClassificationError("unknown classification outcome")

    _validate_authority(authority)
    if row["measurement_plan_id"] != authority["plan_id"]:
        raise ClassificationError("measurement plan identity mismatch")

    artifacts, decoded = _validate_artifacts(
        _list(row["source_artifacts"], "source_artifacts"), Path(repository_root)
    )
    components = _validate_components(_list(row["components"], "components"))
    if row["components"] != authority["components"]:
        raise ClassificationError("measurement plan components differ")
    paths = _validate_call_paths(
        _list(row["call_paths"], "call_paths"), set(artifacts)
    )
    if row["call_paths"] != authority["call_paths"]:
        raise ClassificationError("measurement plan call paths differ")
    _, arm_ids = _validate_comparisons(
        _list(row["comparisons"], "comparisons"), set(components)
    )
    if row["comparisons"] != authority["comparisons"]:
        raise ClassificationError("measurement plan comparisons differ")

    migration = _closed(
        row["migration"],
        {
            "kind",
            "manifest_path",
            "manifest_entry_sha256",
            "measurement_plan_id",
            "measurement_plan_sha256",
        },
        "migration",
    )
    if migration["kind"] not in {"historical", "native"}:
        raise ClassificationError("migration kind unsupported")
    if migration["measurement_plan_id"] != authority["plan_id"]:
        raise ClassificationError("measurement plan migration identity mismatch")
    if migration["measurement_plan_sha256"] != canonical_sha256(authority):
        raise ClassificationError("measurement plan SHA-256 mismatch")

    if row["outcome"] == "blocked":
        _validate_blocked_reason(row["blocked_reason"])
        if row["execution_witnesses"] or row["metrics"] or row["claims"]:
            raise ClassificationError("blocked classification cannot contain evidence claims")
        return row

    if row["blocked_reason"] is not None:
        raise ClassificationError("complete classification cannot have blocked_reason")
    witnesses = _validate_witnesses(
        _list(row["execution_witnesses"], "execution_witnesses"),
        paths,
        components,
        set(artifacts),
        arm_ids,
    )
    for comparison in row["comparisons"]:
        for arm in comparison["arms"]:
            if not set(arm["execution_witness_ids"]) <= set(witnesses):
                raise ClassificationError("comparison witness reference invalid")
    metrics = _validate_metrics(
        _list(row["metrics"], "metrics"),
        _list(row["metric_exclusions"], "metric_exclusions"),
        artifacts,
        decoded,
        components,
        witnesses,
    )
    if row["metrics"] != authority["metric_bindings"]:
        raise ClassificationError(
            "measurement plan metric bindings differ in layer or ownership"
        )
    _validate_claims(_list(row["claims"], "claims"), metrics)
    if row["claims"] != authority["claims"]:
        raise ClassificationError("measurement plan claims differ")
    return row


def is_successful_evidence(document: dict[str, Any]) -> bool:
    """Return whether a valid classification may support a successful claim."""
    return document.get("outcome") == "complete"


def write_classification(
    run_dir: str | Path,
    document: dict[str, Any],
    *,
    repository_root: str | Path,
    authority: dict[str, Any],
) -> Path:
    """Validate and atomically publish one immutable classification sidecar."""
    destination_dir = Path(run_dir)
    destination_dir.mkdir(parents=True, exist_ok=True)
    destination = destination_dir / SIDECAR_NAME
    payload = (canonical_json(document) + "\n").encode("utf-8")
    if destination.is_file():
        if destination.read_bytes() == payload:
            validate_classification(
                document, repository_root=repository_root, authority=authority
            )
            return destination
        raise ClassificationError("measurement_classification_conflict")
    validate_classification(document, repository_root=repository_root, authority=authority)
    temporary = destination_dir / f".{SIDECAR_NAME}.{uuid.uuid4().hex}.tmp"
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, destination)
        except FileExistsError:
            if destination.read_bytes() != payload:
                raise ClassificationError("measurement_classification_conflict")
        directory_fd = os.open(destination_dir, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    finally:
        temporary.unlink(missing_ok=True)
    return destination


def validate_index_prefix(index_path: str | Path, policy: dict[str, Any]) -> None:
    """Prove that a frozen append-only index prefix is byte-identical."""
    path = Path(index_path)
    data = path.read_bytes()
    length = policy["prefix_bytes"]
    prefix = data[:length]
    if (
        len(prefix) != length
        or _sha256_bytes(prefix) != policy["prefix_sha256"]
        or len(prefix.splitlines()) != policy["prefix_lines"]
    ):
        raise ClassificationError("historical_prefix_changed")


def validate_post_cutover_presence(
    *, experiments_dir: str | Path, index_path: str | Path, prefix_lines: int
) -> None:
    """Require one normal sidecar for every index row after the frozen prefix."""
    lines = Path(index_path).read_text(encoding="utf-8").splitlines()
    for line in lines[prefix_lines:]:
        try:
            row = json.loads(line)
            run_id = row["run_id"]
        except (json.JSONDecodeError, KeyError, TypeError) as exc:
            raise ClassificationError("post-cutover index row is invalid") from exc
        sidecar = Path(experiments_dir) / "runs" / run_id / SIDECAR_NAME
        if not sidecar.is_file():
            raise ClassificationError(f"classification_missing: {run_id}")


def _load_json(path: Path, label: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ClassificationError(f"{label} is unavailable or invalid: {path}") from exc
    if not isinstance(value, dict):
        raise ClassificationError(f"{label} must be an object")
    return value


def _repository_path(repository_root: Path, relative: str, label: str) -> Path:
    root = repository_root.resolve()
    path = (root / relative).resolve()
    if path != root and root not in path.parents:
        raise ClassificationError(f"{label} escapes repository")
    return path


def _load_measurement_plan(
    repository_root: Path, reference: Any
) -> dict[str, Any]:
    ref = _closed(reference, {"path", "sha256", "plan_id"}, "measurement_plan")
    path = _repository_path(repository_root, ref["path"], "measurement plan path")
    plan = _load_json(path, "measurement plan")
    _validate_authority(plan)
    if canonical_sha256(plan) != ref["sha256"] or plan["plan_id"] != ref["plan_id"]:
        raise ClassificationError("measurement plan reference mismatch")
    return plan


def _validate_historical_manifest(
    repository_root: Path,
    experiments_dir: Path,
    manifest: dict[str, Any],
    prefix_rows: list[dict[str, Any]],
) -> None:
    _closed(manifest, {"schema_version", "included", "excluded"}, "historical manifest")
    if manifest["schema_version"] != HISTORICAL_VERSION:
        raise ClassificationError("unsupported historical manifest version")
    included = _list(manifest["included"], "historical included")
    excluded = _list(manifest["excluded"], "historical excluded")
    included_ids: set[str] = set()
    for index, value in enumerate(included):
        entry = _closed(
            value,
            {
                "run_id",
                "record_sha256",
                "metrics_sha256",
                "authority",
                "audit_receipt_sha256",
            },
            f"historical included[{index}]",
        )
        run_id = _nonempty_string(entry["run_id"], "historical run_id")
        if run_id in included_ids:
            raise ClassificationError("duplicate historical included run_id")
        included_ids.add(run_id)
        run_dir = experiments_dir / "runs" / run_id
        record_path = run_dir / "record.json"
        metrics_path = run_dir / "metrics.json"
        if _sha256_bytes(record_path.read_bytes()) != entry["record_sha256"]:
            raise ClassificationError("historical record SHA-256 mismatch")
        if _sha256_bytes(metrics_path.read_bytes()) != entry["metrics_sha256"]:
            raise ClassificationError("historical metrics SHA-256 mismatch")
        audit_path = repository_root / HISTORICAL_AUDIT_PATH
        if _sha256_bytes(audit_path.read_bytes()) != entry["audit_receipt_sha256"]:
            raise ClassificationError("historical audit receipt SHA-256 mismatch")
        authority = entry["authority"]
        _validate_authority(authority)
        sidecar = _load_json(run_dir / SIDECAR_NAME, "historical classification")
        if sidecar["migration"]["manifest_entry_sha256"] != canonical_sha256(entry):
            raise ClassificationError("historical manifest entry SHA-256 mismatch")
        validate_classification(
            sidecar, repository_root=repository_root, authority=authority
        )

    excluded_ids: set[str] = set()
    for index, value in enumerate(excluded):
        entry = _closed(value, {"run_id", "reason"}, f"historical excluded[{index}]")
        if entry["reason"] != "not_decision_bearing_complete_comparison":
            raise ClassificationError("historical exclusion reason unsupported")
        run_id = _nonempty_string(entry["run_id"], "historical excluded run_id")
        if run_id in excluded_ids:
            raise ClassificationError("duplicate historical excluded run_id")
        excluded_ids.add(run_id)
    if included_ids & excluded_ids:
        raise ClassificationError("historical included/excluded overlap")
    indexed_global = {
        row["run_id"]
        for row in prefix_rows
        if isinstance(row.get("run_id"), str)
        and row["run_id"].startswith("global-01-")
    }
    if included_ids | excluded_ids != indexed_global:
        raise ClassificationError("historical GLOBAL-01 inventory is not closed")


def validate_repository(repository_root: str | Path) -> None:
    """Run the portable clean-clone measurement-classification gate."""
    root = Path(repository_root).resolve()
    experiments_dir = root / "experiments"
    policy = _load_json(experiments_dir / POLICY_NAME, "classification policy")
    _closed(
        policy,
        {"schema_version", "classifier_version", "index", "historical_manifest_path"},
        "classification policy",
    )
    if (
        policy["schema_version"] != POLICY_VERSION
        or policy["classifier_version"] != CLASSIFIER_VERSION
    ):
        raise ClassificationError("unsupported classification policy version")
    index_policy = _closed(
        policy["index"],
        {"path", "prefix_bytes", "prefix_lines", "prefix_sha256"},
        "classification index policy",
    )
    index_path = _repository_path(root, index_policy["path"], "index path")
    validate_index_prefix(index_path, index_policy)
    lines = index_path.read_text(encoding="utf-8").splitlines()
    try:
        prefix_rows = [json.loads(line) for line in lines[: index_policy["prefix_lines"]]]
    except json.JSONDecodeError as exc:
        raise ClassificationError("historical index prefix contains invalid JSON") from exc
    manifest_path = _repository_path(
        root, policy["historical_manifest_path"], "historical manifest path"
    )
    manifest = _load_json(manifest_path, "historical manifest")
    _validate_historical_manifest(root, experiments_dir, manifest, prefix_rows)

    validate_post_cutover_presence(
        experiments_dir=experiments_dir,
        index_path=index_path,
        prefix_lines=index_policy["prefix_lines"],
    )
    for line in lines[index_policy["prefix_lines"] :]:
        index_row = json.loads(line)
        run_id = index_row["run_id"]
        run_dir = experiments_dir / "runs" / run_id
        record = _load_json(run_dir / "record.json", "post-cutover record")
        if record.get("run_id") != run_id:
            raise ClassificationError("post-cutover record run_id mismatch")
        try:
            plan_ref = record["config"]["resolved"]["measurement_plan"]
        except (KeyError, TypeError) as exc:
            raise ClassificationError("post-cutover measurement plan is missing") from exc
        authority = _load_measurement_plan(root, plan_ref)
        sidecar = _load_json(run_dir / SIDECAR_NAME, "post-cutover classification")
        if sidecar.get("run_id") != run_id:
            raise ClassificationError("post-cutover classification run_id mismatch")
        validate_classification(sidecar, repository_root=root, authority=authority)


def _validate_tree_command(args: argparse.Namespace) -> int:
    validate_repository(args.repository_root)
    print("PASS measurement-classification: portable tree is valid")
    return 0


def _git_blob(repository_root: Path, locator: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", locator],
        cwd=repository_root,
        check=False,
        capture_output=True,
    )
    if completed.returncode:
        raise ClassificationError(f"source artifact unavailable: {locator}")
    return completed.stdout


def build_historical_audit(
    repository_root: str | Path, external_root: str | Path
) -> dict[str, Any]:
    """Derive the compact GLOBAL-01 audit from preserved raw artifacts."""
    root = Path(repository_root).resolve()
    external = Path(external_root)
    if not external.is_absolute():
        external = (root / external).resolve()
    else:
        external = external.resolve()

    runs: list[dict[str, Any]] = []
    records: dict[str, dict[str, Any]] = {}
    for run_id in (INITIAL_RUN_ID, HELDOUT_RUN_ID):
        record_path = root / "experiments" / "runs" / run_id / "record.json"
        record = _load_json(record_path, "historical record")
        records[run_id] = record
        artifacts = []
        for artifact in record["artifacts"]:
            artifact_path = (root / artifact["path"]).resolve()
            if artifact_path != external and external not in artifact_path.parents:
                raise ClassificationError("historical artifact escapes external root")
            try:
                digest = _sha256_bytes(artifact_path.read_bytes())
            except OSError as exc:
                raise ClassificationError(
                    f"historical artifact unavailable: {artifact['path']}"
                ) from exc
            if digest != artifact["sha256"]:
                raise ClassificationError("historical external artifact SHA-256 mismatch")
            artifacts.append({"path": artifact["path"], "sha256": digest})
        runs.append(
            {
                "run_id": run_id,
                "record_sha256": _sha256_bytes(record_path.read_bytes()),
                "metrics_sha256": _sha256_bytes(
                    (record_path.parent / "metrics.json").read_bytes()
                ),
                "external_artifacts": artifacts,
                "engine_search": {"state": "unknown_historical"},
            }
        )

    heldout_record = records[HELDOUT_RUN_ID]
    checkpoint_path = (root / heldout_record["artifacts"][1]["path"]).resolve()
    checkpoint = _load_json(checkpoint_path, "held-out checkpoint")
    cells = checkpoint.get("cells")
    if not isinstance(cells, dict):
        raise ClassificationError("held-out checkpoint cells are invalid")
    control_cells = [key for key in cells if key.startswith("retrieval/control/")]
    treatment_cells = [key for key in cells if key.startswith("retrieval/treatment/")]
    decomposition_cells = [
        key for key in cells if key.startswith("decomposition/treatment/")
    ]
    if not (
        len(control_cells) == len(treatment_cells) == len(decomposition_cells) == 42
    ):
        raise ClassificationError("held-out retrieval cell count drifted")
    for key in decomposition_cells:
        try:
            subqueries = cells[key]["value"]["subqueries"]
        except (KeyError, TypeError) as exc:
            raise ClassificationError("held-out decomposition cell is invalid") from exc
        if not isinstance(subqueries, list) or len(subqueries) != 4:
            raise ClassificationError("held-out decomposition width drifted")

    lazy_locator = f"{HELDOUT_COMMIT}:experiments/global_01_lazy.py"
    live_locator = f"{HELDOUT_COMMIT}:experiments/global_01_lazy_live.py"
    lazy_source = _git_blob(root, lazy_locator)
    live_source = _git_blob(root, live_locator)
    if b"for query_ordinal, query in enumerate(queries)" not in lazy_source or b"engine.search(" not in lazy_source:
        raise ClassificationError("historical retrieval call derivation drifted")
    if b'[question["text"], *(subqueries or [])]' not in live_source:
        raise ClassificationError("historical treatment query derivation drifted")
    if lazy_source.count(b"engine.search(") < 6:
        raise ClassificationError("historical lifecycle call derivation drifted")

    runs[1]["engine_search"] = {
        "state": "executed",
        "control_retrieval_cells": 42,
        "control_queries_per_cell": 1,
        "control_call_lower_bound": 42,
        "treatment_retrieval_cells": 42,
        "treatment_queries_per_cell": 5,
        "treatment_call_lower_bound": 210,
        "lifecycle_call_lower_bound": 5,
        "implementation_blobs": [
            {"locator": lazy_locator, "sha256": _sha256_bytes(lazy_source)},
            {"locator": live_locator, "sha256": _sha256_bytes(live_source)},
        ],
    }
    return {
        "schema_version": "measurement.classification-historical-audit.v1",
        "runs": runs,
    }


def _write_generated_json(path: Path, value: Any) -> Path:
    payload = (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        if path.read_bytes() != payload:
            raise ClassificationError(f"generated artifact conflict: {path}")
        return path
    temporary = path.parent / f".{path.name}.{uuid.uuid4().hex}.tmp"
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError:
            if path.read_bytes() != payload:
                raise ClassificationError(f"generated artifact conflict: {path}")
    finally:
        temporary.unlink(missing_ok=True)
    return path


def _metric_bindings(
    payload: dict[str, Any],
    roots: list[str],
    *,
    prefix: str,
    ownership: Any,
) -> list[dict[str, Any]]:
    bindings = []
    for root in roots:
        for pointer in _leaf_pointers(_json_pointer(payload, root), root):
            value = _json_pointer(payload, pointer)
            component_ids, witness_ids = ownership(pointer)
            layer = max(
                (COMPONENT_LAYERS[item["kind"]] for item in component_ids),
                key=LAYER_RANK.__getitem__,
            )
            if isinstance(value, bool):
                value_type, allowed = "boolean", []
            elif isinstance(value, (int, float)):
                value_type, allowed = "number", []
            elif isinstance(value, str) and value in {"pass", "fail"}:
                value_type, allowed = "enum", ["pass", "fail"]
            else:
                raise ClassificationError(f"historical metric is not scalar: {pointer}")
            bindings.append(
                {
                    "id": f"{prefix}-{hashlib.sha256(pointer.encode()).hexdigest()[:12]}",
                    "source_artifact_id": "metrics",
                    "json_pointer": pointer,
                    "value_type": value_type,
                    "allowed_values": allowed,
                    "layer": layer,
                    "contributing_component_ids": [item["id"] for item in component_ids],
                    "execution_witness_ids": witness_ids,
                }
            )
    return bindings


def _historical_authorities(repository_root: Path) -> dict[str, dict[str, Any]]:
    initial_metrics = _load_json(
        repository_root / "experiments" / "runs" / INITIAL_RUN_ID / "metrics.json",
        "initial GLOBAL-01 metrics",
    )
    heldout_metrics = _load_json(
        repository_root / "experiments" / "runs" / HELDOUT_RUN_ID / "metrics.json",
        "held-out GLOBAL-01 metrics",
    )

    initial_components = [
        {"id": "corpus", "name": "AP News corpus", "kind": "corpus_gold"},
        {"id": "storage", "name": "FathomDB storage", "kind": "fathomdb_storage"},
        {"id": "engine", "name": "FathomDB Engine.search", "kind": "fathomdb_engine_search"},
        {"id": "external", "name": "GraphRAG retrieval", "kind": "external_retrieval"},
        {"id": "answer", "name": "answer generator", "kind": "answer_generator"},
        {"id": "judge", "name": "semantic judge", "kind": "semantic_judge"},
        {"id": "metric", "name": "deterministic aggregation", "kind": "deterministic_metric"},
    ]
    initial_arms = [
        {
            "id": "fathomdb",
            "component_ids": ["corpus", "storage", "engine", "answer", "judge", "metric"],
            "execution_witness_ids": ["initial-fathom-search"],
        },
        {
            "id": "graphrag",
            "component_ids": ["corpus", "external", "answer", "judge", "metric"],
            "execution_witness_ids": ["initial-graph-search"],
        },
    ]
    initial_comparison = {
        "id": "global-first-run",
        "arms": initial_arms,
        "shared_component_ids": ["corpus", "answer", "judge", "metric"],
        "differing_component_ids": ["storage", "engine", "external"],
    }

    def initial_owner(_pointer: str) -> tuple[list[dict[str, Any]], list[str]]:
        return initial_components, ["initial-fathom-search", "initial-graph-search"]

    initial_bindings = _metric_bindings(
        initial_metrics,
        ["/metrics"],
        prefix="initial",
        ownership=initial_owner,
    )
    initial_authority = {
        "schema_version": PLAN_VERSION,
        "plan_id": f"historical-{INITIAL_RUN_ID}",
        "components": initial_components,
        "call_paths": [
            {
                "id": "initial-fathom-path",
                "operation": "Engine.search",
                "source_artifact_ids": ["audit"],
            },
            {
                "id": "initial-graph-path",
                "operation": "external_retrieval",
                "source_artifact_ids": ["audit"],
            },
        ],
        "comparisons": [initial_comparison],
        "metric_bindings": initial_bindings,
        "claims": [
            {
                "id": "initial-global-end-to-end",
                "layer": "end_to_end",
                "metric_ids": [item["id"] for item in initial_bindings],
            }
        ],
    }

    heldout_components = [
        {"id": "corpus", "name": "AP News corpus", "kind": "corpus_gold"},
        {"id": "storage", "name": "FathomDB storage", "kind": "fathomdb_storage"},
        {"id": "engine", "name": "FathomDB Engine.search", "kind": "fathomdb_engine_search"},
        {"id": "planner", "name": "query planner", "kind": "query_planner"},
        {"id": "candidate", "name": "candidate planner", "kind": "candidate_planner"},
        {"id": "context", "name": "context packer", "kind": "context_packer"},
        {"id": "answer", "name": "answer generator", "kind": "answer_generator"},
        {"id": "judge", "name": "semantic judge", "kind": "semantic_judge"},
        {"id": "metric", "name": "deterministic aggregation", "kind": "deterministic_metric"},
    ]
    by_id = {item["id"]: item for item in heldout_components}
    control_ids = ["corpus", "storage", "engine", "context", "answer", "judge", "metric"]
    treatment_ids = [
        "corpus",
        "storage",
        "engine",
        "planner",
        "candidate",
        "context",
        "answer",
        "judge",
        "metric",
    ]
    heldout_comparison = {
        "id": "global-heldout",
        "arms": [
            {
                "id": "control",
                "component_ids": control_ids,
                "execution_witness_ids": ["heldout-control-search"],
            },
            {
                "id": "treatment",
                "component_ids": treatment_ids,
                "execution_witness_ids": ["heldout-treatment-search"],
            },
        ],
        "shared_component_ids": control_ids,
        "differing_component_ids": ["planner", "candidate"],
    }

    def heldout_owner(pointer: str) -> tuple[list[dict[str, Any]], list[str]]:
        if pointer.startswith("/lifecycle/"):
            ids = ["storage", "engine", "metric"]
            witnesses = ["heldout-lifecycle-search"]
        elif pointer.startswith("/operations/control/"):
            ids = control_ids
            witnesses = ["heldout-control-search"]
        elif pointer.startswith("/operations/treatment/"):
            ids = treatment_ids
            witnesses = ["heldout-treatment-search"]
        else:
            ids = treatment_ids
            witnesses = ["heldout-control-search", "heldout-treatment-search"]
        return [by_id[item] for item in ids], witnesses

    heldout_roots = [
        "/canonical_source_link_completeness",
        "/lifecycle",
        "/operations",
        "/acceptance/boundaries",
        "/pairwise",
        "/scoring",
    ]
    heldout_bindings = _metric_bindings(
        heldout_metrics,
        heldout_roots,
        prefix="heldout",
        ownership=heldout_owner,
    )
    lifecycle_ids = [
        item["id"]
        for item in heldout_bindings
        if item["json_pointer"].startswith("/lifecycle/")
    ]
    end_ids = [item["id"] for item in heldout_bindings if item["id"] not in lifecycle_ids]
    heldout_authority = {
        "schema_version": PLAN_VERSION,
        "plan_id": f"historical-{HELDOUT_RUN_ID}",
        "components": heldout_components,
        "call_paths": [
            {
                "id": "heldout-control-path",
                "operation": "Engine.search",
                "source_artifact_ids": ["audit", "lazy-implementation", "live-implementation"],
            },
            {
                "id": "heldout-treatment-path",
                "operation": "Engine.search",
                "source_artifact_ids": ["audit", "lazy-implementation", "live-implementation"],
            },
            {
                "id": "heldout-lifecycle-path",
                "operation": "Engine.search",
                "source_artifact_ids": ["audit", "lazy-implementation"],
            },
        ],
        "comparisons": [heldout_comparison],
        "metric_bindings": heldout_bindings,
        "claims": [
            {"id": "heldout-lifecycle", "layer": "data_plane", "metric_ids": lifecycle_ids},
            {"id": "heldout-end-to-end", "layer": "end_to_end", "metric_ids": end_ids},
        ],
    }
    return {INITIAL_RUN_ID: initial_authority, HELDOUT_RUN_ID: heldout_authority}


def _repository_artifact(
    repository_root: Path,
    *,
    identity: str,
    path: str,
    role: str,
    roots: list[str] | None = None,
) -> dict[str, Any]:
    payload = (repository_root / path).read_bytes()
    return {
        "id": identity,
        "locator_kind": "repository_path",
        "locator": path,
        "role": role,
        "sha256": _sha256_bytes(payload),
        "measurement_root_json_pointers": roots or [],
    }


def _git_artifact(
    repository_root: Path, *, identity: str, locator: str
) -> dict[str, Any]:
    return {
        "id": identity,
        "locator_kind": "git_blob",
        "locator": locator,
        "role": "implementation",
        "sha256": _sha256_bytes(_git_blob(repository_root, locator)),
        "measurement_root_json_pointers": [],
    }


def _historical_classification(
    repository_root: Path,
    *,
    run_id: str,
    authority: dict[str, Any],
    manifest_entry: dict[str, Any],
) -> dict[str, Any]:
    run_base = f"experiments/runs/{run_id}"
    artifacts = [
        _repository_artifact(
            repository_root,
            identity="metrics",
            path=f"{run_base}/metrics.json",
            role="metrics_payload",
            roots=(
                ["/metrics"]
                if run_id == INITIAL_RUN_ID
                else [
                    "/canonical_source_link_completeness",
                    "/lifecycle",
                    "/operations",
                    "/acceptance/boundaries",
                    "/pairwise",
                    "/scoring",
                ]
            ),
        ),
        _repository_artifact(
            repository_root,
            identity="record",
            path=f"{run_base}/record.json",
            role="record",
        ),
        _repository_artifact(
            repository_root,
            identity="audit",
            path=str(HISTORICAL_AUDIT_PATH),
            role="derivation_spec",
        ),
    ]
    if run_id == HELDOUT_RUN_ID:
        artifacts.extend(
            [
                _git_artifact(
                    repository_root,
                    identity="lazy-implementation",
                    locator=f"{HELDOUT_COMMIT}:experiments/global_01_lazy.py",
                ),
                _git_artifact(
                    repository_root,
                    identity="live-implementation",
                    locator=f"{HELDOUT_COMMIT}:experiments/global_01_lazy_live.py",
                ),
            ]
        )
        witnesses = [
            {
                "id": "heldout-control-search",
                "arm_id": "control",
                "call_path_id": "heldout-control-path",
                "component_id": "engine",
                "engine_search_state": "executed",
                "call_count": 42,
                "count_semantics": "lower_bound",
                "evidence_kind": "source_result",
                "source_artifact_ids": [
                    "audit",
                    "lazy-implementation",
                    "live-implementation",
                ],
            },
            {
                "id": "heldout-treatment-search",
                "arm_id": "treatment",
                "call_path_id": "heldout-treatment-path",
                "component_id": "engine",
                "engine_search_state": "executed",
                "call_count": 210,
                "count_semantics": "lower_bound",
                "evidence_kind": "source_result",
                "source_artifact_ids": [
                    "audit",
                    "lazy-implementation",
                    "live-implementation",
                ],
            },
            {
                "id": "heldout-lifecycle-search",
                "arm_id": None,
                "call_path_id": "heldout-lifecycle-path",
                "component_id": "engine",
                "engine_search_state": "executed",
                "call_count": 5,
                "count_semantics": "lower_bound",
                "evidence_kind": "source_result",
                "source_artifact_ids": ["audit", "lazy-implementation"],
            },
        ]
    else:
        witnesses = [
            {
                "id": "initial-fathom-search",
                "arm_id": "fathomdb",
                "call_path_id": "initial-fathom-path",
                "component_id": "engine",
                "engine_search_state": "unknown_historical",
                "call_count": None,
                "count_semantics": "unknown",
                "evidence_kind": "immutable_receipt",
                "source_artifact_ids": ["audit"],
            },
            {
                "id": "initial-graph-search",
                "arm_id": "graphrag",
                "call_path_id": "initial-graph-path",
                "component_id": "external",
                "engine_search_state": "unknown_historical",
                "call_count": None,
                "count_semantics": "unknown",
                "evidence_kind": "immutable_receipt",
                "source_artifact_ids": ["audit"],
            },
        ]
    document = {
        "schema_version": SCHEMA_VERSION,
        "classifier_version": CLASSIFIER_VERSION,
        "classification_id": "",
        "run_id": run_id,
        "outcome": "complete",
        "blocked_reason": None,
        "measurement_plan_id": authority["plan_id"],
        "source_artifacts": artifacts,
        "components": authority["components"],
        "call_paths": authority["call_paths"],
        "execution_witnesses": witnesses,
        "metrics": authority["metric_bindings"],
        "metric_exclusions": [],
        "comparisons": authority["comparisons"],
        "claims": authority["claims"],
        "migration": {
            "kind": "historical",
            "manifest_path": f"experiments/{HISTORICAL_MANIFEST_NAME}",
            "manifest_entry_sha256": canonical_sha256(manifest_entry),
            "measurement_plan_id": authority["plan_id"],
            "measurement_plan_sha256": canonical_sha256(authority),
        },
    }
    document["classification_id"] = classification_id(document)
    return document


def materialize_historical(
    repository_root: str | Path, external_root: str | Path
) -> None:
    """Create the portable audit, closed manifest, sidecars, and cutover policy."""
    root = Path(repository_root).resolve()
    experiments_dir = root / "experiments"
    audit = build_historical_audit(root, external_root)
    audit_path = _write_generated_json(root / HISTORICAL_AUDIT_PATH, audit)
    audit_sha = _sha256_bytes(audit_path.read_bytes())
    authorities = _historical_authorities(root)
    included = []
    for run_id in (INITIAL_RUN_ID, HELDOUT_RUN_ID):
        run_dir = experiments_dir / "runs" / run_id
        included.append(
            {
                "run_id": run_id,
                "record_sha256": _sha256_bytes((run_dir / "record.json").read_bytes()),
                "metrics_sha256": _sha256_bytes((run_dir / "metrics.json").read_bytes()),
                "authority": authorities[run_id],
                "audit_receipt_sha256": audit_sha,
            }
        )
    index_path = experiments_dir / "index.jsonl"
    index_bytes = index_path.read_bytes()
    index_rows = [json.loads(line) for line in index_bytes.splitlines()]
    included_ids = {INITIAL_RUN_ID, HELDOUT_RUN_ID}
    excluded = [
        {
            "run_id": row["run_id"],
            "reason": "not_decision_bearing_complete_comparison",
        }
        for row in index_rows
        if row["run_id"].startswith("global-01-") and row["run_id"] not in included_ids
    ]
    manifest = {
        "schema_version": HISTORICAL_VERSION,
        "included": included,
        "excluded": excluded,
    }
    _write_generated_json(experiments_dir / HISTORICAL_MANIFEST_NAME, manifest)
    for entry in included:
        document = _historical_classification(
            root,
            run_id=entry["run_id"],
            authority=entry["authority"],
            manifest_entry=entry,
        )
        write_classification(
            experiments_dir / "runs" / entry["run_id"],
            document,
            repository_root=root,
            authority=entry["authority"],
        )
    policy = {
        "schema_version": POLICY_VERSION,
        "classifier_version": CLASSIFIER_VERSION,
        "index": {
            "path": "experiments/index.jsonl",
            "prefix_bytes": len(index_bytes),
            "prefix_lines": len(index_rows),
            "prefix_sha256": _sha256_bytes(index_bytes),
        },
        "historical_manifest_path": f"experiments/{HISTORICAL_MANIFEST_NAME}",
    }
    _write_generated_json(experiments_dir / POLICY_NAME, policy)


def _audit_historical_command(args: argparse.Namespace) -> int:
    root = Path(args.repository_root).resolve()
    expected = _load_json(root / HISTORICAL_AUDIT_PATH, "historical audit receipt")
    observed = build_historical_audit(root, args.external_root)
    if observed != expected:
        raise ClassificationError("historical audit receipt differs from raw evidence")
    print("PASS measurement-classification: historical raw evidence matches audit")
    return 0


def _materialize_historical_command(args: argparse.Namespace) -> int:
    materialize_historical(args.repository_root, args.external_root)
    print("PASS measurement-classification: historical artifacts materialized")
    return 0


NATIVE_CONFIG_KEYS = {
    "schema_version",
    "program_track",
    "seed",
    "device",
    "embedder",
    "cross_encoder",
    "query",
    "limit",
    "expected_source_id",
    "records",
    "database_root",
    "fathomdb_bin",
    "measurement_plan",
}


def _validate_native_config(config: dict[str, Any], repository_root: Path) -> dict[str, Any]:
    _closed(config, NATIVE_CONFIG_KEYS, "native config")
    if (
        config["schema_version"] != "measurement-classification.native-search.v1"
        or config["program_track"] != "GLOBAL-01"
        or config["device"] != "cpu"
        or config["embedder"] != "none"
        or config["cross_encoder"] != "disabled"
    ):
        raise ClassificationError("native config fixed policy drifted")
    if not isinstance(config["seed"], int) or isinstance(config["seed"], bool):
        raise ClassificationError("native config seed is invalid")
    _nonempty_string(config["query"], "native query")
    _nonempty_string(config["expected_source_id"], "native expected_source_id")
    if config["limit"] != 3:
        raise ClassificationError("native config limit drifted")
    records = _list(config["records"], "native records")
    if len(records) != 3:
        raise ClassificationError("native fixture requires exactly three records")
    logical_ids: set[str] = set()
    source_ids: set[str] = set()
    for index, value in enumerate(records):
        row = _closed(
            value,
            {"kind", "logical_id", "source_id", "body"},
            f"native records[{index}]",
        )
        if row["kind"] != "document":
            raise ClassificationError("native fixture record kind drifted")
        logical_ids.add(_nonempty_string(row["logical_id"], "native logical_id"))
        source_ids.add(_nonempty_string(row["source_id"], "native source_id"))
        _nonempty_string(row["body"], "native body")
    if len(logical_ids) != 3 or len(source_ids) != 3:
        raise ClassificationError("native fixture identities must be unique")
    if config["expected_source_id"] not in source_ids:
        raise ClassificationError("native expected source is not in fixture")
    _nonempty_string(config["database_root"], "native database_root")
    binary = _repository_path(repository_root, config["fathomdb_bin"], "fathomdb_bin")
    if not binary.is_file():
        raise ClassificationError("native FathomDB binary is unavailable")
    _load_measurement_plan(repository_root, config["measurement_plan"])
    return config


def _native_classification(
    repository_root: Path,
    *,
    run_id: str,
    authority: dict[str, Any],
    code_sha: str,
    outcome: str,
    blocked_reason: dict[str, Any] | None,
) -> dict[str, Any]:
    run_base = f"experiments/runs/{run_id}"
    artifacts = [
        _repository_artifact(
            repository_root,
            identity="record",
            path=f"{run_base}/record.json",
            role="record",
        ),
        _repository_artifact(
            repository_root,
            identity="configuration",
            path="experiments/configs/measurement-classification/native-search.v1.json",
            role="configuration",
        ),
        _repository_artifact(
            repository_root,
            identity="plan",
            path="experiments/configs/measurement-classification/native-search.measurement-plan.v1.json",
            role="configuration",
        ),
        _git_artifact(
            repository_root,
            identity="implementation",
            locator=f"{code_sha}:experiments/measurement_classification.py",
        ),
    ]
    witnesses: list[dict[str, Any]] = []
    metrics: list[dict[str, Any]] = []
    claims: list[dict[str, Any]] = []
    if outcome == "complete":
        artifacts.insert(
            0,
            _repository_artifact(
                repository_root,
                identity="metrics",
                path=f"{run_base}/metrics.json",
                role="metrics_payload",
                roots=["/retrieval"],
            ),
        )
        witnesses = [
            {
                "id": "native-search-call",
                "arm_id": None,
                "call_path_id": "native-search",
                "component_id": "engine",
                "engine_search_state": "executed",
                "call_count": 1,
                "count_semantics": "exact",
                "evidence_kind": "instrumented_call",
                "source_artifact_ids": ["implementation"],
            }
        ]
        metrics = authority["metric_bindings"]
        claims = authority["claims"]
    document = {
        "schema_version": SCHEMA_VERSION,
        "classifier_version": CLASSIFIER_VERSION,
        "classification_id": "",
        "run_id": run_id,
        "outcome": outcome,
        "blocked_reason": blocked_reason,
        "measurement_plan_id": authority["plan_id"],
        "source_artifacts": artifacts,
        "components": authority["components"],
        "call_paths": authority["call_paths"],
        "execution_witnesses": witnesses,
        "metrics": metrics,
        "metric_exclusions": [],
        "comparisons": authority["comparisons"],
        "claims": claims,
        "migration": {
            "kind": "native",
            "manifest_path": None,
            "manifest_entry_sha256": None,
            "measurement_plan_id": authority["plan_id"],
            "measurement_plan_sha256": canonical_sha256(authority),
        },
    }
    document["classification_id"] = classification_id(document)
    return document


def _native_blocker(exc: Exception) -> dict[str, Any]:
    if isinstance(exc, ClassificationError):
        code, stage = "config_invalid_value", "configuration"
    elif isinstance(exc, FileExistsError):
        code, stage = "run_id_collision", "database_setup"
    elif isinstance(exc, RuntimeError):
        code, stage = "database_setup_failed", "database_setup"
    else:
        code, stage = "engine_search_failed", "search"
    return {"code": code, "stage": stage, "message": str(exc), "detail": {}}


def run_native(repository_root: str | Path, config_path: str | Path) -> str:
    """Run the pinned one-call retrieval-only witness and register its receipt."""
    from experiments import _lib
    from experiments.fathomdb_test_setup import prepare_test_database

    root = Path(repository_root).resolve()
    path = Path(config_path)
    if not path.is_absolute():
        path = (root / path).resolve()
    config = _load_json(path, "native config")
    authority = _load_measurement_plan(root, config.get("measurement_plan"))
    timestamp = datetime.now(UTC).replace(microsecond=0)
    code = _lib.git_info()
    if code["dirty"]:
        raise ClassificationError("native witness requires a clean committed tree")

    prepared = None
    fathomdb_version = "unavailable"
    metrics: dict[str, Any]
    blocker: dict[str, Any] | None = None
    try:
        _validate_native_config(config, root)
        test_id = f"slice-10-native-{timestamp.strftime('%Y%m%d%H%M%S').lower()}"
        prepared = prepare_test_database(
            root / config["database_root"],
            test_id=test_id,
            embed_device="cpu",
            rerank_device="cpu",
            embedder="none",
            warm_cache=False,
            check_reranker=False,
            fathomdb_bin=str(root / config["fathomdb_bin"]),
        )
        from fathomdb import Engine, __version__ as fathomdb_version

        engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
        try:
            engine.write(config["records"])
            engine.drain(timeout_s=60)
            call_count = 0
            call_count += 1
            result = engine.search(config["query"], limit=config["limit"])
        finally:
            engine.close()
        source_ids = [hit.source_id for hit in result.results]
        expected = config["expected_source_id"]
        if call_count != 1:
            raise ClassificationError("native search call count is not one")
        if expected not in source_ids:
            raise ClassificationError("native expected hit is missing")
        rank = source_ids.index(expected) + 1
        metrics = {
            "schema_version": "measurement-classification.native-result.v1",
            "program_track": "GLOBAL-01",
            "state": "complete",
            "expected_source_id": expected,
            "returned_source_ids": source_ids,
            "retrieval": {
                "call_count": call_count,
                "recall_at_3": 1.0,
                "reciprocal_rank": 1.0 / rank,
                "result_count": len(source_ids),
            },
        }
        verdict = "pass"
        read = "One direct native Engine.search call returned the pinned source."
    except Exception as exc:  # the blocked receipt must survive typed refusals
        blocker = _native_blocker(exc)
        metrics = {
            "schema_version": "measurement-classification.native-blocked.v1",
            "program_track": "GLOBAL-01",
            "state": "blocked",
            "blocked_reason": blocker,
        }
        verdict = "blocked"
        read = f"Native retrieval-only witness blocked: {blocker['code']}."

    artifacts = []
    if prepared is not None:
        artifacts = [
            {"path": str(prepared.config_path), "sha256": _sha256_bytes(prepared.config_path.read_bytes())},
            {"path": str(prepared.doctor_path), "sha256": _sha256_bytes(prepared.doctor_path.read_bytes())},
        ]
    run_id, run_dir = _lib.write_record(
        "measurement-classification-native-search",
        ts=timestamp,
        config_obj=config,
        metrics=metrics,
        verdict=verdict,
        read=read,
        code=code,
        corpus={
            "source": "literal Slice 10 retrieval-only fixture",
            "manifest_sha256": canonical_sha256(config["records"]),
            "datasets": [{"name": "slice-10-native", "documents": 3, "questions": 1}],
        },
        seeds={"fixture": config["seed"]},
        env=_lib.env_info(key_deps={"fathomdb": fathomdb_version}),
        cost_usd=0.0,
        headline={"program_track": "GLOBAL-01", "status": verdict},
        n=1,
        config_path=str(path.relative_to(root)),
        tests=["tests/experiments/test_measurement_classification.py"],
        artifacts=artifacts,
        base_dir=root / "experiments",
    )
    document = _native_classification(
        root,
        run_id=run_id,
        authority=authority,
        code_sha=code["git_sha"],
        outcome="complete" if blocker is None else "blocked",
        blocked_reason=blocker,
    )
    write_classification(run_dir, document, repository_root=root, authority=authority)
    _lib.regen_index_md(
        index_path=root / "experiments" / "index.jsonl",
        md_path=root / "experiments" / "INDEX.md",
    )
    if blocker is not None:
        raise ClassificationError(read)
    return run_id


def _run_native_command(args: argparse.Namespace) -> int:
    run_id = run_native(args.repository_root, args.config)
    print(json.dumps({"status": "pass", "run_id": run_id}, sort_keys=True))
    return 0


def _not_implemented(_args: argparse.Namespace) -> int:
    raise ClassificationError("Slice 10 command not implemented")


def main(argv: list[str] | None = None) -> int:
    """Run the portable validator, historical audit, or native fixture."""
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    for name in (
        "validate-tree",
        "audit-historical",
        "materialize-historical",
        "run-native",
    ):
        command = commands.add_parser(name)
        command.add_argument("--repository-root", default=".")
        if name in {"audit-historical", "materialize-historical"}:
            command.add_argument("--external-root", required=True)
        if name == "run-native":
            command.add_argument("--config", required=True)
        handlers = {
            "validate-tree": _validate_tree_command,
            "audit-historical": _audit_historical_command,
            "materialize-historical": _materialize_historical_command,
            "run-native": _run_native_command,
        }
        command.set_defaults(handler=handlers[name])
    args = parser.parse_args(argv)
    try:
        return args.handler(args)
    except ClassificationError as exc:
        print(f"FAIL measurement-classification: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
