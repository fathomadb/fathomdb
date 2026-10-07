#!/usr/bin/env python3
"""Validate the governed canonical-operation map and observed SDK surface."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any


SCHEMA = "fathomdb.governed-operation-parity/v1"
BINDINGS = ("python", "typescript", "rust")
LOCATORS = {"package", "engine_static", "engine_instance", "admin", "read", "graph"}
RECOVERY_DENYLIST = ["recover", "restore", "repair", "fix", "rebuild"]

# `fathomdb-sdk` members that are not governed operations, matching the members
# the Python and TypeScript oracles exclude (dev/interfaces/rust-sdk.md).
RUST_NON_COMMAND = {
    "engine_instance:attach_subscriber",
    "engine_instance:config",
    "engine_instance:counters",
    "engine_instance:dense_disabled",
    "engine_instance:dense_disabled_reason",
    "engine_instance:drain",
    "engine_instance:enable_telemetry",
    "engine_instance:last_telemetry_query_id",
    "engine_instance:open_report",
    "engine_instance:record_feedback",
    "engine_instance:set_profiling",
    "engine_instance:set_slow_threshold_ms",
    "engine_instance:vector_equivalence_refusal_count",
    "admin:configure_runtime",
    "package:embed_batch_cls",
}
# Trait impls the SDK `Engine` may carry; anything else (e.g. `Deref`) could
# expose the core engine without a governed operation.
RUST_ENGINE_TRAITS = {"Debug", "Drop"}
RUST_NAMESPACE_FILES = {"read.rs": "read", "graph.rs": "graph", "admin.rs": "admin"}


class ParityError(ValueError):
    """The canonical map or an observed binding surface is inconsistent."""


def _strings(value: Any, label: str, *, nonempty: bool = False) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) and item for item in value):
        raise ParityError(f"{label} must be a list of non-empty strings")
    if nonempty and not value:
        raise ParityError(f"{label} must not be empty")
    if len(value) != len(set(value)):
        raise ParityError(f"{label} contains duplicate members")
    return value


def _endpoint(operation: dict[str, Any], binding: str) -> str:
    operation_id = operation.get("id", "<missing>")
    value = operation.get(binding)
    if not isinstance(value, dict):
        raise ParityError(
            f"{operation.get('state', '<missing>')} operation {operation_id} has no {binding} mapping"
        )
    locator = value.get("locator")
    spelling = value.get("spelling")
    if locator not in LOCATORS:
        raise ParityError(
            f"operation {operation_id} {binding} locator {locator!r} is not in {sorted(LOCATORS)}"
        )
    if not isinstance(spelling, str) or not spelling:
        raise ParityError(f"operation {operation_id} {binding} spelling must be non-empty")
    if ":" in spelling:
        raise ParityError(f"operation {operation_id} {binding} spelling must not contain ':'")
    return f"{locator}:{spelling}"


def validate_contract(signed: dict[str, Any], companion: dict[str, Any]) -> None:
    """Validate mapping structure and exact signed-member preservation."""
    if not isinstance(signed, dict) or not isinstance(companion, dict):
        raise ParityError("signed and companion contracts must be JSON objects")
    allowlist = _strings(signed.get("allowlist"), "signed allowlist", nonempty=True)
    denylist = _strings(signed.get("recovery_denylist"), "recovery_denylist")
    if denylist != RECOVERY_DENYLIST:
        raise ParityError(
            f"recovery_denylist must remain exactly {RECOVERY_DENYLIST!r}, got {denylist!r}"
        )
    if companion.get("schema_version") != SCHEMA:
        raise ParityError(f"schema_version must be {SCHEMA!r}")
    operations = companion.get("operations")
    if not isinstance(operations, list) or not operations:
        raise ParityError("operations must be a non-empty list")

    ids: set[str] = set()
    signed_members: list[str] = []
    per_binding: dict[str, dict[str, str]] = {binding: {} for binding in BINDINGS}
    cross_binding: dict[str, str] = {}
    for index, raw in enumerate(operations):
        if not isinstance(raw, dict):
            raise ParityError(f"operations[{index}] must be an object")
        operation_id = raw.get("id")
        if not isinstance(operation_id, str) or not operation_id:
            raise ParityError(f"operations[{index}].id must be a non-empty string")
        if operation_id in ids:
            raise ParityError(f"duplicate canonical operation id {operation_id}")
        ids.add(operation_id)
        state = raw.get("state")
        if state not in {"live", "reserved"}:
            raise ParityError(f"operation {operation_id} has invalid state {state!r}")
        members = _strings(raw.get("signed_members"), f"operation {operation_id} signed_members", nonempty=True)
        signed_members.extend(members)
        for binding in BINDINGS:
            located = _endpoint(raw, binding)
            prior = per_binding[binding].get(located)
            if prior is not None:
                raise ParityError(
                    f"duplicate {binding} locator/spelling {located} for {prior} and {operation_id}"
                )
            per_binding[binding][located] = operation_id
            cross_prior = cross_binding.get(located)
            if cross_prior is not None and cross_prior != operation_id:
                raise ParityError(
                    f"locator/spelling {located} maps across bindings to {cross_prior} and {operation_id}"
                )
            cross_binding[located] = operation_id

    if len(signed_members) != len(set(signed_members)):
        duplicates = sorted({member for member in signed_members if signed_members.count(member) > 1})
        raise ParityError(f"duplicate signed member(s): {', '.join(duplicates)}")
    got = set(signed_members)
    want = set(allowlist)
    if got != want:
        added = sorted(got - want)
        removed = sorted(want - got)
        details = []
        if added:
            details.append(f"ADDED {', '.join(added)}")
        if removed:
            details.append(f"REMOVED {', '.join(removed)}")
        raise ParityError("signed member flattening differs from allowlist: " + "; ".join(details))


def live_canonical_ids(companion: dict[str, Any]) -> set[str]:
    return {
        operation["id"]
        for operation in companion["operations"]
        if operation.get("state") == "live"
    }


def validate_observed(
    companion: dict[str, Any], binding: str, observed: set[str] | list[str]
) -> set[str]:
    """Require one binding's observed commands to equal its live mapping."""
    if binding not in BINDINGS:
        raise ParityError(f"binding must be one of {BINDINGS!r}, got {binding!r}")
    if not isinstance(observed, (set, list)) or not all(
        isinstance(item, str) and item for item in observed
    ):
        raise ParityError("observed surface must be a set/list of non-empty strings")
    observed_set = set(observed)
    expected: dict[str, str] = {}
    reserved: dict[str, str] = {}
    for operation in companion["operations"]:
        located = _endpoint(operation, binding)
        target = expected if operation["state"] == "live" else reserved
        target[located] = operation["id"]
    reserved_hits = sorted(observed_set & set(reserved))
    if reserved_hits:
        located = reserved_hits[0]
        raise ParityError(
            f"{binding} observed reserved operation {reserved[located]} at {located}"
        )
    extra = sorted(observed_set - set(expected))
    missing = sorted(set(expected) - observed_set)
    if extra or missing:
        details = []
        if extra:
            details.append(
                "observed ungoverned command(s): "
                + ", ".join(f"{located} (<ungoverned>)" for located in extra)
            )
        if missing:
            details.append(
                "missing live command(s): "
                + ", ".join(f"{located} ({expected[located]})" for located in missing)
            )
        raise ParityError(f"{binding} " + "; ".join(details))
    return {expected[located] for located in observed_set}


def _blank_literals(source: str) -> str:
    """Replace comments, string and char literals with spaces, keeping offsets."""
    out = list(source)
    i = 0
    n = len(source)
    while i < n:
        if source.startswith("//", i):
            end = source.find("\n", i)
            end = n if end < 0 else end
        elif source.startswith("/*", i):
            end = source.find("*/", i)
            end = n if end < 0 else end + 2
        elif source[i] == '"' or source.startswith('r"', i) or source.startswith('r#"', i):
            if source[i] == "r":
                hashes = len(source[i + 1 :]) - len(source[i + 1 :].lstrip("#"))
                close = '"' + "#" * hashes
                end = source.find(close, i + 2 + hashes)
                end = n if end < 0 else end + len(close)
            else:
                end = i + 1
                while end < n and source[end] != '"':
                    end += 2 if source[end] == "\\" else 1
                end = min(end + 1, n)
        elif re.match(r"'(\\.|[^\\'])'", source[i : i + 4]):
            end = i + len(re.match(r"'(\\.|[^\\'])'", source[i : i + 4]).group(0))
        else:
            i += 1
            continue
        for j in range(i, end):
            if out[j] != "\n":
                out[j] = " "
        i = end
    return "".join(out)


def _block_end(text: str, open_brace: int) -> int:
    depth = 0
    for index in range(open_brace, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return index + 1
    raise ParityError("unbalanced braces in Rust SDK source")


def _strip_cfg_test(text: str) -> str:
    for match in reversed(list(re.finditer(r"#\[cfg\(test\)\]\s*(pub\s+)?mod\s+\w+\s*\{", text))):
        end = _block_end(text, match.end() - 1)
        text = text[: match.start()] + " " * (end - match.start()) + text[end:]
    return text


_PUB_FN = re.compile(
    r"(?<![\w)])pub\s+(?:(?:const|async|unsafe|extern\s+\"[^\"]*\")\s+)*fn\s+(\w+)"
)
_CORE_ONLY_NAMES = {"Engine", "OpenedEngine", "EmbedderChoice"}


def _core_aliases(text: str) -> set[str]:
    """Local names bound to the core `Engine` by a `use` of `fathomdb_engine`."""
    aliases = {"fathomdb_engine::Engine"}
    for use in re.finditer(r"\buse\s+(?:::)?fathomdb_engine\b([^;]*);", text):
        for name, alias in re.findall(r"\b(Engine|OpenedEngine)\b(?:\s+as\s+(\w+))?", use.group(1)):
            aliases.add(alias or name)
    return aliases


def _refuse_core_leaks(path: Path, text: str) -> None:
    """Refuse type-level routes to the core that a `pub fn` scan cannot see."""
    for item in re.finditer(
        r"(?<![\w)])pub\s+(use|type|extern\s+crate)\s+([^;]+);", text
    ):
        kind, target = item.group(1), " ".join(item.group(2).split())
        if "fathomdb_engine" not in target:
            continue
        if kind != "use":
            raise ParityError(f"pub {kind} naming fathomdb_engine in {path.name}: {target}")
        target = target.removeprefix("::")
        if not target.startswith("fathomdb_engine::"):
            raise ParityError(f"re-export of the fathomdb_engine crate itself in {path.name}: {target}")
        if re.search(r"(::|\{|,)\s*(\*|self\b)", target):
            raise ParityError(f"glob or self re-export {target} in {path.name}")
        leaves = re.findall(r"(?:::|\{|,)\s*(\w+)(?=\s*(?:as\b|,|\}|$))", target)
        leaked = sorted(set(leaves) & _CORE_ONLY_NAMES)
        if leaked:
            raise ParityError(f"re-export of core {', '.join(leaked)} from {target} in {path.name}")
        functions = sorted(name for name in leaves if name[0].islower())
        if functions:
            raise ParityError(
                f"re-export of fathomdb_engine function(s) {', '.join(functions)} in {path.name}; "
                "wrap them as governed SDK functions"
            )
    aliases = _core_aliases(text)
    core = re.compile("|".join(rf"(?<![\w:]){re.escape(a)}\b" for a in sorted(aliases)))
    for fn in _PUB_FN.finditer(text):
        signature = text[fn.end() : text.find("{", fn.end())]
        if core.search(signature):
            raise ParityError(f"core engine type in public fn {fn.group(1)} signature in {path.name}")
    for impl in re.finditer(r"\bimpl\b[^{;]*\{", text):
        if core.search(impl.group(0)):
            raise ParityError(f"impl naming the core engine in {path.name}: {impl.group(0)[:-1].strip()}")
    for tuple_struct in re.finditer(r"\bpub\s+struct\s+Engine\s*\(([^;]*)\)\s*;", text):
        if re.search(r"(?<![\w)])pub\b", tuple_struct.group(1)):
            raise ParityError(f"public field on SDK Engine in {path.name}")
    for struct in re.finditer(r"\bpub\s+struct\s+Engine\s*\{", text):
        body = text[struct.end() : _block_end(text, struct.end() - 1) - 1]
        if re.search(r"(?<![\w)])pub\b", body):
            raise ParityError(f"public field on SDK Engine in {path.name}")
    if path.name == "lib.rs":
        for module in re.finditer(r"(?<![\w)])pub\s+mod\s+(\w+)\s*([;{])", text):
            name, opener = module.group(1), module.group(2)
            if opener == ";" and f"{name}.rs" not in RUST_NAMESPACE_FILES:
                raise ParityError(f"pub mod {name} in lib.rs is not a governed namespace")
            if opener == "{":
                body = text[module.end() : _block_end(text, module.end() - 1) - 1]
                if body.strip():
                    raise ParityError(f"pub mod {name} in lib.rs must be empty")


def observe_rust_sdk(crate: Path) -> set[str]:
    """Observe `fathomdb-sdk` governed members from its source tree.

    Engine members are every `pub fn` in any `impl Engine` block; namespace
    members are top-level `pub fn` in `read.rs`, `graph.rs` and `admin.rs`;
    every other top-level `pub fn` counts as a package root member (fail
    closed). `pub(crate)` items, `#[cfg(test)]` modules and methods of other
    types are not surface. Unapproved trait impls for `Engine`, public
    `Engine` fields, glob or core-`Engine` re-exports of `fathomdb_engine`, and
    `pub mod`s other than the namespaces (or an empty doc module) are refused.
    Re-exported data types are pinned by the crate's consumer test instead.
    """
    src = crate / "src"
    files = sorted(src.rglob("*.rs"))
    if not files:
        raise ParityError(f"no Rust sources under {src}")
    observed: set[str] = set()
    for path in files:
        text = _strip_cfg_test(_blank_literals(path.read_text(encoding="utf-8")))
        _refuse_core_leaks(path, text)
        impl_spans: list[tuple[int, int]] = []
        for match in re.finditer(r"\bimpl\b(?:\s*<[^{]*?>)?\s+([^{;]+?)\s*\{", text):
            header = " ".join(match.group(1).split())
            end = _block_end(text, match.end() - 1)
            impl_spans.append((match.start(), end))
            trait, _, target = header.partition(" for ")
            target = target or trait
            if re.search(r"(^|::)Engine$", target.strip()):
                if _:
                    trait_name = re.sub(r"<.*", "", trait.strip()).split("::")[-1]
                    if trait_name not in RUST_ENGINE_TRAITS:
                        raise ParityError(
                            f"trait impl {trait.strip()} for Engine in {path.name} is not an approved SDK surface"
                        )
                    continue
                body = text[match.end() : end - 1]
                for fn in _PUB_FN.finditer(body):
                    signature = body[fn.end() : body.find("{", fn.end())]
                    locator = "engine_instance" if "self" in signature else "engine_static"
                    observed.add(f"{locator}:{fn.group(1)}")
        locator = RUST_NAMESPACE_FILES.get(path.name, "package")
        for fn in _PUB_FN.finditer(text):
            if any(start <= fn.start() < end for start, end in impl_spans):
                continue
            observed.add(f"{locator}:{fn.group(1)}")
    return observed - RUST_NON_COMMAND


def _load(path: Path) -> dict[str, Any]:
    try:
        with path.open(encoding="utf-8") as handle:
            value = json.load(handle)
    except (OSError, json.JSONDecodeError) as error:
        raise ParityError(f"cannot load {path}: {error}") from error
    if not isinstance(value, dict):
        raise ParityError(f"{path} must contain a JSON object")
    return value


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--signed", type=Path, default=Path("src/conformance/governed-surface-allowlist.json")
    )
    parser.add_argument(
        "--companion", type=Path, default=Path("src/conformance/governed-operation-parity.json")
    )
    parser.add_argument("--binding", choices=BINDINGS)
    parser.add_argument("--observed-json")
    parser.add_argument(
        "--rust-crate", type=Path, help="observe the rust binding from a fathomdb-sdk crate"
    )
    args = parser.parse_args(argv)
    try:
        signed = _load(args.signed)
        companion = _load(args.companion)
        validate_contract(signed, companion)
        if args.binding is None:
            if args.observed_json is not None:
                raise ParityError("--observed-json requires --binding")
            print(
                "ok    sdk-surface-parity: companion preserves "
                f"{len(signed['allowlist'])} signed members / "
                f"{len(live_canonical_ids(companion))} live canonical operations"
            )
            return 0
        if args.rust_crate is not None:
            if args.binding != "rust" or args.observed_json is not None:
                raise ParityError("--rust-crate requires --binding rust and no --observed-json")
            observed = observe_rust_sdk(args.rust_crate)
        elif args.observed_json is None:
            raise ParityError("--binding requires --observed-json or --rust-crate")
        else:
            try:
                observed = json.loads(args.observed_json)
            except json.JSONDecodeError as error:
                raise ParityError(f"observed JSON is invalid: {error}") from error
        canonical = validate_observed(companion, args.binding, observed)
        expected = live_canonical_ids(companion)
        if canonical != expected:
            raise ParityError(
                f"{args.binding} canonical set differs: got {sorted(canonical)!r}, "
                f"expected {sorted(expected)!r}"
            )
        print(
            f"ok    sdk-surface-parity: {args.binding} "
            f"{len(canonical)}/{len(expected)} live canonical operations"
        )
        return 0
    except ParityError as error:
        print(f"FAIL  sdk-surface-parity: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
