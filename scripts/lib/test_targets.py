#!/usr/bin/env python3
"""Read every workspace test target and the feature set it requires.

Shared by the feature-complete test gate (`scripts/test-feature-complete.sh`),
the fast-tier coverage check (`scripts/check-test-target-coverage.py`), and the
0.8.27 hidden-surface oracle. A target's requirements are its Cargo
`required-features` plus any file-level `#![cfg(...)]`, parsed here by a small
parser for Rust source cfgs. Nothing is built.

`--write-matrix` regenerates `scripts/test-feature-matrix.toml`, the committed
list of per-crate requirement sets; the coverage check fails when the source
and that file disagree, so a new set arrives as a reviewed diff.
"""

from __future__ import annotations

import argparse
from itertools import combinations
import json
from pathlib import Path
import re
import subprocess
import sys
from typing import Any, Callable, Iterable, Sequence

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.10 fallback
    import tomli as tomllib  # type: ignore[import-not-found,no-redef]


REPO_ROOT = Path(__file__).resolve().parents[2]
MATRIX_PATH = REPO_ROOT / "scripts" / "test-feature-matrix.toml"
ALLOWLIST_PATH = REPO_ROOT / "scripts" / "test-skip-allowlist.toml"
MAX_CANONICAL_ATOMS = 12
ALLOWLIST_CLASSES = frozenset(
    {"ignored-by-design", "opt-in-experiment", "benign-message", "platform-excluded"}
)
PROFILE_ATOMS = {"debug_assertions": True, "test": True}
PLATFORM_NAMES = frozenset(
    {
        "unix",
        "windows",
        "target_os",
        "target_arch",
        "target_family",
        "target_env",
        "target_vendor",
        "target_pointer_width",
        "target_endian",
    }
)

Cfg = tuple  # ("atom", name, value|None) | ("not", [x]) | ("any", [...]) | ("all", [...])


class TestTargetsError(Exception):
    """Unreadable manifests, unparseable cfgs, or unsatisfiable requirements."""


# --------------------------------------------------------------------------
# Rust cfg syntax


_TOKEN = re.compile(r'\s*(?:(?P<ident>[A-Za-z_][A-Za-z0-9_]*)|(?P<string>"(?:[^"\\]|\\.)*")|(?P<punct>[(),=]))')


def parse_cfg(text: str) -> Cfg:
    """Parse the inside of a `cfg(...)` attribute into a predicate tree."""

    tokens: list[tuple[str, str]] = []
    position = 0
    stripped = text.rstrip()
    while position < len(stripped):
        match = _TOKEN.match(stripped, position)
        if match is None:
            raise TestTargetsError(f"unparseable cfg {text!r} at {position}")
        kind = match.lastgroup or ""
        tokens.append((kind, match.group(kind)))
        position = match.end()
    index = 0

    def peek(value: str) -> bool:
        return index < len(tokens) and tokens[index][1] == value

    def expect(value: str) -> None:
        nonlocal index
        if not peek(value):
            raise TestTargetsError(f"unparseable cfg {text!r}: expected {value!r}")
        index += 1

    def predicate() -> Cfg:
        nonlocal index
        if index >= len(tokens) or tokens[index][0] != "ident":
            raise TestTargetsError(f"unparseable cfg {text!r}: expected a name")
        name = tokens[index][1]
        index += 1
        if name in ("any", "all", "not") and peek("("):
            expect("(")
            operands: list[Cfg] = []
            while not peek(")"):
                operands.append(predicate())
                if not peek(")"):
                    expect(",")
            expect(")")
            if name == "not" and len(operands) != 1:
                raise TestTargetsError(f"unparseable cfg {text!r}: not() takes one predicate")
            return (name, operands)
        if peek("="):
            expect("=")
            if index >= len(tokens) or tokens[index][0] != "string":
                raise TestTargetsError(f"unparseable cfg {text!r}: expected a string")
            value = tokens[index][1][1:-1]
            index += 1
            return ("atom", name, value)
        return ("atom", name, None)

    result = predicate()
    if index != len(tokens):
        raise TestTargetsError(f"unparseable cfg {text!r}: trailing tokens")
    return result


def render_atom(atom: Cfg) -> str:
    return atom[1] if atom[2] is None else f'{atom[1]} = "{atom[2]}"'


def cfg_atoms(node: Cfg) -> set[Cfg]:
    if node[0] == "atom":
        return {node}
    return set().union(*(cfg_atoms(child) for child in node[1])) if node[1] else set()


def evaluate(node: Cfg, assign: Callable[[Cfg], bool]) -> bool:
    if node[0] == "atom":
        return assign(node)
    if node[0] == "not":
        return not evaluate(node[1][0], assign)
    if node[0] == "any":
        return any(evaluate(child, assign) for child in node[1])
    return all(evaluate(child, assign) for child in node[1])


def conjunction(nodes: Iterable[Cfg]) -> Cfg | None:
    items = list(nodes)
    if not items:
        return None
    return items[0] if len(items) == 1 else ("all", items)


def canonical_cfg(node: Cfg | None) -> str | None:
    """Sign a predicate semantically: the minimal sum-of-products over its
    sorted atoms (fewest terms, then fewest literals, then lexicographically
    smallest rendering). `None` means always true; `any()` never true."""

    if node is None:
        return None
    atoms = sorted(cfg_atoms(node), key=render_atom)
    if len(atoms) > MAX_CANONICAL_ATOMS:
        raise TestTargetsError(
            f"cfg has {len(atoms)} atoms; more than {MAX_CANONICAL_ATOMS} is not canonicalized"
        )
    width = len(atoms)
    names = [render_atom(atom) for atom in atoms]
    position = {atom: n for n, atom in enumerate(atoms)}
    true_rows = [
        row
        for row in range(1 << width)
        if evaluate(node, lambda atom: bool(row >> position[atom] & 1))
    ]
    if len(true_rows) == 1 << width:
        return None
    if not true_rows:
        return "any()"
    primes = _prime_implicants(true_rows, width)

    def covers(term: tuple[int, int], row: int) -> bool:
        value, care = term
        return row & care == value

    def literals(term: tuple[int, int]) -> list[str]:
        value, care = term
        rendered = [
            names[bit] if value >> bit & 1 else f"not({names[bit]})"
            for bit in range(width)
            if care >> bit & 1
        ]
        return sorted(rendered)

    def render_term(term: tuple[int, int]) -> str:
        parts = literals(term)
        return parts[0] if len(parts) == 1 else f"all({', '.join(parts)})"

    # Every essential prime implicant is in every minimal cover; search only
    # the rows they leave uncovered.
    rows_to_primes = {row: [p for p in primes if covers(p, row)] for row in true_rows}
    essential = sorted({ps[0] for ps in rows_to_primes.values() if len(ps) == 1})
    remaining = [row for row in true_rows if not any(covers(p, row) for p in essential)]
    optional = [p for p in primes if p not in essential and any(covers(p, row) for row in remaining)]

    def score(chosen: Sequence[tuple[int, int]]) -> tuple[int, int, str]:
        terms = sorted(render_term(term) for term in chosen)
        text = terms[0] if len(terms) == 1 else f"any({', '.join(terms)})"
        return (len(chosen), sum(len(literals(term)) for term in chosen), text)

    best: tuple[int, int, str] | None = None
    for size in range(0, len(optional) + 1):
        for extra in combinations(optional, size):
            if all(any(covers(term, row) for term in extra) for row in remaining):
                key = score(essential + list(extra))
                if best is None or key < best:
                    best = key
        if best is not None:
            return best[2]
    raise TestTargetsError("cfg canonicalization found no cover")  # unreachable


def _prime_implicants(rows: list[int], width: int) -> list[tuple[int, int]]:
    """Quine-McCluskey prime implicants as (value, care-mask) pairs."""

    full = (1 << width) - 1
    current = {(row, full) for row in rows}
    primes: set[tuple[int, int]] = set()
    while current:
        merged: set[tuple[int, int]] = set()
        used: set[tuple[int, int]] = set()
        for value, care in current:
            for bit in range(width):
                mask = 1 << bit
                if care & mask and value & mask == 0 and (value | mask, care) in current:
                    merged.add((value, care & ~mask))
                    used.update(((value, care), (value | mask, care)))
        primes.update(current - used)
        current = merged
    return sorted(primes)


def canonical_cfg_text(text: str) -> str | None:
    return canonical_cfg(parse_cfg(text))


def host_assignment(triple: str) -> Callable[[Cfg], bool]:
    """Truth of platform and profile atoms for a test build on `triple`.
    Features are not handled here; unknown names fail closed."""

    parts = triple.split("-")
    arch = parts[0]
    os_name = "linux" if "linux" in parts else ("windows" if "windows" in parts else parts[2] if len(parts) > 2 else "")
    if os_name == "darwin":
        os_name = "macos"
    family = "windows" if os_name == "windows" else "unix"
    env = parts[3] if len(parts) > 3 else ""
    if env.startswith("gnu"):
        env = "gnu"
    values = {
        "target_arch": arch,
        "target_os": os_name,
        "target_family": family,
        "target_env": env,
        "target_vendor": parts[1] if len(parts) > 1 else "",
        "target_pointer_width": "32" if arch in ("i686", "armv7", "arm") else "64",
        "target_endian": "little",
    }

    def assign(atom: Cfg) -> bool:
        name, value = atom[1], atom[2]
        if name in PROFILE_ATOMS and value is None:
            return PROFILE_ATOMS[name]
        if name == "unix" and value is None:
            return family == "unix"
        if name == "windows" and value is None:
            return family == "windows"
        if name in values and value is not None:
            return values[name] == value
        raise TestTargetsError(f"cannot evaluate cfg atom {render_atom(atom)!r}")

    return assign


# --------------------------------------------------------------------------
# Workspace reading


class Target:
    """One integration-test target: `source` is relative to its crate."""

    __slots__ = ("crate", "name", "source", "required_features", "ast", "cfg")

    def __init__(
        self,
        crate: str,
        name: str,
        source: str,
        required_features: tuple[str, ...],
        ast: Cfg | None,
    ) -> None:
        self.crate = crate
        self.name = name
        self.source = source
        self.required_features = required_features
        self.ast = ast
        self.cfg = canonical_cfg(ast)

    @property
    def id(self) -> str:
        return f"{self.crate}::{self.name}"


class Crate:
    """A workspace member with its feature graph and test targets."""

    __slots__ = ("name", "directory", "features", "targets")

    def __init__(
        self, name: str, directory: Path, features: dict[str, list[str]], targets: list[Target]
    ) -> None:
        self.name = name
        self.directory = directory
        self.features = features
        self.targets = targets


def _load_toml(path: Path) -> dict[str, Any]:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise TestTargetsError(f"cannot read {path}: {exc}") from exc


_INNER_ATTRIBUTE = re.compile(r"#!\[\s*cfg\s*\(")


def file_cfg(text: str) -> Cfg | None:
    """The conjunction of the leading `#![cfg(...)]` inner attributes."""

    position = 0
    found: list[Cfg] = []
    while position < len(text):
        if text[position].isspace():
            position += 1
            continue
        if text.startswith("//", position):
            end = text.find("\n", position)
            position = len(text) if end < 0 else end + 1
            continue
        if text.startswith("/*", position):
            end = text.find("*/", position)
            position = len(text) if end < 0 else end + 2
            continue
        if not text.startswith("#![", position):
            break
        end = _bracket_end(text, position + 2)
        attribute = text[position:end]
        match = _INNER_ATTRIBUTE.match(attribute)
        if match is not None:
            inner = attribute[match.end() : attribute.rindex(")")]
            found.append(parse_cfg(inner))
        position = end
    return conjunction(found)


def _bracket_end(text: str, position: int) -> int:
    depth = 0
    in_string = False
    while position < len(text):
        char = text[position]
        if in_string:
            if char == "\\":
                position += 1
            elif char == '"':
                in_string = False
        elif char == '"':
            in_string = True
        elif char == "[":
            depth += 1
        elif char == "]":
            depth -= 1
            if depth == 0:
                return position + 1
        position += 1
    raise TestTargetsError("unterminated attribute")


def _read_crate(directory: Path) -> Crate:
    manifest = _load_toml(directory / "Cargo.toml")
    package = manifest.get("package") or {}
    name = package.get("name")
    if not isinstance(name, str):
        raise TestTargetsError(f"{directory / 'Cargo.toml'} has no package name")
    features = {k: list(v) for k, v in (manifest.get("features") or {}).items()}
    declared: dict[str, tuple[str, tuple[str, ...]]] = {}
    targets: list[Target] = []
    for entry in manifest.get("test") or []:
        target_name = entry["name"]
        source = entry.get("path", f"tests/{target_name}.rs")
        if not (directory / source).is_file() and "path" not in entry:
            nested = f"tests/{target_name}/main.rs"
            if (directory / nested).is_file():
                source = nested
        declared[source] = (target_name, tuple(sorted(entry.get("required-features", []))))
    discovered: list[str] = []
    if package.get("autotests", True) and (directory / "tests").is_dir():
        discovered = sorted(
            [p.relative_to(directory).as_posix() for p in (directory / "tests").glob("*.rs")]
            + [p.relative_to(directory).as_posix() for p in (directory / "tests").glob("*/main.rs")]
        )
    for source in sorted(set(declared) | set(discovered)):
        if source in declared:
            target_name, required = declared[source]
        else:
            target_name = Path(source).parent.name if source.endswith("/main.rs") else Path(source).stem
            required = ()
        path = directory / source
        if not path.is_file():
            raise TestTargetsError(f"{name}: test target {target_name} has no source {source}")
        targets.append(
            Target(name, target_name, source, required, file_cfg(path.read_text(encoding="utf-8")))
        )
    targets.sort(key=lambda target: target.name)
    return Crate(name, directory, features, targets)


def read_workspace(root: Path) -> list[Crate]:
    """Every workspace member (or the root package alone) with its targets."""

    manifest = _load_toml(root / "Cargo.toml")
    members = (manifest.get("workspace") or {}).get("members")
    if members:
        directories = [root / member for member in members]
    elif "package" in manifest:
        directories = [root]
    else:
        raise TestTargetsError(f"{root / 'Cargo.toml'} has no members or package")
    return sorted((_read_crate(d) for d in directories), key=lambda crate: crate.name)


def feature_closure(crate: Crate, features: Iterable[str]) -> frozenset[str]:
    """Features enabled by `features` through the crate's own feature graph."""

    enabled: set[str] = set()
    pending = list(features)
    while pending:
        feature = pending.pop()
        if feature in enabled:
            continue
        enabled.add(feature)
        for implied in crate.features.get(feature, []):
            if "/" not in implied and not implied.startswith("dep:"):
                pending.append(implied)
    return frozenset(enabled)


def _satisfied(
    target: Target, crate: Crate, enabled: frozenset[str], platform: Callable[[Cfg], bool]
) -> bool:
    if not set(target.required_features) <= enabled:
        return False
    if target.ast is None:
        return True

    def assign(atom: Cfg) -> bool:
        if atom[1] == "feature" and atom[2] is not None:
            return atom[2] in enabled
        return platform(atom)

    return evaluate(target.ast, assign)


def requirement(crate: Crate, target: Target) -> tuple[str, ...]:
    """The smallest feature set (then lexicographically first) under which the
    target compiles in a dev test build on some platform."""

    mentioned = {
        atom[2]
        for atom in (cfg_atoms(target.ast) if target.ast is not None else set())
        if atom[1] == "feature" and atom[2] is not None and atom[2] in crate.features
    }
    candidates = sorted(mentioned | set(target.required_features))
    platform_atoms = sorted(
        (
            atom
            for atom in (cfg_atoms(target.ast) if target.ast is not None else set())
            if atom[1] != "feature"
        ),
        key=render_atom,
    )
    for atom in platform_atoms:
        if atom[1] not in PLATFORM_NAMES and atom[1] not in PROFILE_ATOMS:
            raise TestTargetsError(f"{target.id}: cannot evaluate cfg atom {render_atom(atom)!r}")
    for size in range(len(candidates) + 1):
        for chosen in combinations(candidates, size):
            enabled = feature_closure(crate, chosen)
            if not set(target.required_features) <= enabled:
                continue
            for row in range(1 << len(platform_atoms)):
                truth = {atom: bool(row >> n & 1) for n, atom in enumerate(platform_atoms)}

                def platform(atom: Cfg, truth: dict[Cfg, bool] = truth) -> bool:
                    if atom[1] in PROFILE_ATOMS and atom[2] is None:
                        return PROFILE_ATOMS[atom[1]]
                    return truth[atom]

                if _satisfied(target, crate, enabled, platform):
                    return tuple(chosen)
    raise TestTargetsError(f"{target.id}: no feature set satisfies its requirements")


def derive_matrix(crates: Iterable[Crate]) -> list[tuple[str, tuple[str, ...]]]:
    """Every distinct (crate, requirement set), sorted."""

    return sorted({(crate.name, requirement(crate, target)) for crate in crates for target in crate.targets})


def render_matrix(entries: Iterable[tuple[str, tuple[str, ...]]]) -> str:
    lines = [
        "# Generated by `python3 scripts/lib/test_targets.py --write-matrix`; do not edit.",
        "# Every distinct feature set a workspace test target requires (Cargo",
        "# `required-features` plus file-level `#![cfg(...)]`). The feature-complete",
        "# gate runs each set's targets that the workspace gate cannot, and",
        "# scripts/check-test-target-coverage.py fails when the source drifts from it.",
    ]
    for crate, features in sorted(set(entries)):
        lines += ["", "[[entry]]", f"crate = {json.dumps(crate)}", f"features = {json.dumps(list(features))}"]
    return "\n".join(lines) + "\n"


def load_matrix(path: Path) -> list[tuple[str, tuple[str, ...]]]:
    data = _load_toml(path)
    return sorted((entry["crate"], tuple(entry["features"])) for entry in data.get("entry", []))


def load_allowlist(path: Path) -> list[dict[str, str]]:
    data = _load_toml(path)
    return [dict(entry) for entry in data.get("entry", [])]


# --------------------------------------------------------------------------
# Coverage


def _index(crates: Sequence[Crate]) -> dict[str, tuple[Crate, Target]]:
    return {target.id: (crate, target) for crate in crates for target in crate.targets}


def _entry_target(entry_id: str, targets: dict[str, tuple[Crate, Target]]) -> str | None:
    parts = entry_id.split("::")
    if len(parts) >= 2 and "::".join(parts[:2]) in targets:
        return "::".join(parts[:2])
    return None


def check_coverage(
    crates: Sequence[Crate],
    workspace_features: dict[str, frozenset[str]],
    matrix: Sequence[tuple[str, tuple[str, ...]]],
    allowlist: Sequence[dict[str, str]],
    host: str,
) -> tuple[list[str], list[str]]:
    """Return (failures, targets only the feature-complete gate covers)."""

    failures: list[str] = []
    platform = host_assignment(host)
    derived = derive_matrix(crates)
    committed = sorted(set(matrix))
    if derived != committed:
        added = sorted(set(derived) - set(committed))
        removed = sorted(set(committed) - set(derived))
        failures.append(
            "feature matrix drift (run `python3 scripts/lib/test_targets.py --write-matrix`): "
            f"source-only {added}, matrix-only {removed}"
        )
    by_crate = {crate.name: crate for crate in crates}
    targets = _index(crates)
    excluded = {
        entry["id"]
        for entry in allowlist
        if entry.get("class") == "platform-excluded"
    }
    gate_only: list[str] = []
    covered_on_host: dict[str, bool] = {}
    for target_id, (crate, target) in sorted(targets.items()):
        workspace = feature_closure(crate, workspace_features.get(crate.name, frozenset()))
        by_workspace = _satisfied(target, crate, workspace, platform)
        # The gate runs a target only under its own requirement set.
        needed = requirement(crate, target)
        by_matrix = (crate.name, needed) in committed and _satisfied(
            target, crate, feature_closure(crate, needed), platform
        )
        covered_on_host[target_id] = by_workspace or by_matrix
        if by_workspace:
            continue
        if by_matrix:
            gate_only.append(target_id)
        elif target_id not in excluded:
            failures.append(
                f"{target_id} is not covered by the workspace gate or the feature matrix "
                f"(requires {list(requirement(crate, target))}"
                f"{', cfg ' + target.cfg if target.cfg else ''})"
            )
    for entry in allowlist:
        entry_id = str(entry.get("id", ""))
        entry_class = entry.get("class")
        if entry_class not in ALLOWLIST_CLASSES:
            failures.append(f"allowlist entry {entry_id!r} has unknown class {entry_class!r}")
            continue
        if not str(entry.get("reason", "")).strip():
            failures.append(f"allowlist entry {entry_id!r} has no reason")
            continue
        target_id = _entry_target(entry_id, targets)
        if target_id is None:
            failures.append(f"stale allowlist entry {entry_id!r}: no such test target")
            continue
        if entry_class == "platform-excluded":
            _, target = targets[target_id]
            has_platform = target.ast is not None and any(
                atom[1] in PLATFORM_NAMES for atom in cfg_atoms(target.ast)
            )
            if entry_id != target_id or not has_platform or covered_on_host[target_id]:
                failures.append(
                    f"stale allowlist entry {entry_id!r}: target is not platform-excluded on {host}"
                )
        elif target_id not in gate_only:
            failures.append(
                f"stale allowlist entry {entry_id!r}: the feature-complete gate does not run {target_id}"
            )
    del by_crate
    return failures, gate_only


def gate_plan(
    crates: Sequence[Crate],
    workspace_features: dict[str, frozenset[str]],
    matrix: Sequence[tuple[str, tuple[str, ...]]],
    allowlist: Sequence[dict[str, str]],
    host: str,
) -> list[tuple[str, tuple[str, ...], list[str]]]:
    """Per matrix entry, the targets requiring exactly that set that the
    workspace gate does not run on this host."""

    platform = host_assignment(host)
    plan: list[tuple[str, tuple[str, ...], list[str]]] = []
    by_crate = {crate.name: crate for crate in crates}
    for name, features in sorted(set(matrix)):
        crate = by_crate.get(name)
        if crate is None:
            continue
        workspace = feature_closure(crate, workspace_features.get(name, frozenset()))
        enabled = feature_closure(crate, features)
        chosen = [
            target.name
            for target in crate.targets
            if requirement(crate, target) == features
            and not _satisfied(target, crate, workspace, platform)
            and _satisfied(target, crate, enabled, platform)
        ]
        if chosen:
            plan.append((name, features, sorted(chosen)))
    return plan


def host_triple() -> str:
    output = subprocess.run(
        ["rustc", "-vV"], cwd=REPO_ROOT, check=True, capture_output=True, text=True
    ).stdout
    for line in output.splitlines():
        if line.startswith("host:"):
            return line.split(":", 1)[1].strip()
    raise TestTargetsError("rustc -vV reported no host")


def workspace_features(root: Path, host: str) -> dict[str, frozenset[str]]:
    """Per-member features `cargo test --workspace` enables on `host`."""

    completed = subprocess.run(
        [
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--offline",
            "--filter-platform",
            host,
        ],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise TestTargetsError(f"cargo metadata failed:\n{completed.stderr}")
    metadata = json.loads(completed.stdout)
    members = set(metadata["workspace_members"])
    names = {package["id"]: package["name"] for package in metadata["packages"]}
    return {
        names[node["id"]]: frozenset(node["features"])
        for node in metadata["resolve"]["nodes"]
        if node["id"] in members
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    parser.add_argument("--write-matrix", action="store_true")
    args = parser.parse_args(argv)
    try:
        crates = read_workspace(args.root)
        if args.write_matrix:
            path = args.root / "scripts" / "test-feature-matrix.toml"
            path.write_text(render_matrix(derive_matrix(crates)), encoding="utf-8")
            print(f"wrote {path}")
            return 0
        for crate in crates:
            for target in crate.targets:
                print(f"{target.id}\t{list(requirement(crate, target))}\t{target.cfg or ''}")
        return 0
    except TestTargetsError as exc:
        print(f"test-targets: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
