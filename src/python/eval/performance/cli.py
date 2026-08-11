"""Local CLI for independent performance evidence linked to an EARP run."""

from __future__ import annotations

import argparse
import sys
from datetime import datetime, timezone
from pathlib import Path

from eval.earp.config import resolve_config
from eval.performance.earp_adapter import (
    PerformancePlan,
    load_earp_workload,
    run_and_write_characterization_performance,
    run_and_write_diagnostic_performance,
)


def main(argv: list[str] | None = None) -> int:
    """Run a named independent performance campaign from EARP artifacts."""
    parser = argparse.ArgumentParser(
        prog="fathomdb-performance",
        description="Independent performance evidence linked to an EARP quality run",
    )
    subcommands = parser.add_subparsers(dest="command", required=True)
    for command, help_text in (
        ("diagnostic", "repeat a resolved EARP diagnostic workload"),
        ("characterization", "repeat a resolved EARP characterization workload"),
    ):
        subcommand = subcommands.add_parser(command, help=help_text)
        subcommand.add_argument("--experiments-root", required=True, type=Path)
        subcommand.add_argument("--quality-run", required=True)
        subcommand.add_argument("--repetitions", required=True, type=int)
        subcommand.add_argument(
            "--treatments",
            default="fresh_store,fresh_store_warm_query",
            help="comma-separated treatments (default: fresh_store,fresh_store_warm_query)",
        )
    args = parser.parse_args(argv)
    try:
        root = Path(args.experiments_root)
        workload = load_earp_workload(root, args.quality_run)
        config = dict(workload.resolved_config_document)
        resolution = resolve_config(config)
        if resolution.scenario is None or resolution.blockers:
            raise ValueError(f"quality config does not resolve: {list(resolution.blockers)}")
        if config.get("campaign") != args.command:
            raise ValueError(
                f"{args.command} runner requires an EARP {args.command} quality run"
            )
        treatments = tuple(item.strip() for item in args.treatments.split(",") if item.strip())
        predeclared = workload.predeclared_plan
        if predeclared.get("kind") != "descriptive_nonclaim" and (
            args.repetitions != predeclared.get("repetitions")
            or list(treatments) != predeclared.get("treatments")
        ):
            raise ValueError("requested repetitions/treatments do not match the predeclared plan")
        common = {
            "workload": workload,
            "plan": PerformancePlan(repetitions=args.repetitions, treatments=treatments),
            "scenario": resolution.scenario,
            "config_doc": config,
            "experiments_root": root,
            "experiment": f"earp-{args.command}-performance",
            "ts": datetime.now(timezone.utc).replace(second=0, microsecond=0),
        }
        if args.command == "diagnostic":
            outcome = run_and_write_diagnostic_performance(**common)
        else:
            outcome = run_and_write_characterization_performance(**common)
    except (OSError, ValueError, RuntimeError) as exc:
        print(f"fathomdb-performance: {exc}", file=sys.stderr)
        return 2
    print(f"performance run {outcome.run_id}")
    print(f"  artifact {outcome.run_dir}")
    print(f"  parent   {workload.parent_run_id}")
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
