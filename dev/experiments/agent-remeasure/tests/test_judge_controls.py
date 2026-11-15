"""Judge selection/cache and retained rejection-backoff controls, without a client."""
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import score
from experiments import mcp_agent_loop as mal
from test_current_controls import make_run


class JudgeControls(unittest.TestCase):
    def test_another_judge_model_cannot_reuse_cached_verdicts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);make_run(root)
            (root/"judged.jsonl").write_text(json.dumps({"key":score.answer_key("q0","a"),"status":"complete",
                                                       "judge_model":"old-judge","lenient":1.0,"strict":1.0}))
            judge=SimpleNamespace(model="new-judge")
            with self.assertRaises(ValueError):score.score_run(root,judge)

    def test_judges_select_strict_inherited_client_and_single_parse(self):
        prompts=SimpleNamespace()
        with tempfile.TemporaryDirectory() as tmp, patch.object(mal,"load_harness_prompts",return_value=prompts),patch.object(mal,"read_env_file",return_value={"R2_JUDGE_BASE_URL":"unused","R2_JUDGE_API_KEY":"unused"}),patch.object(mal,"ChatClient") as client,patch.object(mal,"load_questions",return_value=([],{})),patch.object(mal,"Grader") as grader:
            judge=score.Judges(Path(tmp),cap_usd=0.56,dataset=Path("dataset"),harness=Path("harness"),env_file=Path("env"),model="gpt-4o-mini")
            judge._conv(1)
            self.assertIs(client.call_args.kwargs["retry_unknown"],False)
            self.assertEqual(grader.call_args.kwargs,{"parse_attempts":1,"judge_model":"gpt-4o-mini"})

    def test_strict_client_still_recovers_a_known_429_rejection(self):
        calls=[]
        response=SimpleNamespace(usage=SimpleNamespace(prompt_tokens=1,completion_tokens=1),choices=[SimpleNamespace(message=SimpleNamespace(content="{}",tool_calls=[]),finish_reason="stop")])
        raw=SimpleNamespace(parse=lambda:response,headers={"x-litellm-model-name":"openai/gpt-4o-mini"})
        def create(**kwargs):
            calls.append(kwargs)
            if len(calls)==1:
                exc=RuntimeError("rate limited");exc.status_code=429;raise exc
            return raw
        client=object.__new__(mal.ChatClient)
        client.retry_unknown=False;client.served={};client.arm="judge"
        client.ledger=SimpleNamespace(check=lambda:None,record=lambda **kw:0)
        client._client=SimpleNamespace(chat=SimpleNamespace(completions=SimpleNamespace(with_raw_response=SimpleNamespace(create=create))))
        backoff=mal.with_backoff
        with patch.object(mal,"with_backoff",side_effect=lambda call,**kw:backoff(call,sleep=lambda _:None,**kw)):
            self.assertEqual(client.complete(qid="q0",role="judge",model="gpt-4o-mini",messages=[]).content,"{}")
        self.assertEqual(len(calls),2)


if __name__=="__main__":unittest.main()
