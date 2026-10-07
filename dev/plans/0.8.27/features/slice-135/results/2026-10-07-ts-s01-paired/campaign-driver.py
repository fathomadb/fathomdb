#!/usr/bin/env python3
"""Run the frozen installed-TypeScript S01 pairs without editing their protocol."""

from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess
import time


WORKTREE = Path("/home/coreyt/projects/fathomdb-worktrees/release-0.8.27-slice-135")
NOISE = WORKTREE / "dev/plans/0.8.27/features/slice-135/results/2026-10-07-ts-s01-noise-pilot"
PROTOCOL = WORKTREE / "dev/plans/0.8.27/features/slice-135/s01-ts-comparison-protocol.json"
PROTOCOL_SHA256 = "bbff7de33f874c4647488705759217ba451b6424ee603beee021ad24e9661bd2"
OUTPUT = Path("/tmp/slice135-ts-paired")
NODE = Path("/home/coreyt/.local/actions-runner/fathomdb-gpu/_work/_tool/node/25.9.0/x64/bin/node")
PYTHON = WORKTREE / ".venv/bin/python"
CHECKOUT = {
    "baseline": Path("/home/coreyt/projects/fathomdb-worktrees/release-0.8.26-slice-135-baseline"),
    "candidate": Path("/tmp/slice135-candidate-source-b2ac808"),
}
CONSUMER = {
    role: Path(f"/tmp/slice135-ts-artifacts/{role}/consumer")
    for role in ("baseline", "candidate")
}


def record(event: dict) -> None:
    with (OUTPUT / "run-order.jsonl").open("a") as stream:
        stream.write(json.dumps(event, sort_keys=True) + "\n")


def main() -> int:
    protocol_bytes = PROTOCOL.read_bytes()
    if hashlib.sha256(protocol_bytes).hexdigest() != PROTOCOL_SHA256:
        raise ValueError("frozen TypeScript S01 protocol hash changed")
    protocol = json.loads(protocol_bytes)
    if protocol["status"] != "FROZEN_TS_S01_PAIRED":
        raise ValueError("TypeScript S01 paired protocol is not frozen")
    OUTPUT.mkdir(parents=True, exist_ok=False)
    work = [(size, pair, role) for size in protocol["rows"]
            for pair, order in enumerate(protocol["pair_order_each_size"], 1)
            for role in order]
    for index, (size, pair, role) in enumerate(work):
        identity = protocol[role]
        if role == "candidate":
            main_archive = NOISE / "candidate-fathomdb-0.8.26.tgz"
            platform_archive = NOISE / "candidate-fathomdb-linux-x64-gnu-0.8.26.tgz"
        else:
            main_archive = NOISE / "fathomdb-0.8.26.tgz"
            platform_archive = NOISE / "fathomdb-linux-x64-gnu-0.8.26.tgz"
        block = f"{size}-{pair:02d}-{role}"
        command = [
            str(PYTHON), str(WORKTREE / "scripts/slice135_ts_s01_block.py"),
            "--checkout", str(CHECKOUT[role]),
            "--source-sha", identity["source_sha"],
            "--main-archive", str(main_archive),
            "--platform-archive", str(platform_archive),
            "--main-sha256", identity["main_archive_sha256"],
            "--platform-sha256", identity["platform_archive_sha256"],
            "--install-root", str(CONSUMER[role]),
            "--node", str(NODE),
            "--rows", str(size), "--samples", str(protocol["warm_samples_per_cell"]),
            "--comparison-protocol", str(PROTOCOL),
            "--role", role,
            "--output-dir", str(OUTPUT / block),
        ]
        record({"event": "start", "block": block, "index": index,
                "epoch_ns": time.time_ns(), "monotonic_ns": time.monotonic_ns(),
                "utc": datetime.now(timezone.utc).isoformat()})
        print(f"RUN {block}", flush=True)
        result = subprocess.run(command, cwd=WORKTREE, capture_output=True,
                                text=True, check=False)
        (OUTPUT / f"{block}.driver.stdout.log").write_text(result.stdout)
        (OUTPUT / f"{block}.driver.stderr.log").write_text(result.stderr)
        record({"event": "finish", "block": block, "index": index,
                "epoch_ns": time.time_ns(), "monotonic_ns": time.monotonic_ns(),
                "returncode": result.returncode,
                "utc": datetime.now(timezone.utc).isoformat()})
        print(f"FINISH {block}: {result.stdout.strip() or result.stderr.strip()}", flush=True)
        if result.returncode:
            raise RuntimeError(f"paired TypeScript S01 block invalid: {block}")
        if index + 1 < len(work):
            time.sleep(protocol["minimum_idle_seconds_between_blocks"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
