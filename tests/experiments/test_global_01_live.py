"""Execution controls for the paid GLOBAL-01 first run."""

from __future__ import annotations

import json
from pathlib import Path

import pyarrow as pa
import pyarrow.parquet as pq
import pytest

from experiments import global_01_live


def test_retry_after_precedes_exponential_fallback():
    assert global_01_live.retry_delay({"Retry-After": "17"}, fallback=2.0) == 17.0
    assert global_01_live.retry_delay({}, fallback=2.0) == 2.0


def test_checkpoint_round_trip_preserves_completed_cells(tmp_path: Path):
    path = tmp_path / "checkpoint.json"
    state = global_01_live.RunState.new("a" * 64, 6.0)
    state.complete("answers/graphrag/q1", {"answer": "complete"}, cost_usd=0.25)
    state.save(path)

    resumed = global_01_live.RunState.load(path, "a" * 64, 6.0)

    assert resumed.cells["answers/graphrag/q1"]["answer"] == "complete"
    assert resumed.cost_usd == 0.25
    assert resumed.missing(["answers/graphrag/q1", "answers/fathomdb/q1"]) == [
        "answers/fathomdb/q1"
    ]


def test_cost_cap_is_checked_before_accepting_a_cell():
    state = global_01_live.RunState.new("b" * 64, 1.0)

    with pytest.raises(global_01_live.Global01LiveError, match="cost cap"):
        state.complete("too-expensive", {"answer": "x"}, cost_usd=1.01)


def test_native_witness_requires_nonempty_outputs(tmp_path: Path):
    output = tmp_path / "output"
    output.mkdir()
    for name in global_01_live.REQUIRED_GRAPH_OUTPUTS:
        (output / name).write_bytes(b"parquet")
    pq.write_table(pa.table({"community": [1, 2]}), output / "communities.parquet")
    pq.write_table(
        pa.table({"community": [1, 2]}), output / "community_reports.parquet"
    )

    global_01_live.assert_native_witness(output, ["one", "two"])

    (output / global_01_live.REQUIRED_GRAPH_OUTPUTS[0]).write_bytes(b"")
    with pytest.raises(global_01_live.Global01LiveError, match="output"):
        global_01_live.assert_native_witness(output, ["one", "two"])


def test_native_witness_rejects_missing_community_reports(tmp_path: Path):
    output = tmp_path / "output"
    output.mkdir()
    for name in global_01_live.REQUIRED_GRAPH_OUTPUTS:
        (output / name).write_bytes(b"parquet")
    pq.write_table(pa.table({"community": [1, 2]}), output / "communities.parquet")
    pq.write_table(pa.table({"community": [1]}), output / "community_reports.parquet")

    with pytest.raises(global_01_live.Global01LiveError, match="community reports"):
        global_01_live.assert_native_witness(output, ["one", "two"])


def test_checkpoint_rejects_config_drift(tmp_path: Path):
    path = tmp_path / "checkpoint.json"
    path.write_text(
        json.dumps(
            {
                "schema_version": "global-01.checkpoint.v1",
                "config_sha256": "c" * 64,
                "cost_cap_usd": 6.0,
                "cost_usd": 0.0,
                "cells": {},
            }
        ),
        encoding="utf-8",
    )

    with pytest.raises(global_01_live.Global01LiveError, match="configuration"):
        global_01_live.RunState.load(path, "d" * 64, 6.0)


def test_runtime_settings_bind_shared_deepseek_generator(tmp_path: Path):
    source = tmp_path / "source.yaml"
    target = tmp_path / "settings.yaml"
    source.write_text(
        "completion_models:\n"
        "  default_completion_model:\n"
        "    model: gpt-5.4\n"
        "    call_args: {}\n",
        encoding="utf-8",
    )

    global_01_live._render_graph_settings(
        source,
        target,
        answer_model="deepseek-v4-pro",
        thinking_mode="disabled",
    )

    rendered = target.read_text(encoding="utf-8")
    assert "model: deepseek-v4-pro" in rendered
    assert 'additional_drop_params: ["response_format"]' in rendered
    assert "thinking:" in rendered
    assert "type: disabled" in rendered


def test_runtime_community_prompt_forbids_markdown_fences(tmp_path: Path):
    prompt = tmp_path / "community_report_graph.txt"
    prompt.write_text("Return the report as JSON.", encoding="utf-8")

    global_01_live._harden_community_prompt(prompt)

    assert "Do not use Markdown code fences" in prompt.read_text(encoding="utf-8")


def test_judge_semantic_retry_checkpoints_invalid_response(tmp_path: Path):
    class FakeClient:
        def __init__(self):
                self.responses = iter(
                    [
                    (
                        '{"comprehensiveness":"A"}',
                        {"prompt_tokens": 10, "completion_tokens": 4},
                        0.01,
                    ),
                    (
                        '{"comprehensiveness":"A","diversity":"B",'
                        '"empowerment":"tie","directness":"A"}',
                        {"prompt_tokens": 12, "completion_tokens": 6},
                        0.02,
                    ),
                ]
            )

        def complete(self, model, prompt, *, max_tokens, temperature):
            return next(self.responses)

    path = tmp_path / "checkpoint.json"
    state = global_01_live.RunState.new("e" * 64, 1.0)

    global_01_live._complete_judgment(
        FakeClient(),
        "prompt",
        state=state,
        checkpoint_path=path,
        cell="judge/q1/0/fg",
    )

    assert state.cost_usd == pytest.approx(0.03)
    assert state.cells["invalid-judge/q1/0/fg/0"]["response_sha256"]
    assert state.cells["judge/q1/0/fg"]["verdicts"]["diversity"] == "B"
    restored = global_01_live.RunState.load(path, "e" * 64, 1.0)
    assert restored.cost_usd == pytest.approx(0.03)
