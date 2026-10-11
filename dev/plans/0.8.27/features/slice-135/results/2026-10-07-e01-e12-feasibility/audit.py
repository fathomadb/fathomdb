import hashlib
import gzip
import json
from pathlib import Path
import statistics

ROOT = Path(__file__).resolve().parent
NAMES = {
    "open_fresh",
    "open_populated",
    "close",
    "canonical_write",
    "projection",
    "model_cpu",
    "text",
    "vector_stage",
    "hybrid",
    "graph_expand",
    "graph_evidence",
    "erasure",
}
COUNTS = {
    "open_fresh": 1,
    "open_populated": 32,
    "close": 32,
    "canonical_write": 1,
    "projection": 1,
    "model_cpu": 384,
    "text": 10,
    "vector_stage": 10,
    "hybrid": 10,
    "graph_expand": 1,
    "graph_evidence": 1,
    "erasure": 1,
}
EMPTY = {"canonical_rows": 0, "projection_rows": 0, "source_ids": []}
ONE = {"canonical_rows": 1, "projection_rows": 1}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def binary_digest(path):
    with gzip.open(path, "rb") as stream:
        return hashlib.sha256(stream.read()).hexdigest()


def check(raw, output):
    assert set(raw["cells"]) == NAMES
    assert raw["environment_valid"] is True
    assert raw["binary_sha256"] == binary_digest(output / "slice115_workload.gz")
    assert raw["runner_sha256"] == digest(ROOT / "feasibility.py.raw")
    assert raw["protocol_sha256"] == digest(ROOT / "slice115_workload.rs")
    summary = {}
    for name in sorted(NAMES):
        attempts = raw["cells"][name]
        assert len(attempts) == 7, (name, len(attempts))
        values = []
        for attempt in attempts:
            assert attempt["valid"] is True, name
            assert type(attempt["latency_ns"]) is int and attempt["latency_ns"] > 0, (
                name
            )
            assert attempt["correctness_count"] == COUNTS[name], name
            values.append(attempt["latency_ns"])
            if name in ("canonical_write", "projection", "erasure"):
                before = attempt["prestate"]
                after = attempt["poststate"]
                if name == "erasure":
                    before, after = after, before
                assert all(before[k] == v for k, v in EMPTY.items()), name
                assert all(after[k] == v for k, v in ONE.items()), name
                assert len(after["source_ids"]) == 1, name
                assert before["digest"] != after["digest"], name
            if name in ("projection", "model_cpu", "hybrid", "vector_stage"):
                assert 0 < attempt["stage_ns"] <= attempt["latency_ns"], name
            if name == "projection":
                assert attempt["write_ns"] > 0 and attempt["drain_ns"] > 0, name
            if name == "model_cpu":
                assert attempt["model_projection_rows"] == 1, name
        summary[name] = {
            "count": len(values),
            "min_ns": min(values),
            "median_ns": int(statistics.median(values)),
            "max_ns": max(values),
        }
    return summary


baseline = ROOT / "baseline"
candidate = ROOT / "candidate"
b = json.loads((baseline / "raw.json").read_text())
c = json.loads((candidate / "raw.json").read_text())
assert b["source_sha"] == "f99e002f0d2e4002f3694c9f8d4986b56089edaa"
assert c["source_sha"] == "b981b021bdfbeb080b2b4f803e99c118c436b13c"
assert b["corpus_sha256"] == c["corpus_sha256"]
assert b["runner_sha256"] == c["runner_sha256"]
assert b["protocol_sha256"] == c["protocol_sha256"]
assert json.loads((baseline / "run-result.json").read_text())["exit"] == 0
assert json.loads((candidate / "run-result.json").read_text())["exit"] == 0
sb = check(b, baseline)
sc = check(c, candidate)
negative = json.loads((baseline / "raw.json").read_text())
negative["cells"]["erasure"][0]["poststate"]["canonical_rows"] = 1
try:
    check(negative, baseline)
except AssertionError:
    rejected = True
else:
    rejected = False
assert rejected
result = {
    "status": "FEASIBILITY_ONLY",
    "baseline_source_sha": b["source_sha"],
    "candidate_source_sha": c["source_sha"],
    "shared_corpus_sha256": b["corpus_sha256"],
    "negative_control_rejected": rejected,
    "baseline": sb,
    "candidate": sc,
    "limits": [
        "seven samples per cell; no latency percentile or regression verdict",
        "legacy query cells check count, not exact IDs/order",
        "raw profile_ref values have no corresponding sampled profile",
        "no start/end environment snapshots",
        "protocol_sha256 raw field binds workload source, not a frozen Slice 135 protocol",
        "model assets verified before run but not captured at run boundary",
    ],
}
print(json.dumps(result, indent=2, sort_keys=True))
