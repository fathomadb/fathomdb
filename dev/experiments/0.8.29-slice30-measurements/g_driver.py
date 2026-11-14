"""Bind G-only table cells to current normal measurement-program commands."""

import json
from pathlib import Path
import subprocess
import sqlite3
import time

from g_span_coverage import stored_coverage


def run_program_step(command, config, output, deadline_ns, observe=None):
    """Retain one normal-program step under the same absolute parent cell deadline."""
    started = time.monotonic_ns()
    observation_wall_ns = observation_parent_cpu_ns = 0
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    path = output.with_name(output.name + "-native-config.json")
    config = dict(config, remaining_cell_ms=max(0, (deadline_ns - time.monotonic_ns()) // 10**6))
    path.write_text(json.dumps(config, indent=2) + "\n")
    step = {"family": config["family"], "config": str(path), "output": str(output),
            "complete": False, "exit": None, "result": None, "gpu_samples": []}
    # Short controls may exit before one observation; observer latency adds to the wait.
    short_control = (config.get("stage") == "G0" or config.get("calibration") is True
                     or (config.get("stage") == "G-O" and config.get("kind") == "branches"))
    observation_wait = 0.05 if observe and short_control else 1
    def collect(pid):
        nonlocal observation_wall_ns, observation_parent_cpu_ns
        wall, cpu = time.monotonic_ns(), time.process_time_ns()
        try:
            step["gpu_samples"].append(observe(pid))
        finally:
            observation_wall_ns += time.monotonic_ns() - wall
            observation_parent_cpu_ns += time.process_time_ns() - cpu
    def finish(value):
        return dict(value, host_wall_ns=time.monotonic_ns() - started,
                    observation_wall_ns=observation_wall_ns,
                    observation_parent_cpu_ns=observation_parent_cpu_ns,
                    observation_count=len(step["gpu_samples"]),
                    observation_raw_bytes=len(json.dumps(step["gpu_samples"]).encode()),
                    observation_wait_ms=int(observation_wait * 1000),
                    observation_scope="callback host wall and parent-process CPU only; sample bytes are compact JSON payload, total retained output measured separately; not exclusive GPU work")
    remaining = (deadline_ns - time.monotonic_ns()) / 10**9
    if config["remaining_cell_ms"] == 0 or remaining <= 0:
        return finish(dict(step, status="not_run_cap", reason="cell_time_exhausted"))
    with output.with_name(output.name + "-native.log").open("wb") as log:
        try:
            child = subprocess.Popen([*command, str(path), str(output)], stdout=log,
                                     stderr=subprocess.STDOUT)
            step["pid"] = child.pid
            step["executable"] = str(Path(command[0]).resolve())
            if observe:
                collect(child.pid)
            while True:
                remaining = (deadline_ns - time.monotonic_ns()) / 10**9
                if remaining <= 0:
                    raise subprocess.TimeoutExpired(command, 0)
                try:
                    child.wait(timeout=min(observation_wait, remaining))
                    break
                except subprocess.TimeoutExpired:
                    if observe:
                        collect(child.pid)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=2)
            return finish(dict(step, status="failed", reason="cell_time_exhausted", exit=child.returncode))
    step["exit"] = child.returncode
    try:
        step["result"] = json.loads((output / "result.json").read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return finish(dict(step, status="failed", reason="missing_or_partial_result"))
    if config["family"] in ("G-seed", "G-source-fixture") and not step["result"].get("runner_only"):
        try:
            step["span_coverage"] = stored_coverage(output / "store.sqlite")
        except (OSError, sqlite3.Error) as error:
            return finish(dict(step, status="failed", reason="stored_span_coverage_unavailable", error=str(error)))
    step["complete"] = (child.returncode == 0 and step["result"].get("complete") is True
                        and time.monotonic_ns() < deadline_ns)
    return finish(dict(step, status="complete" if step["complete"] else "failed"))



def execute_branches(cell, manifest, attempt, deadline_ns):
    """Bind six synthetic diagnostics with isolated source/commit subjects and one cap."""
    corpus, queries = attempt / "branch-corpus.jsonl", attempt / "branch-queries.jsonl"
    corpus.write_text("".join(json.dumps(f"needle body{index}") + "\n" for index in range(10)))
    queries.write_text('"needle"\n')
    common = dict(cell, corpus=str(corpus), queries=str(queries), warmup=0, samples=1,
                  record_first_call=False, observe=True)
    steps = []
    def run(config, output, case):
        step = run_program_step(manifest["command"], dict(config, diagnostic_case=case), output, deadline_ns, observe=manifest.get("_observe_native"))
        step["diagnostic_case"] = case
        steps.append(step)
        return step["complete"]
    base = attempt / "base-seed"
    if not run(dict(common, family="G-seed", adapter="sdk"), base, "base_seed"):
        return {"complete": False, "steps": steps, "cases": cell["cases"]}
    read = dict(common, family="G-O", store=str(base / "store.sqlite"))
    no_match = attempt / "no-match-queries.jsonl"
    no_match.write_text('"gdeliberatelyabsenttoken"\n')
    for case, fields in [
        ("direct_text", {"routes": ["text"]}),
        ("hybrid_no_dependencies", {"routes": ["hybrid"]}),
        ("hybrid_kind_filter", {"routes": ["hybrid"], "diagnostic_filter_kind": "note"}),
        ("no_match", {"routes": ["text", "hybrid"], "queries": str(no_match)}),
    ]:
        if not run(dict(read, **fields), attempt / case, case):
            return {"complete": False, "steps": steps, "cases": cell["cases"]}
    source = attempt / "source-fixture"
    if not run(dict(common, family="G-source-fixture", adapter="sdk"), source, "source_window"):
        return {"complete": False, "steps": steps, "cases": cell["cases"]}
    boundary = steps[-1]["result"]["boundary_epoch_s"]
    if type(boundary) is not int:
        raise ValueError("source fixture did not return its actual boundary")
    source_queries = attempt / "source-queries.jsonl"
    source_queries.write_text('"gderivedneedle"\n')
    source_read = dict(common, family="G-O", n=2, store=str(source / "store.sqlite"),
                       queries=str(source_queries), routes=["text", "hybrid"], samples=2,
                       diagnostic_valid_as_of_sequence=[boundary - 1, boundary])
    if not run(source_read, attempt / "source-views", "source_window"):
        return {"complete": False, "steps": steps, "cases": cell["cases"]}
    for adapter in cell["adapters"]:
        for route in ["text", "hybrid"]:
            prefix = f"commit-{adapter}-{route}"
            seed = attempt / (prefix + "-seed")
            if not run(dict(common, family="G-seed", adapter=adapter), seed, "commit_change"):
                return {"complete": False, "steps": steps, "cases": cell["cases"]}
            write = dict(common, family="G-W", adapters=[adapter], routes=[route],
                         store=str(seed / "store.sqlite"), warmup=1, samples=1,
                         batch_size=1, searches_per_batch=1)
            if not run(write, attempt / (prefix + "-write"), "commit_change"):
                return {"complete": False, "steps": steps, "cases": cell["cases"]}
    return {"complete": all(step["complete"] for step in steps), "steps": steps, "cases": cell["cases"]}

def execute_stage(cell, manifest, attempt, deadline_ns):
    """Bind current G drivers and exact size/attempt subjects under one deadline."""
    attempt = Path(attempt)
    attempt.mkdir(parents=True, exist_ok=True)
    if cell["kind"] == "branches":
        return execute_branches(cell, manifest, attempt, deadline_ns)
    common = dict(cell, corpus=manifest["corpus"], queries=manifest["queries"])
    if cell["kind"] == "smoke":
        corpus, queries = attempt / "tiny-corpus.jsonl", attempt / "tiny-queries.jsonl"
        corpus.write_text('"alpha needle"\n"beta"\n')
        queries.write_text('"needle"\n')
        common.update(corpus=str(corpus), queries=str(queries))
    steps = []
    preparation = None
    if cell.get("fresh_copy"):
        from g_closed_copy import copy_subject
        store, preparation = copy_subject(manifest["closed_subject"], attempt / "subject/store.sqlite", deadline_ns)
    elif cell["kind"] == "seed" or cell.get("fresh_seed"):
        seed_output = attempt / "seed"
        adapter = cell.get("adapter") or cell["adapters"][0]
        seed_config = dict(common, family="G-seed", adapter=adapter)
        seed = run_program_step(manifest["command"], seed_config, seed_output, deadline_ns, observe=manifest.get("_observe_native"))
        steps.append(seed)
        if not seed["complete"] or cell["kind"] == "seed":
            return {"complete": seed["complete"], "steps": steps}
        store = str(seed_output / "store.sqlite")
    else:
        store = manifest["subjects"][cell["seed_subject"]]
    if cell["kind"] == "natural":
        questions = Path(manifest["queries"]).read_text().splitlines()
        if len(questions) < 50:
            raise ValueError("G-O needs the registered first50 questions")
        selected = attempt / "queries-first50.jsonl"
        selected.write_text("\n".join(questions[:50]) + "\n")
        common["queries"] = str(selected)
    family = "G-R" if cell["kind"] == "smoke" else cell["stage"]
    config = dict(common, family=family, store=store)
    output = attempt / ("write" if cell["stage"] == "G-W" else "read")
    steps.append(run_program_step(manifest["command"], config, output, deadline_ns, observe=manifest.get("_observe_native")))
    result = {"complete": all(step["complete"] for step in steps), "steps": steps}
    if preparation is not None:
        result["preparation"] = preparation
    return result
