"""Read-only integrity validation and rendering for SDK benchmark suites."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any

from experiments import _lib, competitive_product, sdk_answer, sdk_benchmark


FAMILIES = ("latency_capacity", "retrieval", "answer")
_METRICS = {
    "latency_capacity": {
        "p50_ms", "p95_ms", "p99_ms", "queries_per_s",
        "database_bytes", "driver_peak_rss_bytes",
    },
    "retrieval": {
        "p50_ms",
        "p95_ms",
        "p99_ms",
        "recall_at_10",
        "mrr",
        "ndcg_at_10",
        "negative_abstention",
    },
    "answer": {
        "answer_accuracy",
        "answer_f1",
        "groundedness",
        "attribution",
        "cost_usd",
        "answer_ms",
        "judge_ms",
    },
}
_ID = re.compile(r"[a-z][a-z0-9-]*\Z")
_REASON = re.compile(r"[A-Za-z][A-Za-z0-9_]{0,63}\Z")


def _required_metrics(plan: dict[str, Any], case: dict[str, Any]) -> set[str]:
    return set(plan["output"]["metrics"]) & sdk_benchmark._METRICS[case["family"]]


def _suite_status(
    cells: list[tuple[bool, str, str | None]],
) -> tuple[str, int]:
    optional_unsupported = sum(
        not required and status == "blocked" and reason == "unsupported_operation"
        for required, status, reason in cells
    )
    statuses = [
        status
        for required, status, reason in cells
        if required or status != "blocked" or reason != "unsupported_operation"
    ]
    status = (
        "invalid"
        if "invalid" in statuses
        else "incomplete"
        if "incomplete" in statuses
        else "blocked"
        if "blocked" in statuses
        else "complete"
    )
    return status, optional_unsupported


def _safe_reason(reason: object) -> str:
    return (
        reason
        if isinstance(reason, str) and _REASON.fullmatch(reason)
        else "invalid_reason"
    )


def _object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_object)
    if not isinstance(value, dict):
        raise ValueError(f"{path.name} must be an object")
    return value


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _require(condition: bool, reason: str) -> None:
    if not condition:
        raise ValueError(reason)


def _is_path_free(value: Any) -> bool:
    if isinstance(value, str):
        return not Path(value).is_absolute() and "\\" not in value
    if isinstance(value, list):
        return all(_is_path_free(item) for item in value)
    if isinstance(value, dict):
        return all(
            _is_path_free(key) and _is_path_free(item) for key, item in value.items()
        )
    return True


def _projection(execution: dict[str, Any], plan_sha: str) -> dict[str, Any]:
    _require(
        execution.get("schema_version") == "sdk.execution-config.v1", "execution schema"
    )
    plan = execution["plan"]
    arm = execution["arm"]
    case = execution["case"]
    sdk = arm["sdk"]
    arm_binding = next(item for item in plan["arms"] if item["sdk"] == sdk)
    _require(
        execution["sampling"] == {**plan["sampling"], "timeout_ms": 30000},
        "sampling drift",
    )
    return {
        "schema_version": "sdk.receipt-config.v1",
        "program_track": plan["program_track"],
        "suite_id": plan["suite_id"],
        "suite_config_sha256": plan_sha,
        "case_id": case["id"],
        "family": case["family"],
        "operation_id": case["operation_id"],
        "operation_version": case["operation_version"],
        **({"scale_tier": case["scale_tier"]} if "scale_tier" in case else {}),
        "sdk": sdk,
        "arguments": sdk_benchmark._safe_arguments(case),
        "sampling": execution["sampling"],
        "fixture_sha256": plan["fixtures"]["manifest"]["sha256"],
        "arm_manifest_sha256": arm_binding["artifact"]["sha256"],
        "engine_revision": arm["engine_revision"],
        "dependency_closure_sha256": arm["dependency_closure_sha256"],
        "native_sha256": arm["native"]["sha256"],
        "driver_sha256": arm["driver"]["sha256"],
    }


def _index_rows(path: Path) -> dict[str, list[dict[str, Any]]]:
    rows: dict[str, list[dict[str, Any]]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        row = json.loads(line, object_pairs_hook=_object)
        _require(
            isinstance(row, dict) and isinstance(row.get("run_id"), str),
            "invalid index row",
        )
        rows.setdefault(row["run_id"], []).append(row)
    return rows


def _cell(
    entry: dict[str, Any],
    *,
    plan: dict[str, Any],
    plan_sha: str,
    attempt: str,
    external_root: Path,
    receipts_root: Path,
    index: dict[str, list[dict[str, Any]]],
) -> dict[str, Any]:
    case = next(item for item in plan["cases"] if item["id"] == entry["case_id"])
    sdk = entry["sdk"]
    cell_id = f"{case['id']}-{sdk}"
    run_id = entry["run_id"]
    _require(
        isinstance(run_id, str) and run_id and "/" not in run_id and "\\" not in run_id,
        "run ID",
    )
    _require(len(index.get(run_id, [])) == 1, "missing or duplicate index row")
    index_row = index[run_id][0]
    _require(
        index_row.get("schema_version") == "experiments.index-row.v1", "index schema"
    )
    record_root = receipts_root / "runs" / run_id
    record = _read_json(record_root / "record.json")
    _require(record.get("schema_version") == "experiments.record.v1", "receipt schema")
    _require(record.get("run_id") == run_id, "receipt run ID")
    _require(
        _is_path_free(record) and _is_path_free(index_row),
        "path-bearing receipt or index",
    )
    config_ref = record["config"]
    config = config_ref["resolved"]
    config_sha = _lib.config_sha256(config)
    _require(config_ref.get("sha256") == config_sha, "receipt config hash")
    _require(
        config_ref.get("path") is None and _is_path_free(config),
        "receipt config contains a path",
    )
    _require(index_row.get("config_sha256") == config_sha, "index config hash")
    _require(
        index_row.get("experiment") == record.get("experiment"), "index experiment"
    )
    _require(index_row.get("verdict") == record.get("verdict"), "index verdict")
    _require(index_row.get("repo") == "fathomdb", "index repository")
    _require(run_id.endswith(config_sha[:8]), "run ID config suffix")
    _require(isinstance(record.get("metrics"), dict), "receipt metrics shape")
    _require(
        index_row.get("headline")
        == {
            key: value
            for key, value in record["metrics"].items()
            if key not in {"status", "result_sha256", "execution_witness_sha256"}
        },
        "index headline",
    )
    _require(record["metrics"].get("status") == record.get("verdict"), "receipt status")
    _require(
        record["artifacts"]
        == [
            {"id": f"{cell_id}:result", "sha256": entry["result_sha256"]},
            {"id": f"{cell_id}:execution", "sha256": entry["witness_sha256"]},
        ],
        "receipt artifact links",
    )
    try:
        import yaml

        safe_config = yaml.safe_load(
            (record_root / "config.resolved.yaml").read_text(encoding="utf-8")
        )
    except ImportError:
        safe_config = _read_json(record_root / "config.resolved.yaml")
    _require(safe_config == config, "resolved config drift")
    _require(
        _read_json(record_root / "metrics.json") == record["metrics"],
        "metrics sidecar drift",
    )
    cell_root = external_root / cell_id
    result_path = cell_root / "result.json"
    witness_path = cell_root / "execution.json"
    execution_path = cell_root / "execution-config.json"
    _require(_sha(result_path) == entry["result_sha256"], "result digest")
    _require(_sha(witness_path) == entry["witness_sha256"], "witness digest")
    result = _read_json(result_path)
    witness = _read_json(witness_path)
    execution = _read_json(execution_path)
    _require(
        execution["plan"] == plan and execution["case"] == case,
        "execution plan/case drift",
    )
    _require(execution["arm"]["sdk"] == sdk, "execution SDK drift")
    _require(
        _projection(execution, plan_sha) == config, "unsafe or mismatched projection"
    )
    _require(witness.get("schema_version") == "sdk.execution.v1", "witness schema")
    for field, expected in {
        "suite_attempt_id": attempt,
        "case_id": case["id"],
        "sdk": sdk,
        "execution_config_sha256": _sha(execution_path),
        "receipt_config_sha256": config_sha,
        "result_sha256": entry["result_sha256"],
        "status": entry["status"],
    }.items():
        _require(witness.get(field) == expected, f"witness {field}")
    _require(witness.get("result_locator") == str(result_path), "witness locator")
    for field, expected in {
        "schema_version": "sdk.result.v1",
        "suite_id": plan["suite_id"],
        "suite_attempt_id": attempt,
        "case_id": case["id"],
        "sdk": sdk,
        "axis": "sdk",
        "family": case["family"],
        "producer": "answer_pipeline" if case["family"] == "answer" else "driver",
        "operation_id": case["operation_id"],
        "operation_version": case["operation_version"],
        **({"scale_tier": case["scale_tier"]} if "scale_tier" in case else {}),
        "status": entry["status"],
    }.items():
        _require(result.get(field) == expected, f"result {field}")
    _require(
        record["metrics"].get("result_sha256") == entry["result_sha256"],
        "receipt result digest",
    )
    _require(
        record["metrics"].get("execution_witness_sha256") == entry["witness_sha256"],
        "receipt witness digest",
    )
    _require(record.get("verdict") == entry["status"], "receipt verdict")
    _require(entry["status"] in {"complete", "blocked"}, "invalid result status")
    metrics = result.get("metrics")
    _require(isinstance(metrics, dict), "result metrics")
    if entry["status"] == "blocked":
        _require(
            _safe_reason(result.get("reason")) == result.get("reason"),
            "unsafe blocked reason",
        )
        _require(
            not metrics and not result.get("samples") and result.get("reason"),
            "blocked result has samples",
        )
        _require(
            not (
                set(record["metrics"])
                - {"status", "result_sha256", "execution_witness_sha256"}
            ),
            "blocked receipt has metrics",
        )
        _require(index_row.get("n") == 0, "blocked index count")
    else:
        _require(
            result.get("reason") is None and metrics, "complete result missing metrics"
        )
        if case["family"] == "retrieval":
            _require(
                result.get("evidence_sha256") == _sha(cell_root / "evidence.json"),
                "retrieval evidence digest",
            )
        _require(set(metrics) <= _METRICS[case["family"]], "unexpected family metric")
        _require(
            all(
                isinstance(v, (int, float))
                and not isinstance(v, bool)
                and math.isfinite(v)
                for v in metrics.values()
            ),
            "non-finite or non-numeric metric",
        )
        _require(
            _required_metrics(plan, case) <= set(metrics),
            "required metric missing",
        )
        if case["family"] == "answer":
            answer_config = case["arguments"]["answer_config"]
            upstream_id = answer_config["upstream"]["case_id"]
            bound = result.get("upstream")
            _require(
                isinstance(bound, dict)
                and bound.get("suite_attempt_id") == attempt
                and bound.get("case_id") == upstream_id
                and bound.get("sdk") == sdk,
                "answer upstream identity",
            )
            upstream_root = external_root / f"{upstream_id}-{sdk}"
            upstream_result = _read_json(upstream_root / "result.json")
            _require(
                bound["retrieval_result_sha256"] == _sha(upstream_root / "result.json")
                and bound["evidence_sha256"] == _sha(upstream_root / "evidence.json"),
                "answer upstream digest",
            )
            _require(
                upstream_result.get("evidence_sha256") == bound["evidence_sha256"],
                "answer upstream evidence binding",
            )
            _require(
                result.get("config_sha256")
                == sdk_answer.validate_answer_config(answer_config),
                "answer config digest",
            )
            samples_path = cell_root / "answer-samples.json"
            _require(
                result.get("external_samples_sha256") == _sha(samples_path)
                and result.get("sample_count") == index_row.get("n"),
                "answer sample count or digest",
            )
            answer_witness = _read_json(cell_root / "answer-witness.json")
            _require(
                answer_witness.get("upstream") == bound
                and answer_witness.get("result_sha256") == entry["result_sha256"],
                "answer stage witness",
            )
        else:
            _require(
                isinstance(result.get("samples"), list)
                and index_row.get("n") == len(result["samples"]),
                "sample count",
            )
        _require(
            {
                key: value
                for key, value in record["metrics"].items()
                if key not in {"status", "result_sha256", "execution_witness_sha256"}
            }
            == metrics,
            "receipt metrics",
        )
    return {
        "case_id": case["id"],
        "arm": sdk,
        "axis": "sdk",
        "scale_tier": case.get("scale_tier"),
        "status": entry["status"],
        "reason": _safe_reason(result.get("reason"))
        if entry["status"] == "blocked"
        else None,
        "evidence_status": "local_descriptive",
        "source_provenance": result.get("source_provenance", "unverified"),
        "metrics": metrics if entry["status"] == "complete" else {},
        "run_id": run_id,
    }


def build_scoreboard(
    plan_path: str | Path,
    manifest_path: str | Path,
    receipts_root: str | Path,
    *,
    product_sources: list[tuple[str | Path, str | Path | None]] | None = None,
) -> dict[str, Any]:
    """Validate one SDK suite and render three axis-separated, path-free views."""
    plan_path = Path(plan_path)
    manifest_path = Path(manifest_path)
    receipts_root = Path(receipts_root)
    plan = _read_json(plan_path)
    plan_sha = _sha(plan_path)
    _require(plan.get("schema_version") == "sdk.experiment.v1", "plan schema")
    _require(
        isinstance(plan.get("suite_id"), str)
        and _ID.fullmatch(plan["suite_id"]) is not None,
        "suite ID",
    )
    _require(
        isinstance(plan.get("cases"), list)
        and bool(plan["cases"])
        and isinstance(plan.get("arms"), list)
        and bool(plan["arms"]),
        "plan roster",
    )
    for case in plan["cases"]:
        _require(
            isinstance(case, dict)
            and isinstance(case.get("id"), str)
            and _ID.fullmatch(case["id"]) is not None,
            "case ID",
        )
        _require(type(case.get("required")) is bool, "case required")
    for arm in plan["arms"]:
        _require(
            isinstance(arm, dict)
            and arm.get("sdk") in {"python", "typescript", "rust"},
            "SDK arm",
        )
    expected = [
        f"{case['id']}-{arm['sdk']}" for case in plan["cases"] for arm in plan["arms"]
    ]
    _require(len(expected) == len(set(expected)), "duplicate plan cell")
    try:
        manifest = _read_json(manifest_path)
    except (OSError, ValueError, json.JSONDecodeError):
        manifest = {}
    attempt = manifest.get("suite_attempt_id")
    manifest_valid = (
        manifest.get("schema_version") == "sdk.suite-manifest.v1"
        and manifest.get("suite_id") == plan.get("suite_id")
        and manifest.get("suite_config_sha256") == plan_sha
        and isinstance(attempt, str)
        and bool(attempt)
        and _is_path_free(attempt)
        and manifest.get("expected_cells") == expected
        and isinstance(manifest.get("cells"), list)
    )
    if not isinstance(attempt, str) or not _is_path_free(attempt):
        attempt = None
    entries: dict[str, dict[str, Any]] = {}
    for entry in (
        manifest.get("cells", []) if isinstance(manifest.get("cells"), list) else []
    ):
        if not isinstance(entry, dict):
            manifest_valid = False
            continue
        cell_id = f"{entry.get('case_id')}-{entry.get('sdk')}"
        if cell_id not in expected or cell_id in entries:
            manifest_valid = False
            continue
        entries[cell_id] = entry
    try:
        index = _index_rows(receipts_root / "index.jsonl")
    except (OSError, ValueError, json.JSONDecodeError):
        index = {}
    families: dict[str, dict[str, list[dict[str, Any]]]] = {
        family: {"sdk": [], "product": []} for family in FAMILIES
    }
    cell_statuses: list[tuple[bool, str, str | None]] = []
    for case in plan["cases"]:
        _require(case["family"] in FAMILIES, "case family")
        for arm in plan["arms"]:
            cell_id = f"{case['id']}-{arm['sdk']}"
            entry = entries.get(cell_id)
            if not manifest_valid:
                row = {
                    "case_id": case["id"],
                    "arm": arm["sdk"],
                    "axis": "sdk",
                    "status": "invalid",
                    "reason": "invalid_suite_manifest",
                    "evidence_status": "local_descriptive",
                    "metrics": {},
                }
            elif entry is None:
                row = {
                    "case_id": case["id"],
                    "arm": arm["sdk"],
                    "axis": "sdk",
                    "status": "incomplete",
                    "reason": "missing_expected_cell",
                    "evidence_status": "local_descriptive",
                    "metrics": {},
                }
            else:
                try:
                    row = _cell(
                        entry,
                        plan=plan,
                        plan_sha=plan_sha,
                        attempt=attempt,
                        external_root=manifest_path.parent,
                        receipts_root=receipts_root,
                        index=index,
                    )
                except (
                    OSError,
                    ValueError,
                    KeyError,
                    TypeError,
                    StopIteration,
                    json.JSONDecodeError,
                ) as exc:
                    row = {
                        "case_id": case["id"],
                        "arm": arm["sdk"],
                        "axis": "sdk",
                        "status": "invalid",
                        "reason": type(exc).__name__,
                        "evidence_status": "local_descriptive",
                        "metrics": {},
                    }
            families[case["family"]]["sdk"].append(row)
            cell_statuses.append((case["required"], row["status"], row.get("reason")))
    product_errors: list[dict[str, str]] = []
    for position, (record_path, external_root) in enumerate(product_sources or []):
        try:
            row = competitive_product.validate_receipt(
                _read_json(Path(record_path)), external_root
            )
            _require(
                row.get("axis") == "product"
                and row.get("family") in FAMILIES
                and _is_path_free(row),
                "invalid product row",
            )
            families[row["family"]]["product"].append(row)
        except (
            OSError,
            ValueError,
            KeyError,
            TypeError,
            AttributeError,
            json.JSONDecodeError,
        ):
            product_errors.append(
                {"source_id": f"product-{position}", "status": "invalid"}
            )
    suite_status, optional_unsupported = _suite_status(cell_statuses)
    return {
        "schema_version": "sdk.scoreboard.v1",
        "suite_id": plan["suite_id"],
        "suite_attempt_id": attempt,
        "suite_status": suite_status,
        "optional_unsupported_cells": optional_unsupported,
        "expected_cells": len(expected),
        "observed_cells": len(entries),
        "product_errors": product_errors,
        "families": families,
    }


def main() -> None:
    """Print a validated scoreboard as JSON without altering source artifacts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plan", type=Path)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("receipts_root", type=Path)
    parser.add_argument("--product-record", action="append", type=Path, default=[])
    parser.add_argument("--product-external", action="append", type=Path, default=[])
    args = parser.parse_args()
    if args.product_external and len(args.product_external) != len(args.product_record):
        parser.error("provide one --product-external per --product-record")
    sources = [
        (record, args.product_external[index] if args.product_external else None)
        for index, record in enumerate(args.product_record)
    ]
    print(
        json.dumps(
            build_scoreboard(
                args.plan, args.manifest, args.receipts_root, product_sources=sources
            ),
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
