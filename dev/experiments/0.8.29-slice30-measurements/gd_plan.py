"""Generate the fixed G/D campaign catalog; generation never launches preparation."""

import argparse
import json
from pathlib import Path


def build(inputs):
    """Return descriptors for pinned input/artifact references, without opening them."""
    cells=[]

    def add(group,n,repeat,identifier,**fields):
        cell={"id":f"g/{identifier}","group":group,"n":n,"repeat":repeat,
              "measurement":group!="GT","gpu_uuid":inputs["gpu_uuid"],
              "quiet_start":group!="GT","cpu_observation":True,**fields}
        if group!="GT":
            cell["template_from"]=f"g/gt-{n}"
        if group in {"G1","G2","GO"}:
            cell.update(warmups=50,query_count=500,queries_from="g/gt-10000",family="G1_CELL")
        cells.append(cell)

    def preparation(n,lower):
        add("GT",n,0,f"gt-{n}",artifact="gn",family="G_PREPARE",lower=lower,
            corpus=inputs["corpus"][str(n)],questions=inputs["queries"])

    forward=["engine_hybrid","engine_text","engine_mean","sdk_hybrid","sdk_text","sdk_mean"]
    preparation(10000,None)
    add("G1",10000,0,"g1-10000-r0",artifact="gn",passes=forward)
    preparation(100000,"g/gt-10000")
    add("G1",100000,0,"g1-100000-r0",artifact="gn",passes=forward)
    for n in [100000,10000]:
        add("G1",n,1,f"g1-{n}-r1",artifact="gn",passes=list(reversed(forward)))
    schedule=[(f"sdk_{mode}",kind) for mode in ["hybrid","text","mean"] for kind in ["readonly","write"]]
    for repeat,pairs in [(0,schedule),(1,list(reversed(schedule)))]:
        add("G2",100000,repeat,f"g2-r{repeat}",artifact="gn",population="G2",
            passes=[mode for mode,_ in pairs],schedules=[kind for _,kind in pairs])
    core=["engine_hybrid","engine_text","engine_mean"]
    for repeat,states in [(0,[False,True]),(1,[True,False])]:
        for enabled in states:
            add("GO",100000,repeat,f"go-r{repeat}-{'on' if enabled else 'off'}",artifact="go",
                observer=enabled,passes=core if repeat==0 else list(reversed(core)))
    for repeat in [0,1]:
        add("GG",100000,repeat,f"gg-r{repeat}",artifact="go",family="GG_CELL",
            observer=True,warmups=10,samples=100)
    def add_d(group,n,workload,repeat,artifact="do",observer=True):
        suffix=f"-{artifact}-off" if group=="DC" else ""
        cell={"id":f"d/{group.lower()}-{n}-{workload}-r{repeat}{suffix}","group":group,
              "n":n,"repeat":repeat,"measurement":True,"artifact":artifact,"observer":observer,
              "quiet_start":False,"cpu_observation":True,
              "workload":workload,"requires_overlap":observer and workload=="overlap",
              "cohort":1024,"warmups":10,"samples":100,"readiness":True,
              "query_source":inputs["queries"],"lower":None}
        if workload=="held":
            cell.update(family="D_HELD_FIXTURE",release_mode="explicit")
        else:
            cell.update(family="D_QUERY_FIXTURE",empty_start=True,records=[])
        if group=="D1" and n==100000:
            cell["lower"]=f"d/d1-10000-{workload}-r0"
        if group=="D3":
            cell.update(lower=f"d/d1-100000-{workload}-r0",conditional=True)
        cells.append(cell)

    d_order=[(10000,"held"),(10000,"overlap"),(100000,"held"),(100000,"overlap")]
    for repeat,order in [(0,d_order),(1,list(reversed(d_order)))]:
        for n,workload in order:
            add_d("D1",n,workload,repeat)
    for artifact in ["dn","do"]:
        for workload in ["held","overlap"]:
            add_d("DC",10000,workload,0,artifact,False)
    for workload in ["held","overlap"]:
        add_d("D3",300000,workload,0)
    return {"schema":"slice30-gd/catalog-v1", "campaign_seconds":14400,"cell_seconds":900,
            "artifacts":inputs["artifacts"],"resources":inputs["resources"],"cells":cells}


def main():
    """Read input references and print the catalog; all execution is separate."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs",type=Path)
    args=parser.parse_args()
    print(json.dumps(build(json.loads(args.inputs.read_text())),indent=2))


if __name__=="__main__":
    main()
