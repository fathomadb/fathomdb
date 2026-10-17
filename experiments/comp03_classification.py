"""Measurement classifications for the COMP-01/02/03 receipts.

The COMP campaign, COMP-02b latency, COMP-03 retrieval-sweep, and COMP-03
answer receipts were written without an embedded measurement plan, so each
gets a post-hoc plan file, a post-cutover plan amendment in the classification
policy, and a classification sidecar.
Every scalar leaf of the receipt's ``metrics.json`` is bound to a metric with
its contributing components or excluded with a reason; the library validator
(``experiments.measurement_classification``) is the check.

    python -m experiments.comp03_classification write RUN_ID [RUN_ID ...]
    python -m experiments.comp03_classification validate RUN_ID [RUN_ID ...]
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

from experiments import _lib
from experiments import measurement_classification as mc

ROOT = Path(__file__).resolve().parent.parent
EXPERIMENTS = ROOT / "experiments"
POLICY = EXPERIMENTS / mc.CURRENT_POLICY_NAME

ARM_COMPONENTS = {"fathomdb-fts": "fts", "fathomdb-hybrid": "hybrid", "lancedb-fts": "lancedb"}
ENGINE_COMPONENTS = {"fts", "hybrid", "body", "projected"}
COMPONENTS = {
    "corpus": ("corpus_gold", "LongMemEval-S cleaned rounds and gold (comp03_answer.convert_question, gold_for)"),
    "fts": ("fathomdb_engine_search", "FathomDB Engine.search_text_only over plain round bodies"),
    "hybrid": ("fathomdb_engine_search",
               "FathomDB Engine.search over plain round bodies with a searchable dense vector projection"),
    "lancedb": ("external_retrieval", "LanceDB 0.40 native FTS index, library defaults"),
    "metric": ("deterministic_metric", "COMP-03 retrieval metrics, accuracy and paired bootstrap"),
    "answerer": ("answer_generator", "openai/gpt-5.4 answerer through Airlock"),
    "judge": ("semantic_judge", "openai/gpt-5.4 LongMemEval answer-check judge through Airlock"),
}
COMP01_COMPONENTS = {
    "corpus": ("corpus_gold", "BEIR NFCorpus test split documents, queries, and qrels (comp01_nfcorpus)"),
    "fts": ("fathomdb_engine_search", "FathomDB Engine.search_text_only over plain document bodies"),
    "hybrid": ("fathomdb_engine_search",
               "FathomDB Engine.search over plain document bodies with a searchable dense vector projection"),
    "lancedb": ("external_retrieval", "LanceDB 0.40 native FTS index, library defaults"),
    "metric": ("deterministic_metric", "COMP-01 nDCG, recall, and MRR at 10 plus latency percentiles"),
}
COMP02_COMPONENTS = {
    "corpus": ("corpus_gold", "SCALE-02 TC-5 input pack with hashed bench_cohort (comp02_inputs)"),
    "body": ("fathomdb_engine_search", "FathomDB Engine.search_text_only over node bodies"),
    "projected": ("fathomdb_engine_search",
                  "FathomDB Engine.search_projected_text over the text projection, optional bench_cohort filter"),
    "surrealdb": ("external_retrieval", "SurrealDB 3.3 embedded RocksDB through experiments/comp_drivers/surrealdb"),
    "metric": ("deterministic_metric", "COMP-02 latency percentiles and build, ingest, and reopen medians"),
}
CALL_PATHS = {
    "fts": ("fts-path", "Engine.search_text_only"),
    "hybrid": ("hybrid-path", "Engine.search"),
    "lancedb": ("lancedb-path", "external_retrieval"),
    "body": ("body-path", "Engine.search_text_only"),
    "projected": ("projected-path", "Engine.search_projected_text"),
    "surrealdb": ("surrealdb-path", "external_retrieval"),
}
RUNNERS = {
    "comp-03-retrieval-sweep": ["experiments/comp03_retrieval.py", "experiments/comp03_answer.py",
                                "experiments/comp_fts.py", "experiments/fathomdb_test_setup.py",
                                "experiments/_lib.py"],
    "comp-03-hermes-answer": ["experiments/comp03_answer.py", "experiments/comp_fts.py",
                              "experiments/fathomdb_test_setup.py", "experiments/_lib.py"],
    "comp01": ["experiments/comp_fts.py", "experiments/comp_record.py", "experiments/_lib.py"],
    "comp02": ["experiments/comp_fts.py", "experiments/comp02_report.py", "experiments/comp_record.py",
               "experiments/_lib.py"],
    "comp-02b-latency": ["experiments/comp02b_latency.py", "experiments/comp_record.py", "experiments/_lib.py"],
}
CAMPAIGN_ARMS = {
    "comp-01-comp01-hybrid-": {"fathomdb": "fts", "fathomdb-hybrid": "hybrid"},
    "comp-01-comp01-": {"fathomdb": "fts", "lancedb": "lancedb"},
    "comp-02a-comp02a-": {"fathomdb": "body", "surrealdb": "surrealdb"},
    "comp-02b-comp02b-": {"fathomdb": "projected", "surrealdb": "surrealdb"},
}


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _json_sha(path: Path) -> str:
    return _sha(path.read_bytes())


def _git_blob(commit: str, path: str) -> dict[str, Any]:
    locator = f"{commit}:{path}"
    data = mc._git_blob(ROOT, locator)
    return {"locator": locator, "locator_kind": "git_blob", "sha256": _sha(data)}


def _config_artifact(record: dict[str, Any], commit: str) -> dict[str, Any] | None:
    """The receipt's config file, checked against its canonical config digest."""
    path = record["config"].get("path")
    if not path:
        return None
    marker = "experiments/configs/"
    relative = marker + path.split(marker, 1)[1] if marker in path else f"{marker}comp-03/{Path(path).name}"
    data = (ROOT / relative).read_bytes()
    if hashlib.sha256(_lib.canonical_json(json.loads(data)).encode("utf-8")).hexdigest() != record["config"]["sha256"]:
        raise mc.ClassificationError(f"{relative} does not match the receipt's config digest")
    try:
        locator = _git_blob(commit, relative)
    except mc.ClassificationError:
        # A config run from outside the tree and committed afterwards has no blob at the run commit.
        locator = {"locator": relative, "locator_kind": "repository_path", "sha256": _sha(data)}
    return {"id": "configuration", **locator, "role": "configuration", "measurement_root_json_pointers": []}


def _segments(pointer: str) -> list[str]:
    return [part.replace("~1", "/").replace("~0", "~") for part in pointer[1:].split("/")]


def _metric_id(pointer: str) -> str:
    return ".".join(_segments(pointer))


# --- leaf rules ----------------------------------------------------------------


def _sweep_leaf(parts: list[str], value: Any, metrics: dict[str, Any]) -> tuple[str, Any]:
    """('exclude', reason) or ('metric', contributing component ids)."""
    head = parts[0]
    if head in {"schema_version", "device"}:
        return "exclude", "run_control"
    if head == "questions":
        return "metric", ["corpus"]
    if head == "blank_rounds_skipped":
        return "metric", ["corpus"]
    if head == "arms":
        if parts[-1] == "version":
            return "exclude", "identifier_or_hash"
        return "metric", ["corpus", ARM_COMPONENTS[parts[1]], "metric"]
    if head == "deltas":
        if parts[1] in {"treatment", "control"}:
            return "exclude", "run_control"
        return "metric", ["corpus", "fts", "hybrid", "metric"]
    if head == "decision":
        if parts[1] == "rule":
            return "exclude", "run_control"
        return "metric", ["corpus", "fts", "hybrid", "metric"]
    raise mc.ClassificationError(f"unclassified sweep leaf: /{'/'.join(parts)}")


def _pair_arms(name: str) -> list[str]:
    left, right = name.split("_vs_")
    return [ARM_COMPONENTS[left], ARM_COMPONENTS[right]]


def _answer_leaf(parts: list[str], value: Any, metrics: dict[str, Any]) -> tuple[str, Any]:
    head = parts[0]
    end_to_end = ["answerer", "judge", "corpus", "metric"]
    if head == "cost_usd" or parts[-1] == "usd":
        return "exclude", "cost_budget"
    if head == "arm_gate":
        return "exclude", "identifier_or_hash" if parts[-1] == "sweep_summary_sha256" else "run_control"
    if head == "question_outcomes":
        if parts[-1] == "question_id":
            return "exclude", "identifier_or_hash"
        return "metric", [ARM_COMPONENTS[parts[-1]], "answerer", "judge", "corpus"]
    if head in {"abstention_n", "selection_matches_seed"}:
        return "metric", ["corpus"]
    if head == "answer":
        return "metric", ["fts", "hybrid", *end_to_end]
    if head == "answer_pairs":
        return "metric", [*_pair_arms(parts[1]), *end_to_end]
    if head == "accuracy_by_arm":
        return "metric", [ARM_COMPONENTS[parts[1]], *end_to_end]
    if head == "retrieval":
        return "metric", ["corpus", ARM_COMPONENTS[parts[1]], "metric"]
    if head == "timing":
        arm, field = ARM_COMPONENTS[parts[1]], parts[2]
        if field == "version":
            return "exclude", "identifier_or_hash"
        if field == "answer_p50_s":
            return "metric", ["answerer"]
        if field == "judge_p50_s":
            return "metric", ["judge"]
        if field == "evidence_chars_p50":
            return "metric", ["corpus", arm]
        return "metric", [arm]
    raise mc.ClassificationError(f"unclassified answer leaf: /{'/'.join(parts)}")


def _campaign_leaf(arms: dict[str, str]):
    def rule(parts: list[str], value: Any, metrics: dict[str, Any]) -> tuple[str, Any]:
        head = parts[0]
        if head in {"roster", "cells_complete", "host"}:
            return "exclude", "run_control"
        if head == "versions":
            return "exclude", "identifier_or_hash"
        if head in {"documents", "affected_queries"}:
            return "metric", ["corpus"]
        if head == "median_by_arm":
            return "metric", ["corpus", arms[parts[1]], "metric"]
        if head in {"cells", "medians"}:
            field = parts[2]
            if field in {"arm", "cohort"}:
                return "exclude", "identifier_or_hash"
            if field in {"repetition", "reps"}:
                return "exclude", "run_control"
            if field == "documents":
                return "metric", ["corpus"]
            return "metric", ["corpus", arms[metrics[head][int(parts[1])]["arm"]], "metric"]
        raise mc.ClassificationError(f"unclassified campaign leaf: /{'/'.join(parts)}")
    return rule


def _latency_component(cell: str) -> str:
    return "body" if cell == "body-text-only" else "projected"


def _latency_plan_id(metrics: dict[str, Any]) -> str:
    """v1 summaries timed every cell on one open; v2 opens the store per cell, so it gets its own plan."""
    return "comp02b-latency-v2" if metrics.get("schema_version") == "comp-02b.latency.summary.v2" else "comp02b-latency-v1"


def _latency_leaf(parts: list[str], value: Any, metrics: dict[str, Any]) -> tuple[str, Any]:
    head = parts[0]
    if head in {"schema_version", "measurement"}:
        return "exclude", "run_control"
    if head == "inputs":
        return "exclude", "identifier_or_hash"
    if head in {"documents", "queries"}:
        return "metric", ["corpus"]
    if head == "build_s":
        return "metric", ["corpus", "projected"]
    if head == "cells":
        if parts[2] == "n":
            return "metric", ["corpus"]
        return "metric", ["corpus", _latency_component(parts[1]), "metric"]
    raise mc.ClassificationError(f"unclassified latency leaf: /{'/'.join(parts)}")


def _claim(claim_id: str, layer: str, pattern: str):
    import re

    return lambda ids: [{"id": claim_id, "layer": layer, "metric_ids": [i for i in ids if re.fullmatch(pattern, i)]}]


# --- documents -----------------------------------------------------------------


def _shape(record: dict[str, Any]) -> dict[str, Any]:
    experiment = record["experiment"]
    metrics = record["metrics"]
    for prefix, arms in CAMPAIGN_ARMS.items():
        if experiment.startswith(prefix):
            comp01 = prefix.startswith("comp-01")
            ids = sorted(set(arms.values()))
            arm_rows = [(f"{arm}-arm", component) for arm, component in arms.items()]
            if comp01:
                counts = {component: min(c["queries"] for c in metrics["cells"] if arms[c["arm"]] == component)
                          for component in ids}
                claims = _claim("ndcg-by-arm", "data_plane", r"median_by_arm\.[^.]+\.ndcg_at_10")
                plan_id = "comp01-nfcorpus-hybrid-v1" if "hybrid" in prefix else "comp01-nfcorpus-fts-v1"
            else:
                counts = {arms[row["arm"]]: row["reps"] for row in metrics["medians"]}
                field = "steady_p50_ms" if prefix.startswith("comp-02a") else "p50_ms"
                claims = _claim(f"{field.replace('_', '-')}-by-arm", "data_plane", rf"medians\.\d+\.{field}")
                plan_id = "comp02a-ingest-scale-v1" if prefix.startswith("comp-02a") else "comp02b-filtered-v1"
            return {"rule": _campaign_leaf(arms), "components": ["corpus", *ids, "metric"],
                    "table": COMP01_COMPONENTS if comp01 else COMP02_COMPONENTS,
                    "comparisons": [(f"{experiment.split('-')[2]}-arms", arm_rows)], "claims": claims,
                    "witness_count": counts, "plan_id": plan_id,
                    "plan_dir": "experiments/configs/comp-measurement-plans",
                    "runners": RUNNERS["comp01" if comp01 else "comp02"]}
    if experiment == "comp-02b-latency":
        cells = metrics["cells"]
        counts = {component: sum(v["n"] for k, v in cells.items() if _latency_component(k) == component)
                  for component in ("body", "projected")}
        return {"rule": _latency_leaf, "components": ["corpus", "body", "projected", "metric"],
                "table": COMP02_COMPONENTS,
                "comparisons": [("latency-projected-vs-body", [("latency-projected", "projected"),
                                                               ("latency-body", "body")])],
                "claims": _claim("latency-p50-by-cell", "data_plane", r"cells\.[^.]+\.p50_ms"),
                "witness_count": counts, "plan_id": _latency_plan_id(metrics), "plan_dir": "experiments/configs/comp-measurement-plans",
                "runners": RUNNERS[experiment]}
    if experiment == "comp-03-retrieval-sweep":
        return {"rule": _sweep_leaf, "components": ["corpus", "fts", "hybrid", "lancedb", "metric"],
                "comparisons": [("sweep-hybrid-vs-fts", [("sweep-fts", "fts"), ("sweep-hybrid", "hybrid")])],
                "claims": lambda ids: [{"id": "hybrid-adoption-verdict", "layer": "data_plane",
                                        "metric_ids": [i for i in ids if i.startswith("decision.")]}],
                "witness_count": metrics["questions"], "plan_id": "comp03-retrieval-sweep-v1"}
    if experiment == "comp-03-hermes-answer":
        return {"rule": _answer_leaf,
                "components": ["corpus", "fts", "hybrid", "lancedb", "metric", "answerer", "judge"],
                "comparisons": [
                    ("answer-hybrid-vs-fts", [("answer-fts", "fts"), ("answer-hybrid", "hybrid")]),
                    ("answer-fts-vs-lancedb", [("answer-fts-l", "fts"), ("answer-lancedb", "lancedb")]),
                ],
                "claims": lambda ids: [{"id": "answer-accuracy-by-arm", "layer": "end_to_end",
                                        "metric_ids": [i for i in ids if i.startswith("accuracy_by_arm.")]}],
                "witness_count": metrics["answer"]["n"], "plan_id": "comp03-hermes-answer-v2"}
    raise mc.ClassificationError(f"no COMP classification shape for {experiment}")


def build(run_id: str) -> tuple[dict[str, Any], dict[str, Any], Path]:
    """Return (plan, sidecar, plan path) for one receipt."""
    run_dir = EXPERIMENTS / "runs" / run_id
    record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
    metrics = json.loads((run_dir / "metrics.json").read_text(encoding="utf-8"))
    shape = _shape({**record, "metrics": metrics})
    table = shape.get("table", COMPONENTS)
    counts = shape["witness_count"]
    commit = record["code"]["git_sha"]
    plan_path = f"{shape.get('plan_dir', 'experiments/configs/comp-03')}/{shape['plan_id']}-measurement-plan.v1.json"

    components = [{"id": cid, "kind": table[cid][0], "name": table[cid][1]} for cid in shape["components"]]
    call_paths = [{"id": CALL_PATHS[cid][0], "operation": CALL_PATHS[cid][1], "source_artifact_ids": ["metrics"]}
                  for cid in shape["components"] if cid in CALL_PATHS]
    witnesses, comparisons = [], []
    witness_of: dict[str, list[str]] = {component: [] for component in ENGINE_COMPONENTS}
    for comparison_id, arms in shape["comparisons"]:
        arm_rows = []
        for arm_id, component in arms:
            ids = sorted({"corpus", "metric", component} | ({"answerer", "judge"}
                                                            if comparison_id.startswith("answer") else set()))
            witness_ids = []
            if component in ENGINE_COMPONENTS:
                witness_id = f"{arm_id}-search"
                witness_ids = [witness_id]
                witness_of[component].append(witness_id)
                witnesses.append({
                    "id": witness_id, "arm_id": arm_id, "call_path_id": CALL_PATHS[component][0],
                    "component_id": component, "engine_search_state": "executed",
                    "call_count": counts[component] if isinstance(counts, dict) else counts,
                    "count_semantics": "lower_bound",
                    "evidence_kind": "source_result", "source_artifact_ids": ["metrics"],
                })
            arm_rows.append({"id": arm_id, "component_ids": ids, "execution_witness_ids": witness_ids})
        sets = [set(arm["component_ids"]) for arm in arm_rows]
        shared = set.intersection(*sets)
        comparisons.append({"id": comparison_id, "arms": arm_rows,
                            "shared_component_ids": sorted(shared),
                            "differing_component_ids": sorted(set.union(*sets) - shared)})

    bindings, exclusions, roots = [], [], set()
    for pointer in mc._leaf_pointers(metrics, ""):
        parts = _segments(pointer)
        value = mc._json_pointer(metrics, pointer)
        verdict, detail = shape["rule"](parts, value, metrics)
        if verdict == "exclude":
            exclusions.append({"source_artifact_id": "metrics", "json_pointer": pointer, "reason": detail})
            continue
        if isinstance(value, bool):
            value_type, allowed = "boolean", []
        elif isinstance(value, (int, float)):
            value_type, allowed = "number", []
        elif pointer == "/decision/verdict":
            value_type, allowed = "enum", ["adopt", "reject", "not_evaluated"]
        else:
            raise mc.ClassificationError(f"no value contract for {pointer}")
        contributors = sorted(set(detail))
        witness_ids = sorted(w for c in contributors if c in ENGINE_COMPONENTS for w in witness_of[c][:1])
        bindings.append({
            "id": _metric_id(pointer), "source_artifact_id": "metrics", "json_pointer": pointer,
            "value_type": value_type, "allowed_values": allowed,
            "layer": mc._maximum_layer(contributors, {c["id"]: c for c in components}),
            "contributing_component_ids": contributors, "execution_witness_ids": witness_ids,
        })
        roots.add("/" + pointer[1:].split("/")[0])
    claims = shape["claims"]([b["id"] for b in bindings])
    for claim in claims:
        claim["layer"] = max((b["layer"] for b in bindings if b["id"] in claim["metric_ids"]),
                             key=mc.LAYER_RANK.__getitem__)
    measurement_roots = [{"source_artifact_id": "metrics", "json_pointers": sorted(roots)}]
    plan = {
        "schema_version": mc.CURRENT_PLAN_VERSION, "plan_id": shape["plan_id"], "components": components,
        "call_paths": call_paths, "comparisons": comparisons, "metric_bindings": bindings,
        "measurement_roots": measurement_roots, "metric_exclusions": exclusions, "claims": claims,
        "execution_witness_bindings": [],
    }
    artifacts = [
        {"id": "metrics", "locator": f"experiments/runs/{run_id}/metrics.json", "locator_kind": "repository_path",
         "role": "metrics_payload", "sha256": _json_sha(run_dir / "metrics.json"),
         "measurement_root_json_pointers": sorted(roots)},
        {"id": "record", "locator": f"experiments/runs/{run_id}/record.json", "locator_kind": "repository_path",
         "role": "record", "sha256": _json_sha(run_dir / "record.json"), "measurement_root_json_pointers": []},
    ]
    configuration = _config_artifact(record, commit)
    if configuration is not None:
        artifacts.append(configuration)
    for path in shape.get("runners", RUNNERS.get(record["experiment"])):
        artifacts.append({"id": "runner-" + Path(path).stem, **_git_blob(commit, path), "role": "implementation",
                          "measurement_root_json_pointers": []})
    sidecar = {
        "schema_version": mc.SCHEMA_VERSION, "classifier_version": mc.CLASSIFIER_VERSION, "classification_id": "",
        "run_id": run_id, "outcome": "complete", "blocked_reason": None, "measurement_plan_id": plan["plan_id"],
        "source_artifacts": artifacts, "components": components, "call_paths": call_paths,
        "execution_witnesses": witnesses, "metrics": bindings, "metric_exclusions": exclusions,
        "comparisons": comparisons, "claims": claims,
        "migration": {"kind": "native", "manifest_path": None, "manifest_entry_sha256": None,
                      "measurement_plan_id": plan["plan_id"], "measurement_plan_sha256": mc.canonical_sha256(plan)},
    }
    sidecar["classification_id"] = mc.classification_id(sidecar)
    return plan, sidecar, ROOT / plan_path


def write(run_ids: list[str]) -> None:
    policy = json.loads(POLICY.read_text(encoding="utf-8"))
    amendments = policy["postcutover_plan_amendments"]
    for run_id in run_ids:
        plan, sidecar, plan_path = build(run_id)
        plan_path.write_text(json.dumps(plan, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        run_dir = EXPERIMENTS / "runs" / run_id
        mc.write_classification(run_dir, sidecar, repository_root=ROOT, authority=plan)
        entry = {"run_id": run_id, "record_sha256": _json_sha(run_dir / "record.json"),
                 "reason": "missing_plan_in_immutable_postcutover_record",
                 "measurement_plan": {"path": str(plan_path.relative_to(ROOT)),
                                      "sha256": mc.canonical_sha256(plan), "plan_id": plan["plan_id"]}}
        amendments[:] = [a for a in amendments if a["run_id"] != run_id] + [entry]
    POLICY.write_text(json.dumps(policy, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def validate(run_ids: list[str]) -> None:
    """Run the tree validator's per-run checks for these receipts only."""
    policy = json.loads(POLICY.read_text(encoding="utf-8"))
    amendments = mc._parse_postcutover_plan_amendments(policy["postcutover_plan_amendments"])
    for run_id in run_ids:
        run_dir = EXPERIMENTS / "runs" / run_id
        record_path = run_dir / "record.json"
        record = json.loads(record_path.read_text(encoding="utf-8"))
        reference = mc._resolve_postcutover_plan_reference(record_path, record, amendments)
        plan = mc._load_measurement_plan(ROOT, reference)
        sidecar = json.loads((run_dir / mc.SIDECAR_NAME).read_text(encoding="utf-8"))
        mc.validate_classification(sidecar, repository_root=ROOT, authority=plan)
        print(f"PASS {run_id}: {len(sidecar['metrics'])} metrics, {len(sidecar['metric_exclusions'])} exclusions")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("write", "validate"))
    parser.add_argument("run_ids", nargs="+")
    args = parser.parse_args(argv)
    if args.command == "write":
        write(args.run_ids)
    validate(args.run_ids)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
