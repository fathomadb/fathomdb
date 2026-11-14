"""Decode actual MCP tool results; preserve raw representations separately."""


def decode_tool(message):
    """Read the existing structured representation without inventing result data."""
    if "error" in message:
        return {"ok": False, "error": f"protocol/{message['error']['code']}",
                "payload": message["error"]}
    result = message["result"]
    return {"ok": result.get("isError") is not True,
            "payload": result.get("structuredContent", {}),
            "text": "".join(block.get("text", "") for block in result.get("content", []))}
