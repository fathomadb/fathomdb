#!/usr/bin/env python3
"""Validate structured, candidate-bound runtime checkpoints in release state."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
HASH_RE = re.compile(r"^[0-9a-f]{64}$")
CHECKPOINT_KEYS = {
    "status",
    "candidate_sha",
    "binding_sha",
    "stage3_start_sha",
    "receipts",
}
RECEIPT_KEYS = {"path", "status", "candidate_sha", "sha256"}
RECEIPT_NAMES = {"performance", "code_review", "verification"}
SLICE90_EVIDENCE = Path("dev/plans/0.8.27/features/slice-90")
SLICE90_RECEIPTS = {
    "performance": SLICE90_EVIDENCE / "runtime-performance-qualification.md",
    "code_review": SLICE90_EVIDENCE / "code-review.md",
    "verification": SLICE90_EVIDENCE / "review-verification.md",
}
MATRIX_CELLS = ("2/1", "2/5", "1/1", "2/2", "4/4", "64/64", "2/no-provider")
RELEASE_SELECTORS = (
    "AC-011a",
    "AC-011b",
    "AC-017",
    "AC-018",
    "AC-029",
    "AC-072",
    "AC-073",
    "AC-076",
    "AC-081a",
    "AC-081b",
    "AC-081c",
)
INSTALLED_BINDINGS = ("Python", "Node")
MATRIX_COMMAND_RE = re.compile(r"\b(?:cargo test|pytest|node|npm|python3?)\b")
MATRIX_RESULT_RE = re.compile(r"\b[1-9][0-9]* passed\b")
AC073_RECEIPT = SLICE90_EVIDENCE / "ac073-stress-receipt.json"
AC073_EXECUTION = SLICE90_EVIDENCE / "ac073-execution.json"
AC073_EU7 = SLICE90_EVIDENCE / "ac073-eu7.json"
AC073_LOG = SLICE90_EVIDENCE / "ac073-run.log"
AC073_COMMAND = (
    "env CARGO_TARGET_DIR={bundle} AGENT_LONG=1 EU7_N_VALUES=7667 EU7_QUERIES=100 EU7_BOOTSTRAP=1000 "
    "EU7_LATENCY_SAMPLES=1000 EU7_STRESS_PER_THREAD=250 "
    "FATHOMDB_EU7_OUTPUT={bundle}/eu7.json cargo test --release "
    "-p fathomdb-engine --features operator,embed-cuda --test eu7_real_corpus_ac "
    "eu7_real_corpus_ac_validation -- --exact --ignored --nocapture --test-threads=1"
)


class Validation:
    def __init__(self, root: Path) -> None:
        self.root = root.resolve()
        self.errors: list[str] = []
        self.checked = 0

    def fail(self, location: str, message: str) -> None:
        self.errors.append(f"FAIL check-runtime-checkpoints: {location}: {message}")

    def commit_exists(self, sha: str) -> bool:
        result = subprocess.run(
            ["git", "-C", str(self.root), "cat-file", "-e", f"{sha}^{{commit}}"],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return result.returncode == 0

    def is_ancestor(self, older: str, newer: str) -> bool:
        result = subprocess.run(
            ["git", "-C", str(self.root), "merge-base", "--is-ancestor", older, newer],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return result.returncode == 0

    def safe_path(self, location: str, raw: object) -> Path | None:
        if not isinstance(raw, str) or not raw:
            self.fail(
                location, "receipt path must be a non-empty repository-relative string"
            )
            return None
        relative = Path(raw)
        if relative.is_absolute() or ".." in relative.parts:
            self.fail(location, f"receipt path escapes the repository: {raw!r}")
            return None
        resolved = (self.root / relative).resolve()
        try:
            resolved.relative_to(self.root)
        except ValueError:
            self.fail(location, f"receipt path escapes the repository: {raw!r}")
            return None
        return resolved

    def committed_bytes(self, sha: str, relative: str) -> bytes | None:
        result = subprocess.run(
            ["git", "-C", str(self.root), "show", f"{sha}:{relative}"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        return result.stdout if result.returncode == 0 else None

    def first_engine_change_after(
        self, older_sha: str, newer_sha: str = "HEAD"
    ) -> str | None:
        result = subprocess.run(
            [
                "git",
                "-C",
                str(self.root),
                "rev-list",
                "--reverse",
                f"{older_sha}..{newer_sha}",
                "--",
                "src/rust/crates/fathomdb-engine/src",
            ],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        return (
            result.stdout.splitlines()[0]
            if result.returncode == 0 and result.stdout.strip()
            else None
        )

    def d27_bundle(
        self, location: str, raw_dir: str, expected_digest: str
    ) -> tuple[dict, dict[str, Path]] | None:
        directory = Path(raw_dir)
        if ".." in directory.parts:
            self.fail(location, "D27 artifact bundle path cannot contain '..'")
            return None
        if not directory.is_absolute():
            directory = self.root / directory
        if not directory.is_dir():
            self.fail(
                location, f"D27 artifact bundle directory does not exist: {raw_dir}"
            )
            return None
        receipt_path = directory / "receipt.json"
        try:
            receipt_bytes = receipt_path.read_bytes()
            receipt = json.loads(receipt_bytes)
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            self.fail(location, f"D27 artifact bundle receipt is unreadable: {error}")
            return None
        if (
            not isinstance(receipt, dict)
            or hashlib.sha256(receipt_bytes).hexdigest() != expected_digest
        ):
            self.fail(location, "D27 artifact bundle receipt SHA-256 mismatch")
            return None
        binary_matches = [
            path
            for path in (directory / "target/release/deps").glob(
                "d27_runtime_workload-*"
            )
            if path.is_file()
            and hashlib.sha256(path.read_bytes()).hexdigest()
            == receipt.get("binary_sha256")
        ]
        if len(binary_matches) != 1:
            self.fail(
                location, "D27 artifact bundle needs one hash-matched test binary"
            )
            return None
        artifacts = {
            "runner": directory / "runner.bundle",
            "binary": binary_matches[0],
            "corpus": directory / "corpus.jsonl",
            "raw": directory / "raw-output.jsonl",
        }
        missing = [name for name, path in artifacts.items() if not path.is_file()]
        if missing:
            self.fail(location, f"D27 artifact bundle missing {missing}")
            return None
        return receipt, artifacts

    def validate_d27_bundles(
        self,
        location: str,
        sections: dict[str, list[str]],
        candidate_receipt: dict,
        candidate_receipt_bytes: bytes,
        candidate_sha: str,
        protocol_path: Path,
    ) -> None:
        rows = [
            [part.strip() for part in line.strip("|").split("|")]
            for line in sections.get("D27 artifact bundles", [])
            if line.startswith("|")
        ]
        bundles = {}
        for phase in ("entry", "candidate"):
            matches = [row for row in rows if len(row) == 3 and row[0] == phase]
            if len(matches) != 1 or HASH_RE.fullmatch(matches[0][2]) is None:
                self.fail(
                    location,
                    f"D27 artifact bundle requires one {phase} row with receipt SHA-256",
                )
                return
            bundle = self.d27_bundle(location, matches[0][1], matches[0][2])
            if bundle is None:
                return
            bundles[phase] = bundle
        entry_receipt, entry_artifacts = bundles["entry"]
        bundle_candidate, candidate_artifacts = bundles["candidate"]
        if (
            bundle_candidate != candidate_receipt
            or (
                Path(candidate_artifacts["runner"]).parent / "receipt.json"
            ).read_bytes()
            != candidate_receipt_bytes
        ):
            self.fail(location, "D27 candidate receipt differs from artifact bundle")
            return
        if candidate_receipt.get("source_sha") != candidate_sha:
            self.fail(
                location,
                "D27 candidate receipt source differs from checkpoint candidate",
            )
            return
        spec = importlib.util.spec_from_file_location(
            "d27_checkpoint_verifier",
            Path(__file__).resolve().parent / "d27-runtime-qualification.py",
        )
        if spec is None or spec.loader is None:
            self.fail(location, "D27 strict verifier cannot be loaded")
            return
        verifier = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(verifier)
        try:
            protocol = json.loads(protocol_path.read_text(encoding="utf-8"))
            entry = verifier.validate_entry_for_candidate(
                entry_receipt, protocol, protocol_path, entry_artifacts
            )
            verifier.verify_raw_linkage(
                candidate_receipt, protocol, candidate_artifacts["raw"]
            )
            validated = verifier.validate_receipt(
                candidate_receipt, protocol, protocol_path, candidate_artifacts, entry
            )
            if validated != candidate_receipt:
                raise ValueError(
                    "candidate aggregate or decision rule differs from raw recomputation"
                )
        except (
            ValueError,
            KeyError,
            TypeError,
            IndexError,
            AttributeError,
            OSError,
            json.JSONDecodeError,
        ) as error:
            self.fail(location, f"D27 candidate: D27 strict validation failed: {error}")

    def validate_text_receipt(
        self, location: str, name: str, path: Path, candidate_sha: str
    ) -> None:
        try:
            content = path.read_text(encoding="utf-8")
        except UnicodeError as error:
            self.fail(location, f"{name} receipt is not UTF-8: {error}")
            return
        fields = {}
        for line in content.splitlines():
            match = re.fullmatch(
                r"(Candidate SHA|Verdict|Reviewer|Evidence):\s*(.+)", line.strip()
            )
            if match:
                fields[match.group(1)] = match.group(2).strip()
        reviewer = fields.get("Reviewer", "")
        expected_reviewer = (
            r"gpt-6-sol.*high" if name == "code_review" else r".*terra.*"
        )
        evidence = fields.get("Evidence", "")
        if (
            fields.get("Candidate SHA") != candidate_sha
            or fields.get("Verdict") != "PASS"
            or re.fullmatch(expected_reviewer, reviewer, flags=re.IGNORECASE) is None
            or len(evidence) < 60
            or not re.search(
                r"\b(?:passed|review|test|finding)\b", evidence, re.IGNORECASE
            )
        ):
            self.fail(location, f"{name} receipt lacks candidate-bound review evidence")

    def validate_ac073_stress(self, location: str, evidence: str, candidate_sha: str) -> bool:
        marker = re.fullmatch(
            r"stress receipt=(\S+) sha256=([0-9a-f]{64}) "
            r"combined-exit=101 AC-075=superseded-by-tc5",
            evidence,
        )
        if marker is None or marker.group(1) != AC073_RECEIPT.as_posix():
            self.fail(location, "AC-073 stress receipt marker invalid")
            return False
        receipt_path = self.safe_path(location, marker.group(1))
        if receipt_path is None:
            return False
        try:
            receipt_bytes = receipt_path.read_bytes()
            receipt = json.loads(receipt_bytes)
            if hashlib.sha256(receipt_bytes).hexdigest() != marker.group(2):
                raise ValueError("receipt SHA-256 mismatch")
            expected_keys = {
                "schema_version", "candidate_sha", "selector_exit", "ac073_stress",
                "ac075", "stress_p99_ms", "stress_bound_ms", "source_receipt",
                "source_receipt_sha256", "raw_log", "raw_log_sha256",
                "execution_manifest", "execution_manifest_sha256", "retained_ac075_result",
            }
            if not isinstance(receipt, dict) or set(receipt) != expected_keys:
                raise ValueError("receipt shape mismatch")
            if (
                receipt["schema_version"] != "fathomdb.slice90-ac073-stress/v1"
                or receipt["candidate_sha"] != candidate_sha
                or type(receipt["selector_exit"]) is not int
                or receipt["selector_exit"] != 101
                or receipt["ac073_stress"] != "pass"
                or receipt["ac075"] != "superseded-by-tc5"
            ):
                raise ValueError("candidate or combined selector outcome mismatch")
            if (
                receipt["source_receipt"] != AC073_EU7.as_posix()
                or receipt["raw_log"] != AC073_LOG.as_posix()
            ):
                raise ValueError("raw evidence path mismatch")
            eu7_path = self.safe_path(location, receipt["source_receipt"])
            log_path = self.safe_path(location, receipt["raw_log"])
            if eu7_path is None or log_path is None:
                return False
            eu7_bytes = eu7_path.read_bytes()
            log_bytes = log_path.read_bytes()
            if (
                hashlib.sha256(eu7_bytes).hexdigest() != receipt["source_receipt_sha256"]
                or hashlib.sha256(log_bytes).hexdigest() != receipt["raw_log_sha256"]
            ):
                raise ValueError("raw artifact SHA-256 mismatch")
            eu7 = json.loads(eu7_bytes)
            log = log_bytes.decode("utf-8")
            config = eu7["config"]
            stress = eu7["ac_019_real_dev_box"]
            recall = eu7["ac_013b_real_dev_box"]
            if (
                config["n_values_requested"] != [7667]
                or type(config["real_corpus_docs"]) is not int
                or config["real_corpus_docs"] < 7667
                or not isinstance(stress, list) or len(stress) != 1
                or not isinstance(recall, list) or len(recall) != 1
            ):
                raise ValueError("EU7 corpus or result shape mismatch")
            stress = stress[0]
            recall = recall[0]
            p99 = receipt["stress_p99_ms"]
            bound = receipt["stress_bound_ms"]
            if (
                type(p99) is not int or type(bound) is not int
                or p99 < 0 or bound <= 0 or p99 > bound
                or stress.get("n") != 7667
                or stress.get("padded_with_synthetic_distractors") is not False
                or stress.get("passed") is not True
                or stress.get("p99_ms") != p99
                or stress.get("bound_ms") != bound
            ):
                raise ValueError("AC-073 stress metric or bound mismatch")
            retained = receipt["retained_ac075_result"]
            if (
                not isinstance(retained, dict)
                or set(retained) != {"verdict", "recall_at_10", "ci_95"}
                or retained["verdict"] != "fail"
                or retained["recall_at_10"] != recall.get("recall_at_10")
                or retained["ci_95"] != [recall.get("ci_lo"), recall.get("ci_hi")]
                or recall.get("passes_ci_gate_0_8_0_one_sided") is not False
                or recall.get("ci_hi", 1) >= recall.get("current_floor_0_90", 0)
            ):
                raise ValueError("AC-075 superseded failure mismatch")
            numbers = re.findall(r"^EU7_NUMBERS .*", log, flags=re.MULTILINE)
            if (
                len(numbers) != 1
                or re.search(r"\bn=7667\b", numbers[0]) is None
                or re.search(r"\bpadded=false\b", numbers[0]) is None
                or re.search(rf"\bstress_p99_ms={p99}\b", numbers[0]) is None
                or re.search(rf"\bstress_bound_ms={bound}\b", numbers[0]) is None
                or re.search(r"\bac019=true\b", numbers[0]) is None
                or "EU7_WROTE " not in log
                or "AC-075 recall verdict" not in log
                or "test result: FAILED. 0 passed; 1 failed" not in log
                or "SKIP" in log
            ):
                raise ValueError("EU7 raw log does not retain AC-073 PASS and AC-075 failure")
            if not self.validate_ac073_execution(location, receipt, candidate_sha, log):
                return False
        except (OSError, UnicodeError, ValueError, KeyError, TypeError, IndexError) as error:
            self.fail(location, f"AC-073 stress receipt invalid: {error}")
            return False
        return True

    def validate_ac073_execution(
        self, location: str, receipt: dict, candidate_sha: str, log: str
    ) -> bool:
        if receipt["execution_manifest"] != AC073_EXECUTION.as_posix():
            self.fail(location, "AC-073 execution manifest path mismatch")
            return False
        manifest_path = self.safe_path(location, receipt["execution_manifest"])
        if manifest_path is None:
            return False
        try:
            manifest_bytes = manifest_path.read_bytes()
            if hashlib.sha256(manifest_bytes).hexdigest() != receipt["execution_manifest_sha256"]:
                raise ValueError("manifest SHA-256 mismatch")
            manifest = json.loads(manifest_bytes)
            expected_keys = {
                "schema_version", "candidate_sha", "pre_source_sha", "post_source_sha",
                "pre_clean", "post_clean", "command", "selector_exit", "bundle_dir",
                "binary_relative_path", "executed_binary_path", "binary_sha256",
                "source_receipt_sha256", "raw_log_sha256",
            }
            if not isinstance(manifest, dict) or set(manifest) != expected_keys:
                raise ValueError("manifest shape mismatch")
            if (
                manifest["schema_version"] != "fathomdb.slice90-ac073-execution/v1"
                or any(manifest[key] != candidate_sha for key in
                       ("candidate_sha", "pre_source_sha", "post_source_sha"))
                or manifest["pre_clean"] is not True
                or manifest["post_clean"] is not True
                or type(manifest["selector_exit"]) is not int
                or manifest["selector_exit"] != receipt["selector_exit"]
                or manifest["source_receipt_sha256"] != receipt["source_receipt_sha256"]
                or manifest["raw_log_sha256"] != receipt["raw_log_sha256"]
            ):
                raise ValueError("candidate, command, or raw artifact binding mismatch")
            raw_bundle = manifest["bundle_dir"]
            if not isinstance(raw_bundle, str) or not raw_bundle:
                raise ValueError("bundle directory missing")
            bundle = Path(raw_bundle)
            if not bundle.is_absolute() or ".." in bundle.parts or bundle.resolve() != bundle:
                raise ValueError("bundle directory must be canonical and absolute")
            if manifest["command"] != AC073_COMMAND.format(bundle=bundle):
                raise ValueError("candidate command binding mismatch")
            relative = manifest["binary_relative_path"]
            if (
                not isinstance(relative, str)
                or re.fullmatch(r"target/release/deps/eu7_real_corpus_ac-[A-Za-z0-9]+", relative)
                is None
            ):
                raise ValueError("test executable path mismatch")
            binary = bundle / relative
            if not binary.is_file() or binary.is_symlink():
                raise ValueError("sealed test executable missing")
            digest = manifest["binary_sha256"]
            if not isinstance(digest, str) or HASH_RE.fullmatch(digest) is None:
                raise ValueError("test executable SHA-256 invalid")
            if hashlib.sha256(binary.read_bytes()).hexdigest() != digest:
                raise ValueError("sealed test executable SHA-256 mismatch")
            executed = manifest["executed_binary_path"]
            if (
                not isinstance(executed, str)
                or executed != str(binary)
            ):
                raise ValueError("executed test executable path differs from sealed binary")
            if log.count(f"Running tests/eu7_real_corpus_ac.rs ({executed})") != 1:
                raise ValueError("raw log does not identify sealed test executable")
        except (OSError, UnicodeError, ValueError, KeyError, TypeError) as error:
            self.fail(location, f"AC-073 execution invalid: {error}")
            return False
        return True

    def validate_performance(
        self, location: str, path: Path, candidate_sha: str
    ) -> None:
        try:
            content = path.read_text(encoding="utf-8")
        except UnicodeError as error:
            self.fail(location, f"performance receipt is not UTF-8: {error}")
            return
        sections: dict[str, list[str]] = {}
        heading = ""
        for line in content.splitlines():
            if line.startswith("## "):
                heading = line[3:].strip()
                sections[heading] = []
            elif heading:
                sections[heading].append(line)

        matrix = sections.get("Configuration matrix", [])
        passed_cells: set[str] = set()
        for line in matrix:
            if not line.startswith("|"):
                continue
            columns = [part.strip() for part in line.strip("|").split("|")]
            if not columns or columns[0] not in MATRIX_CELLS:
                continue
            cell = columns[0]
            if cell in passed_cells:
                self.fail(location, f"duplicate matrix cell: {cell}")
            if len(columns) != 5 or columns[1] != "PASS":
                self.fail(
                    location, f"matrix cell requires complete PASS evidence: {cell}"
                )
                continue
            if columns[2] != candidate_sha:
                self.fail(location, f"matrix cell candidate mismatch: {cell}")
            if (
                MATRIX_COMMAND_RE.search(columns[3]) is None
                or MATRIX_RESULT_RE.search(columns[4]) is None
            ):
                self.fail(location, f"matrix cell has placeholder evidence: {cell}")
            passed_cells.add(cell)
        for cell in MATRIX_CELLS:
            if cell not in passed_cells:
                self.fail(location, f"missing matrix cell: {cell}")

        for section, names, label in (
            ("Named release selectors", RELEASE_SELECTORS, "named selector"),
            ("Installed bindings", INSTALLED_BINDINGS, "installed binding"),
        ):
            seen: set[str] = set()
            for line in sections.get(section, []):
                if not line.startswith("|"):
                    continue
                columns = [part.strip() for part in line.strip("|").split("|")]
                if not columns or columns[0] not in names:
                    continue
                name = columns[0]
                if name in seen:
                    self.fail(location, f"duplicate {label}: {name}")
                if (
                    len(columns) != 5
                    or columns[1] != "PASS"
                    or columns[2] != candidate_sha
                    or MATRIX_COMMAND_RE.search(columns[3]) is None
                    or (
                        not (name == "AC-073" and columns[4].startswith("stress receipt="))
                        and MATRIX_RESULT_RE.search(columns[4]) is None
                    )
                ):
                    self.fail(location, f"incomplete candidate-bound {label}: {name}")
                    continue
                if name == "AC-073" and columns[4].startswith("stress receipt="):
                    if not self.validate_ac073_stress(location, columns[4], candidate_sha):
                        continue
                seen.add(name)
            for name in names:
                if name not in seen:
                    self.fail(location, f"missing {label}: {name}")

        d27 = sections.get("D27 candidate qualification")
        if d27 is None:
            self.fail(location, "missing D27 candidate qualification")
            return
        expected = SLICE90_EVIDENCE / "d27-candidate-receipt.json"
        rows = [
            [part.strip() for part in line.strip("|").split("|")]
            for line in d27
            if line.startswith("|")
        ]
        matches = [
            row
            for row in rows
            if len(row) >= 3 and row[0] == expected.as_posix() and row[2] == "PASS"
        ]
        if len(matches) != 1:
            self.fail(
                location, "D27 candidate qualification requires one PASS receipt row"
            )
            return
        artifact = self.root / expected
        if not artifact.is_file():
            self.fail(location, "D27 candidate receipt file does not exist")
            return
        candidate_receipt_bytes = artifact.read_bytes()
        if matches[0][1] != hashlib.sha256(candidate_receipt_bytes).hexdigest():
            self.fail(location, "D27 candidate receipt sha256 mismatch")
            return
        try:
            receipt = json.loads(candidate_receipt_bytes)
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            self.fail(location, f"cannot parse D27 candidate receipt: {error}")
            return
        protocol_path = (
            self.root / SLICE90_EVIDENCE / "d27-runtime-qualification-protocol.json"
        )
        if not protocol_path.is_file():
            protocol_path = (
                Path(__file__).resolve().parents[1]
                / SLICE90_EVIDENCE
                / "d27-runtime-qualification-protocol.json"
            )
        expected_protocol = hashlib.sha256(protocol_path.read_bytes()).hexdigest()
        if (
            not isinstance(receipt, dict)
            or receipt.get("protocol_sha256") != expected_protocol
        ):
            self.fail(
                location,
                "D27 candidate qualification is not PASS for checkpoint candidate and protocol",
            )
            return
        self.validate_d27_bundles(
            location,
            sections,
            receipt,
            candidate_receipt_bytes,
            candidate_sha,
            protocol_path,
        )

    def validate_receipt(
        self,
        location: str,
        name: str,
        receipt: object,
        checkpoint_status: str,
        candidate_sha: object,
    ) -> None:
        if not isinstance(receipt, dict):
            self.fail(location, f"{name} receipt must be an object")
            return
        unknown = set(receipt) ^ RECEIPT_KEYS
        if unknown:
            self.fail(
                location,
                f"{name} receipt keys must be exactly {sorted(RECEIPT_KEYS)}; delta={sorted(unknown)}",
            )
            return

        receipt_location = f"{location}.{name}"
        path = self.safe_path(receipt_location, receipt["path"])
        is_slice90 = location.startswith(
            "dev/plans/release-state-0.8.27.json slice 90 runtime_checkpoint"
        )
        if is_slice90 and receipt["path"] != SLICE90_RECEIPTS[name].as_posix():
            self.fail(
                receipt_location, "receipt path outside declared Slice 90 evidence set"
            )
        if checkpoint_status == "PENDING":
            if receipt["status"] != "PENDING":
                self.fail(
                    receipt_location,
                    "pending checkpoint requires receipt status PENDING",
                )
            for field in ("candidate_sha", "sha256"):
                if receipt[field] is not None:
                    self.fail(
                        receipt_location, f"pending receipt requires {field}=null"
                    )
            return

        if receipt["status"] != "PASS":
            self.fail(receipt_location, "PASS checkpoint requires receipt status PASS")
        if receipt["candidate_sha"] != candidate_sha:
            self.fail(
                receipt_location,
                f"{name} receipt candidate_sha {receipt['candidate_sha']!r} does not equal checkpoint candidate {candidate_sha!r}",
            )
        digest = receipt["sha256"]
        if not isinstance(digest, str) or HASH_RE.fullmatch(digest) is None:
            self.fail(
                receipt_location,
                "PASS receipt sha256 must be 64 lowercase hex characters",
            )
        if path is None:
            return
        if not path.is_file():
            self.fail(
                receipt_location, f"receipt file does not exist: {receipt['path']}"
            )
            return
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != actual:
            self.fail(
                receipt_location,
                f"{name} receipt sha256 mismatch: state={digest!r} actual={actual!r}",
            )
        if name == "performance" and isinstance(candidate_sha, str) and is_slice90:
            self.validate_performance(receipt_location, path, candidate_sha)
        elif (
            name in {"code_review", "verification"}
            and isinstance(candidate_sha, str)
            and is_slice90
        ):
            self.validate_text_receipt(receipt_location, name, path, candidate_sha)

    def validate_checkpoint(self, state_path: Path, entry: dict[str, Any]) -> None:
        checkpoint = entry["runtime_checkpoint"]
        location = f"{state_path.relative_to(self.root)} slice {entry.get('slice', '?')} runtime_checkpoint"
        self.checked += 1
        if not isinstance(checkpoint, dict):
            self.fail(location, "must be an object")
            return
        delta = set(checkpoint) ^ CHECKPOINT_KEYS
        if delta:
            self.fail(
                location,
                f"keys must be exactly {sorted(CHECKPOINT_KEYS)}; delta={sorted(delta)}",
            )
            return

        status = checkpoint["status"]
        if status not in {"PENDING", "PASS"}:
            self.fail(location, "status must be PENDING or PASS")
            return
        candidate_sha = checkpoint["candidate_sha"]
        binding_sha = checkpoint["binding_sha"]
        stage3_sha = checkpoint["stage3_start_sha"]
        if status == "PENDING":
            for field, value in (
                ("candidate_sha", candidate_sha),
                ("binding_sha", binding_sha),
            ):
                if value is not None:
                    self.fail(location, f"PENDING checkpoint requires {field}=null")
            if stage3_sha is not None:
                self.fail(location, "stage3_start_sha requires a PASS checkpoint")
        else:
            for field, value in (
                ("candidate_sha", candidate_sha),
                ("binding_sha", binding_sha),
            ):
                if not isinstance(value, str) or SHA_RE.fullmatch(value) is None:
                    self.fail(
                        location,
                        f"PASS checkpoint requires {field} as 40 lowercase hex characters",
                    )

        receipts = checkpoint["receipts"]
        if not isinstance(receipts, dict):
            self.fail(location, "receipts must be an object")
        else:
            delta = set(receipts) ^ RECEIPT_NAMES
            if delta:
                self.fail(
                    location,
                    f"receipt names must be exactly {sorted(RECEIPT_NAMES)}; delta={sorted(delta)}",
                )
            for name in sorted(RECEIPT_NAMES & set(receipts)):
                self.validate_receipt(
                    location, name, receipts[name], status, candidate_sha
                )

        checkpoint_commits_exist = False
        if (
            status == "PASS"
            and isinstance(candidate_sha, str)
            and isinstance(binding_sha, str)
        ):
            checkpoint_commits_exist = True
            for field, sha in (
                ("candidate_sha", candidate_sha),
                ("binding_sha", binding_sha),
            ):
                if not self.commit_exists(sha):
                    checkpoint_commits_exist = False
                    self.fail(location, f"{field} does not resolve to a commit: {sha}")
            if checkpoint_commits_exist and not self.is_ancestor(
                candidate_sha, binding_sha
            ):
                self.fail(
                    location,
                    "checkpoint candidate must be an ancestor of its binding commit",
                )
            if (
                checkpoint_commits_exist
                and self.is_ancestor(candidate_sha, binding_sha)
                and self.first_engine_change_after(candidate_sha, binding_sha)
                is not None
            ):
                self.fail(
                    location,
                    "engine source changed between checkpoint candidate and binding",
                )
            if checkpoint_commits_exist and not self.is_ancestor(binding_sha, "HEAD"):
                self.fail(
                    location, "checkpoint binding commit must be an ancestor of HEAD"
                )
            if checkpoint_commits_exist and isinstance(receipts, dict):
                for name in sorted(RECEIPT_NAMES & set(receipts)):
                    receipt = receipts[name]
                    if not isinstance(receipt, dict) or not isinstance(
                        receipt.get("path"), str
                    ):
                        continue
                    path = self.safe_path(f"{location}.{name}", receipt["path"])
                    if path is None or not path.is_file():
                        continue
                    bound = self.committed_bytes(binding_sha, receipt["path"])
                    if bound != path.read_bytes():
                        self.fail(
                            location,
                            f"{name} receipt differs from checkpoint binding commit",
                        )
                if location.startswith(
                    "dev/plans/release-state-0.8.27.json slice 90 runtime_checkpoint"
                ):
                    d27_relative = (
                        SLICE90_EVIDENCE / "d27-candidate-receipt.json"
                    ).as_posix()
                    d27_path = self.root / d27_relative
                    if (
                        d27_path.is_file()
                        and self.committed_bytes(binding_sha, d27_relative)
                        != d27_path.read_bytes()
                    ):
                        self.fail(
                            location,
                            "D27 candidate receipt differs from checkpoint binding commit",
                        )

        first_engine_change = (
            self.first_engine_change_after(binding_sha)
            if checkpoint_commits_exist and self.is_ancestor(binding_sha, "HEAD")
            else None
        )
        if stage3_sha is None:
            if first_engine_change is not None:
                self.fail(
                    location, "stage3_start_sha missing after engine source change"
                )
            return
        if not isinstance(stage3_sha, str) or SHA_RE.fullmatch(stage3_sha) is None:
            self.fail(
                location, "stage3_start_sha must be null or 40 lowercase hex characters"
            )
            return
        if (
            status != "PASS"
            or not isinstance(candidate_sha, str)
            or not isinstance(binding_sha, str)
        ):
            return
        stage3_exists = self.commit_exists(stage3_sha)
        if not stage3_exists:
            self.fail(
                location, f"stage3_start_sha does not resolve to a commit: {stage3_sha}"
            )
        if (
            checkpoint_commits_exist
            and stage3_exists
            and not self.is_ancestor(binding_sha, stage3_sha)
        ):
            self.fail(
                location,
                "checkpoint binding commit must be an ancestor of stage3_start_sha",
            )
        if first_engine_change is not None and stage3_sha != first_engine_change:
            self.fail(
                location, "stage3_start_sha must identify first engine source change"
            )

    def run(self) -> int:
        state_paths = sorted((self.root / "dev" / "plans").glob("release-state-*.json"))
        if not state_paths:
            self.fail("dev/plans", "no release-state files found")
        required_state = self.root / "dev/plans/release-state-0.8.27.json"
        if not required_state.is_file():
            self.fail("dev/plans", "required release-state-0.8.27.json is missing")
        for state_path in state_paths:
            try:
                state = json.loads(state_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as error:
                self.fail(
                    str(state_path.relative_to(self.root)),
                    f"cannot parse JSON: {error}",
                )
                continue
            ladder = state.get("ladder")
            if not isinstance(ladder, list):
                self.fail(
                    str(state_path.relative_to(self.root)), "ladder must be an array"
                )
                continue
            for entry in ladder:
                if isinstance(entry, dict) and "runtime_checkpoint" in entry:
                    self.validate_checkpoint(state_path, entry)
            if state_path == required_state:
                slice90 = [
                    entry
                    for entry in ladder
                    if isinstance(entry, dict) and entry.get("slice") == 90
                ]
                if len(slice90) != 1:
                    self.fail(
                        str(state_path.relative_to(self.root)),
                        "exactly one Slice 90 entry is required",
                    )
                elif "runtime_checkpoint" not in slice90[0]:
                    self.fail(
                        str(state_path.relative_to(self.root)),
                        "slice 90 runtime_checkpoint missing",
                    )
        if self.checked == 0:
            self.fail("dev/plans", "zero structured checkpoints discovered")
        if self.errors:
            print("\n".join(self.errors), file=sys.stderr)
            return 1
        print(
            f"ok    check-runtime-checkpoints: {self.checked} structured checkpoint(s) valid"
        )
        return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    return Validation(args.root).run()


if __name__ == "__main__":
    raise SystemExit(main())
