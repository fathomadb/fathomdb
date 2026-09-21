"""Contract tests for the Performance Gauntlet v1.2 entry point."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import runpy
import subprocess
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / "scripts" / "perf-experiments" / "run_gauntlet.py"
WRAPPER = ROOT / "scripts" / "perf-experiments" / "run-gauntlet.sh"
README = ROOT / "scripts" / "perf-experiments" / "README.md"
AGENT_TEST = ROOT / "scripts" / "agent-test.sh"
EXAMPLE_CONFIG = (
    ROOT / "experiments" / "configs" / "gauntlet" / "directional-release.example.json"
)
EXPECTED_CELLS = (
    "ac076",
    "ac072",
    "ac081",
    "ac073",
    "ac075",
    "scale02",
    "protected-writes",
    "ce-profile",
    "search01",
    "locomo",
)


def load_driver() -> dict[str, object]:
    return runpy.run_path(str(DRIVER))


def common_args() -> list[str]:
    return [
        "--release",
        "0.8.26",
        "--source-root",
        "missing-source",
        "--config",
        "missing-config.json",
        "--output-root",
        "missing-output",
    ]


def run_driver(*args: str, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(DRIVER), *args],
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def _touch(path: Path, content: str | None = None) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content or path.name, encoding="utf-8")
    return str(path)


def valid_configuration(tmp_path: Path) -> tuple[dict[str, object], Path]:
    tmp_path.mkdir(parents=True, exist_ok=True)
    source = tmp_path / "source"
    source.mkdir()
    virtualenv = tmp_path / "venv"
    virtualenv.mkdir()
    data_root = tmp_path / "ir-c-data"
    data_root.mkdir()
    ac073_corpus = tmp_path / "ac073-corpus"
    ac073_corpus.mkdir()
    tc5_corpus = tmp_path / "tc5-corpus"
    tc5_corpus.mkdir()
    embedder_cache = tmp_path / "embedder-cache"
    embedder_cache.mkdir()
    locomo_checkout = tmp_path / "locomo-checkout"
    locomo_checkout.mkdir()
    locomo_external = tmp_path / "locomo-external"
    locomo_external.mkdir()
    reranker_cache = tmp_path / "reranker-cache"
    reranker_cache.mkdir()

    runtime = {
        "fathomdb_cli": _touch(tmp_path / "runtime" / "fathomdb"),
        "python": _touch(tmp_path / "runtime" / "python"),
        "wheel": _touch(tmp_path / "runtime" / "fathomdb.whl"),
        "native_extension": _touch(tmp_path / "runtime" / "_fathomdb.so"),
        "virtualenv": str(virtualenv),
    }
    configs = {
        name: _touch(tmp_path / "configs" / f"{name}.json")
        for name in (
            "ac072",
            "ac073",
            "ac075",
            "scale02",
            "protected_writes",
            "ce_profile",
            "search01",
            "locomo",
        )
    }
    document: dict[str, object] = {
        "schema_version": "fathomdb.performance-gauntlet.config/v1",
        "release": "0.8.26",
        "source": {"root": str(source), "commit": "a" * 40},
        "output_root": str(tmp_path / "output"),
        "runtime": runtime,
        "configs": configs,
        "assets": {
            "ir_c": {
                "data_root": str(data_root),
                "snapshot": _touch(tmp_path / "assets" / "snapshot.json"),
                "manifest": _touch(tmp_path / "assets" / "manifest.json"),
                "gold": _touch(tmp_path / "assets" / "gold.json"),
            },
            "ac073": {"corpus_root": str(ac073_corpus)},
            "tc5": {
                "corpus_root": str(tc5_corpus),
                "qualified_manifest": _touch(
                    tmp_path / "assets" / "tc5-qualified.json"
                ),
                "embedder_model_cache": str(embedder_cache),
            },
            "locomo": {
                "harness_checkout": str(locomo_checkout),
                "harness_python": _touch(tmp_path / "locomo" / "python"),
                "dataset": _touch(tmp_path / "locomo" / "dataset.json"),
                "provenance_manifest": _touch(tmp_path / "locomo" / "provenance.json"),
                "external_output_root": str(locomo_external),
            },
            "ce_profile": {"reranker_model_cache": str(reranker_cache)},
        },
        "gpu": {"enabled": False, "cuda_uuid": None},
        "cells": ["ac076", "search01", "locomo"],
        "timeouts_s": {"default": 300, "cells": {"search01": 900}},
    }
    config_path = tmp_path / "gauntlet.json"
    config_path.write_text(json.dumps(document), encoding="utf-8")
    return document, config_path


def test_default_cells_are_exact_and_stably_ordered() -> None:
    driver = load_driver()

    assert driver["DEFAULT_CELLS"] == EXPECTED_CELLS
    assert driver["parse_cell_selection"](None) == EXPECTED_CELLS


def test_optional_scale_matrix_is_selectable_but_not_in_default() -> None:
    driver = load_driver()

    assert "ac013-scale-matrix" not in driver["DEFAULT_CELLS"]
    assert driver["parse_cell_selection"]("ac013-scale-matrix") == (
        "ac013-scale-matrix",
    )


def test_subset_is_trimmed_and_returned_in_canonical_order() -> None:
    driver = load_driver()

    selected = driver["parse_cell_selection"](" locomo, ac076,scale02 ")

    assert selected == ("ac076", "scale02", "locomo")


@pytest.mark.parametrize(
    "selection",
    ["", "ac076,,ac072", "ac076,ac076", "ac076,unknown"],
)
def test_invalid_cell_selections_are_rejected(selection: str) -> None:
    driver = load_driver()

    with pytest.raises(driver["GauntletArgumentError"]):
        driver["parse_cell_selection"](selection)


def test_help_needs_no_external_inputs() -> None:
    result = run_driver("--help")

    assert result.returncode == 0
    assert "--dry-run" in result.stdout
    assert "--preflight-only" in result.stdout
    assert "--cells" in result.stdout


def test_dry_run_is_metadata_only_and_does_not_create_output(tmp_path: Path) -> None:
    output = tmp_path / "must-not-exist"
    args = common_args()
    args[-1] = str(output)

    result = run_driver(*args, "--dry-run", "--resume", "--fail-fast")

    assert result.returncode == 0, result.stderr
    projection = json.loads(result.stdout)
    assert projection == {
        "cells": list(EXPECTED_CELLS),
        "config": "missing-config.json",
        "fail_fast": True,
        "mode": "dry-run",
        "output_root": str(output),
        "release": "0.8.26",
        "resume": True,
        "schema_version": "fathomdb.performance-gauntlet.plan/v1",
        "source_root": "missing-source",
    }
    assert not output.exists()


def test_conflicting_modes_fail_explicitly() -> None:
    result = run_driver(*common_args(), "--dry-run", "--preflight-only")

    assert result.returncode == 2
    assert "cannot be combined" in result.stderr
    assert result.stdout == ""


def test_wrapper_preserves_caller_working_directory_and_relative_arguments(
    tmp_path: Path,
) -> None:
    assert os.access(WRAPPER, os.X_OK)
    result = subprocess.run(
        [
            str(WRAPPER),
            *common_args(),
            "--dry-run",
            "--cells",
            "locomo,ac076",
        ],
        cwd=tmp_path,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    assert result.returncode == 0, result.stderr
    projection = json.loads(result.stdout)
    assert projection["source_root"] == "missing-source"
    assert projection["config"] == "missing-config.json"
    assert projection["output_root"] == "missing-output"
    assert projection["cells"] == ["ac076", "locomo"]
    assert not (tmp_path / "missing-output").exists()


def test_wrapper_is_thin_and_verification_discovers_the_contract_test() -> None:
    wrapper = WRAPPER.read_text(encoding="utf-8")
    readme = README.read_text(encoding="utf-8")
    agent_test = AGENT_TEST.read_text(encoding="utf-8")

    assert "set -euo pipefail" in wrapper
    assert "BASH_SOURCE" in wrapper
    assert "exec python3" in wrapper
    assert "cargo " not in wrapper
    assert "run-gauntlet.sh" in readme
    assert "parse-ac012-numbers.py" not in readme
    assert "parse-ac020-numbers.py" not in readme
    assert "tests/experiments/test_perf_gauntlet.py" in agent_test


def test_checked_in_directional_example_is_strictly_valid() -> None:
    driver = load_driver()

    document = driver["load_configuration"](EXAMPLE_CONFIG)

    assert document["release"] == "0.8.26"
    assert document["cells"] == list(EXPECTED_CELLS)


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        (lambda value: value.update({"unexpected": True}), "unknown"),
        (lambda value: value.pop("runtime"), "missing"),
        (lambda value: value["source"].update({"branch": "main"}), "unknown"),
        (lambda value: value["assets"]["ir_c"].pop("manifest"), "missing"),
        (lambda value: value["timeouts_s"].update({"minutes": 1}), "unknown"),
    ],
)
def test_configuration_rejects_recursive_shape_drift(
    tmp_path: Path, mutation: object, message: str
) -> None:
    driver = load_driver()
    document, _ = valid_configuration(tmp_path)
    mutation(document)

    with pytest.raises(driver["GauntletConfigError"], match=message):
        driver["validate_configuration"](document)


@pytest.mark.parametrize(
    "cells",
    [[], ["locomo", "ac076"], ["ac076", "ac076"], ["unknown"]],
)
def test_configuration_requires_canonical_nonempty_cells(
    tmp_path: Path, cells: list[str]
) -> None:
    driver = load_driver()
    document, _ = valid_configuration(tmp_path)
    document["cells"] = cells

    with pytest.raises(driver["GauntletConfigError"], match="cells"):
        driver["validate_configuration"](document)


@pytest.mark.parametrize(
    ("gpu", "message"),
    [
        ({"enabled": True, "cuda_uuid": None}, "cuda_uuid"),
        (
            {"enabled": False, "cuda_uuid": "GPU-12345678-1234-1234-1234-123456789abc"},
            "disabled",
        ),
        ({"enabled": True, "cuda_uuid": "0"}, "cuda_uuid"),
    ],
)
def test_configuration_enforces_gpu_identity(
    tmp_path: Path, gpu: dict[str, object], message: str
) -> None:
    driver = load_driver()
    document, _ = valid_configuration(tmp_path)
    document["gpu"] = gpu

    with pytest.raises(driver["GauntletConfigError"], match=message):
        driver["validate_configuration"](document)


@pytest.mark.parametrize("timeout", [0, -1, True, 1.5])
def test_configuration_requires_positive_integer_timeouts(
    tmp_path: Path, timeout: object
) -> None:
    driver = load_driver()
    document, _ = valid_configuration(tmp_path)
    document["timeouts_s"] = {"default": timeout, "cells": {}}

    with pytest.raises(driver["GauntletConfigError"], match="positive integer"):
        driver["validate_configuration"](document)


def test_resolution_records_config_and_file_hashes_and_directory_paths(
    tmp_path: Path,
) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)

    resolved = driver["resolve_configuration"](config_path)

    assert (
        resolved["schema_version"] == "fathomdb.performance-gauntlet.resolved-config/v1"
    )
    assert resolved["source_config"]["path"] == str(config_path.resolve())
    assert len(resolved["source_config"]["sha256"]) == 64
    assert len(resolved["runtime"]["fathomdb_cli"]["sha256"]) == 64
    assert resolved["runtime"]["virtualenv"] == {
        "path": str((tmp_path / "venv").resolve())
    }
    assert resolved["assets"]["ir_c"]["data_root"] == {
        "path": str((tmp_path / "ir-c-data").resolve())
    }
    assert resolved["timeouts_s"] == {
        "ac076": 300,
        "search01": 900,
        "locomo": 300,
    }
    assert not Path(document["output_root"]).exists()


def test_resolution_preserves_null_groups_and_bindings(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    document["runtime"]["wheel"] = None
    document["assets"]["locomo"] = None
    config_path.write_text(json.dumps(document), encoding="utf-8")

    resolved = driver["resolve_configuration"](config_path)

    assert resolved["runtime"]["wheel"] is None
    assert resolved["assets"]["locomo"] is None


def test_resolution_preserves_virtualenv_python_launcher(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    interpreter = tmp_path / "runtime" / "python-real"
    interpreter.write_text("interpreter\n", encoding="utf-8")
    launcher = tmp_path / "venv" / "bin" / "python"
    launcher.parent.mkdir()
    launcher.symlink_to(interpreter)
    document["runtime"]["python"] = str(launcher)
    config_path.write_text(json.dumps(document), encoding="utf-8")

    resolved = driver["resolve_configuration"](config_path)

    assert resolved["runtime"]["python"]["path"] == str(launcher.absolute())


def test_resolution_preserves_locomo_harness_python_launcher(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    interpreter = tmp_path / "locomo" / "python-real"
    interpreter.write_text("interpreter\n", encoding="utf-8")
    launcher = tmp_path / "locomo-venv" / "bin" / "python"
    launcher.parent.mkdir(parents=True)
    launcher.symlink_to(interpreter)
    document["assets"]["locomo"]["harness_python"] = str(launcher)
    config_path.write_text(json.dumps(document), encoding="utf-8")

    resolved = driver["resolve_configuration"](config_path)

    assert resolved["assets"]["locomo"]["harness_python"]["path"] == str(
        launcher.absolute()
    )


def test_encoding_cells_fail_closed_without_pinned_gpu(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    driver = load_driver()

    with pytest.raises(driver["GauntletConfigError"], match="pinned CUDA UUID"):
        driver["_verify_required_gpu"](
            {"gpu": {"enabled": False, "cuda_uuid": None}},
            [{"cell": "ac073"}],
        )

    driver["_verify_required_gpu"](
        {"gpu": {"enabled": False, "cuda_uuid": None}},
        [{"cell": "scale02"}],
    )

    expected = "GPU-11111111-1111-1111-1111-111111111111"

    def unavailable(*_args: object, **_kwargs: object) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(
            args=["nvidia-smi"],
            returncode=9,
            stdout="",
            stderr="NVIDIA driver unavailable",
        )

    monkeypatch.setattr(subprocess, "run", unavailable)
    with pytest.raises(driver["GauntletConfigError"], match="refusing CPU fallback"):
        driver["_verify_required_gpu"](
            {"gpu": {"enabled": True, "cuda_uuid": expected}},
            [{"cell": "ac073"}],
        )


def test_resolution_rejects_missing_or_wrong_kind_bindings(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    Path(document["runtime"]["python"]).unlink()

    with pytest.raises(driver["GauntletConfigError"], match="runtime.python"):
        driver["resolve_configuration"](config_path)

    document, config_path = valid_configuration(tmp_path / "second")
    document["assets"]["ir_c"]["gold"] = document["assets"]["ir_c"]["data_root"]
    config_path.write_text(json.dumps(document), encoding="utf-8")
    with pytest.raises(driver["GauntletConfigError"], match="assets.ir_c.gold"):
        driver["resolve_configuration"](config_path)


def test_output_must_be_outside_every_input_directory(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    document["output_root"] = str(
        Path(document["assets"]["tc5"]["corpus_root"]) / "run"
    )
    config_path.write_text(json.dumps(document), encoding="utf-8")

    with pytest.raises(driver["GauntletConfigError"], match="output_root"):
        driver["resolve_configuration"](config_path)


def test_resolved_configuration_uses_new_output_root_exclusively(
    tmp_path: Path,
) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)

    output = driver["write_resolved_configuration"](config_path)

    assert output == Path(document["output_root"]) / "gauntlet-plan.resolved.json"
    assert json.loads(output.read_text(encoding="utf-8"))["release"] == "0.8.26"
    with pytest.raises(driver["GauntletConfigError"], match="already exists"):
        driver["write_resolved_configuration"](config_path)


def test_cli_configuration_assertions_and_selection(tmp_path: Path) -> None:
    driver = load_driver()
    document, _ = valid_configuration(tmp_path)

    driver["assert_cli_configuration"](
        document,
        release="0.8.26",
        source_root=str(tmp_path / "source"),
        output_root=str(tmp_path / "output"),
    )
    assert driver["select_configured_cells"](document["cells"], None) == (
        "ac076",
        "search01",
        "locomo",
    )
    assert driver["select_configured_cells"](
        document["cells"], ("ac076", "locomo")
    ) == ("ac076", "locomo")

    with pytest.raises(driver["GauntletConfigError"], match="release"):
        driver["assert_cli_configuration"](
            document,
            release="0.8.25",
            source_root=str(tmp_path / "source"),
            output_root=str(tmp_path / "output"),
        )
    with pytest.raises(driver["GauntletConfigError"], match="not enabled"):
        driver["select_configured_cells"](document["cells"], ("ac072",))


def test_slice10_dry_run_remains_metadata_only_after_config_support(
    tmp_path: Path,
) -> None:
    missing_config = tmp_path / "missing-config.json"
    output = tmp_path / "missing-output"

    result = run_driver(
        "--release",
        "0.8.26",
        "--source-root",
        str(tmp_path / "missing-source"),
        "--config",
        str(missing_config),
        "--output-root",
        str(output),
        "--dry-run",
    )

    assert result.returncode == 0, result.stderr
    assert not missing_config.exists()
    assert not output.exists()


def test_resolution_hashes_and_parses_one_config_snapshot(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    stale_document = driver["load_configuration"](config_path)
    document["release"] = "0.8.27"
    config_path.write_text(json.dumps(document), encoding="utf-8")

    resolved = driver["resolve_configuration"](config_path)

    assert stale_document["release"] == "0.8.26"
    assert resolved["release"] == "0.8.27"
    assert (
        resolved["source_config"]["sha256"]
        == hashlib.sha256(config_path.read_bytes()).hexdigest()
    )


def test_resolved_configuration_persists_cli_subset(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)

    output = driver["write_resolved_configuration"](config_path, ("ac076", "locomo"))
    resolved = json.loads(output.read_text(encoding="utf-8"))

    assert resolved["cells"] == ["ac076", "locomo"]
    assert resolved["timeouts_s"] == {"ac076": 300, "locomo": 300}


def test_output_symlink_and_input_file_aliases_are_rejected(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    alias = tmp_path / "corpus-alias"
    alias.symlink_to(
        Path(document["assets"]["tc5"]["corpus_root"]), target_is_directory=True
    )
    document["output_root"] = str(alias / "run")
    config_path.write_text(json.dumps(document), encoding="utf-8")

    with pytest.raises(driver["GauntletConfigError"], match="output_root"):
        driver["resolve_configuration"](config_path)

    document, config_path = valid_configuration(tmp_path / "file-alias")
    document["output_root"] = document["runtime"]["python"]
    config_path.write_text(json.dumps(document), encoding="utf-8")
    with pytest.raises(driver["GauntletConfigError"], match="input file"):
        driver["resolve_configuration"](config_path)


def test_directory_binding_rejects_regular_file(tmp_path: Path) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    document["runtime"]["virtualenv"] = document["runtime"]["python"]
    config_path.write_text(json.dumps(document), encoding="utf-8")

    with pytest.raises(driver["GauntletConfigError"], match="runtime.virtualenv"):
        driver["resolve_configuration"](config_path)


def test_configuration_loader_rejects_invalid_encoding_and_duplicate_keys(
    tmp_path: Path,
) -> None:
    driver = load_driver()
    invalid_utf8 = tmp_path / "invalid-utf8.json"
    invalid_utf8.write_bytes(b"\xff")
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"release":"0.8.26","release":"0.8.27"}', encoding="utf-8")

    with pytest.raises(driver["GauntletConfigError"], match="cannot load"):
        driver["load_configuration"](invalid_utf8)
    with pytest.raises(driver["GauntletConfigError"], match="duplicate"):
        driver["load_configuration"](duplicate)


def test_write_failure_removes_partial_project_state(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    new_parent = tmp_path / "new-parent"
    document["output_root"] = str(new_parent / "new-child" / "output")
    config_path.write_text(json.dumps(document), encoding="utf-8")

    def fail_dump(*args: object, **kwargs: object) -> None:
        handle = args[1]
        handle.write("{partial")
        raise OSError("injected write failure")

    monkeypatch.setattr(driver["json"], "dump", fail_dump)
    with pytest.raises(driver["GauntletConfigError"], match="cannot write"):
        driver["write_resolved_configuration"](config_path)

    assert not Path(document["output_root"]).exists()
    assert not new_parent.exists()


def test_resume_requires_exact_plan_and_source_config_identity(tmp_path: Path) -> None:
    driver = load_driver()
    runner = tmp_path / "runner.sh"
    runner.write_text("#!/bin/sh\n", encoding="utf-8")
    artifact = tmp_path / "result.json"
    artifact.write_text("{}\n", encoding="utf-8")
    plan = {
        "cell": "ac076",
        "adapter": str(runner),
        "invocations": [{"argv": ["bash", str(runner)]}],
    }
    resolved = {
        "release": "0.8.26",
        "source": {"commit": "a" * 40},
        "source_config": {"sha256": "b" * 64},
    }
    identity = driver["_plan_identity"](plan, resolved)
    result = {
        "state": "passed",
        "release": "0.8.26",
        "source_commit": "a" * 40,
        "source_config_sha256": "b" * 64,
        "plan_sha256": identity,
        "runner": driver["_runner_identity"](plan),
        "artifacts": [
            {
                "path": str(artifact),
                "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(),
            }
        ],
    }

    assert driver["_resume_result_matches"](result, plan, resolved)
    changed_runner = tmp_path / "changed.sh"
    changed_runner.write_text("#!/bin/sh\n", encoding="utf-8")
    changed = {**plan, "adapter": str(changed_runner)}
    assert not driver["_resume_result_matches"](result, changed, resolved)
    assert not driver["_resume_result_matches"](
        {**result, "state": "failed"}, plan, resolved
    )
    artifact.write_text('{"changed":true}\n', encoding="utf-8")
    assert not driver["_resume_result_matches"](result, plan, resolved)


def test_unavailable_result_is_typed_and_bound_to_target_release() -> None:
    driver = load_driver()
    resolved = {
        "release": "0.8.26",
        "source": {"commit": "a" * 40},
        "source_config": {"sha256": "b" * 64},
    }

    result = driver["_unavailable_result"](
        "ac075", "pinned CUDA device is unavailable", resolved, phase="preflight"
    )

    assert result["schema_version"] == "fathomdb.performance-gauntlet.cell-result/v1"
    assert result["cell"] == "ac075"
    assert result["state"] == "unavailable"
    assert result["phase"] == "preflight"
    assert result["release"] == "0.8.26"
    assert result["source_commit"] == "a" * 40
    assert result["reason"] == "pinned CUDA device is unavailable"


def test_campaign_summary_covers_every_requested_cell_and_drives_markdown(
    tmp_path: Path,
) -> None:
    driver = load_driver()
    resolved = {
        "release": "0.8.26",
        "source": {"root": "/target", "commit": "a" * 40},
        "source_config": {"path": "/config.json", "sha256": "b" * 64},
        "output_root": str(tmp_path),
        "cells": ["ac076", "ac075"],
    }
    results = [
        {
            "cell": "ac076",
            "state": "passed",
            "release": "0.8.26",
            "source_commit": "a" * 40,
            "artifacts": [],
        },
        driver["_unavailable_result"](
            "ac075", "CUDA unavailable", resolved, phase="preflight"
        ),
    ]

    paths = driver["_write_campaign_documents"](resolved, results, mode="run")
    summary = json.loads(paths["summary_json"].read_text(encoding="utf-8"))
    markdown = paths["summary_markdown"].read_text(encoding="utf-8")
    status = json.loads(paths["status"].read_text(encoding="utf-8"))

    assert summary["schema_version"] == "fathomdb.performance-gauntlet.summary/v1"
    assert summary["comparison"] == "directional"
    assert summary["state"] == "incomplete"
    assert [item["cell"] for item in summary["cells"]] == ["ac076", "ac075"]
    assert "| ac076 | passed |" in markdown
    assert "| ac075 | unavailable | CUDA unavailable |" in markdown
    assert status["cells"] == {"ac075": "unavailable", "ac076": "passed"}


def test_fail_fast_marks_remaining_cells_not_run(tmp_path: Path) -> None:
    driver = load_driver()
    failing = tmp_path / "fail.sh"
    failing.write_text("#!/usr/bin/env bash\nexit 7\n", encoding="utf-8")
    failing.chmod(0o755)
    later = tmp_path / "later.sh"
    marker = tmp_path / "later-ran"
    later.write_text(f"#!/usr/bin/env bash\ntouch {marker}\n", encoding="utf-8")
    later.chmod(0o755)
    resolved = {
        "release": "0.8.26",
        "source": {"commit": "a" * 40},
        "source_config": {"sha256": "b" * 64},
        "output_root": str(tmp_path / "output"),
    }
    Path(resolved["output_root"]).mkdir()

    def plan(cell: str, script: Path) -> dict[str, object]:
        return {
            "cell": cell,
            "adapter": str(script),
            "cwd": str(tmp_path),
            "env": {},
            "unset_env": [],
            "invocations": [{"label": "run", "argv": [str(script)]}],
            "inputs": {},
            "outputs": {
                "run": str(Path(resolved["output_root"]) / "cells" / cell / "run")
            },
            "timeout_s": 30,
            "workload_identity": {},
        }

    results = driver["_run_ready_plans"](
        [plan("ac076", failing), plan("ac072", later)],
        resolved,
        resume=False,
        fail_fast=True,
    )

    assert [item["state"] for item in results] == ["failed", "not-run"]
    assert results[1]["reason"] == "not run after fail-fast stopped the campaign"
    assert not marker.exists()


def test_zero_exit_without_declared_output_is_failed(tmp_path: Path) -> None:
    driver = load_driver()
    runner = tmp_path / "runner.sh"
    runner.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
    runner.chmod(0o755)
    output = tmp_path / "output"
    output.mkdir()
    resolved = {
        "release": "0.8.26",
        "source": {"commit": "a" * 40},
        "source_config": {"sha256": "b" * 64},
        "output_root": str(output),
    }
    plan = {
        "cell": "ac076",
        "adapter": str(runner),
        "cwd": str(tmp_path),
        "env": {},
        "unset_env": [],
        "invocations": [{"label": "run", "argv": [str(runner)]}],
        "inputs": {},
        "outputs": {"required": str(output / "cells" / "ac076" / "required.json")},
        "timeout_s": 30,
        "workload_identity": {},
    }

    result = driver["_run_ready_plans"](
        [plan], resolved, resume=False, fail_fast=False
    )[0]

    assert result["state"] == "failed"
    assert "missing declared output" in result["reason"]


def test_execute_retains_unavailable_peer_and_writes_full_summary(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    driver = load_driver()
    document, config_path = valid_configuration(tmp_path)
    document["cells"] = ["ac076", "ac075"]
    document["timeouts_s"] = {"default": 30, "cells": {}}
    output = Path(document["output_root"])
    payload = output / "cells" / "ac076" / "payload.json"
    runner = tmp_path / "fake-runner.sh"
    runner.write_text(
        "#!/usr/bin/env bash\n"
        f"mkdir -p {payload.parent}\n"
        f"printf '{{}}\\n' > {payload}\n",
        encoding="utf-8",
    )
    runner.chmod(0o755)
    config_path.write_text(json.dumps(document), encoding="utf-8")

    def fake_preflight(
        resolved: dict[str, object], module: object, repo_root: Path
    ) -> tuple[list[dict[str, object]], list[dict[str, object]], dict[str, str]]:
        del module, repo_root
        plan = {
            "cell": "ac076",
            "adapter": str(runner),
            "cwd": str(tmp_path),
            "env": {},
            "unset_env": [],
            "invocations": [{"label": "run", "argv": [str(runner)]}],
            "inputs": {},
            "outputs": {"payload": str(payload)},
            "timeout_s": 30,
            "workload_identity": {},
        }
        return (
            [plan],
            [
                driver["_ready_result"](plan, resolved),
                driver["_unavailable_result"](
                    "ac075", "CUDA unavailable", resolved, phase="preflight"
                ),
            ],
            {},
        )

    monkeypatch.setitem(driver["_execute"].__globals__, "_preflight_cells", fake_preflight)
    args = argparse.Namespace(
        release="0.8.26",
        source_root=document["source"]["root"],
        config=str(config_path),
        output_root=str(output),
        cells=None,
        resume=False,
        fail_fast=False,
        preflight_only=False,
    )

    returncode = driver["_execute"](args, ("ac076", "ac075"))

    assert returncode == 1
    summary = json.loads((output / "gauntlet-summary.json").read_text())
    assert [(item["cell"], item["state"]) for item in summary["cells"]] == [
        ("ac076", "passed"),
        ("ac075", "unavailable"),
    ]
    assert (output / "gauntlet-summary.md").is_file()
    assert (output / "gauntlet-status.json").is_file()
    assert (output / "cells" / "ac076" / "cell-plan.json").is_file()
    assert (output / "cells" / "ac075" / "cell-result.json").is_file()
