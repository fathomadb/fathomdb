"""Guard the installed-wheel S02 sequence against vacuous state results."""

from __future__ import annotations

import hashlib
import importlib.util
from pathlib import Path
import sys

import pytest


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location(
    "slice135_python_s02", SCRIPTS / "slice135_python_s02.py"
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def test_graph_seed_has_exact_source_provenance_and_one_edge() -> None:
    records = MODULE.make_graph_records()
    assert len(records) == 4
    assert all(record.get("kind") == "doc" for record in records[:3])
    source = records[0]
    digest = hashlib.sha256(source["body"].encode()).hexdigest()
    assert [record["logical_id"] for record in records[:3]] == [
        "s02-source",
        "s02-root",
        "s02-claim",
    ]
    for record in [*records[1:3], records[3]["edge"]]:
        provenance = record["provenance"]
        assert provenance["source_revision_id"] == "s02-source-r1"
        assert provenance["canonical_source_hash"]["digest_hex"] == digest
        assert record["source_id"] == MODULE.GRAPH_SOURCE
    assert records[3]["edge"]["from"] == "s02-root"
    assert records[3]["edge"]["to"] == "s02-claim"


def good_observation() -> dict:
    return {
        "embedder": "fathomdb-bge-small-en-v1.5",
        "unsupported_kinds": [],
        "readiness": "ready",
        "anchor_before": True,
        "text": {"ids": ["A"], "branches": ["text"]},
        "vector": {"ids": ["B"], "branches": ["vector"]},
        "hybrid": {"ids": ["A"], "branches": ["text"]},
        "hybrid_lexical_ids": ["A"],
        "evidence": {"logical_id": "s02-claim", "source_body": MODULE.SOURCE_BODY},
        "graph": {
            "target_id": "s02-claim",
            "target_revision": "s02-claim-r1",
            "edge_revision": "s02-edge-r1",
            "edge_from": "s02-root",
            "edge_to": "s02-claim",
            "source_body": MODULE.SOURCE_BODY,
        },
        "erasure": {
            "source_ref": MODULE.GRAPH_SOURCE,
            "nodes_excised": 3,
            "edges_excised": 1,
        },
        "second_erasure": {"nodes_excised": 0, "edges_excised": 0},
        "after_erasure": {
            "source_absent": True,
            "root_absent": True,
            "claim_absent": True,
            "anchor_retained": True,
            "evidence_query_empty": True,
            "graph_empty": True,
            "canonical_counts": {
                "graph_nodes": 0,
                "graph_edges": 0,
                "corpus_nodes": 32,
            },
        },
        "after_reopen": {
            "source_absent": True,
            "root_absent": True,
            "claim_absent": True,
            "anchor_retained": True,
            "evidence_query_empty": True,
            "graph_empty": True,
            "readiness": "ready",
            "canonical_counts": {
                "graph_nodes": 0,
                "graph_edges": 0,
                "corpus_nodes": 32,
            },
        },
    }


def test_observation_guard_rejects_lost_evidence_or_surviving_erasure() -> None:
    observed = good_observation()
    MODULE.validate_observations(observed)
    observed["evidence"]["source_body"] = "wrong"
    with pytest.raises(AssertionError, match="canonical evidence"):
        MODULE.validate_observations(observed)
    observed = good_observation()
    observed["after_reopen"]["claim_absent"] = False
    with pytest.raises(AssertionError, match="reopened erased"):
        MODULE.validate_observations(observed)
    observed = good_observation()
    observed["erasure"]["nodes_excised"] = 2
    with pytest.raises(AssertionError, match="erasure count"):
        MODULE.validate_observations(observed)


def test_observation_guard_rejects_retained_edge_and_vacuous_retrieval() -> None:
    observed = good_observation()
    observed["after_reopen"]["canonical_counts"]["graph_edges"] = 1
    with pytest.raises(AssertionError, match="canonical persistence"):
        MODULE.validate_observations(observed)
    observed = good_observation()
    observed["vector"]["ids"] = []
    with pytest.raises(AssertionError, match="vector branch"):
        MODULE.validate_observations(observed)
    observed = good_observation()
    observed["hybrid_lexical_ids"] = []
    with pytest.raises(AssertionError, match="hybrid lexical"):
        MODULE.validate_observations(observed)
    observed = good_observation()
    observed["graph"]["edge_from"] = "wrong"
    with pytest.raises(AssertionError, match="graph evidence"):
        MODULE.validate_observations(observed)
