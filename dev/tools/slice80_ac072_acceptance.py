#!/usr/bin/env python3
"""Fail-closed validation for the bounded Slice 80.n AC-072 campaign."""

from __future__ import annotations

import json
import math
import re
from pathlib import Path
from typing import Any

from dev.tools.slice80_read_acceptance import qualify_environment


IDENTITY_PREFIX = "SLICE80_AC072_IDENTITY"
INVOKE_PREFIX = "SLICE80_AC072_INVOKE"
ENV_PREFIX = "SLICE71_ENV"
EXIT_PREFIX = "SLICE80_AC072_TEST_EXIT"
SELECTOR = "ac_013_vector_retrieval_latency"
IDENTITY_FIELDS = {
    "source_sha",
    "binary_sha256",
    "input_sha256",
    "runner_sha256",
    "scanner_sha256",
    "mode",
    "purpose",
    "selector",
    "corpus_n",
    "vector_dim",
    "requested_samples",
    "compiled_samples",
    "treatment",
}
INVOKE_FIELDS = {
    "AGENT_LONG",
    "AC013_CORPUS_N",
    "AC013_VECTOR_DIM",
    "AC013_SAMPLES",
    "AC013_SCALE_TREATMENT",
}
NUMBERS = re.compile(
    r"AC013_NUMBERS n=(?P<n>\d+) samples=(?P<samples>\d+) seed_ms=(?P<seed>\d+) "
    r"p50_ms=(?P<p50>\d+) p99_ms=(?P<p99>\d+)(?=\n|$)"
)
TREATMENT = re.compile(r"^AC013_TREATMENT_RECORD (?P<fields>.+)$", re.MULTILINE)


def _fields(line: str, expected: set[str], label: str) -> dict[str, str]:
    fields = [item.split("=", maxsplit=1) for item in line.split()]
    if any(len(item) != 2 or not item[1] for item in fields):
        raise ValueError(f"{label} contains malformed fields")
    result = dict(fields)
    if len(result) != len(fields) or set(result) != expected:
        raise ValueError(f"{label} is incomplete or contains duplicate fields")
    return result


def _one_line(text: str, prefix: str) -> str:
    matches = re.findall(rf"^{re.escape(prefix)} (.+)$", text, flags=re.MULTILINE)
    if len(matches) != 1:
        raise ValueError(f"expected exactly one {prefix} record")
    return matches[0]


def _purpose_contract(purpose: str) -> int:
    if purpose == "smoke":
        return 10
    if purpose == "acceptance":
        return 10_000
    raise ValueError("unsupported purpose")


def _percentile_ms(samples_us: list[int], numerator: int) -> int:
    rank = math.ceil(len(samples_us) * numerator / 100) - 1
    return sorted(samples_us)[rank] // 1000


def parse_cell_text(text: str, label: str, purpose: str) -> dict[str, Any]:
    expected_n = _purpose_contract(purpose)
    identity = _fields(_one_line(text, IDENTITY_PREFIX), IDENTITY_FIELDS, "identity")
    invoke = _fields(_one_line(text, INVOKE_PREFIX), INVOKE_FIELDS, "invocation")
    required_identity = {
        "mode": "performance",
        "purpose": purpose,
        "selector": SELECTOR,
        "corpus_n": str(expected_n),
        "vector_dim": "384",
        "requested_samples": "1000",
        "compiled_samples": "1000",
        "treatment": "warm",
    }
    if any(identity[field] != value for field, value in required_identity.items()):
        raise ValueError("identity purpose or fixture does not bind the required workload")
    required_invoke = {
        "AGENT_LONG": "1",
        "AC013_CORPUS_N": str(expected_n),
        "AC013_VECTOR_DIM": "384",
        "AC013_SAMPLES": "1000",
        "AC013_SCALE_TREATMENT": "warm",
    }
    if invoke != required_invoke:
        raise ValueError("child invocation does not match the sealed fixture")

    number_matches = list(NUMBERS.finditer(text))
    if len(number_matches) != 1:
        raise ValueError("expected exactly one AC013_NUMBERS record")
    numbers = number_matches[0]
    if int(numbers["n"]) != expected_n or int(numbers["samples"]) != 1000:
        raise ValueError("numbers record has wrong workload counts")
    treatments = list(TREATMENT.finditer(text))
    if len(treatments) != 1:
        raise ValueError("expected exactly one AC013_TREATMENT_RECORD")
    treatment = _fields(
        treatments[0]["fields"],
        {
            "treatment", "n", "seed_write_ms", "embedding_ms", "projection_drain_ms",
            "accepted_writes", "vector_rows_after_drain", "drain_outcome", "samples_us", "result_counts",
        },
        "treatment",
    )
    if (
        treatment["treatment"] != "warm"
        or treatment["n"] != str(expected_n)
        or treatment["accepted_writes"] != str(expected_n)
        or treatment["vector_rows_after_drain"] != str(expected_n)
        or treatment["drain_outcome"] != "ok"
        or treatment["result_counts"] != "not_retained_per_query"
    ):
        raise ValueError("treatment record does not prove the required warm workload")
    try:
        samples = [int(value) for value in treatment["samples_us"].split(",")]
    except ValueError as error:
        raise ValueError("samples_us is malformed") from error
    if len(samples) != 1000 or any(value <= 0 for value in samples):
        raise ValueError("expected exactly 1,000 positive retained samples")
    if _percentile_ms(samples, 50) != int(numbers["p50"]) or _percentile_ms(samples, 99) != int(numbers["p99"]):
        raise ValueError("printed percentiles disagree with retained samples")

    environments = [
        json.loads(item)
        for item in re.findall(rf"^{ENV_PREFIX} (.+)$", text, flags=re.MULTILINE)
    ]
    if len(environments) != 2 or [item.get("phase") for item in environments] != ["start", "end"]:
        raise ValueError("expected one ordered environment pair")
    exits = re.findall(rf"^{EXIT_PREFIX} status=(\d+)$", text, flags=re.MULTILINE)
    passed = bool(re.search(rf"test {SELECTOR} \.\.\. ok", text)) and bool(
        re.search(r"test result: ok\. 1 passed; 0 failed;", text)
    )
    failed = bool(re.search(rf"test {SELECTOR} \.\.\. FAILED", text)) and bool(
        re.search(r"test result: FAILED\. 0 passed; 1 failed;", text)
    )
    qualification = qualify_environment(environments[0], environments[1])
    printed_within_budget = int(numbers["p50"]) <= 80 and int(numbers["p99"]) <= 300
    if exits == ["0"] and passed:
        if not printed_within_budget:
            raise ValueError("successful strict Rust exit disagrees with printed timings")
        numeric_pass = True
    elif exits == ["101"] and failed:
        # The Rust gate compares full Duration values while its marker truncates
        # to milliseconds. A displayed 80/300 can therefore be a true failure.
        numeric_pass = False
    else:
        raise ValueError("strict Rust exit disagrees with the test result")
    return {
        **identity,
        "label": label,
        "corpus_n": expected_n,
        "sample_count": len(samples),
        "p50_ms": int(numbers["p50"]),
        "p99_ms": int(numbers["p99"]),
        "printed_within_budget": printed_within_budget,
        "numeric_pass": numeric_pass,
        "environment_applicable": qualification["applicable"],
        "environment_reasons": qualification["reasons"],
        "environment": {"start": environments[0], "end": environments[1]},
    }


def cell_status(cell: dict[str, Any]) -> str:
    if not cell["environment_applicable"]:
        return "ENVIRONMENT_INVALID"
    return "PASS" if cell["numeric_pass"] else "FAIL"


def summarize_campaign(cells: list[dict[str, Any]]) -> dict[str, Any]:
    if len(cells) != 3 or len({cell["label"] for cell in cells}) != 3:
        raise ValueError("campaign requires exactly three uniquely labelled cells")
    if any(cell_status(cell) != "PASS" for cell in cells):
        raise ValueError("campaign cells are not passing and applicable")
    identities = [
        tuple(cell[field] for field in ("binary_sha256", "input_sha256", "runner_sha256", "scanner_sha256"))
        for cell in cells
    ]
    if len(set(identities)) != 1:
        raise ValueError("campaign identity drift")
    return {"status": "PASS", "observations": cells}


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    cell = commands.add_parser("validate-cell")
    cell.add_argument("--log", type=Path, required=True)
    cell.add_argument("--label", required=True)
    cell.add_argument("--purpose", choices=("smoke", "acceptance"), required=True)
    campaign = commands.add_parser("summarize-campaign")
    campaign.add_argument("--logs", type=Path, nargs=3, required=True)
    args = parser.parse_args()
    if args.command == "validate-cell":
        try:
            parsed = parse_cell_text(args.log.read_text(encoding="utf-8", errors="replace"), args.label, args.purpose)
            parsed["status"] = cell_status(parsed)
            print(json.dumps(parsed, sort_keys=True))
            return 0 if parsed["status"] == "PASS" else 1
        except (OSError, ValueError) as error:
            print(json.dumps({"label": args.label, "status": "INCOMPLETE", "error": str(error)}, sort_keys=True))
            return 1
    try:
        cells = [parse_cell_text(path.read_text(encoding="utf-8", errors="replace"), f"R{index}", "acceptance") for index, path in enumerate(args.logs, 1)]
        print(json.dumps(summarize_campaign(cells), sort_keys=True))
        return 0
    except ValueError as error:
        print(json.dumps({"status": "INCOMPLETE", "error": str(error)}, sort_keys=True))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
