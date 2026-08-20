#!/usr/bin/env python3
"""Contract tests for the redacted Gitleaks history multiset classifier."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = REPO_ROOT / "scripts/security/check-gitleaks-history.py"
HISTORY_GUARD = REPO_ROOT / "scripts/security/gitleaks-history.sh"
MANIFEST = REPO_ROOT / "scripts/security/gitleaks-history-manifest.json"
PREFIX = b"fathomdb-gitleaks-history-v1\0"


def fingerprint(record: str) -> str:
    digest = hashlib.sha256(PREFIX + record.replace("|", "\0").encode()).hexdigest()
    return f"{digest[:32]}.{digest[32:]}"


def fixture_records() -> list[str]:
    records = [
        f"{index:040x}|generic-api-key|fixtures/record-{index}.txt|{index + 1}"
        for index in range(51)
    ]
    return [
        *records,
        *([records[0]] * 6),
        *([records[1]] * 6),
        *([records[2]] * 6),
        *([records[3]] * 6),
        *([records[4]] * 6),
        *([records[5]] * 6),
        *([records[6]] * 6),
        *([records[7]] * 3),
        *([records[8]] * 3),
        records[9],
    ]


def manifest_for(records: list[str]) -> dict[str, object]:
    counts = Counter(fingerprint(record) for record in records)
    return {
        "schema": 1,
        "scanner_version": "8.30.1",
        "tuple_encoding": "fathomdb-gitleaks-history-v1 NUL commit NUL rule NUL path NUL start_line",
        "expected_records": sum(counts.values()),
        "expected_unique_fingerprints": len(counts),
        "entries": [
            {"fingerprint": key, "count": value} for key, value in sorted(counts.items())
        ],
    }


def run_case(manifest: dict[str, object], records: list[str]) -> subprocess.CompletedProcess[str]:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        manifest_path = root / "manifest.json"
        report_path = root / "safe-report.txt"
        manifest_path.write_text(json.dumps(manifest))
        report_path.write_text("\n".join(records) + "\n")
        return subprocess.run(
            [sys.executable, str(CHECKER), str(manifest_path), str(report_path)],
            check=False,
            capture_output=True,
            text=True,
        )


def expect(condition: bool, description: str) -> int:
    print(f"{'PASS' if condition else 'FAIL'}  {description}")
    return 0 if condition else 1


def main() -> int:
    failures = 0
    records = fixture_records()
    manifest = manifest_for(records)

    module_spec = importlib.util.spec_from_file_location("gitleaks_history", CHECKER)
    if module_spec is None or module_spec.loader is None:
        raise RuntimeError("history classifier cannot be imported")
    classifier = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(classifier)
    tracked_manifest = json.loads(MANIFEST.read_text())
    tracked_counts = classifier.load_manifest(MANIFEST)
    failures += expect(
        tracked_counts is not None
        and sum(tracked_counts.values()) == 100
        and len(tracked_counts) == 51
        and sorted(tracked_counts.values()).count(7) == 7
        and sorted(tracked_counts.values()).count(4) == 2
        and sorted(tracked_counts.values()).count(2) == 1,
        "tracked manifest is the exact 100-record / 51-fingerprint multiset",
    )
    tracked_manifest["entries"].pop()
    with tempfile.TemporaryDirectory() as temporary:
        mutated_manifest = Path(temporary) / "manifest.json"
        mutated_manifest.write_text(json.dumps(tracked_manifest))
        failures += expect(
            classifier.load_manifest(mutated_manifest) is None,
            "tracked manifest rejects a deleted fingerprint mutation",
        )

    exact = run_case(manifest, records)
    failures += expect(exact.returncode == 0, "accepts exact 100-record / 51-fingerprint multiset")

    unknown = run_case(manifest, [*records, "f" * 40 + "|generic-api-key|fixtures/new.txt|1"])
    failures += expect(unknown.returncode != 0, "rejects an unknown safe tuple")
    failures += expect("fixtures/new.txt" not in unknown.stderr, "does not print unknown tuple data")

    missing = run_case(manifest, records[1:])
    failures += expect(missing.returncode != 0, "rejects a missing expected tuple")

    duplicate = run_case(manifest, [records[9], *records])
    failures += expect(duplicate.returncode != 0, "rejects an extra count-two tuple occurrence")

    malformed = run_case(manifest, [*records[:-1], "not-a-safe-record"])
    failures += expect(malformed.returncode != 0, "rejects malformed safe scanner output")

    bad_manifest = dict(manifest)
    bad_manifest["expected_records"] = 34
    manifest_failure = run_case(bad_manifest, records)
    failures += expect(manifest_failure.returncode != 0, "rejects inconsistent manifest counts")

    history_text = HISTORY_GUARD.read_text()
    required = (
        '"$SCRIPT_DIR/gitleaks-current.sh"',
        "check-gitleaks-history.py",
        'git clone --mirror "$repo" "$history_repo"',
        'empty_ignore="$scan_root/empty-ignore"',
        ': >"$empty_ignore"',
        'if [ -s "$empty_ignore" ]; then',
        "unset GITLEAKS_CONFIG",
        "unset GITLEAKS_CONFIG_TOML",
        "--ignore-gitleaks-allow",
        '--gitleaks-ignore-path "$empty_ignore"',
        '--log-opts="--all"',
        "--report-format template",
        "--report-template",
        "--report-path",
    )
    forbidden = (
        "--baseline-path",
        "--exit-code 0",
        ".gitleaksignore",
        "--config",
        "--enable-rule",
        "--allowlist",
        "global-allowlist",
    )
    def guard_contract(text: str) -> bool:
        config_env_lines = [
            line.strip() for line in text.splitlines() if "GITLEAKS_CONFIG" in line
        ]
        return (
            all(token in text for token in required)
            and not any(token in text for token in forbidden)
            and config_env_lines == ["unset GITLEAKS_CONFIG", "unset GITLEAKS_CONFIG_TOML"]
        )
    failures += expect(
        guard_contract(history_text),
        "history guard requires current zero and forbids unsafe scanner bypasses",
    )
    failures += expect(
        not guard_contract(history_text.replace('"$SCRIPT_DIR/gitleaks-current.sh"', "", 1)),
        "history guard contract rejects a deleted current-tree prerequisite",
    )
    failures += expect(
        not guard_contract(history_text + "\n--baseline-path ignored.json\n"),
        "history guard contract rejects a baseline mutation",
    )
    failures += expect(
        not guard_contract(history_text.replace("--ignore-gitleaks-allow", "", 1)),
        "history guard contract rejects a deleted inline-allow protection",
    )
    failures += expect(
        not guard_contract(history_text.replace('--gitleaks-ignore-path "$empty_ignore"', "", 1)),
        "history guard contract rejects a deleted controlled-ignore path",
    )
    failures += expect(
        not guard_contract(history_text.replace('git clone --mirror "$repo" "$history_repo"', "", 1)),
        "history guard contract rejects a deleted isolated-source boundary",
    )
    failures += expect(
        not guard_contract(
            history_text.replace(': >"$empty_ignore"', 'printf %s nonempty >"$empty_ignore"', 1)
        ),
        "history guard contract rejects a nonempty owned ignore initialization",
    )
    failures += expect(
        not guard_contract(history_text.replace('if [ -s "$empty_ignore" ]; then', "", 1)),
        "history guard contract rejects a deleted owned-ignore size check",
    )
    failures += expect(
        not guard_contract(history_text + "\n--config unreviewed.toml\n"),
        "history guard contract rejects an unreviewed scanner config input",
    )
    failures += expect(
        not guard_contract(history_text + "\nGITLEAKS_CONFIG=unreviewed.toml\n"),
        "history guard contract rejects an unreviewed scanner config environment input",
    )
    failures += expect(
        not guard_contract(history_text.replace("unset GITLEAKS_CONFIG\n", "", 1)),
        "history guard contract rejects a deleted path-config neutralization",
    )
    failures += expect(
        not guard_contract(history_text.replace("unset GITLEAKS_CONFIG_TOML", "", 1)),
        "history guard contract rejects a deleted TOML-config neutralization",
    )
    failures += expect(
        not guard_contract(history_text + "\nGITLEAKS_CONFIG_TOML=unreviewed\n"),
        "history guard contract rejects an unreviewed TOML-config environment input",
    )
    failures += expect(
        not guard_contract(history_text + "\n--allowlist unreviewed.txt\n"),
        "history guard contract rejects an unreviewed global allowlist input",
    )

    print(f"{len(records)} safe fixture records, {len(manifest['entries'])} fingerprints")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
