"""Stdio MCP proxy that re-renders ``search`` and ``expand`` results as compact text.

``compact_proxy.py <fathomdb-mcp> [server args...]`` starts the server and
forwards newline-delimited JSON-RPC both ways unchanged, except:

- ``tools/list`` result: ``outputSchema`` is removed from ``search`` and
  ``expand``, so a client does not expect ``structuredContent`` from them.
- ``tools/call`` results of ``search`` / ``expand`` that are not errors: the
  text block becomes ``render_search`` / ``render_expand`` of the
  ``structuredContent``, and ``structuredContent`` is dropped. Claude Code shows
  the model ``structuredContent`` (as JSON) whenever it is present, so keeping
  it would leave the arm identical to the JSON arm.

The renderers are pure functions so tests can check what survives.
"""

from __future__ import annotations

import json
import subprocess
import sys
import threading

RENDERED = ("search", "expand")


def _score(value) -> str:
    return f"{value:.3f}" if isinstance(value, (int, float)) else str(value)


def _hint(hint: dict) -> str:
    fields = [f"{key}={','.join(value) if isinstance(value, list) else value}"
              for key, value in hint.items() if key not in ("action", "reason")]
    line = f"hint: {hint.get('action')} {' '.join(fields)} ({hint.get('reason')})"
    if hint.get("reason") in ("more_results", "cut_by_budget"):
        line += ": call search with only the continuation"
    return line


def _envelope(tool: str, data: dict, extra: list[str]) -> list[str]:
    budget = data.get("budget") or {}
    head = [tool, f"store={data.get('store')}", *extra,
            f"tokens={budget.get('used_tokens')}/{budget.get('max_tokens')}" + (" cut" if budget.get("cut") else "")]
    lines = [" ".join(str(part) for part in head if part is not None)]
    for entry in data.get("degradation") or []:
        lines.append(f"degraded: {entry.get('arm')} {entry.get('reason')}")
    lines.extend(_hint(hint) for hint in data.get("hints") or [])
    return lines


def render_search(data: dict) -> str:
    """Compact text of a ``search`` structuredContent: header, hints, one line per hit."""
    results = data.get("results") or []
    kinds = {hit.get("kind") for hit in results}
    shared_kind = kinds.pop() if len(kinds) == 1 else None
    extra = [f"mode={data.get('mode')}", f"hits={len(results)}", data.get("completeness")]
    if shared_kind:
        extra.append(f"kind={shared_kind}")
    if data.get("continuation"):
        extra.append(f"continuation={data['continuation']}")
    extra.append(f"response_id={data.get('response_id')}")
    lines = _envelope("search", data, extra)
    for hit in results:
        meta = [hit.get("ref"), hit.get("arm"), _score(hit.get("score"))]
        if hit.get("ce_score") is not None:
            meta.append(f"ce={_score(hit['ce_score'])}")
        if hit.get("snippet_anchor") not in (None, "whole"):
            meta.append(f"@{hit['snippet_anchor']}")
        if hit.get("snippet_cut"):
            meta.append("[cut]")
        if not shared_kind:
            meta.append(f"kind={hit.get('kind')}")
        meta.append(f"src={hit.get('source_id')}")
        if hit.get("highlights"):
            meta.append("hl=" + ",".join(f"{a}-{b}" for a, b in hit["highlights"]))
        lines.append(" ".join(str(part) for part in meta) + " | " + str(hit.get("snippet", "")).replace("\n", " "))
    if "explanation" in data:
        lines.append("explanation: " + json.dumps(data["explanation"], separators=(",", ":")))
    return "\n".join(lines)


def render_expand(data: dict) -> str:
    """Compact text of an ``expand`` structuredContent: header, hints, one line per target."""
    targets = data.get("targets") or []
    extra = [f"targets={len(targets)}", f"complete={str(data.get('complete')).lower()}", f"bound={data.get('bound')}"]
    lines = _envelope("expand", data, extra)
    for target in targets:
        meta = [target.get("ref"), f"kind={target.get('kind')}", f"hop={target.get('hop_count')}",
                f"edge={target.get('terminal_edge_kind')}"]
        if target.get("body_cut"):
            meta.append("[cut]")
        lines.append(" ".join(str(part) for part in meta) + " | " + str(target.get("body", "")).replace("\n", " "))
    return "\n".join(lines)


RENDER = {"search": render_search, "expand": render_expand}


def rewrite_result(tool: str, result: dict) -> dict:
    """The compact form of one ``tools/call`` result; errors pass through."""
    data = result.get("structuredContent")
    if result.get("isError") or not isinstance(data, dict) or "error" in data:
        return result
    out = {key: value for key, value in result.items() if key != "structuredContent"}
    out["content"] = [{"type": "text", "text": RENDER[tool](data)}]
    return out


def rewrite_tools_list(result: dict) -> dict:
    tools = [{key: value for key, value in tool.items() if not (key == "outputSchema" and tool.get("name") in RENDERED)}
             for tool in result.get("tools", [])]
    return {**result, "tools": tools}


def main() -> int:
    child = subprocess.Popen(sys.argv[1:], stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0)
    pending: dict[object, str] = {}
    lock = threading.Lock()

    def upstream() -> None:
        for raw in sys.stdin.buffer:
            try:
                message = json.loads(raw)
            except json.JSONDecodeError:
                message = None
            if isinstance(message, dict) and "id" in message:
                params = message.get("params") or {}
                with lock:
                    if message.get("method") == "tools/list":
                        pending[message["id"]] = "tools/list"
                    elif message.get("method") == "tools/call" and params.get("name") in RENDERED:
                        pending[message["id"]] = params["name"]
            child.stdin.write(raw)
            child.stdin.flush()
        child.stdin.close()

    threading.Thread(target=upstream, daemon=True).start()
    for raw in child.stdout:
        try:
            message = json.loads(raw)
        except json.JSONDecodeError:
            message = None
        if isinstance(message, dict) and "result" in message and "method" not in message:
            with lock:
                kind = pending.pop(message.get("id"), None)
            if kind == "tools/list":
                message["result"] = rewrite_tools_list(message["result"])
                raw = (json.dumps(message) + "\n").encode()
            elif kind in RENDERED:
                message["result"] = rewrite_result(kind, message["result"])
                raw = (json.dumps(message) + "\n").encode()
        sys.stdout.buffer.write(raw)
        sys.stdout.buffer.flush()
    return child.wait()


if __name__ == "__main__":
    raise SystemExit(main())
