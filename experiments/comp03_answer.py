"""COMP-03: Hermes-style LongMemEval-S answer comparison, FathomDB vs LanceDB.

Each selected question is its own isolated memory store, built from identical
history rounds for both arms. Each arm retrieves the top five rounds through
its native FTS. The same evidence renderer, answerer, and blinded judge then
score both arms. One typed config fixes every input, model, and budget; a run
writes resumable per-question artifacts to an external attempt directory and an
``experiments.record.v1`` receipt plus index row.

    python -m experiments.comp03_answer select CONFIG --dataset FILE
    python -m experiments.comp03_answer run CONFIG --dataset FILE --attempt-root DIR
        [--fathomdb-bin PATH] [--max-questions N] [--sweep-summary FILE]

Config ``comp-03.answer.v2`` adds arms with a ``product_arm`` (the
``comp_fts.ARMS`` key) and arms gated on the COMP-03 retrieval sweep's
verdict, which ``--sweep-summary`` supplies.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import random
import shutil
import statistics
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Mapping, Sequence

from experiments import _lib
from experiments import comp_fts

DEFAULT_CONFIG = _lib.EXPERIMENTS_DIR / "configs" / "comp-03" / "hermes-answer.v1.json"
EXPERIMENT = "comp-03-hermes-answer"
QUESTION_TYPES = (
    "knowledge-update", "multi-session", "single-session-assistant",
    "single-session-preference", "single-session-user", "temporal-reasoning",
)
REPEATS = 5
CONFIG_KEYS = {
    "schema_version", "program_track", "campaign", "corpus", "selection", "document_unit",
    "arms", "retrieval", "evidence", "live", "metrics", "claim_boundary",
}


class Comp03Error(RuntimeError):
    pass


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _digest(value: Any) -> str:
    return hashlib.sha256(_lib.canonical_json(value).encode("utf-8")).hexdigest()


V2_ARMS = ["fathomdb-fts", "fathomdb-hybrid", "lancedb-fts"]


def load_config(path: Path) -> dict[str, Any]:
    config = json.loads(Path(path).read_text(encoding="utf-8"))
    if set(config) != CONFIG_KEYS:
        raise Comp03Error(f"config keys mismatch: {sorted(set(config) ^ CONFIG_KEYS)}")
    if config["schema_version"] == "comp-03.answer.v1":
        if [arm["id"] for arm in config["arms"]] != ["fathomdb", "lancedb"]:
            raise Comp03Error("COMP-03 arms are fathomdb then lancedb")
    elif config["schema_version"] == "comp-03.answer.v2":
        if [arm["id"] for arm in config["arms"]] != V2_ARMS:
            raise Comp03Error(f"COMP-03 v2 arms are {', '.join(V2_ARMS)}")
        if any(product_arm(arm) not in comp_fts.ARMS for arm in config["arms"]):
            raise Comp03Error("every v2 arm names a comp_fts product arm")
    else:
        raise Comp03Error("unsupported config schema_version")
    return config


def product_arm(arm: Mapping[str, Any]) -> str:
    """The ``comp_fts.ARMS`` key behind a config arm; v1 arm IDs are product names."""
    return arm.get("product_arm", arm["id"])


def arm_order(index: int, arms: Sequence[str]) -> list[str]:
    """Rotate ``arms`` by ``index``: v1's alternation for two arms, a Latin cycle for more."""
    shift = index % len(arms)
    return list(arms[shift:]) + list(arms[:shift])


def active_arms(config: Mapping[str, Any], sweep_summary: Path | None) -> tuple[list[Mapping[str, Any]], dict[str, Any]]:
    """Config arms that run, and the gate record. A gated arm needs a matching sweep verdict."""
    verdict, digest = None, None
    if sweep_summary is not None:
        verdict = json.loads(Path(sweep_summary).read_text(encoding="utf-8"))["decision"]["verdict"]
        digest = sha256_file(Path(sweep_summary))
    running = [arm for arm in config["arms"]
               if "requires_sweep_verdict" not in arm or arm["requires_sweep_verdict"] == verdict]
    excluded = [arm["id"] for arm in config["arms"] if arm not in running]
    return running, {"sweep_verdict": verdict, "sweep_summary_sha256": digest, "excluded_arms": excluded}


# --- inputs ----------------------------------------------------------------


def select_questions(dataset: Sequence[Mapping[str, Any]], *, per_type: int, seed: int) -> list[str]:
    """``per_type`` seeded IDs from each question type, sorted within type."""
    picked: list[str] = []
    for qtype in sorted(QUESTION_TYPES):
        ids = sorted(item["question_id"] for item in dataset if item["question_type"] == qtype)
        if len(ids) < per_type:
            raise Comp03Error(f"{qtype} has {len(ids)} questions, needs {per_type}")
        picked.extend(sorted(random.Random(f"{seed}:{qtype}").sample(ids, per_type)))
    return picked


def _rounds(session: Sequence[Mapping[str, Any]]) -> list[list[Mapping[str, Any]]]:
    rounds: list[list[Mapping[str, Any]]] = []
    for turn in session:
        if turn["role"] == "user" or not rounds:
            rounds.append([turn])
        else:
            rounds[-1].append(turn)
    return rounds


def convert_question(item: Mapping[str, Any]) -> dict[str, Any]:
    """Indexable history and query contract, built only from whitelisted fields.

    Session IDs are replaced by position because LongMemEval names gold
    sessions ``answer_*``; turn ``has_answer`` flags and answers are never read.
    """
    docs = []
    for s_index, (date, session) in enumerate(zip(item["haystack_dates"], item["haystack_sessions"], strict=True)):
        for r_index, turns in enumerate(_rounds(session)):
            docs.append({
                "id": f"s{s_index:03d}-r{r_index:03d}",
                "date": date,
                "roles": [turn["role"] for turn in turns],
                "body": "\n".join(turn["content"] for turn in turns),
            })
    return {
        "question_id": item["question_id"],
        "question_type": item["question_type"],
        "question": item["question"],
        "question_date": item["question_date"],
        "docs": docs,
    }


def gold_for(item: Mapping[str, Any]) -> dict[str, Any]:
    """Grading-only gold: the reference answer and relevant round IDs."""
    answer_sessions = set(item["answer_session_ids"])
    session_relevant, turn_relevant = [], []
    for s_index, (sid, session) in enumerate(zip(item["haystack_session_ids"], item["haystack_sessions"], strict=True)):
        for r_index, turns in enumerate(_rounds(session)):
            doc_id = f"s{s_index:03d}-r{r_index:03d}"
            if sid in answer_sessions:
                session_relevant.append(doc_id)
            if any(turn.get("has_answer") for turn in turns):
                turn_relevant.append(doc_id)
    return {
        "answer": str(item["answer"]),
        "abstention": item["question_id"].endswith("_abs"),
        "session_relevant": session_relevant,
        "turn_relevant": turn_relevant,
    }


# --- evidence and prompts ----------------------------------------------------


def render_evidence(docs: Sequence[Mapping[str, Any]], ids: Sequence[str], *,
                    max_item_chars: int, max_total_chars: int) -> str:
    """Ordered evidence text: per-item cap first, then a total cap."""
    by_id = {doc["id"]: doc for doc in docs}
    remaining = max_total_chars
    blocks = []
    for doc_id in ids:
        if remaining <= 0:
            break
        doc = by_id[doc_id]
        text = doc["body"][:max_item_chars][:remaining]
        remaining -= len(text)
        blocks.append(f"[Session date: {doc['date']}]\n{text}")
    return "\n\n".join(blocks)


def answer_messages(case: Mapping[str, Any], evidence: str) -> list[dict[str, str]]:
    prompt = (
        "I will give you several history chats between you and a user. Please answer the question "
        "based on the relevant chat history.\n\n"
        f"History Chats:\n\n{evidence}\n\n"
        f"Current Date: {case['question_date']}\nQuestion: {case['question']}\nAnswer:"
    )
    return [{"role": "user", "content": prompt}]


_JUDGE_BASE = (
    "I will give you a question, a correct answer, and a response from a model. Please answer yes if "
    "the response contains the correct answer. Otherwise, answer no. If the response is equivalent to "
    "the correct answer or contains all the intermediate steps to get the correct answer, you should "
    "also answer yes. If the response only contains a subset of the information required by the "
    "answer, answer no."
)
_JUDGE_EXTRA = {
    "temporal-reasoning": (
        " In addition, do not penalize off-by-one errors for the number of days. If the question asks "
        "for the number of days/weeks/months, etc., and the model makes off-by-one errors (e.g., "
        "predicting 19 days when the answer is 18), the model's response is still correct."
    ),
}


def judge_messages(qtype: str, question: str, answer: str, response: str, *, abstention: bool) -> list[dict[str, str]]:
    """LongMemEval's per-type answer-check prompts (evaluate_qa.py)."""
    if abstention:
        prompt = (
            "I will give you an unanswerable question, an explanation, and a response from a model. "
            "Please answer yes if the model correctly identifies the question as unanswerable. The model "
            "could say that the information is incomplete, or some other information is given but the "
            f"asked information is not.\n\nQuestion: {question}\n\nExplanation: {answer}\n\n"
            f"Model Response: {response}\n\nDoes the model correctly identify the question as "
            "unanswerable? Answer yes or no only."
        )
    elif qtype == "knowledge-update":
        prompt = (
            "I will give you a question, a correct answer, and a response from a model. Please answer yes "
            "if the response contains the correct answer. Otherwise, answer no. If the response contains "
            "some previous information along with an updated answer, the response should be considered "
            "as correct as long as the updated answer is the required answer.\n\n"
            f"Question: {question}\n\nCorrect Answer: {answer}\n\nModel Response: {response}\n\n"
            "Is the model response correct? Answer yes or no only."
        )
    elif qtype == "single-session-preference":
        prompt = (
            "I will give you a question, a rubric for desired personalized response, and a response from "
            "a model. Please answer yes if the response satisfies the desired response. Otherwise, answer "
            "no. The model does not need to reflect all the points in the rubric. The response is correct "
            "as long as it recalls and utilizes the user's personal information correctly.\n\n"
            f"Question: {question}\n\nRubric: {answer}\n\nModel Response: {response}\n\n"
            "Is the model response correct? Answer yes or no only."
        )
    else:
        prompt = (
            _JUDGE_BASE + _JUDGE_EXTRA.get(qtype, "")
            + f" \n\nQuestion: {question}\n\nCorrect Answer: {answer}\n\nModel Response: {response}\n\n"
            "Is the model response correct? Answer yes or no only."
        )
    return [{"role": "user", "content": prompt}]


def parse_verdict(text: str) -> bool:
    return "yes" in text.strip().lower()


# --- metrics -----------------------------------------------------------------


def retrieval_scores(ids: Sequence[str], session_relevant: Sequence[str], turn_relevant: Sequence[str],
                     k: int = 5) -> dict[str, float | None]:
    top = list(ids)[:k]
    out: dict[str, float | None] = {}
    for name, relevant in (("session", set(session_relevant)), ("turn", set(turn_relevant))):
        if not relevant:
            out[f"{name}_recall_any_at_{k}"] = out[f"{name}_mrr_at_{k}"] = None
            continue
        rank = next((i for i, doc_id in enumerate(top, 1) if doc_id in relevant), None)
        out[f"{name}_recall_any_at_{k}"] = 0.0 if rank is None else 1.0
        out[f"{name}_mrr_at_{k}"] = 0.0 if rank is None else 1.0 / rank
    return out


def _wilson(correct: int, n: int) -> list[float]:
    if n == 0:
        return [0.0, 0.0]
    z, p = 1.96, correct / n
    centre = (p + z * z / (2 * n)) / (1 + z * z / n)
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / (1 + z * z / n)
    return [centre - half, centre + half]


def paired_summary(rows: Sequence[Mapping[str, Any]], arms: tuple[str, str], *, seed: int) -> dict[str, Any]:
    """Paired answer accuracy, discordant pairs, and a bootstrap CI on A − B."""
    a, b = arms
    n = len(rows)
    diffs = [int(r[a]) - int(r[b]) for r in rows]
    rng = random.Random(seed)
    boots = sorted(statistics.fmean(rng.choices(diffs, k=n)) for _ in range(10_000)) if n else [0.0]
    per_type: dict[str, Any] = {}
    for qtype in sorted({r["question_type"] for r in rows}):
        members = [r for r in rows if r["question_type"] == qtype]
        per_type[qtype] = {
            arm: {"correct": sum(bool(r[arm]) for r in members), "n": len(members),
                  "rate": sum(bool(r[arm]) for r in members) / len(members),
                  "wilson_95": _wilson(sum(bool(r[arm]) for r in members), len(members))}
            for arm in arms
        }
    return {
        "n": n,
        "accuracy": {arm: (sum(bool(r[arm]) for r in rows) / n if n else 0.0) for arm in arms},
        "difference": statistics.fmean(diffs) if n else 0.0,
        "difference_bootstrap_95": [boots[249], boots[9749]] if n else [0.0, 0.0],
        "discordant": {f"{a}_only": diffs.count(1), f"{b}_only": diffs.count(-1)},
        "per_type": per_type,
    }


# --- retrieval -----------------------------------------------------------------


def _cli_identity(fathomdb_bin: str) -> dict[str, Any]:
    resolved = shutil.which(fathomdb_bin) or fathomdb_bin
    return {"fathomdb_bin": fathomdb_bin, "fathomdb_bin_sha256": sha256_file(Path(resolved))}


def _prepared_root(workdir: Path, test_id: str, arm: str, fathomdb_bin: str) -> tuple[Path, dict[str, Any]]:
    from experiments.fathomdb_test_setup import prepare_test_database

    if arm == "fathomdb":
        prepared = prepare_test_database(workdir, test_id=test_id, embed_device="cpu", rerank_device="cpu",
                                         embedder="none", check_reranker=False, fathomdb_bin=fathomdb_bin)
    else:
        prepared = prepare_test_database(workdir, test_id=test_id,
                                         embed_device=os.environ.get("FATHOMDB_EMBED_DEVICE", "auto"),
                                         rerank_device="cpu", embedder="default", warm_cache=True,
                                         check_reranker=False, fathomdb_bin=fathomdb_bin)
    doctor = json.loads(prepared.doctor_path.read_text(encoding="utf-8"))
    setup = {"doctor_sha256": sha256_file(prepared.doctor_path),
             "integrity": doctor["integrity"].get("status", doctor["integrity"]),
             **_cli_identity(fathomdb_bin)}
    if arm == "fathomdb-hybrid":
        # The doctor evidence must describe the same build as the wheel that embeds.
        cli_cuda = doctor["gpu"].get("cuda_compiled")
        wheel_cuda = (doctor["open_report"]["embedder_device_resolution"] or {}).get("cuda_compiled")
        if cli_cuda != wheel_cuda:
            raise Comp03Error(f"fathomdb CLI doctor gpu cuda_compiled={cli_cuda} but the wheel's "
                              f"embedder reports cuda_compiled={wheel_cuda}; use the matching CLI build")
        setup["cuda_compiled"] = wheel_cuda
    return prepared.database_path.parent, setup


def _fresh_root(workdir: Path, test_id: str, arm: str, fathomdb_bin: str | None) -> tuple[Path, dict[str, Any] | None]:
    """A new, empty store root; a failed setup removes the root it created."""
    root = workdir / test_id
    if arm in ("fathomdb", "fathomdb-hybrid") and fathomdb_bin:
        if root.exists():
            raise FileExistsError(f"store root already exists: {root}")
        try:
            return _prepared_root(workdir, test_id, arm, fathomdb_bin)
        except BaseException:
            shutil.rmtree(root, ignore_errors=True)
            raise
    if root.exists():
        raise FileExistsError(f"store root already exists: {root}")
    root.mkdir(parents=True)
    return root, None


def retrieve(arm_name: str, case: Mapping[str, Any], workdir: Path, *, top_k: int,
             fathomdb_bin: str | None, store_label: str | None = None) -> dict[str, Any]:
    """Build one fresh isolated store, query it, record IDs and timings, delete it.

    ``arm_name`` is the ``comp_fts.ARMS`` key; ``store_label`` (default
    ``arm_name``) names the store directory. Whitespace-only rounds are not
    indexed (FathomDB rejects blank bodies, and every arm must index the same
    set); round IDs stay positional and ``case["docs"]`` is left intact.
    """
    arm = comp_fts.ARMS[arm_name]
    indexed = [{"id": d["id"], "body": d["body"]} for d in case["docs"] if d["body"].strip()]
    test_id = f"{case['question_id'].lower().replace('_', '-')}-{store_label or arm_name}"
    root, setup = _fresh_root(workdir, test_id, arm_name, fathomdb_bin)
    try:
        build = arm.build(root, indexed)
        handle = arm.open(root)
        try:
            started = time.perf_counter_ns()
            ids = arm.query(handle, case["question"], limit=top_k)
            latency_ms = (time.perf_counter_ns() - started) / 1e6
            repeats = []
            for _ in range(REPEATS):
                started = time.perf_counter_ns()
                again = arm.query(handle, case["question"], limit=top_k)
                repeats.append((time.perf_counter_ns() - started) / 1e6)
                if again != ids:
                    raise Comp03Error(f"{arm_name} returned unstable results for {case['question_id']}")
        finally:
            arm.close(handle)
        disk = comp_fts._disk_bytes(root)
    finally:
        shutil.rmtree(root, ignore_errors=True)
    return {
        "arm": arm_name, "version": arm.version(), "settings": arm.settings(), "documents": len(indexed),
        "blank_rounds_skipped": len(case["docs"]) - len(indexed),
        "ids": ids, "latency_ms": latency_ms, "repeat_latency_ms": repeats, "disk_bytes": disk,
        "setup": setup, **build,
    }


# --- model client ----------------------------------------------------------------


@dataclass(frozen=True)
class Reply:
    content: str
    prompt_tokens: int
    completion_tokens: int


class AirlockClient:
    """OpenAI-compatible loopback client with bounded retries."""

    def __init__(self, base_url: str, key: str, max_retries: int) -> None:
        if not base_url.startswith(("http://127.0.0.1:", "http://localhost:")):
            raise Comp03Error("Airlock must use a loopback route")
        if not key:
            raise Comp03Error("AIRLOCK_VIRTUAL_KEY or R2_JUDGE_API_KEY is required")
        self.base_url, self.key, self.max_retries = base_url.rstrip("/"), key, max_retries

    def complete(self, model: str, messages: Sequence[Mapping[str, str]], max_tokens: int, *,
                 temperature: float, seed: int) -> Reply:
        payload = {"model": model, "messages": list(messages), "max_tokens": max_tokens,
                   "temperature": temperature, "seed": seed}
        request = urllib.request.Request(
            self.base_url + "/v1/chat/completions", data=json.dumps(payload).encode("utf-8"), method="POST",
            headers={"Authorization": f"Bearer {self.key}", "Content-Type": "application/json"},
        )
        for attempt in range(self.max_retries + 1):
            try:
                with urllib.request.urlopen(request, timeout=300) as response:  # noqa: S310
                    value = json.loads(response.read())
                content = value["choices"][0]["message"]["content"]
                if not isinstance(content, str) or not content.strip():
                    raise Comp03Error("empty completion")
                usage = value.get("usage") or {}
                return Reply(content, int(usage.get("prompt_tokens", 0)), int(usage.get("completion_tokens", 0)))
            except urllib.error.HTTPError as exc:
                detail = exc.read().decode("utf-8", errors="replace")[:300]
                if exc.code == 429 and "Airlock temporarily blocked" in detail:
                    raise Comp03Error(f"Airlock quarantine: {detail}") from exc
                if not (exc.code == 429 or exc.code >= 500) or attempt == self.max_retries:
                    raise Comp03Error(f"Airlock HTTP {exc.code}: {detail}") from exc
            except (urllib.error.URLError, TimeoutError, Comp03Error, KeyError, IndexError) as exc:
                if attempt == self.max_retries:
                    raise Comp03Error(f"Airlock call failed: {exc}") from exc
            time.sleep(min(2 ** (attempt + 1), 60))
        raise AssertionError("unreachable")


# --- run -----------------------------------------------------------------------


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    tmp.write_text(json.dumps(value, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    tmp.replace(path)


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _model_call(client: Any, spec: Mapping[str, Any], messages: list[dict[str, str]], live: Mapping[str, Any],
                spend: dict[str, float]) -> dict[str, Any]:
    if spend["usd"] >= live["max_usd"]:
        raise Comp03Error(f"cost cap reached: ${spend['usd']:.2f}")
    started = time.perf_counter()
    reply = client.complete(spec["model"], messages, spec["max_tokens"],
                            temperature=live["temperature"], seed=live["seed"])
    usd = (reply.prompt_tokens * spec["input_usd_per_million"]
           + reply.completion_tokens * spec["output_usd_per_million"]) / 1e6
    spend["usd"] += usd
    return {"model": spec["model"], "content": reply.content, "prompt_tokens": reply.prompt_tokens,
            "completion_tokens": reply.completion_tokens, "usd": usd, "elapsed_s": time.perf_counter() - started,
            "messages_sha256": _digest(messages)}


def _host() -> dict[str, Any]:
    return {**comp_fts._host(), "machine": platform.machine()}


def run(config: dict[str, Any], *, dataset_path: Path, attempt_root: Path, client: Any, base_dir: Path | None,
        fathomdb_bin: str | None, config_path: str | None, max_questions: int | None = None,
        sweep_summary: Path | None = None) -> tuple[str, Path]:
    """Execute or resume every stage, then write the receipt and index row."""
    if sha256_file(dataset_path) != config["corpus"]["sha256"]:
        raise Comp03Error("dataset digest does not match the config")
    dataset = {item["question_id"]: item for item in json.loads(dataset_path.read_text(encoding="utf-8"))}
    sel = config["selection"]
    qids = list(sel["question_ids"])
    missing = [q for q in qids if q not in dataset]
    if missing:
        raise Comp03Error(f"selected questions missing from dataset: {missing[:3]}")
    seeded = select_questions(list(dataset.values()), per_type=sel["per_type"], seed=sel["seed"]) \
        if len(dataset) >= sel["per_type"] * len(QUESTION_TYPES) else None
    if max_questions is not None:
        qids = qids[:max_questions]
    running, gate = active_arms(config, sweep_summary)
    arms = tuple(arm["id"] for arm in running)
    products = {arm["id"]: product_arm(arm) for arm in running}
    live, ev, top_k = config["live"], config["evidence"], config["retrieval"]["top_k"]
    attempt_root.mkdir(parents=True, exist_ok=True)
    _write_json(attempt_root / "config.resolved.json", config)
    spend = {"usd": sum(_read_json(p)["usd"] for p in attempt_root.glob("questions/*/*/*-call.json"))}
    rows, retrieval_rows = [], []
    for index, qid in enumerate(qids):
        case, gold = convert_question(dataset[qid]), gold_for(dataset[qid])
        qdir = attempt_root / "questions" / qid
        order = arm_order(index, arms)
        verdicts: dict[str, bool] = {}
        for arm in order:
            adir = qdir / arm
            if not (adir / "retrieval.json").exists():
                result = retrieve(products[arm], case, attempt_root / "work", top_k=top_k,
                                  fathomdb_bin=fathomdb_bin, store_label=arm)
                evidence = render_evidence(case["docs"], result["ids"], max_item_chars=ev["max_item_chars"],
                                           max_total_chars=ev["max_total_chars"])
                result["evidence_sha256"] = hashlib.sha256(evidence.encode("utf-8")).hexdigest()
                result["evidence_chars"] = len(evidence)
                _write_json(adir / "retrieval.json", result)
            retrieval = _read_json(adir / "retrieval.json")
            evidence = render_evidence(case["docs"], retrieval["ids"], max_item_chars=ev["max_item_chars"],
                                       max_total_chars=ev["max_total_chars"])
            if hashlib.sha256(evidence.encode("utf-8")).hexdigest() != retrieval["evidence_sha256"]:
                raise Comp03Error(f"evidence for {qid}/{arm} no longer matches its retrieval")
            if not (adir / "answer-call.json").exists():
                call = _model_call(client, live["answerer"], answer_messages(case, evidence), live, spend)
                call["retrieval_ids"], call["evidence_sha256"] = retrieval["ids"], retrieval["evidence_sha256"]
                _write_json(adir / "answer-call.json", call)
            answer = _read_json(adir / "answer-call.json")
            if not (adir / "judge-call.json").exists():
                messages = judge_messages(case["question_type"], case["question"], gold["answer"],
                                          answer["content"], abstention=gold["abstention"])
                call = _model_call(client, live["judge"], messages, live, spend)
                call["verdict"] = parse_verdict(call["content"])
                _write_json(adir / "judge-call.json", call)
            verdicts[arm] = _read_json(adir / "judge-call.json")["verdict"]
            if not gold["abstention"]:
                retrieval_rows.append({"arm": arm, "question_type": case["question_type"],
                                       **retrieval_scores(retrieval["ids"], gold["session_relevant"],
                                                          gold["turn_relevant"], k=top_k)})
        rows.append({"question_id": qid, "question_type": case["question_type"],
                     "abstention": gold["abstention"], **verdicts})
        print(f"[{index + 1}/{len(qids)}] {qid} {verdicts} ${spend['usd']:.2f}", flush=True)
    metrics = summarize(rows, retrieval_rows, attempt_root, arms, live, k=top_k)
    metrics["selection_matches_seed"] = seeded == list(sel["question_ids"])
    if config["schema_version"] != "comp-03.answer.v1":
        metrics["arm_gate"] = gate
    _write_json(attempt_root / "summary.json", metrics)
    return write_receipt(config, metrics, attempt_root=attempt_root, dataset_path=dataset_path,
                         base_dir=base_dir, config_path=config_path)


def _accuracy_only(rows: Sequence[Mapping[str, Any]], arms: Sequence[str]) -> dict[str, Any]:
    n = len(rows)
    return {"n": n, "accuracy": {arm: (sum(bool(r[arm]) for r in rows) / n if n else 0.0) for arm in arms}}


def summarize(rows: list[dict[str, Any]], retrieval_rows: list[dict[str, Any]], attempt_root: Path,
              arms: tuple[str, ...], live: Mapping[str, Any], *, k: int = 5) -> dict[str, Any]:
    retrieval: dict[str, Any] = {}
    for arm in arms:
        mine = [r for r in retrieval_rows if r["arm"] == arm]
        retrieval[arm] = {"n": len(mine)}
        for key in (f"session_recall_any_at_{k}", f"session_mrr_at_{k}", f"turn_recall_any_at_{k}", f"turn_mrr_at_{k}"):
            values = [r[key] for r in mine if r[key] is not None]
            retrieval[arm][key] = statistics.fmean(values) if values else None
            retrieval[arm][f"{key}_n"] = len(values)
    timing: dict[str, Any] = {}
    for arm in arms:
        rets = [_read_json(p) for p in sorted(attempt_root.glob(f"questions/*/{arm}/retrieval.json"))]
        calls = {stage: [_read_json(p) for p in sorted(attempt_root.glob(f"questions/*/{arm}/{stage}-call.json"))]
                 for stage in ("answer", "judge")}
        first = [r["latency_ms"] for r in rets]
        warm = [statistics.median(r["repeat_latency_ms"]) for r in rets]
        timing[arm] = {
            "version": rets[0]["version"] if rets else None,
            "retrieval_first_p50_ms": statistics.median(first) if first else None,
            "retrieval_warm_p50_ms": statistics.median(warm) if warm else None,
            "retrieval_warm_p95_ms": comp_fts._pct(warm, 0.95) if warm else None,
            "build_p50_s": statistics.median(r["build_return_s"] for r in rets) if rets else None,
            "evidence_chars_p50": statistics.median(r["evidence_chars"] for r in rets) if rets else None,
            "answer_p50_s": statistics.median(c["elapsed_s"] for c in calls["answer"]) if calls["answer"] else None,
            "judge_p50_s": statistics.median(c["elapsed_s"] for c in calls["judge"]) if calls["judge"] else None,
            "usd": sum(c["usd"] for stage in calls.values() for c in stage),
        }
    out: dict[str, Any] = {
        "answer": paired_summary(rows, arms[:2], seed=live["seed"]) if len(arms) >= 2 else _accuracy_only(rows, arms),
        "abstention_n": sum(r["abstention"] for r in rows),
        "retrieval": retrieval,
        "timing": timing,
        "cost_usd": sum(t["usd"] for t in timing.values()),
        "rows": rows,
    }
    if len(arms) > 2:
        out["answer_pairs"] = {
            f"{a}_vs_{b}": paired_summary(rows, (a, b), seed=live["seed"])
            for i, a in enumerate(arms) for b in arms[i + 1:]
        }
        out["accuracy_by_arm"] = _accuracy_only(rows, arms)["accuracy"]
    return out


def write_receipt(config: dict[str, Any], metrics: dict[str, Any], *, attempt_root: Path, dataset_path: Path,
                  base_dir: Path | None, config_path: str | None) -> tuple[str, Path]:
    base = base_dir if base_dir is not None else _lib.EXPERIMENTS_DIR
    git = _lib.git_info()
    answer = metrics["answer"]
    arms = list(metrics["timing"])
    if len(arms) >= 2:
        a, b = arms[:2]
        read = (
            f"{a} {answer['accuracy'][a]:.3f} vs {b} {answer['accuracy'][b]:.3f} answer accuracy on "
            f"{answer['n']} paired questions (difference {answer['difference']:+.3f}, bootstrap 95% "
            f"{answer['difference_bootstrap_95'][0]:+.3f}..{answer['difference_bootstrap_95'][1]:+.3f})"
        )
        if len(arms) > 2:
            read += "; " + ", ".join(f"{arm} {metrics['accuracy_by_arm'][arm]:.3f}" for arm in arms[2:])
    else:
        read = f"{arms[0]} {answer['accuracy'][arms[0]]:.3f} answer accuracy on {answer['n']} questions"
    safe_metrics = {k: v for k, v in metrics.items() if k != "rows"}
    safe_metrics["question_outcomes"] = [
        {"question_id": r["question_id"], **{arm: r[arm] for arm in arms}} for r in metrics["rows"]
    ]
    run_id, run_dir = _lib.write_record(
        EXPERIMENT,
        ts=datetime.now(UTC),
        config_obj=config,
        metrics=safe_metrics,
        verdict="complete" if answer["n"] == len(config["selection"]["question_ids"]) else "partial",
        read=read,
        title="COMP-03 Hermes-style LongMemEval-S answer comparison",
        headline={"accuracy": answer["accuracy"], "difference": answer.get("difference"), "n": answer["n"]},
        n=answer["n"],
        code={"git_sha": git.get("git_sha"), "dirty": git.get("dirty"), "branch": git.get("branch"),
              "baseline_commit": None},
        corpus={"source": config["corpus"]["identifier"], "manifest_sha256": None,
                "datasets": [{"name": dataset_path.name, "sha256": config["corpus"]["sha256"]}]},
        seeds={"selection": config["selection"]["seed"], "model": config["live"]["seed"]},
        env=_lib.env_info(key_deps={arm: metrics["timing"][arm]["version"] for arm in arms}
                          | {"answerer": config["live"]["answerer"]["model"], "judge": config["live"]["judge"]["model"]}),
        cost_usd=round(metrics["cost_usd"], 4),
        config_path=config_path,
        artifacts=[{"kind": "external_attempt", "summary_sha256": sha256_file(attempt_root / "summary.json")}],
        base_dir=base,
    )
    _lib.regen_index_md(index_path=base / "index.jsonl", md_path=base / "INDEX.md")
    return run_id, run_dir


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sel = sub.add_parser("select")
    sel.add_argument("config", type=Path)
    sel.add_argument("--dataset", type=Path, required=True)
    live = sub.add_parser("run")
    live.add_argument("config", type=Path)
    live.add_argument("--dataset", type=Path, required=True)
    live.add_argument("--attempt-root", type=Path, required=True)
    live.add_argument("--fathomdb-bin")
    live.add_argument("--max-questions", type=int)
    live.add_argument("--airlock-url", default="http://127.0.0.1:4000")
    live.add_argument("--base-dir", type=Path, help="receipt root; defaults to experiments/")
    live.add_argument("--sweep-summary", type=Path,
                      help="COMP-03 retrieval sweep summary.json; gated arms run only on its verdict")
    args = parser.parse_args(argv)
    config = load_config(args.config)
    if args.command == "select":
        dataset = json.loads(args.dataset.read_text(encoding="utf-8"))
        picked = select_questions(dataset, per_type=config["selection"]["per_type"], seed=config["selection"]["seed"])
        print(json.dumps({"question_ids": picked, "matches_config": picked == config["selection"]["question_ids"]}))
        return 0
    key = os.environ.get("AIRLOCK_VIRTUAL_KEY") or os.environ.get("R2_JUDGE_API_KEY") or ""
    client = AirlockClient(args.airlock_url, key, config["live"]["max_retries"])
    run_id, run_dir = run(config, dataset_path=args.dataset, attempt_root=args.attempt_root, client=client,
                          base_dir=args.base_dir, fathomdb_bin=args.fathomdb_bin, config_path=str(args.config),
                          max_questions=args.max_questions, sweep_summary=args.sweep_summary)
    print(json.dumps({"run_id": run_id, "receipt": str(run_dir / "record.json")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
