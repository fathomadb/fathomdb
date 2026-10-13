"""Path-free successor product receipts backed by external execution evidence."""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from experiments import _lib, competitive_run, competitive_runner


CONFIG_SCHEMA = "competitive.run.v2"
RESULT_SCHEMA = "competitive.result.v2"
EXECUTION_CONFIG_SCHEMA = "competitive.execution-config.v2"
EXECUTION_SCHEMA = "competitive.execution.v2"
RECEIPT_WITNESS_SCHEMA = "competitive.receipt-witness.v2"
_SHA = re.compile(r"[0-9a-f]{64}\Z")
_ATTEMPT = re.compile(r"[0-9a-f]{16}\Z")


def _hash_obj(value: object) -> str:
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), allow_nan=False
        ).encode()
    ).hexdigest()


def _write_json(path: Path, value: object) -> str:
    path.write_text(
        json.dumps(value, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    return competitive_runner._digest(path)


def _exact(value: object, name: str, keys: set[str]) -> dict[str, Any]:
    return competitive_runner._exact(value, name, keys)


def _sha(value: object, name: str) -> str:
    if not isinstance(value, str) or _SHA.fullmatch(value) is None:
        raise ValueError(f"{name} must be a lowercase SHA-256 digest")
    return value


def _assert_safe(value: object, label: str = "safe payload") -> None:
    if isinstance(value, dict):
        for key, item in value.items():
            _assert_safe(key, label)
            _assert_safe(item, label)
    elif isinstance(value, list):
        for item in value:
            _assert_safe(item, label)
    elif isinstance(value, str) and (
        value.startswith(("/", "\\"))
        or re.search(r"(?:^|[\s='\"(])(?:/(?!/)|[A-Za-z]:[/\\])", value)
        or re.search(r"file:(?:/{1,3}|\\)", value, re.I)
        or re.search(r"(?:sk-[A-Za-z0-9]|api[_-]?key|secret)", value, re.I)
    ):
        raise ValueError(f"{label} contains a path or secret-looking value")


def safe_projection(execution_config: object) -> dict[str, Any]:
    """Project a full external product config to a deterministic path-free config."""
    full = _exact(
        execution_config,
        "execution config",
        {
            "schema_version",
            "program_track",
            "plan_sha256",
            "run",
            "adapter",
            "inputs",
            "output",
        },
    )
    if full["schema_version"] != EXECUTION_CONFIG_SCHEMA:
        raise ValueError("execution config schema mismatch")
    _sha(full["plan_sha256"], "plan_sha256")
    run = competitive_run.validate_definition(full["run"])
    adapter = _exact(
        full["adapter"], "adapter", {"protocol", "id", "path", "sha256", "timeout_s"}
    )
    if adapter["protocol"] != competitive_runner.ADAPTER_PROTOCOL:
        raise ValueError("adapter protocol mismatch")
    _sha(adapter["sha256"], "adapter.sha256")
    if not isinstance(adapter["path"], str) or not Path(adapter["path"]).is_absolute():
        raise ValueError("adapter.path must be absolute")
    if not isinstance(full["inputs"], dict) or not {"corpus", "queries"} <= set(
        full["inputs"]
    ):
        raise ValueError("execution inputs missing")
    digests = {}
    for name, binding in full["inputs"].items():
        _exact(binding, f"inputs.{name}", {"path", "sha256"})
        if (
            not isinstance(binding["path"], str)
            or not Path(binding["path"]).is_absolute()
        ):
            raise ValueError(f"inputs.{name}.path must be absolute")
        digests[name] = _sha(binding["sha256"], f"inputs.{name}.sha256")
    output = _exact(full["output"], "output", {"schema_version", "metrics"})
    if output["schema_version"] != RESULT_SCHEMA:
        raise ValueError("output schema mismatch")
    safe = {
        **run,
        "schema_version": CONFIG_SCHEMA,
        "input_sha256": digests,
        "adapter": {
            key: adapter[key] for key in ("protocol", "id", "sha256", "timeout_s")
        },
        "output_metrics": output["metrics"],
    }
    _assert_safe(safe)
    return safe


def _execution_config(resolved: dict[str, Any]) -> dict[str, Any]:
    return {
        **resolved,
        "schema_version": EXECUTION_CONFIG_SCHEMA,
    }


def _validate_safe_config(value: object) -> dict[str, Any]:
    run_keys = {
        "schema_version",
        "case_id",
        "scoreboard",
        "product",
        "evidence",
        "workload",
        "measurement",
        "target_hardware",
        "settings",
    }
    safe = _exact(
        value, "safe config", run_keys | {"input_sha256", "adapter", "output_metrics"}
    )
    if safe["schema_version"] != CONFIG_SCHEMA:
        raise ValueError("safe config schema mismatch")
    base = {key: safe[key] for key in run_keys}
    base["schema_version"] = competitive_run.CONFIG_SCHEMA
    competitive_run.validate_definition(base)
    if base["evidence"]["kind"] != "local":
        raise ValueError("safe product config requires local evidence")
    digests = safe["input_sha256"]
    if (
        not isinstance(digests, dict)
        or not {"corpus", "queries"} <= set(digests)
        or set(digests) - {"corpus", "queries", "gold"}
    ):
        raise ValueError("safe input digest set mismatch")
    for name, digest in digests.items():
        _sha(digest, f"input_sha256.{name}")
    for name in ("corpus", "queries"):
        if digests[name] != base["workload"][f"{name}_sha256"]:
            raise ValueError(f"safe input digest drift: {name}")
    if base["scoreboard"] in {"retrieval", "answer"} and "gold" not in digests:
        raise ValueError("safe config requires gold digest")
    adapter = _exact(
        safe["adapter"], "safe adapter", {"protocol", "id", "sha256", "timeout_s"}
    )
    if (
        adapter["protocol"] != competitive_runner.ADAPTER_PROTOCOL
        or not isinstance(adapter["id"], str)
        or competitive_runner._ADAPTER_ID.fullmatch(adapter["id"]) is None
    ):
        raise ValueError("safe adapter identity mismatch")
    _sha(adapter["sha256"], "safe adapter digest")
    if (
        isinstance(adapter["timeout_s"], bool)
        or not isinstance(adapter["timeout_s"], int)
        or adapter["timeout_s"] < 1
    ):
        raise ValueError("safe adapter timeout mismatch")
    metrics = safe["output_metrics"]
    if (
        not isinstance(metrics, list)
        or not metrics
        or any(not isinstance(name, str) for name in metrics)
        or len(metrics) != len(set(metrics))
        or set(metrics) - competitive_run._METRICS[base["scoreboard"]]
    ):
        raise ValueError("safe output metric set mismatch")
    _assert_safe(safe)
    return safe


def _safe_result(
    full_result: dict[str, Any], safe_config: dict[str, Any], execution_config_sha: str
) -> dict[str, Any]:
    if (
        full_result["schema_version"] != RESULT_SCHEMA
        or full_result["config_sha256"] != execution_config_sha
    ):
        raise ValueError("external result execution config hash mismatch")
    source = _exact(
        full_result["source"], "source", {"url", "locator", "sha256", "evidence_kind"}
    )
    safe = {
        **full_result,
        "config_sha256": _lib.config_sha256(safe_config),
        "source": {
            "url": source["url"],
            "sha256": source["sha256"],
            "evidence_kind": source["evidence_kind"],
        },
    }
    _assert_safe(safe)
    return safe


def _validate_safe_result(config: dict[str, Any], result: object) -> dict[str, Any]:
    _validate_safe_config(config)
    safe = _exact(
        result,
        "safe result",
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
    if safe["schema_version"] != RESULT_SCHEMA or safe[
        "config_sha256"
    ] != _lib.config_sha256(config):
        raise ValueError("safe result config hash or schema mismatch")
    source = _exact(safe["source"], "safe source", {"url", "sha256", "evidence_kind"})
    _sha(source["sha256"], "source.sha256")
    if source["evidence_kind"] not in {"adapter_summary", "execution_log"}:
        raise ValueError("source evidence_kind mismatch")
    base = {
        key: value
        for key, value in config.items()
        if key not in {"input_sha256", "adapter", "output_metrics"}
    }
    base["schema_version"] = competitive_run.CONFIG_SCHEMA
    legacy = {
        **safe,
        "schema_version": competitive_run.RESULT_SCHEMA,
        "config_sha256": _lib.config_sha256(base),
        "source": {
            "url": source["url"],
            "locator": "external-evidence",
            "sha256": source["sha256"],
        },
    }
    # The legacy result validator owns metric and observed-state semantics.
    competitive_run.validate_result(base, legacy)
    if safe["status"] == "complete" and set(safe["metrics"]) != set(
        config["output_metrics"]
    ):
        raise ValueError("safe result metric set mismatch")
    _assert_safe(safe)
    return safe


def _validate_witness(
    config: dict[str, Any], result: dict[str, Any], witness: object
) -> dict[str, Any]:
    item = _exact(
        witness,
        "receipt witness",
        {
            "schema_version",
            "attempt_id",
            "config_sha256",
            "execution_config_sha256",
            "result_sha256",
            "safe_result_sha256",
            "execution_sha256",
            "source_sha256",
            "status",
            "plan_sha256",
            "adapter",
        },
    )
    if (
        item["schema_version"] != RECEIPT_WITNESS_SCHEMA
        or not isinstance(item["attempt_id"], str)
        or _ATTEMPT.fullmatch(item["attempt_id"]) is None
    ):
        raise ValueError("receipt witness schema or attempt mismatch")
    for key in (
        "config_sha256",
        "execution_config_sha256",
        "result_sha256",
        "safe_result_sha256",
        "execution_sha256",
        "source_sha256",
        "plan_sha256",
    ):
        _sha(item[key], key)
    if (
        item["config_sha256"] != _lib.config_sha256(config)
        or item["safe_result_sha256"] != _hash_obj(result)
        or item["source_sha256"] != result["source"]["sha256"]
        or item["status"] != result["status"]
        or item["adapter"] != {key: config["adapter"][key] for key in ("id", "sha256")}
    ):
        raise ValueError("receipt witness hash link mismatch")
    _assert_safe(item)
    return item


def validate_receipt(
    record: object, external_root: str | Path | None = None
) -> dict[str, Any]:
    """Validate a product receipt and optionally verify every external hash link.

    Returns a path-free scoreboard row. Historical v1 receipts are identified
    as legacy path-bearing context, never as verified safe evidence.
    """
    data = _lib.record_from_dict(record) if isinstance(record, dict) else record
    config = data.config.resolved
    if config.get("schema_version") == competitive_run.CONFIG_SCHEMA:
        result = competitive_run.validate_result(
            config, data.metrics["competitive_result"]
        )
        return {
            "run_id": data.run_id,
            "axis": "product",
            "family": config["scoreboard"],
            "status": result["status"],
            "evidence_status": "legacy_path_bearing",
            "metrics": result["metrics"],
            "source_evidence": {
                "sha256": result["source"]["sha256"],
                "kind": "legacy_path_bearing",
            },
            "comparability": "context_only",
            "product": config["product"],
        }
    if config.get("schema_version") != CONFIG_SCHEMA:
        raise ValueError("unsupported product config schema")
    _assert_safe(record if isinstance(record, dict) else data.to_dict())
    _validate_safe_config(config)
    if data.config.sha256 != _lib.config_sha256(config):
        raise ValueError("safe config hash mismatch")
    _assert_safe(config)
    result = _validate_safe_result(config, data.metrics["competitive_result"])
    witness = _validate_witness(config, result, data.metrics["competitive_execution"])
    if external_root is not None:
        root = Path(external_root)
        config_path = root / "execution-config.json"
        result_path = root / "result.json"
        execution_path = root / "execution.json"
        if (
            competitive_runner._digest(config_path)
            != witness["execution_config_sha256"]
        ):
            raise ValueError("execution config hash mismatch")
        if competitive_runner._digest(result_path) != witness["result_sha256"]:
            raise ValueError("external result hash mismatch")
        if competitive_runner._digest(execution_path) != witness["execution_sha256"]:
            raise ValueError("execution witness hash mismatch")
        full_config = json.loads(config_path.read_text(encoding="utf-8"))
        full_result = json.loads(result_path.read_text(encoding="utf-8"))
        execution = json.loads(execution_path.read_text(encoding="utf-8"))
        if full_config["plan_sha256"] != witness["plan_sha256"]:
            raise ValueError("execution config plan hash mismatch")
        if (
            safe_projection(full_config) != config
            or _safe_result(full_result, config, witness["execution_config_sha256"])
            != result
        ):
            raise ValueError("safe projection hash mismatch")
        expected = {
            "schema_version": EXECUTION_SCHEMA,
            "attempt_id": witness["attempt_id"],
            "plan_sha256": witness["plan_sha256"],
            "config_locator": str(config_path),
            "config_sha256": witness["execution_config_sha256"],
            "safe_config_sha256": witness["config_sha256"],
            "result_locator": str(result_path),
            "result_sha256": witness["result_sha256"],
            "safe_result_sha256": witness["safe_result_sha256"],
            "source_locator": full_result["source"]["locator"],
            "source_sha256": witness["source_sha256"],
            "status": result["status"],
        }
        if execution != expected:
            raise ValueError("execution witness link mismatch")
        source_path = Path(full_result["source"]["locator"])
        if (
            not source_path.is_absolute()
            or not source_path.resolve(strict=True).is_relative_to(
                root.resolve(strict=True)
            )
            or competitive_runner._digest(source_path) != witness["source_sha256"]
        ):
            raise ValueError("source evidence hash mismatch")
    return {
        "run_id": data.run_id,
        "axis": "product",
        "family": config["scoreboard"],
        "status": result["status"],
        "evidence_status": "verified_local"
        if external_root is not None
        else "hash_linked_local",
        "metrics": result["metrics"],
        "source_evidence": {
            "sha256": result["source"]["sha256"],
            "kind": result["source"]["evidence_kind"],
        },
        "comparability": result["comparability"],
        "product": config["product"],
    }


def run_experiment(
    plan_path: str | Path,
    output_root: str | Path,
    receipts_root: str | Path = _lib.EXPERIMENTS_DIR,
    *,
    ts: datetime | None = None,
) -> tuple[str, Path]:
    """Execute a local v2 product plan and record its path-free receipt."""
    resolved = competitive_runner.preflight(plan_path, output_root)
    if resolved["schema_version"] != competitive_runner.PRODUCT_PLAN_SCHEMA:
        raise ValueError("successor product runner requires a v2 plan")
    output = Path(output_root).resolve()
    full_config = _execution_config(resolved)
    safe_config = _validate_safe_config(safe_projection(full_config))
    output.mkdir(parents=True, exist_ok=False)
    config_path = output / "execution-config.json"
    config_sha = _write_json(config_path, full_config)
    worker_path = output / "experiment.resolved.json"
    _write_json(worker_path, resolved)
    stdout = output / "stdout.log"
    stderr = output / "stderr.log"
    environment = os.environ.copy()
    environment["PYTHONPATH"] = os.pathsep.join(sys.path)
    with stdout.open("wb") as out, stderr.open("wb") as err:
        try:
            completed = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "experiments.competitive_runner",
                    "worker",
                    str(worker_path),
                    str(output),
                ],
                cwd=competitive_runner._ROOT,
                env=environment,
                stdout=out,
                stderr=err,
                timeout=resolved["adapter"]["timeout_s"],
                check=False,
            )
            exit_code = completed.returncode
            reason = "adapter_failed" if exit_code else None
        except subprocess.TimeoutExpired:
            exit_code = 124
            reason = "adapter_timed_out"
    try:
        if (
            competitive_runner._digest(Path(resolved["adapter"]["path"]))
            != resolved["adapter"]["sha256"]
        ):
            reason = "adapter_mutated"
        elif any(
            competitive_runner._digest(Path(item["path"])) != item["sha256"]
            for item in resolved["inputs"].values()
        ):
            reason = "input_mutated"
    except OSError:
        reason = "input_or_adapter_missing"
    legacy_result = None
    if reason is None:
        try:
            legacy_result = competitive_runner._verified_adapter_result(
                resolved["run"], output
            )
            if legacy_result["status"] == "complete" and set(
                legacy_result["metrics"]
            ) != set(resolved["output"]["metrics"]):
                raise ValueError("metric set drift")
            competitive_runner._verify_source(legacy_result, output)
        except (OSError, ValueError, TypeError, KeyError):
            reason = "adapter_result_invalid"
    if reason is not None:
        legacy_result = competitive_runner._blocked(resolved["run"], stderr, reason)
    assert legacy_result is not None
    evidence_kind = (
        "adapter_summary" if legacy_result["status"] == "complete" else "execution_log"
    )
    full_result = {
        **legacy_result,
        "schema_version": RESULT_SCHEMA,
        "config_sha256": config_sha,
        "source": {**legacy_result["source"], "evidence_kind": evidence_kind},
    }
    try:
        safe_result = _safe_result(full_result, safe_config, config_sha)
    except ValueError:
        legacy_result = competitive_runner._blocked(
            resolved["run"], stderr, "unsafe_adapter_output"
        )
        evidence_kind = "execution_log"
        full_result = {
            **legacy_result,
            "schema_version": RESULT_SCHEMA,
            "config_sha256": config_sha,
            "source": {**legacy_result["source"], "evidence_kind": evidence_kind},
        }
        safe_result = _safe_result(full_result, safe_config, config_sha)
    _validate_safe_result(safe_config, safe_result)
    result_path = output / "result.json"
    result_sha = _write_json(result_path, full_result)
    attempt_id = hashlib.sha256(str(output).encode()).hexdigest()[:16]
    execution = {
        "schema_version": EXECUTION_SCHEMA,
        "attempt_id": attempt_id,
        "plan_sha256": resolved["plan_sha256"],
        "config_locator": str(config_path),
        "config_sha256": config_sha,
        "safe_config_sha256": _lib.config_sha256(safe_config),
        "result_locator": str(result_path),
        "result_sha256": result_sha,
        "safe_result_sha256": _hash_obj(safe_result),
        "source_locator": full_result["source"]["locator"],
        "source_sha256": full_result["source"]["sha256"],
        "status": full_result["status"],
    }
    execution_path = output / "execution.json"
    execution_sha = _write_json(execution_path, execution)
    if (
        competitive_runner._digest(config_path) != config_sha
        or competitive_runner._digest(result_path) != result_sha
        or competitive_runner._digest(execution_path) != execution_sha
        or competitive_runner._digest(Path(full_result["source"]["locator"]))
        != full_result["source"]["sha256"]
    ):
        raise ValueError("external execution evidence hash drift before receipt")
    receipt_witness = {
        "schema_version": RECEIPT_WITNESS_SCHEMA,
        "attempt_id": attempt_id,
        "config_sha256": _lib.config_sha256(safe_config),
        "execution_config_sha256": config_sha,
        "result_sha256": result_sha,
        "safe_result_sha256": _hash_obj(safe_result),
        "execution_sha256": execution_sha,
        "source_sha256": full_result["source"]["sha256"],
        "status": full_result["status"],
        "plan_sha256": resolved["plan_sha256"],
        "adapter": {key: resolved["adapter"][key] for key in ("id", "sha256")},
    }
    now = ts or datetime.now(UTC)
    experiment = "competitive-product-" + safe_config["scoreboard"] + "-" + attempt_id
    base = Path(receipts_root)
    run_id = _lib.make_run_id(experiment, now, _lib.config_sha256(safe_config))
    if (
        run_id in _lib.index_run_ids(base / "index.jsonl")
        or (base / "runs" / run_id).exists()
    ):
        raise ValueError(f"competitive product run {run_id} already exists")
    read = f"{safe_config['case_id']}: {safe_result['status']}"
    code = {**_lib.git_info(), "baseline_commit": None}
    corpus = {
        "source": safe_config["workload"]["name"],
        "manifest_sha256": safe_config["workload"]["corpus_sha256"],
        "datasets": [safe_config["workload"]["name"]],
    }
    seeds = {"query_order": safe_config["measurement"]["seed"]}
    env = _lib.env_info()
    headline = {
        "case_id": safe_config["case_id"],
        "scoreboard": safe_config["scoreboard"],
        "axis": "product",
        "program_track": resolved["program_track"],
    }
    artifacts = [{"sha256": receipt_witness["source_sha256"], "kind": evidence_kind}]
    _assert_safe(
        {
            "run_id": run_id,
            "config": safe_config,
            "result": safe_result,
            "witness": receipt_witness,
            "read": read,
            "code": code,
            "corpus": corpus,
            "seeds": seeds,
            "env": env,
            "headline": headline,
            "artifacts": artifacts,
        }
    )
    result = _lib.write_record(
        experiment,
        ts=now,
        config_obj=safe_config,
        metrics={
            "competitive_result": safe_result,
            "competitive_execution": receipt_witness,
        },
        verdict=safe_result["status"],
        read=read,
        code=code,
        corpus=corpus,
        seeds=seeds,
        env=env,
        cost_usd=safe_result["metrics"].get("cost_usd"),
        n=safe_result["observed"]["queries_completed"]
        if safe_result["observed"]
        else 0,
        headline=headline,
        artifacts=artifacts,
        base_dir=base,
    )
    validate_receipt(json.loads((result[1] / "record.json").read_text()), output)
    return result
