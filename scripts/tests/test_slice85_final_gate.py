#!/usr/bin/env python3
"""Mutation tests for the Slice 85 evidence gate and installed-runtime route."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "scripts/release/verify-slice85-manifest.py"
LEGACY = ROOT / "dev/plans/0.8.25/features/slice-75/slice75-closure-manifest.json"
CE_BASE = ROOT / "dev/plans/0.8.25/features/slice-72/ce-profile-manifest.json"
CE_EXCEPTION = (
    ROOT / "dev/plans/0.8.25/features/slice-85/ce-engine-p95-exception.md"
)
SMOKE = ROOT / "scripts/release/smoke/smoke-local-native-artifacts.sh"
WINDOWS_RUNNER = ROOT / "scripts/release/slice75-windows-runner.sh"
HASH = "2" * 64


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Slice85FinalGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="slice85-gate-test-")
        self.repo = Path(self.temporary.name)
        legacy_path = self.repo / LEGACY.relative_to(ROOT)
        legacy_path.parent.mkdir(parents=True)
        legacy_path.write_bytes(LEGACY.read_bytes())
        ce_base_path = self.repo / CE_BASE.relative_to(ROOT)
        ce_base_path.parent.mkdir(parents=True)
        ce_base_path.write_bytes(CE_BASE.read_bytes())
        exception_path = self.repo / CE_EXCEPTION.relative_to(ROOT)
        exception_path.parent.mkdir(parents=True, exist_ok=True)
        exception_path.write_bytes(CE_EXCEPTION.read_bytes())
        overlay = json.loads(CE_BASE.read_text(encoding="utf-8"))
        overlay_path = (
            self.repo / "dev/plans/runs/0.8.25-slice-85/slice72-ce-manifest.json"
        )
        overlay_path.parent.mkdir(parents=True)
        (self.repo / "source-input.txt").write_text(
            "candidate input\n", encoding="utf-8"
        )
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        subprocess.run(
            ["git", "-C", str(self.repo), "config", "user.name", "Slice 85 Test"],
            check=True,
        )
        subprocess.run(
            [
                "git",
                "-C",
                str(self.repo),
                "config",
                "user.email",
                "slice85@example.invalid",
            ],
            check=True,
        )
        subprocess.run(["git", "-C", str(self.repo), "add", "."], check=True)
        subprocess.run(
            ["git", "-C", str(self.repo), "commit", "-qm", "fixture"], check=True
        )
        self.candidate_sha = subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "HEAD"], text=True
        ).strip()
        self.candidate_tree = subprocess.check_output(
            ["git", "-C", str(self.repo), "rev-parse", "HEAD^{tree}"], text=True
        ).strip()
        overlay["candidate_sha"] = self.candidate_sha
        overlay_path.write_text(json.dumps(overlay), encoding="utf-8")
        self.input_hash = digest(self.repo / "source-input.txt")
        self.artifacts = {}
        for name in (
            "python-wheel",
            "napi-linux-x64-gnu",
            "cli-linux-x64-gnu",
            "tc5-benchmark",
        ):
            path = self.repo / "artifacts" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(name, encoding="utf-8")
            self.artifacts[name] = {
                "path": str(path.relative_to(self.repo)),
                "sha256": digest(path),
            }

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def obligation(
        self,
        obligation_id: str,
        origin: str = "legacy",
        commands: list[str] | None = None,
    ) -> dict:
        return {
            "id": obligation_id,
            "origin": origin,
            "disposition": "run",
            "candidate_sha": self.candidate_sha,
            "input_paths": ["source-input.txt"],
            "current_input_sha256": self.input_hash,
            "accepted_input_sha256": None,
            "commands": commands if commands is not None else ["true"],
            "retained_receipt": None,
            "retained_receipt_sha256": None,
            "artifact_sha256": None,
            "evidence": [],
            "verdict": "pending",
        }

    def manifest(self) -> dict:
        legacy = json.loads(LEGACY.read_text(encoding="utf-8"))
        performance = [
            "timeout 3600 env -u AC_FULL_SCALE AGENT_LONG=1 AC012_CORPUS_N=10000 "
            "cargo test --release -p fathomdb-engine --test perf_gates "
            "ac_012_text_query_latency_on_fts5_path -- --exact --nocapture --test-threads=1"
        ]
        floors = [
            "env FATHOMDB_SMOKE_PYTHON=/home/coreyt/.local/share/uv/python/"
            "cpython-3.10.20-linux-x86_64-gnu/bin/python3.10 "
            "FATHOMDB_SMOKE_NODE=/home/coreyt/.nvm/versions/node/v25.9.0/bin/node "
            "FATHOMDB_SMOKE_LIGHTWEIGHT=1 bash scripts/release/smoke/smoke-local-native-artifacts.sh",
            "env FATHOMDB_SMOKE_PYTHON=/home/coreyt/.local/share/uv/python/"
            "cpython-3.11.15-linux-x86_64-gnu/bin/python3.11 "
            "FATHOMDB_SMOKE_NODE=/home/coreyt/.nvm/versions/node/v25.9.0/bin/node "
            "FATHOMDB_SMOKE_LIGHTWEIGHT=1 bash scripts/release/smoke/smoke-local-native-artifacts.sh",
            "env FATHOMDB_SMOKE_PYTHON=/usr/bin/python3.12 "
            "FATHOMDB_SMOKE_NODE=/home/coreyt/.nvm/versions/node/v25.9.0/bin/node "
            "FATHOMDB_SMOKE_LIGHTWEIGHT=1 bash scripts/release/smoke/smoke-local-native-artifacts.sh",
        ]
        artifact_build = [
            "bash -c 'cd src/python && maturin build --locked --release --out "
            "../../${RUN_DIR}/artifacts/python --features "
            "pyo3/extension-module,default-embedder'",
            "npm ci --prefix src/ts",
            "npm run build:native --prefix src/ts",
            "src/ts/node_modules/.bin/tsc -p src/ts/tsconfig.build.json",
        ]
        cuda_package = [
            "env FATHOMDB_CANDIDATE_SHA=${FINAL_SHA} "
            "FATHOMDB_CUDA_GPU_UUID=GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b "
            "bash scripts/release/cuda-preflight.sh ${RUN_DIR}/cuda-preflight --rerank-cuda",
            "env CUDA_HOME=/usr/local/cuda-12.6 "
            "LIBRARY_PATH=/usr/local/cuda-12.6/lib64 "
            "PATH=/usr/local/cuda-12.6/bin:${PATH} cargo build --locked --release "
            "-p fathomdb-cli --features embed-cuda,rerank-cuda "
            "--target x86_64-unknown-linux-gnu",
            "bash scripts/release/seal-cuda-cli-archive.sh --binary "
            "target/x86_64-unknown-linux-gnu/release/fathomdb --version 0.8.24 "
            "--output ${RUN_DIR}/cuda-preflight.packages/"
            "fathomdb-0.8.24-x86_64-unknown-linux-gnu.tar.gz",
            "python3 scripts/release/seal-slice75-cuda-packages.py "
            "--candidate-sha ${FINAL_SHA} --packages ${RUN_DIR}/cuda-preflight.packages "
            "--witness ${RUN_DIR}/cuda-preflight --output ${RUN_DIR}/cuda-package-manifest.json",
            "env FATHOMDB_CUDA_GPU_UUID=GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b "
            "CUDA_HOME=/usr/local/cuda-12.6 bash scripts/release/"
            "slice75-cuda-package-smoke.sh --candidate-sha ${FINAL_SHA} "
            "--packages ${RUN_DIR}/cuda-preflight.packages --package-manifest "
            "${RUN_DIR}/cuda-package-manifest.json --witness ${RUN_DIR}/cuda-preflight "
            "--hf-home ${HF_HOME} --output ${RUN_DIR}/cuda-package-smoke",
        ]
        obligations = [
            self.obligation(
                cell["id"],
                commands=(
                    performance
                    if cell["id"] == "performance"
                    else floors
                    if cell["id"] == "linux-runtime-floor-smokes"
                    else artifact_build
                    if cell["id"] == "linux-artifact-build"
                    else cuda_package
                    if cell["id"] == "linux-cuda-package"
                    else cell["commands"]
                ),
            )
            for cell in legacy["cells"]
        ]
        obligations.extend(
            [
                self.obligation(
                    "runtime-configuration",
                    "additional",
                    [
                        "cargo test -p fathomdb-engine --test runtime_configuration",
                        "cargo test -p fathomdb-engine statement_reuse_ --lib",
                        "python -m pytest src/python/tests/test_slice79_runtime_configuration.py",
                        "npm run build:debug --prefix src/ts && "
                        "src/ts/node_modules/.bin/tsc -p src/ts/tsconfig.json && "
                        "node --test src/ts/dist/tests/slice79-runtime-configuration.test.js",
                        "bash scripts/release/smoke/smoke-local-native-artifacts.sh",
                    ],
                ),
                self.obligation(
                    "protected-writes",
                    "additional",
                    [
                        "cargo build --offline --locked --release slice71b-probe",
                        *[
                            f"slice71b-probe --fixture {fixture} --treatment production --ordinal {ordinal}"
                            for fixture in ("scale02", "ac013")
                            for ordinal in range(1, 4)
                        ],
                    ],
                ),
                self.obligation(
                    "slice72-ce",
                    "additional",
                    [
                        "python3 scripts/release/slice72_ce_artifact.py --device cpu",
                        "python3 scripts/release/run-slice72-ce-profile.py --device cpu",
                        "python3 scripts/release/run-slice72-ce-profile.py --device cuda",
                        "python3 scripts/release/verify-slice72-ce-profile.py",
                    ],
                ),
                self.obligation(
                    "tc5-bridge",
                    "additional",
                    [
                        "cargo build --locked --release -p fathomdb-tc5-benchmark --features tc5-benchmark-cuda",
                        "python3 -m venv ${RUN_DIR}/tc5-runtime",
                        "${RUN_DIR}/tc5-runtime/bin/python -m pip install --no-deps ${RUN_DIR}/cuda-preflight.packages/fathomdb-0.8.24-cp310-abi3-manylinux_2_28_x86_64.whl",
                        "${RUN_DIR}/tc5-runtime/bin/python -m experiments.tc5_gpu_v2 dry-run --config ${RUN_DIR}/tc5-candidate-config.json --arm bridge --output-root ${RUN_DIR}/tc5-bridge",
                        "${RUN_DIR}/tc5-runtime/bin/python -m experiments.tc5_gpu_v2 run --config ${RUN_DIR}/tc5-candidate-config.json --arm bridge --output-root ${RUN_DIR}/tc5-bridge --binary ${RUN_DIR}/artifacts/fathomdb-tc5-benchmark",
                    ],
                ),
            ]
        )
        ac034c = self.obligation("ac034c", "authorized-exception")
        ac034c.update(
            disposition="unavailable",
            commands=[],
            evidence=[],
            verdict="unavailable",
        )
        obligations.append(ac034c)
        return {
            "schema_version": "fathomdb.slice85-final-manifest/v1",
            "release": "0.8.25",
            "branch": "release/0.8.25",
            "candidate": {
                "sha": self.candidate_sha,
                "tree": self.candidate_tree,
                "artifacts": self.artifacts,
            },
            "legacy_manifest": {
                "path": str(LEGACY.relative_to(ROOT)),
                "sha256": digest(LEGACY),
            },
            "ce_overlay": {
                "base_path": str(CE_BASE.relative_to(ROOT)),
                "path": "dev/plans/runs/0.8.25-slice-85/slice72-ce-manifest.json",
            },
            "global_rules": {
                "forbidden_output_patterns": ["[skip]", "0 tests", "timed out"],
                "retry_count": 0,
            },
            "sealed_thresholds": {
                "text_p50_ms_max": 20,
                "text_p99_ms_max": 150,
                "read_p50_ms_max": 80,
                "read_p99_ms_max": 300,
                "scale02_ack_ms_max": 1543.539,
                "scale02_total_ms_max": 1548.545,
                "ac013_total_ms_max": 1442.198,
                "write_spread_pct_max": 25,
                "ce_p95_ratio_max": 1.1,
            },
            "obligations": obligations,
        }

    def run_checker(
        self, value: dict, phase: str = "plan"
    ) -> subprocess.CompletedProcess[str]:
        path = self.repo / "manifest.json"
        path.write_text(json.dumps(value), encoding="utf-8")
        return subprocess.run(
            [
                "python3",
                str(CHECKER),
                "--manifest",
                str(path),
                "--repo",
                str(self.repo),
                "--phase",
                phase,
            ],
            check=False,
            capture_output=True,
            text=True,
        )

    def assert_rejected(self, value: dict, fragment: str, phase: str = "plan") -> None:
        result = self.run_checker(value, phase)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(fragment, result.stderr)

    @staticmethod
    def row(value: dict, obligation_id: str) -> dict:
        return next(row for row in value["obligations"] if row["id"] == obligation_id)

    def complete(self, value: dict) -> None:
        run_dir = self.repo / "dev/plans/runs/0.8.25-slice-85"
        run_dir.mkdir(parents=True, exist_ok=True)
        for row in value["obligations"]:
            if row["disposition"] == "unavailable":
                continue
            log = run_dir / f"{row['id']}.log"
            log.write_text("PASS positive evidence\n", encoding="utf-8")
            row["evidence"] = [
                {
                    "path": str(log.relative_to(self.repo)),
                    "sha256": digest(log),
                    "tests": 200,
                    "skipped": 0,
                    "verdict": "pass",
                }
            ]
            row["verdict"] = "pass"
        eu7 = self.row(value, "eu7-real")
        eu7_path = self.repo / eu7["evidence"][0]["path"]
        eu7_path.write_text(
            json.dumps(
                {
                    "schema_version": "fathomdb.slice85-eu7-ac073/v1",
                    "candidate_sha": self.candidate_sha,
                    "ac073_stress": "pass",
                    "stress_p99_ms": 391,
                    "stress_bound_ms": 730,
                    "ac075": "superseded-by-tc5",
                }
            ),
            encoding="utf-8",
        )
        eu7["evidence"][0]["sha256"] = digest(eu7_path)
        tc5 = self.row(value, "tc5-bridge")
        tc5_path = self.repo / tc5["evidence"][0]["path"]
        tc5_path.write_text(
            json.dumps(
                {
                    "schema_version": "tc5-gpu-arm-result.v2",
                    "candidate": {
                        "sha": self.candidate_sha,
                        "version": "0.8.25",
                        "package_version": "0.8.24",
                        "python_wheel_sha256": self.artifacts["python-wheel"]["sha256"],
                        "fathomdb_bin_sha256": self.artifacts["cli-linux-x64-gnu"]["sha256"],
                        "benchmark_binary_sha256": self.artifacts["tc5-benchmark"]["sha256"],
                    },
                    "arm": "bridge",
                    "document_count": 7667,
                    "query_completion_count": 100,
                    "bootstrap_resamples": 1000,
                    "synthetic_document_count": 0,
                    "fixture_digest": "9e92d236e44fc7443c1940f6870877a6e6eb07e92136ca2da331f88221e622ed",
                    "ground_truth_sha256": "ef6be77b9b5670b0992606167f6cc191849f51ac90b6c4b7d25f403c3dc7f34b",
                    "sut_result_sha256": "436493dcd17973f33cde5424391cd73288a9740d1c837ed5231a7bc0db7cf84a",
                    "metrics": {"recall_at_10": 0.958, "ci_95": [0.938, 0.974]},
                    "provenance": {
                        "candidate_execution": "cpu/sqlite-vec",
                        "exact_f32_rerank_execution": "cpu/sqlite-vec",
                        "embedding_execution": "cuda:0",
                        "candidate_k": 192,
                        "top_k": 10,
                    },
                }
            ),
            encoding="utf-8",
        )
        tc5["evidence"][0]["sha256"] = digest(tc5_path)
        tc5["evidence"][0]["tests"] = 100
        ce = self.row(value, "slice72-ce")
        ce["origin"] = "authorized-exception"
        ce["disposition"] = "accepted-non-pass"
        ce["retained_receipt"] = str(CE_EXCEPTION.relative_to(ROOT))
        ce["retained_receipt_sha256"] = digest(CE_EXCEPTION)
        ce["verdict"] = "accepted-non-pass"
        ce["evidence"][0]["tests"] = 4
        ce["evidence"][0]["verdict"] = "accepted-non-pass"
        for obligation_id in ("runtime-configuration", "linux-artifact-current-smoke"):
            self.row(value, obligation_id)["artifact_sha256"] = self.artifacts[
                "python-wheel"
                if obligation_id == "runtime-configuration"
                else "napi-linux-x64-gnu"
            ]["sha256"]

    def test_planning_manifest_validates(self) -> None:
        result = self.run_checker(self.manifest())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("31 obligations", result.stdout)

    def test_missing_or_duplicate_obligation_is_rejected(self) -> None:
        for mutation in ("missing", "duplicate"):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                if mutation == "missing":
                    value["obligations"].pop()
                else:
                    value["obligations"].append(copy.deepcopy(value["obligations"][0]))
                self.assert_rejected(value, "obligation IDs")

    def test_forbidden_disposition_is_rejected(self) -> None:
        value = self.manifest()
        self.row(value, "default-tree")["disposition"] = "waived"
        self.assert_rejected(value, "disposition")

    def test_final_pass_requires_positive_skip_free_evidence(self) -> None:
        for mutation, fragment in (
            ("missing", "evidence"),
            ("zero", "positive test count"),
            ("skip", "skipped must be zero"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                row = self.row(value, "default-tree")
                if mutation == "missing":
                    row["evidence"] = []
                elif mutation == "zero":
                    row["evidence"][0]["tests"] = 0
                else:
                    row["evidence"][0]["skipped"] = 1
                self.assert_rejected(value, fragment, "final")

    def test_stale_candidate_or_artifact_is_rejected(self) -> None:
        for mutation, fragment in (
            ("candidate", "candidate_sha"),
            ("artifact", "artifact_sha256"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                if mutation == "candidate":
                    self.row(value, "default-tree")["candidate_sha"] = "9" * 40
                else:
                    self.row(value, "runtime-configuration")["artifact_sha256"] = (
                        "9" * 64
                    )
                self.assert_rejected(value, fragment, "final")

    def test_relaxed_threshold_is_rejected(self) -> None:
        value = self.manifest()
        value["sealed_thresholds"]["text_p50_ms_max"] = 21
        self.assert_rejected(value, "sealed_thresholds")

    def test_false_reuse_is_rejected(self) -> None:
        value = self.manifest()
        row = self.row(value, "performance")
        row.update(
            disposition="reuse",
            commands=[],
            retained_receipt="dev/plans/runs/slice80.json",
            accepted_input_sha256="9" * 64,
        )
        self.assert_rejected(value, "reuse input digest")

    def test_final_rejects_missing_or_tampered_evidence_files(self) -> None:
        for mutation, fragment in (
            ("missing", "evidence file missing"),
            ("tampered", "evidence sha256 mismatch"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                path = (
                    self.repo / self.row(value, "default-tree")["evidence"][0]["path"]
                )
                if mutation == "missing":
                    path.unlink()
                else:
                    path.write_text("tampered\n", encoding="utf-8")
                self.assert_rejected(value, fragment, "final")

    def test_final_rejects_missing_reuse_receipt(self) -> None:
        value = self.manifest()
        self.complete(value)
        row = self.row(value, "performance")
        row.update(
            disposition="reuse",
            commands=[],
            input_paths=[
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
            ],
            current_input_sha256="95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274",
            accepted_input_sha256="95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274",
            retained_receipt="dev/plans/0.8.25/features/slice-80/current-evidence.md",
            retained_receipt_sha256="e91bbcff1434e4fb32509513b3c3b4c5a07b2f319ceaeaac61924fc5591435c7",
        )
        self.assert_rejected(value, "retained receipt file missing", "final")

    def test_final_rejects_input_digest_not_computed_from_paths(self) -> None:
        value = self.manifest()
        self.complete(value)
        self.row(value, "default-tree")["current_input_sha256"] = "8" * 64
        self.assert_rejected(value, "computed input digest", "final")

    def test_final_rejects_nonexistent_candidate_and_tampered_artifact(self) -> None:
        for mutation, fragment in (
            ("candidate", "candidate commit"),
            ("artifact", "artifact sha256 mismatch"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                if mutation == "candidate":
                    value["candidate"]["sha"] = "9" * 40
                    for row in value["obligations"]:
                        row["candidate_sha"] = "9" * 40
                    overlay = self.repo / value["ce_overlay"]["path"]
                    overlay_value = json.loads(overlay.read_text(encoding="utf-8"))
                    overlay_value["candidate_sha"] = "9" * 40
                    overlay.write_text(json.dumps(overlay_value), encoding="utf-8")
                else:
                    artifact = (
                        self.repo
                        / value["candidate"]["artifacts"]["python-wheel"]["path"]
                    )
                    artifact.write_text("tampered\n", encoding="utf-8")
                self.assert_rejected(value, fragment, "final")

    def test_final_rejects_unsealed_legacy_command_or_too_small_count(self) -> None:
        for mutation, fragment in (
            ("command", "legacy command contract"),
            ("count", "positive count contract"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                row = self.row(value, "default-tree")
                if mutation == "command":
                    row["commands"] = ["true"]
                else:
                    row["evidence"][0]["tests"] = 1
                self.assert_rejected(value, fragment, "final")

    def test_final_rejects_tc5_bridge_candidate_or_fidelity_drift(self) -> None:
        for mutation, fragment in (
            ("candidate", "TC-5 candidate binding"),
            ("fidelity", "TC-5 bridge equivalence"),
        ):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                self.complete(value)
                row = self.row(value, "tc5-bridge")
                path = self.repo / row["evidence"][0]["path"]
                receipt = json.loads(path.read_text(encoding="utf-8"))
                if mutation == "candidate":
                    receipt["candidate"]["sha"] = "9" * 40
                else:
                    receipt["sut_result_sha256"] = "9" * 64
                path.write_text(json.dumps(receipt), encoding="utf-8")
                row["evidence"][0]["sha256"] = digest(path)
                self.assert_rejected(value, fragment, "final")

    def test_final_rejects_eu7_without_positive_ac073_stress(self) -> None:
        value = self.manifest()
        self.complete(value)
        row = self.row(value, "eu7-real")
        path = self.repo / row["evidence"][0]["path"]
        receipt = json.loads(path.read_text(encoding="utf-8"))
        receipt["ac073_stress"] = "fail"
        path.write_text(json.dumps(receipt), encoding="utf-8")
        row["evidence"][0]["sha256"] = digest(path)
        self.assert_rejected(value, "EU7 AC-073 stress", "final")

    def test_tc5_route_installs_and_invokes_the_pinned_wheel(self) -> None:
        value = self.manifest()
        row = self.row(value, "tc5-bridge")
        row["commands"] = [
            command
            for command in row["commands"]
            if "pip install --no-deps" not in command
        ]
        self.assert_rejected(value, "TC-5 bridge command contract")

    def test_runtime_route_requires_fresh_typescript_test_compilation(self) -> None:
        value = self.manifest()
        row = self.row(value, "runtime-configuration")
        row["commands"] = [
            command.replace(
                "src/ts/node_modules/.bin/tsc -p src/ts/tsconfig.json && ", ""
            )
            for command in row["commands"]
        ]
        self.assert_rejected(value, "TypeScript compile order")

    def test_linux_artifact_route_uses_package_metadata_and_rooted_tsconfig(self) -> None:
        for mutation in ("cwd", "locked", "release", "output", "tsconfig"):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                row = self.row(value, "linux-artifact-build")
                if mutation == "cwd":
                    row["commands"][0] = row["commands"][0].replace(
                        "cd src/python && ", ""
                    )
                elif mutation == "locked":
                    row["commands"][0] = row["commands"][0].replace("--locked ", "")
                elif mutation == "release":
                    row["commands"][0] = row["commands"][0].replace("--release ", "")
                elif mutation == "output":
                    row["commands"][0] = row["commands"][0].replace(
                        "../../${RUN_DIR}/artifacts/python",
                        "${RUN_DIR}/artifacts/python",
                    )
                else:
                    row["commands"][-1] = (
                        "npm exec --prefix src/ts -- tsc -p tsconfig.build.json"
                    )
                self.assert_rejected(value, "Linux artifact build command contract")

    def test_linux_cuda_route_seals_executor_and_toolkit_inputs(self) -> None:
        for mutation in ("gpu", "toolkit", "toolkit-path", "seal-sha", "smoke-sha"):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                row = self.row(value, "linux-cuda-package")
                if mutation == "gpu":
                    row["commands"][0] = row["commands"][0].replace(
                        "FATHOMDB_CUDA_GPU_UUID=GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b ",
                        "",
                    )
                elif mutation == "toolkit":
                    row["commands"][1] = row["commands"][1].replace(
                        "CUDA_HOME=/usr/local/cuda-12.6 ", ""
                    )
                elif mutation == "toolkit-path":
                    row["commands"][1] = row["commands"][1].replace(
                        "PATH=/usr/local/cuda-12.6/bin:${PATH} ", ""
                    )
                elif mutation == "seal-sha":
                    row["commands"][3] = row["commands"][3].replace(
                        "${FINAL_SHA}", "0000000000000000000000000000000000000000"
                    )
                else:
                    row["commands"][4] = row["commands"][4].replace(
                        "--candidate-sha ${FINAL_SHA} ", ""
                    )
                self.assert_rejected(value, "Linux CUDA package command contract")

    def test_ac034c_is_the_only_unavailable_row_and_never_passes(self) -> None:
        for mutation in ("other-unavailable", "ac034c-pass"):
            with self.subTest(mutation=mutation):
                value = self.manifest()
                if mutation == "other-unavailable":
                    self.row(value, "default-tree")["disposition"] = "unavailable"
                else:
                    self.row(value, "ac034c")["verdict"] = "pass"
                self.assert_rejected(value, "AC-034c")

    def test_ac034c_order_does_not_matter(self) -> None:
        value = self.manifest()
        value["obligations"].insert(0, value["obligations"].pop())
        result = self.run_checker(value)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_ce_overlay_may_only_change_candidate_sha(self) -> None:
        overlay_path = self.repo / self.manifest()["ce_overlay"]["path"]
        overlay = json.loads(overlay_path.read_text(encoding="utf-8"))
        overlay["steady_calls"] += 1
        overlay_path.write_text(json.dumps(overlay), encoding="utf-8")
        self.assert_rejected(self.manifest(), "CE overlay")

    def test_final_accepts_only_the_documented_ce_non_pass(self) -> None:
        value = self.manifest()
        self.complete(value)
        row = self.row(value, "slice72-ce")
        row["origin"] = "authorized-exception"
        row["disposition"] = "accepted-non-pass"
        row["retained_receipt"] = str(CE_EXCEPTION.relative_to(ROOT))
        row["retained_receipt_sha256"] = digest(CE_EXCEPTION)
        row["verdict"] = "accepted-non-pass"
        row["evidence"][0]["tests"] = 4
        row["evidence"][0]["verdict"] = "accepted-non-pass"
        result = self.run_checker(value, "final")
        self.assertEqual(result.returncode, 0, result.stderr)

        value = self.manifest()
        self.row(value, "default-tree")["disposition"] = "accepted-non-pass"
        self.assert_rejected(value, "accepted non-pass")

    def test_final_rejects_quarantined_evidence_and_artifacts(self) -> None:
        for kind in ("evidence", "artifact"):
            with self.subTest(kind=kind):
                value = self.manifest()
                self.complete(value)
                if kind == "evidence":
                    source = self.repo / self.row(value, "default-tree")["evidence"][0][
                        "path"
                    ]
                    target = (
                        self.repo
                        / "dev/plans/runs/0.8.25-slice-85/invalid-evidence/log.txt"
                    )
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(source.read_bytes())
                    self.row(value, "default-tree")["evidence"][0]["path"] = str(
                        target.relative_to(self.repo)
                    )
                else:
                    artifact = value["candidate"]["artifacts"]["python-wheel"]
                    source = self.repo / artifact["path"]
                    target = self.repo / "invalid-artifacts/python-wheel.bin"
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(source.read_bytes())
                    artifact["path"] = str(target.relative_to(self.repo))
                self.assert_rejected(value, "quarantined", phase="final")


class Slice85InstalledRuntimeContractTest(unittest.TestCase):
    def test_smoke_runs_six_fresh_runtime_configuration_processes(self) -> None:
        text = SMOKE.read_text(encoding="utf-8")
        for marker in (
            "slice85-runtime-python-performance: pass",
            "slice85-runtime-python-conflict: pass",
            "slice85-runtime-python-invalid: pass",
            "slice85-runtime-node-performance: pass",
            "slice85-runtime-node-conflict: pass",
            "slice85-runtime-node-invalid: pass",
            "slice85-runtime-configuration-result: python=3 node=3 skipped=0",
        ):
            with self.subTest(marker=marker):
                self.assertIn(marker, text)

    def test_windows_runner_normalizes_powershell_crlf_status(self) -> None:
        text = WINDOWS_RUNNER.read_text(encoding="utf-8")
        self.assertIn("tr -d '\\r' | grep -Fx Running", text)


class Slice85NodeSupportContractTest(unittest.TestCase):
    def test_package_and_docs_support_only_node_25(self) -> None:
        package = json.loads((ROOT / "src/ts/package.json").read_text(encoding="utf-8"))
        self.assertEqual(package.get("engines", {}).get("node"), ">=25 <26")
        install = (ROOT / "docs/install/typescript.md").read_text(encoding="utf-8")
        compatibility = (ROOT / "docs/compatibility/index.md").read_text(
            encoding="utf-8"
        )
        readme = (ROOT / "src/ts/README.md").read_text(encoding="utf-8")
        for text in (install, compatibility, readme):
            self.assertIn("25.9.0", text)
            self.assertNotIn("24.19.0", text)

    def test_ci_and_release_node_is_25(self) -> None:
        for relative in (
            ".github/workflows/aarch64-release-preflight.yml",
            ".github/workflows/ci.yml",
            ".github/workflows/release.yml",
        ):
            text = (ROOT / relative).read_text(encoding="utf-8")
            setup_count = text.count("uses: actions/setup-node@")
            self.assertGreater(setup_count, 0)
            self.assertEqual(text.count('node-version: "25.9.0"'), setup_count)
            self.assertNotIn('node-version: "24.19.0"', text)

    def test_node_25_arm64_archive_digest_is_sealed(self) -> None:
        expected = "bf007bf0dcc2fddd90888fde374a1ad33c1ab2ca2ad324c645dd7aed0f9f1460"
        for relative in (
            "scripts/release/Dockerfile.napi-manylinux",
            "scripts/release/napi-artifact-contract.sh",
        ):
            text = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("25.9.0", text)
            self.assertIn(expected, text)


if __name__ == "__main__":
    unittest.main()
