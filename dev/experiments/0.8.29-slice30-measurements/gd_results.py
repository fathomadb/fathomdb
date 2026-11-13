"""Read actual G/D outputs without running children or assigning performance gates."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from control import read_jsonl_records
from metrics import aggregate, paired_difference


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


def pair_groups(left,right):
    """Compare matching successful raw calls, preserving unmatched and attribution issues."""
    issues=[f"{side}: {issue}" for side,group in [("left",left),("right",right)] for issue in group["issues"]]
    issues.extend(f"{group['id']}: workload not complete" for group in [left,right] if group.get("status","complete")!="complete")
    for side,group in [("left",left),("right",right)]:
        observer=group["observer"]
        if observer.get("available") and (observer.get("dropped")!=0 or observer.get("query_dropped")!=0
                                          or observer.get("attribution_complete") is not True):
            issues.append(f"{side}: observer attribution incomplete")
    def keyed(group):
        rows={}
        for row in group["queries"]:
            key=(row["sample_index"],row.get("query_id"),row.get("query_fingerprint"))
            rows.setdefault(key,[]).append(row)
        return rows
    a,b=keyed(left),keyed(right)
    raw=[]
    records=[]
    for key,values in a.items():
        other=b.get(key,[])
        if len(values)!=1 or len(other)!=1 or key[2] is None:
            continue
        first,second=values[0],other[0]
        if not first["ok"] or not second["ok"]:
            continue
        index=len(raw)
        raw.append({"sample_index":key[0],"query_id":key[1],"query_fingerprint":key[2],
                    "left_ns":first["elapsed_ns"],"right_ns":second["elapsed_ns"],
                    "right_minus_left_ms":(second["elapsed_ns"]-first["elapsed_ns"])/1_000_000})
        for adapter,row in [("engine",first),("sdk",second)]:
            records.append(dict(row,adapter=adapter,repeat=0,sample_index=index,query_index=(key[1],key[2])))
    result=paired_difference(records)
    unpaired=len(left["queries"])+len(right["queries"])-2*len(raw)
    return {"left":left["id"],"right":right["id"],"matched_calls":len(raw),
            "unpaired_calls":unpaired,"raw_pairs":raw,"distinct_queries":result["distinct_queries"],
            "right_minus_left_mean_ms":result["sdk_minus_engine_mean_ms"],
            "bootstrap95_ms":result["bootstrap95_ms"],"bootstrap_seed":result["bootstrap_seed"],
            "errors":{side:dict(Counter(row.get("error") or "unknown" for row in group["queries"] if not row["ok"]))
                      for side,group in [("left",left),("right",right)]},
            "complete":bool(raw) and unpaired==0 and not issues,"issues":issues}


def comparisons(groups):
    pairs=[]
    keyed={}
    for group in groups:
        if "dataset_sha256" not in group:
            continue
        key=tuple(group.get(field) for field in ["dataset_sha256","mode","adapter","repeat","n","workload","population","schedule","stage_family","stage","valid_as_of"])
        role="enabled" if group["observer_enabled"] else "disabled" if group["observer_feature"] else "normal"
        keyed.setdefault(key,{}).setdefault(role,[]).append(group)
    for roles in keyed.values():
        for left,right in [("normal","disabled"),("disabled","enabled")]:
            if len(roles.get(left,[]))==len(roles.get(right,[]))==1:
                pairs.append(pair_groups(roles[left][0],roles[right][0]))
    return pairs


def caller_comparisons(groups):
    """Pair normal Core/SDK passes from the same actual cell/template/query population."""
    keyed={}
    for group in groups:
        if group.get("observer_feature") or "cell_id" not in group or "dataset_sha256" not in group:
            continue
        key=tuple(group.get(field) for field in ["cell_id","dataset_sha256","mode","repeat","n","workload","population","schedule"])
        keyed.setdefault(key,{}).setdefault(group.get("adapter"),[]).append(group)
    return [pair_groups(adapters["engine"][0],adapters["sdk"][0]) for adapters in keyed.values()
            if len(adapters.get("engine",[]))==len(adapters.get("sdk",[]))==1]


def observer_summary(calls,enabled):
    """Describe actual per-call observer transport and delta counters."""
    if not enabled:
        return {"available":False,"reason":"observer_disabled","query_records":None,"worker_records":None,
                "dropped":None,"query_dropped":None,"attribution_complete":None,"issues":[]}
    observations=[row.get("observer",{}) for row in calls]
    issues=[issue for row in observations for issue in row.get("issues",[])]
    available=bool(observations) and all(row.get("records_available") is True for row in observations)
    deltas=[row.get("dropped_delta") for row in observations]
    dropped=sum(deltas) if deltas and all(isinstance(value,int) for value in deltas) else None
    if not available:
        issues.append("observer record transport unavailable")
    if dropped is None:
        issues.append("observer counter unavailable")
    elif dropped:
        issues.append("observer records dropped")
    if any(row.get("status") not in {"observed","not_applicable"} or row.get("unpaired_records")!=0 for row in observations):
        issues.append("observer records missing or unpaired")
    complete=available and dropped==0 and not issues
    return {"available":available,"reason":"not_applicable" if observations and all(row.get("status")=="not_applicable" for row in observations) else "observed" if complete else "incomplete",
            "query_records":sum(len(row["records"]) for row in observations) if available else None,
            "worker_records":None,"dropped":dropped,"query_dropped":dropped,
            "attribution_complete":complete,"issues":issues}


def _g_pass(base,cell,attempt,path,mode,outcome,dataset):
    group:dict=dict(base,issues=list(base["issues"]),cell_id=cell["id"],id=f"{cell['id']}/{path.name}")
    issues=group["issues"]
    proof=_json(path/"api-proof.json",issues)
    warmups=read_jsonl_records(path/"warmups.jsonl",issues)
    samples=read_jsonl_records(path/"samples.jsonl",issues)
    calls=[dict(row,phase=phase) for phase,rows in [("warmup",warmups),("timed",samples)] for row in rows]
    stats=describe(calls,proof.get("loop_ns"))
    stats["throughput_scope"]=proof.get("throughput_scope")
    feature=base["observer_feature"] or "observer_setup_ns" in proof
    group.update(mode=mode.split("_",1)[1],adapter=mode.split("_",1)[0],
                 dataset_sha256=dataset,dataset_identity_scope="closed source template SHA256 before writable cell copy",
                 population=cell.get("population","G1"),warmup_count=len(warmups),statistics=stats,
                 whole_cell_elapsed_ns=outcome.get("cell_elapsed_ns"),observer_feature=feature,
                 queries=[{"query_id":row.get("query_id"),"sample_index":row["sample_index"],
                           "query_fingerprint":row.get("query_fingerprint"),"elapsed_ns":row["elapsed_ns"],
                           "ok":row["ok"],"error":row.get("error_class") or row.get("error")} for row in samples],
                 observer={"available":None if feature else False,
                           "reason":"observer_reporting_pending" if feature else "feature_absent",
                           "query_records":None,"worker_records":None,"dropped":None,"query_dropped":None,
                           "attribution_complete":None})
    if feature:
        group["observer"]=observer_summary(calls,cell.get("observer",False))
        issues.extend(group["observer"]["issues"])
        for normalized,raw in zip(group["queries"],samples,strict=True):
            normalized["observer"]=raw.get("observer")
    if issues and group["status"]=="complete":
        group["status"]="incomplete"
    return group


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
               "observer_feature":"slice30-measurement" in manifest["artifacts"][cell["artifact"]].get("features",[]),
               "repeat":cell.get("repeat"),"n":cell.get("n"),"workload":cell.get("workload"),
               "issues":list(state_issues)}
        groups.append(group)
        if outcome is None or outcome.get("attempt") is None:
            if outcome is not None:
                group["reason"]=outcome.get("reason") or outcome.get("error")
            continue
        attempt=Path(outcome["attempt"])
        issues=group["issues"]
        if cell["family"]=="G_PREPARE":
            group["kind"]="preparation"
            continue
        if cell["family"]=="GG_CELL":
            copy=_json(attempt/"copy.json",issues)
            groups.pop()
            for family,stage,as_of in [("time","baseline",100),("time","before",80),("time","inside",100),
                                      ("time","after",120),("governed","valid",100),("governed","deleted",100)]:
                row=_g_pass(group,cell,attempt,attempt/family/stage,"engine_hybrid",outcome,copy.get("source_main_sha256"))
                row.update(id=f"{cell['id']}/{family}/{stage}",population="GG",stage_family=family,stage=stage,valid_as_of=as_of)
                groups.append(row)
            continue
        if cell["family"]=="G1_CELL":
            copy=_json(attempt/"copy.json",issues)
            groups.pop()
            for ordinal,mode in enumerate(cell["passes"]):
                row=_g_pass(group,cell,attempt,attempt/f"pass-{ordinal}",mode,outcome,copy.get("source_main_sha256"))
                if cell.get("schedules"):
                    row["schedule"]=cell["schedules"][ordinal]
                groups.append(row)
            continue
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
    return {"cells":groups,"pairs":comparisons(groups),"caller_pairs":caller_comparisons(groups),"gated":False}


def main():
    """Print descriptive results for one selected manifest and output directory."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest",type=Path)
    parser.add_argument("root",type=Path)
    args=parser.parse_args()
    print(json.dumps(summarize(json.loads(args.manifest.read_text()),args.root),indent=2))


if __name__=="__main__":
    main()
