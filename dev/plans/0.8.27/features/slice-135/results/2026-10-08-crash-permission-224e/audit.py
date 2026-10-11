"""Independently check the retained crash and persistent-refusal test records."""

import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
SOURCE = "224e44c593c13d86ece648adabe445723db04070"
CANDIDATE = Path("/tmp/slice135-candidate-224e44")


def read_records(root: Path) -> tuple[str, str]:
    """Read the test harness summary and emitted state records."""
    return (root / "test.stdout").read_text(), (root / "test.stderr").read_text()


def check_records(stdout: str, stderr: str) -> dict:
    """Reject absent cases, false states, untyped refusal or weak recovery."""
    expected_cases = (
        "persistent_connection_readonly_write_refusal_reopens",
        "process_kill_after_commit_failure_before_queue_cleanup_reopens",
        "persistent_sqlite_busy_refusal_reopens",
    )
    if not all(f"test {name} ... ok" in stdout for name in expected_cases):
        raise ValueError("a required real-database probe did not pass")
    if (
        "test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out;"
        not in stdout
    ):
        raise ValueError("test summary changed")
    records = {}
    for line in stderr.splitlines():
        if line.startswith("SLICE135_"):
            label, payload = line.split(" ", 1)
            if label in records:
                raise ValueError("duplicate state record")
            records[label] = json.loads(payload)
    if set(records) != {"SLICE135_READONLY", "SLICE135_CRASH", "SLICE135_BUSY"}:
        raise ValueError("fault records missing or unexpected")
    expected_state = ["durable body base", None, None, None, "durable body recovery"]
    for label in ("SLICE135_READONLY", "SLICE135_BUSY"):
        record = records[label]
        if (
            record.get("errors") != ["Storage"] * 3
            or record.get("state") != expected_state
        ):
            raise ValueError(f"{label}: typed refusal or reopened state changed")
        if record.get("integrity") != ["ok"]:
            raise ValueError(f"{label}: SQLite structural check failed")
    crash = records["SLICE135_CRASH"]
    if (
        crash.get("body") != "durable body crash-pending"
        or crash.get("cursor") != 1
        or crash.get("vector") is not True
        or crash.get("failure_count") != 0
        or crash.get("integrity") != ["ok"]
        or crash.get("victim_status") != "ExitStatus(unix_wait_status(9))"
    ):
        raise ValueError("kill/reopen state or recovery changed")
    if not isinstance(records["SLICE135_BUSY"].get("elapsed_ms"), int):
        raise ValueError("busy refusal duration missing")
    return records


def digest(path: Path) -> str:
    """Hash exact retained file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit_receipt(root: Path) -> dict:
    """Verify source identity and all retained real-database test records."""
    if (root / "source-commit.txt").read_text().strip() != SOURCE:
        raise ValueError("candidate source declaration changed")
    head = subprocess.check_output(
        ["git", "-C", str(CANDIDATE), "rev-parse", "HEAD"], text=True
    ).strip()
    if head != SOURCE:
        raise ValueError("candidate checkout differs from declared source")
    records = check_records(*read_records(root))
    files = (
        "Cargo.toml.snapshot",
        "Cargo.lock",
        "probe.rs",
        "test.stdout",
        "test.stderr",
        "source-commit.txt",
    )
    return {
        "status": "PASS",
        "candidate_source_sha": SOURCE,
        "files_sha256": {name: digest(root / name) for name in files},
        "records": records,
        "limitation": "Test assertions establish reopened product state; temporary SQLite databases are not retained. PRAGMA query_only tests connection-level write refusal, not OS file permissions. SIGKILL does not simulate power-loss ordering.",
    }


if __name__ == "__main__":
    print(json.dumps(audit_receipt(ROOT), indent=2))
