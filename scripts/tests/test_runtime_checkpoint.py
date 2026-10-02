#!/usr/bin/env python3
"""Behavioral tests for the runtime-checkpoint release-state gate."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
GATE = REPO_ROOT / "scripts" / "check-runtime-checkpoints.py"
EVIDENCE_DIR = Path("dev/plans/0.8.27/features/slice-90")
MATRIX_CELLS = ("2/1", "2/5", "1/1", "2/2", "4/4", "64/64", "2/no-provider")
PROTOCOL = json.loads((REPO_ROOT / EVIDENCE_DIR / "d27-runtime-qualification-protocol-v2.json").read_text())
LEGACY_PROTOCOL_PATH = REPO_ROOT / EVIDENCE_DIR / "d27-runtime-qualification-protocol.json"
RELEASE_SELECTORS = ("AC-011a", "AC-011b", "AC-017", "AC-018", "AC-029", "AC-072", "AC-073", "AC-076", "AC-081a", "AC-081b", "AC-081c")
AC073_COMMAND = (
    "env CARGO_TARGET_DIR={bundle}/target AGENT_LONG=1 EU7_N_VALUES=7667 EU7_QUERIES=100 EU7_BOOTSTRAP=1000 "
    "EU7_LATENCY_SAMPLES=1000 EU7_STRESS_PER_THREAD=250 "
    "FATHOMDB_EU7_OUTPUT={bundle}/eu7.json cargo test --release "
    "-p fathomdb-engine --features operator,embed-cuda --test eu7_real_corpus_ac "
    "eu7_real_corpus_ac_validation -- --exact --ignored --nocapture --test-threads=1"
)


def load_script(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


D27_RUNNER = load_script("checkpoint_d27_runner", REPO_ROOT / "scripts/d27-runtime-runner.py")
D27_VERIFIER = load_script("checkpoint_d27_verifier", REPO_ROOT / "scripts/d27-runtime-qualification.py")


class RuntimeCheckpointGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        (self.root / EVIDENCE_DIR).mkdir(parents=True)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def run_gate(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["python3", str(GATE), "--root", str(self.root)],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def write_state(self, checkpoint: dict[str, object]) -> None:
        state = {
            "release": "0.8.27",
            "ladder": [
                {
                    "slice": 90,
                    "status": "PLANNED",
                    "runtime_checkpoint": checkpoint,
                }
            ],
        }
        (self.root / "dev" / "plans" / "release-state-0.8.27.json").write_text(
            json.dumps(state), encoding="utf-8"
        )

    @staticmethod
    def pending_receipt(path: str) -> dict[str, object]:
        return {"path": path, "status": "PENDING", "candidate_sha": None, "sha256": None}

    def pending_checkpoint(self) -> dict[str, object]:
        return {
            "status": "PENDING",
            "candidate_sha": None,
            "binding_sha": None,
            "stage3_start_sha": None,
            "receipts": {
                "performance": self.pending_receipt(str(EVIDENCE_DIR / "runtime-performance-qualification.md")),
                "code_review": self.pending_receipt(str(EVIDENCE_DIR / "code-review.md")),
                "verification": self.pending_receipt(str(EVIDENCE_DIR / "review-verification.md")),
            },
        }

    def test_pending_checkpoint_with_predeclared_receipts_passes(self) -> None:
        self.write_state(self.pending_checkpoint())
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

    @staticmethod
    def raw_repetition(phase: str, direction: str, repetition: int) -> dict:
        counts = PROTOCOL["execution"][f"{direction}_epoch"]
        classes = (["canonical_write"] * counts["canonical_writes"]
                   + ["foreground_hybrid_query"] * counts["foreground_hybrid_queries"]
                   + ["direct_embed"] * counts["direct_embeds"])
        operations = []
        completions = []
        events = []
        for sequence, kind in enumerate(classes):
            admitted = 1_000_000_000 + sequence * 1_000_000
            completed = admitted + 100_000
            operation = {"class": kind, "sequence": sequence, "admitted_ns": admitted,
                         "completed_ns": completed, "outcome": "completed"}
            if kind == "canonical_write":
                operation["cursor"] = f"cursor-{sequence}"
                completions.append({"cursor": operation["cursor"], "committed_ns": completed,
                                    "observed_ns": completed + 100_000})
                owner = {"projection_cursors": [operation["cursor"]]}
            else:
                owner = {"operation_sequence": sequence}
            operations.append(operation)
            events.append({"source": "engine", "request_id": sequence,
                           "admitted_ns": admitted + 100, "started_ns": admitted + 1_000,
                           "terminal_ns": admitted + 2_000, "owner": owner})
        observation = {
            "pid_namespace": PROTOCOL["runner"]["host_pid_namespace"],
            "pid_one_namespace": PROTOCOL["runner"]["host_pid_namespace"],
            "pid_one_comm": "systemd", "ps_pid_one_comm": "systemd", "procfs_hidepid": "0",
            "runner_pid": 4242, "proc_self_pid": 4242, "ps_self_pid": 4242,
            "cpu_governor": "performance", "competing_processes": [],
            "swap_pages_in": 0, "swap_pages_out": 0, "database_device": "/dev/nvme0n1",
        }
        raw = {
            "direction": direction, "repetition": repetition, "smoke": False,
            "warmup_seconds": 10, "measurement_seconds": 60, "epoch_size": 10,
            "measurement_elapsed_ns": 60_000_000_000,
            "operation_counts": counts, "operations": operations,
            "projection_completions": completions, "close_result": "Ok(())",
            "close_start_ns": 20_000_000_000, "close_end_ns": 20_001_000_000,
            "residual_workers_after_close": 0, "projection_backlog_high_water": 1,
            "engine_thread_inventory": 12 if phase == "entry" else 16, "provider_peak_concurrency": 1,
            "environment_start": observation, "environment_end": observation,
            "environment_samples": [observation], "environment_valid": True,
            "connection_inventory": "Err(Storage)",
        }
        if phase == "candidate":
            raw.update(
                connection_inventory="live=writer:1,readers:8,dispatcher:1,workers:2,probes:0",
                configuration_observation={"source": "engine", "scheduler_runtime_threads": 2,
                                           "embedder_pool_size": 5},
                projection_admission_observation={"source": "engine", "active_plus_queued_high_water": 1},
                embed_dispatch_events=events,
                embed_requests_waiting_high_water=1,
            )
        return raw

    def write_d27_bundles(self, candidate: str) -> tuple[str, str, str]:
        protocol_path = REPO_ROOT / EVIDENCE_DIR / "d27-runtime-qualification-protocol-v2.json"
        entry_receipt = None
        paths = {}
        for phase in ("entry", "candidate"):
            phase_protocol_path = LEGACY_PROTOCOL_PATH if phase == "entry" else protocol_path
            phase_protocol = json.loads(phase_protocol_path.read_text())
            bundle = self.root / "evidence" / phase
            binary = bundle / "target/release/deps/d27_runtime_workload-test"
            binary.parent.mkdir(parents=True)
            artifacts = {"runner": bundle / "runner.bundle", "binary": binary,
                         "corpus": bundle / "corpus.jsonl", "raw": bundle / "raw-output.jsonl"}
            for name, path in artifacts.items():
                path.write_bytes(b"fixture-" + name.encode())
            artifacts["runner"].write_bytes(
                b"fixture-runner\n--PROTOCOL--\n" + phase_protocol_path.read_bytes()
            )
            raw = []
            metrics = {"projection_heavy": [], "foreground_heavy": []}
            for name in D27_RUNNER.repetition_order(phase_protocol, phase):
                direction = "projection_heavy" if "projection_heavy" in name else "foreground_heavy"
                repetition = int(name[-1])
                item = self.raw_repetition(phase, direction, repetition)
                raw.append(item)
                metrics[direction].append(D27_RUNNER.summarize_raw(item, phase, phase_protocol))
            artifacts["raw"].write_text("".join(json.dumps(item) + "\n" for item in raw))
            receipt = {
                "phase": phase,
                "source_sha": PROTOCOL["entry_engine_candidate_sha"] if phase == "entry" else candidate,
                "status": "PASS", "protocol_sha256": hashlib.sha256(phase_protocol_path.read_bytes()).hexdigest(),
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "runner_sha256": hashlib.sha256(artifacts["runner"].read_bytes()).hexdigest(),
                "corpus_sha256": hashlib.sha256(artifacts["corpus"].read_bytes()).hexdigest(),
                "raw_output_sha256": hashlib.sha256(artifacts["raw"].read_bytes()).hexdigest(),
                "runner_inventory": {"host": "windchill3", "operating_system": "Linux x86_64",
                                     "online_cpus": 8, "memory_gib": 16, "database_storage": "local NVMe",
                                     "build": "cargo test --release", "features": ["test-hooks"]},
                "environment_start": raw[0]["environment_start"],
                "environment_end": raw[-1]["environment_end"],
                "per_repetition_metrics": metrics,
                "aggregate_metrics": {}, "decision_rule_evaluation": {},
                "historical_unavailable": phase_protocol["metrics"]["historical_unavailable"] if phase == "entry" else [],
            }
            D27_VERIFIER.verify_raw_linkage(receipt, phase_protocol, artifacts["raw"])
            if phase == "candidate":
                entry_receipt = D27_VERIFIER.validate_entry_for_candidate(
                    entry_receipt, PROTOCOL, protocol_path,
                    {"runner": paths["entry"] / "runner.bundle",
                     "binary": paths["entry"] / "target/release/deps/d27_runtime_workload-test",
                     "corpus": paths["entry"] / "corpus.jsonl",
                     "raw": paths["entry"] / "raw-output.jsonl"},
                )
            receipt = D27_VERIFIER.validate_receipt(receipt, phase_protocol, phase_protocol_path, artifacts, entry_receipt)
            (bundle / "receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
            paths[phase] = bundle
            if phase == "entry":
                entry_receipt = receipt
        candidate_receipt = self.root / EVIDENCE_DIR / "d27-candidate-receipt.json"
        candidate_receipt.write_bytes((paths["candidate"] / "receipt.json").read_bytes())
        return (str(paths["entry"].relative_to(self.root)),
                str(paths["candidate"].relative_to(self.root)),
                hashlib.sha256(candidate_receipt.read_bytes()).hexdigest())

    def write_performance(self, candidate: str, cells: tuple[str, ...] = MATRIX_CELLS, d27: bool = True) -> str:
        rows = "\n".join(f"| {cell} | PASS | {candidate} | cargo test -p fathomdb-engine slice90_matrix_{cell.replace('/', '_').replace('-', '_')} | 3 passed; exact resource inventory and cleanup |" for cell in cells)
        qualification = ""
        if d27:
            d27_path = EVIDENCE_DIR / "d27-candidate-receipt.json"
            entry_bundle, candidate_bundle, candidate_digest = self.write_d27_bundles(candidate)
            qualification = (
                "\n## D27 candidate qualification\n\n"
                "| Receipt path | SHA-256 | Status |\n| --- | --- | --- |\n"
                f"| {d27_path} | {candidate_digest} | PASS |\n\n"
                "## D27 artifact bundles\n\n"
                "| Phase | Bundle directory | Receipt SHA-256 |\n| --- | --- | --- |\n"
                f"| entry | {entry_bundle} | {hashlib.sha256((self.root / entry_bundle / 'receipt.json').read_bytes()).hexdigest()} |\n"
                f"| candidate | {candidate_bundle} | {candidate_digest} |\n"
            )
        return (
            "# Runtime performance qualification\n\n"
            "## Configuration matrix\n\n"
            "| Cell | Status | Candidate SHA | Command | Evidence |\n| --- | --- | --- | --- | --- |\n"
            f"{rows}\n\n"
            "## Named release selectors\n\n"
            "| Selector | Status | Candidate SHA | Command | Evidence |\n| --- | --- | --- | --- | --- |\n"
            + "\n".join(f"| {selector} | PASS | {candidate} | cargo test -p fathomdb-engine {selector} | 1 passed; unchanged selector |" for selector in RELEASE_SELECTORS)
            + "\n\n## Installed bindings\n\n"
            "| Language | Status | Candidate SHA | Command | Evidence |\n| --- | --- | --- | --- | --- |\n"
            f"| Python | PASS | {candidate} | python3 -m pytest installed_binding | 36 passed; clean consumer install |\n"
            f"| Node | PASS | {candidate} | npm test -- installed_binding | 18 passed; clean consumer install |\n"
            + qualification
        )

    def pass_checkpoint(self, *, cells: tuple[str, ...] = MATRIX_CELLS, d27: bool = True) -> dict[str, object]:
        def git(*args: str) -> str:
            return subprocess.run(["git", "-C", str(self.root), *args], check=True, text=True, capture_output=True).stdout.strip()

        git("init", "-q")
        git("config", "user.email", "runtime-checkpoint@example.invalid")
        git("config", "user.name", "Runtime Checkpoint Test")
        (self.root / "candidate.txt").write_text("candidate\n", encoding="utf-8")
        git("add", "candidate.txt")
        git("commit", "-qm", "candidate")
        candidate = git("rev-parse", "HEAD")
        checkpoint = self.pending_checkpoint()
        for name, receipt in checkpoint["receipts"].items():  # type: ignore[union-attr]
            path = self.root / str(receipt["path"])
            if name == "performance":
                body = self.write_performance(candidate, cells, d27)
            else:
                reviewer = "gpt-6-sol high" if name == "code_review" else "Terra"
                body = (f"# {name}\n\nCandidate SHA: {candidate}\nVerdict: PASS\nReviewer: {reviewer}\n"
                        "Evidence: Independent review of the exact candidate found no open findings; "
                        "focused runtime and checkpoint tests passed with complete receipts.\n")
            path.write_text(body, encoding="utf-8")
            receipt.update(status="PASS", candidate_sha=candidate, sha256=hashlib.sha256(path.read_bytes()).hexdigest())
        git("add", "dev/plans/0.8.27/features/slice-90")
        git("commit", "-qm", "bind checkpoint receipts")
        checkpoint.update(status="PASS", candidate_sha=candidate, binding_sha=git("rev-parse", "HEAD"))
        self.write_state(checkpoint)
        return checkpoint

    def test_complete_pass_checkpoint_accepts_required_matrix_and_d27(self) -> None:
        self.pass_checkpoint()
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_v2_candidate_rejects_legacy_entry_with_wrong_protocol_hash(self) -> None:
        self.pass_checkpoint()
        bundle = self.root / "evidence/entry"
        receipt = json.loads((bundle / "receipt.json").read_text())
        receipt["protocol_sha256"] = hashlib.sha256(
            (REPO_ROOT / EVIDENCE_DIR / "d27-runtime-qualification-protocol-v2.json").read_bytes()
        ).hexdigest()
        (bundle / "receipt.json").write_text(json.dumps(receipt))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27", result.stdout)

    def test_v2_candidate_rejects_legacy_entry_with_modified_raw(self) -> None:
        self.pass_checkpoint()
        bundle = self.root / "evidence/entry"
        raw = bundle / "raw-output.jsonl"
        raw.write_bytes(raw.read_bytes().replace(b'"environment_valid": true', b'"environment_valid": false', 1))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27", result.stdout)

    def test_pass_performance_requires_every_matrix_cell(self) -> None:
        self.pass_checkpoint(cells=MATRIX_CELLS[:-1])
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("missing matrix cell: 2/no-provider", result.stdout)

    def test_pass_performance_requires_default_five_matrix_cell(self) -> None:
        self.pass_checkpoint(cells=tuple(cell for cell in MATRIX_CELLS if cell != "2/5"))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("missing matrix cell: 2/5", result.stdout)

    def write_ac073_stress_receipt(self, checkpoint: dict[str, object]) -> tuple[Path, Path, Path]:
        candidate = checkpoint["candidate_sha"]
        eu7_path = self.root / EVIDENCE_DIR / "ac073-eu7.json"
        log_path = self.root / EVIDENCE_DIR / "ac073-run.log"
        receipt_path = self.root / EVIDENCE_DIR / "ac073-stress-receipt.json"
        manifest_path = self.root / EVIDENCE_DIR / "ac073-execution.json"
        bundle = self.root / "evidence" / "ac073-bundle"
        binary_relative = Path("target/release/deps/eu7_real_corpus_ac-test")
        binary = bundle / binary_relative
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"frozen EU7 candidate test binary")
        executed_binary = binary
        eu7_path.write_text(json.dumps({
            "config": {"n_values_requested": [7667], "real_corpus_docs": 18472},
            "ac_019_real_dev_box": [{"n": 7667, "padded_with_synthetic_distractors": False,
                                     "baseline_p99_ms": 49, "p99_ms": 418, "bound_ms": 491,
                                     "passed": True}],
            "ac_013b_real_dev_box": [{"n": 7667, "recall_at_10": 0.772,
                                      "ci_lo": 0.743, "ci_hi": 0.798,
                                      "current_floor_0_90": 0.9,
                                      "passes_ci_gate_0_8_0_one_sided": False}],
        }))
        log_path.write_text(
            f"     Running tests/eu7_real_corpus_ac.rs ({executed_binary})\n"
            "EU7_SETUP real_docs=18472 queries=100 n_values=[7667] bootstrap=1000 "
            "latency_samples=1000 stress_per_thread=250\n"
            "EU7_NUMBERS n=7667 padded=false stress_p99_ms=418 stress_bound_ms=491 "
            "ac013=true ac019=true\n"
            "EU7_WROTE /tmp/eu7.json\n"
            "AC-075 recall verdict: recall_ci_hi 0.7980 < floor 0.9\n"
            "FAILED\n"
            "test result: FAILED. 0 passed; 1 failed; 0 ignored\n"
        )
        manifest = {
            "schema_version": "fathomdb.slice90-ac073-execution/v1",
            "candidate_sha": candidate,
            "pre_source_sha": candidate,
            "post_source_sha": candidate,
            "pre_clean": True,
            "post_clean": True,
            "command": AC073_COMMAND.format(bundle=bundle),
            "selector_exit": 101,
            "bundle_dir": str(bundle),
            "binary_relative_path": binary_relative.as_posix(),
            "executed_binary_path": str(executed_binary),
            "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "source_receipt_sha256": hashlib.sha256(eu7_path.read_bytes()).hexdigest(),
            "raw_log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest(),
        }
        manifest_path.write_text(json.dumps(manifest))
        receipt = {
            "schema_version": "fathomdb.slice90-ac073-stress/v1",
            "candidate_sha": candidate,
            "selector_exit": 101,
            "ac073_stress": "pass",
            "ac075": "superseded-by-tc5",
            "stress_p99_ms": 418,
            "stress_bound_ms": 491,
            "source_receipt": str(EVIDENCE_DIR / "ac073-eu7.json"),
            "source_receipt_sha256": hashlib.sha256(eu7_path.read_bytes()).hexdigest(),
            "raw_log": str(EVIDENCE_DIR / "ac073-run.log"),
            "raw_log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest(),
            "execution_manifest": str(EVIDENCE_DIR / "ac073-execution.json"),
            "execution_manifest_sha256": hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
            "retained_ac075_result": {"verdict": "fail", "recall_at_10": 0.772,
                                            "ci_95": [0.743, 0.798]},
        }
        receipt_path.write_text(json.dumps(receipt))
        self.rebind_ac073(checkpoint, receipt_path)
        return receipt_path, eu7_path, log_path

    def rebind_ac073(self, checkpoint: dict[str, object], receipt_path: Path) -> None:
        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        lines = path.read_text().splitlines()
        for index, line in enumerate(lines):
            if line.startswith("| AC-073 |"):
                lines[index] = (
                    f"| AC-073 | PASS | {checkpoint['candidate_sha']} | "
                    "cargo test --release -p fathomdb-engine eu7_real_corpus_ac_validation | "
                    f"stress receipt={EVIDENCE_DIR / 'ac073-stress-receipt.json'} "
                    f"sha256={hashlib.sha256(receipt_path.read_bytes()).hexdigest()} "
                    "combined-exit=101 AC-075=superseded-by-tc5 |"
                )
        path.write_text("\n".join(lines) + "\n")
        self._rebind_performance(checkpoint)

    def test_ac073_stress_receipt_preserves_combined_failure(self) -> None:
        checkpoint = self.pass_checkpoint()
        self.write_ac073_stress_receipt(checkpoint)
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_ac073_stress_receipt_rejects_rehashed_stress_failure(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt_path, eu7_path, _ = self.write_ac073_stress_receipt(checkpoint)
        eu7 = json.loads(eu7_path.read_text())
        eu7["ac_019_real_dev_box"][0]["p99_ms"] = 492
        eu7["ac_019_real_dev_box"][0]["passed"] = False
        eu7_path.write_text(json.dumps(eu7))
        receipt = json.loads(receipt_path.read_text())
        receipt["stress_p99_ms"] = 492
        receipt["source_receipt_sha256"] = hashlib.sha256(eu7_path.read_bytes()).hexdigest()
        receipt_path.write_text(json.dumps(receipt))
        self.rebind_ac073(checkpoint, receipt_path)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("AC-073 stress", result.stdout)

    def test_ac073_stress_rejects_rehashed_wrong_candidate_execution(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt_path, _, _ = self.write_ac073_stress_receipt(checkpoint)
        manifest_path = self.root / EVIDENCE_DIR / "ac073-execution.json"
        manifest = json.loads(manifest_path.read_text())
        manifest["pre_source_sha"] = "0" * 40
        manifest_path.write_text(json.dumps(manifest))
        receipt = json.loads(receipt_path.read_text())
        receipt["execution_manifest_sha256"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
        receipt_path.write_text(json.dumps(receipt))
        self.rebind_ac073(checkpoint, receipt_path)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("AC-073 execution", result.stdout)

    def test_ac073_stress_rejects_missing_sealed_binary(self) -> None:
        checkpoint = self.pass_checkpoint()
        self.write_ac073_stress_receipt(checkpoint)
        manifest = json.loads((self.root / EVIDENCE_DIR / "ac073-execution.json").read_text())
        (Path(manifest["bundle_dir"]) / manifest["binary_relative_path"]).unlink()
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("AC-073 execution", result.stdout)

    def test_ac073_stress_rejects_changed_command_and_dirty_checkout(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt_path, _, _ = self.write_ac073_stress_receipt(checkpoint)
        manifest_path = self.root / EVIDENCE_DIR / "ac073-execution.json"
        manifest = json.loads(manifest_path.read_text())
        manifest["command"] = manifest["command"].replace("EU7_N_VALUES=7667", "EU7_N_VALUES=100")
        manifest["post_clean"] = False
        manifest_path.write_text(json.dumps(manifest))
        receipt = json.loads(receipt_path.read_text())
        receipt["execution_manifest_sha256"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
        receipt_path.write_text(json.dumps(receipt))
        self.rebind_ac073(checkpoint, receipt_path)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("AC-073 execution", result.stdout)

    def test_ac073_stress_rejects_cargo_target_that_cannot_produce_sealed_binary(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt_path, _, _ = self.write_ac073_stress_receipt(checkpoint)
        manifest_path = self.root / EVIDENCE_DIR / "ac073-execution.json"
        manifest = json.loads(manifest_path.read_text())
        bundle = manifest["bundle_dir"]
        manifest["command"] = manifest["command"].replace(
            f"CARGO_TARGET_DIR={bundle}/target", f"CARGO_TARGET_DIR={bundle}"
        )
        manifest_path.write_text(json.dumps(manifest))
        receipt = json.loads(receipt_path.read_text())
        receipt["execution_manifest_sha256"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
        receipt_path.write_text(json.dumps(receipt))
        self.rebind_ac073(checkpoint, receipt_path)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("candidate command binding mismatch", result.stdout)

    def test_ac073_stress_rejects_binary_path_not_in_raw_log(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt_path, _, log_path = self.write_ac073_stress_receipt(checkpoint)
        log_path.write_text(log_path.read_text().replace("eu7_real_corpus_ac-test", "other-test"))
        manifest_path = self.root / EVIDENCE_DIR / "ac073-execution.json"
        manifest = json.loads(manifest_path.read_text())
        manifest["raw_log_sha256"] = hashlib.sha256(log_path.read_bytes()).hexdigest()
        manifest_path.write_text(json.dumps(manifest))
        receipt = json.loads(receipt_path.read_text())
        receipt["raw_log_sha256"] = manifest["raw_log_sha256"]
        receipt["execution_manifest_sha256"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
        receipt_path.write_text(json.dumps(receipt))
        self.rebind_ac073(checkpoint, receipt_path)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("AC-073 execution", result.stdout)

    def test_ac073_stress_rejects_distinct_executed_binary(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt_path, _, log_path = self.write_ac073_stress_receipt(checkpoint)
        manifest_path = self.root / EVIDENCE_DIR / "ac073-execution.json"
        manifest = json.loads(manifest_path.read_text())
        sealed = Path(manifest["executed_binary_path"])
        alternate = self.root / manifest["binary_relative_path"]
        alternate.parent.mkdir(parents=True)
        alternate.write_bytes(b"different test executable")
        manifest["executed_binary_path"] = str(alternate)
        log_path.write_text(log_path.read_text().replace(str(sealed), str(alternate)))
        manifest["raw_log_sha256"] = hashlib.sha256(log_path.read_bytes()).hexdigest()
        manifest_path.write_text(json.dumps(manifest))
        receipt = json.loads(receipt_path.read_text())
        receipt["raw_log_sha256"] = manifest["raw_log_sha256"]
        receipt["execution_manifest_sha256"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
        receipt_path.write_text(json.dumps(receipt))
        self.rebind_ac073(checkpoint, receipt_path)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("executed test executable path differs", result.stdout)

    def test_ac073_stress_exception_does_not_apply_to_ac072(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt_path, _, _ = self.write_ac073_stress_receipt(checkpoint)
        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        content = path.read_text()
        content = content.replace("| AC-072 | PASS |", "| AC-072 | PASS |", 1).replace(
            "| 1 passed; unchanged selector |\n| AC-073 |",
            f"| stress receipt={EVIDENCE_DIR / 'ac073-stress-receipt.json'} "
            f"sha256={hashlib.sha256(receipt_path.read_bytes()).hexdigest()} "
            "combined-exit=101 AC-075=superseded-by-tc5 |\n| AC-073 |", 1,
        )
        path.write_text(content)
        self._rebind_performance(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("incomplete candidate-bound named selector: AC-072", result.stdout)

    def test_pass_performance_requires_candidate_d27_qualification(self) -> None:
        self.pass_checkpoint(d27=False)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27 candidate qualification", result.stdout)

    def test_receipt_path_outside_declared_slice90_evidence_set_is_rejected(self) -> None:
        checkpoint = self.pass_checkpoint()
        receipt = checkpoint["receipts"]["performance"]  # type: ignore[index]
        outside = self.root / "dev/plans/other-performance.md"
        outside.write_bytes((self.root / str(receipt["path"])).read_bytes())
        receipt["path"] = "dev/plans/other-performance.md"
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("outside declared Slice 90 evidence set", result.stdout)

    def test_zero_structured_checkpoints_fails_closed(self) -> None:
        state = {"release": "9.9.9", "ladder": [{"slice": 90, "status": "PLANNED"}]}
        (self.root / "dev" / "plans" / "release-state-9.9.9.json").write_text(
            json.dumps(state), encoding="utf-8"
        )
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("zero structured checkpoints", result.stdout)

    def test_stage3_cannot_start_while_checkpoint_is_pending(self) -> None:
        checkpoint = self.pending_checkpoint()
        checkpoint["stage3_start_sha"] = "a" * 40
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stage3_start_sha requires a PASS checkpoint", result.stdout)

    def test_pass_checkpoint_rejects_missing_receipt(self) -> None:
        checkpoint = self.pending_checkpoint()
        checkpoint.update(
            status="PASS",
            candidate_sha="a" * 40,
            binding_sha="b" * 40,
        )
        for receipt in checkpoint["receipts"].values():  # type: ignore[union-attr]
            receipt.update(status="PASS", candidate_sha="a" * 40, sha256="0" * 64)
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("receipt file does not exist", result.stdout)

    def test_pass_checkpoint_rejects_receipt_hash_or_candidate_drift(self) -> None:
        checkpoint = self.pending_checkpoint()
        checkpoint.update(
            status="PASS",
            candidate_sha="a" * 40,
            binding_sha="b" * 40,
        )
        for name, receipt in checkpoint["receipts"].items():  # type: ignore[union-attr]
            path = self.root / str(receipt["path"])
            path.write_text(f"{name}\n", encoding="utf-8")
            receipt.update(
                status="PASS",
                candidate_sha="a" * 40,
                sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            )
        checkpoint["receipts"]["verification"]["candidate_sha"] = "c" * 40  # type: ignore[index]
        checkpoint["receipts"]["code_review"]["sha256"] = "d" * 64  # type: ignore[index]
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("code_review receipt sha256 mismatch", result.stdout)
        self.assertIn("verification receipt candidate_sha", result.stdout)

    def test_pass_checkpoint_requires_resolving_candidate_and_binding_commits(self) -> None:
        checkpoint = self.pending_checkpoint()
        checkpoint.update(
            status="PASS",
            candidate_sha="a" * 40,
            binding_sha="b" * 40,
        )
        for name, receipt in checkpoint["receipts"].items():  # type: ignore[union-attr]
            path = self.root / str(receipt["path"])
            path.write_text(f"{name}\n", encoding="utf-8")
            receipt.update(
                status="PASS",
                candidate_sha="a" * 40,
                sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            )
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("candidate_sha does not resolve to a commit", result.stdout)
        self.assertIn("binding_sha does not resolve to a commit", result.stdout)

    def test_stage3_commit_must_descend_from_checkpoint_binding(self) -> None:
        def git(*args: str) -> str:
            result = subprocess.run(
                ["git", "-C", str(self.root), *args],
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            )
            return result.stdout.strip()

        checkpoint = self.pass_checkpoint()
        candidate = checkpoint["candidate_sha"]

        (self.root / "stage3.txt").write_text("stage3\n", encoding="utf-8")
        git("add", "stage3.txt")
        git("commit", "-qm", "stage3")
        stage3 = git("rev-parse", "HEAD")
        checkpoint["stage3_start_sha"] = stage3
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout)

        checkpoint["stage3_start_sha"] = candidate
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("binding commit must be an ancestor", result.stdout)

    def test_unrelated_checkpoint_cannot_mask_missing_slice90_checkpoint(self) -> None:
        state = {"release": "0.8.27", "ladder": [{"slice": 90, "status": "PLANNED"},
                  {"slice": 80, "runtime_checkpoint": self.pending_checkpoint()}]}
        (self.root / "dev/plans/release-state-0.8.27.json").write_text(json.dumps(state))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("slice 90 runtime_checkpoint missing", result.stdout)

    def test_minimal_d27_pass_payload_is_rejected(self) -> None:
        checkpoint = self.pass_checkpoint()
        path = self.root / EVIDENCE_DIR / "d27-candidate-receipt.json"
        receipt = json.loads(path.read_text())
        receipt["aggregate_metrics"] = {"projection_heavy": {}, "foreground_heavy": {}}
        path.write_text(json.dumps(receipt))
        self._rebind_performance(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27 candidate", result.stdout)

    def test_one_word_review_and_verification_are_rejected(self) -> None:
        checkpoint = self.pass_checkpoint()
        for name in ("code_review", "verification"):
            path = self.root / checkpoint["receipts"][name]["path"]
            path.write_text("PASS\n")
            checkpoint["receipts"][name]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        self._commit_rebound(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("code_review receipt lacks candidate-bound review evidence", result.stdout)
        self.assertIn("verification receipt lacks candidate-bound review evidence", result.stdout)

    def test_duplicate_or_placeholder_matrix_rows_are_rejected(self) -> None:
        checkpoint = self.pass_checkpoint(cells=MATRIX_CELLS + ("2/1",))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate matrix cell: 2/1", result.stdout)

        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        content = path.read_text().replace("| PASS | " + checkpoint["candidate_sha"], "| PASS | " + "f" * 40, 1)
        path.write_text(content)
        self._rebind_performance(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("matrix cell candidate mismatch", result.stdout)

    def test_structural_source_change_requires_stage3_start_sha(self) -> None:
        checkpoint = self.pass_checkpoint()
        source = self.root / "src/rust/crates/fathomdb-engine/src"
        source.mkdir(parents=True)
        (source / "open.rs").write_text("// structural move\n")
        subprocess.run(["git", "-C", str(self.root), "add", "src"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "first structural move"], check=True)
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stage3_start_sha missing after engine source change", result.stdout)

    def test_binding_commit_cannot_contain_engine_source_change(self) -> None:
        checkpoint = self.pass_checkpoint()
        source = self.root / "src/rust/crates/fathomdb-engine/src"
        source.mkdir(parents=True)
        (source / "open.rs").write_text("// structural move in binding\n")
        subprocess.run(["git", "-C", str(self.root), "add", "src"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "--amend", "--no-edit", "-q"], check=True)
        checkpoint["binding_sha"] = subprocess.run(
            ["git", "-C", str(self.root), "rev-parse", "HEAD"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("engine source changed between checkpoint candidate and binding", result.stdout)

    def test_performance_requires_named_selectors_and_installed_bindings(self) -> None:
        checkpoint = self.pass_checkpoint()
        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        content = path.read_text()
        content = content.replace("| AC-072 | PASS", "| AC-072 | PENDING")
        content = content.replace("| Node | PASS", "| Node | PENDING")
        path.write_text(content)
        checkpoint["receipts"]["performance"]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        self._commit_rebound(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("missing named selector: AC-072", result.stdout)
        self.assertIn("missing installed binding: Node", result.stdout)

    def test_stage3_marker_must_name_first_engine_source_commit(self) -> None:
        checkpoint = self.pass_checkpoint()
        source = self.root / "src/rust/crates/fathomdb-engine/src"
        source.mkdir(parents=True)
        for name in ("open.rs", "connection_runtime.rs"):
            (source / name).write_text("// structural move\n")
            subprocess.run(["git", "-C", str(self.root), "add", "src"], check=True)
            subprocess.run(["git", "-C", str(self.root), "commit", "-qm", f"move {name}"], check=True)
        checkpoint["stage3_start_sha"] = subprocess.run(
            ["git", "-C", str(self.root), "rev-parse", "HEAD"], check=True, text=True, capture_output=True
        ).stdout.strip()
        self.write_state(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stage3_start_sha must identify first engine source change", result.stdout)

    def test_malformed_d27_metric_group_fails_without_traceback(self) -> None:
        checkpoint = self.pass_checkpoint()
        path = self.root / EVIDENCE_DIR / "d27-candidate-receipt.json"
        receipt = json.loads(path.read_text())
        receipt["per_repetition_metrics"]["projection_heavy"][0]["throughput"] = "PASS"
        path.write_text(json.dumps(receipt))
        self._rebind_performance(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27 strict validation failed", result.stdout)
        self.assertNotIn("Traceback", result.stdout)

    def test_fabricated_d27_hashes_without_raw_bundle_are_rejected(self) -> None:
        self.pass_checkpoint()
        shutil.rmtree(self.root / "evidence")
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27 artifact bundle", result.stdout)

    def test_changed_candidate_raw_with_rehashed_receipt_is_rejected(self) -> None:
        checkpoint = self.pass_checkpoint()
        bundle = self.root / "evidence/candidate"
        raw_path = bundle / "raw-output.jsonl"
        rows = [json.loads(line) for line in raw_path.read_text().splitlines()]
        rows[0]["operations"][0]["completed_ns"] += 100
        raw_path.write_text("".join(json.dumps(row) + "\n" for row in rows))
        receipt = json.loads((bundle / "receipt.json").read_text())
        receipt["raw_output_sha256"] = hashlib.sha256(raw_path.read_bytes()).hexdigest()
        encoded = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
        (bundle / "receipt.json").write_text(encoded)
        (self.root / EVIDENCE_DIR / "d27-candidate-receipt.json").write_text(encoded)
        digest = hashlib.sha256(encoded.encode()).hexdigest()
        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        lines = path.read_text().splitlines()
        for index, line in enumerate(lines):
            if line.startswith(f"| {EVIDENCE_DIR / 'd27-candidate-receipt.json'} |"):
                lines[index] = f"| {EVIDENCE_DIR / 'd27-candidate-receipt.json'} | {digest} | PASS |"
            if line.startswith("| candidate | evidence/candidate |"):
                lines[index] = f"| candidate | evidence/candidate | {digest} |"
        path.write_text("\n".join(lines) + "\n")
        checkpoint["receipts"]["performance"]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        self._commit_rebound(checkpoint)
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("D27 strict validation failed", result.stdout)

    def test_unrelated_checkpoint_cannot_mask_missing_slice90_state_file(self) -> None:
        state = {"release": "9.9.9", "ladder": [{"slice": 90, "runtime_checkpoint": self.pending_checkpoint()}]}
        (self.root / "dev/plans/release-state-9.9.9.json").write_text(json.dumps(state))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("required release-state-0.8.27.json is missing", result.stdout)

    def test_duplicate_slice90_entries_are_rejected(self) -> None:
        checkpoint = self.pending_checkpoint()
        state = {"release": "0.8.27", "ladder": [
            {"slice": 90, "runtime_checkpoint": checkpoint},
            {"slice": 90, "runtime_checkpoint": checkpoint},
        ]}
        (self.root / "dev/plans/release-state-0.8.27.json").write_text(json.dumps(state))
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("exactly one Slice 90 entry", result.stdout)

    def _commit_rebound(self, checkpoint: dict[str, object]) -> None:
        subprocess.run(["git", "-C", str(self.root), "add", "dev/plans/0.8.27/features/slice-90"], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-qm", "rebind receipts"], check=True)
        checkpoint["binding_sha"] = subprocess.run(["git", "-C", str(self.root), "rev-parse", "HEAD"], check=True, capture_output=True, text=True).stdout.strip()
        self.write_state(checkpoint)

    def _rebind_performance(self, checkpoint: dict[str, object]) -> None:
        path = self.root / EVIDENCE_DIR / "runtime-performance-qualification.md"
        d27 = self.root / EVIDENCE_DIR / "d27-candidate-receipt.json"
        bundle_receipt = self.root / "evidence/candidate/receipt.json"
        if bundle_receipt.is_file():
            bundle_receipt.write_bytes(d27.read_bytes())
        digest = hashlib.sha256(d27.read_bytes()).hexdigest()
        lines = path.read_text().splitlines()
        for index, line in enumerate(lines):
            if line.startswith(f"| {EVIDENCE_DIR / 'd27-candidate-receipt.json'} |"):
                lines[index] = f"| {EVIDENCE_DIR / 'd27-candidate-receipt.json'} | {digest} | PASS |"
            if line.startswith("| candidate | evidence/candidate |"):
                lines[index] = f"| candidate | evidence/candidate | {digest} |"
        path.write_text("\n".join(lines) + "\n")
        checkpoint["receipts"]["performance"]["sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        self._commit_rebound(checkpoint)


if __name__ == "__main__":
    unittest.main()
