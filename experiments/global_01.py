"""Zero-spend preparation and authorization guard for GLOBAL-01."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import urllib.error
import urllib.request
import zipfile
from html.parser import HTMLParser
from pathlib import Path
from typing import Any, Mapping

from experiments.fathomdb_test_setup import prepare_test_database


SCHEMA = "global-01.first-run.v1"
PROGRAM_TRACK = "GLOBAL-01"
REPORT_SCHEMA = "global-01.preflight.v1"
PRIVATE_SCHEMA = "global-01.private-input-manifest.v1"
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_SECRET_KEY = re.compile(r"(?:api[_-]?key|secret|password|token)", re.IGNORECASE)


class Global01Error(ValueError):
    """Raised when the GLOBAL-01 first-run contract is not satisfied."""


class _TextExtractor(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []

    def handle_data(self, data: str) -> None:
        value = data.strip()
        if value:
            self.parts.append(value)


def canonical_sha256(value: object) -> str:
    """Return the SHA-256 of canonical JSON."""
    payload = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise Global01Error(f"{label} config keys do not match {SCHEMA}")
    return value


def _require_sha(value: object, label: str) -> str:
    if not isinstance(value, str) or _SHA256.fullmatch(value) is None:
        raise Global01Error(f"{label} must be a lowercase SHA-256")
    return value


def validate_config(value: object) -> dict[str, Any]:
    """Strictly validate the dated GLOBAL-01 first-run configuration."""
    root = _exact(
        value,
        "root",
        {
            "schema_version",
            "program_track",
            "run_label",
            "approval",
            "corpus",
            "questions",
            "graphrag",
            "fathomdb",
            "judge",
            "pricing",
        },
    )
    if root["schema_version"] != SCHEMA or root["program_track"] != PROGRAM_TRACK:
        raise Global01Error("GLOBAL-01 schema or program track drifted")
    if root["run_label"] != "apnews-15doc-first-run":
        raise Global01Error("GLOBAL-01 run label drifted")

    approval = _exact(
        root["approval"],
        "approval",
        {"state", "approved_by", "approved_at", "cost_cap_usd"},
    )
    if approval["state"] not in {"pending_hitl", "approved"}:
        raise Global01Error("approval state must be pending_hitl or approved")

    corpus = _exact(
        root["corpus"],
        "corpus",
        {
            "root",
            "archive_sha256",
            "article_count",
            "witness_count",
            "preserved_input_root",
        },
    )
    _require_sha(corpus["archive_sha256"], "corpus archive")
    if corpus["article_count"] < corpus["witness_count"] or corpus["witness_count"] < 1:
        raise Global01Error("corpus witness count is invalid")

    questions = _exact(
        root["questions"],
        "questions",
        {"scope", "count", "selection_rule", "selection_sha256"},
    )
    if (
        questions["scope"] != "global"
        or questions["count"] < 1
        or questions["selection_rule"] != "ordered_global_stride_floor"
    ):
        raise Global01Error("question selection contract drifted")
    _require_sha(questions["selection_sha256"], "question selection")

    graphrag = _exact(
        root["graphrag"],
        "graphrag",
        {
            "version",
            "freeze",
            "freeze_sha256",
            "settings",
            "settings_sha256",
            "prompts",
            "embedding_shim",
            "embedding_shim_sha256",
            "answer_model",
            "thinking_mode",
            "community_level",
            "dynamic_selection",
            "concurrency",
        },
    )
    if (
        graphrag["version"] != "3.1.0"
        or graphrag["answer_model"] != "deepseek-v4-pro"
        or graphrag["thinking_mode"] != "disabled"
        or graphrag["community_level"] != 1
        or graphrag["dynamic_selection"] is not False
        or graphrag["concurrency"] != 4
    ):
        raise Global01Error("GraphRAG contract drifted")
    for key in ("freeze_sha256", "settings_sha256", "embedding_shim_sha256"):
        _require_sha(graphrag[key], f"GraphRAG {key}")

    fathomdb = _exact(
        root["fathomdb"],
        "fathomdb",
        {
            "version",
            "profile",
            "embedder",
            "reranker",
            "map_batch_documents",
            "map_max_tokens",
            "reduce_max_tokens",
        },
    )
    if fathomdb != {
        "version": "0.8.23",
        "profile": "source_mapreduce_c_v1",
        "embedder": "none",
        "reranker": "disabled",
        "map_batch_documents": 5,
        "map_max_tokens": 300,
        "reduce_max_tokens": 1500,
    }:
        raise Global01Error("FathomDB treatment drifted")

    judge = _exact(
        root["judge"],
        "judge",
        {"model", "repetitions", "order_swapped", "metrics"},
    )
    if judge != {
        "model": "claude-haiku",
        "repetitions": 5,
        "order_swapped": True,
        "metrics": [
            "comprehensiveness",
            "diversity",
            "empowerment",
            "directness",
        ],
    }:
        raise Global01Error("judge contract drifted")

    pricing = _exact(
        root["pricing"],
        "pricing",
        {
            "currency",
            "source",
            "deepseek-v4-pro",
            "claude-haiku",
            "projected_total_usd",
            "recommended_cap_usd",
        },
    )
    if pricing["currency"] != "USD" or pricing["projected_total_usd"] <= 0:
        raise Global01Error("pricing contract drifted")
    if pricing["recommended_cap_usd"] < pricing["projected_total_usd"]:
        raise Global01Error("recommended cap is below projected spend")
    return json.loads(json.dumps(root))


def assert_execution_authorized(config: Mapping[str, Any]) -> None:
    """Refuse paid execution without explicit HITL identity, time, and cap."""
    approval = config["approval"]
    if (
        approval["state"] != "approved"
        or not approval["approved_by"]
        or not approval["approved_at"]
        or not isinstance(approval["cost_cap_usd"], (int, float))
        or approval["cost_cap_usd"] <= 0
    ):
        raise Global01Error("paid execution requires explicit HITL approval and a positive cap")


def assert_safe_document(document: object, *, secret_values: list[str]) -> None:
    """Reject secret-bearing keys or known environment secret values."""
    def inspect(value: object) -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                if _SECRET_KEY.search(str(key)) and key not in {"key_source"}:
                    raise Global01Error("safe document contains a secret-bearing key")
                inspect(child)
        elif isinstance(value, list):
            for child in value:
                inspect(child)

    inspect(document)
    serialized = json.dumps(document, sort_keys=True)
    if any(secret and secret in serialized for secret in secret_values):
        raise Global01Error("safe document contains an environment secret value")


def _article_text(row: Mapping[str, Any]) -> tuple[str, str]:
    altids = row.get("altids")
    source_id = altids.get("itemid") if isinstance(altids, dict) else None
    headline = row.get("headline")
    body = row.get("body_nitf")
    if not all(isinstance(item, str) and item for item in (source_id, headline, body)):
        raise Global01Error("AP News article lacks source identity or text")
    parser = _TextExtractor()
    parser.feed(body)
    text = f"{headline}\n\n{' '.join(parser.parts)}".strip()
    return source_id, text


def _load_documents(
    config: Mapping[str, Any], repository_root: Path
) -> tuple[list[dict[str, Any]], str, str]:
    corpus_root = repository_root / config["corpus"]["root"]
    manifest_path = corpus_root / "MANIFEST.json"
    archive_path = corpus_root / "raw_data.zip"
    if not manifest_path.is_file() or not archive_path.is_file():
        raise Global01Error("AP News corpus or manifest is unavailable")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    archive_hash = _sha256(archive_path)
    if (
        archive_hash != config["corpus"]["archive_sha256"]
        or manifest.get("raw_data_zip_sha256") != archive_hash
        or manifest.get("n_articles") != config["corpus"]["article_count"]
    ):
        raise Global01Error("AP News corpus identity drifted")

    documents: list[dict[str, Any]] = []
    with zipfile.ZipFile(archive_path) as archive:
        names = sorted(name for name in archive.namelist() if name.endswith(".json"))
        if len(names) != config["corpus"]["article_count"]:
            raise Global01Error("AP News archive article count drifted")
        for ordinal, name in enumerate(names[: config["corpus"]["witness_count"]]):
            row = json.loads(archive.read(name))
            source_id, text = _article_text(row)
            documents.append(
                {
                    "ordinal": ordinal,
                    "source_id": source_id,
                    "text": text,
                    "content_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
                    "bytes": len(text.encode("utf-8")),
                }
            )
    return documents, _sha256(manifest_path), archive_hash


def inspect_inputs(
    config: Mapping[str, Any], *, repository_root: Path
) -> tuple[dict[str, Any], dict[str, Any]]:
    """Bind the selected corpus and questions without exposing payloads publicly."""
    documents, manifest_hash, archive_hash = _load_documents(config, repository_root)
    preserved = repository_root / config["corpus"]["preserved_input_root"]
    for document in documents:
        path = preserved / f"doc_{document['ordinal']:02d}.txt"
        if not path.is_file() or _sha256(path) != document["content_sha256"]:
            raise Global01Error("preserved GraphRAG input does not match the corpus")

    corpus_root = repository_root / config["corpus"]["root"]
    question_path = corpus_root / "generated_questions_v1" / "activity_global_questions_text.json"
    pool = json.loads(question_path.read_text(encoding="utf-8"))
    if not isinstance(pool, list) or not all(isinstance(item, str) for item in pool):
        raise Global01Error("global question pool is invalid")
    data_global_path = corpus_root / "generated_questions_v1" / "data_global_questions_text.json"
    if data_global_path.is_file():
        data_global = json.loads(data_global_path.read_text(encoding="utf-8"))
        if not isinstance(data_global, list) or not all(isinstance(item, str) for item in data_global):
            raise Global01Error("data-global question pool is invalid")
        pool.extend(data_global)
    assertion_path = corpus_root / "generated_questions_v2" / "data_global_questions_assertions.json"
    if assertion_path.is_file():
        assertions = json.loads(assertion_path.read_text(encoding="utf-8"))
        if not isinstance(assertions, list) or not all(
            isinstance(item, dict) and isinstance(item.get("question_text"), str)
            for item in assertions
        ):
            raise Global01Error("assertion-backed global question pool is invalid")
        pool.extend(item["question_text"] for item in assertions)
    count = config["questions"]["count"]
    if len(pool) < count:
        raise Global01Error("global question pool is too small")
    stride = max(1, len(pool) // count)
    selected = pool[::stride][:count]
    if canonical_sha256(selected) != config["questions"]["selection_sha256"]:
        raise Global01Error("global question selection drifted")

    private_manifest = {
        "schema_version": PRIVATE_SCHEMA,
        "documents": [
            {key: document[key] for key in ("ordinal", "source_id", "content_sha256", "bytes")}
            for document in documents
        ],
        "questions": [
            {
                "ordinal": ordinal,
                "question_id": f"global-{ordinal:02d}-{hashlib.sha256(text.encode()).hexdigest()[:12]}",
                "text": text,
                "sha256": hashlib.sha256(text.encode()).hexdigest(),
            }
            for ordinal, text in enumerate(selected)
        ],
    }
    report = {
        "state": "input_ready",
        "corpus": {
            "manifest_sha256": manifest_hash,
            "archive_sha256": archive_hash,
            "article_count": config["corpus"]["article_count"],
            "witness_count": len(documents),
            "document_binding_sha256": canonical_sha256(private_manifest["documents"]),
            "all_preserved_inputs_match": True,
            "license_class": "noncommercial_nonredistributable_evaluation_only",
        },
        "questions": {
            "global_pool_count": len(pool),
            "count": len(selected),
            "stride": stride,
            "selection_sha256": canonical_sha256(selected),
        },
    }
    return report, private_manifest


def inspect_contract_files(config: Mapping[str, Any], repository_root: Path) -> dict[str, Any]:
    """Verify the frozen native GraphRAG settings, prompts, and embed shim."""
    graphrag = config["graphrag"]
    for key, digest_key in (
        ("freeze", "freeze_sha256"),
        ("settings", "settings_sha256"),
        ("embedding_shim", "embedding_shim_sha256"),
    ):
        path = repository_root / graphrag[key]
        if not path.is_file() or _sha256(path) != graphrag[digest_key]:
            raise Global01Error(f"GraphRAG {key} drifted")
    prompt_root = repository_root / graphrag["prompts"]
    prompts = [
        {"name": path.name, "sha256": _sha256(path)}
        for path in sorted(prompt_root.iterdir())
        if path.is_file()
    ]
    if not prompts:
        raise Global01Error("GraphRAG prompt set is empty")
    return {
        "freeze_sha256": graphrag["freeze_sha256"],
        "settings_sha256": graphrag["settings_sha256"],
        "embedding_shim_sha256": graphrag["embedding_shim_sha256"],
        "prompt_count": len(prompts),
        "prompt_manifest_sha256": canonical_sha256(prompts),
    }


def inspect_airlock(
    base_url: str, key: str, config: Mapping[str, Any]
) -> dict[str, Any]:
    """Perform a zero-spend authenticated model-discovery probe."""
    request = urllib.request.Request(
        f"{base_url.rstrip('/')}/v1/models",
        headers={"Authorization": f"Bearer {key}"},
    )
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            payload = json.load(response)
    except urllib.error.HTTPError as exc:
        retry_after = exc.headers.get("Retry-After")
        suffix = f"; Retry-After={retry_after}" if retry_after else ""
        raise Global01Error(f"Airlock model discovery failed with HTTP {exc.code}{suffix}") from exc
    models = {
        item.get("id"): item for item in payload.get("data", []) if isinstance(item, dict)
    }
    required = (config["judge"]["model"], config["graphrag"]["answer_model"])
    if any(name not in models for name in required):
        raise Global01Error("Airlock lacks a required GLOBAL-01 model alias")
    capabilities = {}
    for name in required:
        info = models[name].get("model_info") or {}
        capabilities[name] = {
            key: info.get(key)
            for key in ("airlock_provider", "underlying", "deprecated", "endpoints")
        }
    return {
        "key_source": "AIRLOCK_VIRTUAL_KEY",
        "required_aliases": list(required),
        "capabilities": capabilities,
        "model_count": len(models),
    }


def inspect_environment(
    config: Mapping[str, Any],
    repository_root: Path,
    artifact_root: Path,
    documents: list[dict[str, Any]],
    fathomdb_bin: str,
) -> dict[str, Any]:
    """Check installed toolchains and create one fresh FathomDB witness database."""
    graph_python = (
        repository_root
        / "data/performance-benchmarking/global-01/toolchains/graphrag-3.1.0-venv/bin/python"
    )
    installed_freeze = (
        repository_root
        / "data/performance-benchmarking/global-01/toolchains/installed-freeze.txt"
    )
    if not graph_python.is_file() or not installed_freeze.is_file():
        raise Global01Error("pinned GraphRAG toolchain is unavailable")
    version = subprocess.run(
        [str(graph_python), "-c", "import importlib.metadata as m; print(m.version('graphrag'))"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if version != config["graphrag"]["version"]:
        raise Global01Error("installed GraphRAG version drifted")

    prepared = prepare_test_database(
        artifact_root / "fathomdb",
        test_id="global-01-preflight",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        warm_cache=False,
        check_reranker=False,
        fathomdb_bin=fathomdb_bin,
    )
    from fathomdb import Engine, __version__ as fathomdb_version

    if fathomdb_version != config["fathomdb"]["version"]:
        raise Global01Error("installed FathomDB version drifted")
    engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "document",
                    "logical_id": f"global-01-{document['ordinal']:02d}",
                    "source_id": document["source_id"],
                    "body": document["text"],
                }
                for document in documents
            ]
        )
        engine.drain(timeout_s=60)
    finally:
        engine.close()

    cli_path = Path(fathomdb_bin).resolve()
    return {
        "python": f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
        "disk_free_bytes": shutil.disk_usage(artifact_root).free,
        "graphrag": {
            "version": version,
            "python_version": subprocess.run(
                [str(graph_python), "-c", "import platform; print(platform.python_version())"],
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip(),
            "installed_freeze_sha256": _sha256(installed_freeze),
        },
        "fathomdb": {
            "version": fathomdb_version,
            "cli_sha256": _sha256(cli_path),
            "database_count": 1,
            "written_count": len(documents),
            "config_sha256": _sha256(prepared.config_path),
            "doctor_sha256": _sha256(prepared.doctor_path),
        },
        "gpu_required": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--repository-root", type=Path, default=Path.cwd())
    parser.add_argument("--fathomdb-bin", default="fathomdb")
    parser.add_argument("--airlock-url", default="http://127.0.0.1:4000")
    args = parser.parse_args()

    if args.artifact_root.exists():
        raise Global01Error("preflight artifact root already exists")
    args.artifact_root.mkdir(parents=True, mode=0o700)
    config = validate_config(json.loads(args.config.read_text(encoding="utf-8")))
    inputs, private_manifest = inspect_inputs(config, repository_root=args.repository_root)
    documents, _, _ = _load_documents(config, args.repository_root)
    key = os.environ.get("AIRLOCK_VIRTUAL_KEY")
    if not key:
        raise Global01Error("AIRLOCK_VIRTUAL_KEY is required")
    report = {
        "schema_version": REPORT_SCHEMA,
        "program_track": PROGRAM_TRACK,
        "state": "ready_for_hitl",
        "zero_spend": True,
        "cost_usd": 0.0,
        "inputs": inputs,
        "contract_files": inspect_contract_files(config, args.repository_root),
        "environment": inspect_environment(
            config,
            args.repository_root,
            args.artifact_root,
            documents,
            args.fathomdb_bin,
        ),
        "authentication": inspect_airlock(args.airlock_url, key, config),
        "pricing": {
            "source": config["pricing"]["source"],
            "projected_total_usd": config["pricing"]["projected_total_usd"],
            "recommended_cap_usd": config["pricing"]["recommended_cap_usd"],
            "airlock_serves_pricing": False,
        },
        "execution_authorized": False,
        "next_gate": "explicit_hitl_cost_and_execution_authorization",
    }
    private_path = args.artifact_root / "private-input-manifest.json"
    private_path.write_text(json.dumps(private_manifest, indent=2) + "\n", encoding="utf-8")
    private_path.chmod(0o600)
    report["private_manifest_sha256"] = _sha256(private_path)
    secrets = [
        value
        for name, value in os.environ.items()
        if _SECRET_KEY.search(name) and value
    ]
    assert_safe_document(report, secret_values=secrets)
    safe_path = args.artifact_root / "safe-preflight.json"
    safe_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    safe_path.chmod(0o600)
    print(json.dumps({"state": report["state"], "safe_preflight": str(safe_path)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
