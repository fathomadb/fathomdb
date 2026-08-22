#!/usr/bin/env python3
"""Executable contract for proportional CI routing.

Path results come from the exact pinned dorny/paths-filter action through
scripts/tests/lib/run-paths-filter-action.sh. This checker does not implement
glob semantics. It parses and evaluates only the small boolean-expression
subset used by the workflow's job-level conditions.
"""

from __future__ import annotations

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap


REPO_ROOT = Path(__file__).resolve().parents[2]
CI_PATH = REPO_ROOT / ".github/workflows/ci.yml"
AARCH_PATH = REPO_ROOT / ".github/workflows/aarch64-release-preflight.yml"
HISTORY_PATH = REPO_ROOT / ".github/workflows/gitleaks-history.yml"
RELEASE_PATH = REPO_ROOT / ".github/workflows/release.yml"
SMOKE_PATH = REPO_ROOT / "scripts/release/smoke/smoke-local-native-artifacts.sh"
MATCHER_RUNNER = REPO_ROOT / "scripts/tests/lib/run-paths-filter-action.sh"
PATHS_FILTER_PIN = "fbd0ab8f3e69293af611ebaee6363fc25e6d187d"

SCOPED_DRIVERS = {
    "verify": ("rust", "python", "typescript", "verify_harness", "rust_test_harness"),
    "rust-workspace-race-report": ("rust", "rust_test_harness"),
    "security": ("rust", "python", "typescript", "security_harness"),
    "default-embedder-tests": ("rust", "python", "typescript"),
    "wheel-size-gate": ("rust", "python"),
    "native-artifact-runtime-validation": (
        "rust",
        "python",
        "typescript",
        "native_artifact_harness",
    ),
    "windows-wal-checkpoint-diagnosis": ("rust",),
    "windows-wal-attribution": ("rust", "python", "windows"),
}
CATEGORIES = (
    "rust",
    "python",
    "typescript",
    "windows",
    "ci_workflow",
    "verify_harness",
    "rust_test_harness",
    "security_harness",
    "native_artifact_harness",
)


class Checks:
    def __init__(self) -> None:
        self.failures: list[str] = []

    def require(self, condition: bool, message: str) -> None:
        if not condition:
            self.failures.append(message)


def job_block(workflow: str, name: str) -> str | None:
    jobs_marker = workflow.find("jobs:\n")
    if jobs_marker < 0:
        return None
    jobs = workflow[jobs_marker + len("jobs:\n") :]
    headings = list(re.finditer(r"^  ([A-Za-z0-9_-]+):\n", jobs, re.MULTILINE))
    for index, heading in enumerate(headings):
        if heading.group(1) != name:
            continue
        end = headings[index + 1].start() if index + 1 < len(headings) else len(jobs)
        return jobs[heading.start() : end]
    return None


def step_block(job: str, *, step_id: str | None = None, name: str | None = None) -> str | None:
    starts = list(re.finditer(r"^      - (?:uses|name):", job, re.MULTILINE))
    for index, start in enumerate(starts):
        end = starts[index + 1].start() if index + 1 < len(starts) else len(job)
        candidate = job[start.start() : end]
        if step_id is not None and re.search(
            rf"^        id:\s*{re.escape(step_id)}\s*$", candidate, re.MULTILINE
        ):
            return candidate
        if name is not None and re.search(
            rf"^      - name:\s*{re.escape(name)}\s*$", candidate, re.MULTILINE
        ):
            return candidate
    return None


def block_scalar(block: str, key: str, indentation: int) -> str | None:
    lines = block.splitlines()
    marker = " " * indentation + key + ": |"
    for index, line in enumerate(lines):
        if line != marker:
            continue
        content: list[str] = []
        for following in lines[index + 1 :]:
            if following.strip() and len(following) - len(following.lstrip()) <= indentation:
                break
            content.append(following)
        return textwrap.dedent("\n".join(content)).rstrip() + "\n"
    return None


def paths_filter_step(job: str, step_id: str, checks: Checks) -> tuple[str, str] | None:
    step = step_block(job, step_id=step_id)
    if step is None:
        checks.require(False, f"changes job is missing paths-filter step {step_id}")
        return None
    checks.require(
        f"dorny/paths-filter@{PATHS_FILTER_PIN}" in step,
        f"{step_id} does not use the exact workflow pin",
    )
    filters = block_scalar(step, "filters", 10)
    if filters is None:
        checks.require(False, f"{step_id} is missing its filters block")
        return None
    quantifier_match = re.search(r"^          predicate-quantifier:\s*(\S+)\s*$", step, re.MULTILINE)
    quantifier = quantifier_match.group(1) if quantifier_match else "some"
    return quantifier, filters


def action_outputs(path: Path) -> dict[str, str]:
    lines = path.read_text().splitlines()
    outputs: dict[str, str] = {}
    index = 0
    while index < len(lines):
        line = lines[index]
        if "<<" in line:
            key, delimiter = line.split("<<", 1)
            values: list[str] = []
            index += 1
            while index < len(lines) and lines[index] != delimiter:
                values.append(lines[index])
                index += 1
            outputs[key] = "\n".join(values)
        elif "=" in line:
            key, value = line.split("=", 1)
            outputs[key] = value
        index += 1
    return outputs


def run_matcher(
    quantifier: str, filters: str, paths: tuple[str, ...], work: Path, label: str, checks: Checks
) -> dict[str, str]:
    filters_path = work / f"{label}-filters.yml"
    output_path = work / f"{label}-output"
    filters_path.write_text(filters)
    completed = subprocess.run(
        [str(MATCHER_RUNNER), quantifier, str(filters_path), str(output_path), *paths],
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        checks.require(False, f"exact matcher failed for {label}: {completed.stderr.strip()}")
        return {}
    return action_outputs(output_path)


def normalized_condition(job: str) -> str | None:
    lines = job.splitlines()
    for index, line in enumerate(lines):
        match = re.match(r"^    if:\s*(.*)$", line)
        if not match:
            continue
        value = match.group(1).strip()
        if value in {">", ">-", "|", "|-"}:
            parts: list[str] = []
            for following in lines[index + 1 :]:
                if following.strip() and len(following) - len(following.lstrip()) <= 4:
                    break
                parts.append(following.strip())
            value = " ".join(part for part in parts if part)
        if value.startswith("${{") and value.endswith("}}"):  # job conditions permit either form
            value = value[3:-2].strip()
        return " ".join(value.split())
    return None


TOKEN = re.compile(r"\s*(\|\||&&|==|!=|!|\(|\)|'(?:[^']*)'|[A-Za-z_][A-Za-z0-9_.-]*)")


class Expression:
    def __init__(self, source: str, context: dict[str, str]) -> None:
        self.tokens = TOKEN.findall(source)
        compact = "".join(self.tokens)
        expected = re.sub(r"\s+", "", source)
        if compact != expected:
            raise ValueError(f"unsupported condition syntax: {source}")
        self.position = 0
        self.context = context

    def evaluate(self) -> bool:
        result = bool(self.parse_or())
        if self.position != len(self.tokens):
            raise ValueError(f"unconsumed condition tokens: {self.tokens[self.position:]}")
        return result

    def accept(self, token: str) -> bool:
        if self.position < len(self.tokens) and self.tokens[self.position] == token:
            self.position += 1
            return True
        return False

    def parse_or(self) -> object:
        value = self.parse_and()
        while self.accept("||"):
            right = self.parse_and()
            value = bool(value) or bool(right)
        return value

    def parse_and(self) -> object:
        value = self.parse_equality()
        while self.accept("&&"):
            right = self.parse_equality()
            value = bool(value) and bool(right)
        return value

    def parse_equality(self) -> object:
        value = self.parse_unary()
        if self.accept("=="):
            return value == self.parse_unary()
        if self.accept("!="):
            return value != self.parse_unary()
        return value

    def parse_unary(self) -> object:
        if self.accept("!"):
            return not bool(self.parse_unary())
        if self.accept("("):
            value = self.parse_or()
            if not self.accept(")"):
                raise ValueError("missing closing parenthesis")
            return value
        if self.position >= len(self.tokens):
            raise ValueError("unexpected end of condition")
        token = self.tokens[self.position]
        self.position += 1
        if token in {"always", "cancelled"} and self.accept("("):
            if not self.accept(")"):
                raise ValueError(f"malformed {token}()")
            # always() is true even while the run is being cancelled, which is
            # exactly why job conditions must not use it: GitHub does not cancel
            # a running job whose condition stays true. cancelled() mirrors the
            # run's cancellation state supplied by the scenario.
            return True if token == "always" else self.context.get("run.cancelled") == "true"
        if token.startswith("'"):
            return token[1:-1]
        if token == "true":
            return True
        if token == "false":
            return False
        return self.context.get(token, "")


def expected_scoped_condition(categories: tuple[str, ...]) -> str:
    drivers = " || ".join(f"needs.changes.outputs.{name} == 'true'" for name in categories)
    return " ".join(
        (
            "needs.changes.outputs.ci_workflow == 'true' || (",
            "needs.changes.outputs.docs_only != 'true' &&",
            f"( {drivers} ) &&",
            "needs.changes.outputs.ci_mode != 'lite' )",
        )
    )


def evaluate_routes(
    conditions: dict[str, str],
    outputs: dict[str, str],
    result: str,
    checks: Checks,
    label: str,
    *,
    run_cancelled: bool = False,
) -> set[str]:
    context = {f"needs.changes.outputs.{key}": value for key, value in outputs.items()}
    context["needs.changes.result"] = result
    context["run.cancelled"] = "true" if run_cancelled else "false"
    selected: set[str] = set()
    for job_name, condition in conditions.items():
        try:
            if Expression(condition, context).evaluate():
                selected.add(job_name)
        except ValueError as error:
            checks.require(False, f"{label}: cannot evaluate {job_name}: {error}")
    return selected


def git(cwd: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", "-C", str(cwd), *arguments],
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()


def marker_repository(root: Path, message: str) -> tuple[Path, str]:
    repository = root / "repository"
    repository.mkdir()
    git(repository, "init", "--quiet", "--initial-branch=main")
    git(repository, "config", "user.email", "ci-mode-fixture@example.invalid")
    git(repository, "config", "user.name", "CI Mode Fixture")
    (repository / "base").write_text("base\n")
    git(repository, "add", "base")
    git(repository, "commit", "--quiet", "-m", "base")
    (repository / "change").write_text("change\n")
    git(repository, "add", "change")
    git(repository, "commit", "--quiet", "-m", message)
    return repository, git(repository, "rev-parse", "HEAD")


def run_marker_script(
    script: str,
    root: Path,
    *,
    message: str,
    event: str,
    association: str = "OWNER",
    head_repository: str = "fathomadb/fathomdb",
) -> str:
    repository, candidate = marker_repository(root, message)
    output = root / "github-output"
    output.touch()
    environment = os.environ.copy()
    environment.update(
        {
            "EVENT_NAME": event,
            "PR_AUTHOR_ASSOCIATION": association,
            "PR_HEAD_SHA": candidate if event == "pull_request" else "",
            "PR_HEAD_REPOSITORY": head_repository if event == "pull_request" else "",
            "PUSH_SHA": candidate if event == "push" else "",
            "REPOSITORY": "fathomadb/fathomdb",
            "GITHUB_OUTPUT": str(output),
        }
    )
    completed = subprocess.run(
        ["bash", "-c", script], cwd=repository, env=environment, text=True, capture_output=True, check=False
    )
    if completed.returncode != 0:
        return f"error:{completed.stderr.strip()}"
    return action_outputs(output).get("ci_mode", "missing")


def run_merge_marker_script(script: str, root: Path) -> str:
    repository = root / "repository"
    repository.mkdir()
    git(repository, "init", "--quiet", "--initial-branch=main")
    git(repository, "config", "user.email", "ci-mode-fixture@example.invalid")
    git(repository, "config", "user.name", "CI Mode Fixture")
    (repository / "base").write_text("base\n")
    git(repository, "add", "base")
    git(repository, "commit", "--quiet", "-m", "base")
    git(repository, "switch", "--quiet", "-c", "feature")
    (repository / "feature").write_text("feature\n")
    git(repository, "add", "feature")
    git(repository, "commit", "--quiet", "-m", "[ci-lite]")
    git(repository, "switch", "--quiet", "main")
    (repository / "main").write_text("main\n")
    git(repository, "add", "main")
    git(repository, "commit", "--quiet", "-m", "main advance")
    git(repository, "merge", "--quiet", "--no-ff", "feature", "-m", "merge without marker")
    merge_sha = git(repository, "rev-parse", "HEAD")
    output = root / "github-output"
    output.touch()
    environment = os.environ.copy()
    environment.update(
        {
            "EVENT_NAME": "push",
            "PR_AUTHOR_ASSOCIATION": "",
            "PR_HEAD_SHA": "",
            "PR_HEAD_REPOSITORY": "",
            "PUSH_SHA": merge_sha,
            "REPOSITORY": "fathomadb/fathomdb",
            "GITHUB_OUTPUT": str(output),
        }
    )
    completed = subprocess.run(
        ["bash", "-c", script], cwd=repository, env=environment, text=True, capture_output=True, check=False
    )
    if completed.returncode != 0:
        return f"error:{completed.stderr.strip()}"
    return action_outputs(output).get("ci_mode", "missing")


def main() -> int:
    checks = Checks()
    ci = CI_PATH.read_text()
    release = RELEASE_PATH.read_text()
    aarch = AARCH_PATH.read_text()
    changes = job_block(ci, "changes")
    checks.require(changes is not None, "ci.yml is missing the changes job")
    if changes is None:
        changes = ""

    permissions = ci.split("jobs:\n", 1)[0]
    checks.require("contents: read" in permissions, "workflow permissions lost contents: read")
    checks.require("pull-requests: read" in permissions, "paths-filter PR API lacks pull-requests: read")
    checks.require(
        "ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}"
        in changes,
        "changes checkout does not select the real PR head",
    )

    primary = paths_filter_step(changes, "filter", checks)
    python_filter = paths_filter_step(changes, "python_non_windows", checks)

    conditions: dict[str, str] = {}
    verify_fast = job_block(ci, "verify-fast")
    if verify_fast is None:
        checks.require(False, "ci.yml is missing verify-fast")
    else:
        condition = normalized_condition(verify_fast)
        expected = "!cancelled() && ( needs.changes.result != 'success' || needs.changes.outputs.docs_only != 'true' )"
        checks.require(condition == expected, "verify-fast does not have the failure-aware baseline condition")
        checks.require(
            condition is None or "always()" not in condition,
            "verify-fast uses always(), which keeps a superseded run alive through cancel-in-progress",
        )
        if condition is not None:
            conditions["verify-fast"] = condition
        checks.require(re.search(r"^    needs:\s*changes\s*$", verify_fast, re.MULTILINE) is not None, "verify-fast lost needs: changes")

    for job_name, categories in SCOPED_DRIVERS.items():
        block = job_block(ci, job_name)
        if block is None:
            checks.require(False, f"ci.yml is missing scoped job {job_name}")
            continue
        condition = normalized_condition(block)
        expected = expected_scoped_condition(categories)
        checks.require(condition == expected, f"{job_name} does not use its dependency-accurate condition")
        checks.require(re.search(r"^    needs:\s*changes\s*$", block, re.MULTILINE) is not None, f"{job_name} lost needs: changes")
        if condition is not None:
            conditions[job_name] = condition

    scenarios = (
        ("rust-markdown", ("src/rust/crates/fathomdb/README.md",), {"rust": "true"}, set(), False),
        ("python-markdown", ("src/python/README.md",), {"python": "true"}, set(), False),
        ("typescript-markdown", ("src/ts/README.md",), {"typescript": "true"}, set(), False),
        (
            "windows-only",
            ("src/python/tests/test_slice65_wal_attribution_installed.py",),
            {"windows": "true", "python": "false"},
            {"windows-wal-attribution"},
            True,
        ),
        (
            "windows-plus-python",
            (
                "src/python/tests/test_slice65_wal_attribution_installed.py",
                "src/python/fathomdb/database.py",
            ),
            {"windows": "true", "python": "true"},
            {
                "verify",
                "security",
                "default-embedder-tests",
                "wheel-size-gate",
                "native-artifact-runtime-validation",
                "windows-wal-attribution",
            },
            True,
        ),
        ("root-npm-tooling", ("package.json", "package-lock.json"), {"typescript": "false"}, set(), True),
        (
            "rust-source",
            ("src/rust/crates/fathomdb/src/lib.rs",),
            {"rust": "true"},
            set(SCOPED_DRIVERS),
            True,
        ),
        (
            "python-source",
            ("src/python/fathomdb/database.py",),
            {"python": "true"},
            {
                "verify",
                "security",
                "default-embedder-tests",
                "wheel-size-gate",
                "native-artifact-runtime-validation",
                "windows-wal-attribution",
            },
            True,
        ),
        (
            "typescript-source",
            ("src/ts/tests/embed-batch-cls.test.ts",),
            {"typescript": "true"},
            {"verify", "security", "default-embedder-tests", "native-artifact-runtime-validation"},
            True,
        ),
        ("verify-harness", ("scripts/agent-test.sh",), {"verify_harness": "true"}, {"verify"}, True),
        (
            "rust-test-harness",
            ("scripts/test-rust-workspace.sh",),
            {"rust_test_harness": "true"},
            {"verify", "rust-workspace-race-report"},
            True,
        ),
        (
            "security-harness",
            ("scripts/agent-security.sh",),
            {"security_harness": "true"},
            {"security"},
            True,
        ),
        (
            "native-harness",
            ("scripts/release/smoke/smoke-local-native-artifacts.sh",),
            {"native_artifact_harness": "true"},
            {"native-artifact-runtime-validation"},
            True,
        ),
        (
            "ci-workflow-lite",
            (".github/workflows/ci.yml",),
            {"ci_workflow": "true"},
            set(SCOPED_DRIVERS),
            True,
        ),
    )

    with tempfile.TemporaryDirectory(prefix="fathomdb-ci-routing-") as temp:
        work = Path(temp)
        for label, paths, expected_outputs, expected_scoped, expected_verify_fast in scenarios:
            outputs = {category: "false" for category in CATEGORIES}
            outputs["ci_mode"] = "lite" if label == "ci-workflow-lite" else "normal"
            if primary is not None:
                primary_outputs = run_matcher(*primary, paths, work, f"{label}-primary", checks)
                for category in CATEGORIES:
                    if category != "python" and category in primary_outputs:
                        outputs[category] = primary_outputs[category]
                nonmd = primary_outputs.get("nonmd", "")
                outputs["docs_only"] = "true" if nonmd == "false" else "false"
            else:
                outputs["docs_only"] = ""
            if python_filter is not None:
                python_outputs = run_matcher(*python_filter, paths, work, f"{label}-python", checks)
                outputs["python"] = python_outputs.get("python", "")
            for output_name, expected_value in expected_outputs.items():
                checks.require(
                    outputs.get(output_name) == expected_value,
                    f"{label}: {output_name}={outputs.get(output_name)!r}, expected {expected_value!r}",
                )
            selected = evaluate_routes(conditions, outputs, "success", checks, label)
            selected_scoped = selected.intersection(SCOPED_DRIVERS)
            checks.require(
                selected_scoped == expected_scoped,
                f"{label}: selected scoped jobs {sorted(selected_scoped)}, expected {sorted(expected_scoped)}",
            )
            checks.require(
                ("verify-fast" in selected) is expected_verify_fast,
                f"{label}: verify-fast selection did not match non-Markdown policy",
            )

    failed_outputs = {category: "" for category in (*CATEGORIES, "docs_only", "ci_mode")}
    failed_routes = evaluate_routes(conditions, failed_outputs, "failure", checks, "classifier-failure")
    checks.require("verify-fast" in failed_routes, "classifier failure suppresses verify-fast")
    checks.require(
        not failed_routes.intersection(SCOPED_DRIVERS),
        "classifier failure fans out scoped jobs from unknown outputs",
    )
    # A superseded PR run is cancelled (concurrency cancel-in-progress); the
    # classifier reports `cancelled` and nothing may keep running.
    cancelled_routes = evaluate_routes(
        conditions, failed_outputs, "cancelled", checks, "classifier-cancelled", run_cancelled=True
    )
    checks.require(not cancelled_routes, f"cancelled run still selects {sorted(cancelled_routes)}")

    marker_step = step_block(changes, name="Determine CI mode")
    marker_script = block_scalar(marker_step, "run", 8) if marker_step is not None else None
    marker_scenarios = (
        ("trusted-pr", "[ci-lite]", "pull_request", "OWNER", "fathomadb/fathomdb", "lite"),
        ("fork-pr", "[ci-lite]", "pull_request", "CONTRIBUTOR", "fork/fathomdb", "normal"),
        ("untrusted-same-repo", "[ci-lite]", "pull_request", "CONTRIBUTOR", "fathomadb/fathomdb", "normal"),
        ("incidental", "please use [ci-lite] here", "pull_request", "OWNER", "fathomadb/fathomdb", "normal"),
        ("direct-push", "[ci-lite]", "push", "", "", "lite"),
        ("rebased-tip", "[ci-lite]", "push", "", "", "lite"),
        ("squash-tip", "summary\n\n[ci-lite]", "push", "", "", "lite"),
        ("squash-without-marker", "summary only", "push", "", "", "normal"),
    )
    if marker_script is None:
        for label, *_rest in marker_scenarios:
            checks.require(False, f"{label}: changes job is missing the CI mode script")
        checks.require(False, "merge-push: changes job is missing the CI mode script")
    else:
        for label, message, event, association, head_repository, expected in marker_scenarios:
            with tempfile.TemporaryDirectory(prefix=f"fathomdb-ci-mode-{label}-") as temp:
                actual = run_marker_script(
                    marker_script,
                    Path(temp),
                    message=message,
                    event=event,
                    association=association,
                    head_repository=head_repository,
                )
            checks.require(actual == expected, f"{label}: ci_mode={actual!r}, expected {expected!r}")
        with tempfile.TemporaryDirectory(prefix="fathomdb-ci-mode-merge-") as temp:
            actual = run_merge_marker_script(marker_script, Path(temp))
        checks.require(actual == "lite", f"merge-push: ci_mode={actual!r}, expected 'lite'")

    aarch_trigger = aarch.split("permissions:\n", 1)[0]
    checks.require(re.search(r"^  push:\s*$", aarch_trigger, re.MULTILINE) is None, "AArch64 preflight still triggers on push")
    checks.require(re.search(r"^  workflow_dispatch:\s*$", aarch_trigger, re.MULTILINE) is not None, "AArch64 preflight is not dispatchable")
    smoke = SMOKE_PATH.read_text()
    checks.require("cp310-abi3" in smoke, "native artifact smoke does not assert the shipped ABI3 wheel tag")

    gitleaks = job_block(ci, "gitleaks") or ""
    checks.require("gitleaks-current.sh" in gitleaks, "per-push Gitleaks lost current-tree scanning")
    checks.require("gitleaks-history.sh" not in gitleaks, "per-push Gitleaks still scans full history")
    checks.require("fetch-depth: 0" not in gitleaks, "per-push Gitleaks still fetches full history it no longer scans")
    checks.require(HISTORY_PATH.exists(), "dispatchable full-history Gitleaks workflow is missing")
    if HISTORY_PATH.exists():
        history = HISTORY_PATH.read_text()
        history_trigger = history.split("permissions:\n", 1)[0]
        checks.require(re.search(r"^  workflow_dispatch:\s*$", history_trigger, re.MULTILINE) is not None, "history scan is not dispatchable")
        checks.require(re.search(r"^  (?:push|schedule):", history_trigger, re.MULTILINE) is None, "history scan gained an automatic trigger")
        checks.require("fetch-depth: 0" in history, "history scan does not fetch full history")
        checks.require("install-gitleaks.sh" in history and "gitleaks-history.sh" in history, "history workflow does not run the existing guard")

    advisory = job_block(release, "gitleaks-history-advisory")
    checks.require(advisory is not None, "release workflow lacks independent advisory history scan")
    if advisory is not None:
        checks.require("continue-on-error: true" in advisory, "release history scan is not advisory")
        checks.require(re.search(r"^    needs:", advisory, re.MULTILINE) is None, "release history scan is not independent")
        checks.require("fetch-depth: 0" in advisory, "release history scan does not fetch full history")
        checks.require(
            "ref: ${{ env.RELEASE_CHECKOUT_REF }}" in advisory,
            "release history scan does not check out the selected release candidate",
        )
        checks.require("install-gitleaks.sh" in advisory and "gitleaks-history.sh" in advisory, "release history scan does not run the existing guard")
    for publisher in re.finditer(r"^  ((?:publish|promote|github-release)[A-Za-z0-9_-]*):\n", release, re.MULTILINE):
        block = job_block(release, publisher.group(1)) or ""
        checks.require("gitleaks-history-advisory" not in block, f"{publisher.group(1)} depends on advisory CI")

    release_trigger = release.split("permissions:\n", 1)[0]
    checks.require(re.search(r"^    tags:\s*$", release_trigger, re.MULTILINE) is not None, "release workflow lost its tag trigger")
    checks.require("ci_mode" not in release and "[ci-lite]" not in release, "release workflow can be suppressed by lite mode")

    actionlint = shutil.which("actionlint")
    checks.require(actionlint is not None, "actionlint is required for the routing fixture")
    if actionlint is not None:
        completed = subprocess.run(
            [
                actionlint,
                "-config-file",
                str(REPO_ROOT / ".github/actionlint.yaml"),
                str(CI_PATH),
                str(AARCH_PATH),
                str(RELEASE_PATH),
                *([str(HISTORY_PATH)] if HISTORY_PATH.exists() else []),
            ],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        checks.require(completed.returncode == 0, f"actionlint rejected routed workflows: {completed.stdout}{completed.stderr}")

    if checks.failures:
        for failure in checks.failures:
            print(f"FAIL test-ci-proportional-routing: {failure}")
        print(f"test-ci-proportional-routing: {len(checks.failures)} failure(s)")
        return 1
    print("PASS test-ci-proportional-routing")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
