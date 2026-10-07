"""Independently check installed TypeScript S02 package and state receipts."""

from __future__ import annotations

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import tarfile


ROOT = Path(__file__).resolve().parent
RUNNER_SHA = "996e21916327747fdff52cc05a0fa42c4e2071fcee28f977a960acb18e96cf44"
HELPER_SHA = "45639bd5f4a345db7aa08ade548d1d3ac17cb9fd225c95408af3709a4336841d"
SOURCE_BODY = '{"summary": "s02 canonical source evidence bytes"}'
EXPECTED_SOURCES = {
    "baseline": "f99e002f0d2e4002f3694c9f8d4986b56089edaa",
    "candidate": "438cff2995a489f9d5b4e3d6a77fd3b63670bc7d",
}
EXPECTED_PACKAGES = {
    "baseline": ("90363762405041b11e6dd654da9cb6347695399eb37b81c5c09fde91296ac336", "b59b1862b11b8ed3edcdc2e5ee86f9db142268bb5e14571054f6245718fc28f6"),
    "candidate": ("fdd4a72438e83a570c57d7775c4ed0bb0e638542180bd8bf4f0e6a2ca9ec7d1a", "80682b991ee04c55748fb928c2f3b38c29cf7a368930fad1fa7a2d09dde6c06f"),
}


def sha(path: Path) -> str:
    """Return a retained file's SHA-256 digest."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def member(archive: Path, name: str) -> bytes:
    """Read one named npm tar member without trusting the installed path."""
    with tarfile.open(archive, "r:gz") as package:
        stream = package.extractfile(name)
        if stream is None:
            raise ValueError(f"missing archive member {name}")
        return stream.read()


def check_observed(observed: dict) -> None:
    """Check fixture-derived state independently of the TypeScript runner."""
    if observed["embedder"] != "fathomdb-bge-small-en-v1.5":
        raise ValueError("embedder changed")
    if observed["readiness"] != "ready" or observed["unsupported_kinds"] != []:
        raise ValueError("projection not ready")
    if observed["anchor_before"] is not True:
        raise ValueError("pre-query anchor missing")
    if observed["text"] != {"ids": ["A"], "branches": ["text"]}:
        raise ValueError("text anchor changed")
    if not observed["vector"]["ids"] or "vector" not in observed["vector"]["branches"]:
        raise ValueError("vector arm missing")
    if "A" not in observed["hybrid"]["ids"] or "A" not in observed["hybrid_lexical_ids"]:
        raise ValueError("hybrid anchor missing")
    if any(branch not in ("text", "vector") for branch in observed["hybrid"]["branches"]):
        raise ValueError("hybrid branch changed")
    if observed["evidence"] != {"logical_id": "s02-claim", "source_body": SOURCE_BODY}:
        raise ValueError("canonical evidence changed")
    if observed["graph"] != {
        "target_id": "s02-claim", "target_revision": "s02-claim-r1",
        "edge_revision": "s02-edge-r1", "edge_from": "s02-root",
        "edge_to": "s02-claim", "source_body": SOURCE_BODY,
    }:
        raise ValueError("graph evidence changed")
    if observed["erasure"] != {
        "source_ref": "slice135-s02-graph", "nodes_excised": 3, "edges_excised": 1,
    } or observed["second_erasure"] != {"nodes_excised": 0, "edges_excised": 0}:
        raise ValueError("erasure report changed")
    for name in ("after_erasure", "after_reopen"):
        state = observed[name]
        if state["canonical_counts"] != {"graph_nodes": 0, "graph_edges": 0, "corpus_nodes": 32}:
            raise ValueError(f"{name} canonical persistence changed")
        for key in (
            "source_absent", "root_absent", "claim_absent", "anchor_retained",
            "evidence_query_empty", "graph_empty",
        ):
            if state[key] is not True:
                raise ValueError(f"{name} {key} changed")
    if observed["after_reopen"]["readiness"] != "ready":
        raise ValueError("reopened projection not ready")


def main() -> None:
    """Audit both receipts, package bytes and a retained-edge negative control."""
    if sha(ROOT / "runner.mjs") != RUNNER_SHA or sha(ROOT / "s01-helper.mjs") != HELPER_SHA:
        raise ValueError("runner/helper snapshot changed")
    records = {}
    baseline_archives = ROOT.parent / "2026-10-07-ts-s01-noise-pilot"
    for label in ("baseline", "candidate"):
        raw = json.loads((ROOT / f"{label}.json").read_text())
        main_archive = (baseline_archives / "fathomdb-0.8.26.tgz") if label == "baseline" else (ROOT / "candidate-fathomdb-0.8.26.tgz")
        native_archive = (baseline_archives / "fathomdb-linux-x64-gnu-0.8.26.tgz") if label == "baseline" else (ROOT / "candidate-fathomdb-linux-x64-gnu-0.8.26.tgz")
        expected_main, expected_native = EXPECTED_PACKAGES[label]
        if sha(main_archive) != expected_main or sha(native_archive) != expected_native:
            raise ValueError(f"{label} package archive changed")
        artifact = raw["artifact"]
        if raw["source_sha"] != EXPECTED_SOURCES[label] or raw["runner_sha256"] != RUNNER_SHA:
            raise ValueError(f"{label} source/runner identity changed")
        if raw["s01_helper_sha256"] != HELPER_SHA or raw["semantic_ok"] is not True:
            raise ValueError(f"{label} helper/semantic state changed")
        for archive, name, expected in (
            (main_archive, "package/dist/index.js", artifact["module_sha256"]),
            (main_archive, "package/package.json", artifact["package_sha256"]),
            (native_archive, "package/fathomdb.linux-x64-gnu.node", artifact["native_sha256"]),
        ):
            if hashlib.sha256(member(archive, name)).hexdigest() != expected:
                raise ValueError(f"{label} installed artifact differs from archive")
        if artifact["node_version"] != "v25.9.0" or artifact["package_version"] != "0.8.26":
            raise ValueError(f"{label} host/package identity changed")
        if raw["whole_sequence_including_checks_ns"] < sum(raw["stage_ns"].values()):
            raise ValueError(f"{label} whole time shorter than stages")
        if any(value <= 0 for value in raw["stage_ns"].values()):
            raise ValueError(f"{label} nonpositive stage")
        if (ROOT / f"{label}.stdout").stat().st_size or (ROOT / f"{label}.stderr").stat().st_size:
            raise ValueError(f"{label} unexpected process output")
        check_observed(raw["observed"])
        records[label] = raw
    if records["baseline"]["observed"] != records["candidate"]["observed"]:
        raise ValueError("versions differ on materialized observations")
    retained_edge = deepcopy(records["candidate"]["observed"])
    retained_edge["after_reopen"]["canonical_counts"]["graph_edges"] = 1
    try:
        check_observed(retained_edge)
    except ValueError as error:
        negative_control = str(error)
    else:
        raise AssertionError("retained-edge negative control was accepted")
    output = {
        "schema_version": 1,
        "baseline_raw_sha256": sha(ROOT / "baseline.json"),
        "candidate_raw_sha256": sha(ROOT / "candidate.json"),
        "observed_equal": True,
        "semantic_checks_passed": True,
        "negative_control_rejected": negative_control,
        "whole_ms_baseline": round(records["baseline"]["whole_sequence_including_checks_ns"] / 1e6, 3),
        "whole_ms_candidate": round(records["candidate"]["whole_sequence_including_checks_ns"] / 1e6, 3),
    }
    (ROOT / "audit.json").write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps(output))


if __name__ == "__main__":
    main()
