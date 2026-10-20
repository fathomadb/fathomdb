"""Measurement-correctness tests: compact rendering keeps the information; the audit counts right.

The search fixture is two real ``search`` structuredContent results (conversation 0,
hybrid) with each snippet's text replaced by filler of the same byte length.
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import compact_proxy  # noqa: E402
import score  # noqa: E402

FIXTURE = json.loads((HERE / "search_fixture.json").read_text())


def _hit_lines(text):
    return [line for line in text.splitlines() if line.startswith("l:")]


def test_compact_search_keeps_every_ref_cut_flag_snippet_and_order():
    for data in FIXTURE.values():
        text = compact_proxy.render_search(data)
        lines = _hit_lines(text)
        assert [line.split(" ", 1)[0] for line in lines] == [hit["ref"] for hit in data["results"]]
        for line, hit in zip(lines, data["results"]):
            meta, snippet = line.split(" | ", 1)
            assert snippet == hit["snippet"]
            assert ("[cut]" in meta.split()) == hit["snippet_cut"]
            assert hit["source_id"] in meta and hit["arm"] in meta


def test_compact_search_keeps_continuation_and_hints():
    for data in FIXTURE.values():
        text = compact_proxy.render_search(data)
        assert f"continuation={data['continuation']}" in text.splitlines()[0]
        hint_lines = [line for line in text.splitlines() if line.startswith("hint: ")]
        assert len(hint_lines) == len(data["hints"])
        for line, hint in zip(hint_lines, data["hints"]):
            assert hint["reason"] in line and line.startswith(f"hint: {hint['action']} ")
            for ref in hint.get("refs", []):
                assert ref in line
            if "continuation" in hint:
                assert hint["continuation"] in line


def test_compact_search_audit_round_trip_matches_json():
    for data in FIXTURE.values():
        as_json = json.dumps(data, separators=(",", ":"), sort_keys=True)
        assert score.cut_refs_in_result(compact_proxy.render_search(data)) == score.cut_refs_in_result(as_json)


def test_rewrite_drops_structured_content_and_passes_errors():
    data = FIXTURE["search"]
    out = compact_proxy.rewrite_result("search", {"content": [{"type": "text", "text": "x"}], "structuredContent": data})
    assert "structuredContent" not in out and out["content"][0]["text"] == compact_proxy.render_search(data)
    error = {"isError": True, "content": [{"type": "text", "text": "busy"}], "structuredContent": {"error": {}}}
    assert compact_proxy.rewrite_result("search", error) == error


def test_tools_list_strips_output_schema_only_for_rendered_tools():
    tools = {"tools": [{"name": "search", "outputSchema": {}}, {"name": "fetch", "outputSchema": {}}]}
    out = compact_proxy.rewrite_tools_list(tools)
    assert "outputSchema" not in out["tools"][0] and "outputSchema" in out["tools"][1]


def _transcript(result_text, fetch_refs):
    def use(i, name, inp):
        return {"type": "assistant", "message": {"id": f"m{i}", "usage": {"input_tokens": 1},
                "content": [{"type": "tool_use", "id": f"t{i}", "name": name, "input": inp}]}}

    def res(i, text):
        return {"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": f"t{i}",
                                                         "content": text}]}}

    records = [
        {"type": "system", "subtype": "init", "tools": [], "mcp_servers": [{"status": "connected"}]},
        use(1, "mcp__fathomdb__search", {"query": "q"}), res(1, result_text),
        use(2, "mcp__fathomdb__fetch", {"refs": fetch_refs}), res(2, "fetched"),
        use(3, "Bash", {"command": "ls"}), res(3, "denied"),
        {"type": "result", "usage": {"input_tokens": 7, "output_tokens": 3}, "total_cost_usd": 0.01},
    ]
    return [json.dumps(r) for r in records]


def test_audit_counts_calls_bytes_cut_fetches_and_off_policy():
    data = FIXTURE["search"]
    cut = [hit["ref"] for hit in data["results"] if hit["snippet_cut"]]
    for text in (json.dumps(data), compact_proxy.render_search(data)):
        audit = score.audit_transcript(_transcript(text, [cut[0], "l:not-shown"]))
        assert audit["tool_calls"] == {"search": 1, "fetch": 1}
        assert audit["cut_refs_shown"] == len(cut) and audit["cut_refs_fetched"] == 1
        assert audit["result_bytes"]["search"] == len(text.encode())
        assert audit["result_bytes_total"] == len(text.encode()) + len(b"fetched") + len(b"denied")
        assert audit["off_policy"] == ["Bash"] and audit["invalid"]
        assert audit["api_calls"] == 3 and audit["input_tokens"] == 7 and audit["cost_usd"] == 0.01
