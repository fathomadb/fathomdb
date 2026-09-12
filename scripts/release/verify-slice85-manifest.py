#!/usr/bin/env python3
"""Validate Slice 85's evidence inventory without executing its commands."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from typing import Any


SCHEMA = "fathomdb.slice85-final-manifest/v1"
TOP_LEVEL_KEYS = {
    "schema_version",
    "release",
    "branch",
    "candidate",
    "legacy_manifest",
    "ce_overlay",
    "global_rules",
    "sealed_thresholds",
    "obligations",
}
ROW_KEYS = {
    "id",
    "origin",
    "disposition",
    "candidate_sha",
    "input_paths",
    "current_input_sha256",
    "accepted_input_sha256",
    "commands",
    "retained_receipt",
    "retained_receipt_sha256",
    "artifact_sha256",
    "evidence",
    "verdict",
}
EVIDENCE_KEYS = {"path", "sha256", "tests", "skipped", "verdict"}
ADDITIONAL_IDS = {
    "runtime-configuration",
    "protected-writes",
    "slice72-ce",
    "tc5-bridge",
    "ac034c",
}
DISPOSITIONS = {
    "run",
    "rerun",
    "reuse",
    "unavailable",
    "blocked",
    "accepted-non-pass",
}
CE_EXCEPTION = "dev/plans/0.8.25/features/slice-85/ce-engine-p95-exception.md"
CE_EXCEPTION_SHA256 = (
    "c3e3323583d75c8b78d7dacca6e0f0b9ce9a92c61a7ceea37d1804a9c140183d"
)
SEALED_THRESHOLDS = {
    "text_p50_ms_max": 20,
    "text_p99_ms_max": 150,
    "read_p50_ms_max": 80,
    "read_p99_ms_max": 300,
    "scale02_ack_ms_max": 1543.539,
    "scale02_total_ms_max": 1548.545,
    "ac013_total_ms_max": 1442.198,
    "write_spread_pct_max": 25,
    "ce_p95_ratio_max": 1.1,
}
MINIMUM_COUNTS = {
    "default-tree": 109,
    "final-interactions": 4,
    "schema26-upgrade": 2,
    "ac021": 1,
    "ac059b": 2,
    "ac034ab": 1,
    "performance": 1,
    "eu7-real": 1,
    "model-cache": 1,
    "model-ts": 33,
    "model-engine": 9,
    "model-python": 13,
    "model-cli": 2,
    "rust-release-build": 1,
    "rust-leaf-packages": 3,
    "cli-installed-smoke": 1,
    "linux-artifact-build": 1,
    "linux-artifact-current-smoke": 180,
    "linux-runtime-floor-smokes": 3,
    "linux-cuda-package": 3,
    "global-01-native": 1,
    "windows-runner-preflight": 1,
    "hosted-native-validation": 5,
    "jetson-tegra": 1,
    "mkdocs": 1,
    "hosted-ci": 1,
    "runtime-configuration": 6,
    "protected-writes": 6,
    "slice72-ce": 4,
    "tc5-bridge": 100,
}
OVERRIDDEN_LEGACY = {
    "performance",
    "linux-artifact-build",
    "linux-cuda-package",
    "linux-runtime-floor-smokes",
}
SLICE80_INPUT_PATHS = [
    "Cargo.toml",
    "Cargo.lock",
    ".cargo/config.toml",
    "src/rust/crates/fathomdb-engine/Cargo.toml",
    "src/rust/crates/fathomdb-engine/src",
    "src/rust/crates/fathomdb-engine/tests/perf_gates.rs",
    "src/rust/crates/fathomdb-engine/tests/reader_pool.rs",
    "src/rust/crates/fathomdb-query",
    "src/rust/crates/fathomdb-schema",
    "src/rust/crates/fathomdb-embedder",
    "src/rust/crates/fathomdb-embedder-api",
]
SLICE80_INPUT_SHA256 = (
    "95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274"
)
SLICE80_RECEIPT = "dev/plans/0.8.25/features/slice-80/current-evidence.md"
SLICE80_RECEIPT_SHA256 = (
    "e91bbcff1434e4fb32509513b3c3b4c5a07b2f319ceaeaac61924fc5591435c7"
)
SHA = re.compile(r"^[0-9a-f]{40}$")
DIGEST = re.compile(r"^[0-9a-f]{64}$")


def validate_override_commands(row_id: str, commands: list[str]) -> None:
    joined = "\n".join(commands)
    if row_id == "performance":
        require(
            len(commands) == 1
            and "ac_012_text_query_latency_on_fts5_path" in joined
            and "AC012_CORPUS_N=10000" in joined,
            "performance override command contract changed",
        )
    elif row_id == "linux-runtime-floor-smokes":
        required = (
            "cpython-3.10.20-linux-x86_64-gnu/bin/python3.10",
            "cpython-3.11.15-linux-x86_64-gnu/bin/python3.11",
            "/usr/bin/python3.12",
            "node/v25.9.0/bin/node",
        )
        require(
            len(commands) == 3
            and all(value in joined for value in required)
            and all("FATHOMDB_SMOKE_LIGHTWEIGHT=1" in value for value in commands),
            "runtime-floor override command contract changed",
        )
    elif row_id == "linux-artifact-build":
        python_build = (
            "bash -c 'cd src/python && maturin build --locked --release --out "
            "../../${RUN_DIR}/artifacts/python --features "
            "pyo3/extension-module,default-embedder'"
        )
        require(
            len(commands) == 4
            and commands[0] == python_build
            and commands[1] == "npm ci --prefix src/ts"
            and commands[2] == "npm run build:native --prefix src/ts"
            and commands[3]
            == "src/ts/node_modules/.bin/tsc -p src/ts/tsconfig.build.json",
            "Linux artifact build command contract changed",
        )
    elif row_id == "linux-cuda-package":
        gpu_uuid = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b"
        require(
            len(commands) == 5
            and "FATHOMDB_CANDIDATE_SHA=${FINAL_SHA}" in commands[0]
            and f"FATHOMDB_CUDA_GPU_UUID={gpu_uuid}" in commands[0]
            and "cuda-preflight.sh" in commands[0]
            and "--rerank-cuda" in commands[0]
            and "CUDA_HOME=/usr/local/cuda-12.6" in commands[1]
            and "LIBRARY_PATH=/usr/local/cuda-12.6/lib64" in commands[1]
            and "PATH=/usr/local/cuda-12.6/bin:${PATH}" in commands[1]
            and "cargo build --locked --release -p fathomdb-cli" in commands[1]
            and "--features embed-cuda,rerank-cuda" in commands[1]
            and "seal-cuda-cli-archive.sh" in commands[2]
            and "--output ${RUN_DIR}/cuda-preflight.packages/"
            "fathomdb-0.8.24-x86_64-unknown-linux-gnu.tar.gz"
            in commands[2]
            and "seal-slice75-cuda-packages.py" in commands[3]
            and "--candidate-sha ${FINAL_SHA}" in commands[3]
            and "--packages ${RUN_DIR}/cuda-preflight.packages" in commands[3]
            and "--witness ${RUN_DIR}/cuda-preflight" in commands[3]
            and "--output ${RUN_DIR}/cuda-package-manifest.json" in commands[3]
            and f"FATHOMDB_CUDA_GPU_UUID={gpu_uuid}" in commands[4]
            and "CUDA_HOME=/usr/local/cuda-12.6" in commands[4]
            and "slice75-cuda-package-smoke.sh" in commands[4]
            and "--candidate-sha ${FINAL_SHA}" in commands[4]
            and "--packages ${RUN_DIR}/cuda-preflight.packages" in commands[4]
            and "--package-manifest ${RUN_DIR}/cuda-package-manifest.json"
            in commands[4]
            and "--witness ${RUN_DIR}/cuda-preflight" in commands[4]
            and "--output ${RUN_DIR}/cuda-package-smoke" in commands[4],
            "Linux CUDA package command contract changed",
        )
    elif row_id == "tc5-bridge":
        require(
            len(commands) == 5
            and "cargo build --locked --release -p fathomdb-tc5-benchmark" in commands[0]
            and "--features tc5-benchmark-cuda" in commands[0]
            and commands[1] == "python3 -m venv ${RUN_DIR}/tc5-runtime"
            and commands[2].startswith("${RUN_DIR}/tc5-runtime/bin/python -m pip install --no-deps ")
            and "${RUN_DIR}/artifacts/python/fathomdb-0.8.24-" in commands[2]
            and commands[3].startswith(
                "${RUN_DIR}/tc5-runtime/bin/python -m experiments.tc5_gpu_v2 dry-run"
            )
            and "--config ${RUN_DIR}/tc5-candidate-config.json" in commands[3]
            and "--arm bridge" in commands[3]
            and commands[4].startswith(
                "${RUN_DIR}/tc5-runtime/bin/python -m experiments.tc5_gpu_v2 run"
            )
            and "--config ${RUN_DIR}/tc5-candidate-config.json" in commands[4]
            and "--arm bridge" in commands[4]
            and "--binary ${RUN_DIR}/artifacts/fathomdb-tc5-benchmark" in commands[4],
            "TC-5 bridge command contract changed",
        )
    elif row_id == "runtime-configuration":
        required = (
            "runtime_configuration",
            "statement_reuse_",
            "test_slice79_runtime_configuration.py",
            "slice79-runtime-configuration.test.js",
            "smoke-local-native-artifacts.sh",
        )
        require(
            len(commands) == 5 and all(value in joined for value in required),
            "runtime-configuration override command contract changed",
        )
        ts_commands = [
            command
            for command in commands
            if "slice79-runtime-configuration.test.js" in command
        ]
        require(
            len(ts_commands) == 1,
            "runtime-configuration: TypeScript compile order changed",
        )
        ts_command = ts_commands[0]
        ts_steps = (
            "npm run build:debug --prefix src/ts",
            "src/ts/node_modules/.bin/tsc -p src/ts/tsconfig.json",
            "node --test src/ts/dist/tests/slice79-runtime-configuration.test.js",
        )
        require(
            all(step in ts_command for step in ts_steps)
            and [ts_command.index(step) for step in ts_steps]
            == sorted(ts_command.index(step) for step in ts_steps),
            "runtime-configuration: TypeScript compile order changed",
        )
    elif row_id == "protected-writes":
        require(
            len(commands) == 7
            and all(
                value in commands[0]
                for value in ("cargo build", "--offline", "--locked", "--release")
            )
            and sum(
                "--fixture scale02" in value and "--treatment production" in value
                for value in commands
            )
            == 3
            and sum(
                "--fixture ac013" in value and "--treatment production" in value
                for value in commands
            )
            == 3,
            "protected-write override command contract changed",
        )
    elif row_id == "slice72-ce":
        required = (
            "slice72_ce_artifact.py",
            "--device cpu",
            "--device cuda",
            "verify-slice72-ce-profile.py",
        )
        require(
            len(commands) == 4 and all(value in joined for value in required),
            "Slice 72 CE override command contract changed",
        )


class InvalidManifest(ValueError):
    """The Slice 85 manifest is incomplete or internally inconsistent."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise InvalidManifest(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def repo_file(repo: Path, relative: object, label: str) -> Path:
    require(isinstance(relative, str) and relative, f"{label} path must be nonempty")
    path = (repo / relative).resolve()
    require(path != repo and repo in path.parents, f"{label} path escapes repository")
    require(
        not {"invalid-evidence", "invalid-artifacts"}.intersection(path.parts),
        f"{label} path is quarantined: {relative}",
    )
    require(path.is_file(), f"{label} file missing: {relative}")
    return path


def computed_input_digest(repo: Path, paths: object) -> str:
    require(isinstance(paths, list) and paths, "input_paths must be a nonempty list")
    files: list[tuple[str, Path]] = []
    for relative in paths:
        path = (repo / relative).resolve() if isinstance(relative, str) else repo
        require(
            repo in path.parents and path.exists(),
            f"input path missing or unsafe: {relative}",
        )
        if path.is_file():
            files.append((str(relative), path))
        else:
            for child in sorted(value for value in path.rglob("*") if value.is_file()):
                files.append((str(child.relative_to(repo)), child))
    require(files, "input_paths contain no files")
    if len(files) == 1 and len(paths) == 1 and (repo / paths[0]).is_file():
        return sha256(files[0][1])
    digest = hashlib.sha256()
    for relative, path in files:
        digest.update(relative.encode("utf-8") + b"\0")
        digest.update(bytes.fromhex(sha256(path)))
    return digest.hexdigest()


def git_output(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], check=False, capture_output=True, text=True
    )
    require(result.returncode == 0, "candidate commit is unavailable in repository Git")
    return result.stdout.strip()


def git_input_digest(repo: Path, candidate_sha: str, paths: list[str]) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), "ls-tree", "-r", candidate_sha, "--", *paths],
        check=False,
        capture_output=True,
    )
    require(result.returncode == 0, "could not compute sealed Git input digest")
    return hashlib.sha256(result.stdout).hexdigest()


def validate_candidate(
    candidate: object, repo: Path, phase: str
) -> tuple[str, set[str]]:
    require(isinstance(candidate, dict), "candidate must be an object")
    require(set(candidate) == {"sha", "tree", "artifacts"}, "candidate keys changed")
    candidate_sha = candidate["sha"]
    tree = candidate["tree"]
    require(
        isinstance(candidate_sha, str) and SHA.fullmatch(candidate_sha) is not None,
        "invalid candidate sha",
    )
    require(
        isinstance(tree, str) and SHA.fullmatch(tree) is not None,
        "invalid candidate tree",
    )
    if phase == "final":
        require(
            git_output(repo, "rev-parse", f"{candidate_sha}^{{commit}}")
            == candidate_sha,
            "candidate commit mismatch",
        )
        require(
            git_output(repo, "rev-parse", f"{candidate_sha}^{{tree}}") == tree,
            "candidate tree mismatch",
        )
    artifacts = candidate["artifacts"]
    require(
        isinstance(artifacts, dict) and artifacts,
        "candidate artifacts must be nonempty",
    )
    verified: set[str] = set()
    for name, artifact in artifacts.items():
        require(
            isinstance(name, str) and isinstance(artifact, dict),
            "candidate artifact entry is invalid",
        )
        require(
            set(artifact) == {"path", "sha256"},
            f"candidate artifact {name}: keys changed",
        )
        claimed = artifact["sha256"]
        require(
            isinstance(claimed, str) and DIGEST.fullmatch(claimed) is not None,
            f"candidate artifact {name}: invalid sha256",
        )
        if phase == "final":
            path = repo_file(repo, artifact["path"], "artifact")
            require(
                sha256(path) == claimed,
                f"candidate artifact {name}: artifact sha256 mismatch",
            )
        verified.add(claimed)
    return candidate_sha, verified


def validate_ce_overlay(
    manifest: dict[str, Any], repo: Path, candidate_sha: str
) -> None:
    value = manifest["ce_overlay"]
    require(
        isinstance(value, dict) and set(value) == {"base_path", "path"},
        "ce_overlay keys changed",
    )
    base = json.loads(
        repo_file(repo, value["base_path"], "CE base").read_text(encoding="utf-8")
    )
    overlay = json.loads(
        repo_file(repo, value["path"], "CE overlay").read_text(encoding="utf-8")
    )
    require(
        isinstance(base, dict) and isinstance(overlay, dict),
        "CE overlay inputs must be objects",
    )
    expected = dict(base)
    expected["candidate_sha"] = candidate_sha
    require(overlay == expected, "CE overlay may only change candidate_sha")


def evidence_json(repo: Path, row: dict[str, Any]) -> dict[str, Any]:
    require(len(row["evidence"]) == 1, f"{row['id']}: requires one structured receipt")
    path = repo_file(repo, row["evidence"][0]["path"], "evidence")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise InvalidManifest(f"{row['id']}: structured receipt is invalid JSON") from exc
    require(isinstance(value, dict), f"{row['id']}: structured receipt must be an object")
    return value


def validate_eu7_ac073(repo: Path, row: dict[str, Any], candidate_sha: str) -> None:
    value = evidence_json(repo, row)
    require(
        value.get("schema_version") == "fathomdb.slice85-eu7-ac073/v1"
        and value.get("candidate_sha") == candidate_sha
        and value.get("ac073_stress") == "pass"
        and value.get("ac075") == "superseded-by-tc5"
        and isinstance(value.get("stress_p99_ms"), (int, float))
        and isinstance(value.get("stress_bound_ms"), (int, float))
        and value["stress_p99_ms"] <= value["stress_bound_ms"],
        "EU7 AC-073 stress receipt is not a candidate-bound pass",
    )


def validate_tc5_bridge(
    repo: Path, row: dict[str, Any], candidate_sha: str, artifact_hashes: set[str]
) -> None:
    value = evidence_json(repo, row)
    candidate = value.get("candidate")
    require(
        isinstance(candidate, dict)
        and candidate.get("sha") == candidate_sha
        and candidate.get("version") == "0.8.25",
        "TC-5 candidate binding is missing or stale",
    )
    candidate_artifacts = {
        candidate.get("python_wheel_sha256"),
        candidate.get("fathomdb_bin_sha256"),
        candidate.get("benchmark_binary_sha256"),
    }
    require(
        candidate.get("package_version") == "0.8.24",
        "TC-5 candidate package version is missing or stale",
    )
    require(
        len(candidate_artifacts) == 3
        and all(
            isinstance(value, str) and DIGEST.fullmatch(value) is not None
            for value in candidate_artifacts
        )
        and candidate_artifacts <= artifact_hashes,
        "TC-5 candidate artifact binding is missing or stale",
    )
    provenance = value.get("provenance")
    metrics = value.get("metrics")
    require(
        value.get("schema_version") == "tc5-gpu-arm-result.v2"
        and value.get("arm") == "bridge"
        and value.get("document_count") == 7667
        and value.get("query_completion_count") == 100
        and value.get("bootstrap_resamples") == 1000
        and value.get("synthetic_document_count") == 0
        and value.get("fixture_digest")
        == "9e92d236e44fc7443c1940f6870877a6e6eb07e92136ca2da331f88221e622ed"
        and value.get("ground_truth_sha256")
        == "ef6be77b9b5670b0992606167f6cc191849f51ac90b6c4b7d25f403c3dc7f34b"
        and value.get("sut_result_sha256")
        == "436493dcd17973f33cde5424391cd73288a9740d1c837ed5231a7bc0db7cf84a"
        and isinstance(metrics, dict)
        and isinstance(metrics.get("recall_at_10"), (int, float))
        and metrics["recall_at_10"] >= 0.958
        and isinstance(metrics.get("ci_95"), list)
        and len(metrics["ci_95"]) == 2
        and metrics["ci_95"][0] >= 0.938
        and metrics["ci_95"][1] >= 0.974
        and isinstance(provenance, dict)
        and provenance.get("candidate_execution") == "cpu/sqlite-vec"
        and provenance.get("exact_f32_rerank_execution") == "cpu/sqlite-vec"
        and provenance.get("embedding_execution") == "cuda:0"
        and provenance.get("candidate_k") == 192
        and provenance.get("top_k") == 10,
        "TC-5 bridge equivalence receipt drifted from the frozen bridge",
    )


def validate(manifest: dict[str, Any], repo: Path, phase: str) -> None:
    require(set(manifest) == TOP_LEVEL_KEYS, "top-level manifest keys changed")
    require(manifest["schema_version"] == SCHEMA, "wrong schema_version")
    require(manifest["release"] == "0.8.25", "release must be 0.8.25")
    require(manifest["branch"] == "release/0.8.25", "branch must be release/0.8.25")
    candidate_sha, artifact_hashes = validate_candidate(
        manifest["candidate"], repo, phase
    )

    legacy_ref = manifest["legacy_manifest"]
    require(
        isinstance(legacy_ref, dict) and set(legacy_ref) == {"path", "sha256"},
        "legacy_manifest keys changed",
    )
    legacy_path = repo_file(repo, legacy_ref["path"], "legacy manifest")
    require(
        legacy_ref["sha256"] == sha256(legacy_path), "legacy manifest sha256 mismatch"
    )
    legacy = json.loads(legacy_path.read_text(encoding="utf-8"))
    legacy_by_id = {cell.get("id"): cell for cell in legacy.get("cells", [])}
    require(
        len(legacy_by_id) == 26 and None not in legacy_by_id,
        "legacy manifest must define 26 unique cells",
    )
    require(
        manifest["global_rules"]
        == {
            "forbidden_output_patterns": ["[skip]", "0 tests", "timed out"],
            "retry_count": 0,
        },
        "global_rules must remain fail-closed",
    )
    require(
        manifest["sealed_thresholds"] == SEALED_THRESHOLDS, "sealed_thresholds changed"
    )
    validate_ce_overlay(manifest, repo, candidate_sha)

    rows = manifest["obligations"]
    require(isinstance(rows, list), "obligations must be a list")
    ids = [row.get("id") if isinstance(row, dict) else None for row in rows]
    require(
        len(ids) == len(set(ids)) and set(ids) == set(legacy_by_id) | ADDITIONAL_IDS,
        "obligation IDs must be unique and complete",
    )
    unavailable: set[str] = set()
    accepted_non_pass: set[str] = set()
    for row in rows:
        row_id = row["id"]
        require(set(row) == ROW_KEYS, f"{row_id}: obligation keys changed")
        disposition = row["disposition"]
        require(disposition in DISPOSITIONS, f"{row_id}: invalid disposition")
        require(
            row["origin"] in {"legacy", "additional", "authorized-exception"},
            f"{row_id}: invalid origin",
        )
        require(
            row["candidate_sha"] == candidate_sha,
            f"{row_id}: candidate_sha does not match candidate",
        )
        current_input = row["current_input_sha256"]
        require(
            isinstance(current_input, str)
            and DIGEST.fullmatch(current_input) is not None,
            f"{row_id}: invalid current input digest",
        )
        commands = row["commands"]
        require(
            isinstance(commands, list)
            and all(isinstance(item, str) and item for item in commands),
            f"{row_id}: commands must be strings",
        )
        if row_id in legacy_by_id and row_id not in OVERRIDDEN_LEGACY:
            require(
                commands == legacy_by_id[row_id]["commands"],
                f"{row_id}: legacy command contract changed",
            )
        if (
            row_id in OVERRIDDEN_LEGACY | (ADDITIONAL_IDS - {"ac034c"})
            and disposition != "reuse"
        ):
            validate_override_commands(row_id, commands)
        if disposition in {"run", "rerun"}:
            require(commands, f"{row_id}: execution disposition requires commands")
        if disposition == "reuse":
            require(not commands, f"{row_id}: reuse must not carry execution commands")
            require(
                row["accepted_input_sha256"] == current_input,
                f"{row_id}: reuse input digest mismatch",
            )
        receipt = row["retained_receipt"]
        receipt_digest = row["retained_receipt_sha256"]
        uses_slice80 = row_id == "performance" and (
            disposition == "reuse" or receipt is not None
        )
        if uses_slice80:
            require(
                current_input == row["accepted_input_sha256"] == SLICE80_INPUT_SHA256,
                "performance: reuse input digest mismatch",
            )
            require(
                row["input_paths"] == SLICE80_INPUT_PATHS,
                "performance: Slice 80 reuse input paths changed",
            )
            require(
                receipt == SLICE80_RECEIPT and receipt_digest == SLICE80_RECEIPT_SHA256,
                "performance: retained receipt identity changed",
            )
        if disposition == "reuse" or receipt is not None:
            require(
                isinstance(receipt_digest, str)
                and DIGEST.fullmatch(receipt_digest) is not None,
                f"{row_id}: retained receipt sha256 missing",
            )
            if phase == "final":
                receipt_path = repo_file(repo, receipt, "retained receipt")
                require(
                    sha256(receipt_path) == receipt_digest,
                    f"{row_id}: retained receipt sha256 mismatch",
                )
        if uses_slice80:
            require(
                git_input_digest(repo, candidate_sha, SLICE80_INPUT_PATHS)
                == SLICE80_INPUT_SHA256,
                "performance: computed input digest mismatch",
            )
        else:
            require(
                computed_input_digest(repo, row["input_paths"]) == current_input,
                f"{row_id}: computed input digest mismatch",
            )
        if disposition == "unavailable":
            unavailable.add(row_id)
        if disposition == "accepted-non-pass":
            accepted_non_pass.add(row_id)
            require(
                row_id == "slice72-ce"
                and row["origin"] == "authorized-exception",
                "accepted non-pass is authorized only for slice72-ce",
            )
            require(
                receipt == CE_EXCEPTION
                and receipt_digest == CE_EXCEPTION_SHA256,
                "slice72-ce: accepted non-pass disposition reference changed",
            )
        artifact_sha = row["artifact_sha256"]
        if artifact_sha is not None:
            require(
                artifact_sha in artifact_hashes, f"{row_id}: artifact_sha256 is stale"
            )
        evidence = row["evidence"]
        require(isinstance(evidence, list), f"{row_id}: evidence must be a list")
        if phase == "final" and disposition != "unavailable":
            require(
                disposition != "blocked", f"{row_id}: blocked disposition cannot close"
            )
            expected_verdict = (
                "accepted-non-pass" if disposition == "accepted-non-pass" else "pass"
            )
            require(
                row["verdict"] == expected_verdict,
                f"{row_id}: final verdict must be {expected_verdict}",
            )
            require(evidence, f"{row_id}: closing row requires evidence")
            total = 0
            for item in evidence:
                require(
                    isinstance(item, dict) and set(item) == EVIDENCE_KEYS,
                    f"{row_id}: evidence keys changed",
                )
                require(
                    isinstance(item["tests"], int)
                    and not isinstance(item["tests"], bool)
                    and item["tests"] > 0,
                    f"{row_id}: evidence requires a positive test count",
                )
                require(
                    item["skipped"] == 0, f"{row_id}: evidence skipped must be zero"
                )
                require(
                    item["verdict"] == expected_verdict,
                    f"{row_id}: evidence verdict must be {expected_verdict}",
                )
                path = repo_file(repo, item["path"], "evidence")
                run_dir = (repo / "dev/plans/runs/0.8.25-slice-85").resolve()
                require(
                    run_dir in path.parents,
                    f"{row_id}: evidence must be in the Slice 85 run directory",
                )
                require(
                    sha256(path) == item["sha256"],
                    f"{row_id}: evidence sha256 mismatch",
                )
                text = path.read_text(encoding="utf-8", errors="replace")
                for pattern in manifest["global_rules"]["forbidden_output_patterns"]:
                    require(
                        pattern not in text,
                        f"{row_id}: forbidden output pattern {pattern!r}",
                    )
                total += item["tests"]
            require(
                total >= MINIMUM_COUNTS[row_id],
                f"{row_id}: positive count contract requires {MINIMUM_COUNTS[row_id]}",
            )
            if row_id == "eu7-real":
                validate_eu7_ac073(repo, row, candidate_sha)
            elif row_id == "tc5-bridge":
                validate_tc5_bridge(repo, row, candidate_sha, artifact_hashes)

    require(
        unavailable == {"ac034c"}, "AC-034c must be the only unavailable obligation"
    )
    require(
        accepted_non_pass <= {"slice72-ce"},
        "accepted non-pass is authorized only for slice72-ce",
    )
    if phase == "final":
        require(
            accepted_non_pass == {"slice72-ce"},
            "slice72-ce must use its documented accepted non-pass disposition",
        )
    ac034c = next(row for row in rows if row["id"] == "ac034c")
    require(
        ac034c["verdict"] == "unavailable",
        "AC-034c must remain an unavailable non-pass",
    )
    if phase == "final":
        for row_id in ("runtime-configuration", "linux-artifact-current-smoke"):
            row = next(value for value in rows if value["id"] == row_id)
            require(
                row["artifact_sha256"] is not None, f"{row_id}: artifact_sha256 missing"
            )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--phase", choices=("plan", "final"), required=True)
    args = parser.parse_args()
    try:
        value = json.loads(args.manifest.read_text(encoding="utf-8"))
        require(isinstance(value, dict), "manifest root must be an object")
        validate(value, args.repo.resolve(), args.phase)
    except (OSError, json.JSONDecodeError, InvalidManifest) as error:
        print(f"slice85-manifest: {error}", file=sys.stderr)
        return 1
    print(
        f"slice85-manifest: valid {args.phase} manifest ({len(value['obligations'])} obligations)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
