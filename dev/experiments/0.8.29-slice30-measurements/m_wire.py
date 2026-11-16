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


def _hint(line):
    head, explanation = line[len("hint: "):].rsplit(" (", 1)
    reason, _ = explanation.split(": ", 1)
    action, _, rest = head.partition(" ")
    hint = {"action": action, "reason": reason}
    if rest.startswith("["):
        refs, size = json.JSONDecoder().raw_decode(rest)
        hint["refs"] = refs
        rest = rest[size:]
    fields, remaining = _fields(rest)
    if remaining:
        raise ValueError("unexpected compact hint body")
    hint.update(fields)
    return hint


def _fetch(text):
    header, separator, tail = text.partition("\n")
    if not separator:
        raise ValueError("compact fetch lacks its framing LF")
    payload, _ = _fields(header[len("fetch "):])
    expected = payload.pop("items")
    payload["budget"] = _budget(payload)
    payload.update(items=[], hints=[])
    data = tail.encode("utf-8")
    while data:
        line, separator, data = data.partition(b"\n")
        if not separator:
            raise ValueError("compact fetch item header lacks its framing LF")
        line = line.decode("utf-8")
        if line.startswith("hint: "):
            payload["hints"].append(_hint(line))
            continue
        word, fields = line.split(" ", 1)
        reference, size = json.JSONDecoder().raw_decode(fields)
        tail = fields[size:].lstrip()
        if word == "record":
            fields, _ = _fields(tail)
            count = fields["bytes"]
            if type(count) is not int or count < 0 or data[count:count + 1] != b"\n":
                raise ValueError("compact fetch body byte frame is incomplete")
            record = {"kind": fields["kind"], "body": data[:count].decode("utf-8"),
                      "body_cut": fields.get("cut", False)}
            data = data[count + 1:]
            payload["items"].append({"ref": reference, "record": record})
        elif word == "error":
            payload["items"].append({"ref": reference, "error": _error(tail)})
        else:
            raise ValueError("unrecognized compact fetch item")
    if len(payload["items"]) != expected:
        raise ValueError("compact fetch count differs from decoded items")
    return payload


def _coverage_decimal(value, maximum):
    if (type(value) is not str or re.fullmatch(r"0|[1-9][0-9]*", value) is None
            or len(value) > len(str(maximum)) or int(value) > maximum):
        raise ValueError("invalid exact coverage decimal")
    return int(value)


def _coverage(value):
    common = {"status", "population", "profile"}
    if type(value) is not dict or value.get("population") != "stored_mapped_spans":
        raise ValueError("invalid coverage population")
    profile = value.get("profile")
    if (type(profile) is not dict or set(profile) != {"key", "name", "revision", "dimension"}
            or profile["key"] != "default" or type(profile["name"]) is not str
            or type(profile["revision"]) is not str or type(profile["dimension"]) is not int
            or not 1 <= profile["dimension"] <= 2**32 - 1):
        raise ValueError("invalid coverage profile")
    if value.get("status") == "unavailable":
        if set(value) != common | {"reason"} or value["reason"] not in ("cache_cold", "state_changed"):
            raise ValueError("invalid unavailable coverage")
        return
    counts = {"records", "known_records", "unknown_records", "truncated_known_records", "unattributed_heads"}
    byte_counts = {"known_body_bytes", "unrepresented_known_bytes"}
    shares = {"truncated_known_share", "unrepresented_known_byte_share"}
    if value.get("status") != "available" or set(value) != common | counts | byte_counts | shares:
        raise ValueError("invalid available coverage fields")
    numbers = {key: _coverage_decimal(value[key], 2**63 - 1) for key in counts}
    numbers.update({key: _coverage_decimal(value[key], 2**128 - 1) for key in byte_counts})
    if (numbers["records"] != numbers["known_records"] + numbers["unknown_records"]
            or numbers["truncated_known_records"] > numbers["known_records"]
            or numbers["unrepresented_known_bytes"] > numbers["known_body_bytes"]):
        raise ValueError("inconsistent coverage counts")
    for share, numerator, denominator in [
            ("truncated_known_share", "truncated_known_records", "known_records"),
            ("unrepresented_known_byte_share", "unrepresented_known_bytes", "known_body_bytes")]:
        expected = None if numbers[denominator] == 0 else {
            "numerator": value[numerator], "denominator": value[denominator]}
        if value[share] != expected:
            raise ValueError("inconsistent unreduced coverage share")


def _current_metadata(payload):
    if type(payload) is not dict:
        raise ValueError("tool payload must be an object")
    if "stores" in payload:
        if set(payload) - {"stores", "default_store", "hints"}:
            raise ValueError("unknown store-list metadata")
        if "default_store" not in payload or (payload["default_store"] is not None
                                              and type(payload["default_store"]) is not str):
            raise ValueError("invalid nullable session default")
        if type(payload["stores"]) is not list:
            raise ValueError("invalid store listing")
        defaults = []
        for row in payload["stores"]:
            if (type(row) is not dict or set(row) != {"name", "open", "readiness", "default"}
                    or type(row["name"]) is not str or type(row["open"]) is not bool
                    or type(row["readiness"]) is not str or type(row["default"]) is not bool):
                raise ValueError("invalid store-list row")
            if row["default"]:
                defaults.append(row["name"])
        if defaults != ([] if payload["default_store"] is None else [payload["default_store"]]):
            raise ValueError("store flags disagree with session default")
    if "projection_status" in payload:
        if payload.get("not_in_this_release") != ["state", "verdict", "kinds", "counts"]:
            raise ValueError("invalid unsupported-capability disclosure")
        _coverage(payload.get("vector_coverage"))
    return payload


def _compact(text):
    if text.startswith("error "):
        return {"error": _error(text[len("error "):])}
    if text.startswith("fetch "):
        return _fetch(text)
    if text.startswith("orient "):
        lines = text.split("\n")
        payload, _ = _fields(lines[0][len("orient "):])
        payload["budget"] = _budget(payload)
        payload.update(projections=[], projection_status={"projections": []}, degradation=[])
        for line in lines[1:]:
            if line.startswith("projection "):
                name, size = json.JSONDecoder().raw_decode(line[len("projection "):])
                fields, _ = _fields(line[len("projection ") + size:])
                if "roles" in fields:
                    payload["projections"].append({"name": name, "roles": fields["roles"].split(","),
                                                   "fts": fields["fts"], "vector": fields["vector"]})
                if "dense" in fields:
                    payload["projection_status"]["projections"].append(
                        {"name": name, "dense_readiness": fields["dense"]})
            elif line.startswith("embedding: "):
                fields, _ = _fields(line[len("embedding: "):])
                fields["pending_count"] = fields.pop("pending")
                payload["embedding_readiness"] = fields
            elif line.startswith("runtime_embedder: "):
                fields, _ = _fields(line[len("runtime_embedder: "):])
                payload["projection_status"].update(runtime_embedder_available=fields["available"],
                    runtime_unavailability_reason=fields["reason"])
            elif line.startswith("vector_unsupported_kinds="):
                fields, _ = _fields(line)
                payload["projection_status"].update(fields)
            elif line.startswith("not_in_this_release="):
                if "not_in_this_release" in payload:
                    raise ValueError("duplicate unsupported-capability disclosure")
                payload["not_in_this_release"] = json.loads(line[len("not_in_this_release="):])
            elif line.startswith("coverage: "):
                if "vector_coverage" in payload:
                    raise ValueError("duplicate coverage")
                payload["vector_coverage"] = json.loads(line[len("coverage: "):])
            elif line.startswith("degraded: "):
                arm, reason = line[len("degraded: "):].split(" ", 1)
                payload["degradation"].append({"arm": arm, "reason": reason})
            else:
                raise ValueError(f"unrecognized compact orient line: {line!r}")
        return payload
    if text.startswith("stores: "):
        lines = text.split("\n")
        header = re.fullmatch(r"stores: ([0-9]+) default=(.+)", lines[0])
        if header is None:
            raise ValueError("invalid compact store header")
        expected = int(header[1])
        payload = {"default_store": json.loads(header[2]), "stores": [], "hints": []}
        for line in lines[1:]:
            if line.startswith("hint: "):
                payload["hints"].append(_hint(line))
                continue
            name, size = json.JSONDecoder().raw_decode(line)
            fields, remaining = _fields(line[size:])
            if remaining or set(fields) - {"open", "readiness", "default"}:
                raise ValueError("unknown compact store metadata")
            payload["stores"].append({"name": name, "open": fields["open"],
                                      "readiness": fields["readiness"],
                                      "default": fields.get("default", False)})
        if len(payload["stores"]) != expected:
            raise ValueError("compact store count differs from decoded listing")
        return payload
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
            payload["hints"].append(_hint(line))
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
    payload = _current_metadata(payload)
    decoded = {"ok": result.get("isError") is not True, "payload": payload, "text": text}
    if not decoded["ok"] and "error" in payload:
        error = payload["error"]
        decoded["error"] = f"{error['class']}/{error['code']}"
        if "busy" in error:
            decoded["error"] += f"/{error['busy']['reason']}"
    return decoded
