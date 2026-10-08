#!/usr/bin/env python3
"""Independently audit paired source-labeled evidence-set retrieval."""

from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import importlib.util
import json
from pathlib import Path
import random
from typing import Any
import zipfile


def sha256(path: Path) -> str:
    """Hash one local campaign artifact."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _hash_text(value: str) -> str:
    return hashlib.sha256(value.encode()).hexdigest()


def _module(path: Path, name: str) -> Any:
    specification = importlib.util.spec_from_file_location(name, path)
    assert specification is not None and specification.loader is not None
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def paired_summary(scores: dict[str, dict[str, Any]], query_ids: list[str]) -> dict[str, Any]:
    """Compare complete paired cases and retain every coverage loss."""
    if (set(scores) != {"baseline", "candidate"} or not query_ids
            or len(set(query_ids)) != len(query_ids)):
        raise ValueError("incomplete paired scores or query IDs")
    left = scores["baseline"]["per_query"]
    right = scores["candidate"]["per_query"]
    if set(left) != set(query_ids) or set(right) != set(query_ids):
        raise ValueError("incomplete paired query set")
    result: dict[str, Any] = {"query_count": len(query_ids)}
    for k in (10, 20):
        key = f"complete_at_{k}"
        losses = []
        wins = []
        grouped: dict[str, list[int]] = defaultdict(list)
        for query_id in query_ids:
            lrow = left[query_id]
            rrow = right[query_id]
            if lrow["class"] != rrow["class"]:
                raise ValueError("paired class changed")
            delta = int(rrow[key]) - int(lrow[key])
            grouped[lrow["class"]].append(delta)
            if delta < 0:
                losses.append(query_id)
            elif delta > 0:
                wins.append(query_id)
        result[f"candidate_worse_at_{k}"] = losses
        result[f"candidate_better_at_{k}"] = wins
        result[f"coverage_delta_by_class_at_{k}"] = {
            name: sum(values) / len(values) for name, values in sorted(grouped.items())
        }
        rng = random.Random(135 + k)
        intervals = {}
        for name, values in sorted(grouped.items()):
            samples = sorted(sum(rng.choices(values, k=len(values))) / len(values)
                             for _ in range(10_000))
            intervals[name] = [samples[249], samples[9749]]
        result[f"coverage_delta_bootstrap_95pct_at_{k}"] = intervals
    return result


def _rebuild(data: dict[str, Any], source: Path, inputs_module: Any) -> None:
    if data["dataset"] == "musique":
        selection = data["selection"]
        quotas = {int(key): value for key, value in selection["quotas"].items()}
        rebuilt = inputs_module.build_musique(source, quotas=quotas, salt=selection["salt"])
    elif data["dataset"] == "locomo":
        rebuilt = inputs_module.build_locomo(source)
    else:
        raise ValueError("unknown support dataset")
    if json.loads(json.dumps(rebuilt)) != data:
        raise ValueError("derived input differs from source mapping")


def _audit_one(
    *, directory: Path, version: str, protocol: dict[str, Any], protocol_sha: str,
    wheel: Path, data: dict[str, Any], scorer: Any, ir_audit: Any,
) -> tuple[dict[str, Any], dict[str, Any]]:
    database = directory / "support.sqlite"
    raw = json.loads((directory / "raw.json").read_text())
    specification = protocol["versions"][version]
    if sha256(wheel) != specification["wheel_sha256"]:
        raise ValueError("wheel bytes changed")
    with zipfile.ZipFile(wheel) as archive:
        native_bytes = archive.read("fathomdb/_fathomdb.abi3.so")
        package_bytes = archive.read("fathomdb/__init__.py")
    if hashlib.sha256(native_bytes).hexdigest() != specification["native_sha256"]:
        raise ValueError("wheel native bytes changed")
    expected_documents = [{
        "logical_id": row["logical_id"],
        "body_sha256": _hash_text(json.dumps(
            {"summary": row["body"]}, ensure_ascii=False, separators=(",", ":")
        )),
    } for row in data["documents"]]
    required = {
        "schema_version": 1, "version": version, "dataset": protocol["dataset"],
        "source_sha": specification["source_sha"],
        "wheel_sha256": specification["wheel_sha256"],
        "native_sha256": specification["native_sha256"],
        "protocol_sha256": protocol_sha,
        "runner_sha256": protocol["runner_sha256"],
        "scorer_sha256": protocol["scorer_sha256"],
        "inputs_sha256": protocol["inputs_sha256"],
        "source_sha256": protocol["source"]["sha256"],
        "derived_sha256": protocol["derived"]["sha256"],
        "database_sha256": sha256(database),
        "model_name": protocol["model_name"],
        "documents": expected_documents,
    }
    for key, value in required.items():
        if raw.get(key) != value:
            raise ValueError(f"{version}: {key} receipt mismatch")
    identity = raw.get("identity")
    environment = raw.get("environment")
    if not isinstance(identity, dict) or not isinstance(environment, dict):
        raise ValueError("missing installed identity")
    native_path = Path(identity.get("native_path", ""))
    package_path = Path(identity.get("module_path", ""))
    python_path = Path(environment.get("python_executable", ""))
    if (not native_path.is_file() or not package_path.is_file() or not python_path.is_file()
            or sha256(native_path) != specification["native_sha256"]
            or sha256(package_path) != hashlib.sha256(package_bytes).hexdigest()):
        raise ValueError("installed package identity mismatch")
    observations = raw.get("queries")
    if not isinstance(observations, list) or len(observations) != len(data["queries"]):
        raise ValueError("incomplete query receipt")
    for expected, actual in zip(data["queries"], observations):
        if not isinstance(actual, dict):
            raise ValueError("malformed query receipt")
        for key in ("query_id", "query_class", "text", "required_ids"):
            if actual.get(key) != expected[key]:
                raise ValueError(f"{version}: query {key} mismatch")
        if (actual.get("text_sha256") != _hash_text(expected["text"])
                or actual.get("strict_multi_session")
                != expected.get("strict_multi_session", False)):
            raise ValueError("query identity mismatch")
    expected_rows = [(
        row["logical_id"], "doc",
        json.dumps({"summary": row["body"]}, ensure_ascii=False, separators=(",", ":")),
        row["source_id"],
    ) for row in data["documents"]]
    database_result = ir_audit.audit_database(database, expected_rows, vector_count=len(expected_rows))
    scores = scorer.score_version(
        data["queries"], observations,
        {row["logical_id"] for row in data["documents"]}, limit=protocol["result_limit"],
    )
    return scores, database_result


def audit(
    *, protocol_path: Path, source: Path, derived: Path, baseline_wheel: Path,
    candidate_wheel: Path, baseline_dir: Path, candidate_dir: Path,
) -> dict[str, Any]:
    """Rebuild source mapping, inspect both databases and recompute scores."""
    protocol_bytes = protocol_path.read_bytes()
    protocol = json.loads(protocol_bytes)
    if (protocol.get("schema_version") != 1 or set(protocol.get("versions", {}))
            != {"baseline", "candidate"} or protocol.get("result_limit") != 20):
        raise ValueError("protocol identity mismatch")
    scripts = Path(__file__).resolve().parent
    paths = {
        "runner_sha256": scripts / "slice135_phase2_support_runner.py",
        "scorer_sha256": scripts / "slice135_phase2_support_scorer.py",
        "inputs_sha256": scripts / "slice135_phase2_support_inputs.py",
        "ir_runner_sha256": scripts / "slice135_phase2_ir_runner.py",
        "ir_audit_sha256": scripts / "slice135_phase2_ir_campaign_audit.py",
        "locomo_loader_sha256": scripts.parent / "src/python/eval/locomo_loader.py",
        "audit_sha256": Path(__file__).resolve(),
    }
    for key, path in paths.items():
        if sha256(path) != protocol.get(key):
            raise ValueError(f"frozen {key} mismatch")
    if (sha256(source) != protocol["source"]["sha256"]
            or sha256(derived) != protocol["derived"]["sha256"]):
        raise ValueError("source or derived input changed")
    data = json.loads(derived.read_text())
    if data.get("dataset") != protocol["dataset"]:
        raise ValueError("dataset identity changed")
    runner = _module(paths["runner_sha256"], "slice135_support_runner")
    runner.validate_inputs(data, protocol["denominator"])
    inputs = _module(paths["inputs_sha256"], "slice135_support_inputs")
    _rebuild(data, source, inputs)
    scorer = _module(paths["scorer_sha256"], "slice135_support_scorer")
    ir_audit = _module(paths["ir_audit_sha256"], "slice135_ir_audit")
    scores = {}
    databases = {}
    for version, wheel, directory in (
        ("baseline", baseline_wheel, baseline_dir),
        ("candidate", candidate_wheel, candidate_dir),
    ):
        scores[version], databases[version] = _audit_one(
            directory=directory, version=version, protocol=protocol,
            protocol_sha=hashlib.sha256(protocol_bytes).hexdigest(),
            wheel=wheel, data=data, scorer=scorer, ir_audit=ir_audit,
        )
    return {
        "protocol_sha256": hashlib.sha256(protocol_bytes).hexdigest(),
        "databases": databases,
        "scores": scores,
        "paired": paired_summary(scores, [q["query_id"] for q in data["queries"]]),
        "denominator": {"documents": len(data["documents"]), "queries": len(data["queries"]),
                        "by_class": dict(Counter(q["query_class"] for q in data["queries"]))},
    }


def main() -> None:
    """Audit one exact-identity paired support-set campaign."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", required=True, type=Path)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--derived", required=True, type=Path)
    parser.add_argument("--baseline-wheel", required=True, type=Path)
    parser.add_argument("--candidate-wheel", required=True, type=Path)
    parser.add_argument("--baseline-dir", required=True, type=Path)
    parser.add_argument("--candidate-dir", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(audit(
        protocol_path=args.protocol, source=args.source, derived=args.derived,
        baseline_wheel=args.baseline_wheel, candidate_wheel=args.candidate_wheel,
        baseline_dir=args.baseline_dir, candidate_dir=args.candidate_dir,
    ), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
