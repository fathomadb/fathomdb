# Supplemental manual acceptance, not newly passing regression coverage.
import os, hashlib, json, pathlib, re, select, subprocess, sys, time

binary = pathlib.Path(sys.argv[1]).resolve(strict=True)
run = pathlib.Path(sys.argv[2]).resolve(strict=True)
assert run.is_relative_to(
    pathlib.Path(os.environ["S60_TASK"]).resolve() / "gpu-metadata"
)
controls = [
    json.loads(line) for line in (run / "controls.jsonl").read_text().splitlines()
]
assert len(controls) == 14
sessions = []
formats = sys.argv[3:] or ["structured", "text"]
assert formats and all(format in ["structured", "text"] for format in formats)
for format in formats:
    config = run / f"mcp-{format}.json"
    config.write_text(
        json.dumps(
            {
                "stores": {"a": {"path": str(run / "store.sqlite")}},
                "default_store": "a",
                "use_default_embedder": False,
                "result_format": format,
                "search_mode": "text",
                "hints": False,
            }
        )
    )
    transcript = []
    stderr = (run / f"mcp-{format}-stderr.log").open("wb")
    process = subprocess.Popen(
        [str(binary), "--config", str(config)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=stderr,
        text=True,
    )
    next_id = 0

    def write(value):
        process.stdin.write(json.dumps(value, separators=(",", ":")) + "\n")
        process.stdin.flush()
        transcript.append({"sent": value})

    def request(method, params):
        global next_id
        next_id += 1
        write({"jsonrpc": "2.0", "id": next_id, "method": method, "params": params})
        deadline = time.monotonic() + 30
        while True:
            if not select.select(
                [process.stdout], [], [], max(0, deadline - time.monotonic())
            )[0]:
                raise RuntimeError("manual MCP response timed out")
            raw = process.stdout.readline()
            if not raw:
                raise RuntimeError("manual MCP exited before response")
            value = json.loads(raw)
            transcript.append({"received": value, "raw": raw})
            if value.get("id") == next_id:
                assert "error" not in value, value
                return value["result"]

    def call(tool, args):
        result = request("tools/call", {"name": tool, "arguments": args})
        assert result.get("isError") is not True, result
        assert len(result["content"]) == 1 and result["content"][0]["type"] == "text"
        text = result["content"][0]["text"]
        if format == "structured":
            assert json.loads(text) == result["structuredContent"]
            return result["structuredContent"], text
        assert "structuredContent" not in result
        return None, text

    try:
        request(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "s60-manual-metadata-acceptance",
                    "version": "1",
                },
            },
        )
        write({"jsonrpc": "2.0", "method": "notifications/initialized"})
        tools = request("tools/list", {})["tools"]
        assert all(
            ("outputSchema" in tool) == (format == "structured") for tool in tools
        )
        orient, text = call("orient", {})
        if format == "structured":
            assert orient["projection_status"]["runtime_embedder_available"] is False
            assert (
                orient["projection_status"]["runtime_unavailability_reason"]
                == "no_runtime"
            )
            assert orient["embedding_readiness"]["usable_embedder"] is False
        else:
            assert "runtime_embedder: available=false reason=no_runtime" in text, text
            assert any(
                line.startswith("embedding: ") and " usable_embedder=false " in line
                for line in text.splitlines()
            ), text
        maps = pathlib.Path("/proc", str(process.pid), "maps").read_text()
        (run / f"mcp-{format}-process-maps.txt").write_text(maps)
        assert not any(name in maps for name in ["libcuda", "libcublas", "libnvrtc"])
        for case in controls:
            payload, text = call("search", case["args"])
            if format == "structured":
                assert payload["mode"] == "text" and payload["degradation"] == []
                assert [hit["ref"] for hit in payload["results"]] == case[
                    "expected_refs"
                ], (case, payload)
                keys = ["ref", "score", "arm", "kind", "source_id"]
                assert [
                    {key: hit[key] for key in keys} for hit in payload["results"]
                ] == [{key: hit[key] for key in keys} for hit in case["sdk_results"]], (
                    case,
                    payload,
                )
                assert (2 * len(text.encode()) + 3) // 4 <= payload["budget"][
                    "max_tokens"
                ]
            else:
                lines = text.splitlines()
                header = lines[0]
                rows = lines[1:]
                assert header.startswith(
                    'search store="a" epoch=1 validity_instant=1900000000 mode=text '
                ), header
                assert f"hits={len(case['expected_refs'])} " in header
                assert (
                    f"completeness={'complete' if case['expected_refs'] else 'empty'} "
                    in header
                )
                assert re.search(
                    r" response_id=r1-[0-9a-f]{16} tokens=\d+/4000$", header
                ), header
                assert (
                    "continuation=" not in header
                    and "continuation_exact=" not in header
                )
                count = int(header.rsplit(" tokens=", 1)[1].split("/")[0])
                assert count == (len(text.encode()) + 3) // 4 and count <= 4000
                expected = []
                for hit in case["sdk_results"]:
                    assert f" kind={json.dumps(hit['kind'])} " in header, header
                    expected.append(
                        f"{json.dumps(hit['ref'])} arm={hit['arm']} score={hit['display_score']} src={json.dumps(hit['source_id'])} | {hit['body']}"
                    )
                assert rows == expected, (case, text, expected)
                if not expected:
                    assert " kind=" not in header
        process.stdin.close()
        code = process.wait(timeout=30)
        assert code == 0, code
        sessions.append(
            {
                "format": format,
                "cases": len(controls),
                "mcp_exit": code,
                "no_cuda_libraries_loaded": True,
            }
        )
    finally:
        (run / f"mcp-{format}-transcript.json").write_text(
            json.dumps(transcript, indent=2) + "\n"
        )
        stderr.close()
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=30)
(run / "consumer-result.json").write_text(
    json.dumps(
        {
            "manual_acceptance": "PASS",
            "sessions": sessions,
            "mcp_binary": str(binary),
            "mcp_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            "consumer_features": "normal candidate MCP, no default-embedder/CUDA/test-seams; use_default_embedder=false",
        },
        indent=2,
    )
    + "\n"
)
