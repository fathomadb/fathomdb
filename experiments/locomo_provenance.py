"""Content-free provenance mapping for official LOCOMO ingestion payloads."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


_SCHEMA_VERSION = "locomo-provenance.v1"
_USER_ID = re.compile(r"^locomo_(\d+)_.+$")
_ENTRY_KEYS = {"fingerprint", "conversation_id", "session_id", "turn_ids"}
_SESSION_KEY = re.compile(r"^session_(\d+)$")


def _normalized_payload(payload: object) -> dict[str, object]:
    if not isinstance(payload, dict):
        raise ValueError("ingestion payload must be an object")
    user_id, messages = payload.get("user_id"), payload.get("messages")
    match = _USER_ID.fullmatch(user_id) if isinstance(user_id, str) else None
    if match is None:
        raise ValueError("ingestion payload must carry an official LOCOMO user_id")
    if not isinstance(messages, list) or not messages:
        raise ValueError("ingestion payload must carry non-empty messages")
    for message in messages:
        if not isinstance(message, dict) or set(message) != {"role", "content"}:
            raise ValueError("each ingestion message must contain only role and content")
        if not isinstance(message["role"], str) or not isinstance(message["content"], str):
            raise ValueError("each ingestion message role and content must be strings")
    normalized: dict[str, object] = {"user_id": f"locomo_{match.group(1)}", "messages": messages}
    if "timestamp" in payload and payload["timestamp"] is not None:
        timestamp = payload["timestamp"]
        if isinstance(timestamp, bool) or not isinstance(timestamp, int):
            raise ValueError("ingestion payload timestamp must be an integer")
        normalized["timestamp"] = timestamp
    return normalized


def payload_fingerprint(payload: object) -> str:
    """Hash the run-id-normalized official ingestion payload without retaining text."""
    encoded = json.dumps(_normalized_payload(payload), sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


@dataclass(frozen=True)
class ProvenanceEntry:
    """Stable, content-free location of one unique ingestion payload."""

    fingerprint: str
    conversation_id: str
    session_id: str
    turn_ids: tuple[str, ...]

    def __post_init__(self) -> None:
        if not re.fullmatch(r"[0-9a-f]{64}", self.fingerprint):
            raise ValueError("provenance fingerprint must be a lowercase sha256")
        if not self.conversation_id or not self.session_id or not self.turn_ids or not all(self.turn_ids):
            raise ValueError("provenance entry must carry non-empty stable identifiers")

    def safe_metadata(self) -> dict[str, object]:
        """Return evaluation identifiers only; never payload text."""
        return {
            "conversation_id": self.conversation_id,
            "session_id": self.session_id,
            "turn_ids": list(self.turn_ids),
        }


class ProvenanceMap:
    """Resolve official ingestion payloads, rejecting gaps and collisions."""

    def __init__(self, entries: list[ProvenanceEntry]) -> None:
        self._entries: dict[str, ProvenanceEntry] = {}
        for entry in entries:
            if entry.fingerprint in self._entries:
                raise ValueError("ambiguous provenance fingerprint")
            self._entries[entry.fingerprint] = entry

    def resolve(self, payload: object) -> ProvenanceEntry:
        """Resolve one payload or reject it before the façade writes data."""
        fingerprint = payload_fingerprint(payload)
        try:
            return self._entries[fingerprint]
        except KeyError as exc:
            raise ValueError("unmapped provenance fingerprint") from exc

    @classmethod
    def from_document(cls, document: object) -> "ProvenanceMap":
        """Load and validate a content-free LOCOMO provenance manifest."""
        if not isinstance(document, dict) or set(document) != {"schema_version", "entries"}:
            raise ValueError("provenance manifest keys mismatch")
        if document["schema_version"] != _SCHEMA_VERSION or not isinstance(document["entries"], list):
            raise ValueError("invalid provenance manifest schema")
        entries: list[ProvenanceEntry] = []
        for raw in document["entries"]:
            if not isinstance(raw, dict) or set(raw) != _ENTRY_KEYS or not isinstance(raw["turn_ids"], list):
                raise ValueError("provenance entry keys mismatch")
            entries.append(ProvenanceEntry(
                fingerprint=raw["fingerprint"],
                conversation_id=raw["conversation_id"],
                session_id=raw["session_id"],
                turn_ids=tuple(raw["turn_ids"]),
            ))
        return cls(entries)


def load_manifest(path: str | Path) -> ProvenanceMap:
    """Load a content-free provenance manifest from external campaign storage."""
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError("invalid provenance manifest JSON") from exc
    return ProvenanceMap.from_document(document)


def _messages_for_session(conversation: dict[str, Any], turns: object) -> list[tuple[dict[str, str], str]]:
    if not isinstance(turns, list):
        raise ValueError("LOCOMO session must be a list")
    speaker_a = conversation.get("speaker_a")
    if not isinstance(speaker_a, str) or not speaker_a:
        raise ValueError("LOCOMO conversation is missing speaker_a")
    messages: list[tuple[dict[str, str], str]] = []
    for turn in turns:
        if not isinstance(turn, dict):
            raise ValueError("LOCOMO turn must be an object")
        speaker, text, dia_id = turn.get("speaker"), turn.get("text"), turn.get("dia_id")
        if not isinstance(speaker, str) or not isinstance(text, str) or not isinstance(dia_id, str) or not dia_id:
            raise ValueError("LOCOMO turn is missing stable speaker, text, or dia_id")
        query, blip = turn.get("query", ""), turn.get("blip_caption", "")
        if not isinstance(query, str) or not isinstance(blip, str):
            raise ValueError("LOCOMO image annotations must be strings")
        if query and blip:
            tag = f"[Sharing image - query: {query}. The image shows: {blip}]"
        elif query:
            tag = f"[Sharing image - query for: {query}]"
        elif blip:
            tag = f"[Sharing image that shows: {blip}]"
        else:
            tag = ""
        if tag:
            text = f"{text} {tag}" if text else tag
        if text:
            messages.append(({"role": "user" if speaker == speaker_a else "assistant", "content": f"{speaker}: {text}"}, dia_id))
    return messages


def _locomo_epoch(value: object) -> int | None:
    if not isinstance(value, str):
        return None
    for date_format in ("%I:%M %p on %d %B, %Y", "%I:%M %p on %d %b, %Y"):
        try:
            return int(datetime.strptime(value, date_format).replace(tzinfo=timezone.utc).timestamp())
        except ValueError:
            continue
    return None


def build_manifest_document(corpus: object, *, ingest_unit: str) -> dict[str, object]:
    """Build a content-free manifest for turn or complete-session ingestion."""
    if ingest_unit not in {"turn", "session"}:
        raise ValueError("ingest_unit must be turn or session")
    if not isinstance(corpus, list):
        raise ValueError("LOCOMO corpus must be a list")
    entries: list[dict[str, object]] = []
    for conversation_index, item in enumerate(corpus):
        if not isinstance(item, dict) or not isinstance(item.get("conversation"), dict):
            raise ValueError("LOCOMO corpus entry is missing conversation")
        conversation = item["conversation"]
        sessions = [
            (key, conversation.get(f"{key}_date_time", ""), conversation[key])
            for key in conversation if _SESSION_KEY.fullmatch(key)
        ]
        sessions.sort(key=lambda item: (
            0 if _locomo_epoch(item[1]) is not None else 1,
            _locomo_epoch(item[1]) if _locomo_epoch(item[1]) is not None else int(_SESSION_KEY.fullmatch(item[0]).group(1)),
        ))
        for session_id, session_date, turns in sessions:
            messages = _messages_for_session(conversation, turns)
            if not messages:
                continue
            groups = [messages] if ingest_unit == "session" else [[message] for message in messages]
            for group in groups:
                payload: dict[str, object] = {
                    "user_id": f"locomo_{conversation_index}_manifest",
                    "messages": [message for message, _ in group],
                }
                timestamp = _locomo_epoch(session_date)
                if timestamp is not None:
                    payload["timestamp"] = timestamp
                entries.append({
                    "fingerprint": payload_fingerprint(payload),
                    "conversation_id": f"locomo-{conversation_index}",
                    "session_id": session_id,
                    "turn_ids": [turn_id for _, turn_id in group],
                })
    ProvenanceMap.from_document({"schema_version": _SCHEMA_VERSION, "entries": entries})
    return {"schema_version": _SCHEMA_VERSION, "entries": entries}


def write_manifest(*, corpus_path: str | Path, ingest_unit: str, output_path: str | Path) -> None:
    """Write a content-free manifest outside the repository."""
    output = Path(output_path).resolve()
    repository = Path(__file__).resolve().parents[1]
    if output.is_relative_to(repository):
        raise ValueError("provenance manifest output must remain outside the repository")
    try:
        corpus = json.loads(Path(corpus_path).read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError("invalid LOCOMO corpus JSON") from exc
    document = build_manifest_document(corpus, ingest_unit=ingest_unit)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    """Generate an external content-free LOCOMO provenance manifest."""
    parser = argparse.ArgumentParser(description="Generate LOCOMO provenance manifest")
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--ingest-unit", choices=("turn", "session"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        write_manifest(corpus_path=args.corpus, ingest_unit=args.ingest_unit, output_path=args.output)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    return 0


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
