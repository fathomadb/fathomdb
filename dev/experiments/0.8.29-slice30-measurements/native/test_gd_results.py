"""Real deterministic768 DN/DO-off/DO-on result grouping; not a BGE baseline."""

import hashlib
import json
import os
from pathlib import Path
import sqlite3
from contextlib import closing
import subprocess
import sys
import tempfile
import unittest

from hypothesis import Phase, given, settings, strategies as st

sys.path.insert(0,str(Path(__file__).parents[1]))
from gd_campaign import run_manifest  # noqa: E402

TOOL=Path(__file__).parents[1]/"gd_results.py"


def actual_results(suffix="é",reverse=False):
    root=Path(tempfile.mkdtemp(prefix="s30-real-results-"))
    artifact_root=Path(os.environ["S30_GD_TASK"])/"bin/gd-results"
    artifacts={name:{"path":str(artifact_root/name),"sha256":hashlib.sha256((artifact_root/name).read_bytes()).hexdigest(),
                     "features":["test-hooks"]+(["slice30-measurement"] if name=="do" else []),"profile":"release"}
               for name in ["dn","do"]}
    print(f"actual result children {root} artifacts={artifacts}",flush=True)
    records=[{"id":"row-a","kind":"doc","body":f"pairalpha première {suffix}"},
             {"id":"row-b","kind":"note","body":f"pairβ δεύτερη {suffix}"},
             {"id":"row-c","kind":"doc","body":f"other evidence {suffix}"}]
    if reverse:
        records.reverse()
    queries=[{"id":"qa","query":"pairalpha"},{"id":"qb","query":"pairβ"}]
    base={"family":"D_QUERY_FIXTURE","n":3,"repeat":0,"workload":"overlap","records":[],"empty_start":True,
          "write_records":records,"cohort":2,"warmups":1,"samples":4,"queries":queries,"readiness":True}
    cells=[dict(base,id="d/dn",artifact="dn",observer=False,group="DC"),
           dict(base,id="d/off",artifact="do",observer=False,group="DC"),
           dict(base,id="d/on",artifact="do",observer=True,group="D1")]
    manifest={"schema":"slice30-gd/source-fixture-v1","cell_seconds":180,"campaign_seconds":14400,
              "artifacts":artifacts,"cells":cells}
    outcomes=run_manifest(manifest,root/"results")
    raw={}
    observations={}
    for outcome,cell in zip(outcomes,cells,strict=True):
        assert outcome["status"]=="complete",f"real native outcome controls NOT_RED {outcome}"
        attempt=Path(outcome["attempt"])
        with closing(sqlite3.connect(f"file:{attempt/'query.sqlite'}?mode=ro",uri=True)) as db:
            assert db.execute("SELECT logical_id,kind,body FROM canonical_nodes ORDER BY write_cursor").fetchall()==[(row["id"],row["kind"],row["body"]) for row in records]
            assert db.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows m ON m.kind=c.kind AND m.write_cursor=c.write_cursor JOIN vector_default_rowids v ON v.rowid=m.rowid JOIN _fathomdb_vector_spans s ON s.vector_rowid=m.rowid AND s.write_cursor=c.write_cursor").fetchone()[0]==3
            assert db.execute("SELECT dimension FROM _fathomdb_embedder_profiles WHERE profile='default'").fetchone()[0]==768
        calls=[json.loads(line) for line in (attempt/"queries.jsonl").read_text().splitlines()]
        assert len(calls)==5 and all(row["ok"] and row["elapsed_ns"]>0 for row in calls)
        assert [row["query_id"] for row in calls]==["qa","qa","qb","qa","qb"]
        assert [row["query_sha256"] for row in calls]==[hashlib.sha256(queries[index]["query"].encode()).hexdigest() for index in [0,0,1,0,1]]
        observer=json.loads((attempt/"worker-observer.json").read_text())
        assert observer["available"] is cell["observer"]
        if cell["observer"]:
            assert observer["attribution_complete"] is True
            assert len(observer["query_records"])==5
        else:
            assert observer["records"] is None and observer["dropped"] is None
            assert observer["reason"]==("feature_absent" if cell["artifact"]=="dn" else "observer_disabled")
        raw[cell["id"]]=[row for row in calls if row["phase"]=="timed"]
        observations[cell["id"]]=observer
    manifest=dict(manifest,cells=[*cells,dict(cells[-1],id="d/not-run")])
    path=root/"manifest.json"
    path.write_text(json.dumps(manifest))
    return root,path,raw,observations


def read_result(path,root):
    child=subprocess.run([sys.executable,str(TOOL),str(path),str(root/"results")],capture_output=True,text=True,timeout=10)
    assert child.returncode==0,f"summary CLI setup NOT_RED {child.stderr}"
    return json.loads(child.stdout)


class DResultGroupingTests(unittest.TestCase):
    def groups(self,suffix="é",reverse=False):
        root,path,raw,observed=actual_results(suffix,reverse)
        report=read_result(path,root)
        groups=report.get("cells",[])
        print(f"actual D result grouping {root} groups={len(groups)}/4",flush=True)
        self.assertEqual(len(groups),4,"missing actual mode/identity/repeat/template result groups")
        self.assertEqual([row["id"] for row in groups],["d/dn","d/off","d/on","d/not-run"])
        self.assertEqual([row["status"] for row in groups],["complete","complete","complete","not_run"])
        digests=[]
        for group in groups[:3]:
            identifier=group["id"]
            self.assertEqual((group["mode"],group["adapter"],group["repeat"],group["n"],group["workload"]),("hybrid","engine",0,3,"overlap"))
            self.assertEqual(group["artifact"],"dn" if identifier=="d/dn" else "do")
            self.assertEqual(group["observer_enabled"],identifier=="d/on")
            self.assertEqual(group["warmup_count"],1)
            self.assertEqual([row["query_id"] for row in group["queries"]],["qa","qb","qa","qb"])
            self.assertEqual([row["sample_index"] for row in group["queries"]],list(range(4)))
            self.assertEqual([row["elapsed_ns"] for row in group["queries"]],[row["elapsed_ns"] for row in raw[identifier]])
            self.assertEqual([row["query_fingerprint"] for row in group["queries"]],[row["query_sha256"] for row in raw[identifier]])
            self.assertTrue(all(row["ok"] for row in group["queries"]))
            observer=group["observer"]
            self.assertEqual(observer["available"],observed[identifier]["available"])
            self.assertEqual(observer["dropped"],observed[identifier]["dropped"])
            self.assertEqual(observer["query_dropped"],observed[identifier]["query_dropped"])
            if identifier=="d/on":
                self.assertEqual(observer["query_records"],5)
                self.assertTrue(observer["attribution_complete"])
            else:
                self.assertIsNone(observer["query_records"])
                self.assertEqual(observer["reason"],observed[identifier]["reason"])
            self.assertEqual(group["issues"],[])
            digests.append(group["dataset_sha256"])
        self.assertEqual(len(set(digests)),1,"body identity must exclude wall clocks and observer/artifact settings")
        self.assertEqual(len(digests[0]),64)
        return digests[0]

    def test_actual_normal_off_on_and_missing_cells_keep_raw_identity_groups(self):
        self.groups()

    @settings(max_examples=2,deadline=None,database=None,derandomize=True,phases=[Phase.generate])
    @given(st.sampled_from(["β","中"]))
    def test_ordered_unicode_body_identity_changes_only_with_actual_input(self,suffix):
        forward=self.groups(suffix,False)
        reverse=self.groups(suffix,True)
        self.assertNotEqual(forward,reverse)


if __name__=="__main__":
    unittest.main()
