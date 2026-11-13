"""Literal catalog counts/order plus bounded input-reference preservation."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from hypothesis import Phase, given, settings, strategies as st


class GCatalogTests(unittest.TestCase):
    def generate(self,keys):
        root=Path(tempfile.mkdtemp(prefix="s30-g-catalog-"))
        inputs={"gpu_uuid":"approved-uuid","corpus":{"10000":"/mnt/data/corpus10k.jsonl","100000":"/mnt/data/corpus100k.jsonl"},
                "queries":"/mnt/data/questions500.jsonl",
                "artifacts":{key:{"path":f"/mnt/data/{key}","sha256":key} for key in keys},
                "resources":{"roots":[f"/mnt/data/root-{i}" for i in range(13)],"baseline":69280403456,"task":"/mnt/data/task"}}
        path=root/"inputs.json"
        path.write_text(json.dumps(inputs))
        result=subprocess.run([sys.executable,str(Path(__file__).with_name("gd_plan.py")),str(path)],capture_output=True,text=True,check=True)
        output=json.loads(result.stdout)
        self.assertEqual(output["artifacts"],inputs["artifacts"])
        self.assertEqual(output["resources"],inputs["resources"])
        self.assertEqual((output["campaign_seconds"],output["cell_seconds"]),(14400,900))
        self.assertFalse((root/"campaign-clock.json").exists())
        print(f"catalog CLI {root} descriptors={len(output['cells'])}",flush=True)
        return output["cells"]

    def check_g(self,cells):
        g=[row for row in cells if row["id"].startswith("g/")]
        self.assertEqual(len(g),14,"missing preregistered12 G measurements plus2 preparation descriptors")
        self.assertEqual([(row["group"],row["n"],row["repeat"]) for row in g],
            [("GT",10000,0),("G1",10000,0),("GT",100000,0),("G1",100000,0),
             ("G1",100000,1),("G1",10000,1),("G2",100000,0),("G2",100000,1),
             ("GO",100000,0),("GO",100000,0),("GO",100000,1),("GO",100000,1),
             ("GG",100000,0),("GG",100000,1)])
        self.assertEqual(len({row["id"] for row in g}),14)
        self.assertEqual(sum(row["measurement"] for row in g),12)
        g1=[row for row in g if row["group"]=="G1"]
        forward=["engine_hybrid","engine_text","engine_mean","sdk_hybrid","sdk_text","sdk_mean"]
        self.assertEqual([row["passes"] for row in g1],[forward,forward,list(reversed(forward)),list(reversed(forward))])
        for row in g:
            if row["group"] in {"G1","G2","GO"}:
                self.assertEqual(row["warmups"],50)
                self.assertEqual(row["query_count"],500)
                self.assertEqual(row["queries_from"],"g/gt-10000")
        g2=[row for row in g if row["group"]=="G2"]
        schedule=[("sdk_hybrid","readonly"),("sdk_hybrid","write"),("sdk_text","readonly"),
                  ("sdk_text","write"),("sdk_mean","readonly"),("sdk_mean","write")]
        self.assertEqual([list(zip(row["passes"],row["schedules"],strict=True)) for row in g2],[schedule,list(reversed(schedule))])
        self.assertTrue(all(row["population"]=="G2" and row["artifact"]=="gn" for row in g2))
        go=[row for row in g if row["group"]=="GO"]
        self.assertEqual([row["observer"] for row in go],[False,True,True,False])
        self.assertTrue(all(row["artifact"]=="go" and len(row["passes"])==3 for row in go))
        gt=[row for row in g if row["group"]=="GT"]
        self.assertIsNone(gt[0]["lower"])
        self.assertEqual(gt[1]["lower"],"g/gt-10000")
        self.assertEqual([row["corpus"] for row in gt],["/mnt/data/corpus10k.jsonl","/mnt/data/corpus100k.jsonl"])
        self.assertTrue(all(row["questions"]=="/mnt/data/questions500.jsonl" for row in gt))
        gg=[row for row in g if row["group"]=="GG"]
        self.assertTrue(all((row["warmups"],row["samples"],row["artifact"])==(10,100,"go") for row in gg))

    def test_literal_g_catalog_matches_frozen_workloads(self):
        self.check_g(self.generate(["gn","go","dn","do"]))

    @settings(max_examples=4,deadline=None,database=None,phases=[Phase.generate])
    @given(st.permutations(["gn","go","dn","do"]))
    def test_artifact_reference_order_never_changes_g_schedule(self,keys):
        self.check_g(self.generate(keys))
