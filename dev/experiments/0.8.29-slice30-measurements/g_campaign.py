"""G-only stage inventory; accepted P scheduling and results stay unchanged."""

from itertools import product

from campaign import run_campaign


def make_g_plan(spec):
    """Expand the reviewed fixed G stages and paired orders without executing models."""
    if (spec["schema"] != "slice30-g/v1" or spec["format"] != 37
            or spec["provider"] != "default" or spec["repeats"] != 2
            or spec["sizes"] != [10000, 100000] or spec["seed_batch"] != 1000
            or spec["adapters"] != ["engine", "sdk"]
            or spec["routes"] != ["text", "hybrid", "embed"]
            or (spec["warmup"], spec["samples"], spec["diagnostic_queries"]) != (500, 1000, 50)
            or spec["write_batch_sizes"] != [1, 10]
            or spec["write_search_ratios"] != [1, 10, 100]
            or spec["stage_order"] != ["G0", "G1", "G-R", "G-O", "G-W"]
            or spec["tiny_cases"] != ["direct_text", "hybrid_no_dependencies", "hybrid_kind_filter",
                                       "source_window", "commit_change", "no_match"]
            or spec["model"] != {"name": "fathomdb-bge-small-en-v1.5",
                                  "revision": "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
                                  "dimension": 384, "head_tokens": 512}
            or spec["limits"] != {"cell_seconds": 900, "campaign_seconds": 14400,
                                   "active_bytes": 16 * 1024**3}):
        raise ValueError("G inventory differs from the approved fixed continuation")
    common = dict(spec["limits"], provider="default", negative_control=False, format=37,
                  seed_batch=1000, observe=False, artifact="ordinary")
    plan = []
    def add(stage, identifier, **fields):
        plan.append(dict(common, stage=stage, id=identifier, name=identifier, **fields))
    add("G0", "g0-smoke", kind="smoke", n=2, adapters=["engine", "sdk"],
        routes=["text", "hybrid", "embed"], warmup=0, samples=16,
        record_first_call=False, fresh_seed=True, repeat=0)
    for size in [10000, 100000]:
        add("G1", f"g1-n{size}", kind="seed", n=size, adapter="sdk", fresh_seed=True,
            repeat=0, lower_subject="g1-n10000" if size == 100000 else None)
    for repeat in [0, 1]:
        adapters = ["engine", "sdk"] if repeat == 0 else ["sdk", "engine"]
        routes = ["text", "hybrid", "embed"] if repeat == 0 else ["embed", "hybrid", "text"]
        for size in ([10000, 100000] if repeat == 0 else [100000, 10000]):
            add("G-R", f"g-r-n{size}-r{repeat}", kind="read", n=size, repeat=repeat,
                adapters=adapters, routes=routes, warmup=500, samples=1000,
                record_first_call=True, fresh_seed=repeat == 1,
                seed_subject=f"g1-n{size}" if repeat == 0 else None)
    add("G-O", "g-o-branches", kind="branches", n=10, repeat=0,
        adapters=["engine", "sdk"], cases=list(spec["tiny_cases"]), fresh_seed=True,
        artifact="diagnostic", observe=True, warmup=0, record_first_call=False)
    for repeat, modes in [(0, [False, True]), (1, [True, False])]:
        for observed in modes:
            add("G-O", f"g-o-natural-r{repeat}-{'on' if observed else 'off'}", kind="natural",
                n=100000, repeat=repeat, artifact="diagnostic", observe=observed,
                adapters=["engine", "sdk"] if repeat == 0 else ["sdk", "engine"],
                routes=["text", "hybrid", "embed"] if repeat == 0 else ["embed", "hybrid", "text"],
                seed_subject="g1-n100000", fresh_seed=False, warmup=0, samples=50,
                record_first_call=False)
    combinations = list(product([1, 10], [1, 10, 100], ["engine", "sdk"],
                                ["text", "hybrid", "embed"]))
    for repeat in [0, 1]:
        for batch, ratio, adapter, route in (combinations if repeat == 0 else reversed(combinations)):
            add("G-W", f"g-w-b{batch}-s{ratio}-{adapter}-{route}-r{repeat}", kind="write",
                n=100000, repeat=repeat, batch_size=batch, searches_per_batch=ratio,
                adapters=[adapter], routes=[route], warmup=500, samples=1000,
                record_first_call=False, fresh_seed=True)
    return plan


def run_g_plan(spec, root, command):
    """Run a callable stage schedule with the existing owned-child attempt runner."""
    return run_campaign(make_g_plan(spec), root, command, timeout=spec["limits"]["cell_seconds"])
