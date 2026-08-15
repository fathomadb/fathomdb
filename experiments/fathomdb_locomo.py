"""Run the official LOCOMO harness through the local FathomDB OSS façade."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import URLError
from urllib.request import urlopen

from experiments import _lib, mem0_oss


EXPERIMENT = "fathomdb-locomo-official-seam"
SCHEMA_VERSION = "fathomdb-locomo.v1"
_TOP_LEVEL = {"schema_version", "campaign", "harness", "corpus", "benchmark", "facade", "output"}
_HARNESS = {"checkout", "python", "git_sha"}
_CORPUS = {"dataset_path", "raw_sha256", "normalized_sha256", "sessions", "eligible_questions"}
_BENCHMARK = {"project_name", "conversations", "categories", "top_k", "top_k_cutoffs", "max_workers", "rpm", "predict_only", "resume"}
_FACADE = {"python", "host", "port", "provenance_manifest", "provenance_manifest_sha256"}
_OUTPUT = {"external_root"}


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _exact(value: object, name: str, expected: set[str]) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        raise ValueError(f"{name} keys mismatch: missing={sorted(expected - actual)}, unknown={sorted(actual - expected)}")
    return value


def _reject_secrets(value: object, path: str = "") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            next_path = f"{path}.{key}" if path else str(key)
            if any(token in str(key).lower() for token in ("api_key", "secret", "token", "password", "credential")):
                raise ValueError(f"secrets are forbidden in FathomDB arm config: {next_path}")
            _reject_secrets(child, next_path)
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _reject_secrets(child, f"{path}[{index}]")
    elif isinstance(value, str) and value.startswith("sk-"):
        raise ValueError(f"secrets are forbidden in FathomDB arm config: {path}")


def resolve_config(document: object) -> dict[str, Any]:
    """Validate the secret-free FathomDB LOCOMO arm configuration."""
    root = _exact(document, "config", _TOP_LEVEL)
    _reject_secrets(root)
    if root["schema_version"] != SCHEMA_VERSION or root["campaign"] != "official_seam_predict_only":
        raise ValueError("config must declare the FathomDB official predict-only campaign")
    harness = _exact(root["harness"], "harness", _HARNESS)
    corpus = _exact(root["corpus"], "corpus", _CORPUS)
    benchmark = _exact(root["benchmark"], "benchmark", _BENCHMARK)
    facade = _exact(root["facade"], "facade", _FACADE)
    output = _exact(root["output"], "output", _OUTPUT)
    if not all(isinstance(harness[key], str) and harness[key] for key in _HARNESS):
        raise ValueError("harness values must be non-empty strings")
    dataset = Path(str(corpus["dataset_path"]))
    if not dataset.is_file() or _sha256(dataset) != corpus["raw_sha256"]:
        raise ValueError("corpus dataset path or raw sha256 is invalid")
    if not isinstance(benchmark["top_k"], int) or benchmark["top_k"] != 10:
        raise ValueError("benchmark.top_k must be 10 for FathomDB's public FTS seam")
    if benchmark["top_k_cutoffs"] != [10] or benchmark["predict_only"] is not True or benchmark["resume"] is not True:
        raise ValueError("FathomDB arm requires predict-only resume with cutoff [10]")
    if facade["host"] != "127.0.0.1" or not isinstance(facade["port"], int) or not 1 <= facade["port"] <= 65535:
        raise ValueError("facade must bind a valid loopback port")
    if not isinstance(facade["python"], str) or not facade["python"]:
        raise ValueError("facade.python must be non-empty")
    provenance_manifest = Path(str(facade["provenance_manifest"]))
    if not provenance_manifest.is_file() or _sha256(provenance_manifest) != facade["provenance_manifest_sha256"]:
        raise ValueError("facade provenance manifest path or sha256 is invalid")
    if not isinstance(output["external_root"], str) or not output["external_root"]:
        raise ValueError("output.external_root must be non-empty")
    return root


def build_harness_command(config: dict[str, Any], *, run_id: str, raw_dir: Path) -> list[str]:
    """Build the official harness command, differing only at the OSS host seam."""
    benchmark = config["benchmark"]
    return [
        config["harness"]["python"], "-m", "benchmarks.locomo.run", "--project-name", benchmark["project_name"],
        "--run-id", run_id, "--dataset-path", config["corpus"]["dataset_path"], "--output-dir", str(raw_dir),
        "--conversations", benchmark["conversations"], "--categories", benchmark["categories"],
        "--top-k", "10", "--top-k-cutoffs", "10", "--max-workers", str(benchmark["max_workers"]),
        "--rpm", str(benchmark["rpm"]), "--backend", "oss", "--mem0-host",
        f"http://{config['facade']['host']}:{config['facade']['port']}", "--predict-only", "--resume",
    ]


def receipt_config(config: dict[str, Any]) -> dict[str, Any]:
    """Project an executable config into the reproducible, path-free receipt form."""
    return {
        "schema_version": config["schema_version"], "campaign": config["campaign"],
        "harness": {"checkout": "external-verified-checkout", "python": "external-verified-interpreter",
                    "git_sha": config["harness"]["git_sha"]},
        "corpus": {"dataset_path": "external-verified-corpus", "raw_sha256": config["corpus"]["raw_sha256"],
                   "normalized_sha256": config["corpus"]["normalized_sha256"], "sessions": config["corpus"]["sessions"],
                   "eligible_questions": config["corpus"]["eligible_questions"]},
        "benchmark": dict(config["benchmark"]),
        "facade": {"python": "external-verified-interpreter", "host": config["facade"]["host"],
                   "port": config["facade"]["port"], "provenance_manifest": "external-provenance-manifest",
                   "provenance_manifest_sha256": config["facade"]["provenance_manifest_sha256"]},
        "output": {"external_root": "external-access-controlled"},
    }


def _receipt_run_id(config: dict[str, Any], *, ts: datetime) -> str:
    return _lib.make_run_id(EXPERIMENT, ts, _lib.config_sha256(receipt_config(config)))


def predict_only_harness_env() -> dict[str, str]:
    """Supply the inert constructor credential the upstream runner requires."""
    environment = dict(os.environ)
    environment["OPENAI_API_KEY"] = "predict-only-placeholder"
    return environment


def write_receipt(config: dict[str, Any], *, ts: datetime, base_dir: str | Path, raw_dir: Path,
                  verdict: str, read: str, completion: dict[str, Any]) -> tuple[str, Path]:
    """Write the generic envelope and a content-free FathomDB arm result."""
    resolved = resolve_config(config)
    public_config = receipt_config(resolved)
    run_id = _receipt_run_id(resolved, ts=ts)
    run_dir = Path(base_dir) / "runs" / run_id
    if raw_dir.resolve().is_relative_to((Path(base_dir) / "runs").resolve()):
        raise ValueError("raw output must remain outside experiments/runs")
    if raw_dir.resolve().is_relative_to(_lib.REPO_ROOT.resolve()):
        raise ValueError("raw output must remain outside the repository")
    raw_dir.mkdir(parents=True, exist_ok=True)
    sidecar = {
        "schema_version": "fathomdb-locomo-arm.result.v1", "run_id": run_id, "verdict": verdict,
        "retrieval_mode": "fts_only", "completion": completion,
    }
    sidecar_path = run_dir / "fathomdb-locomo-arm.result.v1.json"
    run_dir.mkdir(parents=True, exist_ok=True)
    sidecar_path.write_text(json.dumps(sidecar, indent=2) + "\n", encoding="utf-8")
    manifest_path = raw_dir / "external-artifacts.manifest.v1.json"
    manifest_path.write_text(json.dumps(mem0_oss.external_artifact_manifest(raw_dir), indent=2) + "\n", encoding="utf-8")
    _lib.write_record(
        EXPERIMENT, ts=ts, config_obj=public_config, metrics={"phase": "ingest_search_predict_only", "completion": completion},
        verdict=verdict, read=read, code=_lib.git_info(),
        corpus={"source": "LOCOMO", "manifest_sha256": public_config["corpus"]["raw_sha256"], "datasets": []},
        seeds={}, env=_lib.env_info(), cost_usd=0.0, headline={"retrieval_mode": "fts_only"},
        n=public_config["corpus"]["eligible_questions"],
        artifacts=[
            {"path": str(sidecar_path.relative_to(Path(base_dir))), "sha256": _sha256(sidecar_path)},
            {"path": "external-artifacts.manifest.v1", "sha256": _sha256(manifest_path)},
        ], base_dir=base_dir,
    )
    _lib.regen_index_md(index_path=Path(base_dir) / "index.jsonl", md_path=Path(base_dir) / "INDEX.md")
    return run_id, run_dir


def _wait_for_health(host: str, port: int, process: subprocess.Popen[Any]) -> None:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("FathomDB OSS façade exited before becoming healthy")
        try:
            with urlopen(f"http://{host}:{port}/health", timeout=1) as response:  # noqa: S310 - fixed loopback
                if response.status == 200:
                    return
        except (OSError, URLError):
            time.sleep(0.25)
    raise RuntimeError("FathomDB OSS façade health check timed out")


def _capture_facade_sidecar(host: str, port: int, endpoint: str, raw_dir: Path) -> dict[str, Any]:
    """Persist one content-free façade sidecar beside external raw outputs."""
    try:
        with urlopen(f"http://{host}:{port}/{endpoint}", timeout=10) as response:  # noqa: S310 - fixed loopback
            if response.status != 200:
                raise RuntimeError(f"façade {endpoint} endpoint returned HTTP {response.status}")
            payload = json.loads(response.read())
    except (OSError, URLError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"façade {endpoint} sidecar unavailable") from exc
    if not isinstance(payload, dict):
        raise RuntimeError(f"façade {endpoint} sidecar is not an object")
    (raw_dir / f"facade-{endpoint}.v1.json").write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return payload


def run(config: dict[str, Any], *, base_dir: str | Path) -> tuple[str, Path, int]:
    """Start a fresh façade process, run the official harness, then close a receipt."""
    resolved = resolve_config(config)
    ts = datetime.now(timezone.utc).replace(second=0, microsecond=0)
    run_id = _receipt_run_id(resolved, ts=ts)
    raw_dir = Path(resolved["output"]["external_root"]) / run_id
    if raw_dir.resolve().is_relative_to((Path(base_dir) / "runs").resolve()):
        raise ValueError("output.external_root must remain outside experiments/runs")
    if raw_dir.resolve().is_relative_to(_lib.REPO_ROOT.resolve()):
        raise ValueError("output.external_root must remain outside the repository")
    raw_dir.mkdir(parents=True, exist_ok=True)
    checkout = Path(resolved["harness"]["checkout"])
    interpreter = Path(resolved["harness"]["python"])
    facade_interpreter = Path(resolved["facade"]["python"])
    facade_probe = subprocess.run(
        [str(facade_interpreter), "-c", "import fathomdb; print(fathomdb.__file__)"],
        check=False, text=True, capture_output=True,
    ) if facade_interpreter.is_file() and os.access(facade_interpreter, os.X_OK) else None
    git_probe = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=checkout, check=False, text=True,
        capture_output=True, env=_lib.git_env(),
    ) if checkout.is_dir() else None
    clean_probe = subprocess.run(
        ["git", "status", "--porcelain"], cwd=checkout, check=False, text=True,
        capture_output=True, env=_lib.git_env(),
    ) if checkout.is_dir() else None
    invalid_harness = (
        not checkout.is_dir() or not interpreter.is_file() or not os.access(interpreter, os.X_OK)
        or not facade_interpreter.is_file() or not os.access(facade_interpreter, os.X_OK)
        or facade_probe is None or facade_probe.returncode != 0
        or git_probe is None or git_probe.returncode != 0
        or git_probe.stdout.strip() != resolved["harness"]["git_sha"]
        or clean_probe is None or clean_probe.returncode != 0 or bool(clean_probe.stdout.strip())
    )
    if invalid_harness:
        completion = {"complete": False, "expected_questions": resolved["corpus"]["eligible_questions"]}
        receipt_id, receipt_dir = write_receipt(resolved, ts=ts, base_dir=base_dir, raw_dir=raw_dir,
                                                verdict="blocked_prerequisite", read="harness checkout or interpreter unavailable", completion=completion)
        return receipt_id, receipt_dir, 2
    process: subprocess.Popen[Any] | None = None
    facade_metrics: dict[str, Any] | None = None
    facade_provenance: dict[str, Any] | None = None
    try:
        process = subprocess.Popen(
            [resolved["facade"]["python"], "-m", "experiments.fathomdb_oss_facade", "--root", str(raw_dir / "fathomdb"),
             "--host", resolved["facade"]["host"], "--port", str(resolved["facade"]["port"]),
             "--provenance-manifest", resolved["facade"]["provenance_manifest"]],
            cwd=_lib.REPO_ROOT,
            stdout=(raw_dir / "facade.stdout.log").open("w", encoding="utf-8"),
            stderr=(raw_dir / "facade.stderr.log").open("w", encoding="utf-8"),
        )
        _wait_for_health(resolved["facade"]["host"], resolved["facade"]["port"], process)
        command = build_harness_command(resolved, run_id=run_id, raw_dir=raw_dir)
        (raw_dir / "command.json").write_text(json.dumps(command, indent=2) + "\n", encoding="utf-8")
        with (raw_dir / "stdout.log").open("w", encoding="utf-8") as stdout, (raw_dir / "stderr.log").open("w", encoding="utf-8") as stderr:
            completed = subprocess.run(
                command, cwd=checkout, stdout=stdout, stderr=stderr, check=False,
                env=predict_only_harness_env(),
            )
        facade_metrics = _capture_facade_sidecar(resolved["facade"]["host"], resolved["facade"]["port"], "metrics", raw_dir)
        facade_provenance = _capture_facade_sidecar(resolved["facade"]["host"], resolved["facade"]["port"], "provenance", raw_dir)
    except (OSError, RuntimeError) as exc:
        completion = {"complete": False, "expected_questions": resolved["corpus"]["eligible_questions"]}
        receipt_id, receipt_dir = write_receipt(
            resolved, ts=ts, base_dir=base_dir, raw_dir=raw_dir,
            verdict="blocked_prerequisite", read=f"FathomDB OSS façade unavailable: {exc}", completion=completion,
        )
        return receipt_id, receipt_dir, 2
    finally:
        if process is not None and process.poll() is None:
            process.terminate()
        if process is not None:
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
    completion = mem0_oss.predict_completion(resolved, raw_dir)
    completion["facade_metrics"] = facade_metrics
    completion["facade_provenance_requests"] = len(facade_provenance.get("requests", {})) if facade_provenance else 0
    verdict = "complete" if completed.returncode == 0 and completion["complete"] else "incomplete"
    receipt_id, receipt_dir = write_receipt(resolved, ts=ts, base_dir=base_dir, raw_dir=raw_dir, verdict=verdict,
                                            read="official LOCOMO FathomDB seam completed" if verdict == "complete" else "official LOCOMO FathomDB seam incomplete", completion=completion)
    return receipt_id, receipt_dir, 0 if verdict == "complete" else 1


def main(argv: list[str] | None = None) -> int:
    """Validate, print, or execute the FathomDB LOCOMO arm."""
    parser = argparse.ArgumentParser(description="FathomDB LOCOMO official-seam runner")
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("validate", "print-command", "run"):
        command = sub.add_parser(name)
        command.add_argument("config", type=Path)
    args = parser.parse_args(argv)
    try:
        config = resolve_config(json.loads(args.config.read_text(encoding="utf-8")))
        if args.command == "validate":
            print("fathomdb-locomo config resolves")
            return 0
        if args.command == "print-command":
            print(json.dumps(build_harness_command(config, run_id="preview", raw_dir=Path("external-output") / "preview")))
            return 0
        _, _, returncode = run(config, base_dir=_lib.EXPERIMENTS_DIR)
        return returncode
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as exc:
        print(f"fathomdb-locomo: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
