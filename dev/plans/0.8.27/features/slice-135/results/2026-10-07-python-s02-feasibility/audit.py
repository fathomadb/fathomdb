"""Independently check the two installed-wheel S02 feasibility observations."""

from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import sys
import zipfile


ROOT = Path(__file__).resolve().parent
RUNNER_SHA = "75424eb9b939530e4ac039d0e1e4ac5d112ea0fa084cc406688f68c10fac550d"
HELPER_SHA = "91574e04c212c27c57cad23aa0b8ddb16b091966d2109b38cca4f9d50734c17b"
IDENTITIES = {
    "baseline": (
        "f99e002f0d2e4002f3694c9f8d4986b56089edaa",
        "7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282",
    ),
    "candidate": (
        "b2ac8081e79a6e626d01f5f97331be331f1cc16d",
        "6713ade54d62cc1e067fcaf1982a539c1fb8bd3d45d3db18e41155badd0c12e8",
    ),
}
SOURCE_BODY = '{"summary": "s02 canonical source evidence bytes"}'
EXPECTED_COUNTS = {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_observed(observed: dict) -> None:
    if observed["embedder"] != "fathomdb-bge-small-en-v1.5":
        raise ValueError("embedder changed")
    if observed["unsupported_kinds"] != [] or observed["readiness"] != "ready":
        raise ValueError("projection not ready")
    if observed["anchor_before"] is not True:
        raise ValueError("pre-query anchor missing")
    if observed["text"] != {"ids": ["A"], "branches": ["text"]}:
        raise ValueError("text anchor changed")
    if not observed["vector"]["ids"] or "vector" not in observed["vector"]["branches"]:
        raise ValueError("vector arm missing")
    if (
        "A" not in observed["hybrid"]["ids"]
        or "A" not in observed["hybrid_lexical_ids"]
    ):
        raise ValueError("hybrid anchor/lexical eligibility missing")
    if observed["evidence"] != {"logical_id": "s02-claim", "source_body": SOURCE_BODY}:
        raise ValueError("evidence source changed")
    if observed["graph"] != {
        "target_id": "s02-claim",
        "target_revision": "s02-claim-r1",
        "edge_revision": "s02-edge-r1",
        "edge_from": "s02-root",
        "edge_to": "s02-claim",
        "source_body": SOURCE_BODY,
    }:
        raise ValueError("graph evidence changed")
    if observed["erasure"] != {
        "source_ref": "slice135-s02-graph",
        "nodes_excised": 3,
        "edges_excised": 1,
    } or observed["second_erasure"] != {"nodes_excised": 0, "edges_excised": 0}:
        raise ValueError("erasure report changed")
    for label in ("after_erasure", "after_reopen"):
        state = observed[label]
        if state["canonical_counts"] != EXPECTED_COUNTS:
            raise ValueError(f"{label} canonical state changed")
        for key in (
            "source_absent",
            "root_absent",
            "claim_absent",
            "anchor_retained",
            "evidence_query_empty",
            "graph_empty",
        ):
            if state[key] is not True:
                raise ValueError(f"{label} {key} failed")
    if observed["after_reopen"]["readiness"] != "ready":
        raise ValueError("reopened projection not ready")


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: python audit.py BASELINE_WHEEL CANDIDATE_WHEEL")
    if (
        sha(ROOT / "runner.py") != RUNNER_SHA
        or sha(ROOT / "s01-helper.py") != HELPER_SHA
    ):
        raise ValueError("runner/helper snapshot changed")
    raw = {}
    for label, wheel in zip(
        ("baseline", "candidate"), map(Path, sys.argv[1:]), strict=True
    ):
        source_sha, wheel_sha = IDENTITIES[label]
        data = json.loads((ROOT / f"{label}.json").read_text())
        if (
            data["source_sha"] != source_sha
            or data["artifact"]["wheel_sha256"] != wheel_sha
        ):
            raise ValueError(f"{label} identity changed")
        if sha(wheel) != wheel_sha or data["runner_sha256"] != RUNNER_SHA:
            raise ValueError(f"{label} runner/wheel bytes changed")
        if data["s01_helper_sha256"] != HELPER_SHA or data["semantic_ok"] is not True:
            raise ValueError(f"{label} helper/semantic state changed")
        with zipfile.ZipFile(wheel) as archive:
            native = archive.read("fathomdb/_fathomdb.abi3.so")
        if hashlib.sha256(native).hexdigest() != data["artifact"]["native_sha256"]:
            raise ValueError(f"{label} native module differs from wheel")
        if data["whole_sequence_including_checks_ns"] < sum(data["stage_ns"].values()):
            raise ValueError(f"{label} whole time shorter than stages")
        if any(value <= 0 for value in data["stage_ns"].values()):
            raise ValueError(f"{label} nonpositive stage")
        if (ROOT / f"{label}.stderr").stat().st_size or (
            ROOT / f"{label}.stdout"
        ).stat().st_size:
            raise ValueError(f"{label} unexpected process output")
        check_observed(data["observed"])
        raw[label] = data
    if raw["baseline"]["observed"] != raw["candidate"]["observed"]:
        raise ValueError("versions differ on materialized observations")
    changed = deepcopy(raw["candidate"]["observed"])
    changed["after_reopen"]["canonical_counts"]["graph_edges"] = 1
    try:
        check_observed(changed)
    except ValueError as error:
        negative = str(error)
    else:
        raise AssertionError("retained-edge negative control was accepted")
    output = {
        "schema_version": 1,
        "baseline_raw_sha256": sha(ROOT / "baseline.json"),
        "candidate_raw_sha256": sha(ROOT / "candidate.json"),
        "observed_equal": True,
        "semantic_checks_passed": True,
        "negative_control_rejected": negative,
        "whole_ms_baseline": round(
            raw["baseline"]["whole_sequence_including_checks_ns"] / 1e6, 3
        ),
        "whole_ms_candidate": round(
            raw["candidate"]["whole_sequence_including_checks_ns"] / 1e6, 3
        ),
    }
    (ROOT / "audit.json").write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps(output))


if __name__ == "__main__":
    main()
