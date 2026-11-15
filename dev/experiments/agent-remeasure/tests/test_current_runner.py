"""Current native measurement and no-unknown-retry contracts; no live clients."""
import json
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


class CurrentRunnerTests(unittest.TestCase):
    def test_explicit_native_formats_and_direct_server(self):
        for fmt in ("text", "structured"):
            self.assertEqual(runner.server_config(Path("/stores"), "conv0", fmt).get("result_format"), fmt)
            cfg = runner.mcp_config(Path("/binary"), Path("/config"), Path("/stderr"), fmt)
            self.assertNotIn("compact_proxy", json.dumps(cfg))
        self.assertNotIn("Snippets show only the start of each record", runner.PROMPT)
        self.assertEqual(runner.PROMPT.count("Snippets are centred on the match"), 1)

    def test_cli_bounds_and_exact_model(self):
        cmd = runner.claude_command("prompt", "owner-selected-model", Path("cfg"), max_turns=8, cap_usd="0.02")
        self.assertEqual(cmd[cmd.index("--model") + 1], "owner-selected-model")
        self.assertIn("--max-turns", cmd)
        self.assertEqual(cmd[cmd.index("--max-turns") + 1], "8")
        self.assertEqual(cmd[cmd.index("--max-budget-usd") + 1], "0.02")
        self.assertNotIn("--fallback-model", cmd)

    def test_reservation_survives_unknown_failure_and_resume_never_retries(self):
        with tempfile.TemporaryDirectory() as tmp:
            checkpoint = Path(tmp) / "checkpoint.jsonl"
            calls = []
            def unknown(q):
                calls.append(q.qid)
                raise TimeoutError("unknown billed state")
            q = SimpleNamespace(qid="conv1_q0")
            with self.assertRaises(TimeoutError):
                runner.execute_questions([q], unknown, checkpoint, cap_usd="0.02", episode_cap_usd="0.02", resume=False)
            self.assertTrue(checkpoint.exists(), "reservation must precede the billable child")
            self.assertEqual(json.loads(checkpoint.read_text())["status"], "started")
            with self.assertRaises(ValueError):
                runner.execute_questions([q], unknown, checkpoint, cap_usd="0.02", episode_cap_usd="0.02", resume=True)
            self.assertEqual(calls, ["conv1_q0"])

    def test_reservation_stops_before_next_episode_and_success_resumes(self):
        with tempfile.TemporaryDirectory() as tmp:
            checkpoint = Path(tmp) / "checkpoint.jsonl"
            qs = [SimpleNamespace(qid=f"conv1_q{i}") for i in range(2)]
            calls = []
            def process(q):
                calls.append(q.qid)
                return {"qid": q.qid.split("_")[1], "valid": True, "total_cost_usd": 0.02}
            with self.assertRaises(ValueError):
                runner.execute_questions(qs, process, checkpoint, cap_usd="0.02", episode_cap_usd="0.02", resume=False)
            self.assertEqual(calls, ["conv1_q0"])
            runner.execute_questions(qs, process, checkpoint, cap_usd="0.04", episode_cap_usd="0.02", resume=True)
            self.assertEqual(calls, ["conv1_q0", "conv1_q1"])

    def test_missing_cost_and_invalid_episode_halt_before_next(self):
        for result in ({"valid": True, "total_cost_usd": None}, {"valid": False, "total_cost_usd": 0.001}):
            with self.subTest(result=result), tempfile.TemporaryDirectory() as tmp:
                calls = []
                def process(q):
                    calls.append(q.qid)
                    return {"qid": q.qid, **result}
                with self.assertRaises(ValueError):
                    runner.execute_questions([SimpleNamespace(qid="q0"), SimpleNamespace(qid="q1")], process,
                                             Path(tmp) / "checkpoint.jsonl", cap_usd="1", episode_cap_usd="0.02", resume=False)
                self.assertEqual(calls, ["q0"])

    def test_native_quoted_refs_cut_and_hits_not_legacy_only(self):
        ref = 'l:quote " space | newline\n'
        text = 'search store="s" hits=2 completeness=complete\n' + json.dumps(ref) + ' arm=lexical score=1.000 cut | cut in snippet\n"l:uncut" arm=lexical score=0.500 | cut\n'
        self.assertEqual(score.cut_refs_in_result(text), ([ref, "l:uncut"], [ref]))
        audit = score.audit_transcript([json.dumps({"type":"assistant","message":{"content":[{"type":"tool_use","id":"x","name":"mcp__fathomdb__search"}]}}),
                                        json.dumps({"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"x","content":text}]}})])
        self.assertEqual(audit["search_hits_per_page"], [2])

    def test_incomplete_and_invalid_pairs_refuse_instead_of_intersection(self):
        def row(run, qid):
            return {"run":run,"qid":qid,"item":"4","model":"m","arm":run,"exit":0,"answer_parsed":True,
                    "invalid":False,"answer_unparsed":False,"tool_errors":0,"busy_results":0,"result_subtype":"success", "searches":1,
                    "lenient":1.0,"strict":1.0,"cut_refs_shown":0,"cut_refs_fetched":0,"wall_s":1,
                    "instructions_seen":"yes","num_turns":2,"cost_usd":0.01,"tool_calls":{"search":1}}
        a0, a1, b0 = row("text","q0"), row("text","q1"), row("structured","q0")
        with self.assertRaises(ValueError):
            score.summarize([a0,a1,b0])
        for key,value in (("exit",1),("tool_errors",1),("busy_results",1),("answer_parsed",False),("lenient",None),("result_subtype",None),("searches",0)):
            with self.subTest(key=key), self.assertRaises(ValueError):
                score.summarize([{**a0,key:value},b0])
        with self.assertRaises(ValueError):
            score.summarize([a0,b0], expected_by_run={"text":["q0","q1"],"structured":["q0","q1"]})
        self.assertEqual(score.summarize([a0,b0])["paired"][0]["n"], 1)

    def test_actual_inherited_client_does_not_retry_unknown_billed_errors(self):
        for exc in (TimeoutError("unknown"), SimpleApiError(500)):
            with self.subTest(exc=type(exc)):
                calls=[]
                def create(**kwargs):
                    calls.append(kwargs)
                    raise exc
                client=object.__new__(mal.ChatClient)
                client.retry_unknown=False
                client.ledger=SimpleNamespace(check=lambda:None)
                client._client=SimpleNamespace(chat=SimpleNamespace(completions=SimpleNamespace(with_raw_response=SimpleNamespace(create=create))))
                backoff = mal.with_backoff
                with patch.object(mal, "with_backoff", side_effect=lambda call, **kw: backoff(call, sleep=lambda _: None, **kw)), self.assertRaises(type(exc)):
                    client.complete(qid="q0",role="judge",model="gpt-4o-mini",messages=[])
                self.assertEqual(len(calls),1)

    def test_actual_inherited_grader_does_not_repeat_malformed_billed_answer(self):
        calls=[]
        def complete(**kwargs):
            calls.append(kwargs)
            return mal.ModelTurn(content="malformed",tool_calls=[],usage=mal.Usage(1,0,1,0))
        grader=object.__new__(mal.Grader)
        grader.parse_attempts=1
        grader.judge_model="gpt-4o-mini"
        grader.lookup={}
        grader.client=SimpleNamespace(complete=complete)
        grader.prompts=SimpleNamespace(preprocess_answer=lambda *a:"gold",get_judge_prompt=lambda *a:"prompt",JUDGE_SYSTEM_PROMPT="system")
        question=mal.Question("q0",1,"question","gold",[])
        with patch.object(mal.time,"sleep",return_value=None), self.assertRaises(ValueError):
            grader.judge(question,"answer",[])
        self.assertEqual(len(calls),1)

    def test_pending_judge_record_blocks_repeated_charged_grading(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            (root/"run.json").write_text(json.dumps({"conv":1,"item":"5","arm":"text","model":"m","questions":["conv1_q0"]}))
            (root/"checkpoint.jsonl").write_text(json.dumps({"qid":"q0","status":"complete","valid":True,"answer_json":{"answer":"a"},"answer":"a","exit":0,"wall_s":1}))
            (root/"q").mkdir()
            transcript=[{"type":"assistant","message":{"content":[{"type":"tool_use","id":"s","name":"mcp__fathomdb__search","input":{}}]}},
                        {"type":"result","subtype":"success","total_cost_usd":0.01,"num_turns":2,"result":"answer"}]
            (root/"q/q0.stream.jsonl").write_text("\n".join(map(json.dumps,transcript)))
            (root/"judged.jsonl").write_text(json.dumps({"key":score.answer_key("q0","a"),"status":"started"}))
            judges=SimpleNamespace(judge=lambda *a: self.fail("must not rejudge unknown spend"))
            with self.assertRaises(ValueError): score.score_run(root,judges)


class SimpleApiError(Exception):
    def __init__(self,status): self.status_code=status


if __name__ == "__main__": unittest.main()
