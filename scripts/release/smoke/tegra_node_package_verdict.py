#!/usr/bin/env python3
"""Verdict and witness extraction for smoke-tegra-node-package.sh.

`witness` converts the forced-CUDA embed row's in-process allocation witness
(camelCase, as the Node SDK reports it) into the canonical snake_case record
that `scripts/release/verify-tegra-gpu-witness.py` verifies.

`verdict` reads the retained evidence directory and decides PASS or FAIL. It
writes `summary.json` either way. Every required row must be present and
satisfied; a missing, skipped, raised or CPU-fallback row is a failure, never
a pass. Exit status is 0 only for PASS.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from pathlib import Path
from typing import Any

CONTEXT_SCHEMA = "fathomdb.tegra-node-package-smoke-context/v1"
ROW_SCHEMA = "fathomdb.tegra-node-package-row/v1"
SUMMARY_SCHEMA = "fathomdb.tegra-node-package-smoke-summary/v1"
TEGRA_PACKAGE = "fathomdb-linux-arm64-gnu-tegra"
NATIVE_NAME = "fathomdb.linux-arm64-gnu.node"
EMBEDDING_DIM = 384
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
CANDIDATE_SHA = re.compile(r"[0-9a-f]{40}\Z")
HEAP_PASS = re.compile(r"PASS  run (\d+)/(\d+):")
# Policy rows run in the primary consumer; the effective device each must
# reach on a confirmed classic Jetson Orin. `auto` must find the GPU there.
POLICY_ROWS = (
    ("embed", "cpu", "cpu"),
    ("embed", "auto", "cuda"),
    ("embed", "cuda:0", "cuda"),
    ("rerank", "cpu", "cpu"),
    ("rerank", "auto", "cuda"),
    ("rerank", "cuda:0", "cuda"),
)
REQUIRED_CONSUMERS = {
    "local": {"tegra-only", "generic-coinstalled"},
    "pages": {"pages"},
}


def row_name(component: str, policy: str) -> str:
    return f"{component}-{policy.replace(':', '')}"


def snake(name: str) -> str:
    return re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", name).lower()


def canonical_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True).encode("ascii") + b"\n"


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def extract_witness(evidence: Path, out: Path) -> int:
    row_path = evidence / "rows" / f"{row_name('embed', 'cuda:0')}.json"
    try:
        row = load_json(row_path)
    except (OSError, json.JSONDecodeError) as error:
        print(f"tegra-node-smoke: cannot read the forced-CUDA embed row: {error}", file=sys.stderr)
        return 1
    witness = row.get("witness") if isinstance(row, dict) else None
    if not isinstance(witness, dict) or not witness:
        print("tegra-node-smoke: the forced-CUDA embed row carries no GPU allocation witness", file=sys.stderr)
        return 1
    out.write_bytes(canonical_json({snake(key): value for key, value in witness.items()}))
    return 0


class Verdict:
    def __init__(self) -> None:
        self.rows: list[dict[str, str]] = []
        self.failures: list[str] = []

    def record(self, name: str, problems: list[str], detail: str = "") -> None:
        if problems:
            for problem in problems:
                self.failures.append(f"{name}: {problem}")
            self.rows.append({"row": name, "status": "FAIL", "detail": "; ".join(problems)})
        else:
            self.rows.append({"row": name, "status": "PASS", "detail": detail})


def check_context(evidence: Path) -> tuple[dict[str, Any] | None, list[str]]:
    try:
        context = load_json(evidence / "context.json")
    except (OSError, json.JSONDecodeError) as error:
        return None, [f"context.json is unreadable: {error}"]
    problems = []
    if not isinstance(context, dict) or context.get("schema") != CONTEXT_SCHEMA:
        return None, [f"context schema must be {CONTEXT_SCHEMA}"]
    if not CANDIDATE_SHA.fullmatch(str(context.get("candidate_sha", ""))):
        problems.append("candidate SHA must be 40 lowercase hexadecimal characters")
    if context.get("source") not in REQUIRED_CONSUMERS:
        problems.append(f"source must be one of {sorted(REQUIRED_CONSUMERS)}")
    if not SHA256.fullmatch(str(context.get("tegra_node_sha256", ""))):
        problems.append("the packed Tegra addon SHA-256 is not recorded")
    consumers = context.get("consumers")
    if not isinstance(consumers, dict):
        problems.append("consumers are not recorded")
    elif context.get("source") in REQUIRED_CONSUMERS:
        missing = REQUIRED_CONSUMERS[context["source"]] - set(consumers)
        if missing:
            problems.append(f"required consumer(s) not recorded: {', '.join(sorted(missing))}")
    if not isinstance(consumers, dict) or context.get("primary_consumer") not in consumers:
        problems.append("primary consumer is not a recorded consumer")
    return (context if not problems else None), problems


def check_selection(row: dict[str, Any], context: dict[str, Any], consumer: str) -> list[str]:
    problems = []
    root = context["consumers"][consumer]["root"]
    expected = f"{root}/node_modules/{TEGRA_PACKAGE}/{NATIVE_NAME}"
    if row.get("consumer") != consumer or row.get("consumer_root") != root:
        problems.append(f"row ran outside consumer {consumer}")
    loaded = row.get("loaded_native_addons")
    if not isinstance(loaded, list):
        return problems + ["loaded native addons were not recorded"]
    fathomdb_addons = [path for path in loaded if Path(str(path)).name.startswith("fathomdb")]
    if fathomdb_addons != [expected]:
        problems.append(f"loaded {fathomdb_addons}, expected only {expected}")
        return problems
    digests = row.get("loaded_native_sha256")
    actual = digests.get(expected) if isinstance(digests, dict) else None
    if actual != context["tegra_node_sha256"]:
        problems.append(
            f"loaded addon SHA-256 {actual} differs from the packed Tegra addon {context['tegra_node_sha256']}"
        )
    return problems


def check_row(evidence: Path, name: str, context: dict[str, Any], consumer: str) -> tuple[dict[str, Any] | None, list[str]]:
    path = evidence / "rows" / f"{name}.json"
    if not path.is_file():
        return None, ["row is missing"]
    try:
        row = load_json(path)
    except (OSError, json.JSONDecodeError) as error:
        return None, [f"row is unreadable: {error}"]
    if not isinstance(row, dict) or row.get("schema") != ROW_SCHEMA:
        return None, [f"row schema must be {ROW_SCHEMA}"]
    if row.get("outcome") != "completed":
        return row, [f"row did not complete: {row.get('error')}"]
    return row, check_selection(row, context, consumer)


def check_policy(row: dict[str, Any], component: str, policy: str, effective: str) -> list[str]:
    problems = []
    if row.get("component") != component or row.get("policy") != policy:
        problems.append(f"row records {row.get('component')} {row.get('policy')}")
    resolution = row.get("resolution")
    if not isinstance(resolution, dict):
        return problems + ["no device resolution was reported"]
    if resolution.get("requested_policy") != policy:
        problems.append(f"requested policy {resolution.get('requested_policy')}, expected {policy}")
    if resolution.get("cuda_compiled") is not True:
        problems.append("the loaded addon was not built with CUDA")
    if resolution.get("effective_device") != effective:
        problems.append(f"effective device {resolution.get('effective_device')}, expected {effective}")
    events = row.get("embedder_events")
    if not isinstance(events, list):
        problems.append("embedder events were not recorded")
    elif "DefaultEmbedderDownload" in events:
        problems.append("default embedder was downloaded; the offline model cache was not used")
    if component == "embed":
        if row.get("embedding_dim") != EMBEDDING_DIM:
            problems.append(f"embedding has {row.get('embedding_dim')} dimensions, expected {EMBEDDING_DIM}")
        if policy == "cuda:0" and row.get("witness_present") is not True:
            problems.append("no GPU allocation witness was taken under forced CUDA")
        if policy != "cuda:0" and row.get("witness_present") is not False:
            problems.append("an allocation witness was taken without being requested")
    else:
        scores = row.get("ce_scores")
        passages = row.get("passages")
        if (
            not isinstance(scores, list)
            or not isinstance(passages, int)
            or passages < 2
            or len(scores) != passages
            or not all(isinstance(score, (int, float)) and math.isfinite(score) for score in scores)
        ):
            problems.append("reranker produced no cross-encoder score for every passage")
    return problems


def read_rc(path: Path) -> str | None:
    try:
        return path.read_text(encoding="ascii").strip()
    except OSError:
        return None


def check_witness(evidence: Path) -> list[str]:
    rc = read_rc(evidence / "witness-verification.rc")
    if rc is None:
        return ["witness verification was not recorded"]
    if rc != "0":
        return [f"witness verification exited {rc}"]
    return []


def check_heap(evidence: Path) -> tuple[list[str], str]:
    rc = read_rc(evidence / "heap-growth.rc")
    try:
        log = (evidence / "heap-growth.log").read_text(encoding="utf-8")
    except OSError:
        log = None
    if rc is None or log is None:
        return ["heap-growth regression was not recorded"], ""
    problems = []
    skips = [line for line in log.splitlines() if line.startswith("SKIP")]
    if skips:
        problems.append(f"SKIP is not a pass: {skips[0]}")
    fails = [line for line in log.splitlines() if line.startswith("FAIL")]
    if fails:
        problems.append(f"failed: {fails[0]}")
    if rc != "0":
        problems.append(f"exited {rc}")
    passes = [HEAP_PASS.match(line) for line in log.splitlines()]
    passes = [match for match in passes if match]
    if not passes:
        problems.append("no passing run was recorded")
        return problems, ""
    totals = {int(match.group(2)) for match in passes}
    total = max(totals)
    if len(totals) != 1 or len(passes) != total:
        problems.append(f"{len(passes)} of {total} runs passed")
    return problems, f"{len(passes)}/{total} runs"


def run_verdict(evidence: Path) -> int:
    verdict = Verdict()
    context, problems = check_context(evidence)
    verdict.record("context", problems)
    if context is not None:
        for consumer in sorted(context["consumers"]):
            name = f"{consumer}-select"
            _row, problems = check_row(evidence, name, context, consumer)
            verdict.record(name, problems, f"generic={context['consumers'][consumer].get('generic')}")
        primary = context["primary_consumer"]
        for component, policy, effective in POLICY_ROWS:
            name = row_name(component, policy)
            row, problems = check_row(evidence, name, context, primary)
            if row is not None and not problems:
                problems = check_policy(row, component, policy, effective)
            verdict.record(name, problems, f"effective={effective}")
    verdict.record("witness-verification", check_witness(evidence))
    heap_problems, heap_detail = check_heap(evidence)
    verdict.record("heap-growth", heap_problems, heap_detail)

    outcome = "FAIL" if verdict.failures else "PASS"
    summary = {
        "schema": SUMMARY_SCHEMA,
        "outcome": outcome,
        "source": context.get("source") if context else None,
        "candidate_sha": context.get("candidate_sha") if context else None,
        "rows": verdict.rows,
        "failures": verdict.failures,
    }
    (evidence / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    for row in verdict.rows:
        print(f"{row['status']:<5} {row['row']}: {row['detail']}")
    for failure in verdict.failures:
        print(f"tegra-node-smoke: FAIL {failure}", file=sys.stderr)
    print(f"tegra-node-smoke: {outcome}")
    return 0 if outcome == "PASS" else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    witness = commands.add_parser("witness")
    witness.add_argument("--evidence-dir", type=Path, required=True)
    witness.add_argument("--out", type=Path, required=True)
    verdict = commands.add_parser("verdict")
    verdict.add_argument("--evidence-dir", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "witness":
        return extract_witness(args.evidence_dir, args.out)
    return run_verdict(args.evidence_dir)


if __name__ == "__main__":
    raise SystemExit(main())
