"""Typed competitive run definitions and safe experiment receipts."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from experiments import _lib


CONFIG_SCHEMA = "competitive.run.v1"
RESULT_SCHEMA = "competitive.result.v1"
_SHA = re.compile(r"[0-9a-f]{64}\Z")
_SETTING_NAME = re.compile(r"[A-Za-z][A-Za-z0-9_.-]{0,127}\Z")
_SETTING_VALUE = re.compile(r"[A-Za-z0-9/][A-Za-z0-9_.:/+-]{0,127}\Z")
_UNSAFE_SETTING_NAME = re.compile(
    r"(?:^|_)(?:api_key|secret|password|credential|auth|authorization|"
    r"access_token|refresh_token|private_key|token|prompt|question|answer|prediction|"
    r"predictions|raw|payload|content|corpus|document|documents|message|messages)"
    r"(?:_|$)"
)
_METRICS = {
    "latency_capacity": {
        "p50_ms",
        "p95_ms",
        "p99_ms",
        "queries_per_s",
        "ingest_items_per_s",
        "ready_to_search_ms",
        "peak_rss_mib",
        "storage_mib",
    },
    "retrieval": {
        "recall_at_5",
        "recall_at_10",
        "ndcg_at_10",
        "mrr",
        "negative_abstention",
        "complete_support_at_10",
    },
    "answer": {
        "accuracy",
        "f1",
        "grounded_rate",
        "attribution_rate",
        "temporal_correctness",
        "cost_usd",
    },
}
_PROPORTIONS = _METRICS["retrieval"] | (_METRICS["answer"] - {"cost_usd"})


def _object(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be an object")
    actual = set(value)
    if actual != keys:
        raise ValueError(
            f"{label} keys mismatch: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _text(value: object, label: str) -> None:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be non-empty text")


def _positive_int(value: object, label: str) -> None:
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise ValueError(f"{label} must be a positive integer")


def _sha(value: object, label: str, *, optional: bool = False) -> None:
    if optional and value is None:
        return
    if not isinstance(value, str) or _SHA.fullmatch(value) is None:
        raise ValueError(f"{label} must be a lowercase SHA-256 digest")


def _hardware(value: object, label: str) -> dict[str, Any]:
    hardware = _object(value, label, {"cpu", "cores", "ram_gib", "storage", "gpu"})
    for key in ("cpu", "storage", "gpu"):
        _text(hardware[key], f"{label}.{key}")
    _positive_int(hardware["cores"], f"{label}.cores")
    ram = hardware["ram_gib"]
    if (
        isinstance(ram, bool)
        or not isinstance(ram, (int, float))
        or not math.isfinite(ram)
        or ram <= 0
    ):
        raise ValueError(f"{label}.ram_gib must be positive and finite")
    return hardware


def _settings(value: object, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be an object of scalar tuning settings")
    for key, setting in value.items():
        if not isinstance(key, str) or _SETTING_NAME.fullmatch(key) is None:
            raise ValueError(f"{label} names must be short tuning identifiers")
        normalized = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", key)
        normalized = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", normalized)
        normalized = normalized.lower().replace(".", "_").replace("-", "_")
        if normalized != "max_tokens" and _UNSAFE_SETTING_NAME.search(normalized):
            raise ValueError(f"{label}.{key} may contain sensitive or raw content")
        if isinstance(setting, str):
            if _SETTING_VALUE.fullmatch(setting) is None or setting.lower().startswith(
                "sk-"
            ):
                raise ValueError(
                    f"{label}.{key} must be a short, content-free identifier"
                )
        elif setting is not None and not isinstance(setting, (bool, int, float)):
            raise ValueError(f"{label}.{key} must be a scalar tuning value")
    try:
        json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)
    except (TypeError, ValueError) as exc:
        raise ValueError(f"{label} must contain JSON values") from exc
    return value


def validate_definition(value: object) -> dict[str, Any]:
    """Validate a resolved run definition before execution or recording."""
    config = _object(
        value,
        "run definition",
        {
            "schema_version",
            "case_id",
            "scoreboard",
            "product",
            "evidence",
            "workload",
            "measurement",
            "target_hardware",
            "settings",
        },
    )
    if config["schema_version"] != CONFIG_SCHEMA:
        raise ValueError("run definition schema_version mismatch")
    _text(config["case_id"], "case_id")
    if (
        not isinstance(config["scoreboard"], str)
        or config["scoreboard"] not in _METRICS
    ):
        raise ValueError("unknown scoreboard")
    product = _object(config["product"], "product", {"name", "version", "deployment"})
    for key in product:
        _text(product[key], f"product.{key}")
    if product["deployment"] != "embedded":
        raise ValueError("competitive v1 covers embedded deployments only")
    evidence = _object(config["evidence"], "evidence", {"kind", "reference_url"})
    if not isinstance(evidence["kind"], str) or evidence["kind"] not in {
        "local",
        "published",
    }:
        raise ValueError("evidence.kind must be local or published")
    if evidence["kind"] == "published":
        _text(evidence["reference_url"], "evidence.reference_url")
        if not evidence["reference_url"].startswith("https://"):
            raise ValueError("published reference_url must use HTTPS")
    elif evidence["reference_url"] is not None:
        raise ValueError("local evidence has no reference_url")
    workload = _object(
        config["workload"],
        "workload",
        {
            "name",
            "corpus_sha256",
            "queries_sha256",
            "items",
            "queries",
            "search_mode",
            "top_k",
            "filter",
        },
    )
    for key in ("name", "search_mode", "filter"):
        _text(workload[key], f"workload.{key}")
    for key in ("corpus_sha256", "queries_sha256"):
        _sha(workload[key], f"workload.{key}", optional=evidence["kind"] == "published")
    for key in ("items", "queries", "top_k"):
        _positive_int(workload[key], f"workload.{key}")
    measurement = _object(
        config["measurement"],
        "measurement",
        {"boundary", "cache_state", "concurrency", "repetitions", "seed"},
    )
    for key in ("boundary", "cache_state"):
        _text(measurement[key], f"measurement.{key}")
    for key in ("concurrency", "repetitions"):
        _positive_int(measurement[key], f"measurement.{key}")
    if isinstance(measurement["seed"], bool) or not isinstance(
        measurement["seed"], int
    ):
        raise ValueError("measurement.seed must be an integer")
    _hardware(config["target_hardware"], "target_hardware")
    _settings(config["settings"], "settings")
    return config


def validate_result(config: object, value: object) -> dict[str, Any]:
    """Validate a safe aggregate result against its exact run definition."""
    definition = validate_definition(config)
    result = _object(
        value,
        "result",
        {
            "schema_version",
            "case_id",
            "config_sha256",
            "status",
            "observed",
            "metrics",
            "source",
            "comparability",
            "reason",
        },
    )
    if result["schema_version"] != RESULT_SCHEMA:
        raise ValueError("result schema_version mismatch")
    if result["case_id"] != definition["case_id"]:
        raise ValueError("result case_id drift")
    if result["config_sha256"] != _lib.config_sha256(definition):
        raise ValueError("result config_sha256 drift")
    if result["status"] not in {"complete", "blocked"}:
        raise ValueError("result status must be complete or blocked")
    if result["status"] == "complete" or result["observed"] is not None:
        observed = _object(
            result["observed"],
            "observed",
            {
                "product_version",
                "cpu",
                "cores",
                "ram_gib",
                "storage",
                "gpu",
                "settings",
                "workload",
                "measurement",
                "queries_completed",
                "errors",
            },
        )
        _hardware(
            {
                key: observed[key]
                for key in ("cpu", "cores", "ram_gib", "storage", "gpu")
            },
            "observed",
        )
        if observed["product_version"] != definition["product"]["version"]:
            raise ValueError("observed product version drift")
        _settings(observed["settings"], "observed.settings")
        if observed["settings"] != definition["settings"]:
            raise ValueError("observed settings drift")
        if observed["workload"] != definition["workload"]:
            raise ValueError("observed workload drift")
        if observed["measurement"] != definition["measurement"]:
            raise ValueError("observed measurement drift")
        for key in ("queries_completed", "errors"):
            number = observed[key]
            if isinstance(number, bool) or not isinstance(number, int) or number < 0:
                raise ValueError(f"observed.{key} must be a nonnegative integer")
    else:
        observed = None
    if not isinstance(result["metrics"], dict):
        raise ValueError("metrics must be an object")
    unknown = set(result["metrics"]) - _METRICS[definition["scoreboard"]]
    if unknown:
        raise ValueError(
            f"metric not allowed for {definition['scoreboard']}: {sorted(unknown)}"
        )
    for name, metric in result["metrics"].items():
        if (
            isinstance(metric, bool)
            or not isinstance(metric, (int, float))
            or not math.isfinite(metric)
        ):
            raise ValueError(f"metric {name} must be finite numeric")
        if metric < 0 or (name in _PROPORTIONS and metric > 1):
            raise ValueError(f"metric {name} is outside its range")
    source = _object(result["source"], "source", {"url", "locator", "sha256"})
    _text(source["locator"], "source.locator")
    _sha(source["sha256"], "source.sha256")
    if source["url"] != definition["evidence"]["reference_url"]:
        raise ValueError("source URL drift")
    if result["comparability"] not in {"candidate_match", "context_only"}:
        raise ValueError("comparability must be candidate_match or context_only")
    if (
        definition["evidence"]["kind"] == "published"
        and result["comparability"] != "context_only"
    ):
        raise ValueError("published reference must remain context_only")
    if result["status"] == "complete":
        if not result["metrics"] or result["reason"] is not None:
            raise ValueError("complete result needs metrics and no reason")
        expected_queries = (
            definition["workload"]["queries"] * definition["measurement"]["repetitions"]
        )
        if (
            observed is None
            or observed["errors"]
            or observed["queries_completed"] != expected_queries
        ):
            raise ValueError("complete result requires all queries and zero errors")
    elif (
        result["metrics"]
        or not isinstance(result["reason"], str)
        or not result["reason"].strip()
    ):
        raise ValueError("blocked result needs a reason and no metrics")
    return result


def record_run(
    config: object,
    result: object,
    *,
    ts: datetime,
    base_dir: str | Path,
    code: dict[str, Any],
    env: dict[str, Any],
    execution: dict[str, Any] | None = None,
    attempt_id: str | None = None,
) -> tuple[str, Path]:
    """Write a competitive result using the existing experiment envelope."""
    definition = validate_definition(config)
    output = validate_result(definition, result)
    if (
        definition["evidence"]["kind"] == "local"
        and output["status"] == "complete"
        and execution is None
    ):
        raise ValueError("local complete result requires an execution witness")
    if execution is not None:
        if (
            execution.get("schema_version") != "competitive.execution.v1"
            or execution.get("config_sha256") != _lib.config_sha256(definition)
            or execution.get("source_sha256") != output["source"]["sha256"]
            or execution.get("status") != output["status"]
            or (attempt_id is not None and execution.get("attempt_id") != attempt_id)
        ):
            raise ValueError("competitive execution witness drift")
        locator = execution.get("result_locator")
        digest = execution.get("result_sha256")
        _sha(digest, "execution.result_sha256")
        if not isinstance(locator, str) or not Path(locator).is_absolute():
            raise ValueError("execution result artifact must have an absolute locator")
        try:
            artifact = Path(locator).read_bytes()
            actual = json.loads(artifact.decode("utf-8"))
        except (OSError, UnicodeError, ValueError) as exc:
            raise ValueError(
                "execution result artifact is unavailable or invalid"
            ) from exc
        if hashlib.sha256(artifact).hexdigest() != digest or actual != output:
            raise ValueError("execution result artifact drift")
    if attempt_id is not None and re.fullmatch(r"[0-9a-f]{16}", attempt_id) is None:
        raise ValueError("attempt_id must be 16 lowercase hexadecimal characters")
    experiment = "competitive-" + definition["scoreboard"]
    if attempt_id is not None:
        experiment += "-" + attempt_id
    base = Path(base_dir)
    run_id = _lib.make_run_id(experiment, ts, _lib.config_sha256(definition))
    run_dir = base / "runs" / run_id
    if run_id in _lib.index_run_ids(base / "index.jsonl"):
        raise ValueError(f"competitive run {run_id} already exists in the index")
    try:
        run_dir.mkdir(parents=True, exist_ok=False)
    except FileExistsError as exc:
        raise ValueError(f"competitive run {run_id} already exists") from exc
    return _lib.write_record(
        experiment,
        ts=ts,
        config_obj=definition,
        metrics={
            "competitive_result": output,
            **({"competitive_execution": execution} if execution is not None else {}),
        },
        verdict=output["status"],
        read=f"{definition['case_id']}: {output['status']}",
        code=code,
        corpus={
            "source": definition["workload"]["name"],
            "manifest_sha256": definition["workload"]["corpus_sha256"],
            "datasets": [definition["workload"]["name"]],
        },
        seeds={"query_order": definition["measurement"]["seed"]},
        env=env,
        cost_usd=output["metrics"].get("cost_usd"),
        n=output["observed"]["queries_completed"] if output["observed"] else 0,
        headline={
            "case_id": definition["case_id"],
            "scoreboard": definition["scoreboard"],
            "program_track": execution.get("program_track") if execution else None,
        },
        artifacts=[
            {
                "locator": output["source"]["locator"],
                "sha256": output["source"]["sha256"],
            },
            *(
                [
                    {
                        "locator": execution["result_locator"],
                        "sha256": execution["result_sha256"],
                    }
                ]
                if execution is not None
                else []
            ),
        ],
        base_dir=base,
    )


def _cell(value: object) -> str:
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_scoreboards(runs_dir: str | Path) -> str:
    """Render three evidence-labeled views from validated competitive receipts."""
    rows: dict[str, list[str]] = {family: [] for family in _METRICS}
    invalid_rows: list[str] = []

    def append_invalid(path: Path, family: object) -> None:
        identifier = hashlib.sha256(path.read_bytes()).hexdigest()[:12]
        invalid_rows.append(f"| {identifier} | invalid |")
        if not isinstance(family, str) or family not in rows:
            return
        rows[family].append(
            "| "
            + " | ".join(
                [
                    identifier,
                    "",
                    "",
                    "",
                    "",
                    "",
                    "invalid",
                    "",
                    "",
                    "",
                    "invalid",
                    "invalid",
                ]
            )
            + " |"
        )

    for path in sorted(Path(runs_dir).glob("*/record.json")):
        raw_record = None
        try:
            raw_record = json.loads(path.read_text(encoding="utf-8"))
            record = _lib.record_from_dict(raw_record)
        except (ValueError, TypeError, KeyError):
            raw_config = (
                raw_record.get("config", {}) if isinstance(raw_record, dict) else {}
            )
            resolved = (
                raw_config.get("resolved", {}) if isinstance(raw_config, dict) else {}
            )
            family = resolved.get("scoreboard") if isinstance(resolved, dict) else None
            if path.parent.name.startswith("competitive-"):
                append_invalid(path, family)
            continue
        config = record.config.resolved
        if not isinstance(config, dict):
            append_invalid(path, None)
            continue
        if (
            not isinstance(record.metrics, dict)
            or "competitive_result" not in record.metrics
        ):
            if config.get("schema_version") in {
                "competitive.run.v1",
                "competitive.run.v2",
            }:
                append_invalid(path, config.get("scoreboard"))
            continue
        if config.get("schema_version") == "competitive.run.v2":
            from experiments import competitive_product

            try:
                product_row = competitive_product.validate_receipt(raw_record)
            except (ValueError, TypeError, KeyError):
                append_invalid(path, config.get("scoreboard"))
                continue
            result = record.metrics["competitive_result"]
            evidence_label = product_row["evidence_status"]
            comparability_label = product_row["comparability"]
            execution = record.metrics.get("competitive_execution")
            adapter = config["adapter"]
        elif config.get("schema_version") == CONFIG_SCHEMA:
            try:
                result = validate_result(config, record.metrics["competitive_result"])
                if record.config.sha256 != _lib.config_sha256(config):
                    raise ValueError(f"receipt config hash drift: {record.run_id}")
            except (ValueError, TypeError, KeyError):
                append_invalid(path, config.get("scoreboard"))
                continue
            evidence_label = "legacy_path_bearing"
            comparability_label = "context_only"
            execution = record.metrics.get("competitive_execution")
            adapter = (
                execution.get("adapter", {}) if isinstance(execution, dict) else {}
            )
        else:
            append_invalid(path, config.get("scoreboard"))
            continue
        hardware = result["observed"]
        adapter_label = (
            adapter.get("id", "manual") if isinstance(adapter, dict) else "manual"
        )
        adapter_sha = (
            adapter.get("sha256", "manual") if isinstance(adapter, dict) else "manual"
        )
        plan_sha = (
            execution.get("plan_sha256", "manual")
            if isinstance(execution, dict)
            else "manual"
        )
        metric_text = ", ".join(
            f"{key}={value:.6g}" for key, value in sorted(result["metrics"].items())
        )
        values = (
            record.run_id,
            record.config.sha256[:12],
            plan_sha[:12],
            adapter_label,
            adapter_sha[:12],
            f"{config['product']['name']} {config['product']['version']}",
            evidence_label,
            config["workload"]["name"],
            f"{hardware['cpu']}, {hardware['cores']} cores, {hardware['ram_gib']} GiB"
            if hardware
            else "not observed",
            metric_text or result["reason"],
            comparability_label,
            result["status"],
        )
        rows[config["scoreboard"]].append(
            "| " + " | ".join(_cell(value) for value in values) + " |"
        )
    lines = [
        "# Competitive scoreboards",
        "",
        "Generated from validated `experiments.record.v1` receipts. Published rows are context only.",
        "",
    ]
    for family, heading in (
        ("latency_capacity", "Latency and capacity"),
        ("retrieval", "Retrieval"),
        ("answer", "Answer"),
    ):
        lines.extend(
            [
                f"## {heading}",
                "",
                "| Run ID | Config SHA | Plan SHA | Adapter | Adapter SHA | Product | Evidence | Workload | Observed hardware | Metrics | Comparability | Status |",
                "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
                *rows[family],
                "",
            ]
        )
    if invalid_rows:
        lines.extend(
            [
                "## Invalid evidence",
                "",
                "| Receipt SHA | Status |",
                "| --- | --- |",
                *invalid_rows,
                "",
            ]
        )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    """Validate, record, or render competitive experiment evidence."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate")
    validate.add_argument("config", type=Path)
    validate.add_argument("result", type=Path, nargs="?")
    record = sub.add_parser("record")
    record.add_argument("config", type=Path)
    record.add_argument("result", type=Path)
    record.add_argument("--base-dir", type=Path, default=_lib.EXPERIMENTS_DIR)
    render = sub.add_parser("render")
    render.add_argument("--runs-dir", type=Path, default=_lib.EXPERIMENTS_DIR / "runs")
    render.add_argument(
        "--output",
        type=Path,
        default=_lib.EXPERIMENTS_DIR / "COMPETITIVE-SCOREBOARDS.md",
    )
    args = parser.parse_args(argv)
    try:
        if args.command == "render":
            args.output.write_text(render_scoreboards(args.runs_dir), encoding="utf-8")
            return 0
        config = json.loads(args.config.read_text(encoding="utf-8"))
        validate_definition(config)
        if args.command == "validate" and args.result is None:
            return 0
        result = json.loads(args.result.read_text(encoding="utf-8"))
        validate_result(config, result)
        if args.command == "record":
            run_id, _ = record_run(
                config,
                result,
                ts=datetime.now(UTC),
                base_dir=args.base_dir,
                code={**_lib.git_info(), "baseline_commit": None},
                env=_lib.env_info(),
            )
            print(run_id)
    except (OSError, ValueError, TypeError) as exc:
        parser.error(str(exc))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
