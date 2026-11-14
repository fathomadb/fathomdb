"""Decode actual MCP tool results; preserve raw representations separately."""

import json
import re


def _fields(text):
    fields = {}
    decoder = json.JSONDecoder()
    while text.strip():
        text = text.lstrip()
        if text.startswith("| "):
            return fields, text[2:]
        match = re.match(r"([a-z_]+)(=|(?=\s|$))", text)
        if match is None:
            raise ValueError(f"invalid compact metadata: {text!r}")
        key, assignment = match.groups()
        text = text[match.end():]
        if not assignment:
            fields[key] = True
            continue
        if text.startswith(('"', "[", "{")):
            value, size = decoder.raw_decode(text)
            text = text[size:]
        else:
            match = re.match(r"\S+", text)
            if match is None:
                raise ValueError("missing compact metadata value")
            value = match[0]
            text = text[match.end():]
            if value in ("true", "false"):
                value = value == "true"
            elif re.fullmatch(r"-?\d+", value):
                value = int(value)
            elif re.fullmatch(r"-?\d+\.\d+", value):
                value = float(value)
        fields[key] = value
    return fields, ""


def _budget(fields):
    used, maximum = fields.pop("tokens").split("/")
    return {"used_tokens": int(used), "max_tokens": int(maximum),
            "cut": fields.pop("cut", False)}


def _error(text):
    words, tail = text.split(" ", 1)
    error_class, code = words.split("/", 1)
    fields, remaining = _fields(tail)
    if remaining:
        raise ValueError("unexpected compact error body")
    return dict(fields, **{"class": error_class, "code": code})


def _compact(text):
    if text.startswith("error "):
        return {"error": _error(text[len("error "):])}
    lines = text.split("\n")
    if not lines or not lines[0].startswith("search "):
        return {"raw_text": text}
    payload, _ = _fields(lines[0][len("search "):])
    payload["budget"] = _budget(payload)
    payload.update(results=[], degradation=[], hints=[])
    for line in lines[1:]:
        if line.startswith("degraded: "):
            arm, reason = line[len("degraded: "):].split(" ", 1)
            payload["degradation"].append({"arm": arm, "reason": reason})
        elif line.startswith("hint: "):
            payload["hints"].append({"raw_text": line})
        elif line.startswith('"'):
            reference, size = json.JSONDecoder().raw_decode(line)
            fields, snippet = _fields(line[size:])
            fields["ref"] = reference
            fields["snippet"] = snippet
            fields["kind"] = fields.get("kind", payload.get("kind"))
            if "src" in fields:
                fields["source_id"] = fields.pop("src")
            payload["results"].append(fields)
        elif line.startswith(("  explain: ", "explanation: ")):
            payload.setdefault("explanation_lines", []).append(line)
        else:
            raise ValueError(f"unrecognized compact search line: {line!r}")
    if payload["hits"] != len(payload["results"]):
        raise ValueError("compact hit count differs from decoded rows")
    return payload


def decode_tool(message):
    """Decode actual compact or structured data without synthesizing absent fields."""
    if "error" in message:
        return {"ok": False, "error": f"protocol/{message['error']['code']}",
                "payload": message["error"]}
    result = message["result"]
    text = "".join(block.get("text", "") for block in result.get("content", []))
    payload = result["structuredContent"] if "structuredContent" in result else _compact(text)
    decoded = {"ok": result.get("isError") is not True, "payload": payload, "text": text}
    if not decoded["ok"] and "error" in payload:
        error = payload["error"]
        decoded["error"] = f"{error['class']}/{error['code']}"
        if "busy" in error:
            decoded["error"] += f"/{error['busy']['reason']}"
    return decoded
