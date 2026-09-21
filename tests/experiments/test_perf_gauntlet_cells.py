"""Contract tests for Performance Gauntlet v1.2 Slice 30A plans."""

from __future__ import annotations

import hashlib
import json
import runpy
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
PLANNER = ROOT / "scripts" / "perf-experiments" / "gauntlet_cells.py"
PLAN_KEYS = {
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
}
EU7_CORPUS_SHA256 = "af1484a4873e61d98647ea44ab3cb452a3b42d9d7ccd32babb68f50ba531b65e"
EU7_MODEL_SHA256 = {
    "config.json": "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750",
    "tokenizer.json": "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
    "model.safetensors": "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad",
}
TC5_BASE = ROOT / "experiments" / "configs" / "scale-01" / "tc5-gpu-v2.json"
SCALE02_BASE = ROOT / "experiments" / "configs" / "scale-02" / "a0-envelope.v2.json"
TC5_BASE_SHA256 = "b2ea5c25eee0b93807384259262702d3bb04f1fee4f640579160091ecee2417c"
SCALE02_BASE_SHA256 = "eb86d5b41e63b4854bde695200b9a0b9552a5c2474650f7cb0762b862851cd28"
TC5_CORPUS_INDEX_SHA256 = (
    "624b6b42e7e1d40866fea48d37a34b0ef8d786f46f4aa2ed9a3dd245d22a0a0a"
)
TC5_QUALIFIED_SHA256 = (
    "f6180a00d1551a143a7445aa6ed28ed589533400ee702835663729694e393df5"
)


def load_planner() -> dict[str, object]:
    return runpy.run_path(str(PLANNER))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def valid_resolved_plan(tmp_path: Path) -> dict[str, object]:
    source = tmp_path / "target-source"
    raw_dir = tmp_path / "corpus" / "raw"
    raw_dir.mkdir(parents=True)
    for index in range(13):
        (raw_dir / f"source-{index:02d}.jsonl").write_text(
            json.dumps({"doc_id": str(index), "body": "fixture"}) + "\n",
            encoding="utf-8",
        )
    (source / "data").mkdir(parents=True)
    (source / "data" / "corpus-data").symlink_to(
        raw_dir.parent, target_is_directory=True
    )

    model_cache = tmp_path / "embedder-cache"
    model_dir = model_cache / "0b2926f8a9b1"
    model_dir.mkdir(parents=True)
    for name in ("config.json", "tokenizer.json", "model.safetensors"):
        path = model_dir / name
        path.write_text(f"fixture-{name}\n", encoding="utf-8")

    closure_manifest = tmp_path / "slice75-closure-manifest.json"
    closure_manifest.write_text(
        json.dumps(
            {
                "schema_version": "fathomdb.slice75-closure-manifest/v1",
                "release": "0.8.25",
                "external_inputs_by_cell": {
                    "model-cache": {
                        f"${{FATHOMDB_EMBEDDER_CACHE}}/0b2926f8a9b1/{name}": digest
                        for name, digest in EU7_MODEL_SHA256.items()
                    },
                    "eu7-real": {"inherit": "model-cache"},
                    "eu7-corpus": {
                        "path_glob": "data/corpus-data/raw/*.jsonl",
                        "file_count": 13,
                        "canonical_sha256": EU7_CORPUS_SHA256,
                        "consumer": "eu7-real",
                    },
                },
                "cells": [
                    {
                        "id": "eu7-real",
                        "timeout_seconds": 10800,
                        "unset_environment": [
                            "FATHOMDB_SKIP_NETWORK_TESTS",
                            "EU7_FORCE_FULL_RECOMPUTE",
                        ],
                        "commands": [
                            "env AGENT_LONG=1 EU7_N_VALUES=7667 EU7_QUERIES=100 "
                            "EU7_BOOTSTRAP=1000 EU7_LATENCY_SAMPLES=1000 "
                            "EU7_STRESS_PER_THREAD=250 "
                            "FATHOMDB_EU7_OUTPUT=${RUN_DIR}/eu7.json cargo test "
                            "--release -p fathomdb-engine --features "
                            "operator,default-embedder --test eu7_real_corpus_ac "
                            "eu7_real_corpus_ac_validation -- --exact --ignored "
                            "--nocapture --test-threads=1"
                        ],
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    ac072_config = tmp_path / "ac072.json"
    ac072_config.write_text("{}\n", encoding="utf-8")
    output_root = tmp_path / "gauntlet-output"
    return {
        "schema_version": "fathomdb.performance-gauntlet.resolved-config/v1",
        "release": "0.8.26",
        "source": {"root": str(source.resolve()), "commit": "a" * 40},
        "source_config": {"path": str(tmp_path / "gauntlet.json"), "sha256": "b" * 64},
        "output_root": str(output_root),
        "runtime": {},
        "configs": {
            "ac072": {"path": str(ac072_config), "sha256": sha256(ac072_config)},
            "ac073": {
                "path": str(closure_manifest),
                "sha256": sha256(closure_manifest),
            },
        },
        "assets": {
            "ac073": {"corpus_root": {"path": str(raw_dir.parent.resolve())}},
            "tc5": {
                "corpus_root": {"path": str(tmp_path / "unused-tc5")},
                "qualified_manifest": None,
                "embedder_model_cache": {"path": str(model_cache.resolve())},
            },
        },
        "gpu": {
            "enabled": True,
            "cuda_uuid": "GPU-11111111-1111-1111-1111-111111111111",
        },
        "cells": ["ac076", "ac072", "ac081", "ac073"],
        "timeouts_s": {
            "ac076": 3600,
            "ac072": 10800,
            "ac081": 10800,
            "ac073": 10800,
        },
    }


def prepared_plan(tmp_path: Path) -> tuple[dict[str, object], dict[str, object]]:
    resolved = valid_resolved_plan(tmp_path)
    planner = load_planner()
    namespace = planner["build_execution_map"].__globals__
    original_sha = namespace["_sha256"]
    original_corpus = namespace["_canonical_corpus_sha"]
    model_cache = Path(
        resolved["assets"]["tc5"]["embedder_model_cache"]["path"]
    ).resolve()

    def fixture_sha(path: Path) -> str:
        if model_cache in path.parents:
            expected_content = f"fixture-{path.name}\n"
            if path.read_text(encoding="utf-8") == expected_content:
                return EU7_MODEL_SHA256[path.name]
        return original_sha(path)

    def fixture_corpus(raw_dir: Path) -> tuple[int, str]:
        paths = sorted(raw_dir.glob("*.jsonl"))
        if len(paths) == 13 and all("fixture" in path.read_text() for path in paths):
            return 13, EU7_CORPUS_SHA256
        return original_corpus(raw_dir)

    namespace["_sha256"] = fixture_sha
    namespace["_canonical_corpus_sha"] = fixture_corpus
    return planner, resolved


def prepared_30b_plan(tmp_path: Path) -> tuple[dict[str, object], dict[str, object]]:
    planner = load_planner()
    source = tmp_path / "target-source"
    source.mkdir(parents=True)
    output_root = tmp_path / "gauntlet-output"

    configs = tmp_path / "configs"
    configs.mkdir()
    ac075 = configs / "tc5-gpu-v2.json"
    scale02 = configs / "a0-envelope.v2.json"
    ac075.write_bytes(TC5_BASE.read_bytes())
    scale02.write_bytes(SCALE02_BASE.read_bytes())

    runtime_root = tmp_path / "runtime"
    runtime_root.mkdir()
    runtime: dict[str, dict[str, str]] = {}
    for key, name in (
        ("fathomdb_cli", "fathomdb"),
        ("python", "python"),
        ("wheel", "fathomdb.whl"),
        ("native_extension", "_fathomdb.so"),
    ):
        path = runtime_root / name
        path.write_text(f"fixture-{name}\n", encoding="utf-8")
        runtime[key] = {"path": str(path.resolve()), "sha256": sha256(path)}
    virtualenv = runtime_root / "venv"
    virtualenv.mkdir()
    runtime["virtualenv"] = {"path": str(virtualenv.resolve())}

    corpus = tmp_path / "tc5-corpus"
    corpus.mkdir()
    index = corpus / "tc5-corpus-input.v1.json"
    index.write_text("fixture-index\n", encoding="utf-8")
    qualified = tmp_path / "tc5-qualified.json"
    qualified.write_text("fixture-qualified\n", encoding="utf-8")
    cache = tmp_path / "embedder-cache"
    model = cache / "0b2926f8a9b1"
    model.mkdir(parents=True)
    external = tmp_path / "external" / "locomo-raw"
    external.mkdir(parents=True)

    resolved = {
        "schema_version": "fathomdb.performance-gauntlet.resolved-config/v1",
        "release": "0.8.26",
        "source": {"root": str(source.resolve()), "commit": "a" * 40},
        "source_config": {"path": str(tmp_path / "gauntlet.json"), "sha256": "b" * 64},
        "output_root": str(output_root.resolve()),
        "runtime": runtime,
        "configs": {
            "ac075": {"path": str(ac075.resolve()), "sha256": sha256(ac075)},
            "scale02": {"path": str(scale02.resolve()), "sha256": sha256(scale02)},
        },
        "assets": {
            "tc5": {
                "corpus_root": {"path": str(corpus.resolve())},
                "qualified_manifest": {
                    "path": str(qualified.resolve()),
                    "sha256": TC5_QUALIFIED_SHA256,
                },
                "embedder_model_cache": {"path": str(cache.resolve())},
            },
            "locomo": {
                "external_output_root": {"path": str(external.resolve())}
            },
        },
        "gpu": {
            "enabled": True,
            "cuda_uuid": "GPU-11111111-1111-1111-1111-111111111111",
        },
        "cells": ["ac075", "scale02"],
        "timeouts_s": {"ac075": 10800, "scale02": 14400},
    }
    namespace = planner["build_execution_map"].__globals__
    original_sha = namespace["_sha256"]

    def fixture_sha(path: Path) -> str:
        if path.resolve() == index.resolve() and path.read_text() == "fixture-index\n":
            return TC5_CORPUS_INDEX_SHA256
        if (
            path.resolve() == qualified.resolve()
            and path.read_text() == "fixture-qualified\n"
        ):
            return TC5_QUALIFIED_SHA256
        return original_sha(path)

    namespace["_sha256"] = fixture_sha
    return planner, resolved


def test_30a_map_has_exact_cells_order_and_closed_shape(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)

    plans = planner["build_execution_map"](resolved, ROOT)

    assert [plan["cell"] for plan in plans] == ["ac076", "ac072", "ac081", "ac073"]
    assert all(set(plan) == PLAN_KEYS for plan in plans)
    json.dumps(plans)


def test_ac076_delegates_once_with_historical_environment(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    plan = planner["build_execution_map"](resolved, ROOT)[0]

    assert plan["adapter"].endswith("scripts/perf-experiments/run-ac012.sh")
    assert plan["env"] == {
        "AGENT_LONG": "1",
        "AC012_CORPUS_N": "10000",
        "LOG_PATH": str(tmp_path / "gauntlet-output" / "cells" / "ac076" / "ac012.log"),
    }
    assert plan["unset_env"] == ["AC_FULL_SCALE"]
    assert plan["invocations"] == [
        {"label": "run-1", "argv": ["bash", plan["adapter"]]}
    ]
    assert plan["workload_identity"] == {
        "selector": "ac_012_text_query_latency_on_fts5_path",
        "corpus_n": 10000,
        "repetitions": 1,
    }


def test_ac072_uses_exact_dispatcher_contract_and_six_hashes(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    plan = planner["build_execution_map"](resolved, ROOT)[1]
    argv = plan["invocations"][0]["argv"]

    assert argv[:2] == ["bash", plan["adapter"]]
    assert len(argv[2:]) == 10
    assert argv[3] == "artifact:perf_gates.path"
    assert argv[5] == "a" * 40
    assert argv[6:8] == [
        "artifact:perf_gates.sha256",
        "artifact:perf_gates.input_sha256",
    ]
    assert all(len(value) == 64 for value in argv[8:12])
    assert plan["workload_identity"] == {
        "corpus_n": 10000,
        "vector_dim": 384,
        "queries": 1000,
        "treatment": "warm",
        "repetitions": 3,
        "dispatcher_labels": ["R1", "R2", "R3"],
    }
    assert plan["inputs"]["ac072_config"]["usage"] == "provenance-only"


def test_ac081_has_seven_fresh_process_invocations(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    plan = planner["build_execution_map"](resolved, ROOT)[2]

    assert [item["label"] for item in plan["invocations"]] == [
        f"R{index}" for index in range(1, 8)
    ]
    assert len({item["argv"][4] for item in plan["invocations"]}) == 7
    assert all(
        item["argv"][3] == "artifact:perf_gates.path" for item in plan["invocations"]
    )
    assert all(
        item["argv"][6:]
        == [
            "artifact:perf_gates.sha256",
            "artifact:perf_gates.input_sha256",
        ]
        for item in plan["invocations"]
    )
    assert plan["env"]["SLICE80_AC081_SCANNER"].endswith(
        "dev/tools/slice80_read_acceptance.py"
    )
    assert len(plan["inputs"]["scanner"]["sha256"]) == 64
    assert plan["workload_identity"]["repetitions"] == 7
    assert plan["workload_identity"]["selector"] == "ac_081_absolute_read_performance"


def test_ac073_loads_frozen_command_and_binds_real_inputs(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    plan = planner["build_execution_map"](resolved, ROOT)[3]

    assert plan["timeout_s"] == 10800
    assert plan["unset_env"] == [
        "FATHOMDB_SKIP_NETWORK_TESTS",
        "EU7_FORCE_FULL_RECOMPUTE",
    ]
    assert plan["env"]["FATHOMDB_EMBEDDER_CACHE"].endswith("embedder-cache")
    assert plan["env"]["FATHOMDB_EMBED_DEVICE"] == "cuda:0"
    assert plan["env"]["CUDA_VISIBLE_DEVICES"] == (
        "GPU-11111111-1111-1111-1111-111111111111"
    )
    assert plan["env"]["FATHOMDB_EU7_OUTPUT"].endswith("cells/ac073/eu7.json")
    assert plan["env"]["EU7_N_VALUES"] == "7667"
    assert plan["invocations"][0]["argv"][0:4] == [
        "cargo",
        "test",
        "--release",
        "-p",
    ]
    assert "operator,embed-cuda" in plan["invocations"][0]["argv"]
    assert "operator,default-embedder" not in plan["invocations"][0]["argv"]
    assert plan["workload_identity"] == {
        "documents": 7667,
        "queries": 100,
        "bootstrap_samples": 1000,
        "latency_samples": 1000,
        "stress_operations_per_thread": 250,
        "corpus_files": 13,
        "corpus_sha256": plan["inputs"]["corpus"]["sha256"],
        "encoding_device": "cuda:0",
        "cuda_uuid": "GPU-11111111-1111-1111-1111-111111111111",
        "repetitions": 1,
    }


@pytest.mark.parametrize(
    "mutation",
    [
        lambda plan: plan["configs"].update({"ac072": None}),
        lambda plan: plan["configs"].update({"ac073": None}),
        lambda plan: plan["assets"].update({"ac073": None}),
        lambda plan: plan["assets"]["tc5"].update({"embedder_model_cache": None}),
    ],
)
def test_30a_rejects_missing_required_bindings(
    tmp_path: Path, mutation: object
) -> None:
    planner, resolved = prepared_plan(tmp_path)
    mutation(resolved)

    with pytest.raises(planner["CellPlanError"], match="required"):
        planner["build_execution_map"](resolved, ROOT)


def test_ac073_rejects_disconnected_or_drifted_corpus(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    disconnected = tmp_path / "disconnected-corpus"
    disconnected.mkdir()
    resolved["assets"]["ac073"]["corpus_root"]["path"] = str(disconnected)

    with pytest.raises(planner["CellPlanError"], match="canonical-resolve"):
        planner["build_execution_map"](resolved, ROOT)

    planner, resolved = prepared_plan(tmp_path / "drift")
    raw = Path(resolved["assets"]["ac073"]["corpus_root"]["path"]) / "raw"
    next(raw.glob("*.jsonl")).write_text("drift\n", encoding="utf-8")
    with pytest.raises(planner["CellPlanError"], match="corpus identity"):
        planner["build_execution_map"](resolved, ROOT)


def test_ac073_rejects_model_cache_hash_drift(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    cache = Path(resolved["assets"]["tc5"]["embedder_model_cache"]["path"])
    (cache / "0b2926f8a9b1" / "config.json").write_text("drift\n", encoding="utf-8")

    with pytest.raises(planner["CellPlanError"], match="model cache"):
        planner["build_execution_map"](resolved, ROOT)


def test_ac073_rejects_model_cache_contract_drift(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    manifest_path = Path(resolved["configs"]["ac073"]["path"])
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    model_cache = manifest["external_inputs_by_cell"]["model-cache"]
    model_cache["${FATHOMDB_EMBEDDER_CACHE}/0b2926f8a9b1/config.json"] = "c" * 64
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    resolved["configs"]["ac073"]["sha256"] = sha256(manifest_path)

    with pytest.raises(planner["CellPlanError"], match="model cache contract"):
        planner["build_execution_map"](resolved, ROOT)


def test_ac073_canonical_corpus_hash_matches_frozen_shell_algorithm(
    tmp_path: Path,
) -> None:
    planner = load_planner()
    raw_dir = tmp_path / "raw"
    raw_dir.mkdir()
    for name, body in (("b.jsonl", b"second\n"), ("a.jsonl", b"first\n")):
        (raw_dir / name).write_bytes(body)
    expected_lines = "".join(
        f"{hashlib.sha256((raw_dir / name).read_bytes()).hexdigest()}  "
        f"data/corpus-data/raw/{name}\n"
        for name in ("a.jsonl", "b.jsonl")
    )

    assert planner["_canonical_corpus_sha"](raw_dir) == (
        2,
        hashlib.sha256(expected_lines.encode()).hexdigest(),
    )


def test_ac073_rejects_frozen_identity_and_timeout_drift(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    manifest_path = Path(resolved["configs"]["ac073"]["path"])
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["external_inputs_by_cell"]["eu7-corpus"]["canonical_sha256"] = "c" * 64
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    resolved["configs"]["ac073"]["sha256"] = sha256(manifest_path)
    with pytest.raises(planner["CellPlanError"], match="corpus contract"):
        planner["build_execution_map"](resolved, ROOT)

    planner, resolved = prepared_plan(tmp_path / "timeout")
    resolved["timeouts_s"]["ac073"] = 1
    with pytest.raises(planner["CellPlanError"], match="timeout"):
        planner["build_execution_map"](resolved, ROOT)


def test_ac073_rejects_extra_run_dir_substitution(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    manifest_path = Path(resolved["configs"]["ac073"]["path"])
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    command = manifest["cells"][0]["commands"][0]
    manifest["cells"][0]["commands"][0] = command.replace(
        "env ", "env EXTRA=${RUN_DIR}/extra ", 1
    )
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    resolved["configs"]["ac073"]["sha256"] = sha256(manifest_path)

    with pytest.raises(planner["CellPlanError"], match="substitution"):
        planner["build_execution_map"](resolved, ROOT)


def test_ac073_rejects_benchmark_command_argv_drift(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    manifest_path = Path(resolved["configs"]["ac073"]["path"])
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    command = manifest["cells"][0]["commands"][0]
    manifest["cells"][0]["commands"][0] = command[: command.index("cargo ")] + (
        "cargo --version"
    )
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    resolved["configs"]["ac073"]["sha256"] = sha256(manifest_path)

    with pytest.raises(planner["CellPlanError"], match="argv drifted"):
        planner["build_execution_map"](resolved, ROOT)


def test_ac073_constants_pin_frozen_authority() -> None:
    planner = load_planner()

    assert planner["EU7_CORPUS_SHA256"] == EU7_CORPUS_SHA256
    assert planner["EU7_MODEL_SHA256"] == EU7_MODEL_SHA256


def test_ac081_runner_resolves_scanner_from_its_own_checkout() -> None:
    source = (
        ROOT / "scripts" / "perf-experiments" / "run-slice80-ac081-cell.sh"
    ).read_text()

    assert "SLICE80_AC081_SCANNER" in source
    assert 'python3 "$scanner"' in source


def test_30a_planner_contains_no_benchmark_or_scoring_implementation() -> None:
    source = PLANNER.read_text(encoding="utf-8")

    assert "subprocess" not in source
    assert "numpy" not in source
    assert "percentile" not in source
    assert "def _bootstrap" not in source


def test_optional_scale_matrix_delegates_to_existing_runner(tmp_path: Path) -> None:
    planner, resolved = prepared_plan(tmp_path)
    resolved["cells"] = ["ac013-scale-matrix"]
    resolved["timeouts_s"] = {"ac013-scale-matrix": 3600}

    plan = planner["build_execution_map"](resolved, ROOT)[0]

    assert plan["cell"] == "ac013-scale-matrix"
    assert plan["adapter"].endswith("run-scale-ac013-matrix.sh")
    assert plan["invocations"] == [
        {"label": "matrix", "argv": ["bash", plan["adapter"]]}
    ]
    assert plan["workload_identity"] == {
        "rows": [10000, 100000, 1000000],
        "treatments": ["process_cold", "warm"],
        "repetitions": 5,
        "default": False,
    }
def test_30b_maps_tc5_and_scale_in_canonical_order(tmp_path: Path) -> None:
    planner, resolved = prepared_30b_plan(tmp_path)

    plans = planner["build_execution_map"](resolved, ROOT)

    assert [plan["cell"] for plan in plans] == ["ac075", "scale02"]
    assert all(set(plan) == PLAN_KEYS for plan in plans)
    json.dumps(plans)


def test_ac075_uses_frozen_base_and_two_stage_artifact_template(tmp_path: Path) -> None:
    planner, resolved = prepared_30b_plan(tmp_path)
    plan = planner["build_execution_map"](resolved, ROOT)[0]
    template = plan["inputs"]["overlay_template"]
    document = template["document"]

    assert plan["cwd"] == str(ROOT)
    assert plan["invocations"] == [
        {
            "label": "bridge",
            "argv": [
                resolved["runtime"]["python"]["path"],
                "-m",
                "experiments.tc5_gpu_v2",
                "run",
                "--config",
                "artifact:ac075_overlay.path",
                "--arm",
                "bridge",
                "--output-root",
                str(tmp_path / "gauntlet-output" / "cells" / "ac075" / "run"),
                "--binary",
                "artifact:tc5_benchmark.path",
            ],
        }
    ]
    assert plan["inputs"]["base_config"]["sha256"] == TC5_BASE_SHA256
    assert document["release"] == "0.8.26"
    assert document["candidate"]["version"] == "0.8.26"
    assert document["candidate"]["package_version"] == "0.8.26"
    assert document["candidate"]["benchmark_binary"] == "artifact:tc5_benchmark.path"
    assert document["candidate"]["benchmark_binary_sha256"] == (
        "artifact:tc5_benchmark.sha256"
    )
    assert template["changed_pointers"] == [
        "/candidate",
        "/inputs/corpus_root",
        "/inputs/model_asset_directory",
        "/inputs/qualified_manifest",
        "/release",
        "/runtime/cuda_uuid",
        "/runtime/fathomdb_bin",
        "/runtime/python",
    ]
    assert set(plan["inputs"]["modules"]) == {"runner", "test_setup"}
    assert plan["inputs"]["final_overlay"] == {
        "path": "artifact:ac075_overlay.path",
        "sha256": "artifact:ac075_overlay.sha256",
    }


def test_scale02_overlay_and_historical_point_modes_are_exact(tmp_path: Path) -> None:
    planner, resolved = prepared_30b_plan(tmp_path)
    plan = planner["build_execution_map"](resolved, ROOT)[1]
    overlay = plan["inputs"]["overlay"]

    assert plan["cwd"] == str(ROOT)
    assert plan["inputs"]["base_config"]["sha256"] == SCALE02_BASE_SHA256
    assert overlay["document"]["release"] == "0.8.26"
    assert overlay["changed_pointers"] == [
        "/corpus/qualified_manifest",
        "/corpus/root",
        "/release",
        "/runtime/fathomdb_bin",
        "/runtime/fathomdb_bin_sha256",
        "/runtime/python",
        "/runtime/python_extension",
        "/runtime/python_extension_sha256",
        "/runtime/python_package_version",
    ]
    assert [item["label"] for item in plan["invocations"]] == [
        "point-10000",
        "point-17272",
        "point-25000",
        "point-40000",
        "point-50000",
    ]
    for index, item in enumerate(plan["invocations"]):
        assert item["argv"][:4] == [
            resolved["runtime"]["python"]["path"],
            "-m",
            "experiments.scale_02",
            "run-point",
        ]
        assert "--record-base-dir" in item["argv"]
        assert ("--post-boundary-baseline" in item["argv"]) is (index >= 3)
    assert set(plan["inputs"]["modules"]) == {"runner", "lib", "test_setup"}


def test_30b_rejects_non_frozen_base_or_external_input_drift(tmp_path: Path) -> None:
    planner, resolved = prepared_30b_plan(tmp_path)
    config = Path(resolved["configs"]["ac075"]["path"])
    document = json.loads(config.read_text())
    document["measurement"]["query_count"] = 99
    config.write_text(json.dumps(document), encoding="utf-8")
    resolved["configs"]["ac075"]["sha256"] = sha256(config)

    with pytest.raises(planner["CellPlanError"], match="frozen TC-5 base"):
        planner["build_execution_map"](resolved, ROOT)

    planner, resolved = prepared_30b_plan(tmp_path / "input-drift")
    qualified = Path(resolved["assets"]["tc5"]["qualified_manifest"]["path"])
    qualified.write_text("drift\n", encoding="utf-8")
    with pytest.raises(planner["CellPlanError"], match="identity"):
        planner["build_execution_map"](resolved, ROOT)
