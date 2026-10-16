"""Measurement classifications for the COMP-03 Unit D receipts.

The COMP-03 retrieval-sweep and v2 answer receipts were written without an
embedded measurement plan, so each gets a post-hoc plan file, a post-cutover
plan amendment in the classification policy, and a classification sidecar.
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

from experiments import measurement_classification as mc

ROOT = Path(__file__).resolve().parent.parent
EXPERIMENTS = ROOT / "experiments"
POLICY = EXPERIMENTS / mc.CURRENT_POLICY_NAME
PLAN_DIR = "experiments/configs/comp-03"

ARM_COMPONENTS = {"fathomdb-fts": "fts", "fathomdb-hybrid": "hybrid", "lancedb-fts": "lancedb"}
ENGINE_COMPONENTS = {"fts", "hybrid"}
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
CALL_PATHS = {
    "fts": ("fts-path", "Engine.search_text_only"),
    "hybrid": ("hybrid-path", "Engine.search"),
    "lancedb": ("lancedb-path", "external_retrieval"),
}
RUNNERS = {
    "comp-03-retrieval-sweep": ["experiments/comp03_retrieval.py", "experiments/comp03_answer.py",
                                "experiments/comp_fts.py", "experiments/fathomdb_test_setup.py",
                                "experiments/_lib.py"],
    "comp-03-hermes-answer": ["experiments/comp03_answer.py", "experiments/comp_fts.py",
                              "experiments/fathomdb_test_setup.py", "experiments/_lib.py"],
}
CONFIGS = {
    "comp-03-retrieval-sweep": "experiments/configs/comp-03/retrieval-sweep.v1.json",
    "comp-03-hermes-answer": "experiments/configs/comp-03/hermes-answer.v2.json",
}


def _sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _json_sha(path: Path) -> str:
    return _sha(path.read_bytes())


def _git_blob(commit: str, path: str) -> dict[str, Any]:
    locator = f"{commit}:{path}"
    data = mc._git_blob(ROOT, locator)
    return {"locator": locator, "locator_kind": "git_blob", "sha256": _sha(data)}


def _segments(pointer: str) -> list[str]:
    return [part.replace("~1", "/").replace("~0", "~") for part in pointer[1:].split("/")]


def _metric_id(pointer: str) -> str:
    return ".".join(_segments(pointer))


# --- leaf rules ----------------------------------------------------------------


def _sweep_leaf(parts: list[str], value: Any) -> tuple[str, Any]:
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


def _answer_leaf(parts: list[str], value: Any) -> tuple[str, Any]:
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


# --- documents -----------------------------------------------------------------


def _shape(record: dict[str, Any]) -> dict[str, Any]:
    experiment = record["experiment"]
    if experiment == "comp-03-retrieval-sweep":
        return {"rule": _sweep_leaf, "components": ["corpus", "fts", "hybrid", "lancedb", "metric"],
                "comparisons": [("sweep-hybrid-vs-fts", [("sweep-fts", "fts"), ("sweep-hybrid", "hybrid")])],
                "claims": lambda ids: [{"id": "hybrid-adoption-verdict", "layer": "data_plane",
                                        "metric_ids": [i for i in ids if i.startswith("decision.")]}],
                "witness_count": record["metrics"]["questions"], "plan_id": "comp03-retrieval-sweep-v1"}
    if experiment == "comp-03-hermes-answer":
        return {"rule": _answer_leaf,
                "components": ["corpus", "fts", "hybrid", "lancedb", "metric", "answerer", "judge"],
                "comparisons": [
                    ("answer-hybrid-vs-fts", [("answer-fts", "fts"), ("answer-hybrid", "hybrid")]),
                    ("answer-fts-vs-lancedb", [("answer-fts-l", "fts"), ("answer-lancedb", "lancedb")]),
                ],
                "claims": lambda ids: [{"id": "answer-accuracy-by-arm", "layer": "end_to_end",
                                        "metric_ids": [i for i in ids if i.startswith("accuracy_by_arm.")]}],
                "witness_count": record["metrics"]["answer"]["n"], "plan_id": "comp03-hermes-answer-v2"}
    raise mc.ClassificationError(f"no COMP-03 classification shape for {experiment}")


def build(run_id: str) -> tuple[dict[str, Any], dict[str, Any], Path]:
    """Return (plan, sidecar, plan path) for one receipt."""
    run_dir = EXPERIMENTS / "runs" / run_id
    record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
    metrics = json.loads((run_dir / "metrics.json").read_text(encoding="utf-8"))
    shape = _shape({**record, "metrics": metrics})
    commit = record["code"]["git_sha"]
    plan_path = f"{PLAN_DIR}/{shape['plan_id']}-measurement-plan.v1.json"

    components = [{"id": cid, "kind": COMPONENTS[cid][0], "name": COMPONENTS[cid][1]} for cid in shape["components"]]
    call_paths = [{"id": CALL_PATHS[cid][0], "operation": CALL_PATHS[cid][1], "source_artifact_ids": ["metrics"]}
                  for cid in shape["components"] if cid in CALL_PATHS]
    witnesses, comparisons = [], []
    witness_of: dict[str, list[str]] = {"fts": [], "hybrid": []}
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
                    "call_count": shape["witness_count"], "count_semantics": "lower_bound",
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
        verdict, detail = shape["rule"](parts, value)
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
        {"id": "configuration", **_git_blob(commit, CONFIGS[record["experiment"]]), "role": "configuration",
         "measurement_root_json_pointers": []},
    ]
    for path in RUNNERS[record["experiment"]]:
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
