"""Tiny actual GT-to-GG closed-copy controller route; not a natural campaign."""

import fcntl
import hashlib
import json
import os
from pathlib import Path
import sqlite3
from contextlib import closing
import sys
import tempfile
import time
import unittest

from hypothesis import Phase, given, settings, strategies as st

from test_g_text import GPU, _accounting
from test_gd_campaign import actual_resources

sys.path.insert(0,str(Path(__file__).parents[1]))
from gd_campaign import run_manifest  # noqa: E402

STAGES=[("time","baseline",100),("time","before",80),("time","inside",100),
        ("time","after",120),("governed","valid",100),("governed","deleted",100)]


class GGCampaignTests(unittest.TestCase):
    def check_cell(self,warmups,samples):
        task=Path(os.environ["S30_GD_TASK"])
        artifacts=Path(os.environ["S30_GO_ARTIFACTS"])
        identities={}
        for name in ["gn","go"]:
            binary=artifacts/name
            proof=json.loads((artifacts/f"{name}-features.json").read_text())
            digest=hashlib.sha256(binary.read_bytes()).hexdigest()
            self.assertEqual(proof["binary_sha256"],digest,"actual artifact setup NOT_RED")
            self.assertIs(proof["normal_gn"],name=="gn")
            engine=[row for row in proof["artifacts"] if row["target"]["name"]=="fathomdb_engine"]
            self.assertEqual(len(engine),1)
            self.assertEqual("slice30-measurement" in engine[0]["features"],name=="go")
            self.assertFalse(any(feature=="test-hooks" or feature.startswith("tc5-benchmark") for row in proof["artifacts"] for feature in row["features"]))
            identities[name]={"path":str(binary),"sha256":digest}
        if os.environ.get("CUDA_VISIBLE_DEVICES")!=GPU or os.environ.get("FATHOMDB_EMBED_DEVICE")!="cuda:0":
            raise RuntimeError("approved forced CUDA required NOT_RED")
        root=Path(tempfile.mkdtemp(prefix="s30-gg-campaign-"))
        bodies=[f"closed GG source épreuve {i}" for i in range(65)]
        corpus,questions=root/"corpus.jsonl",root/"questions.jsonl"
        corpus.write_text("".join(json.dumps(body)+"\n" for body in bodies))
        questions.write_text(json.dumps("épreuve")+"\n")
        gt={"id":"g/gt-source","family":"G_PREPARE","artifact":"gn","n":65,"corpus":str(corpus),
            "questions":str(questions),"query_count":1,"gpu_uuid":GPU}
        gg={"id":"g/gg-source","family":"GG_CELL","artifact":"go","n":65,"template_from":gt["id"],
            "warmups":warmups,"samples":samples,"observer":True,"gpu_uuid":GPU}
        manifest={"schema":"slice30-gd/source-fixture-v1","campaign_seconds":14400,"cell_seconds":180,
                  "artifacts":identities,"cells":[gt,gg]}
        end=time.monotonic()+150
        observations=[]
        def monitor(pid):
            observations.append(_accounting(pid,min(end,time.monotonic()+5)))
            return None
        with (task/"locks"/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before=_accounting(None,min(end,time.monotonic()+5))
            if before["util"] or before["total_MiB"]-before["used_MiB"]<8192:
                raise RuntimeError("approved GPU not quiet/free NOT_RED")
            outcomes=run_manifest(manifest,root/"results",monitor=monitor,resources=actual_resources())
        prepared,measured=outcomes
        self.assertEqual(prepared["status"],"complete","real normal GT preparation NOT_RED")
        self.assertEqual(measured["exit"],0,"real copied GG entry setup NOT_RED")
        self.assertTrue(measured["cleanup"]["complete"])
        for outcome in outcomes:
            self.assertTrue(any(row["pid"]==outcome["pid"] and row["own_MiB"]>0 and row["util"]>0 for row in observations),"actual own-PID CUDA engagement absent NOT_RED")
        source=Path(prepared["attempt"])/"case-0/template.sqlite"
        source_hash=json.loads((source.parent/"template.json").read_text())["closed_main_sha256"]
        self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(),source_hash)
        attempt=Path(measured["attempt"])
        copied=json.loads((attempt/"copy.json").read_text())
        self.assertEqual((copied["source_main_sha256"],copied["initial_copy_sha256"]),(source_hash,source_hash))
        control=json.loads((attempt/"gg-control.json").read_text())
        self.assertEqual((control["api_complete"],control["base_rows"],control["pending"],control["selected_cuda_uuid"]),(True,65,0,GPU))
        self.assertEqual(control["literal_window_ids"],[])
        with closing(sqlite3.connect(f"file:{attempt/'store.sqlite'}?mode=ro",uri=True)) as db:
            self.assertEqual(db.execute("SELECT body FROM canonical_nodes ORDER BY write_cursor").fetchall(),[(body,) for body in bodies])
            self.assertEqual(db.execute("SELECT count(*) FROM canonical_nodes c JOIN _fathomdb_vector_rows m ON m.kind=c.kind AND m.write_cursor=c.write_cursor JOIN vector_default_rowids v ON v.rowid=m.rowid JOIN _fathomdb_vector_spans s ON s.vector_rowid=m.rowid AND s.write_cursor=c.write_cursor").fetchone()[0],65)
        paths=[attempt/family/stage for family,stage,_ in STAGES]
        actual=sum((path/"api-proof.json").exists() for path in paths)
        print(f"actual owned GT→GG {attempt} completed_stages={actual}/6 GN={identities['gn']['sha256']} GO={identities['go']['sha256']}",flush=True)
        self.assertEqual(actual,6,"missing generic GG measured stages after real closed-copy API controls")
        owners={"time":set(),"governed":set()}
        for (family,stage,as_of),path in zip(STAGES,paths,strict=True):
            proof=json.loads((path/"api-proof.json").read_text())
            self.assertEqual((proof["warmup_calls"],proof["timed_calls"],proof["api_calls"]),(warmups,samples,warmups+samples))
            for stream,count in [("warmups",warmups),("samples",samples)]:
                rows=[json.loads(line) for line in (path/f"{stream}.jsonl").read_text().splitlines()]
                self.assertEqual(len(rows),count)
                for row in rows:
                    self.assertTrue(row["ok"])
                    self.assertEqual((row["stage"],row["valid_as_of"]),(stage,as_of))
                    self.assertGreater(row["elapsed_ns"],0)
                    expected_query="s30ggwindow" if family=="time" else "s30gggoverned"
                    self.assertEqual(row["query_fingerprint"],hashlib.sha256(expected_query.encode()).hexdigest())
                    observation=row["observer"]
                    self.assertEqual((observation["status"],observation["dropped_delta"],observation["unpaired_records"]),("observed",0,0))
                    self.assertEqual(len(observation["records"]),1)
                    event=observation["records"][0]
                    self.assertEqual(event["snapshot"]["effective_at"],as_of)
                    owners[family].add(event["engine_id"])
                    if stage=="deleted":
                        self.assertEqual(row["ids"],[])
                        self.assertTrue(event["blocks"]["vector"]["degraded"])
        self.assertEqual([len(owners[key]) for key in ["time","governed"]],[1,1])
        self.assertTrue(owners["time"].isdisjoint(owners["governed"]))
        self.assertEqual(hashlib.sha256(source.read_bytes()).hexdigest(),source_hash)

    def test_real_closed_gt_dependency_routes_all_six_gg_stages(self):
        self.check_cell(1,2)

    @settings(max_examples=2,deadline=None,database=None,derandomize=True,phases=[Phase.generate])
    @given(st.sampled_from([(0,1),(1,3)]))
    def test_gg_warmup_timed_counts_preserve_independent_stores(self,case):
        self.check_cell(*case)


if __name__=="__main__":
    unittest.main()
