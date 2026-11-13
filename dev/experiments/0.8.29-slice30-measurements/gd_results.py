"""Read actual G/D outputs without running children or assigning performance gates."""

import argparse
import hashlib
import json
from pathlib import Path

from control import read_jsonl_records
from metrics import aggregate


def _json(path,issues):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError,UnicodeDecodeError,json.JSONDecodeError):
        issues.append(f"missing/partial {path.name}")
        return {}


def describe(calls,wall_ns):
    """Separate timed/warmup latency populations and all-call whole-cell throughput."""
    result={"all_requests":len(calls),"wall_ns":wall_ns,
            "throughput_all_per_s":len(calls)*1_000_000_000/wall_ns if wall_ns else None,
            "throughput_scope":"all warmup and timed calls over whole cell including preparation, writes, drain, close and cleanup; excludes outcome serialization"}
    for phase in ["timed","warmup"]:
        rows=[dict(row,error=row.get("error_class") or row.get("error") or "unknown")
              for row in calls if row["phase"]==phase]
        population=aggregate(rows,0)
        population.pop("throughput_all_per_s")
        result[phase]=population
    return result


def _dataset(config):
    records=[*config.get("records",[]),*config.get("write_records",[])]
    digest=hashlib.sha256()
    for row in records:
        actual={key:row[key] for key in ["id","kind","body"]}
        digest.update((json.dumps(actual,sort_keys=True,ensure_ascii=False,separators=(",",":"))+"\n").encode())
    return digest.hexdigest()


def summarize(manifest, root):
    """Normalize actual plan/outcome/raw identities; never infer missing worker records as zero."""
    root=Path(root)
    state_issues=[]
    state_path=root/"campaign-state.json"
    state=_json(state_path,state_issues) if state_path.exists() else []
    previous={row["id"]:row for row in state} if isinstance(state,list) else {}
    groups=[]
    for cell in manifest["cells"]:
        outcome=previous.get(cell["id"])
        group={"id":cell["id"],"status":outcome["status"] if outcome else "not_run",
               "artifact":cell["artifact"],"observer_enabled":cell.get("observer"),
               "repeat":cell.get("repeat"),"n":cell.get("n"),"workload":cell.get("workload"),
               "issues":list(state_issues)}
        groups.append(group)
        if outcome is None or outcome.get("attempt") is None:
            if outcome is not None:
                group["reason"]=outcome.get("reason") or outcome.get("error")
            continue
        attempt=Path(outcome["attempt"])
        issues=group["issues"]
        if cell["family"] not in {"D_QUERY_FIXTURE","D_HELD_FIXTURE"}:
            issues.append("result family not yet supported")
            continue
        config_path=attempt/"native-config.json"
        config=_json(config_path if config_path.exists() else attempt/"cell.json",issues)
        calls=read_jsonl_records(attempt/"queries.jsonl",issues)
        observer=_json(attempt/"worker-observer.json",issues)
        timed=[row for row in calls if row["phase"]=="timed"]
        group.update(attempt=str(attempt),mode="hybrid",adapter="engine",
                     dataset_sha256=_dataset(config),
                     dataset_identity_scope="ordered native API id/kind/body inputs; excludes wall clocks and physical database bytes",
                     warmup_count=sum(row["phase"]=="warmup" for row in calls),
                     statistics=describe(calls,outcome.get("cell_elapsed_ns")),
                     queries=[{"query_id":row.get("query_id"),"sample_index":row["sample_index"],
                               "query_fingerprint":row.get("query_sha256"),"elapsed_ns":row["elapsed_ns"],
                               "ok":row["ok"],"error":row.get("error_class") or row.get("error")}
                              for row in timed],
                     observer={"available":observer.get("available"),"reason":observer.get("reason"),
                               "dropped":observer.get("dropped"),"query_dropped":observer.get("query_dropped"),
                               "attribution_complete":observer.get("attribution_complete"),
                               "query_records":len(observer["query_records"]) if isinstance(observer.get("query_records"),list) else None,
                               "worker_records":len(observer["records"]) if isinstance(observer.get("records"),list) else None})
        if issues and group["status"]=="complete":
            group["status"]="incomplete"
    return {"cells":groups,"pairs":[],"gated":False}


def main():
    """Print descriptive results for one selected manifest and output directory."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest",type=Path)
    parser.add_argument("root",type=Path)
    args=parser.parse_args()
    print(json.dumps(summarize(json.loads(args.manifest.read_text()),args.root),indent=2))


if __name__=="__main__":
    main()
