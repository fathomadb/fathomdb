"""Independently audit the baseline-only installed-Python S02 noise pilot."""

from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import statistics
import sys
import zipfile


ROOT = Path(__file__).resolve().parent
EXPECTED_SOURCE = "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
EXPECTED_COUNTS = {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32}
EXPECTED_GRAPH = {
    "target_id": "s02-claim",
    "target_revision": "s02-claim-r1",
    "edge_revision": "s02-edge-r1",
    "edge_from": "s02-root",
    "edge_to": "s02-claim",
    "source_body": '{"summary": "s02 canonical source evidence bytes"}',
}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_raw(raw: dict, manifest: dict) -> None:
    """Reject wrong identity, impossible timing or a materialized state defect."""
    if (
        raw["source_sha"] != manifest["source_sha"]
        or raw["source_sha"] != EXPECTED_SOURCE
    ):
        raise ValueError("source identity changed")
    if raw["runner_sha256"] != manifest["runner_sha256"]:
        raise ValueError("runner identity changed")
    if raw["s01_helper_sha256"] != manifest["s01_helper_sha256"]:
        raise ValueError("S01 helper identity changed")
    if raw["s02_helper_sha256"] != manifest["s02_helper_sha256"]:
        raise ValueError("S02 helper identity changed")
    if raw["artifact"]["wheel_sha256"] != manifest["wheel_sha256"]:
        raise ValueError("wheel identity changed")
    if raw["semantic_ok"] is not True:
        raise ValueError("producer rejected semantic state")
    stages = raw["stage_ns"]
    if len(stages) != 18 or any(
        type(value) is not int or value <= 0 for value in stages.values()
    ):
        raise ValueError("stage count or duration invalid")
    if raw["whole_product_ns"] < sum(stages.values()) or raw["verification_ns"] <= 0:
        raise ValueError("product/verification timer invalid")
    if raw["final_canonical_counts"] != EXPECTED_COUNTS:
        raise ValueError("reopened canonical rows changed")
    observed = raw["observed"]
    if observed["embedder"] != "fathomdb-bge-small-en-v1.5":
        raise ValueError("embedder changed")
    if observed["unsupported_kinds"] != [] or observed["readiness"] != "ready":
        raise ValueError("projection not ready")
    if observed["anchor_before"] is not True:
        raise ValueError("anchor missing")
    if observed["text"] != {"ids": ["A"], "branches": ["text"]}:
        raise ValueError("text anchor changed")
    if not observed["vector"]["ids"] or "vector" not in observed["vector"]["branches"]:
        raise ValueError("vector arm missing")
    if (
        "A" not in observed["hybrid"]["ids"]
        or "A" not in observed["hybrid_lexical_ids"]
    ):
        raise ValueError("hybrid anchor missing")
    if observed["evidence"] != {
        "logical_id": "s02-claim",
        "source_body": EXPECTED_GRAPH["source_body"],
    }:
        raise ValueError("source evidence changed")
    if observed["graph"] != EXPECTED_GRAPH:
        raise ValueError("graph evidence changed")
    if observed["erasure"] != {
        "source_ref": "slice135-s02-graph",
        "nodes_excised": 3,
        "edges_excised": 1,
    } or observed["second_erasure"] != {"nodes_excised": 0, "edges_excised": 0}:
        raise ValueError("erasure report changed")
    for label in ("after_erasure", "after_reopen"):
        for key in (
            "source_absent",
            "root_absent",
            "claim_absent",
            "anchor_retained",
            "evidence_query_empty",
            "graph_empty",
        ):
            if observed[label][key] is not True:
                raise ValueError(f"{label} {key} failed")
    if observed["after_reopen"]["readiness"] != "ready":
        raise ValueError("reopened projection not ready")


def main() -> None:
    if len(sys.argv) != 5:
        raise SystemExit("usage: audit.py WHEEL RUNNER S01_HELPER S02_HELPER")
    wheel, runner, s01_helper, s02_helper = map(Path, sys.argv[1:])
    manifest = json.loads((ROOT / "manifest.json").read_text())
    if manifest["source_sha"] != EXPECTED_SOURCE or (
        sha(wheel),
        sha(runner),
        sha(s01_helper),
        sha(s02_helper),
    ) != (
        manifest["wheel_sha256"],
        manifest["runner_sha256"],
        manifest["s01_helper_sha256"],
        manifest["s02_helper_sha256"],
    ):
        raise ValueError("manifest/artifact bytes changed")
    with zipfile.ZipFile(wheel) as archive:
        native_sha = hashlib.sha256(
            archive.read("fathomdb/_fathomdb.abi3.so")
        ).hexdigest()
    block_results = []
    raw_hashes: dict[str, str] = {}
    first_raw = None
    for block in range(1, 6):
        directory = ROOT / f"block-{block:02d}"
        samples = []
        for attempt in range(4):
            label = "warmup" if attempt == 0 else f"sample-{attempt:02d}"
            path = directory / f"{label}.json"
            raw = json.loads(path.read_text())
            check_raw(raw, manifest)
            if raw["artifact"]["native_sha256"] != native_sha:
                raise ValueError("installed native bytes differ from wheel")
            if (directory / f"{label}.stderr").stat().st_size or (
                directory / f"{label}.stdout"
            ).stat().st_size:
                raise ValueError("unexpected process output")
            resource = (directory / f"{label}.resource.txt").read_text()
            if "Exit status: 0" not in resource or "Swaps: 0" not in resource:
                raise ValueError("failed or swapping child process")
            raw_hashes[str(path.relative_to(ROOT))] = sha(path)
            if attempt:
                samples.append(raw["whole_product_ns"] / 1e6)
                if first_raw is None:
                    first_raw = raw
        block_results.append(
            {
                "block": block,
                "valid": len(samples),
                "whole_product_ms": samples,
                "median_ms": statistics.median(samples),
                "min_ms": min(samples),
                "max_ms": max(samples),
            }
        )
    assert first_raw is not None
    negative = deepcopy(first_raw)
    negative["observed"]["after_reopen"]["source_absent"] = False
    try:
        check_raw(negative, manifest)
    except ValueError as error:
        negative_message = str(error)
    else:
        raise AssertionError("surviving-source negative control was accepted")
    if negative_message != "after_reopen source_absent failed":
        raise ValueError("negative control rejected for the wrong reason")
    medians = [item["median_ms"] for item in block_results]
    output = {
        "schema_version": 1,
        "status": "BASELINE_ONLY_DIAGNOSTIC_NO_PAIR",
        "manifest_sha256": sha(ROOT / "manifest.json"),
        "raw_sha256": raw_hashes,
        "native_sha256": native_sha,
        "blocks": block_results,
        "median_spread_ms": max(medians) - min(medians),
        "median_spread_fraction": (max(medians) - min(medians))
        / statistics.median(medians),
        "negative_control_rejected": negative_message,
        "total_valid_samples": sum(item["valid"] for item in block_results),
    }
    (ROOT / "audit.json").write_text(json.dumps(output, indent=2) + "\n")
    print(
        json.dumps(
            {
                "valid": output["total_valid_samples"],
                "median_spread_ms": output["median_spread_ms"],
            }
        )
    )


if __name__ == "__main__":
    main()
