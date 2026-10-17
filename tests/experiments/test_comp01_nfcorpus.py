"""Deterministic NFCorpus test-split conversion and quality checks."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from experiments.comp01_nfcorpus import convert


def _source(root: Path) -> Path:
    root.mkdir()
    (root / "corpus.jsonl").write_text(
        "\n".join(
            json.dumps(row)
            for row in (
                {"_id": "d2", "title": "Two", "text": "body"},
                {"_id": "d1", "title": "One", "text": "body"},
                {"_id": "d3", "title": "Two", "text": "body"},
            )
        )
        + "\n",
        encoding="utf-8",
    )
    (root / "queries.jsonl").write_text(
        '{"_id":"q2","text":"two"}\n'
        '{"_id":"q1","text":"one"}\n'
        '{"_id":"unused","text":"unused"}\n',
        encoding="utf-8",
    )
    (root / "qrels").mkdir()
    (root / "qrels/test.tsv").write_text(
        "query-id\tcorpus-id\tscore\nq2\td2\t2\nq1\td1\t1\nq2\td3\t0\n",
        encoding="utf-8",
    )
    return root


def test_conversion_selects_only_qrel_queries_and_freezes_hashes(tmp_path):
    raw = _source(tmp_path / "raw")
    one = tmp_path / "one"
    two = tmp_path / "two"
    manifest = convert(raw, one, expected_documents=3, expected_queries=2)
    assert manifest == convert(raw, two, expected_documents=3, expected_queries=2)
    assert manifest["counts"] == {"documents": 3, "queries": 2, "judgments": 3}
    assert manifest["test_query_ids"] == ["q1", "q2"]
    assert (
        manifest["body_mapping"] == "title + two newlines + text; text if title empty"
    )
    assert manifest["duplicate_body_groups"] == [["d2", "d3"]]
    assert manifest["queries_with_relevant_duplicate_body"] == ["q2"]
    for name, digest in manifest["converted_sha256"].items():
        assert hashlib.sha256((one / name).read_bytes()).hexdigest() == digest
        assert (one / name).read_bytes() == (two / name).read_bytes()
    assert json.loads((one / "queries.json").read_text()) == [
        {"id": "q1", "text": "one"},
        {"id": "q2", "text": "two"},
    ]
    assert json.loads((one / "gold.json").read_text()) == {
        "q1": {"d1": 1},
        "q2": {"d2": 2, "d3": 0},
    }


@pytest.mark.parametrize(
    "damage", ["duplicate_doc", "missing_qrel_doc", "missing_query"]
)
def test_conversion_rejects_id_drift_before_creating_output(tmp_path, damage):
    raw = _source(tmp_path / "raw")
    if damage == "duplicate_doc":
        with (raw / "corpus.jsonl").open("a", encoding="utf-8") as stream:
            stream.write('{"_id":"d1","title":"x","text":"x"}\n')
    elif damage == "missing_qrel_doc":
        path = raw / "qrels/test.tsv"
        path.write_text(path.read_text().replace("q2\td2", "q2\td404"))
    else:
        path = raw / "queries.jsonl"
        path.write_text(path.read_text().replace('"_id":"q2"', '"_id":"gone"'))
    output = tmp_path / "converted"
    with pytest.raises(ValueError):
        convert(raw, output, expected_documents=3, expected_queries=2)
    assert not output.exists()
