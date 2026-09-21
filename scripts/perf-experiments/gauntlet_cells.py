#!/usr/bin/env python3
"""Build closed invocation plans for Performance Gauntlet v1.2 cells."""

from __future__ import annotations

import hashlib
import json
import shlex
from pathlib import Path
from typing import Any


RESOLVED_SCHEMA = "fathomdb.performance-gauntlet.resolved-config/v1"
PERF_GATES_PATH = "artifact:perf_gates.path"
PERF_GATES_SHA256 = "artifact:perf_gates.sha256"
PERF_GATES_INPUT_SHA256 = "artifact:perf_gates.input_sha256"
SLICE_30A_CELLS = ("ac076", "ac072", "ac081", "ac073")
SLICE_30B_CELLS = ("ac075", "scale02")
SLICE_30C_CELLS = ("protected-writes", "ce-profile", "search01", "locomo")
SLICE_40_CELLS = ("ac013-scale-matrix",)
GRAPH_CELLS = ("graph-evidence01", "graph-expand01", "graph-retrieval01")
SUPPORTED_CELLS = (
    SLICE_30A_CELLS + SLICE_30B_CELLS + SLICE_30C_CELLS + SLICE_40_CELLS + GRAPH_CELLS
)
PLAN_KEYS = (
    "cell",
    "adapter",
    "cwd",
    "env",
    "unset_env",
    "invocations",
    "inputs",
    "outputs",
    "timeout_s",
    "workload_identity",
)
EU7_PINS = {
    "AGENT_LONG": "1",
    "EU7_N_VALUES": "7667",
    "EU7_QUERIES": "100",
    "EU7_BOOTSTRAP": "1000",
    "EU7_LATENCY_SAMPLES": "1000",
    "EU7_STRESS_PER_THREAD": "250",
}
EU7_ARGV = [
    "cargo",
    "test",
    "--release",
    "-p",
    "fathomdb-engine",
    "--features",
    "operator,default-embedder",
    "--test",
    "eu7_real_corpus_ac",
    "eu7_real_corpus_ac_validation",
    "--",
    "--exact",
    "--ignored",
    "--nocapture",
    "--test-threads=1",
]
EU7_CORPUS_SHA256 = "af1484a4873e61d98647ea44ab3cb452a3b42d9d7ccd32babb68f50ba531b65e"
EU7_MODEL_SHA256 = {
    "config.json": "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750",
    "tokenizer.json": "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
    "model.safetensors": "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad",
}
TC5_BASE_SHA256 = "b2ea5c25eee0b93807384259262702d3bb04f1fee4f640579160091ecee2417c"
SCALE02_BASE_SHA256 = "eb86d5b41e63b4854bde695200b9a0b9552a5c2474650f7cb0762b862851cd28"
TC5_CORPUS_INDEX_SHA256 = (
    "624b6b42e7e1d40866fea48d37a34b0ef8d786f46f4aa2ed9a3dd245d22a0a0a"
)
TC5_QUALIFIED_SHA256 = (
    "f6180a00d1551a143a7445aa6ed28ed589533400ee702835663729694e393df5"
)
TC5_MODEL_DIRECTORY = "0b2926f8a9b1"
TC5_MODEL_DIGEST = "7a7edec71b9b8c9ce82cae05c4be673b274c36eaa474cc40ca2ef81b77847ea2"
LOCOMO_BASE_SHA256 = "2b955b6fa40b605564fa00d06e27dbdae0ad2fbe49f1120cc1406a127fd87bfb"
LOCOMO_RAW_SHA256 = "79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4"
LOCOMO_PROVENANCE_SHA256 = (
    "43453c5d1b865dd1721bd0892f1af7f965e2c8484fea999eefa5cddf9da78f66"
)
LOCOMO_UPSTREAM_SHA = "4b61c5d31b9c668a12b4f5e78064248a02c82d2b"
LOCOMO_PATCH_SHA256 = "33b3468820c4c9af7653e573a9d8ecd448f9734948cddb454cefd742230b2ce1"
LOCOMO_PATCHED_TREE_SHA = "13921dfe59f19401fd9d681653f88a3aa6228dce"
CE_MANIFEST_SHA256 = "4535895d0ae0e2bb1febd61e036c9013f7c607114b250dab3458980fa6cfaae9"
CE_BASELINE_CPU_SHA256 = (
    "c093a4e131515496552f90bc6e8adf7d3b864b371815c4ceeab408c3bb4c2b39"
)
CE_BASELINE_CUDA_SHA256 = (
    "e40fbccc9c357d6ada801e234b8a9bf372418195d1cb32b6646cd8d28cc1196f"
)
TC5_BENCHMARK_PATH = "artifact:tc5_benchmark.path"
TC5_BENCHMARK_SHA256 = "artifact:tc5_benchmark.sha256"
AC075_OVERLAY_PATH = "artifact:ac075_overlay.path"
AC075_OVERLAY_SHA256 = "artifact:ac075_overlay.sha256"
TC5_ALLOWED_DIFF = {
    "/candidate",
    "/inputs/corpus_root",
    "/inputs/model_asset_directory",
    "/inputs/qualified_manifest",
    "/release",
    "/runtime/cuda_uuid",
    "/runtime/fathomdb_bin",
    "/runtime/python",
}
SCALE02_ALLOWED_DIFF = {
    "/corpus/qualified_manifest",
    "/corpus/root",
    "/release",
    "/runtime/fathomdb_bin",
    "/runtime/fathomdb_bin_sha256",
    "/runtime/python",
    "/runtime/python_extension",
    "/runtime/python_extension_sha256",
    "/runtime/python_package_version",
}


class CellPlanError(ValueError):
    """Report an invalid or unavailable cell-planning input."""


def _object(value: object, location: str) -> dict[str, Any]:
    """Return a string-keyed object or raise a cell-plan error."""
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        raise CellPlanError(f"{location} must be an object")
    return value


def _required_binding(container: object, key: str, location: str) -> dict[str, Any]:
    """Return one required non-null resolved binding."""
    document = _object(container, location)
    value = document.get(key)
    if value is None:
        raise CellPlanError(f"required binding {location}.{key} is unavailable")
    return _object(value, f"{location}.{key}")


def _binding_path(binding: object, location: str) -> Path:
    """Return the canonical path named by a resolved binding."""
    document = _object(binding, location)
    value = document.get("path")
    if not isinstance(value, str) or not value:
        raise CellPlanError(f"required binding {location}.path is unavailable")
    try:
        return Path(value).resolve(strict=True)
    except OSError as error:
        raise CellPlanError(
            f"required path {location} is unavailable: {value}"
        ) from error


def _sha256(path: Path) -> str:
    """Return one file's SHA-256 digest."""
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise CellPlanError(f"cannot hash required input {path}: {error}") from error
    return digest.hexdigest()


def _verified_file_binding(binding: object, location: str) -> Path:
    """Resolve a file binding and verify its recorded content identity."""
    document = _object(binding, location)
    path = _binding_path(document, location)
    expected = document.get("sha256")
    if not path.is_file() or not isinstance(expected, str) or _sha256(path) != expected:
        raise CellPlanError(f"required binding {location} has identity drift")
    return path


def _verified_launcher_binding(binding: object, location: str) -> Path:
    """Verify a venv launcher without resolving away its environment."""
    document = _object(binding, location)
    value = document.get("path")
    expected = document.get("sha256")
    if not isinstance(value, str) or not value:
        raise CellPlanError(f"required binding {location}.path is unavailable")
    path = Path(value).absolute()
    if not path.is_file() or not isinstance(expected, str) or _sha256(path) != expected:
        raise CellPlanError(f"required binding {location} has identity drift")
    return path


def _required_repo_file(repo_root: Path, relative: str) -> Path:
    """Resolve one required script or validator from the gauntlet checkout."""
    path = (repo_root / relative).resolve()
    if not path.is_file():
        raise CellPlanError(f"required gauntlet file is unavailable: {relative}")
    return path


def _base_plan(
    *,
    cell: str,
    adapter: str,
    cwd: Path,
    env: dict[str, str],
    unset_env: list[str],
    invocations: list[dict[str, object]],
    inputs: dict[str, object],
    outputs: dict[str, object],
    timeout_s: int,
    workload_identity: dict[str, object],
) -> dict[str, object]:
    """Construct one plan with the exact shared key set."""
    plan = {
        "cell": cell,
        "adapter": adapter,
        "cwd": str(cwd),
        "env": env,
        "unset_env": unset_env,
        "invocations": invocations,
        "inputs": inputs,
        "outputs": outputs,
        "timeout_s": timeout_s,
        "workload_identity": workload_identity,
    }
    if tuple(plan) != PLAN_KEYS:
        raise AssertionError("cell plan key order drifted")
    return plan


def _plan_ac076(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan the existing one-shot AC-076 text runner."""
    adapter = _required_repo_file(repo_root, "scripts/perf-experiments/run-ac012.sh")
    raw_log = output_root / "cells" / "ac076" / "ac012.log"
    return _base_plan(
        cell="ac076",
        adapter=str(adapter),
        cwd=source_root,
        env={
            "AGENT_LONG": "1",
            "AC012_CORPUS_N": "10000",
            "LOG_PATH": str(raw_log),
        },
        unset_env=["AC_FULL_SCALE"],
        invocations=[{"label": "run-1", "argv": ["bash", str(adapter)]}],
        inputs={"source": resolved["source"]},
        outputs={"raw_log": str(raw_log)},
        timeout_s=_timeout(resolved, "ac076"),
        workload_identity={
            "selector": "ac_012_text_query_latency_on_fts5_path",
            "corpus_n": 10000,
            "repetitions": 1,
        },
    )


def _plan_ac072(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan the identity-bound three-observation AC-072 dispatcher."""
    configs = _object(resolved.get("configs"), "configs")
    config = _required_binding(configs, "ac072", "configs")
    config_path = _verified_file_binding(config, "configs.ac072")
    adapter = _required_repo_file(
        repo_root, "scripts/perf-experiments/run-slice80-ac072-campaign.sh"
    )
    runner = _required_repo_file(
        repo_root, "scripts/perf-experiments/run-slice80-ac072-cell.sh"
    )
    scanner = _required_repo_file(repo_root, "dev/tools/slice80_read_acceptance.py")
    validator = _required_repo_file(repo_root, "dev/tools/slice80_ac072_acceptance.py")
    raw_root = output_root / "cells" / "ac072" / "campaign"
    commit = _source_commit(resolved)
    argv = [
        "bash",
        str(adapter),
        str(source_root),
        PERF_GATES_PATH,
        str(raw_root),
        commit,
        PERF_GATES_SHA256,
        PERF_GATES_INPUT_SHA256,
        _sha256(runner),
        _sha256(scanner),
        _sha256(validator),
        _sha256(adapter),
    ]
    return _base_plan(
        cell="ac072",
        adapter=str(adapter),
        cwd=source_root,
        env={},
        unset_env=[],
        invocations=[{"label": "campaign", "argv": argv}],
        inputs={
            "source": resolved["source"],
            "ac072_config": {
                "path": str(config_path),
                "sha256": config["sha256"],
                "usage": "provenance-only",
            },
            "runner": {"path": str(runner), "sha256": _sha256(runner)},
            "scanner": {"path": str(scanner), "sha256": _sha256(scanner)},
            "validator": {"path": str(validator), "sha256": _sha256(validator)},
            "dispatcher": {"path": str(adapter), "sha256": _sha256(adapter)},
        },
        outputs={"raw_root": str(raw_root)},
        timeout_s=_timeout(resolved, "ac072"),
        workload_identity={
            "corpus_n": 10000,
            "vector_dim": 384,
            "queries": 1000,
            "treatment": "warm",
            "repetitions": 3,
            "dispatcher_labels": ["R1", "R2", "R3"],
        },
    )


def _plan_ac081(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan seven fresh-process AC-081 observations."""
    adapter = _required_repo_file(
        repo_root, "scripts/perf-experiments/run-slice80-ac081-cell.sh"
    )
    scanner = _required_repo_file(repo_root, "dev/tools/slice80_read_acceptance.py")
    raw_root = output_root / "cells" / "ac081"
    commit = _source_commit(resolved)
    invocations = []
    logs = []
    for index in range(1, 8):
        label = f"R{index}"
        raw_log = raw_root / f"{label}.log"
        logs.append(str(raw_log))
        invocations.append(
            {
                "label": label,
                "argv": [
                    "bash",
                    str(adapter),
                    str(source_root),
                    PERF_GATES_PATH,
                    str(raw_log),
                    commit,
                    PERF_GATES_SHA256,
                    PERF_GATES_INPUT_SHA256,
                ],
            }
        )
    return _base_plan(
        cell="ac081",
        adapter=str(adapter),
        cwd=source_root,
        env={"SLICE80_AC081_SCANNER": str(scanner)},
        unset_env=[],
        invocations=invocations,
        inputs={
            "source": resolved["source"],
            "runner": {"path": str(adapter), "sha256": _sha256(adapter)},
            "scanner": {"path": str(scanner), "sha256": _sha256(scanner)},
        },
        outputs={"raw_logs": logs},
        timeout_s=_timeout(resolved, "ac081"),
        workload_identity={
            "selector": "ac_081_absolute_read_performance",
            "repetitions": 7,
            "fresh_process_per_repetition": True,
        },
    )


def _canonical_corpus_sha(raw_dir: Path) -> tuple[int, str]:
    """Reproduce the frozen sorted sha256sum-of-sha256sum corpus identity."""
    paths = sorted(raw_dir.glob("*.jsonl"))
    lines = [f"{_sha256(path)}  data/corpus-data/raw/{path.name}\n" for path in paths]
    return len(paths), hashlib.sha256("".join(lines).encode()).hexdigest()


def _load_json(path: Path, location: str) -> dict[str, Any]:
    """Load one required JSON object."""
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CellPlanError(f"cannot load required {location}: {error}") from error
    return _object(value, location)


def _canonical_json(document: object) -> bytes:
    """Encode one deterministic overlay document."""
    return (
        json.dumps(
            document,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=True,
            allow_nan=False,
        )
        + "\n"
    ).encode()


def _json_pointer_diff(left: object, right: object, prefix: str = "") -> list[str]:
    """Return leaf/add/remove differences as sorted JSON pointers."""
    if isinstance(left, dict) and isinstance(right, dict):
        pointers: list[str] = []
        for key in sorted(set(left) | set(right)):
            escaped = key.replace("~", "~0").replace("/", "~1")
            pointer = f"{prefix}/{escaped}"
            if key not in left or key not in right:
                pointers.append(pointer)
            else:
                pointers.extend(_json_pointer_diff(left[key], right[key], pointer))
        return pointers
    return [] if left == right else [prefix or "/"]


def _verified_frozen_config(
    resolved: dict[str, Any], key: str, expected_sha256: str, label: str
) -> tuple[Path, dict[str, Any]]:
    """Load one recorded config only when it is the exact frozen base."""
    configs = _object(resolved.get("configs"), "configs")
    binding = _required_binding(configs, key, "configs")
    path = _verified_file_binding(binding, f"configs.{key}")
    if _sha256(path) != expected_sha256:
        raise CellPlanError(f"required frozen {label} base identity drifted")
    return path, _load_json(path, f"frozen {label} base")


def _runtime_file(resolved: dict[str, Any], key: str) -> tuple[Path, str]:
    """Return one required verified target runtime file and hash."""
    runtime = _object(resolved.get("runtime"), "runtime")
    binding = _required_binding(runtime, key, "runtime")
    if key == "python":
        value = binding.get("path")
        if not isinstance(value, str) or not value:
            raise CellPlanError("required runtime.python.path is unavailable")
        path = Path(value).absolute()
        if not path.is_file() or _sha256(path) != binding.get("sha256"):
            raise CellPlanError("required binding runtime.python has identity drift")
    else:
        path = _verified_file_binding(binding, f"runtime.{key}")
    digest = binding.get("sha256")
    if not isinstance(digest, str):
        raise CellPlanError(f"required runtime.{key}.sha256 is unavailable")
    return path, digest


def _tc5_assets(
    resolved: dict[str, Any],
) -> tuple[Path, Path, Path]:
    """Resolve and verify the shared frozen TC-5 inputs."""
    assets = _object(resolved.get("assets"), "assets")
    tc5 = _required_binding(assets, "tc5", "assets")
    corpus = _binding_path(
        _required_binding(tc5, "corpus_root", "assets.tc5"),
        "assets.tc5.corpus_root",
    )
    qualified_binding = _required_binding(tc5, "qualified_manifest", "assets.tc5")
    qualified = _verified_file_binding(
        qualified_binding, "assets.tc5.qualified_manifest"
    )
    if qualified_binding.get("sha256") != TC5_QUALIFIED_SHA256:
        raise CellPlanError("required TC-5 qualified manifest identity drifted")
    index = corpus / "tc5-corpus-input.v1.json"
    if not index.is_file() or _sha256(index) != TC5_CORPUS_INDEX_SHA256:
        raise CellPlanError("required TC-5 corpus index identity drifted")
    cache = _binding_path(
        _required_binding(tc5, "embedder_model_cache", "assets.tc5"),
        "assets.tc5.embedder_model_cache",
    )
    model = (cache / TC5_MODEL_DIRECTORY).resolve()
    if not model.is_dir() or not model.is_relative_to(cache):
        raise CellPlanError("required TC-5 model cache identity drifted")
    return corpus, qualified, model


def _gpu_uuid(resolved: dict[str, Any]) -> str:
    """Return the selected enabled GPU UUID."""
    gpu = _object(resolved.get("gpu"), "gpu")
    uuid = gpu.get("cuda_uuid")
    if gpu.get("enabled") is not True or not isinstance(uuid, str):
        raise CellPlanError("required GPU binding is unavailable")
    return uuid


def _eu7_cell(manifest: dict[str, Any]) -> dict[str, Any]:
    """Return the one exact eu7-real cell from a closure manifest."""
    cells = manifest.get("cells")
    if not isinstance(cells, list):
        raise CellPlanError("required closure manifest cells are unavailable")
    matches = [
        cell
        for cell in cells
        if isinstance(cell, dict) and cell.get("id") == "eu7-real"
    ]
    if len(matches) != 1:
        raise CellPlanError("required closure manifest eu7-real cell is not unique")
    return matches[0]


def _parse_eu7_command(
    command: object, run_dir: Path
) -> tuple[dict[str, str], list[str]]:
    """Parse the frozen env-prefixed EU7 command with output-only substitution."""
    if not isinstance(command, str):
        raise CellPlanError("required eu7-real command is unavailable")
    tokens = shlex.split(command)
    if not tokens or tokens[0] != "env":
        raise CellPlanError("eu7-real command must use the frozen env prefix")
    env: dict[str, str] = {}
    index = 1
    while index < len(tokens) and "=" in tokens[index]:
        key, value = tokens[index].split("=", 1)
        if "${" in value and "${RUN_DIR}" not in value:
            raise CellPlanError("eu7-real command contains an unsupported substitution")
        env[key] = value.replace("${RUN_DIR}", str(run_dir))
        index += 1
    argv = tokens[index:]
    if argv != EU7_ARGV:
        raise CellPlanError("eu7-real command argv drifted")
    expected_keys = set(EU7_PINS) | {"FATHOMDB_EU7_OUTPUT"}
    if set(env) != expected_keys or env.get("FATHOMDB_EU7_OUTPUT") != str(
        run_dir / "eu7.json"
    ):
        raise CellPlanError("eu7-real command substitution or environment drifted")
    for key, expected in EU7_PINS.items():
        if env.get(key) != expected:
            raise CellPlanError(f"eu7-real workload pin drifted: {key}")
    return env, argv


def _verify_model_cache(
    manifest: dict[str, Any], cache_root: Path
) -> dict[str, object]:
    """Verify the model files inherited by the frozen EU7 cell."""
    external = _object(
        manifest.get("external_inputs_by_cell"), "external_inputs_by_cell"
    )
    if _object(external.get("eu7-real"), "external_inputs_by_cell.eu7-real") != {
        "inherit": "model-cache"
    }:
        raise CellPlanError("required eu7-real model cache inheritance drifted")
    expected = _object(
        external.get("model-cache"), "external_inputs_by_cell.model-cache"
    )
    required = {
        f"${{FATHOMDB_EMBEDDER_CACHE}}/0b2926f8a9b1/{name}": digest
        for name, digest in EU7_MODEL_SHA256.items()
    }
    if expected != required:
        raise CellPlanError("required model cache contract drifted")
    files = []
    prefix = "${FATHOMDB_EMBEDDER_CACHE}/"
    for template, digest in sorted(expected.items()):
        if not template.startswith(prefix) or not isinstance(digest, str):
            raise CellPlanError("required model cache contract drifted")
        path = (cache_root / template.removeprefix(prefix)).resolve()
        if not path.is_file() or _sha256(path) != digest:
            raise CellPlanError(f"model cache identity drift: {path}")
        files.append({"path": str(path), "sha256": digest})
    return {"path": str(cache_root), "files": files}


def _verify_corpus(
    manifest: dict[str, Any], source_root: Path, configured_root: Path
) -> dict[str, object]:
    """Verify target-path connectivity and the frozen 13-file corpus identity."""
    target_root = source_root / "data" / "corpus-data"
    try:
        target_canonical = target_root.resolve(strict=True)
        configured_canonical = configured_root.resolve(strict=True)
    except OSError as error:
        raise CellPlanError(
            f"required AC-073 corpus is unavailable: {error}"
        ) from error
    if target_canonical != configured_canonical:
        raise CellPlanError(
            "target data/corpus-data must canonical-resolve to configured AC-073 corpus"
        )
    external = _object(
        manifest.get("external_inputs_by_cell"), "external_inputs_by_cell"
    )
    contract = _object(external.get("eu7-corpus"), "external_inputs_by_cell.eu7-corpus")
    expected_contract = {
        "path_glob",
        "file_count",
        "canonical_sha256",
        "consumer",
    }
    if set(contract) != expected_contract or contract != {
        "path_glob": "data/corpus-data/raw/*.jsonl",
        "file_count": 13,
        "canonical_sha256": EU7_CORPUS_SHA256,
        "consumer": "eu7-real",
    }:
        raise CellPlanError("required eu7 corpus contract drifted")
    count, digest = _canonical_corpus_sha(target_canonical / "raw")
    if count != contract["file_count"] or digest != contract["canonical_sha256"]:
        raise CellPlanError("AC-073 corpus identity drifted")
    return {"path": str(target_canonical), "files": count, "sha256": digest}


def _plan_ac073(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan frozen EU7 AC-073 against verified real corpus and model inputs."""
    del repo_root
    configs = _object(resolved.get("configs"), "configs")
    manifest_binding = _required_binding(configs, "ac073", "configs")
    manifest_path = _verified_file_binding(manifest_binding, "configs.ac073")
    manifest = _load_json(manifest_path, "AC-073 closure manifest")
    if manifest.get("schema_version") != "fathomdb.slice75-closure-manifest/v1" or (
        manifest.get("release") != "0.8.25"
    ):
        raise CellPlanError("required AC-073 closure manifest identity drifted")
    assets = _object(resolved.get("assets"), "assets")
    ac073_assets = _required_binding(assets, "ac073", "assets")
    corpus_binding = _required_binding(ac073_assets, "corpus_root", "assets.ac073")
    corpus_root = _binding_path(corpus_binding, "assets.ac073.corpus_root")
    tc5_assets = _required_binding(assets, "tc5", "assets")
    cache_binding = _required_binding(tc5_assets, "embedder_model_cache", "assets.tc5")
    cache_root = _binding_path(cache_binding, "assets.tc5.embedder_model_cache")
    cell = _eu7_cell(manifest)
    commands = cell.get("commands")
    if not isinstance(commands, list) or len(commands) != 1:
        raise CellPlanError("required eu7-real command count drifted")
    run_dir = output_root / "cells" / "ac073"
    env, argv = _parse_eu7_command(commands[0], run_dir)
    uuid = _gpu_uuid(resolved)
    argv = list(argv)
    argv[argv.index("operator,default-embedder")] = "operator,embed-cuda"
    env["FATHOMDB_EMBEDDER_CACHE"] = str(cache_root)
    env["FATHOMDB_EMBED_DEVICE"] = "cuda:0"
    env["CUDA_VISIBLE_DEVICES"] = uuid
    corpus = _verify_corpus(manifest, source_root, corpus_root)
    model_cache = _verify_model_cache(manifest, cache_root)
    unset = cell.get("unset_environment")
    expected_unset = ["FATHOMDB_SKIP_NETWORK_TESTS", "EU7_FORCE_FULL_RECOMPUTE"]
    if unset != expected_unset or cell.get("timeout_seconds") != 10800:
        raise CellPlanError("required eu7-real timeout or unset environment drifted")
    if _timeout(resolved, "ac073") != 10800:
        raise CellPlanError("required AC-073 effective timeout must equal 10800")
    output = run_dir / "eu7.json"
    if env.get("FATHOMDB_EU7_OUTPUT") != str(output):
        raise CellPlanError("required eu7-real output substitution drifted")
    return _base_plan(
        cell="ac073",
        adapter=f"{manifest_path}#eu7-real",
        cwd=source_root,
        env=env,
        unset_env=unset,
        invocations=[{"label": "run-1", "argv": argv}],
        inputs={
            "source": resolved["source"],
            "closure_manifest": {
                "path": str(manifest_path),
                "sha256": manifest_binding["sha256"],
            },
            "corpus": corpus,
            "model_cache": model_cache,
        },
        outputs={
            "result": str(output),
            "completion_requirements": ["no SKIP:", "result exists"],
        },
        timeout_s=10800,
        workload_identity={
            "documents": 7667,
            "queries": 100,
            "bootstrap_samples": 1000,
            "latency_samples": 1000,
            "stress_operations_per_thread": 250,
            "corpus_files": 13,
            "corpus_sha256": corpus["sha256"],
            "encoding_device": "cuda:0",
            "cuda_uuid": uuid,
            "repetitions": 1,
        },
    )


def _plan_ac075(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan the frozen TC-5 bridge with a two-stage candidate overlay."""
    del source_root
    if resolved.get("release") != "0.8.26":
        raise CellPlanError("AC-075 gauntlet overlay requires release 0.8.26")
    base_path, base = _verified_frozen_config(
        resolved, "ac075", TC5_BASE_SHA256, "TC-5"
    )
    corpus, qualified, model = _tc5_assets(resolved)
    python, python_sha = _runtime_file(resolved, "python")
    wheel, wheel_sha = _runtime_file(resolved, "wheel")
    cli, cli_sha = _runtime_file(resolved, "fathomdb_cli")
    uuid = _gpu_uuid(resolved)
    template = json.loads(json.dumps(base))
    template["release"] = "0.8.26"
    template["inputs"]["corpus_root"] = str(corpus)
    template["inputs"]["qualified_manifest"] = str(qualified)
    template["inputs"]["model_asset_directory"] = str(model)
    template["runtime"]["python"] = str(python)
    template["runtime"]["fathomdb_bin"] = str(cli)
    template["runtime"]["cuda_uuid"] = uuid
    template["candidate"] = {
        "sha": _source_commit(resolved),
        "version": "0.8.26",
        "package_version": "0.8.26",
        "python_wheel": str(wheel),
        "python_wheel_sha256": wheel_sha,
        "fathomdb_bin_sha256": cli_sha,
        "benchmark_binary": TC5_BENCHMARK_PATH,
        "benchmark_binary_sha256": TC5_BENCHMARK_SHA256,
    }
    if template["inputs"].get("corpus_index_sha256") != TC5_CORPUS_INDEX_SHA256:
        raise CellPlanError("required frozen TC-5 corpus index contract drifted")
    if template["inputs"].get("qualified_manifest_sha256") != TC5_QUALIFIED_SHA256:
        raise CellPlanError("required frozen TC-5 qualified manifest contract drifted")
    if template["inputs"].get("model_asset_digest") != TC5_MODEL_DIGEST:
        raise CellPlanError("required frozen TC-5 model contract drifted")
    changed = _json_pointer_diff(base, template)
    if not set(changed).issubset(TC5_ALLOWED_DIFF):
        raise CellPlanError("TC-5 overlay changed a non-whitelisted field")
    runner = _required_repo_file(repo_root, "experiments/tc5_gpu_v2.py")
    test_setup = _required_repo_file(repo_root, "experiments/fathomdb_test_setup.py")
    template_path = output_root / "cells" / "ac075" / "tc5-0.8.26.template.json"
    run_root = output_root / "cells" / "ac075" / "run"
    return _base_plan(
        cell="ac075",
        adapter=str(runner),
        cwd=repo_root,
        env={},
        unset_env=[],
        invocations=[
            {
                "label": "bridge",
                "argv": [
                    str(python),
                    "-m",
                    "experiments.tc5_gpu_v2",
                    "run",
                    "--config",
                    AC075_OVERLAY_PATH,
                    "--arm",
                    "bridge",
                    "--output-root",
                    str(run_root),
                    "--binary",
                    TC5_BENCHMARK_PATH,
                ],
            }
        ],
        inputs={
            "source": resolved["source"],
            "base_config": {"path": str(base_path), "sha256": TC5_BASE_SHA256},
            "overlay_template": {
                "path": str(template_path),
                "sha256": hashlib.sha256(_canonical_json(template)).hexdigest(),
                "changed_pointers": changed,
                "document": template,
            },
            "final_overlay": {
                "path": AC075_OVERLAY_PATH,
                "sha256": AC075_OVERLAY_SHA256,
            },
            "runtime": {
                "python": {"path": str(python), "sha256": python_sha},
                "wheel": {"path": str(wheel), "sha256": wheel_sha},
                "fathomdb_cli": {"path": str(cli), "sha256": cli_sha},
            },
            "assets": {
                "corpus_root": str(corpus),
                "qualified_manifest": {
                    "path": str(qualified),
                    "sha256": TC5_QUALIFIED_SHA256,
                },
                "model_directory": str(model),
                "model_digest": TC5_MODEL_DIGEST,
                "cuda_uuid": uuid,
            },
            "artifacts": {
                "tc5_benchmark": {
                    "path": TC5_BENCHMARK_PATH,
                    "sha256": TC5_BENCHMARK_SHA256,
                }
            },
            "modules": {
                "runner": {"path": str(runner), "sha256": _sha256(runner)},
                "test_setup": {
                    "path": str(test_setup),
                    "sha256": _sha256(test_setup),
                },
            },
        },
        outputs={
            "template": str(template_path),
            "final_overlay": AC075_OVERLAY_PATH,
            "run_root": str(run_root),
        },
        timeout_s=_timeout(resolved, "ac075"),
        workload_identity={
            "arm": "bridge",
            "documents": 7667,
            "candidate_k": 192,
            "top_k": 10,
            "queries": 100,
            "bootstrap_resamples": 1000,
            "query_select_seed": "0x0E77C0125E1EC7",
            "bootstrap_seed": "0x0E77B007574A9",
            "ground_truth": "exact-f32-same-model-top-10",
            "repetitions": 1,
        },
    )


def _plan_scale02(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan the frozen five-point SCALE-02 campaign with a private registry."""
    del source_root
    release = resolved.get("release")
    if release != "0.8.26":
        raise CellPlanError("SCALE-02 gauntlet overlay requires release 0.8.26")
    base_path, base = _verified_frozen_config(
        resolved, "scale02", SCALE02_BASE_SHA256, "SCALE-02"
    )
    return _plan_scale02_continued(
        resolved, repo_root, output_root, release, base_path, base
    )


def _asset_group(resolved: dict[str, Any], name: str) -> dict[str, Any]:
    """Return one required resolved asset group."""
    return _required_binding(_object(resolved.get("assets"), "assets"), name, "assets")


def _plan_protected_writes(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan the existing protected-write probe for production treatment only."""
    adapter = _required_repo_file(
        repo_root, "scripts/perf-experiments/run-protected-production.py"
    )
    configs = _object(resolved.get("configs"), "configs")
    manifest_binding = _required_binding(configs, "protected_writes", "configs")
    manifest = _verified_file_binding(manifest_binding, "configs.protected_writes")
    python, python_sha = _runtime_file(resolved, "python")
    raw_root = output_root / "cells" / "protected-writes" / "run"
    return _base_plan(
        cell="protected-writes",
        adapter=str(adapter),
        cwd=repo_root,
        env={},
        unset_env=[],
        invocations=[
            {
                "label": "production",
                "argv": [
                    str(python),
                    str(adapter),
                    "--source-root",
                    str(source_root),
                    "--manifest",
                    str(manifest),
                    "--output-root",
                    str(raw_root),
                ],
            }
        ],
        inputs={
            "source": resolved["source"],
            "directional_manifest": {
                "path": str(manifest),
                "sha256": manifest_binding["sha256"],
            },
            "python": {"path": str(python), "sha256": python_sha},
        },
        outputs={"raw_root": str(raw_root), "receipt": str(raw_root / "receipt.json")},
        timeout_s=_timeout(resolved, "protected-writes"),
        workload_identity={
            "fixtures": ["scale02", "ac013"],
            "treatment": "production",
            "repetitions_per_fixture": 3,
        },
    )


def _plan_ce_profile(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan the Slice 72 CE candidate against the recorded 0.8.25 baseline."""
    adapter = _required_repo_file(
        repo_root, "scripts/perf-experiments/run-ce-profile.py"
    )
    configs = _object(resolved.get("configs"), "configs")
    manifest_binding = _required_binding(configs, "ce_profile", "configs")
    manifest = _verified_file_binding(manifest_binding, "configs.ce_profile")
    if manifest_binding.get("sha256") != CE_MANIFEST_SHA256:
        raise CellPlanError("required frozen CE manifest identity drifted")
    baseline_cpu = _required_repo_file(
        repo_root,
        "dev/plans/runs/0.8.25-slice-72/baseline-cpu/baseline-cpu.json",
    )
    baseline_cuda = _required_repo_file(
        repo_root,
        "dev/plans/runs/0.8.25-slice-72/baseline-cuda/baseline-cuda.json",
    )
    if _sha256(baseline_cpu) != CE_BASELINE_CPU_SHA256:
        raise CellPlanError("required frozen CE CPU baseline identity drifted")
    if _sha256(baseline_cuda) != CE_BASELINE_CUDA_SHA256:
        raise CellPlanError("required frozen CE CUDA baseline identity drifted")
    assets = _asset_group(resolved, "ce_profile")
    model_cache = _binding_path(
        _required_binding(assets, "reranker_model_cache", "assets.ce_profile"),
        "assets.ce_profile.reranker_model_cache",
    )
    if model_cache.parts[-2:] != ("fathomdb", "reranker"):
        raise CellPlanError("required CE reranker cache layout drifted")
    cache_root = model_cache.parents[1]
    python, python_sha = _runtime_file(resolved, "python")
    raw_root = output_root / "cells" / "ce-profile" / "run"
    return _base_plan(
        cell="ce-profile",
        adapter=str(adapter),
        cwd=repo_root,
        env={},
        unset_env=[],
        invocations=[
            {
                "label": "candidate-cpu-cuda",
                "argv": [
                    str(python),
                    str(adapter),
                    "--base-manifest",
                    str(manifest),
                    "--source-root",
                    str(source_root),
                    "--cache-root",
                    str(cache_root),
                    "--candidate-sha",
                    _source_commit(resolved),
                    "--cuda-uuid",
                    _gpu_uuid(resolved),
                    "--output-root",
                    str(raw_root),
                ],
            }
        ],
        inputs={
            "source": resolved["source"],
            "manifest": {"path": str(manifest), "sha256": manifest_binding["sha256"]},
            "baseline_cpu": {
                "path": str(baseline_cpu),
                "sha256": CE_BASELINE_CPU_SHA256,
            },
            "baseline_cuda": {
                "path": str(baseline_cuda),
                "sha256": CE_BASELINE_CUDA_SHA256,
            },
            "model_cache": str(model_cache),
            "cache_root": str(cache_root),
            "python": {"path": str(python), "sha256": python_sha},
        },
        outputs={
            "raw_root": str(raw_root),
            "verifier_status": str(raw_root / "verifier-status.json"),
            "candidate_cpu": str(raw_root / "candidate-cpu" / "candidate-cpu.json"),
            "candidate_cuda": str(raw_root / "candidate-cuda" / "candidate-cuda.json"),
        },
        timeout_s=_timeout(resolved, "ce-profile"),
        workload_identity={
            "baseline_release": "0.8.25",
            "candidate_release": resolved["release"],
            "devices": ["cpu", "cuda"],
            "cold_repetitions": 3,
            "steady_repetitions": 5,
            "steady_calls": 20,
        },
    )


def _plan_search01(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan the frozen SEARCH-01 IR-C characterization."""
    del source_root
    adapter = _required_repo_file(repo_root, "scripts/perf-experiments/run-search01.py")
    configs = _object(resolved.get("configs"), "configs")
    contract_binding = _required_binding(configs, "search01", "configs")
    contract_path = _verified_file_binding(contract_binding, "configs.search01")
    contract = _load_json(contract_path, "SEARCH-01 contract")
    expected_contract = {
        "schema_version": "fathomdb.performance-gauntlet.search01-contract/v1",
        "profile": "IR-C",
        "documents": 10506,
        "queries": 4597,
        "retrieval": "search_text_only",
        "limit": 10,
        "cutoffs": [5, 10],
        "corpus_sha256": "fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e",
        "qrels_version": "ir-c-reused-v2",
        "gold_sha256": "4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb",
    }
    if contract != expected_contract:
        raise CellPlanError("required SEARCH-01 contract identity drifted")
    assets = _asset_group(resolved, "ir_c")
    data_root = _binding_path(
        _required_binding(assets, "data_root", "assets.ir_c"), "assets.ir_c.data_root"
    )
    files = {
        key: _verified_file_binding(
            _required_binding(assets, key, "assets.ir_c"), f"assets.ir_c.{key}"
        )
        for key in ("snapshot", "manifest", "gold")
    }
    python, python_sha = _runtime_file(resolved, "python")
    raw_root = output_root / "cells" / "search01" / "run"
    argv = [str(python), str(adapter), "--data-root", str(data_root)]
    for key in ("snapshot", "manifest", "gold"):
        argv.extend([f"--{key}", str(files[key])])
    argv.extend(
        [
            "--output-root",
            str(raw_root),
            "--expected-version",
            str(resolved["release"]),
        ]
    )
    return _base_plan(
        cell="search01",
        adapter=str(adapter),
        cwd=repo_root,
        env={},
        unset_env=[],
        invocations=[{"label": "ir-c", "argv": argv}],
        inputs={
            "source": resolved["source"],
            "contract": {
                "path": str(contract_path),
                "sha256": contract_binding["sha256"],
            },
            "data_root": str(data_root),
            **{key: str(value) for key, value in files.items()},
            "python": {"path": str(python), "sha256": python_sha},
        },
        outputs={"raw_root": str(raw_root)},
        timeout_s=_timeout(resolved, "search01"),
        workload_identity=contract,
    )


def _plan_locomo(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan LOCOMO using the existing facade, harness, and metrics modules."""
    del source_root
    adapter = _required_repo_file(repo_root, "scripts/perf-experiments/run-locomo.py")
    configs = _object(resolved.get("configs"), "configs")
    base_binding = _required_binding(configs, "locomo", "configs")
    base_path = _verified_file_binding(base_binding, "configs.locomo")
    if base_binding.get("sha256") != LOCOMO_BASE_SHA256:
        raise CellPlanError("required canonical LOCOMO A0 config identity drifted")
    overlay = _load_json(base_path, "LOCOMO base config")
    assets = _asset_group(resolved, "locomo")
    checkout = _binding_path(
        _required_binding(assets, "harness_checkout", "assets.locomo"),
        "assets.locomo.harness_checkout",
    )
    harness_identity = _object(
        resolved.get("locomo_harness_identity"), "locomo_harness_identity"
    )
    head = harness_identity.get("git_sha")
    if (
        harness_identity.get("checkout") != str(checkout)
        or harness_identity.get("upstream_git_sha") != LOCOMO_UPSTREAM_SHA
        or harness_identity.get("tree_sha") != LOCOMO_PATCHED_TREE_SHA
        or not isinstance(head, str)
    ):
        raise CellPlanError("required LOCOMO harness identity is unavailable")
    patch = Path(str(harness_identity.get("patch_path", "")))
    if (
        not patch.is_file()
        or harness_identity.get("patch_sha256") != LOCOMO_PATCH_SHA256
    ):
        raise CellPlanError("required LOCOMO harness patch identity drifted")
    harness_python = _verified_launcher_binding(
        _required_binding(assets, "harness_python", "assets.locomo"),
        "assets.locomo.harness_python",
    )
    dataset = _verified_file_binding(
        _required_binding(assets, "dataset", "assets.locomo"), "assets.locomo.dataset"
    )
    if _sha256(dataset) != LOCOMO_RAW_SHA256:
        raise CellPlanError("required LOCOMO corpus identity drifted")
    provenance_binding = _required_binding(
        assets, "provenance_manifest", "assets.locomo"
    )
    provenance = _verified_file_binding(
        provenance_binding, "assets.locomo.provenance_manifest"
    )
    if _sha256(provenance) != LOCOMO_PROVENANCE_SHA256:
        raise CellPlanError("required LOCOMO provenance identity drifted")
    external_root = _binding_path(
        _required_binding(assets, "external_output_root", "assets.locomo"),
        "assets.locomo.external_output_root",
    )
    if external_root == repo_root or repo_root in external_root.parents:
        raise CellPlanError("LOCOMO raw output root must be outside the repository")
    python, python_sha = _runtime_file(resolved, "python")
    cli, cli_sha = _runtime_file(resolved, "fathomdb_cli")
    cell_root = output_root / "cells" / "locomo"
    overlay["harness"]["checkout"] = str(checkout)
    overlay["harness"]["python"] = str(harness_python)
    overlay["harness"]["git_sha"] = head
    overlay["harness"]["upstream_git_sha"] = LOCOMO_UPSTREAM_SHA
    overlay["corpus"]["dataset_path"] = str(dataset)
    overlay["facade"]["python"] = str(python)
    overlay["facade"]["fathomdb_bin"] = str(cli)
    overlay["facade"]["provenance_manifest"] = str(provenance)
    overlay["facade"]["provenance_manifest_sha256"] = provenance_binding["sha256"]
    raw_root = external_root / f"fathomdb-{resolved['release']}"
    overlay["output"]["external_root"] = str(raw_root)
    overlay_path = cell_root / "locomo.overlay.json"
    result = cell_root / "result.json"
    return _base_plan(
        cell="locomo",
        adapter=str(adapter),
        cwd=repo_root,
        env={},
        unset_env=[],
        invocations=[
            {
                "label": "official-seam",
                "argv": [
                    str(python),
                    str(adapter),
                    "--config",
                    str(overlay_path),
                    "--base-dir",
                    str(cell_root / "receipts"),
                    "--result",
                    str(result),
                ],
            }
        ],
        inputs={
            "source": resolved["source"],
            "base_config": {"path": str(base_path), "sha256": base_binding["sha256"]},
            "harness": {
                "checkout": str(checkout),
                "python": str(harness_python),
                "git_sha": head,
                "upstream_git_sha": LOCOMO_UPSTREAM_SHA,
                "tree_sha": LOCOMO_PATCHED_TREE_SHA,
                "patch": {"path": str(patch), "sha256": LOCOMO_PATCH_SHA256},
            },
            "overlay": {
                "path": str(overlay_path),
                "document": overlay,
                "sha256": hashlib.sha256(_canonical_json(overlay)).hexdigest(),
            },
            "runtime": {
                "python": {"path": str(python), "sha256": python_sha},
                "fathomdb_cli": {"path": str(cli), "sha256": cli_sha},
            },
        },
        outputs={
            "overlay": str(overlay_path),
            "result": str(result),
            "raw_root": str(raw_root),
        },
        timeout_s=_timeout(resolved, "locomo"),
        workload_identity={
            "track": "LOCOMO",
            "conversations": 10,
            "eligible_questions": 1540,
            "top_k": 10,
            "ingest_unit": "turn",
            "retrieval": "fts-only",
            "sessions": 272,
        },
    )


def _plan_ac013_scale_matrix(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    """Plan the optional existing AC-013 cold/warm scale matrix unchanged."""
    adapter = _required_repo_file(
        repo_root, "scripts/perf-experiments/run-scale-ac013-matrix.sh"
    )
    runner = _required_repo_file(repo_root, "scripts/perf-experiments/run-ac013.sh")
    matrix_root = output_root / "cells" / "ac013-scale-matrix" / "run"
    return _base_plan(
        cell="ac013-scale-matrix",
        adapter=str(adapter),
        cwd=source_root,
        env={
            "SCALE_OUTPUT_DIR": str(matrix_root),
            "AC013_RUNNER": str(runner),
        },
        unset_env=[],
        invocations=[{"label": "matrix", "argv": ["bash", str(adapter)]}],
        inputs={
            "source": resolved["source"],
            "runner": {"path": str(runner), "sha256": _sha256(runner)},
        },
        outputs={"matrix_root": str(matrix_root)},
        timeout_s=_timeout(resolved, "ac013-scale-matrix"),
        workload_identity={
            "rows": [10000, 100000, 1000000],
            "treatments": ["process_cold", "warm"],
            "repetitions": 5,
            "default": False,
        },
    )


def _plan_scale02_continued(
    resolved: dict[str, Any],
    repo_root: Path,
    output_root: Path,
    release: str,
    base_path: Path,
    base: dict[str, Any],
) -> dict[str, object]:
    """Finish SCALE-02 planning after the frozen base has been verified."""
    corpus, qualified, _model = _tc5_assets(resolved)
    python, python_sha = _runtime_file(resolved, "python")
    extension, extension_sha = _runtime_file(resolved, "native_extension")
    cli, cli_sha = _runtime_file(resolved, "fathomdb_cli")
    overlay = json.loads(json.dumps(base))
    overlay["release"] = release
    overlay["corpus"]["root"] = str(corpus)
    overlay["corpus"]["qualified_manifest"] = str(qualified)
    overlay["runtime"] = {
        "python": str(python),
        "python_package_version": release,
        "python_extension": str(extension),
        "python_extension_sha256": extension_sha,
        "fathomdb_bin": str(cli),
        "fathomdb_bin_sha256": cli_sha,
    }
    if overlay["corpus"].get("index_sha256") != TC5_CORPUS_INDEX_SHA256:
        raise CellPlanError("required frozen SCALE-02 corpus index contract drifted")
    if overlay["corpus"].get("qualified_manifest_sha256") != TC5_QUALIFIED_SHA256:
        raise CellPlanError("required frozen SCALE-02 manifest contract drifted")
    changed = _json_pointer_diff(base, overlay)
    if not set(changed).issubset(SCALE02_ALLOWED_DIFF):
        raise CellPlanError("SCALE-02 overlay changed a non-whitelisted field")
    runner = _required_repo_file(repo_root, "experiments/scale_02.py")
    library = _required_repo_file(repo_root, "experiments/_lib.py")
    test_setup = _required_repo_file(repo_root, "experiments/fathomdb_test_setup.py")
    cell_root = output_root / "cells" / "scale02"
    overlay_path = cell_root / "scale02-0.8.26.overlay.json"
    external = _asset_group(resolved, "locomo")
    external_root = _binding_path(
        _required_binding(external, "external_output_root", "assets.locomo"),
        "assets.locomo.external_output_root",
    )
    if external_root == repo_root or repo_root in external_root.parents:
        raise CellPlanError("SCALE-02 raw output root must be outside the repository")
    artifact_root = external_root.parent / "scale02-raw" / output_root.name
    registry = cell_root / "registry"
    points = (10000, 17272, 25000, 40000, 50000)
    invocations = []
    for point in points:
        argv = [
            str(python),
            "-m",
            "experiments.scale_02",
            "run-point",
            str(overlay_path),
            str(point),
            str(artifact_root),
            "--record-base-dir",
            str(registry),
        ]
        if point in (40000, 50000):
            argv.append("--post-boundary-baseline")
        invocations.append({"label": f"point-{point}", "argv": argv})
    return _base_plan(
        cell="scale02",
        adapter=str(runner),
        cwd=repo_root,
        env={},
        unset_env=[],
        invocations=invocations,
        inputs={
            "source": resolved["source"],
            "base_config": {"path": str(base_path), "sha256": SCALE02_BASE_SHA256},
            "overlay": {
                "path": str(overlay_path),
                "sha256": hashlib.sha256(_canonical_json(overlay)).hexdigest(),
                "changed_pointers": changed,
                "document": overlay,
            },
            "runtime": {
                "python": {"path": str(python), "sha256": python_sha},
                "native_extension": {
                    "path": str(extension),
                    "sha256": extension_sha,
                },
                "fathomdb_cli": {"path": str(cli), "sha256": cli_sha},
            },
            "assets": {
                "corpus_root": str(corpus),
                "qualified_manifest": {
                    "path": str(qualified),
                    "sha256": TC5_QUALIFIED_SHA256,
                },
            },
            "modules": {
                "runner": {"path": str(runner), "sha256": _sha256(runner)},
                "lib": {"path": str(library), "sha256": _sha256(library)},
                "test_setup": {
                    "path": str(test_setup),
                    "sha256": _sha256(test_setup),
                },
            },
        },
        outputs={
            "overlay": str(overlay_path),
            "artifact_root": str(artifact_root),
            "record_base_dir": str(registry),
        },
        timeout_s=_timeout(resolved, "scale02"),
        workload_identity={
            "points": list(points),
            "repetitions_per_point": 5,
            "formal_points": [10000, 17272, 25000],
            "post_boundary_points": [40000, 50000],
            "profile": "a0_turn_fts",
            "top_k": 10,
        },
    )


def _source_commit(resolved: dict[str, Any]) -> str:
    """Return the target source commit identity."""
    source = _object(resolved.get("source"), "source")
    commit = source.get("commit")
    if not isinstance(commit, str) or len(commit) != 40:
        raise CellPlanError("required source.commit is unavailable")
    return commit


def _timeout(resolved: dict[str, Any], cell: str) -> int:
    """Return one positive effective timeout."""
    timeouts = _object(resolved.get("timeouts_s"), "timeouts_s")
    timeout = timeouts.get(cell)
    if isinstance(timeout, bool) or not isinstance(timeout, int) or timeout <= 0:
        raise CellPlanError(f"required timeout is unavailable for {cell}")
    return timeout


def _plan_graph_cell(
    resolved: dict[str, Any],
    repo_root: Path,
    source_root: Path,
    output_root: Path,
    *,
    cell: str,
    config_key: str,
    adapter_name: str,
) -> dict[str, object]:
    """Plan one graph smoke through its thin output-local adapter."""
    del source_root
    configs = _object(resolved.get("configs"), "configs")
    config_binding = _required_binding(configs, config_key, "configs")
    config = _verified_file_binding(config_binding, f"configs.{config_key}")
    python, python_sha = _runtime_file(resolved, "python")
    cli, cli_sha = _runtime_file(resolved, "fathomdb_cli")
    graph_output = _asset_group(resolved, "graph_output")
    external = _binding_path(
        _required_binding(graph_output, "external_output_root", "assets.graph_output"),
        "assets.graph_output.external_output_root",
    )
    if external.is_relative_to(repo_root):
        raise CellPlanError("graph external output root must be outside the repository")
    adapter = _required_repo_file(repo_root, f"scripts/perf-experiments/{adapter_name}")
    artifact_root = external / output_root.name / cell
    result = output_root / "cells" / cell / "benchmark-result.json"
    argv = [
        str(python),
        str(adapter),
        "--config",
        str(config),
        "--artifact-root",
        str(artifact_root),
        "--result",
        str(result),
        "--fathomdb-bin",
        str(cli),
    ]
    return _base_plan(
        cell=cell,
        adapter=str(adapter),
        cwd=repo_root,
        env={
            "PYTHONPATH": f"{repo_root}:{repo_root / 'src/python'}",
        },
        unset_env=[],
        invocations=[{"label": "native-smoke", "argv": argv}],
        inputs={
            "source": resolved["source"],
            "config": {
                "path": str(config),
                "sha256": config_binding["sha256"],
            },
            "runtime": {
                "python": {"path": str(python), "sha256": python_sha},
                "fathomdb_cli": {"path": str(cli), "sha256": cli_sha},
            },
        },
        outputs={
            "result": str(result),
            "artifact_root": str(artifact_root),
        },
        timeout_s=_timeout(resolved, cell),
        workload_identity={
            "mode": "no-paid-model-native-smoke",
            "config_sha256": config_binding["sha256"],
        },
    )


def _plan_graph_evidence(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    return _plan_graph_cell(
        resolved,
        repo_root,
        source_root,
        output_root,
        cell="graph-evidence01",
        config_key="graph_evidence",
        adapter_name="run-graph-evidence01.py",
    )


def _plan_graph_expand(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    return _plan_graph_cell(
        resolved,
        repo_root,
        source_root,
        output_root,
        cell="graph-expand01",
        config_key="graph_expand",
        adapter_name="run-graph-expand01.py",
    )


def _plan_graph_retrieval(
    resolved: dict[str, Any], repo_root: Path, source_root: Path, output_root: Path
) -> dict[str, object]:
    return _plan_graph_cell(
        resolved,
        repo_root,
        source_root,
        output_root,
        cell="graph-retrieval01",
        config_key="graph_retrieval",
        adapter_name="run-graph-retrieval01.py",
    )


def build_execution_map(
    resolved: object, repo_root: str | Path
) -> list[dict[str, object]]:
    """Build implemented Slice 30 plans in canonical configured order."""
    document = _object(resolved, "resolved configuration")
    if document.get("schema_version") != RESOLVED_SCHEMA:
        raise CellPlanError("resolved configuration schema is unsupported")
    source = _object(document.get("source"), "source")
    source_value = source.get("root")
    output_value = document.get("output_root")
    cells = document.get("cells")
    if not isinstance(source_value, str) or not isinstance(output_value, str):
        raise CellPlanError("required source/output paths are unavailable")
    try:
        source_root = Path(source_value).resolve(strict=True)
    except OSError as error:
        raise CellPlanError(
            f"required target source is unavailable: {error}"
        ) from error
    output_root = Path(output_value).resolve(strict=False)
    if not isinstance(cells, list) or any(
        cell not in SUPPORTED_CELLS for cell in cells
    ):
        raise CellPlanError("selected cell is not implemented")
    planners = {
        "ac076": _plan_ac076,
        "ac072": _plan_ac072,
        "ac081": _plan_ac081,
        "ac073": _plan_ac073,
        "ac075": _plan_ac075,
        "scale02": _plan_scale02,
        "protected-writes": _plan_protected_writes,
        "ce-profile": _plan_ce_profile,
        "search01": _plan_search01,
        "locomo": _plan_locomo,
        "ac013-scale-matrix": _plan_ac013_scale_matrix,
        "graph-evidence01": _plan_graph_evidence,
        "graph-expand01": _plan_graph_expand,
        "graph-retrieval01": _plan_graph_retrieval,
    }
    root = Path(repo_root).resolve()
    return [planners[cell](document, root, source_root, output_root) for cell in cells]
