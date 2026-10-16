"""Typed configs and ``experiments.record.v1`` receipts for COMP-01/02 campaigns.

A ``comp-fts.campaign.v1`` config pins one campaign's inputs by digest, its
arms, roster, run command, and report kind. ``comp_fts run --config`` verifies
the inputs before running; ``record`` re-verifies them, summarizes the cell
outputs, and writes the receipt plus index row.

    python -m experiments.comp_record verify CONFIG --inputs DIR
    python -m experiments.comp_record record CONFIG --inputs DIR --out DIR [--base-dir DIR]
"""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from experiments import _lib, comp02_report, comp_fts

CONFIG_KEYS = {
    "schema_version", "program_track", "campaign", "harness_campaign", "title", "inputs", "arms",
    "roster", "run", "report", "metrics", "result_note", "claim_boundary",
}
REPORTS = {"comp01": "fts", "comp02a": "scale", "comp02b": "filtered"}


class CompRecordError(RuntimeError):
    pass


def load_config(path: Path) -> dict[str, Any]:
    config = json.loads(Path(path).read_text(encoding="utf-8"))
    if set(config) != CONFIG_KEYS:
        raise CompRecordError(f"config keys mismatch: {sorted(set(config) ^ CONFIG_KEYS)}")
    if config["schema_version"] != "comp-fts.campaign.v1":
        raise CompRecordError("unsupported config schema_version")
    if REPORTS.get(config["harness_campaign"]) != config["report"]:
        raise CompRecordError("harness_campaign and report disagree")
    return config


def verify_inputs(config: dict[str, Any], inputs: Path) -> None:
    for name, expected in sorted(config["inputs"]["files"].items()):
        actual = hashlib.sha256((inputs / name).read_bytes()).hexdigest()
        if actual != expected:
            raise CompRecordError(f"{name} digest {actual[:12]} does not match config {expected[:12]}")


def summarize(config: dict[str, Any], inputs: Path, out: Path) -> dict[str, Any]:
    manifest = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
    if manifest["campaign"] != config["harness_campaign"]:
        raise CompRecordError("output manifest belongs to another campaign")
    done = {a["cell"] for a in manifest["attempts"] if a["status"] == "complete"}
    metrics: dict[str, Any] = {
        "roster": manifest["roster"],
        "cells_complete": sorted(done),
        "host": manifest["host"],
    }
    if config["report"] == "fts":
        report = comp_fts.score(out, inputs)
        metrics.update({"median_by_arm": report["median_by_arm"], "cells": report["cells"],
                        "affected_queries": report["affected_queries"]})
    elif config["report"] == "scale":
        metrics["medians"] = comp02_report.medians(comp02_report.scale_rows(out), ("documents", "arm"))
    else:
        metrics["medians"] = comp02_report.medians(comp02_report.filtered_rows(out), ("cohort", "arm"))
    cells = sorted((out / "cells").glob("*.json"))
    first = json.loads(cells[0].read_text(encoding="utf-8")) if cells else {}
    metrics["versions"] = {
        json.loads(p.read_text(encoding="utf-8"))["arm"]: json.loads(p.read_text(encoding="utf-8"))["version"]
        for p in cells
    }
    metrics["documents"] = first.get("documents")
    return metrics


def _headline(config: dict[str, Any], metrics: dict[str, Any]) -> dict[str, Any]:
    if config["report"] == "fts":
        return {arm: {k: round(v[k], 4) for k in ("ndcg_at_10", "p50_ms", "p95_ms")}
                for arm, v in metrics["median_by_arm"].items()}
    key = "steady_p50_ms" if config["report"] == "scale" else "p50_ms"
    group = "documents" if config["report"] == "scale" else "cohort"
    return {f"{row['arm']}@{row[group]}": round(row[key], 3) for row in metrics["medians"]}


def record(config: dict[str, Any], *, inputs: Path, out: Path, base_dir: Path | None,
           config_path: str | None) -> tuple[str, Path]:
    verify_inputs(config, inputs)
    metrics = summarize(config, inputs, out)
    base = base_dir if base_dir is not None else _lib.EXPERIMENTS_DIR
    git = _lib.git_info()
    complete = set(metrics["cells_complete"]) == set(metrics["roster"])
    artifacts = [{"kind": "manifest", "sha256": hashlib.sha256((out / "manifest.json").read_bytes()).hexdigest()}]
    artifacts += [{"kind": "cell", "name": p.stem, "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
                  for p in sorted((out / "cells").glob("*.json"))]
    run_id, run_dir = _lib.write_record(
        f"{config['campaign'].lower()}-{config['harness_campaign']}-{metrics['documents']}",
        ts=datetime.now(UTC),
        config_obj=config,
        metrics=metrics,
        verdict="complete" if complete else "partial",
        read=f"{config['title']}; result note {config['result_note']}",
        title=config["title"],
        headline=_headline(config, metrics),
        n=metrics["documents"],
        code={"git_sha": git.get("git_sha"), "dirty": git.get("dirty"), "branch": git.get("branch"),
              "baseline_commit": None},
        corpus={"source": config["inputs"]["source"], "manifest_sha256": config["inputs"]["files"].get("manifest.json"),
                "datasets": [{"name": k, "sha256": v} for k, v in sorted(config["inputs"]["files"].items())]},
        seeds={"steady_order": "1000 + repetition"},
        env=_lib.env_info(key_deps=metrics["versions"]),
        cost_usd=0.0,
        config_path=config_path,
        artifacts=artifacts,
        base_dir=base,
    )
    _lib.regen_index_md(index_path=base / "index.jsonl", md_path=base / "INDEX.md")
    return run_id, run_dir


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("verify", "record"):
        cmd = sub.add_parser(name)
        cmd.add_argument("config", type=Path)
        cmd.add_argument("--inputs", type=Path, required=True)
        if name == "record":
            cmd.add_argument("--out", type=Path, required=True)
            cmd.add_argument("--base-dir", type=Path)
    args = parser.parse_args(argv)
    config = load_config(args.config)
    if args.command == "verify":
        verify_inputs(config, args.inputs)
        print("inputs match config")
        return 0
    run_id, run_dir = record(config, inputs=args.inputs, out=args.out, base_dir=args.base_dir,
                             config_path=str(args.config))
    print(json.dumps({"run_id": run_id, "receipt": str(run_dir / "record.json")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
