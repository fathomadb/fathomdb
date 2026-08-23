#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = [
#   "huggingface_hub>=0.23",
# ]
# ///
"""Acquire the selected, local LongMemEval-cleaned evaluation files.

The default acquires the standard LongMemEval-S history and the oracle variant.
It deliberately excludes LongMemEval-M: no current FathomDB track requires its
500-session context, so downloading it would not be a registered use.

Payloads are written to ``data/corpus-data/raw/longmemeval-cleaned/`` and
remain gitignored. The script resolves and records the immutable Hugging Face
revision, hashes each selected file and the upstream MIT license copy, and
records question-type/evidence counts in the corpus registry.
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import shutil
import sys
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _config import add_config_cli, resolve_config  # noqa: E402
from _corpus_lib import corpus_data_dir  # noqa: E402


DATASET_ID = "xiaowu0162/longmemeval-cleaned"
ALLOWED_FILES = (
    "longmemeval_oracle.json",
    "longmemeval_s_cleaned.json",
    "longmemeval_m_cleaned.json",
)
DEFAULT_FILES = (
    "longmemeval_s_cleaned.json",
    "longmemeval_oracle.json",
)
LICENSE_URL = "https://raw.githubusercontent.com/xiaowu0162/LongMemEval/main/LICENSE"
LICENSE_SPDX = "MIT"
OUT_DIR = corpus_data_dir() / "raw" / "longmemeval-cleaned"
MANIFEST_PATH = Path(__file__).resolve().parent / "manifest.json"


@dataclass
class LongMemEvalConfig:
    """Selected official LongMemEval files to retain as local evaluation input."""

    files: list[str] = field(default_factory=lambda: list(DEFAULT_FILES))

    def validate(self) -> None:
        if not isinstance(self.files, list) or not self.files:
            raise ValueError("files must be a non-empty list")
        if any(not isinstance(name, str) or name not in ALLOWED_FILES for name in self.files):
            raise ValueError(f"files must be selected from {list(ALLOWED_FILES)}")
        if len(set(self.files)) != len(self.files):
            raise ValueError("files must not contain duplicates")


def _sha256(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def _instances(path: Path) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, list) or not all(isinstance(item, dict) for item in value):
        raise ValueError(f"{path.name}: expected a JSON array of instances")
    return value


def summarize_instances(instances: list[dict[str, Any]]) -> dict[str, object]:
    """Return schema and class counts without writing any corpus-derived text."""
    required = (
        "question_id",
        "question_type",
        "question",
        "answer",
        "question_date",
        "haystack_session_ids",
        "haystack_dates",
        "haystack_sessions",
        "answer_session_ids",
    )
    type_counts: dict[str, int] = {}
    with_answer_session_ids = 0
    with_turn_evidence = 0
    for index, instance in enumerate(instances):
        missing = [field for field in required if field not in instance]
        if missing:
            raise ValueError(f"instance {index}: missing required fields {missing}")
        question_type = instance["question_type"]
        if not isinstance(question_type, str) or not question_type:
            raise ValueError(f"instance {index}: question_type must be non-empty")
        type_counts[question_type] = type_counts.get(question_type, 0) + 1
        answer_sessions = instance["answer_session_ids"]
        if not isinstance(answer_sessions, list):
            raise ValueError(f"instance {index}: answer_session_ids must be a list")
        if answer_sessions:
            with_answer_session_ids += 1
        sessions = instance["haystack_sessions"]
        if not isinstance(sessions, list):
            raise ValueError(f"instance {index}: haystack_sessions must be a list")
        if any(
            isinstance(turn, dict) and turn.get("has_answer") is True
            for session in sessions
            if isinstance(session, list)
            for turn in session
        ):
            with_turn_evidence += 1
    return {
        "instances": len(instances),
        "question_type_counts": dict(sorted(type_counts.items())),
        "with_answer_session_ids": with_answer_session_ids,
        "with_turn_evidence": with_turn_evidence,
    }


def _copy_from_hub(cache_path: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".part")
    shutil.copyfile(cache_path, temporary)
    temporary.replace(destination)


def _download_license(destination: Path) -> None:
    request = urllib.request.Request(
        LICENSE_URL,
        headers={"User-Agent": "fathomdb-corpus-acquire/0.8"},
    )
    with urllib.request.urlopen(request, timeout=120) as response:  # noqa: S310
        contents = response.read()
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".part")
    temporary.write_bytes(contents)
    temporary.replace(destination)


def main() -> int:
    from huggingface_hub import HfApi, hf_hub_download  # type: ignore[import-not-found]

    parser = argparse.ArgumentParser(description="Acquire selected LongMemEval-cleaned files.")
    add_config_cli(parser)
    args = parser.parse_args()
    config = resolve_config(LongMemEvalConfig, args, LongMemEvalConfig())

    revision = HfApi().dataset_info(DATASET_ID).sha
    if not revision:
        raise RuntimeError("LongMemEval dataset revision was not supplied by Hugging Face")
    print(f"[longmemeval] dataset:  {DATASET_ID}@{revision}")
    print(f"[longmemeval] files:    {', '.join(config.files)}")
    print(f"[longmemeval] output:   {OUT_DIR}")

    per_file: dict[str, dict[str, object]] = {}
    for name in config.files:
        cached = hf_hub_download(
            DATASET_ID,
            filename=name,
            repo_type="dataset",
            revision=revision,
        )
        destination = OUT_DIR / name
        _copy_from_hub(cached, destination)
        summary = summarize_instances(_instances(destination))
        metadata: dict[str, object] = {
            "bytes": destination.stat().st_size,
            "sha256": _sha256(destination),
            "output": f"data/corpus-data/raw/longmemeval-cleaned/{name}",
            **summary,
        }
        per_file[name] = metadata
        print(
            f"[longmemeval] {name}: {summary['instances']} instances, "
            f"sha256={metadata['sha256'][:16]}…"
        )

    license_path = OUT_DIR / "LICENSE.txt"
    _download_license(license_path)
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    existing = manifest.get("sources", {}).get("longmemeval", {})
    acquired_at = existing.get("acquired_at") or datetime.date.today().isoformat()
    manifest.setdefault("sources", {})["longmemeval"] = {
        "script": "acquire_longmemeval.py",
        "upstream": {
            "kind": "huggingface_dataset_files",
            "id": DATASET_ID,
            "revision": revision,
            "files": config.files,
        },
        "license": LICENSE_SPDX,
        "license_source": LICENSE_URL,
        "license_sha256": _sha256(license_path),
        "distribution": "cache",
        "output_dir": "data/corpus-data/raw/longmemeval-cleaned/",
        "files": per_file,
        "acquired_at": acquired_at,
        "role_note": (
            "Human-curated conversational-memory evaluation. The standard S and "
            "oracle files support temporal-reasoning and knowledge-update work; "
            "M is excluded unless a future registered contract requires it."
        ),
    }
    MANIFEST_PATH.write_text(
        json.dumps(manifest, indent=2, ensure_ascii=True) + "\n", encoding="utf-8"
    )
    print(f"[longmemeval] license sha256={_sha256(license_path)}")
    print("[longmemeval] DONE — payload remains gitignored")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
