"""Record a native Mem0-OSS LOCOMO run in FathomDB's experiment ledger.

This adapter deliberately does not implement Mem0 or relabel its result as an
EARP run.  It freezes a non-secret native-harness configuration, constructs
the official ``memory-benchmarks`` LOCOMO invocation, and writes the standard
``experiments/runs/<run_id>/`` receipt around its raw output.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import URLError
from urllib.request import Request, urlopen

from experiments import _lib


EXPERIMENT = "mem0-oss-locomo-native"
SCHEMA_VERSION = "mem0-oss.v1"

_TOP_LEVEL = {
    "schema_version",
    "campaign",
    "program_track",
    "harness",
    "corpus",
    "mem0",
    "airlock",
    "benchmark",
    "output",
    "provenance_artifacts",
    "compose",
}
_FIELDS = {
    "harness": {"checkout", "python", "git_sha"},
    "corpus": {"dataset_path", "raw_sha256", "normalized_sha256", "sessions", "eligible_questions"},
    "mem0": {"host", "compose_project", "llm_model", "embedder_model"},
    "airlock": {"base_url", "host_gateway", "llm_alias", "embedder_alias", "redaction_smoke"},
    "benchmark": {
        "project_name",
        "conversations",
        "categories",
        "top_k",
        "top_k_cutoffs",
        "max_workers",
        "rpm",
        "predict_only",
        "resume",
    },
    "output": {"external_root"},
    "provenance_artifact": {"name", "path", "sha256"},
    "compose": {"base_file"},
}


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def external_artifact_manifest(root: Path) -> dict[str, Any]:
    """Return a content-free aggregate receipt for external campaign output."""
    digests: list[str] = []
    total_bytes = 0
    for path in sorted(candidate for candidate in root.rglob("*") if candidate.is_file()):
        if path.name == "external-artifacts.manifest.v1.json":
            continue
        digests.append(_sha256(path))
        total_bytes += path.stat().st_size
    return {
        "schema_version": "mem0-oss.external-artifacts.v1",
        "file_count": len(digests),
        "total_bytes": total_bytes,
        "content_tree_sha256": hashlib.sha256("".join(sorted(digests)).encode("ascii")).hexdigest(),
    }


def _require_keys(value: object, *, name: str, expected: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{name} must be an object")
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        unknown = sorted(actual - expected)
        raise ValueError(f"{name} keys mismatch: missing={missing}, unknown={unknown}")
    return value


def _reject_secrets(value: object, *, path: str = "") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            key_path = f"{path}.{key}" if path else str(key)
            if any(token in str(key).lower() for token in ("api_key", "secret", "token", "password", "credential")):
                raise ValueError(f"secrets are forbidden in experiment config: {key_path}")
            _reject_secrets(child, path=key_path)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _reject_secrets(child, path=f"{path}[{index}]")
    elif isinstance(value, str) and value.startswith("sk-"):
        raise ValueError(f"secrets are forbidden in experiment config: {path}")


def _require_sha256(value: object, *, name: str) -> str:
    if not isinstance(value, str) or len(value) != 64 or any(char not in "0123456789abcdef" for char in value):
        raise ValueError(f"{name} must be a lowercase SHA-256 hex digest")
    return value


def resolve_config(document: object) -> dict[str, Any]:
    """Validate a non-secret native Mem0 LOCOMO configuration.

    The returned mapping is safe to hash and persist as ``config.resolved.yaml``:
    credentials live only in the process/container environment, never here.
    """
    root = _require_keys(document, name="config", expected=_TOP_LEVEL)
    _reject_secrets(root)
    if root["schema_version"] != SCHEMA_VERSION:
        raise ValueError(f"schema_version must be {SCHEMA_VERSION!r}")
    if root["campaign"] != "native_locomo_predict":
        raise ValueError("campaign must be 'native_locomo_predict'")
    if root["program_track"] != "MEMORY-01":
        raise ValueError("program_track must be 'MEMORY-01' for the native Mem0 arm")

    harness = _require_keys(root["harness"], name="harness", expected=_FIELDS["harness"])
    corpus = _require_keys(root["corpus"], name="corpus", expected=_FIELDS["corpus"])
    mem0 = _require_keys(root["mem0"], name="mem0", expected=_FIELDS["mem0"])
    airlock = _require_keys(root["airlock"], name="airlock", expected=_FIELDS["airlock"])
    benchmark = _require_keys(root["benchmark"], name="benchmark", expected=_FIELDS["benchmark"])
    output = _require_keys(root["output"], name="output", expected=_FIELDS["output"])
    compose = _require_keys(root["compose"], name="compose", expected=_FIELDS["compose"])

    if not isinstance(harness["checkout"], str) or not harness["checkout"]:
        raise ValueError("harness.checkout must be a non-empty path")
    if not isinstance(harness["python"], str) or not harness["python"]:
        raise ValueError("harness.python must name the isolated harness interpreter")
    if not isinstance(harness["git_sha"], str) or len(harness["git_sha"]) < 12:
        raise ValueError("harness.git_sha must be a pinned commit")
    if not isinstance(compose["base_file"], str) or not Path(compose["base_file"]).is_file():
        raise ValueError("compose.base_file must name the official Compose file")
    dataset_path = Path(str(corpus["dataset_path"]))
    if not dataset_path.is_file():
        raise ValueError(f"corpus.dataset_path does not exist: {dataset_path}")
    if _sha256(dataset_path) != _require_sha256(corpus["raw_sha256"], name="corpus.raw_sha256"):
        raise ValueError("corpus.raw_sha256 mismatch")
    _require_sha256(corpus["normalized_sha256"], name="corpus.normalized_sha256")
    if not isinstance(corpus["sessions"], int) or corpus["sessions"] <= 0:
        raise ValueError("corpus.sessions must be positive")
    if not isinstance(corpus["eligible_questions"], int) or corpus["eligible_questions"] <= 0:
        raise ValueError("corpus.eligible_questions must be positive")

    if mem0["host"] != "http://127.0.0.1:8888":
        raise ValueError("mem0.host must be the local official OSS service")
    airlock_routes = {
        "http://host.docker.internal:4000/v1": True,
        "http://127.0.0.1:4000/v1": False,
    }
    if airlock["base_url"] not in airlock_routes:
        raise ValueError("airlock.base_url must be a local Airlock OpenAI endpoint")
    if airlock["host_gateway"] is not airlock_routes[airlock["base_url"]]:
        raise ValueError("airlock.host_gateway must match the selected container network route")
    if airlock["llm_alias"] != mem0["llm_model"] or airlock["embedder_alias"] != mem0["embedder_model"]:
        raise ValueError("Airlock aliases must exactly match the Mem0 model identifiers")
    if airlock["redaction_smoke"] != "required":
        raise ValueError("airlock.redaction_smoke must be 'required'")

    if benchmark["predict_only"] is not True:
        raise ValueError("native receipt phase must be predict_only before answer/judge authorization")
    if benchmark["resume"] is not True:
        raise ValueError("native receipt phase must enable resume")
    if not isinstance(benchmark["top_k"], int) or isinstance(benchmark["top_k"], bool) or not 1 <= benchmark["top_k"] <= 10:
        raise ValueError("benchmark.top_k must be an integer in [1, 10] for the matched FathomDB FTS arm")
    if not isinstance(output["external_root"], str) or not output["external_root"]:
        raise ValueError("output.external_root must be a non-empty external path")
    if not isinstance(benchmark["top_k_cutoffs"], list) or not all(isinstance(item, int) for item in benchmark["top_k_cutoffs"]):
        raise ValueError("benchmark.top_k_cutoffs must be an integer list")
    if not benchmark["top_k_cutoffs"] or any(item < 1 or item > benchmark["top_k"] for item in benchmark["top_k_cutoffs"]):
        raise ValueError("benchmark.top_k_cutoffs must be non-empty and not exceed top_k")

    artifacts = root["provenance_artifacts"]
    if not isinstance(artifacts, list) or not artifacts:
        raise ValueError("provenance_artifacts must be a non-empty list")
    names: set[str] = set()
    for item in artifacts:
        artifact = _require_keys(item, name="provenance_artifact", expected=_FIELDS["provenance_artifact"])
        if not isinstance(artifact["name"], str) or not artifact["name"] or artifact["name"] in names:
            raise ValueError("provenance artifact names must be unique and non-empty")
        names.add(artifact["name"])
        path = Path(str(artifact["path"]))
        if not path.is_file():
            raise ValueError(f"provenance artifact does not exist: {path}")
        if _sha256(path) != _require_sha256(artifact["sha256"], name=f"provenance_artifact[{artifact['name']}].sha256"):
            raise ValueError(f"provenance artifact sha256 mismatch: {artifact['name']}")
    if names != {"compose_override", "mem0_airlock_config"}:
        raise ValueError("provenance artifacts must be compose_override and mem0_airlock_config")
    return root


def load_config(path: str | Path) -> dict[str, Any]:
    """Load and resolve a JSON campaign configuration."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read configuration: {exc}") from exc
    return resolve_config(document)


def preflight(config: dict[str, Any]) -> list[str]:
    """Return all local, non-model-bearing prerequisites that are missing."""
    resolved = resolve_config(config)
    failures: list[str] = []
    checkout = Path(resolved["harness"]["checkout"])
    interpreter = Path(resolved["harness"]["python"])
    if not checkout.is_dir():
        failures.append(f"harness.checkout missing: {checkout}")
    if not interpreter.is_file() or not interpreter.stat().st_mode & 0o111:
        failures.append(f"harness.python is not executable: {interpreter}")
    if checkout.is_dir():
        probe = subprocess.run(
            ["git", "status", "--porcelain"], cwd=checkout, check=False,
            text=True, capture_output=True, env=_lib.git_env(),
        )
        if probe.returncode != 0:
            failures.append("cannot inspect harness cleanliness")
        elif probe.stdout.strip():
            failures.append("harness checkout must be clean")
    artifacts_by_name = {item["name"]: Path(item["path"]) for item in resolved["provenance_artifacts"]}
    compose = artifacts_by_name["compose_override"]
    mem0_config = artifacts_by_name["mem0_airlock_config"]
    compose_text = compose.read_text(encoding="utf-8")
    mem0_text = mem0_config.read_text(encoding="utf-8")
    required_compose = (
        ("host.docker.internal:host-gateway", "OPENAI_API_KEY: ${AIRLOCK_VIRTUAL_KEY", ":/app/config.yaml:ro")
        if resolved["airlock"]["host_gateway"]
        else ("network_mode: host", "QDRANT_HOST: 127.0.0.1", "OPENAI_API_KEY: ${AIRLOCK_VIRTUAL_KEY", ":/app/config.yaml:ro")
    )
    for required in required_compose:
        if required not in compose_text:
            failures.append(f"compose overlay is missing {required!r}")
    for required in (
        "gpt-4o-mini",
        "text-embedding-3-small",
        resolved["airlock"]["base_url"],
        "${OPENAI_API_KEY}",
    ):
        if required not in mem0_text:
            failures.append(f"Mem0 Airlock config is missing {required!r}")
    for line in (*compose_text.splitlines(), *mem0_text.splitlines()):
        stripped = line.strip()
        if "api_key:" in stripped.lower() and "${" not in stripped:
            failures.append("overlay contains a literal credential")
            break
    return failures


def preflight_live(config: dict[str, Any]) -> list[str]:
    """Verify the live local services without sending any model request."""
    failures = preflight(config)
    key = __import__("os").environ.get("AIRLOCK_VIRTUAL_KEY")
    if not key:
        return [*failures, "AIRLOCK_VIRTUAL_KEY is not set for authenticated Airlock preflight"]
    try:
        request = Request(
            "http://127.0.0.1:4000/v1/models",
            headers={"Authorization": f"Bearer {key}"},
        )
        with urlopen(request, timeout=15) as response:  # noqa: S310 - fixed loopback endpoint
            models = json.loads(response.read())
        model_ids = {item.get("id") for item in models.get("data", []) if isinstance(item, dict)}
        for alias in (config["airlock"]["llm_alias"], config["airlock"]["embedder_alias"]):
            if alias not in model_ids:
                failures.append(f"Airlock model alias unavailable: {alias}")
    except (OSError, URLError, ValueError, json.JSONDecodeError) as exc:
        failures.append(f"Airlock authenticated preflight failed: {exc}")
    for name, endpoint in (
        ("Mem0", "http://127.0.0.1:8888/health"),
        ("Qdrant", "http://127.0.0.1:6333/collections"),
    ):
        try:
            with urlopen(endpoint, timeout=15) as response:  # noqa: S310 - fixed loopback endpoint
                if response.status != 200:
                    failures.append(f"{name} health returned HTTP {response.status}")
        except (OSError, URLError) as exc:
            failures.append(f"{name} health preflight failed: {exc}")
    return failures


def compose_command(config: dict[str, Any], action: str) -> list[str]:
    """Build the controlled lifecycle command for the pinned Mem0 services."""
    resolved = resolve_config(config)
    artifacts = {item["name"]: item["path"] for item in resolved["provenance_artifacts"]}
    command = [
        "docker", "compose", "-p", resolved["mem0"]["compose_project"],
        "-f", resolved["compose"]["base_file"], "-f", artifacts["compose_override"],
    ]
    if action == "up":
        return [*command, "up", "-d", "mem0", "qdrant"]
    if action == "config":
        return [*command, "config"]
    raise ValueError(f"unsupported compose action: {action}")


def services_up(config: dict[str, Any]) -> int:
    """Start only the isolated Mem0 and Qdrant campaign services."""
    if not __import__("os").environ.get("AIRLOCK_VIRTUAL_KEY"):
        raise ValueError("AIRLOCK_VIRTUAL_KEY is required to start Mem0")
    command = compose_command(config, "up")
    return subprocess.run(command, check=False).returncode


def build_harness_command(config: dict[str, Any], *, run_id: str, raw_dir: Path) -> list[str]:
    """Build the official native Mem0 LOCOMO predict-only command without credentials."""
    benchmark = config["benchmark"]
    return [
        str(config["harness"]["python"]),
        "-m",
        "benchmarks.locomo.run",
        "--project-name",
        str(benchmark["project_name"]),
        "--run-id",
        run_id,
        "--dataset-path",
        str(config["corpus"]["dataset_path"]),
        "--output-dir",
        str(raw_dir),
        "--conversations",
        str(benchmark["conversations"]),
        "--categories",
        str(benchmark["categories"]),
        "--top-k",
        str(benchmark["top_k"]),
        "--top-k-cutoffs",
        ",".join(str(item) for item in benchmark["top_k_cutoffs"]),
        "--max-workers",
        str(benchmark["max_workers"]),
        "--rpm",
        str(benchmark["rpm"]),
        "--backend",
        "oss",
        "--mem0-host",
        str(config["mem0"]["host"]),
        "--predict-only",
        "--resume",
    ]


def _artifact(path: Path, *, run_dir: Path, external: bool = False) -> dict[str, str]:
    reference = str(path) if external else str(path.relative_to(run_dir.parent))
    return {"path": reference, "sha256": _sha256(path)}


def _native_output_dir(config: dict[str, Any], raw_dir: Path) -> Path:
    return raw_dir / f"predicted_{config['benchmark']['project_name']}"


def _expected_question_ids(config: dict[str, Any]) -> list[str]:
    dataset = json.loads(Path(config["corpus"]["dataset_path"]).read_text(encoding="utf-8"))
    conversations = {int(item) for item in str(config["benchmark"]["conversations"]).split(",")}
    categories = {int(item) for item in str(config["benchmark"]["categories"]).split(",")}
    return [
        f"conv{conversation_index}_q{question_index}"
        for conversation_index, conversation in enumerate(dataset)
        if conversation_index in conversations
        for question_index, question in enumerate(conversation.get("qa", conversation.get("qa_pairs", [])))
        if question.get("category") in categories
    ]


def predict_completion(config: dict[str, Any], raw_dir: Path) -> dict[str, Any]:
    """Summarize predict-only checkpoints without retaining their payload."""
    output_dir = _native_output_dir(config, raw_dir)
    missing: list[str] = []
    latencies: list[float] = []
    result_counts: list[int] = []
    failed_ingestions: list[int] = []
    conversations = {int(item) for item in str(config["benchmark"]["conversations"]).split(",")}
    for conversation_index in sorted(conversations):
        try:
            checkpoint_path = output_dir / f"_ingestion_{conversation_index}.json"
            if not checkpoint_path.is_file():
                checkpoint_path = raw_dir / f"_ingestion_{conversation_index}.json"
            checkpoint = json.loads(checkpoint_path.read_text(encoding="utf-8"))
            if checkpoint.get("conversation_idx") != conversation_index or checkpoint.get("total_chunks_failed") != 0:
                raise ValueError("invalid or failed ingestion checkpoint")
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError):
            failed_ingestions.append(conversation_index)
    for question_id in _expected_question_ids(config):
        path = output_dir / f"{question_id}.json"
        try:
            result = json.loads(path.read_text(encoding="utf-8"))
            retrieval = result["retrieval"]
            if result.get("question_id") != question_id or not isinstance(retrieval["search_results"], list):
                raise ValueError("invalid retrieval result")
            latencies.append(float(retrieval["search_latency_ms"]))
            result_counts.append(int(retrieval["total_results"]))
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError):
            missing.append(question_id)
    return {
        "complete": not missing and not failed_ingestions,
        "expected_questions": len(_expected_question_ids(config)),
        "completed_questions": len(_expected_question_ids(config)) - len(missing),
        "missing_question_ids": missing,
        "failed_ingestion_conversations": failed_ingestions,
        "search_latency_ms": {"n": len(latencies), "mean": sum(latencies) / len(latencies) if latencies else None},
        "retrieval_results": {"n": len(result_counts), "total": sum(result_counts)},
    }


def _metrics(config: dict[str, Any], raw_dir: Path) -> dict[str, Any]:
    completion = predict_completion(config, raw_dir)
    return {
        "phase": "ingest_search_predict_only",
        "completion": completion,
        "cost": {
            "status": "unavailable",
            "value": None,
            "reason": "the Airlock service does not provide a run-isolated spend receipt",
        },
    }


def _provenance_refs(config: dict[str, Any], run_dir: Path) -> list[dict[str, str]]:
    """Reference checked overlays by digest without copying their contents."""
    artifacts: list[dict[str, str]] = []
    for item in config["provenance_artifacts"]:
        source = Path(item["path"])
        artifacts.append(_artifact(source, run_dir=run_dir, external=True))
    return artifacts


def write_receipt(
    config: dict[str, Any],
    *,
    ts: datetime,
    base_dir: str | Path,
    code: dict[str, Any],
    env: dict[str, Any],
    verdict: str,
    read: str,
    raw_dir: Path,
) -> tuple[str, Path]:
    """Write a standard receipt that hash-references external native output."""
    resolved = resolve_config(config)
    runs_root = (Path(base_dir) / "runs").resolve()
    if raw_dir.resolve().is_relative_to(runs_root):
        raise ValueError("raw native output must remain outside experiments/runs")
    if raw_dir.resolve().is_relative_to(_lib.REPO_ROOT.resolve()):
        raise ValueError("raw native output must remain outside the repository")
    run_id = _lib.make_run_id(EXPERIMENT, ts, _lib.config_sha256(resolved))
    run_dir = Path(base_dir) / "runs" / run_id
    artifacts = _provenance_refs(resolved, run_dir)
    completion = predict_completion(resolved, raw_dir)
    arm_result = {
        "schema_version": "mem0-oss-arm.result.v1", "run_id": run_id, "verdict": verdict,
        "completion": completion, "retrieval_mode": "mem0_native",
    }
    run_dir.mkdir(parents=True, exist_ok=True)
    arm_result_path = run_dir / "mem0-oss-arm.result.v1.json"
    arm_result_path.write_text(json.dumps(arm_result, indent=2) + "\n", encoding="utf-8")
    artifacts.append(_artifact(arm_result_path, run_dir=run_dir))
    manifest_path = raw_dir / "external-artifacts.manifest.v1.json"
    manifest_path.write_text(
        json.dumps(external_artifact_manifest(raw_dir), indent=2) + "\n", encoding="utf-8"
    )
    artifacts.append(_artifact(manifest_path, run_dir=run_dir, external=True))

    corpus = resolved["corpus"]
    receipt = _lib.write_record(
        EXPERIMENT,
        ts=ts,
        config_obj=resolved,
        metrics=_metrics(resolved, raw_dir),
        verdict=verdict,
        read=read,
        code=code,
        corpus={
            "source": "LOCOMO",
            "manifest_sha256": corpus["raw_sha256"],
            "datasets": [
                {
                    "name": "locomo10",
                    "normalized_sha256": corpus["normalized_sha256"],
                    "sessions": corpus["sessions"],
                    "eligible_questions": corpus["eligible_questions"],
                }
            ],
        },
        seeds={},
        env=env,
        cost_usd=None,
        headline={"phase": "predict_only"},
        n=corpus["eligible_questions"],
        artifacts=artifacts,
        base_dir=base_dir,
    )
    base = Path(base_dir)
    _lib.regen_index_md(index_path=base / "index.jsonl", md_path=base / "INDEX.md")
    return receipt


def run(config: dict[str, Any], *, base_dir: str | Path) -> tuple[str, Path, int]:
    """Execute the official harness and close a complete or incomplete receipt."""
    resolved = resolve_config(config)
    ts = datetime.now(timezone.utc).replace(second=0, microsecond=0)
    run_id = _lib.make_run_id(EXPERIMENT, ts, _lib.config_sha256(resolved))
    raw_dir = Path(resolved["output"]["external_root"]) / run_id
    if raw_dir.resolve().is_relative_to((Path(base_dir) / "runs").resolve()):
        raise ValueError("output.external_root must remain outside experiments/runs")
    if raw_dir.resolve().is_relative_to(_lib.REPO_ROOT.resolve()):
        raise ValueError("output.external_root must remain outside the repository")
    raw_dir.mkdir(parents=True, exist_ok=True)
    prerequisite_failures = preflight_live(resolved)
    if prerequisite_failures:
        message = "; ".join(prerequisite_failures)
        (raw_dir / "stderr.log").write_text(message + "\n", encoding="utf-8")
        receipt_id, receipt_dir = write_receipt(
            resolved,
            ts=ts,
            base_dir=base_dir,
            code=_lib.git_info(),
            env=_lib.env_info(),
            verdict="blocked_prerequisite",
            read=message,
            raw_dir=raw_dir,
        )
        return receipt_id, receipt_dir, 2
    checkout = Path(resolved["harness"]["checkout"])
    if not checkout.is_dir():
        (raw_dir / "stderr.log").write_text(f"harness.checkout does not exist: {checkout}\n", encoding="utf-8")
        receipt_id, receipt_dir = write_receipt(
            resolved,
            ts=ts,
            base_dir=base_dir,
            code=_lib.git_info(),
            env=_lib.env_info(),
            verdict="blocked_prerequisite",
            read=f"harness.checkout does not exist: {checkout}",
            raw_dir=raw_dir,
        )
        return receipt_id, receipt_dir, 2
    git_probe = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=checkout, check=False, text=True, capture_output=True, env=_lib.git_env()
    )
    if git_probe.returncode != 0:
        message = "cannot inspect harness git SHA"
        (raw_dir / "stderr.log").write_text(git_probe.stderr or message + "\n", encoding="utf-8")
        receipt_id, receipt_dir = write_receipt(
            resolved,
            ts=ts,
            base_dir=base_dir,
            code=_lib.git_info(),
            env=_lib.env_info(),
            verdict="blocked_prerequisite",
            read=message,
            raw_dir=raw_dir,
        )
        return receipt_id, receipt_dir, 2
    observed_sha = git_probe.stdout.strip()
    if observed_sha != resolved["harness"]["git_sha"]:
        message = f"harness git SHA mismatch: expected {resolved['harness']['git_sha']}, got {observed_sha}"
        (raw_dir / "stderr.log").write_text(message + "\n", encoding="utf-8")
        receipt_id, receipt_dir = write_receipt(
            resolved,
            ts=ts,
            base_dir=base_dir,
            code=_lib.git_info(),
            env=_lib.env_info(key_deps={"mem0_harness_git_sha": observed_sha}),
            verdict="blocked_prerequisite",
            read=message,
            raw_dir=raw_dir,
        )
        return receipt_id, receipt_dir, 2
    command = build_harness_command(resolved, run_id=run_id, raw_dir=raw_dir)
    (raw_dir / "command.json").write_text(json.dumps(command, indent=2) + "\n", encoding="utf-8")
    with (raw_dir / "stdout.log").open("w", encoding="utf-8") as stdout, (raw_dir / "stderr.log").open("w", encoding="utf-8") as stderr:
        completed = subprocess.run(command, cwd=checkout, stdout=stdout, stderr=stderr, check=False)
    completion = predict_completion(resolved, raw_dir)
    verdict = "complete" if completed.returncode == 0 and completion["complete"] else "incomplete"
    if verdict == "complete":
        read = "native Mem0 LOCOMO predict-only completed"
    elif completed.returncode == 0:
        read = f"native Mem0 LOCOMO predict-only incomplete: {len(completion['missing_question_ids'])} missing or invalid question outputs"
    else:
        read = f"native Mem0 LOCOMO predict-only exited {completed.returncode}"
    receipt_id, receipt_dir = write_receipt(
        resolved,
        ts=ts,
        base_dir=base_dir,
        code=_lib.git_info(),
        env=_lib.env_info(key_deps={"mem0_harness_git_sha": observed_sha}),
        verdict=verdict,
        read=read,
        raw_dir=raw_dir,
    )
    return receipt_id, receipt_dir, completed.returncode if verdict == "complete" else 1


def main(argv: list[str] | None = None) -> int:
    """Validate, print, or execute a native Mem0-OSS LOCOMO receipt."""
    parser = argparse.ArgumentParser(description="Mem0 OSS LOCOMO experiment receipt adapter")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("validate", "print-command", "services-up", "preflight", "run"):
        command = sub.add_parser(name)
        command.add_argument("config", type=Path)
    args = parser.parse_args(argv)
    try:
        config = load_config(args.config)
        if args.command == "validate":
            print("mem0-oss config resolves")
            return 0
        if args.command == "print-command":
            run_id = _lib.make_run_id(EXPERIMENT, datetime.now(timezone.utc), _lib.config_sha256(config))
            print(json.dumps(build_harness_command(config, run_id=run_id, raw_dir=Path("external-output") / run_id)))
            return 0
        if args.command == "services-up":
            return services_up(config)
        if args.command == "preflight":
            failures = preflight_live(config)
            if failures:
                print("\n".join(failures), file=sys.stderr)
                return 2
            print("mem0-oss preflight passes")
            return 0
        run_id, run_dir, returncode = run(config, base_dir=_lib.EXPERIMENTS_DIR)
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        print(f"mem0-oss: {exc}", file=sys.stderr)
        return 2
    print(f"mem0-oss receipt {run_id}: {run_dir}")
    return returncode


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
