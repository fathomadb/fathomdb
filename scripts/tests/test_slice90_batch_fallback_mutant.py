#!/usr/bin/env python3
"""Prove the bounded Slice 90 regression kills fallback-under-guard restoration."""

from __future__ import annotations

import os
import signal
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path("src/rust/crates/fathomdb-engine/src/projection_worker.rs")
ORIGINAL = """            embed_batch_with_watchdog(embedder, &bodies, batch_timeout, &shared.live_embed_threads)
                .ok()"""
MUTANT = """            match embed_batch_with_watchdog(embedder, &bodies, batch_timeout, &shared.live_embed_threads) {
                Ok(vectors) => Some(vectors),
                Err(_) => return per_job(),
            }"""
EXPECTED_FAILURE = "batch failed, then per-row fallback deadlocked for seven seconds"


def interrupt(signum: int, _frame: object) -> None:
    raise KeyboardInterrupt(f"interrupted by signal {signum}")


def main() -> None:
    signal.signal(signal.SIGTERM, interrupt)
    original_source = (ROOT / SOURCE).read_bytes()
    with tempfile.TemporaryDirectory(prefix="fathomdb-s90-batch-mutant-") as temporary:
        checkout = Path(temporary)
        archive = subprocess.Popen(
            ["git", "archive", "HEAD"],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        assert archive.stdout is not None
        extracted = subprocess.run(
            ["tar", "-xf", "-", "-C", str(checkout)],
            stdin=archive.stdout,
            capture_output=True,
            check=False,
        )
        archive.stdout.close()
        archive_stderr = archive.stderr.read().decode() if archive.stderr else ""
        archive_exit = archive.wait()
        if archive_exit or extracted.returncode:
            raise RuntimeError(
                f"archive extraction failed: git={archive_exit} tar={extracted.returncode}\n"
                f"{archive_stderr}{extracted.stderr.decode()}"
            )

        source = checkout / SOURCE
        body = source.read_text()
        if body.count(ORIGINAL) != 1:
            raise RuntimeError("candidate no longer has the reviewed batch fallback anchor")
        source.write_text(body.replace(ORIGINAL, MUTANT))

        environment = os.environ.copy()
        environment["CARGO_NET_OFFLINE"] = "true"
        environment["CARGO_TARGET_DIR"] = str(checkout / "target")
        command = [
            "cargo",
            "test",
            "-p",
            "fathomdb-engine",
            "--test",
            "slice90_projection_dispatch",
            "batch_failure_releases_permit_before_per_row_fallback",
            "--",
            "--exact",
            "--nocapture",
        ]
        try:
            result = subprocess.run(
                command,
                cwd=checkout / "src/rust",
                env=environment,
                capture_output=True,
                text=True,
                timeout=120,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            raise RuntimeError("mutant compilation or parent test exceeded 120 seconds") from error
        output = result.stdout + result.stderr
        if result.returncode == 0 or EXPECTED_FAILURE not in output:
            raise RuntimeError(
                f"restoration mutant did not fail for the intended reason "
                f"(exit {result.returncode}):\n{output}"
            )
        print("PASS: restored under-guard fallback failed in the bounded child")

    if (ROOT / SOURCE).read_bytes() != original_source:
        raise RuntimeError("candidate source changed during isolated mutant test")


if __name__ == "__main__":
    main()
