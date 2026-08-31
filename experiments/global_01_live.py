"""Paid, checkpointed GLOBAL-01 native witness and matched comparison."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import re
import shutil
import socket
import subprocess
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass, field
from pathlib import Path
from statistics import fmean
from typing import Any, Mapping, Sequence

import pyarrow.parquet as pq

from experiments import global_01


CHECKPOINT_SCHEMA = "global-01.checkpoint.v1"
RESULT_SCHEMA = "global-01.result.v1"
REQUIRED_GRAPH_OUTPUTS = (
    "community_reports.parquet",
    "communities.parquet",
    "documents.parquet",
    "entities.parquet",
    "relationships.parquet",
    "text_units.parquet",
)
METRICS = ("comprehensiveness", "diversity", "empowerment", "directness")
HEADLINE_METRICS = METRICS[:3]


class Global01LiveError(RuntimeError):
    """Raised when the paid GLOBAL-01 run cannot preserve its contract."""


def _canonical_sha256(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def retry_delay(headers: Mapping[str, str], *, fallback: float) -> float:
    """Prefer a numeric provider/Airlock Retry-After over exponential fallback."""
    value = headers.get("Retry-After")
    if value is not None:
        try:
            return max(1.0, float(value))
        except ValueError:
            pass
    return max(1.0, fallback)


@dataclass
class RunState:
    """Incrementally persisted cells and spend for idempotent resume."""

    schema_version: str
    config_sha256: str
    cost_cap_usd: float
    cost_usd: float = 0.0
    cells: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def new(cls, config_sha256: str, cost_cap_usd: float) -> RunState:
        return cls(CHECKPOINT_SCHEMA, config_sha256, cost_cap_usd)

    @classmethod
    def load(
        cls, path: Path, config_sha256: str, cost_cap_usd: float
    ) -> RunState:
        value = json.loads(path.read_text(encoding="utf-8"))
        if (
            value.get("schema_version") != CHECKPOINT_SCHEMA
            or value.get("config_sha256") != config_sha256
            or value.get("cost_cap_usd") != cost_cap_usd
        ):
            raise Global01LiveError("checkpoint configuration or cost cap drifted")
        return cls(**value)

    def complete(self, cell: str, value: object, *, cost_usd: float = 0.0) -> None:
        if cell in self.cells:
            raise Global01LiveError(f"cell already completed: {cell}")
        next_cost = self.cost_usd + cost_usd
        if next_cost > self.cost_cap_usd + 1e-9:
            raise Global01LiveError("GLOBAL-01 cost cap would be exceeded")
        self.cost_usd = next_cost
        self.cells[cell] = value

    def missing(self, cells: Sequence[str]) -> list[str]:
        return [cell for cell in cells if cell not in self.cells]

    def save(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(path.suffix + ".tmp")
        temporary.write_text(json.dumps(asdict(self), indent=2) + "\n", encoding="utf-8")
        temporary.chmod(0o600)
        os.replace(temporary, path)


def assert_native_witness(output_root: Path, answers: Sequence[str]) -> None:
    """Require complete native tables and two non-empty witness answers."""
    if any(
        not (output_root / name).is_file() or (output_root / name).stat().st_size == 0
        for name in REQUIRED_GRAPH_OUTPUTS
    ):
        raise Global01LiveError("native GraphRAG output is missing or empty")
    if len(answers) != 2 or any(not answer.strip() for answer in answers):
        raise Global01LiveError("native GraphRAG witness answers are incomplete")
    communities = pq.read_table(
        output_root / "communities.parquet", columns=["community"]
    ).column("community").to_pylist()
    reports = pq.read_table(
        output_root / "community_reports.parquet", columns=["community"]
    ).column("community").to_pylist()
    if not communities or len(reports) != len(communities) or set(reports) != set(communities):
        raise Global01LiveError("native GraphRAG community reports are incomplete")


def _usage_cost(config: Mapping[str, Any], model: str, usage: Mapping[str, Any]) -> float:
    pricing = config["pricing"][model]
    prompt = int(usage.get("prompt_tokens", 0))
    completion = int(usage.get("completion_tokens", 0))
    return prompt * pricing["input_per_million"] / 1_000_000 + completion * pricing[
        "output_per_million"
    ] / 1_000_000


class AirlockClient:
    """OpenAI-compatible caller with checkpoint-friendly bounded retries."""

    def __init__(self, base_url: str, key: str, config: Mapping[str, Any]) -> None:
        self.url = f"{base_url.rstrip('/')}/v1/chat/completions"
        self.key = key
        self.config = config

    def complete(
        self,
        model: str,
        prompt: str,
        *,
        max_tokens: int,
        temperature: float,
    ) -> tuple[str, dict[str, int], float]:
        body = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
            "temperature": temperature,
        }
        if model == self.config["graphrag"]["answer_model"]:
            body["thinking"] = {"type": self.config["graphrag"]["thinking_mode"]}
        for attempt in range(6):
            request = urllib.request.Request(
                self.url,
                data=json.dumps(body).encode("utf-8"),
                headers={
                    "Authorization": f"Bearer {self.key}",
                    "Content-Type": "application/json",
                },
            )
            try:
                with urllib.request.urlopen(request, timeout=330) as response:
                    payload = json.load(response)
                content = payload["choices"][0]["message"]["content"] or ""
                usage = {
                    "prompt_tokens": int(payload.get("usage", {}).get("prompt_tokens", 0)),
                    "completion_tokens": int(
                        payload.get("usage", {}).get("completion_tokens", 0)
                    ),
                }
                if not content.strip():
                    raise Global01LiveError(f"{model} returned an empty completion")
                return content, usage, _usage_cost(self.config, model, usage)
            except urllib.error.HTTPError as exc:
                if exc.code != 429 and exc.code < 500:
                    raise Global01LiveError(f"Airlock returned HTTP {exc.code}") from exc
                if attempt == 5:
                    raise Global01LiveError("Airlock retry budget exhausted") from exc
                time.sleep(retry_delay(exc.headers, fallback=min(60.0, 2.0**attempt)))
            except (TimeoutError, urllib.error.URLError) as exc:
                if attempt == 5:
                    raise Global01LiveError("Airlock timeout retry budget exhausted") from exc
                time.sleep(min(60.0, 2.0**attempt))
        raise AssertionError("unreachable")


def _last_usage(log_path: Path) -> dict[str, int]:
    text = log_path.read_text(encoding="utf-8")
    prompts = re.findall(r'"prompt_tokens":\s*(\d+)', text)
    completions = re.findall(r'"completion_tokens":\s*(\d+)', text)
    if not prompts or not completions:
        raise Global01LiveError(f"usage metrics missing from {log_path.name}")
    return {
        "prompt_tokens": int(prompts[-1]),
        "completion_tokens": int(completions[-1]),
    }


def _wait_for_port(port: int) -> None:
    for _ in range(50):
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                return
        except OSError:
            time.sleep(0.1)
    raise Global01LiveError(f"local embedding shim did not bind port {port}")


def _prepare_workspace(repository_root: Path, artifact_root: Path, config: Mapping[str, Any]) -> Path:
    workspace = artifact_root / "graphrag"
    if workspace.exists():
        return workspace
    workspace.mkdir(parents=True)
    source = repository_root / config["corpus"]["preserved_input_root"]
    shutil.copytree(source, workspace / "input")
    _render_graph_settings(
        repository_root / config["graphrag"]["settings"],
        workspace / "settings.yaml",
        answer_model=config["graphrag"]["answer_model"],
        thinking_mode=config["graphrag"]["thinking_mode"],
    )
    shutil.copytree(
        repository_root / config["graphrag"]["prompts"], workspace / "prompts"
    )
    _harden_community_prompt(workspace / "prompts/community_report_graph.txt")
    shutil.copy2(
        repository_root / config["graphrag"]["embedding_shim"], workspace / "embshim.py"
    )
    return workspace


def _harden_community_prompt(path: Path) -> None:
    marker = "Do not use Markdown code fences"
    text = path.read_text(encoding="utf-8")
    if marker in text:
        return
    instruction = (
        "Return one valid JSON object only. Do not use Markdown code fences or any "
        "text before or after the JSON object."
    )
    path.write_text(f"{text.rstrip()}\n\n{instruction}\n", encoding="utf-8")


def _render_graph_settings(
    source: Path,
    target: Path,
    *,
    answer_model: str,
    thinking_mode: str,
) -> None:
    text = source.read_text(encoding="utf-8")
    text, count = re.subn(
        r"(?m)^(    model:) [^\n]+$", rf"\1 {answer_model}", text, count=1
    )
    if count != 1:
        raise Global01LiveError("GraphRAG completion model binding is missing")
    call_args = (
        "    call_args:\n"
        '      additional_drop_params: ["response_format"]\n'
        "      extra_body:\n"
        "        thinking:\n"
        f"          type: {thinking_mode}"
    )
    text, count = re.subn(r"(?m)^    call_args: \{\}$", call_args, text, count=1)
    if count == 0:
        model_line = f"    model: {answer_model}"
        text = text.replace(model_line, f"{model_line}\n{call_args}", 1)
    target.write_text(text, encoding="utf-8")


def _run_command(command: list[str], env: Mapping[str, str], output_path: Path, timeout: int) -> str:
    result = subprocess.run(
        command,
        env=dict(env),
        text=True,
        capture_output=True,
        timeout=timeout,
        check=False,
    )
    output = result.stdout + "\n--- stderr ---\n" + result.stderr
    output_path.write_text(output, encoding="utf-8")
    output_path.chmod(0o600)
    if result.returncode != 0:
        raise Global01LiveError(
            f"native GraphRAG command failed with exit {result.returncode}; see {output_path}"
        )
    return result.stdout


def _graph_answer(stdout: str) -> str:
    marker = "Global Search Response:"
    answer = stdout.split(marker, 1)[1].strip() if marker in stdout else stdout.strip()
    if not answer:
        raise Global01LiveError("native GraphRAG returned an empty answer")
    return answer[:12000]


def _judge_prompt(question: str, answer_a: str, answer_b: str) -> str:
    return (
        "You are an impartial judge comparing two answers to a global-sensemaking "
        "question. Score each metric independently as A, B, or tie. "
        "Comprehensiveness is scope coverage; diversity is varied relevant detail; "
        "empowerment is support for informed reasoning; directness rewards concise, "
        "on-point answers and must not reward verbosity.\n\n"
        f"Question:\n{question}\n\nAnswer A:\n{answer_a}\n\nAnswer B:\n{answer_b}\n\n"
        'Return only JSON with keys "comprehensiveness", "diversity", '
        '"empowerment", and "directness", each set to "A", "B", or "tie".'
    )


def _parse_judgment(text: str) -> dict[str, str]:
    match = re.search(r"\{[^{}]+\}", text, re.DOTALL)
    if not match:
        raise Global01LiveError("judge response is not JSON")
    value = json.loads(match.group(0))
    if set(value) != set(METRICS) or any(value[key] not in {"A", "B", "tie"} for key in METRICS):
        raise Global01LiveError("judge response is incomplete")
    return value


def _complete_judgment(
    client: AirlockClient,
    prompt: str,
    *,
    state: RunState,
    checkpoint_path: Path,
    cell: str,
) -> None:
    retry_prompt = prompt
    for attempt in range(3):
        invalid_cell = f"invalid-{cell}/{attempt}"
        if invalid_cell in state.cells:
            retry_prompt = (
                f"{prompt}\n\nReturn exactly the four requested JSON keys with no "
                "additional keys, prose, or Markdown."
            )
            continue
        response, usage, cost = client.complete(
            "claude-haiku", retry_prompt, max_tokens=400, temperature=0.7
        )
        try:
            verdicts = _parse_judgment(response)
        except (Global01LiveError, json.JSONDecodeError):
            state.complete(
                invalid_cell,
                {
                    "usage": usage,
                    "response_sha256": hashlib.sha256(response.encode("utf-8")).hexdigest(),
                },
                cost_usd=cost,
            )
            state.save(checkpoint_path)
            retry_prompt = (
                f"{prompt}\n\nReturn exactly the four requested JSON keys with no "
                "additional keys, prose, or Markdown."
            )
            continue
        state.complete(cell, {"verdicts": verdicts, "usage": usage}, cost_usd=cost)
        state.save(checkpoint_path)
        return
    raise Global01LiveError("judge semantic retry budget exhausted")


def _bootstrap(values_by_question: Mapping[str, list[float]], seed: int = 20260829) -> tuple[float, float]:
    question_ids = sorted(values_by_question)
    generator = random.Random(seed)
    samples = []
    for _ in range(2000):
        drawn = [generator.choice(question_ids) for _ in question_ids]
        samples.append(fmean(value for qid in drawn for value in values_by_question[qid]))
    samples.sort()
    return samples[int(0.025 * len(samples))], samples[int(0.975 * len(samples))]


def _summarize(state: RunState, questions: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    metrics: dict[str, Any] = {}
    for metric in METRICS:
        by_question: dict[str, list[float]] = {}
        for question in questions:
            qid = question["question_id"]
            values = []
            for repetition in range(5):
                for order in ("fg", "gf"):
                    verdict = state.cells[f"judge/{qid}/{repetition}/{order}"]["verdicts"][metric]
                    if verdict == "tie":
                        values.append(0.5)
                    elif (order == "fg" and verdict == "A") or (
                        order == "gf" and verdict == "B"
                    ):
                        values.append(1.0)
                    else:
                        values.append(0.0)
            by_question[qid] = values
        all_values = [value for values in by_question.values() for value in values]
        lo, hi = _bootstrap(by_question)
        metrics[metric] = {
            "fathomdb_win_rate": fmean(all_values),
            "clustered_bootstrap_ci95": [lo, hi],
            "judgments": len(all_values),
        }
    lower = [metrics[name]["clustered_bootstrap_ci95"][0] for name in HEADLINE_METRICS]
    verdict = "descriptive_win" if min(lower) > 0.5 else "near_parity" if min(lower) >= 0.45 else "split"
    return {"verdict": verdict, "metrics": metrics}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--repository-root", type=Path, default=Path.cwd())
    parser.add_argument("--airlock-url", default="http://127.0.0.1:4000")
    args = parser.parse_args()

    config = global_01.validate_config(json.loads(args.config.read_text(encoding="utf-8")))
    global_01.assert_execution_authorized(config)
    config_hash = _canonical_sha256(config)
    cap = float(config["approval"]["cost_cap_usd"])
    checkpoint_path = args.artifact_root / "checkpoint.json"
    if checkpoint_path.is_file():
        state = RunState.load(checkpoint_path, config_hash, cap)
    else:
        args.artifact_root.mkdir(parents=True, exist_ok=False, mode=0o700)
        state = RunState.new(config_hash, cap)
        state.save(checkpoint_path)

    _, private_manifest = global_01.inspect_inputs(config, repository_root=args.repository_root)
    documents, _, _ = global_01._load_documents(config, args.repository_root)
    questions = private_manifest["questions"]
    graph_python = (
        args.repository_root
        / "data/performance-benchmarking/global-01/toolchains/graphrag-3.1.0-venv/bin/python"
    )
    workspace = _prepare_workspace(args.repository_root, args.artifact_root, config)
    key = os.environ.get("AIRLOCK_VIRTUAL_KEY")
    if not key:
        raise Global01LiveError("AIRLOCK_VIRTUAL_KEY is required")
    env = dict(os.environ, GRAPHRAG_API_KEY=key)
    client = AirlockClient(args.airlock_url, key, config)
    answer_model = config["graphrag"]["answer_model"]

    shim = subprocess.Popen(
        [str(graph_python), str(workspace / "embshim.py")],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        _wait_for_port(8090)
        if "graphrag/index" not in state.cells:
            if state.cost_usd + 2.25 > cap:
                raise Global01LiveError("insufficient remaining cost cap for GraphRAG indexing")
            _run_command(
                [str(graph_python), "-m", "graphrag", "index", "--root", str(workspace)],
                env,
                args.artifact_root / "graphrag-index.out",
                2400,
            )
            usage = _last_usage(workspace / "logs/indexing-engine.log")
            state.complete(
                "graphrag/index",
                {"usage": usage, "outputs": list(REQUIRED_GRAPH_OUTPUTS)},
                cost_usd=_usage_cost(config, answer_model, usage),
            )
            state.save(checkpoint_path)

        for ordinal, question in enumerate(questions):
            cell = f"answers/graphrag/{question['question_id']}"
            if cell in state.cells:
                continue
            stdout = _run_command(
                [
                    str(graph_python),
                    "-m",
                    "graphrag",
                    "query",
                    "--root",
                    str(workspace),
                    "--method",
                    "global",
                    "--community-level",
                    "1",
                    "--no-dynamic-selection",
                    question["text"],
                ],
                env,
                args.artifact_root / f"graphrag-query-{ordinal:02d}.out",
                600,
            )
            usage = _last_usage(workspace / "logs/query.log")
            state.complete(
                cell,
                {"answer": _graph_answer(stdout), "usage": usage},
                cost_usd=_usage_cost(config, answer_model, usage),
            )
            state.save(checkpoint_path)
            if ordinal == 1:
                witness_answers = [
                    state.cells[f"answers/graphrag/{item['question_id']}"]["answer"]
                    for item in questions[:2]
                ]
                assert_native_witness(workspace / "output", witness_answers)
                state.complete("graphrag/witness", {"state": "valid"})
                state.save(checkpoint_path)
    finally:
        shim.terminate()
        try:
            shim.wait(timeout=5)
        except subprocess.TimeoutExpired:
            shim.kill()

    bodies = [document["text"] for document in documents]
    for question in questions:
        qid = question["question_id"]
        partials = []
        for batch_index, start in enumerate(range(0, len(bodies), 5)):
            cell = f"answers/fathomdb/{qid}/map/{batch_index}"
            if cell not in state.cells:
                context = "\n\n".join(
                    f"[{start + offset + 1}] {body}"
                    for offset, body in enumerate(bodies[start : start + 5])
                )
                prompt = (
                    "Extract source-grounded points relevant to the question. Return NONE "
                    "if the batch has no relevant evidence.\n\n"
                    f"Question: {question['text']}\n\nDocuments:\n{context}\n\nPoints:"
                )
                answer, usage, cost = client.complete(
                    answer_model, prompt, max_tokens=300, temperature=0.0
                )
                state.complete(cell, {"answer": answer, "usage": usage}, cost_usd=cost)
                state.save(checkpoint_path)
            value = state.cells[cell]["answer"]
            if value.strip().upper() != "NONE":
                partials.append(value)
        reduce_cell = f"answers/fathomdb/{qid}/reduce"
        if reduce_cell not in state.cells:
            prompt = (
                "Synthesize a comprehensive, source-grounded global answer from the "
                "extracted points. Do not add unsupported facts.\n\n"
                f"Question: {question['text']}\n\nPoints:\n"
                + "\n\n".join(partials)
                + "\n\nAnswer:"
            )
            answer, usage, cost = client.complete(
                answer_model, prompt, max_tokens=1500, temperature=0.0
            )
            state.complete(reduce_cell, {"answer": answer, "usage": usage}, cost_usd=cost)
            state.save(checkpoint_path)

    for question in questions:
        qid = question["question_id"]
        fathom_answer = state.cells[f"answers/fathomdb/{qid}/reduce"]["answer"]
        graph_answer = state.cells[f"answers/graphrag/{qid}"]["answer"]
        for repetition in range(5):
            for order in ("fg", "gf"):
                cell = f"judge/{qid}/{repetition}/{order}"
                if cell in state.cells:
                    continue
                answer_a, answer_b = (
                    (fathom_answer, graph_answer)
                    if order == "fg"
                    else (graph_answer, fathom_answer)
                )
                prompt = _judge_prompt(question["text"], answer_a, answer_b)
                _complete_judgment(
                    client,
                    prompt,
                    state=state,
                    checkpoint_path=checkpoint_path,
                    cell=cell,
                )

    summary = _summarize(state, questions)
    result = {
        "schema_version": RESULT_SCHEMA,
        "program_track": "GLOBAL-01",
        "config_sha256": config_hash,
        "state": "complete",
        "native_witness": "valid",
        "question_count": len(questions),
        "judge_repetitions": 5,
        "order_swapped": True,
        "cost_usd": state.cost_usd,
        "cost_cap_usd": cap,
        **summary,
        "answer_manifest_sha256": _canonical_sha256(
            {
                arm: [
                    hashlib.sha256(
                        state.cells[
                            f"answers/{arm}/{question['question_id']}"
                            + ("/reduce" if arm == "fathomdb" else "")
                        ]["answer"].encode("utf-8")
                    ).hexdigest()
                    for question in questions
                ]
                for arm in ("graphrag", "fathomdb")
            }
        ),
    }
    result_path = args.artifact_root / "result.json"
    result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    result_path.chmod(0o600)
    print(json.dumps({"state": "complete", "verdict": result["verdict"], "cost_usd": state.cost_usd, "result": str(result_path)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
