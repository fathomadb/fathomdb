#!/usr/bin/env python3
"""Validate structured, candidate-bound runtime checkpoints in release state."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import statistics
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
MATRIX_CELLS = ("2/1", "1/1", "2/2", "4/4", "64/64", "2/no-provider")
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

    def first_engine_change_after(self, binding_sha: str) -> str | None:
        result = subprocess.run(
            [
                "git",
                "-C",
                str(self.root),
                "rev-list",
                "--reverse",
                f"{binding_sha}..HEAD",
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

    @staticmethod
    def finite_nonnegative(value: object) -> bool:
        if not isinstance(value, (float, int)) or isinstance(value, bool):
            return False
        try:
            return math.isfinite(value) and value >= 0
        except OverflowError:
            return False

    @staticmethod
    def metric_at(run: dict, group: str, name: str, percentile: str | None) -> object:
        values = run.get(group)
        if not isinstance(values, dict):
            return None
        value = values.get(name)
        if percentile is None:
            return value
        return value.get(percentile) if isinstance(value, dict) else None

    def validate_d27_receipt(
        self, location: str, receipt: object, protocol: dict, candidate_sha: str
    ) -> None:
        if not isinstance(receipt, dict):
            self.fail(location, "D27 candidate receipt must be an object")
            return
        required = set(protocol["required_receipt_fields"]) | {"phase", "runner_sha256"}
        if not required <= set(receipt):
            self.fail(
                location,
                f"D27 candidate receipt missing fields: {sorted(required - set(receipt))}",
            )
            return
        if (receipt["phase"], receipt["source_sha"], receipt["status"]) != (
            "candidate",
            candidate_sha,
            "PASS",
        ):
            self.fail(
                location,
                "D27 candidate qualification is not PASS for checkpoint candidate",
            )
        for field in (
            "binary_sha256",
            "runner_sha256",
            "protocol_sha256",
            "corpus_sha256",
            "raw_output_sha256",
        ):
            if (
                not isinstance(receipt[field], str)
                or HASH_RE.fullmatch(receipt[field]) is None
            ):
                self.fail(location, f"D27 candidate {field} must be a SHA-256")
        if (
            not isinstance(receipt["runner_inventory"], dict)
            or not receipt["runner_inventory"]
        ):
            self.fail(location, "D27 candidate runner inventory is empty")
        for boundary in ("environment_start", "environment_end"):
            if not isinstance(receipt[boundary], dict) or not receipt[boundary]:
                self.fail(location, f"D27 candidate {boundary} is empty")
        if receipt["decision_rule_evaluation"] != {
            "status": "PASS",
            "rule": "D27 frozen median/MAD",
        }:
            self.fail(location, "D27 candidate frozen decision rule is not PASS")

        metrics = protocol["metrics"]
        repetitions = receipt["per_repetition_metrics"]
        aggregate = receipt["aggregate_metrics"]
        if not isinstance(repetitions, dict) or set(repetitions) != {
            "projection_heavy",
            "foreground_heavy",
        }:
            self.fail(location, "D27 candidate repetition directions are incomplete")
            return
        if not isinstance(aggregate, dict) or set(aggregate) != set(repetitions):
            self.fail(location, "D27 candidate aggregate directions are incomplete")
            return
        for direction, runs in repetitions.items():
            if (
                not isinstance(runs, list)
                or len(runs) != protocol["execution"]["repetitions"]
                or any(not isinstance(run, dict) for run in runs)
            ):
                self.fail(
                    location, f"D27 candidate {direction} requires three repetitions"
                )
                continue
            for index, run in enumerate(runs, 1):
                label = f"D27 candidate {direction} repetition {index}"
                if (
                    run.get("environment_valid") is not True
                    or run.get("starvation_pass") is not True
                ):
                    self.fail(location, f"{label} environment or starvation failed")
                swaps = (
                    run.get("swap_pages_in_delta"),
                    run.get("swap_pages_out_delta"),
                )
                if (
                    any(
                        not isinstance(value, int)
                        or isinstance(value, bool)
                        or value < 0
                        for value in swaps
                    )
                    or sum(swaps)
                    > protocol["swap_policy"]["max_total_pages_per_repetition"]
                ):
                    self.fail(location, f"{label} swap deltas invalid")
                for field, names in (
                    ("counts", metrics["counts"]),
                    ("high_water", metrics["high_water"]),
                    ("inventory", metrics["inventory"]),
                    ("throughput", metrics["throughput_rates_per_second"]),
                ):
                    values = run.get(field)
                    if not isinstance(values, dict) or any(
                        not self.finite_nonnegative(values.get(name)) for name in names
                    ):
                        self.fail(location, f"{label} {field} is incomplete")
                latency = run.get("latency_ms")
                if not isinstance(latency, dict) or any(
                    not isinstance(latency.get(name), dict)
                    or any(
                        not self.finite_nonnegative(latency[name].get(str(p)))
                        for p in metrics["latency_percentiles"]
                    )
                    for name in metrics["latency_classes"]
                    + metrics["candidate_only_latency_classes"]
                ):
                    self.fail(location, f"{label} latency is incomplete")
            direction_aggregate = aggregate.get(direction)
            if not isinstance(direction_aggregate, dict):
                self.fail(location, f"D27 candidate {direction} aggregate is empty")
                continue
            for group, names in (
                ("throughput", metrics["throughput_rates_per_second"]),
                ("latency_ms", metrics["latency_classes"]),
            ):
                values = direction_aggregate.get(group)
                if not isinstance(values, dict):
                    self.fail(
                        location,
                        f"D27 candidate {direction} aggregate {group} is empty",
                    )
                    continue
                for name in names:
                    percentiles = (
                        (str(p) for p in metrics["latency_percentiles"])
                        if group == "latency_ms"
                        else (None,)
                    )
                    for percentile in percentiles:
                        center = (
                            values.get(name)
                            if percentile is None
                            else values.get(name, {}).get(percentile)
                            if isinstance(values.get(name), dict)
                            else None
                        )
                        observed = [
                            self.metric_at(run, group, name, percentile) for run in runs
                        ]
                        if not isinstance(center, dict) or any(
                            not self.finite_nonnegative(value) for value in observed
                        ):
                            self.fail(
                                location,
                                f"D27 candidate {direction} aggregate {group}.{name} is incomplete",
                            )
                            continue
                        median = statistics.median(observed)
                        mad = statistics.median(
                            abs(value - median) for value in observed
                        )
                        if center != {"median": median, "mad": mad}:
                            self.fail(
                                location,
                                f"D27 candidate {direction} aggregate {group}.{name} differs from repetitions",
                            )

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
                    or MATRIX_RESULT_RE.search(columns[4]) is None
                ):
                    self.fail(location, f"incomplete candidate-bound {label}: {name}")
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
        if matches[0][1] != hashlib.sha256(artifact.read_bytes()).hexdigest():
            self.fail(location, "D27 candidate receipt sha256 mismatch")
            return
        try:
            receipt = json.loads(artifact.read_text(encoding="utf-8"))
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
        self.validate_d27_receipt(
            location,
            receipt,
            json.loads(protocol_path.read_text(encoding="utf-8")),
            candidate_sha,
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
            if state_path.name == "release-state-0.8.27.json" and any(
                isinstance(entry, dict)
                and entry.get("slice") == 90
                and "runtime_checkpoint" not in entry
                for entry in ladder
            ):
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
