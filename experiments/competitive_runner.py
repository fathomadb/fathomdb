"""Versioned, config-driven execution of competitive benchmark adapters."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import platform
import re
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from experiments import _lib, competitive_run


PLAN_SCHEMA = "competitive.experiment.v1"
PRODUCT_PLAN_SCHEMA = "competitive.experiment.v2"
EXECUTION_SCHEMA = "competitive.execution.v1"
ADAPTER_PROTOCOL = "competitive.adapter.v1"
_SHA = re.compile(r"[0-9a-f]{64}\Z")
_ADAPTER_ID = re.compile(r"[a-z][a-z0-9-]*\Z")
_ROOT = Path(__file__).resolve().parents[1]


def _digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    value: dict[str, object] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate configuration key: {key}")
        value[key] = item
    return value


def _exact(value: object, name: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise ValueError(
            f"{name} fields mismatch: missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _sha(value: object, name: str) -> str:
    if not isinstance(value, str) or _SHA.fullmatch(value) is None:
        raise ValueError(f"{name} must be a lowercase SHA-256 digest")
    return value


def load_plan(path: str | Path) -> tuple[dict[str, Any], str]:
    """Load a single versioned experiment input and its exact byte identity."""
    raw = Path(path).read_bytes()
    plan = _exact(
        json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object),
        "experiment",
        {"schema_version", "program_track", "run", "adapter", "inputs", "output"},
    )
    if plan["schema_version"] not in {PLAN_SCHEMA, PRODUCT_PLAN_SCHEMA}:
        raise ValueError("experiment schema_version mismatch")
    track = plan["program_track"]
    if track is not None and (
        not isinstance(track, str) or re.fullmatch(r"[A-Z]+-[0-9]+", track) is None
    ):
        raise ValueError("program_track must be a track ID or null")
    run = competitive_run.validate_definition(plan["run"])
    if run["evidence"]["kind"] != "local":
        raise ValueError("runner only executes local evidence")
    adapter = _exact(
        plan["adapter"], "adapter", {"protocol", "id", "path", "sha256", "timeout_s"}
    )
    if adapter["protocol"] != ADAPTER_PROTOCOL:
        raise ValueError("adapter protocol version mismatch")
    if (
        not isinstance(adapter["id"], str)
        or _ADAPTER_ID.fullmatch(adapter["id"]) is None
    ):
        raise ValueError("adapter.id must be a stable lowercase identifier")
    if not isinstance(adapter["path"], str) or not Path(adapter["path"]).is_absolute():
        raise ValueError("adapter.path must be absolute")
    _sha(adapter["sha256"], "adapter.sha256")
    if (
        isinstance(adapter["timeout_s"], bool)
        or not isinstance(adapter["timeout_s"], int)
        or adapter["timeout_s"] < 1
    ):
        raise ValueError("adapter.timeout_s must be a positive integer")
    inputs = plan["inputs"]
    required = {"corpus", "queries"}
    allowed = required | {"gold"}
    if (
        not isinstance(inputs, dict)
        or not required <= set(inputs)
        or set(inputs) - allowed
    ):
        raise ValueError("inputs require corpus and queries; gold is optional")
    if run["scoreboard"] in {"retrieval", "answer"} and "gold" not in inputs:
        raise ValueError("retrieval and answer runs require gold input")
    for name, item in inputs.items():
        binding = _exact(item, f"inputs.{name}", {"path", "sha256"})
        if (
            not isinstance(binding["path"], str)
            or not Path(binding["path"]).is_absolute()
        ):
            raise ValueError(f"inputs.{name}.path must be absolute")
        _sha(binding["sha256"], f"inputs.{name}.sha256")
    for name in required:
        if inputs[name]["sha256"] != run["workload"][f"{name}_sha256"]:
            raise ValueError(f"inputs.{name} digest differs from run workload")
    output = _exact(plan["output"], "output", {"schema_version", "metrics"})
    expected_result = (
        competitive_run.RESULT_SCHEMA
        if plan["schema_version"] == PLAN_SCHEMA
        else "competitive.result.v2"
    )
    if output["schema_version"] != expected_result:
        raise ValueError("output schema_version mismatch")
    metrics = output["metrics"]
    if (
        not isinstance(metrics, list)
        or not metrics
        or any(not isinstance(name, str) for name in metrics)
        or len(metrics) != len(set(metrics))
        or set(metrics) - competitive_run._METRICS[run["scoreboard"]]
    ):
        raise ValueError("output metric set is invalid for the scoreboard")
    return plan, hashlib.sha256(raw).hexdigest()


def preflight(plan_path: str | Path, output_root: str | Path) -> dict[str, Any]:
    """Resolve and hash every input without creating a run directory."""
    plan, plan_sha = load_plan(plan_path)
    output = Path(output_root).resolve(strict=False)
    if (
        not Path(output_root).is_absolute()
        or output.exists()
        or output.is_relative_to(_ROOT)
    ):
        raise ValueError(
            "output root must be new, absolute, and outside the repository"
        )
    adapter_path = Path(plan["adapter"]["path"]).resolve(strict=True)
    if not adapter_path.is_file() or adapter_path.suffix != ".py":
        raise ValueError("adapter.path must name a Python source file")
    actual_adapter_sha = _digest(adapter_path)
    if actual_adapter_sha != plan["adapter"]["sha256"]:
        raise ValueError("adapter digest drift")
    resolved_inputs: dict[str, dict[str, str]] = {}
    for name, binding in plan["inputs"].items():
        path = Path(binding["path"]).resolve(strict=True)
        if not path.is_file():
            raise ValueError(f"inputs.{name} must be a file")
        actual = _digest(path)
        if actual != binding["sha256"]:
            raise ValueError(f"inputs.{name} digest drift")
        if output == path:
            raise ValueError("output root must not overwrite an input")
        resolved_inputs[name] = {"path": str(path), "sha256": actual}
    return {
        "schema_version": plan["schema_version"],
        "program_track": plan["program_track"],
        "plan_sha256": plan_sha,
        "run": plan["run"],
        "adapter": {**plan["adapter"], "path": str(adapter_path)},
        "inputs": resolved_inputs,
        "output": plan["output"],
    }


def _worker(resolved_path: Path, output_root: Path) -> int:
    resolved = json.loads(resolved_path.read_text(encoding="utf-8"))
    adapter_path = Path(resolved["adapter"]["path"])
    if _digest(adapter_path) != resolved["adapter"]["sha256"]:
        raise ValueError("adapter changed after preflight")
    spec = importlib.util.spec_from_file_location(
        "fathomdb_competitive_adapter", adapter_path
    )
    if spec is None or spec.loader is None:
        raise ValueError("adapter cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    inputs = {name: Path(item["path"]) for name, item in resolved["inputs"].items()}
    for name, path in inputs.items():
        if _digest(path) != resolved["inputs"][name]["sha256"]:
            raise ValueError(f"input changed after preflight: {name}")
    result = module.run(resolved["run"], inputs, output_root)
    (output_root / "adapter-result.json").write_text(
        json.dumps(result, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    return 0


def _blocked(run: dict[str, Any], log: Path, reason: str) -> dict[str, Any]:
    return {
        "schema_version": competitive_run.RESULT_SCHEMA,
        "case_id": run["case_id"],
        "config_sha256": _lib.config_sha256(run),
        "status": "blocked",
        "observed": None,
        "metrics": {},
        "source": {"url": None, "locator": str(log), "sha256": _digest(log)},
        "comparability": "context_only",
        "reason": reason,
    }


def _verified_adapter_result(run: dict[str, Any], output: Path) -> dict[str, Any]:
    result = competitive_run.validate_result(
        run, json.loads((output / "adapter-result.json").read_text(encoding="utf-8"))
    )
    return result


def _verify_source(result: dict[str, Any], output: Path) -> None:
    source = Path(result["source"]["locator"])
    if (
        not source.is_absolute()
        or not source.resolve(strict=True).is_relative_to(output)
        or not source.is_file()
        or _digest(source) != result["source"]["sha256"]
    ):
        raise ValueError("adapter source artifact is invalid")


def run_experiment(
    plan_path: str | Path,
    output_root: str | Path,
    receipts_root: str | Path = _lib.EXPERIMENTS_DIR,
    *,
    ts: datetime | None = None,
) -> tuple[str, Path]:
    """Run one adapter attempt and write its validated result and receipt."""
    plan, _ = load_plan(plan_path)
    if plan["schema_version"] == PRODUCT_PLAN_SCHEMA:
        from experiments import competitive_product

        return competitive_product.run_experiment(
            plan_path, output_root, receipts_root, ts=ts
        )
    resolved = preflight(plan_path, output_root)
    output = Path(output_root).resolve()
    output.mkdir(parents=True, exist_ok=False)
    resolved_path = output / "experiment.resolved.json"
    resolved_path.write_text(json.dumps(resolved, indent=2) + "\n", encoding="utf-8")
    stdout = output / "stdout.log"
    stderr = output / "stderr.log"
    started = time.monotonic()
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
                    str(resolved_path),
                    str(output),
                ],
                cwd=_ROOT,
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
    elapsed = time.monotonic() - started
    try:
        if _digest(Path(resolved["adapter"]["path"])) != resolved["adapter"]["sha256"]:
            reason = "adapter_mutated"
        elif any(
            _digest(Path(binding["path"])) != binding["sha256"]
            for binding in resolved["inputs"].values()
        ):
            reason = "input_mutated"
    except OSError:
        reason = "input_or_adapter_missing"
    result: dict[str, Any] | None = None
    if reason is None:
        try:
            result = _verified_adapter_result(resolved["run"], output)
        except (OSError, ValueError, TypeError, KeyError):
            reason = "adapter_result_invalid"
        else:
            if result["status"] == "complete" and set(result["metrics"]) != set(
                resolved["output"]["metrics"]
            ):
                reason = "adapter_result_invalid"
            else:
                try:
                    _verify_source(result, output)
                except (OSError, ValueError):
                    reason = "adapter_source_invalid"
    if reason is not None:
        result = _blocked(resolved["run"], stderr, reason)
    assert result is not None
    result_path = output / "result.json"
    result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    witness = {
        "schema_version": EXECUTION_SCHEMA,
        "program_track": resolved["program_track"],
        "attempt_id": hashlib.sha256(str(output).encode()).hexdigest()[:16],
        "plan_sha256": resolved["plan_sha256"],
        "config_sha256": _lib.config_sha256(resolved["run"]),
        "runner_sha256": _digest(Path(__file__)),
        "adapter": resolved["adapter"],
        "inputs": resolved["inputs"],
        "result_locator": str(result_path),
        "result_sha256": _digest(result_path),
        "source_sha256": result["source"]["sha256"],
        "stdout_sha256": _digest(stdout),
        "stderr_sha256": _digest(stderr),
        "exit_code": exit_code,
        "elapsed_s": elapsed,
        "status": result["status"],
        "host": {
            "system": platform.system(),
            "machine": platform.machine(),
            "cores": os.cpu_count(),
        },
    }
    witness_path = output / "execution.json"
    witness_path.write_text(json.dumps(witness, indent=2) + "\n", encoding="utf-8")
    return competitive_run.record_run(
        resolved["run"],
        result,
        ts=ts or datetime.now(UTC),
        base_dir=receipts_root,
        code={**_lib.git_info(), "baseline_commit": None},
        env=_lib.env_info(),
        execution=witness,
        attempt_id=witness["attempt_id"],
    )


def main(argv: list[str] | None = None) -> int:
    """Preflight, execute, or host the fixed competitive adapter protocol."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for command in ("preflight", "run"):
        selected = sub.add_parser(command)
        selected.add_argument("config", type=Path)
        selected.add_argument("--output-root", type=Path, required=True)
        if command == "run":
            selected.add_argument(
                "--receipts-root", type=Path, default=_lib.EXPERIMENTS_DIR
            )
    worker = sub.add_parser("worker", help=argparse.SUPPRESS)
    worker.add_argument("resolved", type=Path)
    worker.add_argument("output_root", type=Path)
    args = parser.parse_args(argv)
    if args.command == "worker":
        return _worker(args.resolved, args.output_root)
    try:
        if args.command == "preflight":
            print(json.dumps(preflight(args.config, args.output_root), sort_keys=True))
        else:
            run_id, _ = run_experiment(
                args.config, args.output_root, args.receipts_root
            )
            print(run_id)
            result = json.loads(
                (args.output_root / "result.json").read_text(encoding="utf-8")
            )
            return 0 if result["status"] == "complete" else 1
    except (OSError, ValueError, TypeError, ImportError) as exc:
        parser.error(str(exc))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
