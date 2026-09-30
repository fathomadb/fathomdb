"""Bounded architectural witnesses, compiled together then checked in isolation."""
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

root = Path(sys.argv[1]).resolve()
gate = root / "dev/tools/module-boundary-gate/target/debug/fathomdb-module-boundary-gate"
source = Path("src/rust/crates/fathomdb-engine/src")
policy = Path("dev/tools/module-boundary-policy.txt")
features = "test-hooks,operator,tc5-benchmark"
# (id, diagnostic, file/body edits): all are independent compiling architecture
# witnesses unless marked parser/policy below. The bundle uses unique item names.
cases = []

def case(name, diagnostic, *edits, kind="compiled"):
    cases.append((name, diagnostic, edits, kind))

# Every mandatory dependency direction; a referenced carrier compiles, while
# importing the module tests direction without manufacturing arbitrary calls.
for index, (owner, target) in enumerate([
    ("filter", "search"), ("frozen_read", "search"), ("read", "search"),
    ("read", "reader_pool"), ("search", "reader_pool"),
    ("graph_expand", "search"), ("graph_expand", "reader_pool"),
    ("graph_expand", "search_api"),
]):
    file = "graph_expand/mod.rs" if owner == "graph_expand" else f"{owner}.rs"
    case(f"direction-{owner}-{target}", f"forbidden dependency {owner} -> {target}",
         (file, f"#[allow(unused_imports)] use crate::{target} as recovery_direction_{index};"))
case("descendant", "forbidden dependency graph_expand::codec -> search_api",
     ("graph_expand/codec.rs", "#[allow(unused_imports)] use crate::search_api as recovery_descendant;"))
case("callable-reference", "forbidden dependency read -> reader_pool",
     ("reader_pool.rs", "pub(crate) fn recovery_callable() {}"),
     ("read.rs", "fn recovery_reference() { let _ = crate::reader_pool::recovery_callable; }"))
case("capitalized", "forbidden dependency search -> reader_pool",
     ("reader_pool.rs", "pub(crate) const RECOVERY_CONSTANT: usize = 1;"),
     ("search.rs", "fn recovery_capitalized() -> usize { crate::reader_pool::RECOVERY_CONSTANT }"))
case("type", "forbidden dependency read -> reader_pool",
     ("read.rs", "fn recovery_type(_: &crate::reader_pool::ReaderRequest) {}"))
case("macro-body", "forbidden dependency search -> reader_pool",
     ("reader_pool.rs", "pub(crate) fn recovery_macro_value() -> u8 { 0 }"),
     ("search.rs", 'fn recovery_macro() -> String { format!("{}", crate::reader_pool::recovery_macro_value()) }'))
case("engine-alias", "forbidden dependency graph_expand -> search_api",
     ("graph_expand/mod.rs", 'fn recovery_alias(engine: &crate::Engine) { let alias = engine; let _ = alias.search(""); }'))
case("engine-method-import-collision", "forbidden dependency graph_expand -> search_api",
     ("graph_expand/mod.rs", '#[allow(unused_imports)] use crate::fusion::fuse_rrf as search; fn recovery_engine_method(engine: &crate::Engine) { let _ = engine.search(""); }'))
case("engine-field-import-collision", "forbidden dependency graph_expand -> reader_pool",
     ("graph_expand/mod.rs", "#[allow(unused_imports)] use crate::fusion::fuse_rrf as reader_pool; fn recovery_engine_field(engine: &crate::Engine) { let _ = &engine.reader_pool; }"))
case("request-variant", "direct ReaderRequest variant construction outside reader_pool",
     ("read_api.rs", "fn recovery_variant() -> crate::reader_pool::ReaderRequest { crate::reader_pool::ReaderRequest::Shutdown }"))
case("hooks", "forbidden dependency read -> reader_pool",
     ("read.rs", '#[cfg(feature = "test-hooks")] #[allow(unused_imports)] use crate::reader_pool as recovery_hooks;'))
case("operator", "forbidden dependency read -> reader_pool",
     ("read.rs", '#[cfg(feature = "operator")] #[allow(unused_imports)] use crate::reader_pool as recovery_operator;'))
case("tc5", "forbidden dependency read -> reader_pool",
     ("read.rs", '#[cfg(feature = "tc5-benchmark")] #[allow(unused_imports)] use crate::reader_pool as recovery_tc5;'))
case("cfg-test", "forbidden dependency read -> reader_pool",
     ("read.rs", '#[cfg(test)] #[allow(unused_imports)] use crate::reader_pool as recovery_test;'))
case("cfg-linux", "forbidden dependency read -> reader_pool",
     ("read.rs", '#[cfg(target_os = "linux")] #[allow(unused_imports)] use crate::reader_pool as recovery_linux;'))
case("root-helper-chain", "forbidden cycle reader_pool <-> search graph=item",
     ("lib.rs", "fn recovery_root_one() { recovery_root_two(); } fn recovery_root_two() { crate::reader_pool::recovery_pool_bridge(); }"),
     ("search.rs", "pub(crate) fn recovery_root_out() { crate::recovery_root_one(); }"),
     ("reader_pool.rs", "pub(crate) fn recovery_pool_bridge() { crate::search::recovery_root_out(); }"))
case("reported-return", "forbidden cycle reader_pool <-> search graph=item",
     ("search.rs", "pub(crate) fn recovery_reported_out() { crate::fusion::recovery_reported_bridge(); }"),
     ("fusion.rs", "pub(crate) fn recovery_reported_bridge() { crate::reader_pool::recovery_reported_pool(); }"),
     ("reader_pool.rs", "pub(crate) fn recovery_reported_pool() { crate::search::recovery_reported_out(); }"))
case("governed-type-cycle", "unapproved governed cycle read <-> search graph=governed-module",
     ("read.rs", "pub(crate) struct RecoveryRead; fn recovery_to_search(_: &crate::search::RecoverySearch) {}"),
     ("search.rs", "pub(crate) struct RecoverySearch; fn recovery_to_read(_: &crate::read::RecoveryRead) {}"))
case("forbidden-submodule-cycle", "forbidden cycle graph_expand <-> search graph=item",
     ("graph_expand/codec.rs", "pub(crate) fn recovery_codec_out() { crate::search::recovery_codec_back(); }"),
     ("graph_expand/mod.rs", "pub(crate) use codec::recovery_codec_out;"),
     ("search.rs", "pub(crate) fn recovery_codec_back() { crate::graph_expand::recovery_codec_out(); }"))
case("lexical-use", "forbidden cycle reader_pool <-> search graph=item",
     ("lib.rs", "fn recovery_lexical_bridge() { crate::reader_pool::recovery_lexical_pool(); }"),
     ("search.rs", "pub(crate) fn recovery_lexical_out() { use crate::recovery_lexical_bridge as local; local(); }"),
     ("reader_pool.rs", "pub(crate) fn recovery_lexical_pool() { crate::search::recovery_lexical_out(); }"))
# Platform selection and deliberate unsupported forms are parser checks, not
# falsely claimed active host compilations.
case("cfg-nonlinux", "forbidden dependency read -> reader_pool configuration=default-nonlinux",
     ("read.rs", '#[cfg(not(target_os = "linux"))] use crate::reader_pool as recovery_nonlinux;'), kind="parser")
case("cfg-release", "forbidden dependency read -> reader_pool configuration=default-linux-release",
     ("read.rs", '#[cfg(not(debug_assertions))] use crate::reader_pool as recovery_release;'), kind="parser")
case("unknown-cfg", "unsupported cfg predicate",
     ("search.rs", '#[cfg(recovery_unknown)] fn recovery_unknown() {}'), kind="parser")
case("inactive-parent-cfg", "unsupported cfg predicate",
     ("search.rs", '#[cfg(any())] fn recovery_inactive() { #[cfg(recovery_unknown)] let _ = 1; }'), kind="parser")
case("zero-config", "edge has no evaluated configuration",
     ("search.rs", '#[cfg(any())] fn recovery_never() { crate::reader_pool::recovery_missing(); }'), kind="parser")
case("include", "unreviewed include",
     ("search.rs", 'include!("recovery.rs");'), kind="parser")
case("path", "unsupported #[path]",
     ("search.rs", '#[path = "elsewhere.rs"] mod recovery_path;'), kind="parser")
case("extern-crate", "unsupported extern crate",
     ("search.rs", "extern crate self as recovery_crate;"), kind="parser")
case("unparsed-macro", "unparsed macro body",
     ("search.rs", "fn recovery_opaque() { opaque!(=> crate::reader_pool::ReaderRequest); }"), kind="parser")
case("missing-owner", "owner assertion stale",
     ("@policy", "owner RecoveryMissing reader_pool"), kind="policy")
case("stale-admission", "stale admit-type",
     ("@policy", "admit-type errors EngineError::GraphExpansion graph_expand::types GraphExpansionErrorReasonV1"), kind="policy")
case("forbid-floor", "required forbid-cycle graph_expand search is missing",
     ("@delete-policy", "forbid-cycle search graph_expand"), kind="policy")
case("missing-classification", "module classification missing telemetry",
     ("@delete-policy", "governed telemetry"), kind="policy")
case("field-map", "Engine field map missing reader_pool",
     ("@delete-policy", "field reader_pool reader_pool"), kind="policy")


def run(command, cwd):
    completed = subprocess.run(command, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return completed.returncode, completed.stdout


def edit_fixture(fixture, edits):
    for file, body in edits:
        if file == "@delete-policy":
            path = fixture / policy
            path.write_text(path.read_text().replace(body + "\n", ""))
        else:
            path = fixture / (policy if file == "@policy" else source / file)
            with path.open("a") as stream:
                stream.write("\n" + body + "\n")


def restore(fixture):
    shutil.copytree(root / source, fixture / source, dirs_exist_ok=True)
    shutil.copy2(root / policy, fixture / policy)


started = time.monotonic()
with tempfile.TemporaryDirectory(prefix="fathomdb-boundary-qualification-") as temporary:
    fixture = Path(temporary)
    for file in ["Cargo.toml", "Cargo.lock"]:
        shutil.copy2(root / file, fixture / file)
    shutil.copytree(root / "src/rust", fixture / "src/rust")
    shutil.copytree(root / "tests/fixtures", fixture / "tests/fixtures")
    shutil.copytree(root / "dev/fixtures", fixture / "dev/fixtures")
    (fixture / policy.parent).mkdir(parents=True, exist_ok=True)
    restore(fixture)
    # All architectural bodies exist simultaneously in the compilation receipt.
    # Unique names avoid making their compiler diagnostics interfere.
    for _, _, edits, kind in cases:
        if kind == "compiled":
            edit_fixture(fixture, edits)
    command = ["cargo", "check", "--quiet", "--locked", "-p", "fathomdb-engine", "--tests", "--features", features]
    print(f"compile witnesses: --tests features={features}", flush=True)
    rc, output = run(command, fixture)
    if rc:
        sys.stderr.write(output)
        raise SystemExit(rc)
    print(f"compiled {sum(kind == 'compiled' for _, _, _, kind in cases)} active architectural witnesses", flush=True)
    for name, diagnostic, edits, kind in cases:
        restore(fixture)
        edit_fixture(fixture, edits)
        rc, output = run([str(gate), "--root", str(fixture)], root)
        if not rc or diagnostic not in output:
            sys.stderr.write(f"FAIL {name} ({kind}): expected {diagnostic}\n{output}\n")
            raise SystemExit(1)
        print(f"PASS {name} ({kind})", flush=True)
    restore(fixture)
    # A benign foreign receiver sharing names with engine methods must be legal.
    edit_fixture(fixture, [("search.rs", 'fn recovery_external(value: String) -> usize { value.as_str().len() }')])
    rc, output = run([str(gate), "--root", str(fixture)], root)
    if rc:
        sys.stderr.write(output)
        raise SystemExit(rc)
    print("PASS external-same-name control", flush=True)
    restore(fixture)
    for file in (root / source).rglob("*.rs"):
        if file.read_bytes() != (fixture / source / file.relative_to(root / source)).read_bytes():
            raise SystemExit(f"fixture restoration failed: {file}")
print(f"ok    module-boundary qualification: {len(cases)} cases; {time.monotonic() - started:.1f}s", flush=True)
