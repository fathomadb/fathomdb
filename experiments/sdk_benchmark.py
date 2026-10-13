"""Versioned coordinator for reproducible public-SDK benchmark suites."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import random
import re
import signal
import subprocess
import uuid
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from experiments import _lib, sdk_answer, sdk_registry
from experiments.fathomdb_test_setup import prepare_test_database


PLAN_SCHEMA = "sdk.experiment.v1"
RESULT_SCHEMA = "sdk.result.v1"
DRIVER_SCHEMA = "sdk.driver.v1"
_ROOT = Path(__file__).resolve().parents[1]
_SHA256 = re.compile(r"[0-9a-f]{64}\Z")
_ID = re.compile(r"[a-z][a-z0-9-]*\Z")
_FAMILIES = {"latency_capacity", "retrieval", "answer"}
_METRICS = {
    "latency_capacity": {"p50_ms", "p95_ms", "p99_ms", "queries_per_s"},
    "retrieval": {"recall_at_10", "mrr", "ndcg_at_10", "negative_abstention"},
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


def _digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _check_treatment_environment() -> None:
    overrides = sorted(
        key
        for key in os.environ
        if (key.startswith("FATHOMDB_") and key != "FATHOMDB_BENCH_NODE")
        or key
        in {
            "NODE_OPTIONS",
            "PYTHONOPTIMIZE",
            "PYTHONHASHSEED",
            "PYTHONMALLOC",
            "RAYON_NUM_THREADS",
            "OMP_NUM_THREADS",
            "OPENBLAS_NUM_THREADS",
        }
    )
    if overrides:
        raise ValueError(
            "undeclared FathomDB environment override: " + ", ".join(overrides)
        )


def _wrapper_digest(root: Path, suffixes: set[str]) -> str:
    if not root.is_dir():
        raise ValueError("SDK wrapper directory missing")
    digest = hashlib.sha256()
    paths = sorted(
        path
        for path in root.rglob("*")
        if path.is_file()
        and path.suffix in suffixes
        and not {"node_modules", "__pycache__", "target"}.intersection(path.parts)
    )
    if not paths:
        raise ValueError("SDK wrapper files missing")
    for path in paths:
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(bytes.fromhex(_digest(path)))
    return digest.hexdigest()


def wrapper_sha256(
    sdk: str, native_sha256: str = "", native_path: str | Path | None = None
) -> str:
    """Digest the public wrapper loaded by the fixed SDK driver."""
    if sdk == "python":
        package = (
            Path(native_path).resolve().parent
            if native_path is not None
            else _ROOT / "src/python/fathomdb"
        )
        return _wrapper_digest(package, {".py", ".pyi"})
    if sdk == "typescript":
        return _wrapper_digest(_ROOT / "src/ts", {".js", ".ts", ".json"})
    if sdk == "rust":
        return native_sha256
    raise ValueError("unknown SDK")


def _bootstrap_identity() -> dict[str, str]:
    from fathomdb import _fathomdb

    native = Path(_fathomdb.__file__).resolve(strict=True)
    return {
        "native_sha256": _digest(native),
        "wrapper_sha256": wrapper_sha256("python", native_path=native),
    }


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


def _binding(value: object, name: str) -> dict[str, str]:
    binding = _exact(value, name, {"path", "sha256"})
    if not isinstance(binding["path"], str) or not Path(binding["path"]).is_absolute():
        raise ValueError(f"{name}.path must be absolute")
    if not isinstance(binding["sha256"], str) or not _SHA256.fullmatch(
        binding["sha256"]
    ):
        raise ValueError(f"{name}.sha256 must be lowercase SHA-256")
    return binding


def _positive_int(value: object, name: str, *, zero: bool = False) -> int:
    minimum = 0 if zero else 1
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise ValueError(f"{name} must be an integer >= {minimum}")
    return value


def load_plan(path: str | Path) -> tuple[dict[str, Any], str]:
    """Read a closed suite definition and return its unchanged byte digest."""
    raw = Path(path).read_bytes()
    plan = _exact(
        json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object),
        "sdk experiment",
        {
            "schema_version",
            "program_track",
            "suite_id",
            "fixtures",
            "arms",
            "cases",
            "sampling",
            "output",
        },
    )
    if plan["schema_version"] != PLAN_SCHEMA:
        raise ValueError("sdk experiment schema_version mismatch")
    track = plan["program_track"]
    if track is not None and (
        not isinstance(track, str) or re.fullmatch(r"[A-Z]+-[0-9]+", track) is None
    ):
        raise ValueError("program_track must be a track ID or null")
    if not isinstance(plan["suite_id"], str) or _ID.fullmatch(plan["suite_id"]) is None:
        raise ValueError("suite_id must be a stable lowercase identifier")
    fixture_keys = (
        {"manifest", "answer_gold"}
        if any(
            isinstance(case, dict) and case.get("family") == "answer"
            for case in plan["cases"]
        )
        else {"manifest"}
    )
    fixtures = _exact(plan["fixtures"], "fixtures", fixture_keys)
    _binding(fixtures["manifest"], "fixtures.manifest")
    if "answer_gold" in fixtures:
        _binding(fixtures["answer_gold"], "fixtures.answer_gold")
    arms = plan["arms"]
    if not isinstance(arms, list) or not arms or len(arms) > 3:
        raise ValueError("arms must contain one to three SDKs")
    seen_arms: set[str] = set()
    for position, item in enumerate(arms):
        arm = _exact(item, f"arms[{position}]", {"sdk", "artifact"})
        sdk = arm["sdk"]
        if sdk not in {"python", "typescript", "rust"} or sdk in seen_arms:
            raise ValueError("arms must have distinct known SDKs")
        seen_arms.add(sdk)
        _binding(arm["artifact"], f"arms[{position}].artifact")
    cases = plan["cases"]
    if not isinstance(cases, list) or not cases:
        raise ValueError("cases must be a nonempty list")
    seen_cases: set[str] = set()
    registry = sdk_registry.load_registry()
    for position, item in enumerate(cases):
        case = _exact(
            item,
            f"cases[{position}]",
            {
                "id",
                "family",
                "operation_id",
                "operation_version",
                "required",
                "arguments",
                "setup",
                "boundary",
            },
        )
        case_id = case["id"]
        if (
            not isinstance(case_id, str)
            or _ID.fullmatch(case_id) is None
            or case_id in seen_cases
        ):
            raise ValueError("case IDs must be distinct stable lowercase identifiers")
        seen_cases.add(case_id)
        if case["family"] not in _FAMILIES:
            raise ValueError("invalid metric family")
        if not isinstance(case["required"], bool):
            raise ValueError("case.required must be boolean")
        if case["family"] == "answer":
            if (
                case["operation_id"] != "answer_pipeline.first_sentence_v1"
                or case["operation_version"] != 1
            ):
                raise ValueError("unsupported answer pipeline operation")
            if (
                case["setup"] != "bound-retrieval-v1"
                or case["boundary"] != "answer_and_judge"
            ):
                raise ValueError("unsupported answer setup or boundary")
            answer_args = _exact(
                case["arguments"], "answer arguments", {"answer_config"}
            )
            sdk_answer.validate_answer_config(answer_args["answer_config"])
            upstream = answer_args["answer_config"]["upstream"]["case_id"]
            if (
                upstream not in seen_cases - {case_id}
                or next(
                    previous
                    for previous in cases[:position]
                    if previous["id"] == upstream
                )["family"]
                != "retrieval"
            ):
                raise ValueError("answer upstream must be a preceding retrieval case")
            if (
                answer_args["answer_config"]["gold_sha256"]
                != fixtures["answer_gold"]["sha256"]
            ):
                raise ValueError("answer gold digest mismatch")
            continue
        descriptor = registry.get(case["operation_id"])
        if (
            descriptor is None
            or descriptor["disposition"] != "measure"
            or case["operation_version"] != descriptor["semantic_version"]
        ):
            raise ValueError("unsupported operation or version")
        if not isinstance(case["required"], bool):
            raise ValueError("case.required must be boolean")
        if (
            case["setup"] != "canonical-nodes-v1"
            or case["boundary"] != "sdk_call_to_materialized_result"
        ):
            raise ValueError("unsupported setup or timing boundary")
        operation_id = case["operation_id"]
        keys = (
            {"query_set", "limit", "view"}
            if operation_id == "engine.search_text_only"
            else {"view"}
            if operation_id == "read.get"
            else {"depth", "view"}
        )
        arguments = _exact(case["arguments"], "case.arguments", keys)
        if operation_id == "engine.search_text_only":
            if arguments["query_set"] != "all":
                raise ValueError("unsupported query_set")
            _positive_int(arguments["limit"], "arguments.limit")
        if operation_id == "graph.neighbors":
            depth = _positive_int(arguments["depth"], "arguments.depth")
            if depth > 3:
                raise ValueError("graph depth exceeds registered bound")
        view = _exact(arguments["view"], "arguments.view", {"valid_as_of"})
        _positive_int(view["valid_as_of"], "arguments.view.valid_as_of", zero=True)
    sampling = _exact(
        plan["sampling"],
        "sampling",
        {
            "warmup_per_query",
            "repetitions",
            "seed",
            "cache_state",
            "concurrency",
            "order",
        },
    )
    for name in ("warmup_per_query", "seed"):
        _positive_int(sampling[name], f"sampling.{name}", zero=True)
    _positive_int(sampling["repetitions"], "sampling.repetitions")
    if sampling["repetitions"] != 1 and any(
        case["family"] == "answer" for case in cases
    ):
        raise ValueError("answer cases require exactly one repetition")
    if sampling["concurrency"] != 1 or sampling["order"] != "balanced":
        raise ValueError("only balanced serial sampling is implemented")
    if sampling["cache_state"] not in {
        "steady_after_declared_warmup",
        "first_pass_after_populated_reopen",
    }:
        raise ValueError("unsupported cache_state")
    if (
        sampling["cache_state"] == "first_pass_after_populated_reopen"
        and sampling["warmup_per_query"]
    ):
        raise ValueError("first-pass cache state cannot have query warmup")
    output = _exact(plan["output"], "output", {"result_schema", "metrics"})
    if output["result_schema"] != RESULT_SCHEMA:
        raise ValueError("sdk result schema mismatch")
    metrics = output["metrics"]
    if (
        not isinstance(metrics, list)
        or not metrics
        or any(not isinstance(name, str) for name in metrics)
    ):
        raise ValueError("output.metrics must be a nonempty list of names")
    if len(metrics) != len(set(metrics)):
        raise ValueError("duplicate output metric")
    allowed = set.union(*(_METRICS[case["family"]] for case in cases))
    if set(metrics) - allowed:
        raise ValueError("output metric is invalid for the selected families")
    return plan, hashlib.sha256(raw).hexdigest()


def preflight(plan_path: str | Path, output_root: str | Path) -> dict[str, Any]:
    """Validate pinned local artifacts without creating a run directory."""
    _check_treatment_environment()
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
    bindings = list(plan["fixtures"].values()) + [
        arm["artifact"] for arm in plan["arms"]
    ]
    for binding in bindings:
        path = Path(binding["path"]).resolve(strict=True)
        if not path.is_file() or _digest(path) != binding["sha256"]:
            raise ValueError("input or SDK artifact digest drift")
    return {"schema_version": PLAN_SCHEMA, "plan_sha256": plan_sha, "plan": plan}


def validate_experiment(
    plan_path: str | Path, output_root: str | Path
) -> dict[str, Any]:
    """Resolve every pinned input without creating an output or receipt."""
    resolved = preflight(plan_path, output_root)
    plan = resolved["plan"]
    _load_fixture(plan["fixtures"]["manifest"])
    for item in plan["arms"]:
        _load_arm(item["sdk"], item["artifact"])
    return resolved


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(
        path.read_text(encoding="utf-8"), object_pairs_hook=_unique_object
    )
    if not isinstance(value, dict):
        raise ValueError(f"{path.name} must contain a JSON object")
    return value


def _write_json(path: Path, value: dict[str, Any]) -> str:
    path.write_text(
        json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return _digest(path)


def _verified_binding(value: object, name: str) -> Path:
    binding = _binding(value, name)
    path = Path(binding["path"]).resolve(strict=True)
    if not path.is_file() or _digest(path) != binding["sha256"]:
        raise ValueError(f"{name} digest drift")
    return path


def _verify_pinned(
    plan: dict[str, Any], arm_binding: dict[str, Any], arm: dict[str, Any]
) -> None:
    _verified_binding(plan["fixtures"]["manifest"], "fixture")
    if "answer_gold" in plan["fixtures"]:
        _verified_binding(plan["fixtures"]["answer_gold"], "answer gold")
    _verified_binding(arm_binding["artifact"], "arm manifest")
    for name in ("runtime", "driver", "native", "bootstrap_cli"):
        _verified_binding(arm[name], f"arm.{name}")
    if (
        wrapper_sha256(arm["sdk"], arm["native"]["sha256"], arm["native"]["path"])
        != arm["wrapper_sha256"]
    ):
        raise ValueError("SDK public wrapper digest drift")


def _load_fixture(binding: dict[str, str]) -> dict[str, Any]:
    fixture = _read_json(_verified_binding(binding, "fixture"))
    _exact(
        fixture,
        "fixture",
        {"schema_version", "corpus", "queries", "gold"}
        | ({"oracles"} if "oracles" in fixture else set()),
    )
    if fixture["schema_version"] != "sdk.fixture.v1":
        raise ValueError("fixture version mismatch")
    corpus, queries, gold = fixture["corpus"], fixture["queries"], fixture["gold"]
    if (
        not isinstance(corpus, list)
        or not corpus
        or not isinstance(queries, list)
        or not queries
        or not isinstance(gold, dict)
    ):
        raise ValueError("fixture corpus, queries, or gold shape invalid")
    seen_docs: set[str] = set()
    for row in corpus:
        item = _exact(row, "corpus row", {"id", "body"})
        if (
            not isinstance(item["id"], str)
            or not item["id"]
            or item["id"] in seen_docs
            or not isinstance(item["body"], str)
        ):
            raise ValueError("invalid corpus row")
        seen_docs.add(item["id"])
    seen_queries: set[str] = set()
    for row in queries:
        item = _exact(row, "query row", {"id", "text"})
        if (
            not isinstance(item["id"], str)
            or not item["id"]
            or item["id"] in seen_queries
            or not isinstance(item["text"], str)
            or not item["text"]
        ):
            raise ValueError("invalid query row")
        seen_queries.add(item["id"])
    if set(gold) != seen_queries or any(
        not isinstance(ids, list)
        or any(not isinstance(item, str) or not item for item in ids)
        for ids in gold.values()
    ):
        raise ValueError("gold must map every query ID to relevance IDs")
    if "oracles" in fixture:
        oracles = fixture["oracles"]
        if not isinstance(oracles, dict) or any(
            not isinstance(operation, str)
            or not isinstance(by_query, dict)
            or set(by_query) != seen_queries
            or any(
                not isinstance(ids, list)
                or any(not isinstance(identifier, str) for identifier in ids)
                for ids in by_query.values()
            )
            for operation, by_query in oracles.items()
        ):
            raise ValueError(
                "fixture oracles must map operation and every query to IDs"
            )
    return fixture


def _check_oracle(
    case: dict[str, Any], fixture: dict[str, Any], ordered_ids: dict[str, list[str]]
) -> str:
    """Verify binding correctness independently of retrieval relevance gold."""
    expected = fixture.get("oracles", {}).get(case["operation_id"])
    if expected is None:
        if case["family"] == "latency_capacity":
            raise ValueError("latency case requires an exact operation oracle")
        return "unverified"
    if ordered_ids != expected:
        raise ValueError("driver output differs from operation oracle")
    return "verified"


def _publishable_metrics(
    case: dict[str, Any], metrics: dict[str, float], oracle_status: str
) -> dict[str, float]:
    if case["family"] == "retrieval" and oracle_status != "verified":
        return {
            key: value
            for key, value in metrics.items()
            if key not in {"p50_ms", "p95_ms", "p99_ms", "queries_per_s"}
        }
    return metrics


def _load_arm(sdk: str, binding: dict[str, str]) -> dict[str, Any]:
    arm = _exact(
        _read_json(_verified_binding(binding, f"{sdk} arm")),
        f"{sdk} arm",
        {
            "schema_version",
            "sdk",
            "sdk_version",
            "engine_revision",
            "dependency_closure_sha256",
            "wrapper_sha256",
            "features",
            "device",
            "runtime",
            "driver",
            "native",
            "bootstrap_cli",
        },
    )
    if arm["schema_version"] != "sdk.arm.v1" or arm["sdk"] != sdk:
        raise ValueError("arm schema or SDK mismatch")
    for name in ("sdk_version", "engine_revision"):
        if not isinstance(arm[name], str) or not arm[name]:
            raise ValueError(f"arm.{name} must be nonempty")
    if (
        not isinstance(arm["dependency_closure_sha256"], str)
        or _SHA256.fullmatch(arm["dependency_closure_sha256"]) is None
    ):
        raise ValueError("arm dependency closure digest invalid")
    if (
        not isinstance(arm["wrapper_sha256"], str)
        or _SHA256.fullmatch(arm["wrapper_sha256"]) is None
    ):
        raise ValueError("arm wrapper digest invalid")
    if not isinstance(arm["features"], list) or any(
        not isinstance(item, str) for item in arm["features"]
    ):
        raise ValueError("arm features invalid")
    if arm["device"] != "cpu":
        raise ValueError("only explicit CPU synthetic SDK arms are implemented")
    for name in ("runtime", "driver", "native", "bootstrap_cli"):
        _verified_binding(arm[name], f"arm.{name}")
    if (
        wrapper_sha256(sdk, arm["native"]["sha256"], arm["native"]["path"])
        != arm["wrapper_sha256"]
    ):
        raise ValueError("arm public wrapper digest drift")
    return arm


def _driver_command(arm: dict[str, Any], request_path: Path) -> list[str]:
    sdk = arm["sdk"]
    runtime = arm["runtime"]["path"]
    driver = arm["driver"]["path"]
    if sdk == "python":
        if (
            Path(driver).resolve()
            != (_ROOT / "experiments/sdk_drivers/python_driver.py").resolve()
        ):
            raise ValueError("Python driver is not the fixed repository module")
        return [
            runtime,
            "-m",
            "experiments.sdk_drivers.python_driver",
            str(request_path),
        ]
    if sdk == "typescript":
        if (
            Path(driver).resolve()
            != (_ROOT / "experiments/sdk_drivers/typescript_driver.mjs").resolve()
        ):
            raise ValueError("TypeScript driver is not the fixed repository module")
        return [runtime, driver, str(request_path)]
    if Path(driver).resolve() != Path(arm["native"]["path"]).resolve():
        raise ValueError("Rust driver binary must be its attested native artifact")
    return [driver, str(request_path)]


def _invoke_driver(
    arm: dict[str, Any], request: dict[str, Any], root: Path, timeout_s: int
) -> dict[str, Any]:
    request_path = root / "request.json"
    _write_json(request_path, request)
    for name in ("runtime", "driver", "native"):
        _verified_binding(arm[name], f"arm.{name}")
    command = _driver_command(arm, request_path)
    environment = dict(os.environ)
    module_roots = [str(_ROOT)]
    if arm["sdk"] == "python":
        module_roots.append(str(Path(arm["native"]["path"]).resolve().parent.parent))
    environment["PYTHONPATH"] = os.pathsep.join(module_roots)
    process = subprocess.Popen(
        command,
        cwd=_ROOT,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        start_new_session=True,
    )
    try:
        stdout, stderr = process.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired as exc:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.communicate()
        raise TimeoutError("SDK driver timed out") from exc
    (root / "stderr.log").write_text(stderr, encoding="utf-8")
    if process.returncode != 0 or not stdout.strip():
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        raise ValueError("SDK driver failed or returned no response")
    try:
        os.killpg(process.pid, 0)
    except ProcessLookupError:
        pass
    else:
        os.killpg(process.pid, signal.SIGKILL)
        raise ValueError("SDK driver descendants survived process exit")
    response = json.loads(stdout, object_pairs_hook=_unique_object)
    if (
        not isinstance(response, dict)
        or response.get("schema_version") != DRIVER_SCHEMA
        or response.get("sdk") != arm["sdk"]
    ):
        raise ValueError("SDK driver protocol mismatch")
    return response


def _prepare_repetition(
    root: Path, arm: dict[str, Any], fixture: dict[str, Any], repetition: int
) -> tuple[Path, dict[str, Any]]:
    prepared = prepare_test_database(
        root,
        test_id=f"rep-{repetition}",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=arm["bootstrap_cli"]["path"],
    )
    from fathomdb import Engine

    engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
    try:
        rows = [
            {
                "kind": "doc",
                "body": row["body"],
                "logical_id": row["id"],
                "source_id": f"sdk:{row['id']}",
            }
            for row in fixture["corpus"]
        ]
        engine.write(rows)
        engine.drain(timeout_s=30)
    finally:
        engine.close()
    return prepared.database_path, {
        "setup_sha256": _digest(prepared.config_path),
        "doctor_sha256": _digest(prepared.doctor_path),
        "database_sha256": _digest(prepared.database_path),
        "bootstrap": _bootstrap_identity(),
    }


def _nearest(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def _aggregate(
    case: dict[str, Any], samples: list[dict[str, Any]], fixture: dict[str, Any]
) -> tuple[dict[str, float], dict[str, list[str]]]:
    queries = [row["id"] for row in fixture["queries"]]
    expected_count = len(queries)
    if not samples or len(samples) % expected_count:
        raise ValueError("driver sample count differs from query set")
    ordered_ids: dict[str, list[str]] = {}
    times: list[float] = []
    expected_order = queries * (len(samples) // expected_count)
    if [sample.get("query_id") for sample in samples] != expected_order:
        raise ValueError("duplicate, missing, or out-of-order query in repetition")
    for sample in samples:
        item = _exact(sample, "driver sample", {"query_id", "elapsed_ns", "ids"})
        query_id = item["query_id"]
        if (
            query_id not in queries
            or not isinstance(item["elapsed_ns"], int)
            or item["elapsed_ns"] < 0
        ):
            raise ValueError("invalid driver sample")
        ids = item["ids"]
        if not isinstance(ids, list) or any(
            not isinstance(value, str) for value in ids
        ):
            raise ValueError("invalid driver IDs")
        if len(ids) != len(set(ids)):
            raise ValueError("duplicate result ID in a sample")
        if (
            case["operation_id"] == "engine.search_text_only"
            and len(ids) > case["arguments"]["limit"]
        ):
            raise ValueError("result count exceeds declared limit")
        if query_id in ordered_ids and ordered_ids[query_id] != ids:
            raise ValueError("inconsistent ordered IDs across repetitions")
        ordered_ids[query_id] = ids
        times.append(item["elapsed_ns"] / 1_000_000)
    if set(ordered_ids) != set(queries):
        raise ValueError("driver omitted a query")
    metrics: dict[str, float] = {"p50_ms": _nearest(times, 0.5)}
    if len(times) >= 20:
        metrics["p95_ms"] = _nearest(times, 0.95)
    if len(times) >= 100:
        metrics["p99_ms"] = _nearest(times, 0.99)
    if case["family"] == "latency_capacity" and sum(times) > 0:
        metrics["queries_per_s"] = 1000.0 * len(times) / sum(times)
    if case["family"] == "retrieval":
        found = 0
        total = 0
        reciprocal_ranks: list[float] = []
        discounted_gains: list[float] = []
        negative_count = 0
        abstained_count = 0
        for query_id, ids in ordered_ids.items():
            gold = {f"l:{item}" for item in fixture["gold"][query_id]}
            if not gold:
                negative_count += 1
                abstained_count += not ids
                continue
            found += len(set(ids[:10]) & gold)
            total += len(gold)
            rank = next(
                (index + 1 for index, item in enumerate(ids) if item in gold), None
            )
            reciprocal_ranks.append(1.0 / rank if rank is not None else 0.0)
            gain = sum(
                1.0 / math.log2(index + 2)
                for index, item in enumerate(ids[:10])
                if item in gold
            )
            ideal = sum(
                1.0 / math.log2(index + 2) for index in range(min(10, len(gold)))
            )
            discounted_gains.append(gain / ideal)
        if total:
            metrics["recall_at_10"] = found / total
            metrics["mrr"] = sum(reciprocal_ranks) / len(reciprocal_ranks)
            metrics["ndcg_at_10"] = sum(discounted_gains) / len(discounted_gains)
        if negative_count:
            metrics["negative_abstention"] = abstained_count / negative_count
    return metrics, ordered_ids


def _write_retrieval_evidence(
    cell_root: Path,
    case: dict[str, Any],
    sdk: str,
    suite_attempt_id: str,
    ordered_ids: dict[str, list[str]],
    fixture: dict[str, Any],
) -> str:
    corpus = {f"l:{row['id']}": row for row in fixture["corpus"]}
    questions = []
    for query in fixture["queries"]:
        hits = []
        for identifier in ordered_ids[query["id"]]:
            if identifier not in corpus:
                raise ValueError("retrieval ID has no canonical fixture source")
            row = corpus[identifier]
            hits.append(
                {"id": identifier, "source_id": f"sdk:{row['id']}", "text": row["body"]}
            )
        questions.append({"id": query["id"], "hits": hits})
    _write_json(
        cell_root / "evidence.json",
        {
            "schema_version": "sdk.retrieval-evidence.v1",
            "suite_attempt_id": suite_attempt_id,
            "case_id": case["id"],
            "sdk": sdk,
            "questions": questions,
        },
    )
    return _digest(cell_root / "evidence.json")


def _begin_cell(
    *,
    plan: dict[str, Any],
    plan_sha: str,
    case: dict[str, Any],
    arm_binding: dict[str, Any],
    arm: dict[str, Any],
    fixture: dict[str, Any],
    output: Path,
    receipts_root: Path,
    suite_attempt_id: str,
    ts: datetime,
) -> dict[str, Any]:
    cell_root = output / f"{case['id']}-{arm['sdk']}"
    cell_root.mkdir(mode=0o700)
    state = {
        "plan": plan,
        "plan_sha": plan_sha,
        "case": case,
        "arm_binding": arm_binding,
        "arm": arm,
        "fixture": fixture,
        "output": output,
        "receipts_root": receipts_root,
        "suite_attempt_id": suite_attempt_id,
        "ts": ts,
        "cell_root": cell_root,
        "all_samples": [],
        "setup_witnesses": [],
        "repetitions": [],
        "status": "complete",
        "reason": None,
    }
    capability_root = cell_root / "capabilities"
    capability_root.mkdir(mode=0o700)
    try:
        capabilities = _invoke_driver(
            arm,
            {"schema_version": DRIVER_SCHEMA, "mode": "capabilities"},
            capability_root,
            10,
        )
        offered = capabilities.get("operations")
        if capabilities.get("status") != "complete" or not isinstance(offered, list):
            raise ValueError("SDK capability negotiation failed")
        requested = {
            "operation_id": case["operation_id"],
            "operation_version": case["operation_version"],
        }
        if requested not in offered:
            state["status"], state["reason"] = "blocked", "unsupported_operation"
    except Exception:
        state["status"], state["reason"] = "blocked", "capability_preflight_failed"
    return state


def _run_cell_repetition(state: dict[str, Any], repetition: int) -> None:
    if state["status"] != "complete":
        return
    plan, case, arm = state["plan"], state["case"], state["arm"]
    cell_root, fixture = state["cell_root"], state["fixture"]
    rep_root = cell_root / f"run-{repetition}"
    rep_root.mkdir(mode=0o700)
    try:
        _verify_pinned(plan, state["arm_binding"], arm)
        database_path, setup_witness = _prepare_repetition(
            cell_root, arm, fixture, repetition
        )
        state["setup_witnesses"].append(setup_witness)
        request = {
            "schema_version": DRIVER_SCHEMA,
            "mode": "run-repetition",
            "sdk": arm["sdk"],
            "operation_id": case["operation_id"],
            "operation_version": case["operation_version"],
            "database_path": str(database_path),
            "queries": fixture["queries"],
            "arguments": {
                key: value
                for key, value in case["arguments"].items()
                if key != "query_set"
            },
            "warmup_per_query": plan["sampling"]["warmup_per_query"],
            "timeout_ms": 30000,
            "expected_native_sha256": arm["native"]["sha256"],
        }
        response = _invoke_driver(arm, request, rep_root, 35)
        _write_json(rep_root / "response.json", response)
        if response.get("status") != "complete":
            state["status"] = "blocked"
            state["reason"] = str(
                response.get("error", {}).get("kind")
                or response.get("error", {}).get("code")
                or "driver_blocked"
            )
            return
        if (
            response.get("operation_id") != case["operation_id"]
            or response.get("operation_version") != case["operation_version"]
        ):
            raise ValueError("driver operation identity mismatch")
        identity = response.get("identity")
        if (
            not isinstance(identity, dict)
            or identity.get("native_sha256") != arm["native"]["sha256"]
            or identity.get("sdk_version") != arm["sdk_version"]
        ):
            raise ValueError("driver native or SDK identity mismatch")
        effective = response.get("effective_options")
        requested_options = {
            key: value for key, value in case["arguments"].items() if key != "query_set"
        }
        if not isinstance(effective, dict) or any(
            effective.get(key) != value for key, value in requested_options.items()
        ):
            raise ValueError("driver effective options mismatch")
        samples = response.get("samples")
        if not isinstance(samples, list) or len(samples) != len(fixture["queries"]):
            raise ValueError("driver sample count mismatch")
        state["repetitions"].append(
            {
                "identity": identity,
                "effective_options": effective,
                "response_sha256": _digest(rep_root / "response.json"),
            }
        )
        state["all_samples"].extend(samples)
        _verify_pinned(plan, state["arm_binding"], arm)
    except Exception as exc:
        state["status"], state["reason"] = "blocked", type(exc).__name__
        (rep_root / "error.log").write_text(str(exc), encoding="utf-8")


def _finish_cell(state: dict[str, Any]) -> dict[str, Any]:
    plan, case, arm = state["plan"], state["case"], state["arm"]
    cell_root, fixture = state["cell_root"], state["fixture"]
    if state["status"] == "complete":
        try:
            _verify_pinned(plan, state["arm_binding"], arm)
        except (OSError, ValueError):
            state["status"], state["reason"] = "blocked", "artifact_digest_drift"
    metrics: dict[str, float] = {}
    ordered_ids: dict[str, list[str]] = {}
    evidence_sha256: str | None = None
    oracle_status = "unverified"
    if state["status"] == "complete":
        try:
            metrics, ordered_ids = _aggregate(case, state["all_samples"], fixture)
            oracle_status = _check_oracle(case, fixture, ordered_ids)
            metrics = _publishable_metrics(case, metrics, oracle_status)
            required_metrics = set(plan["output"]["metrics"]) & _METRICS[case["family"]]
            if required_metrics - set(metrics):
                raise ValueError("required metric unavailable at sample count")
            if case["family"] == "retrieval":
                evidence_sha256 = _write_retrieval_evidence(
                    cell_root,
                    case,
                    arm["sdk"],
                    state["suite_attempt_id"],
                    ordered_ids,
                    fixture,
                )
        except ValueError as exc:
            state["status"], state["reason"] = "blocked", "invalid_samples"
            (cell_root / "aggregate-error.log").write_text(str(exc), encoding="utf-8")
            metrics, ordered_ids = {}, {}
    result = {
        "schema_version": RESULT_SCHEMA,
        "suite_id": plan["suite_id"],
        "suite_attempt_id": state["suite_attempt_id"],
        "case_id": case["id"],
        "sdk": arm["sdk"],
        "axis": "sdk",
        "family": case["family"],
        "producer": "driver",
        "operation_id": case["operation_id"],
        "operation_version": case["operation_version"],
        "status": state["status"],
        "reason": state["reason"],
        "metrics": metrics,
        "ordered_ids": ordered_ids,
        "oracle_status": oracle_status,
        "source_provenance": "unverified",
        "samples": state["all_samples"] if state["status"] == "complete" else [],
        "repetitions": state["repetitions"],
        "setup_witnesses": state["setup_witnesses"],
    }
    if case["family"] == "retrieval" and state["status"] == "complete":
        result["evidence_sha256"] = evidence_sha256
    return _persist_cell(
        plan=plan,
        plan_sha=state["plan_sha"],
        case=case,
        arm_binding=state["arm_binding"],
        arm=arm,
        output=state["output"],
        receipts_root=state["receipts_root"],
        suite_attempt_id=state["suite_attempt_id"],
        ts=state["ts"],
        result=result,
    )


def _cell(
    *,
    plan: dict[str, Any],
    plan_sha: str,
    case: dict[str, Any],
    arm_binding: dict[str, Any],
    arm: dict[str, Any],
    fixture: dict[str, Any],
    output: Path,
    receipts_root: Path,
    suite_attempt_id: str,
    ts: datetime,
) -> dict[str, Any]:
    """Run a standalone cell while retaining the suite's single-cell API."""
    state = _begin_cell(
        plan=plan,
        plan_sha=plan_sha,
        case=case,
        arm_binding=arm_binding,
        arm=arm,
        fixture=fixture,
        output=output,
        receipts_root=receipts_root,
        suite_attempt_id=suite_attempt_id,
        ts=ts,
    )
    for repetition in range(plan["sampling"]["repetitions"]):
        _run_cell_repetition(state, repetition)
    return _finish_cell(state)


def _persist_cell(
    *,
    plan: dict[str, Any],
    plan_sha: str,
    case: dict[str, Any],
    arm_binding: dict[str, Any],
    arm: dict[str, Any],
    output: Path,
    receipts_root: Path,
    suite_attempt_id: str,
    ts: datetime,
    result: dict[str, Any],
) -> dict[str, Any]:
    sdk = arm["sdk"]
    cell_id = f"{case['id']}-{sdk}"
    cell_root = output / cell_id
    status = result["status"]
    metrics = result["metrics"]
    result_sha = _write_json(cell_root / "result.json", result)
    resolved_sampling = {**plan["sampling"], "timeout_ms": 30000}
    safe_config = {
        "schema_version": "sdk.receipt-config.v1",
        "program_track": plan["program_track"],
        "suite_id": plan["suite_id"],
        "suite_config_sha256": plan_sha,
        "case_id": case["id"],
        "family": case["family"],
        "operation_id": case["operation_id"],
        "operation_version": case["operation_version"],
        "sdk": sdk,
        "arguments": _safe_arguments(case),
        "sampling": resolved_sampling,
        "fixture_sha256": plan["fixtures"]["manifest"]["sha256"],
        "arm_manifest_sha256": arm_binding["artifact"]["sha256"],
        "engine_revision": arm["engine_revision"],
        "dependency_closure_sha256": arm["dependency_closure_sha256"],
        "native_sha256": arm["native"]["sha256"],
        "driver_sha256": arm["driver"]["sha256"],
    }
    execution_config = {
        "schema_version": "sdk.execution-config.v1",
        "plan": plan,
        "arm": arm,
        "case": case,
        "sampling": resolved_sampling,
    }
    execution_config_sha = _write_json(
        cell_root / "execution-config.json", execution_config
    )
    witness = {
        "schema_version": "sdk.execution.v1",
        "suite_attempt_id": suite_attempt_id,
        "case_id": case["id"],
        "sdk": sdk,
        "execution_config_sha256": execution_config_sha,
        "receipt_config_sha256": _lib.config_sha256(safe_config),
        "result_sha256": result_sha,
        "status": status,
        "result_locator": str(cell_root / "result.json"),
        "host": {
            "system": platform.system(),
            "machine": platform.machine(),
            "cores": os.cpu_count(),
        },
    }
    witness_sha = _write_json(cell_root / "execution.json", witness)
    experiment = f"sdk-{plan['suite_id']}-{case['id']}-{sdk}-{suite_attempt_id}"
    run_id = _lib.make_run_id(experiment, ts, _lib.config_sha256(safe_config))
    run_dir = receipts_root / "runs" / run_id
    if run_id in _lib.index_run_ids(receipts_root / "index.jsonl"):
        raise ValueError("cell run ID already exists in the index")
    run_dir.mkdir(parents=True, exist_ok=False)
    written_id, _ = _lib.write_record(
        experiment,
        ts=ts,
        config_obj=safe_config,
        metrics={
            **metrics,
            "result_sha256": result_sha,
            "execution_witness_sha256": witness_sha,
            "status": status,
        },
        verdict=status,
        read="sdk benchmark cell",
        code={**_lib.git_info(), "baseline_commit": None},
        corpus={
            "source": None,
            "manifest_sha256": plan["fixtures"]["manifest"]["sha256"],
            "datasets": [],
        },
        seeds={"sampling": plan["sampling"]["seed"]},
        env=_lib.env_info(key_deps={"sdk": sdk, "sdk_version": arm["sdk_version"]}),
        cost_usd=None,
        title=f"SDK {case['id']} {sdk}",
        headline=metrics,
        n=(
            result.get("sample_count", len(result.get("samples", [])))
            if status == "complete"
            else 0
        ),
        artifacts=[
            {"id": f"{cell_id}:result", "sha256": result_sha},
            {"id": f"{cell_id}:execution", "sha256": witness_sha},
        ],
        base_dir=receipts_root,
    )
    if written_id != run_id:
        raise ValueError("receipt run ID mismatch")
    return {
        "case_id": case["id"],
        "sdk": sdk,
        "run_id": run_id,
        "status": status,
        "result_sha256": result_sha,
        "witness_sha256": witness_sha,
    }


def _safe_arguments(case: dict[str, Any]) -> dict[str, Any]:
    if case["family"] != "answer":
        return case["arguments"]
    config = case["arguments"]["answer_config"]
    return {
        "answer_config_sha256": sdk_answer.validate_answer_config(config),
        "gold_sha256": config["gold_sha256"],
        "upstream_case_id": config["upstream"]["case_id"],
    }


def _answer_cell(
    *,
    plan: dict[str, Any],
    plan_sha: str,
    case: dict[str, Any],
    arm_binding: dict[str, Any],
    arm: dict[str, Any],
    output: Path,
    receipts_root: Path,
    suite_attempt_id: str,
    ts: datetime,
) -> dict[str, Any]:
    sdk = arm["sdk"]
    cell_root = output / f"{case['id']}-{sdk}"
    cell_root.mkdir(mode=0o700)
    config = case["arguments"]["answer_config"]
    try:
        _verify_pinned(plan, arm_binding, arm)
        upstream_id = config["upstream"]["case_id"]
        upstream_root = output / f"{upstream_id}-{sdk}"
        retrieval_bytes = (upstream_root / "result.json").read_bytes()
        evidence_bytes = (upstream_root / "evidence.json").read_bytes()
        retrieval_result = json.loads(retrieval_bytes, object_pairs_hook=_unique_object)
        evidence_sha256 = hashlib.sha256(evidence_bytes).hexdigest()
        if (
            not isinstance(retrieval_result, dict)
            or retrieval_result.get("evidence_sha256") != evidence_sha256
        ):
            raise ValueError("retrieval evidence digest mismatch")
        gold_bytes = _verified_binding(
            plan["fixtures"]["answer_gold"], "answer gold"
        ).read_bytes()
        binding = {
            "suite_attempt_id": suite_attempt_id,
            "case_id": upstream_id,
            "sdk": sdk,
            "retrieval_result_sha256": hashlib.sha256(retrieval_bytes).hexdigest(),
            "evidence_sha256": evidence_sha256,
        }
        answer = sdk_answer.run_answer_case(
            config=config,
            gold_bytes=gold_bytes,
            retrieval_result_bytes=retrieval_bytes,
            evidence_bytes=evidence_bytes,
            binding=binding,
            suite_id=plan["suite_id"],
            suite_attempt_id=suite_attempt_id,
            sdk=sdk,
            case_id=case["id"],
        )
        result = answer["result"]
        _write_json(cell_root / "answer-samples.json", answer["external_samples"])
        _write_json(cell_root / "answer-witness.json", answer["witness"])
        required_metrics = set(plan["output"]["metrics"]) & _METRICS["answer"]
        if required_metrics - set(result["metrics"]):
            raise ValueError("required answer metric unavailable")
        _verify_pinned(plan, arm_binding, arm)
    except Exception as exc:
        (cell_root / "error.log").write_text(str(exc), encoding="utf-8")
        result = {
            "schema_version": RESULT_SCHEMA,
            "suite_id": plan["suite_id"],
            "suite_attempt_id": suite_attempt_id,
            "case_id": case["id"],
            "sdk": sdk,
            "axis": "sdk",
            "family": "answer",
            "producer": "answer_pipeline",
            "operation_id": case["operation_id"],
            "operation_version": case["operation_version"],
            "status": "blocked",
            "reason": type(exc).__name__,
            "metrics": {},
            "sample_count": 0,
        }
    return _persist_cell(
        plan=plan,
        plan_sha=plan_sha,
        case=case,
        arm_binding=arm_binding,
        arm=arm,
        output=output,
        receipts_root=receipts_root,
        suite_attempt_id=suite_attempt_id,
        ts=ts,
        result=result,
    )


def run_experiment(
    plan_path: str | Path,
    output_root: str | Path,
    receipts_root: str | Path = _lib.EXPERIMENTS_DIR,
    *,
    ts: datetime | None = None,
) -> dict[str, Any]:
    """Execute an immutable SDK suite into fresh external artifacts and safe receipts."""
    resolved = validate_experiment(plan_path, output_root)
    plan = resolved["plan"]
    fixture = _load_fixture(plan["fixtures"]["manifest"])
    arms = {
        item["sdk"]: _load_arm(item["sdk"], item["artifact"]) for item in plan["arms"]
    }
    output = Path(output_root).resolve()
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    receipts = Path(receipts_root)
    suite_attempt_id = uuid.uuid4().hex
    now = ts or datetime.now(UTC)
    manifest = {
        "schema_version": "sdk.suite-manifest.v1",
        "suite_id": plan["suite_id"],
        "suite_attempt_id": suite_attempt_id,
        "suite_config_sha256": resolved["plan_sha256"],
        "expected_cells": [
            f"{case['id']}-{arm['sdk']}"
            for case in plan["cases"]
            for arm in plan["arms"]
        ],
        "cells": [],
    }
    for case in plan["cases"]:
        if case["family"] == "answer":
            for arm_binding in plan["arms"]:
                manifest["cells"].append(
                    _answer_cell(
                        plan=plan,
                        plan_sha=resolved["plan_sha256"],
                        case=case,
                        arm_binding=arm_binding,
                        arm=arms[arm_binding["sdk"]],
                        output=output,
                        receipts_root=receipts,
                        suite_attempt_id=suite_attempt_id,
                        ts=now,
                    )
                )
            continue
        states = {
            arm_binding["sdk"]: _begin_cell(
                plan=plan,
                plan_sha=resolved["plan_sha256"],
                case=case,
                arm_binding=arm_binding,
                arm=arms[arm_binding["sdk"]],
                fixture=fixture,
                output=output,
                receipts_root=receipts,
                suite_attempt_id=suite_attempt_id,
                ts=now,
            )
            for arm_binding in plan["arms"]
        }
        arm_order = list(states)
        random.Random(f"{plan['sampling']['seed']}:{case['id']}").shuffle(arm_order)
        for repetition in range(plan["sampling"]["repetitions"]):
            offset = repetition % len(arm_order)
            for sdk in arm_order[offset:] + arm_order[:offset]:
                _run_cell_repetition(states[sdk], repetition)
        for arm_binding in plan["arms"]:
            manifest["cells"].append(_finish_cell(states[arm_binding["sdk"]]))
    _write_json(output / "suite-manifest.json", manifest)
    return manifest


def main(argv: list[str] | None = None) -> int:
    """Validate or run one pinned SDK suite from a versioned experiment file."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("validate", "run"))
    parser.add_argument("experiment_file", type=Path)
    parser.add_argument("output_root", type=Path)
    args = parser.parse_args(argv)
    result = (
        validate_experiment(args.experiment_file, args.output_root)
        if args.action == "validate"
        else run_experiment(args.experiment_file, args.output_root)
    )
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
