"""Fake-transport controls for real CLI paths; no inference or native code."""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run as runner
import score
from experiments import mcp_agent_loop as mal


def transcript(qid="q0"):
    return [
        {"type":"assistant","message":{"content":[{"type":"tool_use","id":"s","name":"mcp__fathomdb__search","input":{}}]}},
        {"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"s","content":'search hits=1\n"l:1:D1:1" arm=lexical score=1.000 cut | a'}]}},
        {"type":"result","subtype":"success","num_turns":2,"total_cost_usd":0.001,"result":'INSTRUCTIONS_SEEN: yes\n'+json.dumps({"qid":qid,"answer":"a"})},
    ]


def make_run(root, item="5"):
    (root/"run.json").write_text(json.dumps({"conv":1,"item":item,"arm":"text","model":"m","questions":["conv1_q0"]}))
    (root/"checkpoint.jsonl").write_text(json.dumps({"qid":"q0","status":"complete","valid":True,"answer_json":{"answer":"a"},"answer":"a","exit":0,"wall_s":1}))
    (root/"q").mkdir()
    (root/"q/q0.stream.jsonl").write_text("\n".join(map(json.dumps,transcript())))


class CurrentControls(unittest.TestCase):
    def test_native_content_busy_word_is_not_an_admission_failure(self):
        self.assertFalse(score.is_busy('fetch store="s"\nbody_bytes=12\nI was "busy" all week'))
        self.assertTrue(score.is_busy('error busy/admission message="full"'))

    def test_item4_requires_fixed30_not_a_smaller_self_declared_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            make_run(root,item="4")
            with self.assertRaises(ValueError): score.score_run(root,None)

    def test_real_run_cli_uses_native_config_checkpoints_and_resumes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); store=root/"all.sqlite"; store.write_text("fake transport store")
            run_dir=root/"smoke"
            args=["run.py","--run-dir",str(run_dir),"--store",str(store),"--conv","1","--model","chosen-model",
                  "--result-format","structured","--qids","q0","--binary","/unused-candidate", "--cap-usd","0.02"]
            calls=[]
            def popen(cmd,**kw):
                calls.append(cmd)
                kw["stdout"].write("\n".join(map(json.dumps,transcript())))
                return SimpleNamespace(returncode=0,communicate=lambda **kw:(None,""))
            q=SimpleNamespace(qid="conv1_q0",question="question")
            with patch.object(sys,"argv",args), patch.object(runner,"load_questions",return_value=([q],{})),patch.object(runner.subprocess,"Popen",side_effect=popen):
                self.assertEqual(runner.main(),0)
            config=json.loads((run_dir/"workers/w0/server.json").read_text())
            self.assertEqual(config["result_format"],"structured")
            self.assertEqual(calls[0][calls[0].index("--model")+1],"chosen-model")
            self.assertEqual(score.score_run(run_dir,None)[0]["cut_refs_shown"],1)
            with patch.object(sys,"argv",args+["--resume"]),patch.object(runner,"load_questions",return_value=([q],{})),patch.object(runner.subprocess,"Popen",side_effect=AssertionError("completed episode retried")):
                self.assertEqual(runner.main(),0)

    def test_timeout_terminates_and_kills_child_process_group(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); worker=root/"worker"; worker.mkdir(); (worker/"cwd").mkdir()
            args=SimpleNamespace(binary="/unused",result_format="text",model="chosen",max_turns=8,episode_cap_usd="0.02",timeout=180,item="5")
            process=SimpleNamespace(pid=123,communicate=unittest.mock.Mock(side_effect=[subprocess.TimeoutExpired("fake",180),subprocess.TimeoutExpired("fake",2),(None,"")]))
            with patch.object(runner.subprocess,"Popen",return_value=process),patch.object(runner.os,"killpg") as kill:
                row=runner.run_question(SimpleNamespace(qid="conv1_q0",question="question"),worker=worker,run_dir=root,args=args,default_store="all")
            self.assertEqual([call.args[1] for call in kill.call_args_list],[runner.signal.SIGTERM,runner.signal.SIGKILL])
            self.assertEqual(row["exit"],"timeout");self.assertFalse(row["valid"])

    def test_actual_judges_strict_parse_failure_is_reserved_and_never_repeated(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);make_run(root)
            calls=[]
            def complete(**kw):
                calls.append(kw)
                return SimpleNamespace(content="malformed")
            judge=object.__new__(score.Judges);judge.mal=mal;judge.model="gpt-4o-mini"
            judge.client=SimpleNamespace(complete=complete,ledger=SimpleNamespace(check=lambda:None))
            judge.prompts=SimpleNamespace(JUDGE_SYSTEM_PROMPT="system")
            question=mal.Question("conv1_q0",1,"question","a",[])
            grader=SimpleNamespace(judge=lambda *a:{"score":1.0},lookup={})
            judge._conv=lambda index:({"conv1_q0":question},grader)
            with self.assertRaises(ValueError):score.score_run(root,judge)
            with self.assertRaises(ValueError):score.score_run(root,judge)
            self.assertEqual(len(calls),1)


if __name__=="__main__":unittest.main()
