"""Slice 40 Unit F: the known-run baseline manifest and its drift check."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

ENTRY = {
    "id": "comp01-nfcorpus",
    "metrics": [
        {"name": "fathomdb ndcg@10", "pointer": "/median_by_arm/fathomdb/ndcg_at_10",
         "expected": 0.3265, "tolerance": {"abs": 0.002}},
        {"name": "fathomdb mrr@10", "pointer": "/median_by_arm/fathomdb/mrr_at_10",
         "expected": 0.531, "tolerance": {"abs": 0.005}},
        {"name": "fathomdb p50", "pointer": "/median_by_arm/fathomdb/p50_ms",
         "expected": 1.26, "tolerance": {"rel": 0.25}},
        {"name": "preference recall@5", "pointer": "/arms/fathomdb-fts/per_type/pref/session_recall_any_at_5",
         "expected": 0.733, "tolerance": {"questions": 1, "n": 30}},
    ],
}


def _check():
    from experiments import baseline_check

    return baseline_check


def _summary(ndcg=0.3265, mrr=0.531, p50=1.26, pref=0.733) -> dict:
    return {
        "median_by_arm": {"fathomdb": {"ndcg_at_10": ndcg, "mrr_at_10": mrr, "p50_ms": p50}},
        "arms": {"fathomdb-fts": {"per_type": {"pref": {"session_recall_any_at_5": pref}}}},
    }


def test_every_metric_inside_its_tolerance_passes():
    rows = _check().check(ENTRY, _summary(ndcg=0.3284, mrr=0.527, p50=1.55, pref=0.766))
    assert [row["status"] for row in rows] == ["PASS"] * 4
    assert rows[0]["measured"] == 0.3284 and rows[0]["expected"] == 0.3265
    assert rows[0]["tolerance"] == {"abs": 0.002}


def test_a_metric_outside_its_tolerance_drifts():
    rows = _check().check(ENTRY, _summary(ndcg=0.3240, p50=1.60, pref=0.666))
    status = {row["name"]: row["status"] for row in rows}
    assert status["fathomdb ndcg@10"] == "DRIFT", "0.0025 below with a 0.002 tolerance"
    assert status["fathomdb p50"] == "DRIFT", "+27% with a 25% tolerance"
    assert status["preference recall@5"] == "DRIFT", "two questions down with one allowed"
    assert status["fathomdb mrr@10"] == "PASS"


def test_a_missing_metric_is_reported_and_is_not_a_pass():
    summary = _summary()
    del summary["median_by_arm"]["fathomdb"]["mrr_at_10"]
    rows = _check().check(ENTRY, summary)
    missing = next(row for row in rows if row["name"] == "fathomdb mrr@10")
    assert missing["status"] == "MISSING" and missing["measured"] is None


def test_question_tolerance_is_one_question_of_the_type_count():
    assert _check().allowed_delta({"questions": 1, "n": 30}, expected=0.733) == pytest.approx(1 / 30)
    assert _check().allowed_delta({"abs": 0.002}, expected=0.3265) == 0.002
    assert _check().allowed_delta({"rel": 0.25}, expected=2.0) == 0.5


def test_cli_exits_non_zero_on_drift_or_missing(tmp_path: Path, capsys):
    check = _check()
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps({"schema_version": "fathomdb.baseline.v1", "engine_commit": "x" * 40,
                                    "benchmarks": [ENTRY]}))
    good, bad = tmp_path / "good.json", tmp_path / "bad.json"
    good.write_text(json.dumps(_summary()))
    bad.write_text(json.dumps(_summary(ndcg=0.30)))
    assert check.main([str(manifest), "comp01-nfcorpus", str(good)]) == 0
    assert check.main([str(manifest), "comp01-nfcorpus", str(bad)]) != 0
    out = capsys.readouterr().out
    assert "DRIFT" in out and "fathomdb ndcg@10" in out and "0.3" in out


def test_slice_40_baseline_manifest_lists_the_eight_benchmarks():
    check = _check()
    manifest = check.load_manifest(check.DEFAULT_MANIFEST)
    assert manifest["schema_version"] == "fathomdb.baseline.v1"
    assert [b["id"] for b in manifest["benchmarks"]] == [
        "comp01-nfcorpus", "comp01-hybrid", "comp03-retrieval-sweep", "comp03-answer-v2", "tc5-10k-latency",
        "comp03-answer-mix", "comp02a-10000", "comp02a-17272", "comp02b-10000",
    ]
    for benchmark in manifest["benchmarks"]:
        assert {"inputs", "engine_commit", "wheel_build", "command", "receipt", "metrics"} <= set(benchmark)
        assert benchmark["inputs"]["sha256"] and benchmark["command"]
        assert benchmark["wheel_build"]["compatibility"] == "linux"
        for metric in benchmark["metrics"]:
            assert metric["pointer"].startswith("/") and "tolerance" in metric
    answer = next(b for b in manifest["benchmarks"] if b["id"] == "comp03-answer-v2")
    assert all(m.get("gate") is False for m in answer["metrics"]), "answer accuracy is noise, not a gate"
    mix = next(b for b in manifest["benchmarks"] if b["id"] == "comp03-answer-mix")
    assert [m["pointer"] for m in mix["metrics"]] == [
        "/accuracy_by_arm/fathomdb-fts", "/accuracy_by_arm/fathomdb-hybrid", "/accuracy_by_arm/lancedb-fts",
    ]
    assert all(m.get("gate") is False and m["tolerance"] == {"questions": 3, "n": 60} for m in mix["metrics"])
    assert "provisional" in mix["note"], "the evidence policy and mix are expected to change"
    # Owner ruling Q-H5: COMP-02a and COMP-02b are manifest benchmarks, read from their receipts' metrics.
    for scale_id in ("comp02a-10000", "comp02a-17272"):
        scale = next(b for b in manifest["benchmarks"] if b["id"] == scale_id)
        assert [(m["pointer"], m["tolerance"]) for m in scale["metrics"]] == [
            ("/medians/0/query_ready_s", {"rel": 0.25}),
            ("/medians/0/steady_p50_ms", {"rel": 0.25}),
            ("/medians/0/steady_p95_ms", {"rel": 0.35}),
        ]
        assert all(m["name"].startswith("fathomdb ") for m in scale["metrics"])
        assert scale["summary"].endswith("metrics.json") and scale["receipt"].startswith("comp-02a-")
    filtered = next(b for b in manifest["benchmarks"] if b["id"] == "comp02b-10000")
    expected = []
    for index, cohort in ((0, "c01"), (2, "c10"), (4, "c33")):
        expected += [(f"fathomdb {cohort} p50", f"/medians/{index}/p50_ms", {"rel": 0.25}),
                     (f"fathomdb {cohort} p95", f"/medians/{index}/p95_ms", {"rel": 0.35}),
                     (f"fathomdb {cohort} violations", f"/medians/{index}/violations", {"abs": 0}),
                     (f"fathomdb {cohort} short results", f"/medians/{index}/short_results", {"abs": 0})]
    assert [(m["name"], m["pointer"], m["tolerance"]) for m in filtered["metrics"]] == expected
    assert filtered["receipt"].startswith("comp-02b-comp02b-10000-")


def test_latency_harness_keeps_the_diagnostic_cells_and_percentiles():
    from experiments import comp02b_latency

    assert [cell["id"] for cell in comp02b_latency.CELLS] == [
        "projected-unfiltered", "projected-c01", "projected-c10", "projected-c33", "body-text-only",
        "projected-single-term",
    ]
    timings = [float(i) for i in range(1, 21)]
    assert comp02b_latency.percentiles(timings) == {"p50_ms": 10.5, "p95_ms": 19.0}


def test_engine_identity_records_the_commit_and_the_native_digest():
    pytest.importorskip("fathomdb")
    import hashlib

    import fathomdb
    from fathomdb import _fathomdb

    from experiments import comp_record

    commit = "0123456789abcdef0123456789abcdef01234567"
    identity = comp_record.engine_identity(commit)
    assert identity == {
        "commit": commit,
        "native_sha256": hashlib.sha256(Path(_fathomdb.__file__).read_bytes()).hexdigest(),
        "version": fathomdb.__version__,
    }
    with pytest.raises(ValueError):
        comp_record.engine_identity("not-a-commit")
