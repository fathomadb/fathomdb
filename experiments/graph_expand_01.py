"""GRAPH-EXPAND-01 independent oracle, generator, and strict contract."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import random
import resource
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Literal

from experiments import graph_benchmarks
from experiments.fathomdb_test_setup import prepare_test_database


SCHEMA_VERSION = "graph-expand-01.config.v1"
PROGRAM_TRACK = "GRAPH-EXPAND-01"
Direction = Literal["outgoing", "incoming", "both"]
_ROOT_KEYS = {
    "schema_version",
    "program_track",
    "cell_id",
    "claim_boundary",
    "generator",
    "scales",
    "query_seeds",
    "cells",
    "measurement",
    "outputs",
}
_CELL_KEYS = {
    "id",
    "scale",
    "seed",
    "direction",
    "depth",
    "degree",
    "filter",
    "context",
    "explain",
    "evidence",
    "concurrency",
    "bound",
}
_CELL_IDS = (
    "base",
    "seed-e4",
    "seed-e16",
    "seed-q1",
    "seed-q5",
    "seed-q10",
    "direction-in",
    "direction-both",
    "depth-0",
    "depth-2",
    "degree-low",
    "degree-high",
    "filter-edge",
    "filter-target",
    "filter-eligibility",
    "context-frozen",
    "explanation",
    "evidence",
    "concurrency-2",
    "concurrency-4",
    "concurrency-8",
    "bound-below",
    "bound-exact",
    "bound-above",
    "scale-small",
    "scale-medium",
    "scale-large",
)


class GraphExpandBenchmarkError(ValueError):
    """Report an invalid traversal benchmark contract or oracle input."""


@dataclass(frozen=True)
class Config:
    """Resolved GRAPH-EXPAND-01 execution contract."""

    program_track: str
    cell_id: str
    generator: dict[str, object]
    scales: tuple[dict[str, object], ...]
    query_seeds: tuple[dict[str, object], ...]
    cells: tuple[dict[str, object], ...]
    measurement: dict[str, object]
    resolved: dict[str, object]


@dataclass(frozen=True)
class Edge:
    """One raw graph edge row visible to the independent oracle."""

    logical_id: str
    kind: str
    from_id: str
    to_id: str
    write_cursor: int
    active: bool = True


@dataclass(frozen=True)
class OracleTarget:
    """One ordered target and the winning public origin identity."""

    logical_id: str
    hop_count: int
    seed_ordinal: int
    predecessor_logical_id: str
    terminal_direction: str
    edge_kind: str
    edge_logical_id: str

    def order_key(self) -> tuple[object, ...]:
        """Return the public target/origin ordering key."""
        return (
            self.hop_count,
            self.seed_ordinal,
            self.predecessor_logical_id,
            0 if self.terminal_direction == "outgoing" else 1,
            self.edge_kind,
            self.logical_id,
        )


@dataclass(frozen=True)
class OracleResult:
    """Independent bounded traversal result."""

    seeds: tuple[str, ...]
    targets: tuple[OracleTarget, ...]
    work_units: int


def _exact(value: object, label: str, keys: set[str]) -> dict[str, Any]:
    try:
        return graph_benchmarks.exact_mapping(value, label, keys)
    except graph_benchmarks.GraphBenchmarkError as error:
        raise GraphExpandBenchmarkError(str(error)) from error


def resolve_config(document: object) -> Config:
    """Strictly validate and resolve the frozen traversal configuration."""
    root = _exact(document, "config", _ROOT_KEYS)
    if (
        root["schema_version"] != SCHEMA_VERSION
        or root["program_track"] != PROGRAM_TRACK
    ):
        raise GraphExpandBenchmarkError("configuration identity is invalid")
    if root["cell_id"] != "graph-expand01":
        raise GraphExpandBenchmarkError("cell_id is invalid")
    if root["claim_boundary"] != "linkbench_ldbc_inspired_not_official_compliance":
        raise GraphExpandBenchmarkError("claim boundary drifted")
    generator = _exact(root["generator"], "generator", {"algorithm", "seed"})
    if generator != {"algorithm": "graph-expand-powerlaw-v1", "seed": 20260921}:
        raise GraphExpandBenchmarkError("generator identity drifted")
    scale_value = root["scales"]
    if not isinstance(scale_value, list):
        raise GraphExpandBenchmarkError("scales must be a list")
    scales = tuple(
        _exact(item, "scale", {"id", "nodes", "edges"}) for item in scale_value
    )
    expected_scales = (
        ("smoke", 1_000, 5_000),
        ("small", 10_000, 50_000),
        ("medium", 50_000, 1_000_000),
        ("large", 100_000, 5_000_000),
    )
    if (
        tuple((item["id"], item["nodes"], item["edges"]) for item in scales)
        != expected_scales
    ):
        raise GraphExpandBenchmarkError("scale identity drifted")
    query_value = root["query_seeds"]
    if not isinstance(query_value, list) or len(query_value) != 3:
        raise GraphExpandBenchmarkError("query seed manifest drifted")
    query_seeds = tuple(
        _exact(item, "query seed", {"id", "text", "expected_ids"})
        for item in query_value
    )
    expected_query_counts = {"Q1": 1, "Q5": 5, "Q10": 10}
    for item in query_seeds:
        expected_count = expected_query_counts.get(item["id"])
        ids = item["expected_ids"]
        if (
            expected_count is None
            or not isinstance(item["text"], str)
            or not isinstance(ids, list)
            or len(ids) != expected_count
            or len(ids) != len(set(ids))
        ):
            raise GraphExpandBenchmarkError("query seed manifest drifted")
    cells_value = root["cells"]
    if not isinstance(cells_value, list):
        raise GraphExpandBenchmarkError("cells must be a list")
    cells = tuple(_exact(item, "cell", _CELL_KEYS) for item in cells_value)
    if tuple(item["id"] for item in cells) != _CELL_IDS:
        raise GraphExpandBenchmarkError("registered cell identity drifted")
    for cell in cells:
        if cell["depth"] not in {0, 1, 2}:
            raise GraphExpandBenchmarkError("cell depth must be 0, 1, or 2")
        if cell["direction"] not in {"outgoing", "incoming", "both"}:
            raise GraphExpandBenchmarkError("cell direction is invalid")
        if cell["evidence"] and cell["context"] != "F":
            raise GraphExpandBenchmarkError("evidence requires a frozen context")
    measurement = _exact(
        root["measurement"],
        "measurement",
        {
            "warmup_operations",
            "smoke_operations",
            "scale_operations",
            "repetitions",
            "loop_model",
            "percentile_estimator",
        },
    )
    if measurement != {
        "warmup_operations": 5,
        "smoke_operations": 100,
        "scale_operations": 1000,
        "repetitions": 3,
        "loop_model": "closed",
        "percentile_estimator": "linear-p-n-minus-1",
    }:
        raise GraphExpandBenchmarkError("measurement identity drifted")
    outputs = _exact(
        root["outputs"], "outputs", {"correctness_section", "performance_section"}
    )
    if outputs != {
        "correctness_section": "correctness",
        "performance_section": "performance",
    }:
        raise GraphExpandBenchmarkError("output sections drifted")
    return Config(
        program_track=PROGRAM_TRACK,
        cell_id="graph-expand01",
        generator=dict(generator),
        scales=tuple(dict(item) for item in scales),
        query_seeds=tuple(dict(item) for item in query_seeds),
        cells=tuple(dict(item) for item in cells),
        measurement=dict(measurement),
        resolved=dict(root),
    )


def load_config(path: str | Path) -> Config:
    """Load and validate one checked-in traversal configuration."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GraphExpandBenchmarkError(
            "configuration is unavailable or invalid"
        ) from error
    return resolve_config(document)


def validate_query_seeds(expected: tuple[str, ...], actual: tuple[str, ...]) -> None:
    """Require native query-derived seed order to match the manifest oracle."""
    if actual != expected:
        raise GraphExpandBenchmarkError(
            f"query seeds mismatch: expected={expected!r}, actual={actual!r}"
        )


def _incident(edge: Edge, current: str, direction: Direction) -> tuple[str, str] | None:
    if direction in {"outgoing", "both"} and edge.from_id == current:
        return "outgoing", edge.to_id
    if direction in {"incoming", "both"} and edge.to_id == current:
        return "incoming", edge.from_id
    return None


def bfs_oracle(
    *,
    nodes: dict[str, dict[str, object]],
    edges: list[Edge],
    seeds: tuple[str, ...],
    direction: Direction,
    max_depth: int,
    result_limit: int,
    edge_kinds: tuple[str, ...] = (),
    target_kinds: tuple[str, ...] = (),
    eligible: Callable[[str], bool] | None = None,
) -> OracleResult:
    """Compute exact bounded traversal and raw incident-edge work independently."""
    if direction not in {"outgoing", "incoming", "both"}:
        raise GraphExpandBenchmarkError("direction is invalid")
    if not 0 <= max_depth <= 2 or result_limit <= 0:
        raise GraphExpandBenchmarkError("oracle bounds are invalid")
    if len(seeds) != len(set(seeds)) or any(seed not in nodes for seed in seeds):
        raise GraphExpandBenchmarkError("oracle seeds are invalid")
    eligibility = eligible or (lambda logical_id: logical_id in nodes)
    seed_ids = set(seeds)
    visited = [set((seed,)) for seed in seeds]
    frontier = [(index, seed) for index, seed in enumerate(seeds)]
    candidates: dict[str, OracleTarget] = {}
    work_units = 0
    for depth in range(max_depth):
        frontier.sort(key=lambda value: (value[0], value[1]))
        next_frontier: list[tuple[int, str]] = []
        for seed_ordinal, current in frontier:
            incident: list[tuple[int, str, str, str, int, Edge, str]] = []
            for edge in edges:
                route = _incident(edge, current, direction)
                if route is None:
                    continue
                terminal_direction, next_id = route
                incident.append(
                    (
                        0 if terminal_direction == "outgoing" else 1,
                        edge.kind,
                        next_id,
                        edge.logical_id,
                        edge.write_cursor,
                        edge,
                        terminal_direction,
                    )
                )
            incident.sort(key=lambda value: value[:5])
            work_units += len(incident)
            for _, _, next_id, _, _, edge, terminal_direction in incident:
                if edge_kinds and edge.kind not in edge_kinds:
                    continue
                if not edge.active:
                    continue
                if depth + 1 < max_depth and next_id in visited[seed_ordinal]:
                    continue
                if depth + 1 < max_depth:
                    visited[seed_ordinal].add(next_id)
                node = nodes.get(next_id)
                if node is None or not eligibility(next_id):
                    continue
                if depth + 1 < max_depth:
                    next_frontier.append((seed_ordinal, next_id))
                if next_id in seed_ids:
                    continue
                kind = node.get("kind")
                if target_kinds and kind not in target_kinds:
                    continue
                target = OracleTarget(
                    logical_id=next_id,
                    hop_count=depth + 1,
                    seed_ordinal=seed_ordinal,
                    predecessor_logical_id=current,
                    terminal_direction=terminal_direction,
                    edge_kind=edge.kind,
                    edge_logical_id=edge.logical_id,
                )
                existing = candidates.get(next_id)
                if existing is None or target.order_key() < existing.order_key():
                    candidates[next_id] = target
        frontier = next_frontier
    targets = tuple(
        sorted(candidates.values(), key=OracleTarget.order_key)[:result_limit]
    )
    return OracleResult(seeds=seeds, targets=targets, work_units=work_units)


def generate_graph(*, node_count: int, edge_count: int, seed: int) -> dict[str, object]:
    """Generate an exact-size deterministic directed power-law-like graph."""
    if node_count < 30 or edge_count <= 0:
        raise GraphExpandBenchmarkError("generator sizes are invalid")
    rng = random.Random(seed)
    nodes = []
    for index in range(node_count):
        tokens = []
        if index == 1:
            tokens.append("graphseed-one")
        if 10 <= index <= 14:
            tokens.append("graphseed-five")
        if 20 <= index <= 29:
            tokens.append("graphseed-ten")
        nodes.append(
            {
                "logical_id": f"node-{index:06d}",
                "kind": "seed" if tokens else "fact",
                "body": " ".join(tokens) or f"graph node {index}",
            }
        )
    edges: list[dict[str, object]] = []
    while len(edges) < edge_count:
        source = int(node_count * (rng.random() ** 2.5))
        target = rng.randrange(node_count)
        if source == target:
            continue
        ordinal = len(edges)
        edges.append(
            {
                "logical_id": f"edge-{ordinal:09d}",
                "kind": ("supports", "related", "cites")[ordinal % 3],
                "from": f"node-{source:06d}",
                "to": f"node-{target:06d}",
                "write_cursor": ordinal + 1,
            }
        )
    adjacency = [
        (edge["logical_id"], edge["kind"], edge["from"], edge["to"]) for edge in edges
    ]
    return {
        "schema_version": "graph-expand-01.generated-graph.v1",
        "algorithm": "graph-expand-powerlaw-v1",
        "seed": seed,
        "nodes": nodes,
        "edges": edges,
        "adjacency_sha256": graph_benchmarks.canonical_sha256(adjacency),
    }


def bound_budgets(work_units: int) -> dict[str, int]:
    """Return only valid public request budgets around oracle work ``W``."""
    if not 0 <= work_units <= 10_000:
        raise GraphExpandBenchmarkError("work units are outside the public range")
    if work_units == 0:
        return {"minimum_valid": 1}
    result: dict[str, int] = {}
    if work_units > 1:
        result["below"] = work_units - 1
    result["exact"] = work_units
    if work_units < 10_000:
        result["above"] = work_units + 1
    return result


def run_smoke(
    config: Config, artifact_root: Path, *, config_path: Path
) -> dict[str, object]:
    """Load the smoke graph and verify native traversal against the oracle."""
    import fathomdb

    scale = config.scales[0]
    generated = generate_graph(
        node_count=int(scale["nodes"]),
        edge_count=int(scale["edges"]),
        seed=int(config.generator["seed"]),
    )
    manifest = artifact_root / "generated-graph.v1.json"
    manifest.write_text(json.dumps(generated, sort_keys=True) + "\n", encoding="utf-8")
    fathomdb_bin = os.environ.get(
        "FATHOMDB_BIN",
        str(Path(__file__).resolve().parent.parent / "target/debug/fathomdb"),
    )
    prepared = prepare_test_database(
        artifact_root,
        test_id="graph-expand01",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=fathomdb_bin,
    )
    nodes = {
        str(item["logical_id"]): {"kind": item["kind"]} for item in generated["nodes"]
    }
    edges = [
        Edge(
            logical_id=str(item["logical_id"]),
            kind=str(item["kind"]),
            from_id=str(item["from"]),
            to_id=str(item["to"]),
            write_cursor=int(item["write_cursor"]),
        )
        for item in generated["edges"]
    ]
    seed = "node-000001"
    oracle = bfs_oracle(
        nodes=nodes,
        edges=edges,
        seeds=(seed,),
        direction="outgoing",
        max_depth=1,
        result_limit=50,
    )
    if oracle.work_units > 10_000:
        raise GraphExpandBenchmarkError("smoke oracle exceeds the public work bound")

    engine = fathomdb.Engine.open(
        str(prepared.database_path), use_default_embedder=False
    )
    try:
        writes = [
            {
                "kind": item["kind"],
                "body": item["body"],
                "source_id": "graph-expand-01",
                "logical_id": item["logical_id"],
            }
            for item in generated["nodes"]
        ]
        writes.extend(
            {
                "edge": {
                    "kind": item["kind"],
                    "from": item["from"],
                    "to": item["to"],
                    "source_id": "graph-expand-01",
                    "logical_id": item["logical_id"],
                }
            }
            for item in generated["edges"]
        )
        engine.write(writes)
        engine.drain(timeout_s=60)

        query = fathomdb.graph.expand(
            engine,
            fathomdb.GraphExpandRequestV1(
                schema_version=1,
                seed=fathomdb.GraphQuerySeedV1(
                    schema_version=1,
                    type="query",
                    text="graphseed-one",
                    ranked_limit=1,
                ),
                direction="outgoing",
                edge_kinds=(),
                target_kinds=(),
                context=fathomdb.CurrentGraphReadContextV1(
                    schema_version=1,
                    type="current",
                    context=fathomdb.ReadContextV1(),
                ),
                max_depth=0,
                result_limit=1,
                max_work_units="1",
                include_explanation=False,
            ),
        )
        validate_query_seeds(
            ("node-000001",), tuple(item.logical_id for item in query.seeds)
        )

        request = fathomdb.GraphExpandRequestV1(
            schema_version=1,
            seed=fathomdb.GraphExplicitSeedV1(
                schema_version=1,
                type="explicit",
                logical_ids=(fathomdb.IdSpace(space="logical", value=seed),),
            ),
            direction="outgoing",
            edge_kinds=(),
            target_kinds=(),
            context=fathomdb.CurrentGraphReadContextV1(
                schema_version=1,
                type="current",
                context=fathomdb.ReadContextV1(),
            ),
            max_depth=1,
            result_limit=50,
            max_work_units=str(max(1, oracle.work_units)),
            include_explanation=False,
        )
        budgets = bound_budgets(oracle.work_units)
        if "below" in budgets:
            below = fathomdb.GraphExpandRequestV1(
                **{**request.__dict__, "max_work_units": str(budgets["below"])}
            )
            try:
                fathomdb.graph.expand(engine, below)
            except fathomdb.GraphExpansionError as error:
                if error.reason != "graph_expansion_bound_exceeded":
                    raise
            else:
                raise GraphExpandBenchmarkError("W-1 did not refuse atomically")
        for key in ("exact", "above"):
            if key not in budgets:
                continue
            probe = fathomdb.GraphExpandRequestV1(
                **{**request.__dict__, "max_work_units": str(budgets[key])}
            )
            if (
                int(fathomdb.graph.expand(engine, probe).work_units)
                != oracle.work_units
            ):
                raise GraphExpandBenchmarkError(f"{key} work-bound probe drifted")
        for _ in range(int(config.measurement["warmup_operations"])):
            fathomdb.graph.expand(engine, request)
        latencies = []
        native = None
        for _ in range(int(config.measurement["smoke_operations"])):
            started = time.perf_counter_ns()
            native = fathomdb.graph.expand(engine, request)
            latencies.append((time.perf_counter_ns() - started) / 1_000_000)
        assert native is not None
        actual = [
            (
                item.logical_id,
                item.origin.hop_count,
                item.origin.seed_ordinal,
                item.origin.predecessor_logical_id,
                item.origin.terminal_direction,
                item.origin.terminal_edge_kind,
            )
            for item in native.targets
        ]
        expected = [
            (
                item.logical_id,
                item.hop_count,
                item.seed_ordinal,
                item.predecessor_logical_id,
                item.terminal_direction,
                item.edge_kind,
            )
            for item in oracle.targets
        ]
        if actual != expected or int(native.work_units) != oracle.work_units:
            raise GraphExpandBenchmarkError(
                "native traversal disagrees with the oracle"
            )
        concurrency: dict[str, object] = {}
        for workers in (2, 4, 8):
            operations = workers * 4
            started = time.perf_counter_ns()
            with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
                results = list(
                    pool.map(
                        lambda _: fathomdb.graph.expand(engine, request),
                        range(operations),
                    )
                )
            elapsed_s = (time.perf_counter_ns() - started) / 1_000_000_000
            if any(int(item.work_units) != oracle.work_units for item in results):
                raise GraphExpandBenchmarkError("concurrent correctness drifted")
            concurrency[str(workers)] = {
                "operations": operations,
                "throughput_ops_s": operations / elapsed_s,
            }
    finally:
        engine.close()
    performance = {
        "latency_ms": {
            "p50": graph_benchmarks.percentile(latencies, 0.50),
            "p95": graph_benchmarks.percentile(latencies, 0.95),
            "p99": graph_benchmarks.percentile(latencies, 0.99),
        },
        "operations": len(latencies),
        "throughput_ops_s": len(latencies) / (sum(latencies) / 1000.0),
        "work_units_per_operation": oracle.work_units,
        "concurrency": concurrency,
        "database_bytes": prepared.database_path.stat().st_size,
        "max_rss_kib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
    }
    metrics = {
        "correctness": {
            "target_order_exact": True,
            "origin_exact": True,
            "work_units_exact": True,
            "work_bound_all_or_nothing": True,
            "query_seed_exact": True,
            "targets": len(oracle.targets),
            "work_units": oracle.work_units,
        },
        "performance": performance,
        "denominators": graph_benchmarks.reconcile_denominators(
            attempted=len(latencies),
            completed=len(latencies),
            errors=0,
            typed_refusals=0,
        ),
    }
    record = graph_benchmarks.write_benchmark_record(
        experiment=config.cell_id,
        config=config.resolved,
        metrics=metrics,
        base_dir=artifact_root,
        config_path=config_path,
        corpus_manifest_sha256=str(generated["adjacency_sha256"]),
        datasets=["graph-expand-powerlaw-v1-smoke"],
    )
    return {
        "state": "complete",
        "manifest": str(manifest),
        "metrics": metrics,
        "record": str(record),
    }


def main(argv: list[str] | None = None) -> int:
    """Validate or generate the deterministic smoke workload."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("validate", "dry-run", "smoke"))
    parser.add_argument("config", type=Path)
    parser.add_argument("artifact_root", type=Path, nargs="?")
    args = parser.parse_args(argv)
    config = load_config(args.config)
    if args.command == "smoke":
        if args.artifact_root is None or args.artifact_root.exists():
            raise GraphExpandBenchmarkError("smoke artifact root must be new")
        args.artifact_root.mkdir(parents=True)
        print(
            json.dumps(
                run_smoke(config, args.artifact_root, config_path=args.config),
                sort_keys=True,
            )
        )
    else:
        if args.artifact_root is not None and args.artifact_root.exists():
            raise GraphExpandBenchmarkError("artifact root must be new")
        print(json.dumps({"state": "ready", "cell": config.cell_id}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
