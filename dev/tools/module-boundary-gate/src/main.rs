use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fathomdb_module_boundary_gate::{
    analyze_source, analyze_source_in, Analysis, ConfigSet, ConfigSpace, EdgeKind, InherentMethod,
};

/// `(module, item, method, receiver expression)` of untyped dot calls.
type ReceiverKey = (String, String, String, String);

/// A reviewed receiver exception: the exact call count and the receiver's
/// type, outside the crate (`external-receiver`) or an in-crate type whose
/// method becomes a typed edge (`typed-receiver`).
#[derive(Debug)]
struct ReceiverEntry {
    count: usize,
    typed: bool,
    ty: String,
}

impl ReceiverEntry {
    fn directive(&self) -> &'static str {
        if self.typed {
            "typed-receiver"
        } else {
            "external-receiver"
        }
    }
}

#[derive(Debug, Default)]
struct Policy {
    classified: BTreeMap<String, String>,
    field_owners: BTreeMap<String, String>,
    owners: BTreeMap<String, String>,
    forbidden_dependencies: BTreeSet<(String, String)>,
    forbidden_cycles: BTreeSet<(String, String)>,
    allowed_cycles: BTreeSet<(String, String)>,
    reported_cycles: BTreeSet<(String, String)>,
    admissions: BTreeSet<(String, String, String, String)>,
    receivers: BTreeMap<ReceiverKey, ReceiverEntry>,
    allowed_local_macros: BTreeSet<(String, String, String)>,
    allowed_unparsed_macros: BTreeSet<(String, String, String, String)>,
    inherent_methods: BTreeSet<(String, String)>,
    expected_edges: BTreeSet<(String, String, String, String, String, String)>,
    module_cycles: BTreeSet<(String, String, String)>,
    module_sccs: BTreeSet<(String, String)>,
    configuration_features: Vec<(String, String)>,
    configuration_cross: Vec<String>,
    configuration_profiles: Vec<(String, Vec<String>)>,
}

struct ModuleInfo {
    file: PathBuf,
    analysis: Analysis,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(errors) => {
            for error in errors {
                eprintln!("FAIL module-boundary: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Vec<String>> {
    let mut arguments = env::args().skip(1);
    let mut root = PathBuf::from(".");
    let mut policy_path = PathBuf::from("dev/tools/module-boundary-policy.txt");
    let mut report_only = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--root" => {
                root = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| vec!["--root requires a repository path".to_string()])?,
                );
            }
            "--policy" => {
                policy_path = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| vec!["--policy requires a file path".to_string()])?,
                );
            }
            "--report" => report_only = true,
            other => return Err(vec![format!("unknown argument {other:?}")]),
        }
    }
    if policy_path.is_relative() {
        policy_path = root.join(policy_path);
    }
    let source_root = root.join("src/rust/crates/fathomdb-engine/src");
    let manifest_path = root.join("src/rust/crates/fathomdb-engine/Cargo.toml");
    let mut policy = parse_policy(&policy_path)?;
    let manifest = fs::read_to_string(&manifest_path).map_err(|error| {
        vec![format!("cannot read engine manifest {}: {error}", manifest_path.display())]
    })?;
    let space = ConfigSpace::from_manifest_with(
        &manifest,
        &policy.configuration_features,
        &policy.configuration_cross,
        &policy.configuration_profiles,
    )?;
    let policy_errors = normalize_policy_configurations(&mut policy, &space);
    let modules = discover_modules(&source_root, &space)?;
    let mut result = evaluate(&source_root, &modules, &policy, &space, report_only);
    if !policy_errors.is_empty() {
        let mut errors = policy_errors;
        errors.extend(result.err().unwrap_or_default());
        result = Err(errors);
    }
    if report_only {
        return result;
    }
    result?;
    println!(
        "ok    module-boundary: {} modules, {} governed, configurations={} (test x linux x \
         debug_assertions x {{{} combinations of {}; {} single-feature closures and {} consumer \
         profiles ({}), each x {} combinations of {}; all-features}})",
        module_inventory(&modules).len(),
        policy.classified.values().filter(|kind| kind.as_str() == "governed").count(),
        space.len(),
        1usize << space.axes.len(),
        space.axes.iter().map(|(feature, _)| feature.as_str()).collect::<Vec<_>>().join(", "),
        space.profiles.len().saturating_sub(2 + space.consumer_profiles.len()),
        space.consumer_profiles.len(),
        space
            .consumer_profiles
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        1usize << space.cross.len(),
        if space.cross.is_empty() { "none".to_string() } else { space.cross.join(", ") }
    );
    Ok(())
}

fn discover_modules(
    source_root: &Path,
    space: &ConfigSpace,
) -> Result<BTreeMap<String, ModuleInfo>, Vec<String>> {
    let mut files = Vec::new();
    collect_rust_files(source_root, &mut files)
        .map_err(|error| vec![format!("cannot enumerate {}: {error}", source_root.display())])?;
    files.sort();
    let mut modules = BTreeMap::new();
    let mut errors = Vec::new();
    for file in files {
        let relative = file.strip_prefix(source_root).expect("file lies below source root");
        let module = module_name(relative);
        let source = match fs::read_to_string(&file) {
            Ok(source) => source,
            Err(error) => {
                errors.push(format!("cannot read {}: {error}", file.display()));
                continue;
            }
        };
        match analyze_source(&source, space) {
            Ok(analysis) => {
                if modules.insert(module.clone(), ModuleInfo { file, analysis }).is_some() {
                    errors.push(format!("duplicate module path {module}"));
                }
            }
            Err(error) => errors.push(format!("cannot parse {module}: {error}")),
        }
    }
    // A file's module exists only where its declaring `mod` item does:
    // re-analyse, parents first, every file whose declaration is cfg-gated.
    let mut order = modules.keys().cloned().collect::<Vec<_>>();
    order.sort_by_key(|module| (module != "root", module.matches("::").count()));
    for module in order {
        if module == "root" {
            continue;
        }
        let (parent, name) = module.rsplit_once("::").unwrap_or(("root", module.as_str()));
        let Some(declared) = modules.get(parent).and_then(|info| {
            info.analysis
                .modules
                .iter()
                .find(|item| item.depth == 0 && !item.inline && item.name == name)
                .map(|item| item.configurations)
        }) else {
            continue;
        };
        if declared == space.all() {
            continue;
        }
        let info = modules.get_mut(&module).expect("module is discovered");
        match fs::read_to_string(&info.file)
            .map_err(|error| format!("cannot read {}: {error}", info.file.display()))
            .and_then(|source| {
                analyze_source_in(&source, space, declared)
                    .map_err(|error| format!("cannot parse {module}: {error}"))
            }) {
            Ok(analysis) => info.analysis = analysis,
            Err(error) => errors.push(error),
        }
    }
    let mut declared_files = BTreeSet::from(["root".to_string()]);
    for (parent, info) in &modules {
        for declaration in
            info.analysis.modules.iter().filter(|item| item.depth == 0 && !item.inline)
        {
            let child = if parent == "root" {
                declaration.name.clone()
            } else {
                format!("{parent}::{}", declaration.name)
            };
            declared_files.insert(child);
        }
    }
    let physical_files = modules.keys().cloned().collect::<BTreeSet<_>>();
    for missing in declared_files.difference(&physical_files) {
        errors.push(format!("declared out-of-line module has no source file {missing}"));
    }
    for undeclared in physical_files.difference(&declared_files) {
        errors.push(format!("Rust source file is not declared from lib.rs {undeclared}"));
    }
    if errors.is_empty() {
        Ok(modules)
    } else {
        Err(errors)
    }
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn module_name(relative: &Path) -> String {
    let mut components = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let file = components.pop().expect("Rust source has a file name");
    let stem = file.strip_suffix(".rs").expect("Rust source suffix");
    if stem != "lib" && stem != "mod" {
        components.push(stem.to_string());
    }
    if components.is_empty() {
        "root".to_string()
    } else {
        components.join("::")
    }
}

fn scoped_module(module: &str, scope: &str) -> String {
    if scope.is_empty() {
        module.to_string()
    } else {
        format!("{module}::{scope}")
    }
}

fn module_inventory(modules: &BTreeMap<String, ModuleInfo>) -> BTreeSet<String> {
    let mut inventory = modules.keys().cloned().collect::<BTreeSet<_>>();
    for (module, info) in modules {
        for declaration in info.analysis.modules.iter().filter(|item| item.inline) {
            let scope = if declaration.scope.is_empty() {
                declaration.name.clone()
            } else {
                format!("{}::{}", declaration.scope, declaration.name)
            };
            inventory.insert(scoped_module(module, &scope));
        }
    }
    inventory
}

fn parse_policy(path: &Path) -> Result<Policy, Vec<String>> {
    let source = fs::read_to_string(path)
        .map_err(|error| vec![format!("cannot read policy {}: {error}", path.display())])?;
    let mut policy = Policy::default();
    let mut errors = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let fields = line.split_whitespace().collect::<Vec<_>>();
        match fields.as_slice() {
            ["version", "1"] => {}
            ["configuration-feature", feature, label] => {
                policy.configuration_features.push(((*feature).to_string(), (*label).to_string()));
            }
            ["configuration-cross", feature] => {
                policy.configuration_cross.push((*feature).to_string());
            }
            ["configuration-profile", name, features] => {
                policy.configuration_profiles.push((
                    (*name).to_string(),
                    features
                        .split(',')
                        .filter(|feature| !feature.is_empty())
                        .map(str::to_string)
                        .collect(),
                ));
            }
            [kind @ ("governed" | "admitted" | "reported"), module] => {
                if policy.classified.insert((*module).to_string(), (*kind).to_string()).is_some() {
                    errors.push(format!(
                        "policy:{} duplicates classification for {module}",
                        index + 1
                    ));
                }
            }
            ["field", field, owner] => {
                if policy.field_owners.insert((*field).to_string(), (*owner).to_string()).is_some()
                {
                    errors.push(format!("policy:{} duplicates Engine field {field}", index + 1));
                }
            }
            ["owner", item, module] => {
                if policy.owners.insert((*item).to_string(), (*module).to_string()).is_some() {
                    errors.push(format!("policy:{} duplicates owner assertion {item}", index + 1));
                }
            }
            ["forbid-dependency", source, target] => {
                policy
                    .forbidden_dependencies
                    .insert(((*source).to_string(), (*target).to_string()));
            }
            ["forbid-cycle", left, right] => {
                policy.forbidden_cycles.insert(sorted_pair(left, right));
            }
            ["allow-cycle", left, right] => {
                policy.allowed_cycles.insert(sorted_pair(left, right));
            }
            ["report-cycle", left, right] => {
                policy.reported_cycles.insert(sorted_pair(left, right));
            }
            [kind @ ("external-receiver" | "typed-receiver"), module, item, method, count, receiver, ty] => {
                match count.parse::<usize>() {
                    Ok(count) if count > 0 => {
                        let key = (
                            (*module).to_string(),
                            (*item).to_string(),
                            (*method).to_string(),
                            (*receiver).to_string(),
                        );
                        let entry = ReceiverEntry {
                            count,
                            typed: *kind == "typed-receiver",
                            ty: (*ty).to_string(),
                        };
                        if policy.receivers.insert(key, entry).is_some() {
                            errors.push(format!(
                                "policy:{} duplicates receiver entry {module} {item} {method} {receiver}",
                                index + 1
                            ));
                        }
                    }
                    _ => errors.push(format!(
                        "policy:{} {kind} count must be a positive integer",
                        index + 1
                    )),
                }
            }
            ["admit-type", source, source_item, target, target_item] => {
                policy.admissions.insert((
                    (*source).to_string(),
                    (*source_item).to_string(),
                    (*target).to_string(),
                    (*target_item).to_string(),
                ));
            }
            ["local-macro", module, name, fingerprint] => {
                policy.allowed_local_macros.insert((
                    (*module).to_string(),
                    (*name).to_string(),
                    (*fingerprint).to_string(),
                ));
            }
            ["unparsed-macro", module, item, name, fingerprint] => {
                policy.allowed_unparsed_macros.insert((
                    (*module).to_string(),
                    (*item).to_string(),
                    (*name).to_string(),
                    (*fingerprint).to_string(),
                ));
            }
            ["inherent", module, method] => {
                if !policy.inherent_methods.insert(((*module).to_string(), (*method).to_string())) {
                    errors.push(format!(
                        "policy:{} duplicates inherent method {module} {method}",
                        index + 1
                    ));
                }
            }
            ["edge", source, source_item, target, target_item, kind, configurations] => {
                if !policy.expected_edges.insert((
                    (*source).to_string(),
                    (*source_item).to_string(),
                    (*target).to_string(),
                    (*target_item).to_string(),
                    (*kind).to_string(),
                    (*configurations).to_string(),
                )) {
                    errors.push(format!(
                        "policy:{} duplicates edge {source} {source_item} {target} {target_item} {kind} {configurations}",
                        index + 1
                    ));
                }
            }
            ["module-cycle", governed, other, configurations] => {
                policy.module_cycles.insert((
                    (*governed).to_string(),
                    (*other).to_string(),
                    (*configurations).to_string(),
                ));
            }
            ["module-scc", module, configurations] => {
                policy.module_sccs.insert(((*module).to_string(), (*configurations).to_string()));
            }
            _ => errors.push(format!("policy:{} has invalid directive {line:?}", index + 1)),
        }
    }
    if policy.classified.is_empty() {
        errors.push("policy has an empty module classification".to_string());
    }
    if policy.field_owners.is_empty() {
        errors.push("policy has an empty Engine field map".to_string());
    }
    if errors.is_empty() {
        Ok(policy)
    } else {
        Err(errors)
    }
}

/// Rewrites every expected edge's configuration expression to its canonical
/// form so that equivalent spellings compare equal.
fn normalize_policy_configurations(policy: &mut Policy, space: &ConfigSpace) -> Vec<String> {
    let mut errors = Vec::new();
    let mut normalized = BTreeSet::new();
    for (source, source_item, target, target_item, kind, configurations) in &policy.expected_edges {
        match space.parse_expression(configurations) {
            Ok(set) => {
                normalized.insert((
                    source.clone(),
                    source_item.clone(),
                    target.clone(),
                    target_item.clone(),
                    kind.clone(),
                    space.expression(set),
                ));
            }
            Err(error) => errors.push(format!(
                "policy edge {source} {source_item} {target} {target_item} {kind} has an invalid \
                 configuration expression {configurations:?}: {error}"
            )),
        }
    }
    policy.expected_edges = normalized;
    let mut cycles = BTreeSet::new();
    for (governed, other, configurations) in &policy.module_cycles {
        match space.parse_expression(configurations) {
            Ok(set) => {
                cycles.insert((governed.clone(), other.clone(), space.expression(set)));
            }
            Err(error) => errors.push(format!(
                "policy module-cycle {governed} {other} has an invalid configuration expression \
                 {configurations:?}: {error}"
            )),
        }
    }
    policy.module_cycles = cycles;
    let mut sccs = BTreeSet::new();
    for (module, configurations) in &policy.module_sccs {
        match space.parse_expression(configurations) {
            Ok(set) => {
                sccs.insert((module.clone(), space.expression(set)));
            }
            Err(error) => errors.push(format!(
                "policy module-scc {module} has an invalid configuration expression \
                 {configurations:?}: {error}"
            )),
        }
    }
    policy.module_sccs = sccs;
    errors
}

fn sorted_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn evaluate(
    source_root: &Path,
    modules: &BTreeMap<String, ModuleInfo>,
    policy: &Policy,
    space: &ConfigSpace,
    report_only: bool,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let discovered = module_inventory(modules);
    let classified = policy.classified.keys().cloned().collect::<BTreeSet<_>>();
    for missing in discovered.difference(&classified) {
        errors.push(format!("module classification missing {missing}"));
    }
    for stale in classified.difference(&discovered) {
        errors.push(format!("module classification stale {stale}"));
    }

    let root = modules.get("root");
    let engine_fields =
        root.map_or_else(BTreeSet::new, |module| module.analysis.engine_fields.clone());
    let configured_fields = policy.field_owners.keys().cloned().collect::<BTreeSet<_>>();
    for missing in engine_fields.difference(&configured_fields) {
        errors.push(format!("Engine field map missing {missing}"));
    }
    for stale in configured_fields.difference(&engine_fields) {
        errors.push(format!("Engine field map stale {stale}"));
    }

    let mut method_owners: MethodOwners = BTreeMap::new();
    for (module, info) in modules {
        for (method, configurations) in &info.analysis.engine_methods {
            let entry =
                method_owners.entry(method.clone()).or_default().entry(module.clone()).or_default();
            *entry = entry.union(*configurations);
        }
    }
    if method_owners.is_empty() {
        errors.push("source-derived Engine method map is empty".to_string());
    }
    for (method, owners) in &method_owners {
        let owners = owners.iter().collect::<Vec<_>>();
        for (left_index, (left, left_configurations)) in owners.iter().enumerate() {
            for (right, right_configurations) in owners.iter().skip(left_index + 1) {
                let overlap = left_configurations.intersect(**right_configurations);
                if let Some(first) = overlap.indices().next() {
                    errors.push(format!(
                        "Engine method {method} has multiple defining modules in configuration \
                         {}: {left}, {right} (overlap {})",
                        space.configurations[first].label,
                        space.expression(overlap)
                    ));
                }
            }
        }
    }

    let mut declared_owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (module, info) in modules {
        for item in &info.analysis.declared_items {
            declared_owners.entry(item.clone()).or_default().insert(module.clone());
        }
    }
    let governed_callable_names = declared_owners
        .iter()
        .filter(|(_, owners)| {
            owners
                .iter()
                .any(|owner| policy.classified.get(owner).map(String::as_str) == Some("governed"))
        })
        .map(|(item, _)| item.clone())
        .chain(method_owners.keys().cloned())
        .collect::<BTreeSet<_>>();

    for (item, owner) in &policy.owners {
        match modules.get(owner) {
            Some(module) if module.analysis.declared_items.contains(item) => {}
            Some(_) => {
                errors.push(format!("owner assertion stale: {owner} does not declare {item}"))
            }
            None => errors.push(format!("owner assertion names missing module {owner} for {item}")),
        }
    }

    let actual_inherent = modules
        .iter()
        .filter(|(module, _)| {
            policy.classified.get(*module).map(String::as_str) == Some("governed")
        })
        .flat_map(|(module, info)| {
            info.analysis.inherent_methods.iter().map(move |method: &InherentMethod| {
                (module.clone(), format!("{}::{}", method.owner, method.method))
            })
        })
        .collect::<BTreeSet<_>>();
    if !report_only {
        for missing in actual_inherent.difference(&policy.inherent_methods) {
            errors.push(format!("unlisted governed inherent method {} {}", missing.0, missing.1));
        }
        for stale in policy.inherent_methods.difference(&actual_inherent) {
            errors.push(format!("stale governed inherent method {} {}", stale.0, stale.1));
        }
    }

    let root_aliases = root.map(|module| &module.analysis.import_aliases);
    let mut merged_edges: BTreeMap<EdgeKey, (ConfigSet, (String, usize, usize, String))> =
        BTreeMap::new();
    let mut actual_local_macros = BTreeSet::new();
    let mut actual_unparsed_macros = BTreeSet::new();
    // Unresolved dot calls whose name is only a governed inherent method of
    // another module, counted per source item for reviewed exceptions.
    let mut unresolved_receivers = BTreeMap::<ReceiverKey, usize>::new();
    let mut unresolved_origins = BTreeMap::<ReceiverKey, Vec<String>>::new();
    // Every graph is kept as `(from, to) -> configurations`.
    let mut item_graph = MaskedGraph::new();
    let mut module_dependencies = MaskedGraph::new();
    // Three graphs per configuration. `item`: the whole-crate item graph of
    // executable (callable, field, Engine-method, typed-receiver) and type
    // references. `governed-module`: module-level, induced on the governed
    // set, over every non-composition kind. `module`: the whole-crate
    // module-level graph (root split into items), reported only. Re-exports,
    // the Engine storage layout and admitted type-only edges are composition
    // metadata and join none of them.
    let mut governed_graph = MaskedGraph::new();
    let mut module_graph = MaskedGraph::new();
    let impl_methods = modules
        .values()
        .flat_map(|info| info.analysis.impl_methods.iter().cloned())
        .collect::<BTreeSet<_>>();
    let constructors = modules
        .values()
        .flat_map(|info| info.analysis.constructors.iter().cloned())
        .collect::<BTreeSet<_>>();
    // Every externally visible inherent method crate-wide, by name: the
    // item-graph over-approximation of an untyped governed dot call.
    let mut crate_inherent = BTreeMap::<String, BTreeSet<String>>::new();
    for (module, info) in modules {
        for method in &info.analysis.inherent_methods {
            crate_inherent.entry(method.method.clone()).or_default().insert(graph_node(
                &scoped_module(module, &method.scope),
                &format!("{}::{}", method.owner, method.method),
            ));
        }
    }
    let governed_engine_methods = method_owners
        .iter()
        .filter(|(_, owners)| {
            owners.keys().any(|owner| classification(policy, owner) == Some("governed"))
        })
        .map(|(method, _)| method.clone())
        .collect::<BTreeSet<_>>();
    let inherent_owners = actual_inherent.iter().fold(
        BTreeMap::<String, BTreeSet<String>>::new(),
        |mut owners, (module, method)| {
            let name = method.rsplit_once("::").map_or(method.as_str(), |(_, name)| name);
            owners.entry(name.to_string()).or_default().insert(module.clone());
            owners
        },
    );
    for (module, info) in modules {
        for predicate in &info.analysis.unsupported_cfg {
            errors.push(format!(
                "unsupported cfg predicate in {}: {predicate}",
                relative(source_root, &info.file)
            ));
        }
        for edge in &info.analysis.edges {
            let source_module = scoped_module(module, &edge.source_scope);
            if edge.configurations.is_empty() {
                let targets = edge_targets(
                    &source_module,
                    edge,
                    modules,
                    root_aliases,
                    &engine_fields,
                    &policy.field_owners,
                    &method_owners,
                );
                if frozen_scope(policy, &source_module)
                    || targets.iter().any(|target| frozen_scope(policy, &target.module))
                {
                    errors.push(format!(
                        "edge has no evaluated configuration source={source_module} source_item={} \
                         syntax={} kind={} at {}:{}:{}; every edge in the frozen scope must be \
                         active in at least one evaluated configuration",
                        edge.source_item,
                        edge.target,
                        edge.kind.as_str(),
                        relative(source_root, &info.file),
                        edge.location.line,
                        edge.location.column
                    ));
                }
                continue;
            }
            let governed =
                policy.classified.get(&source_module).map(String::as_str) == Some("governed");
            if governed {
                if edge.kind == EdgeKind::Import
                    && ["crate::*", "super::*", "self::*"].contains(&edge.target.as_str())
                {
                    errors.push(format!(
                        "governed internal glob import {}:{}:{}",
                        relative(source_root, &info.file),
                        edge.location.line,
                        edge.location.column
                    ));
                }
            }
            if governed
                && matches!(edge.kind, EdgeKind::Import | EdgeKind::Reexport | EdgeKind::Type)
                && root_reexport_indirection(&source_module, &edge.target, root_aliases)
            {
                errors.push(format!(
                    "governed root re-export indirection at {}:{}:{} item={}; import the semantic owner",
                    relative(source_root, &info.file),
                    edge.location.line,
                    edge.location.column,
                    edge.target
                ));
            }
            if governed
                && module != "reader_pool"
                && edge.kind == EdgeKind::Callable
                && direct_reader_request_variant(&edge.target)
            {
                errors.push(format!(
                    "direct ReaderRequest variant construction outside reader_pool at {}:{}:{} item={}",
                    relative(source_root, &info.file),
                    edge.location.line,
                    edge.location.column,
                    edge.target
                ));
            }
            let mut targets = edge_targets(
                &source_module,
                edge,
                modules,
                root_aliases,
                &engine_fields,
                &policy.field_owners,
                &method_owners,
            );
            if edge.kind == EdgeKind::Callable {
                if let Some(rest) = edge.target.strip_prefix(".<") {
                    let (method, receiver_type) = rest
                        .split_once(">:")
                        .map_or((rest.trim_end_matches('>'), None), |(method, ty)| {
                            (method, Some(ty))
                        });
                    // A syntactic receiver type holds only when its
                    // constructor returns it and, for an in-crate type, the
                    // type has the method; otherwise the call is untyped. A
                    // type outside the crate keeps its (never governed)
                    // methods.
                    let typed = receiver_type.is_some_and(|ty| {
                        let owner = ty.rsplit("::").next().unwrap_or(ty);
                        if targets.is_empty() {
                            // Outside the crate; a trait's associated
                            // function names no concrete type.
                            return edge.constructor.is_none()
                                || !CONSTRUCTOR_TRAITS.contains(&owner);
                        }
                        let constructed = edge.constructor.as_ref().is_none_or(|constructor| {
                            DERIVED_CONSTRUCTORS.contains(&constructor.as_str())
                                || constructors.contains(&(owner.to_string(), constructor.clone()))
                        });
                        constructed
                            && impl_methods.contains(&(owner.to_string(), method.to_string()))
                    });
                    if !typed {
                        targets.clear();
                        let cross_boundary_inherent =
                            inherent_owners.get(method).is_some_and(|owners| {
                                owners.iter().any(|owner| owner != &source_module)
                            });
                        let governed_name = if governed {
                            governed_callable_names.contains(method)
                        } else {
                            governed_engine_methods.contains(method)
                        };
                        let mut reviewed_type = None;
                        if !governed_name && cross_boundary_inherent {
                            let key = (
                                source_module.clone(),
                                edge.source_item.clone(),
                                method.to_string(),
                                edge.receiver.clone().unwrap_or_else(|| "_".to_string()),
                            );
                            *unresolved_receivers.entry(key.clone()).or_default() += 1;
                            unresolved_origins.entry(key.clone()).or_default().push(format!(
                                "{}:{}:{}",
                                relative(source_root, &info.file),
                                edge.location.line,
                                edge.location.column
                            ));
                            // A reviewed in-crate receiver type is a typed
                            // call like any other.
                            reviewed_type = policy
                                .receivers
                                .get(&key)
                                .filter(|entry| entry.typed)
                                .and_then(|entry| resolve_type(&entry.ty, modules))
                                .map(|target| {
                                    chase_reexports(
                                        ResolvedTarget {
                                            module: target.module,
                                            item: format!("{}::{method}", target.item),
                                        },
                                        modules,
                                    )
                                });
                        } else if governed_name {
                            errors.push(format!(
                                "unresolved governed method receiver at {}:{}:{} source={} source_item={} method={method} receiver_type={}; use an owner-qualified call or a syntactically typed receiver",
                                relative(source_root, &info.file),
                                edge.location.line,
                                edge.location.column,
                                source_module,
                                edge.source_item,
                                receiver_type.unwrap_or("<unknown>")
                            ));
                        }
                        if let Some(reviewed) = reviewed_type {
                            targets = reviewed;
                        } else if governed {
                            for node in crate_inherent.get(method).into_iter().flatten() {
                                item_graph.add(
                                    &graph_node(&source_module, &edge.source_item),
                                    node,
                                    edge.configurations,
                                );
                            }
                        }
                    }
                }
            }
            if governed
                && edge.kind == EdgeKind::Callable
                && !edge.target.starts_with(".<")
                && !edge.target.contains("::")
                && info.analysis.local_functions.contains_key(&edge.target)
                && declared_owners.get(&edge.target).is_some_and(|owners| {
                    owners.iter().any(|owner| {
                        owner != module
                            && policy.classified.get(owner).map(String::as_str) == Some("governed")
                    })
                })
            {
                errors.push(format!(
                    "local function shadows governed callable at {}:{}:{} source={} source_item={} function={}",
                    relative(source_root, &info.file),
                    edge.location.line,
                    edge.location.column,
                    source_module,
                    edge.source_item,
                    edge.target
                ));
            }
            for target in targets {
                let nonroot_local = target.module == source_module && source_module != "root";
                let configurations = if edge.kind == EdgeKind::EngineMethod {
                    method_owners
                        .get(&edge.target)
                        .and_then(|owners| owners.get(&target.module))
                        .map_or(edge.configurations, |owner| edge.configurations.intersect(*owner))
                } else {
                    edge.configurations
                };
                let admitted = edge.kind == EdgeKind::Type
                    && policy.admissions.contains(&(
                        source_module.clone(),
                        edge.source_item.clone(),
                        target.module.clone(),
                        target.item.clone(),
                    ));
                let engine_layout = source_module == "root" && edge.source_item == "Engine";
                let dependency_source = dependency_node(&source_module, &edge.source_item);
                let dependency_target = dependency_node(&target.module, &target.item);
                let composition = edge.kind == EdgeKind::Reexport || admitted || engine_layout;
                let item_edge = !composition && edge.kind != EdgeKind::Import;
                let governed_edge = !composition
                    && governed
                    && target.module != source_module
                    && classification(policy, &target.module) == Some("governed");
                let module_edge = !composition && dependency_source != dependency_target;
                module_dependencies.add(&source_module, &target.module, configurations);
                if governed_edge {
                    governed_graph.add(&source_module, &target.module, configurations);
                }
                if module_edge {
                    module_graph.add(&dependency_source, &dependency_target, configurations);
                }
                if item_edge {
                    item_graph.add(
                        &graph_node(&source_module, &edge.source_item),
                        &graph_node(&target.module, &target.item),
                        configurations,
                    );
                }
                if nonroot_local {
                    continue;
                }
                if !configurations.is_empty()
                    && (frozen_scope(policy, &source_module)
                        || frozen_scope(policy, &target.module))
                {
                    let key = (
                        source_module.clone(),
                        edge.source_item.clone(),
                        target.module,
                        target.item,
                        edge.kind.as_str().to_string(),
                    );
                    let merged = merged_edges.entry(key).or_insert_with(|| {
                        (
                            ConfigSet::default(),
                            (
                                relative(source_root, &info.file),
                                edge.location.line,
                                edge.location.column,
                                edge.target.clone(),
                            ),
                        )
                    });
                    merged.0 = merged.0.union(configurations);
                }
            }
        }

        reject_relevant_macros(
            module,
            info,
            policy,
            source_root,
            &mut actual_local_macros,
            &mut errors,
        );
        reject_unparsed_macros(
            module,
            info,
            policy,
            source_root,
            &mut actual_unparsed_macros,
            &mut errors,
        );
    }

    for (key, count) in &unresolved_receivers {
        if policy.receivers.get(key).map(|entry| entry.count) != Some(*count) {
            errors.push(format!(
                "unresolved governed method receiver at {} source={} source_item={} method={} \
                 calls={count} receiver={}; the name is a governed inherent method of another \
                 module, so use an owner-qualified call, a syntactically typed receiver, or a \
                 reviewed external-receiver/typed-receiver entry naming this receiver, its type \
                 and this exact call count",
                unresolved_origins[key].join(","),
                key.0,
                key.1,
                key.2,
                key.3
            ));
        }
    }
    for (key, entry) in &policy.receivers {
        let (module, item, method, receiver) = key;
        if !unresolved_receivers.contains_key(key) {
            errors.push(format!(
                "stale {} {module} {item} {method} {} {receiver} {}: no such unresolved call",
                entry.directive(),
                entry.count,
                entry.ty
            ));
        }
        let owner = entry.ty.rsplit("::").next().unwrap_or(&entry.ty);
        let resolved = resolve_type(&entry.ty, modules);
        if entry.typed {
            if resolved.is_none() || !impl_methods.contains(&(owner.to_string(), method.clone())) {
                errors.push(format!(
                    "typed-receiver {module} {item} {method} names {}, which has no method \
                     {method} (or is not an in-crate type)",
                    entry.ty
                ));
            }
        } else if resolved.is_some()
            || (!entry.ty.contains("::") && declared_owners.contains_key(&entry.ty))
        {
            errors.push(format!(
                "external-receiver {module} {item} {method} names the in-crate type {}; record it \
                 as a typed-receiver",
                entry.ty
            ));
        }
    }

    let merged_edge_kinds = merged_edges.keys().cloned().collect::<BTreeSet<_>>();
    let mut actual_edges = BTreeSet::new();
    let mut edge_origins = BTreeMap::new();
    for ((source, source_item, target, target_item, kind), (configurations, origin)) in merged_edges
    {
        let record =
            (source, source_item, target, target_item, kind, space.expression(configurations));
        actual_edges.insert(record.clone());
        edge_origins.insert(record, origin);
    }

    for (module, name, _) in policy.allowed_local_macros.difference(&actual_local_macros) {
        errors.push(format!("stale local macro policy source={module} macro={name}"));
    }
    for (module, item, name, fingerprint) in
        policy.allowed_unparsed_macros.difference(&actual_unparsed_macros)
    {
        errors.push(format!(
            "stale unparsed macro policy source={module} item={item} macro={name} fingerprint={fingerprint}"
        ));
    }

    validate_cycle_policy(policy, &merged_edge_kinds, &mut errors);

    // A forbidden dependency covers descendant modules on both sides.
    for ((source, target), configurations) in &module_dependencies.edges {
        if source == target
            || !policy.forbidden_dependencies.iter().any(|(forbidden_source, forbidden_target)| {
                covers(forbidden_source, source) && covers(forbidden_target, target)
            })
        {
            continue;
        }
        for index in configurations.indices() {
            errors.push(format!(
                "forbidden dependency {source} -> {target} configuration={}",
                space.configurations[index].label
            ));
        }
    }
    let mut cycle_findings = CycleFindings::default();
    for (graph, masked) in [("item", &item_graph), ("governed-module", &governed_graph)] {
        for (index, component) in masked.components_by_configuration() {
            cycle_findings.record(policy, graph, index, component);
        }
    }
    cycle_findings.report_errors(policy, space, &mut errors);

    let (module_cycles, module_sccs) = module_cycle_inventory(policy, &module_graph, space);

    if report_only {
        for module in &discovered {
            println!("module\t{module}");
        }
        for field in &engine_fields {
            println!("field\t{field}");
        }
        for (module, method) in &actual_inherent {
            println!("inherent\t{module}\t{method}");
        }
        for (source, source_item, target, target_item, kind, configurations) in &actual_edges {
            println!(
                "edge\t{source}\t{source_item}\t{target}\t{target_item}\t{kind}\t{configurations}"
            );
        }
        for (module, name, fingerprint) in &actual_local_macros {
            println!("local-macro\t{module}\t{name}\t{fingerprint}");
        }
        for (key, count) in &unresolved_receivers {
            let (module, item, method, receiver) = key;
            match policy.receivers.get(key) {
                Some(entry) => println!(
                    "{}\t{module}\t{item}\t{method}\t{count}\t{receiver}\t{}",
                    entry.directive(),
                    entry.ty
                ),
                None => println!(
                    "unreviewed-receiver\t{module}\t{item}\t{method}\t{count}\t{receiver}\t{}",
                    unresolved_origins[key].join(",")
                ),
            }
        }
        for (module, item, name, fingerprint) in &actual_unparsed_macros {
            println!("unparsed-macro\t{module}\t{item}\t{name}\t{fingerprint}");
        }
        for ((source, target), configurations) in &module_graph.edges {
            println!(
                "module-dependency\t{source}\t{target}\t{}",
                space.expression(*configurations)
            );
        }
        // Whole-crate module-level SCCs are reported, not enforced: module
        // granularity merges unrelated items (for example every payload of
        // the central error enum), so these are inventory for item-level
        // review, not boundary verdicts.
        for (governed, other, configurations) in &module_cycles {
            println!("module-cycle\t{governed}\t{other}\t{configurations}");
        }
        for (module, configurations) in &module_sccs {
            println!("module-scc\t{module}\t{configurations}");
        }
        let mut module_components = BTreeMap::<BTreeSet<String>, ConfigSet>::new();
        for (index, component) in module_graph.components_by_configuration() {
            module_components.entry(component).or_default().insert(index);
        }
        for (component, configurations) in &module_components {
            println!(
                "module-scc-members\t{}\t{}",
                space.expression(*configurations),
                component.iter().cloned().collect::<Vec<_>>().join(",")
            );
        }
        for ((graph, component), configurations) in &cycle_findings.components {
            let modules = component.iter().map(|node| node_module(node)).collect::<BTreeSet<_>>();
            println!(
                "scc\t{graph}\t{}\t{}\t{}",
                space.expression(*configurations),
                modules.into_iter().collect::<Vec<_>>().join(","),
                component.iter().cloned().collect::<Vec<_>>().join(",")
            );
        }
        for (index, configuration) in space.configurations.iter().enumerate() {
            let edge_count = item_graph
                .edges
                .values()
                .filter(|configurations| configurations.contains(index))
                .count();
            println!("configuration\t{}\titem_edges={edge_count}", configuration.label);
        }
    } else {
        if policy.expected_edges.is_empty() {
            errors.push("policy expected-edge set is empty".to_string());
        }
        for (source, source_item, target, target_item, kind, configurations) in
            &policy.expected_edges
        {
            if !frozen_scope(policy, source) && !frozen_scope(policy, target) {
                errors.push(format!(
                    "policy edge outside the frozen scope source={source} source_item={source_item} \
                     destination={target} target_item={target_item} kind={kind} \
                     configurations={configurations}; only edges with a governed or root \
                     endpoint are frozen"
                ));
            }
        }
        for missing in actual_edges.difference(&policy.expected_edges) {
            let origin = edge_origins.get(missing).expect("actual edge has an origin");
            errors.push(format!(
                "unexpected boundary edge source={} source_item={} destination={} target_item={} syntax={} kind={} configurations={} at {}:{}:{}",
                missing.0,
                missing.1,
                missing.2,
                missing.3,
                origin.3,
                missing.4,
                missing.5,
                origin.0,
                origin.1,
                origin.2
            ));
        }
        for stale in policy.expected_edges.difference(&actual_edges) {
            if !frozen_scope(policy, &stale.0) && !frozen_scope(policy, &stale.2) {
                continue;
            }
            errors.push(format!(
                "stale boundary edge source={} source_item={} destination={} target_item={} kind={} configurations={}",
                stale.0, stale.1, stale.2, stale.3, stale.4, stale.5
            ));
        }
        for (governed, other, configurations) in module_cycles.difference(&policy.module_cycles) {
            errors.push(format!(
                "unreviewed module-level cycle {governed} <-> {other} configurations={configurations}; \
                 each depends on the other at module level, so review the pair and record a \
                 module-cycle line"
            ));
        }
        for (governed, other, configurations) in policy.module_cycles.difference(&module_cycles) {
            errors.push(format!("stale module-cycle {governed} {other} {configurations}"));
        }
        for (module, configurations) in module_sccs.difference(&policy.module_sccs) {
            errors.push(format!(
                "module {module} joins a governed module-level SCC configurations={configurations}; \
                 review the new module-level cycle and record a module-scc line"
            ));
        }
        for (module, configurations) in policy.module_sccs.difference(&module_sccs) {
            errors.push(format!("stale module-scc {module} {configurations}"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Edges are frozen in the policy only when at least one endpoint is a
/// governed module or the crate root; the rest of the crate is extracted for
/// reachability and SCCs but its edges are not inventoried.
fn frozen_scope(policy: &Policy, module: &str) -> bool {
    module == "root" || policy.classified.get(module).map(String::as_str) == Some("governed")
}

fn direct_reader_request_variant(target: &str) -> bool {
    let segments = target.split("::").collect::<Vec<_>>();
    segments.windows(2).any(|pair| {
        pair[0] == "ReaderRequest" && pair[1].chars().next().is_some_and(char::is_uppercase)
    })
}

fn root_reexport_indirection(
    module: &str,
    target: &str,
    root_aliases: Option<&BTreeMap<String, String>>,
) -> bool {
    // `super::X` from a top-level module names the root just as `crate::X`.
    let relative = qualify_relative(module, target);
    let Some(rest) = target.strip_prefix("crate::").or(relative.as_deref()) else {
        return false;
    };
    let first = rest.split("::").next().unwrap_or(rest);
    root_aliases.is_some_and(|aliases| aliases.contains_key(first))
}

/// Associated functions that return `Self` through a derivable trait, so
/// `Type::f(..)` has type `Type` without a visible `impl` returning it.
const DERIVED_CONSTRUCTORS: [&str; 2] = ["default", "clone"];

/// Traits whose associated functions (`Default::default()`, `From::from`)
/// return a type the call does not name.
const CONSTRUCTOR_TRAITS: [&str; 7] =
    ["Default", "From", "TryFrom", "FromStr", "FromIterator", "Clone", "Into"];

/// Engine method name to each defining module and its active configurations.
type MethodOwners = BTreeMap<String, BTreeMap<String, ConfigSet>>;

type EdgeKey = (String, String, String, String, String);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ResolvedTarget {
    module: String,
    item: String,
}

/// Dependency-graph node: a module, except that the crate root is split into
/// its items so that unrelated root helpers do not form paths.
fn dependency_node(module: &str, item: &str) -> String {
    if module == "root" {
        graph_node(module, item)
    } else {
        module.to_string()
    }
}

fn classification<'p>(policy: &'p Policy, module: &str) -> Option<&'p str> {
    policy.classified.get(module).map(String::as_str)
}

/// Checks the reviewed cycle and admission directives against the
/// classifications and the extracted edges.
fn validate_cycle_policy(
    policy: &Policy,
    merged_edge_kinds: &BTreeSet<EdgeKey>,
    errors: &mut Vec<String>,
) {
    if classification(policy, "root") != Some("admitted") {
        errors.push(
            "module root must be classified admitted: it is the composition root whose \
             contracts are admitted item by item"
                .to_string(),
        );
    }
    for (left, right) in &policy.allowed_cycles {
        let kinds = [classification(policy, left), classification(policy, right)];
        let joins_governed = kinds.contains(&Some("governed"));
        let bounded = kinds.iter().all(|kind| matches!(kind, Some("governed" | "admitted")));
        if !joins_governed || !bounded {
            errors.push(format!(
                "allow-cycle {left} {right} must join a governed module with a governed or \
                 admitted module (found {:?} / {:?})",
                kinds[0], kinds[1]
            ));
        }
        if policy.forbidden_cycles.contains(&(left.clone(), right.clone())) {
            errors.push(format!("allow-cycle {left} {right} names a forbidden cycle"));
        }
    }
    for (left, right) in &policy.reported_cycles {
        let mut kinds = [classification(policy, left), classification(policy, right)];
        kinds.sort();
        if kinds != [Some("governed"), Some("reported")] {
            errors.push(format!(
                "report-cycle {left} {right} must join a governed module with a reported module"
            ));
        }
        if policy.forbidden_cycles.contains(&(left.clone(), right.clone()))
            || policy.allowed_cycles.contains(&(left.clone(), right.clone()))
        {
            errors.push(format!("report-cycle {left} {right} is also forbidden or allowed"));
        }
    }
    for (source, source_item, target, target_item) in &policy.admissions {
        if classification(policy, source) != Some("admitted") {
            errors.push(format!(
                "admit-type source {source} is not an admitted module ({source_item} -> \
                 {target}::{target_item})"
            ));
        }
        if classification(policy, target) != Some("governed") {
            errors.push(format!(
                "admit-type target {target} is not a governed module ({source}::{source_item} -> \
                 {target_item})"
            ));
        }
        let key = (
            source.clone(),
            source_item.clone(),
            target.clone(),
            target_item.clone(),
            EdgeKind::Type.as_str().to_string(),
        );
        if !merged_edge_kinds.contains(&key) {
            errors.push(format!(
                "stale admit-type {source} {source_item} {target} {target_item}: no such type edge"
            ));
        }
    }
    for (module, kind) in &policy.classified {
        if kind != "admitted" || module == "root" {
            continue;
        }
        let named =
            policy.allowed_cycles.iter().any(|(left, right)| left == module || right == module)
                || policy.admissions.iter().any(|(source, ..)| source == module);
        if !named {
            errors.push(format!(
                "admitted module {module} has no allow-cycle or admit-type entry; classify it \
                 reported"
            ));
        }
    }
}

/// A cycle directive names modules; it also covers their descendants, so
/// `graph_expand` covers `graph_expand::traversal`.
fn covers(directive: &str, module: &str) -> bool {
    module == directive || module.starts_with(&format!("{directive}::"))
}

fn matching_pairs<'p>(
    directives: &'p BTreeSet<(String, String)>,
    left: &'p str,
    right: &'p str,
) -> impl Iterator<Item = &'p (String, String)> + 'p {
    directives.iter().filter(move |(first, second)| {
        (covers(first, left) && covers(second, right))
            || (covers(first, right) && covers(second, left))
    })
}

/// SCCs of both graphs across configurations, and the module pairs they join.
#[derive(Default)]
struct CycleFindings {
    components: BTreeMap<(&'static str, BTreeSet<String>), ConfigSet>,
    pairs: BTreeMap<(String, String), ConfigSet>,
    unapproved: BTreeMap<(String, String, &'static str), (ConfigSet, BTreeSet<String>)>,
    forbidden: BTreeMap<(String, String, &'static str), (ConfigSet, BTreeSet<String>)>,
}

impl CycleFindings {
    fn record(
        &mut self,
        policy: &Policy,
        graph: &'static str,
        configuration: usize,
        component: BTreeSet<String>,
    ) {
        let modules = component.iter().map(|node| node_module(node)).collect::<BTreeSet<_>>();
        for governed in
            modules.iter().filter(|module| classification(policy, module) == Some("governed"))
        {
            for other in modules.iter().filter(|other| *other != governed) {
                let pair = sorted_pair(governed, other);
                let mut accounted = false;
                for directive in matching_pairs(&policy.forbidden_cycles, governed, other) {
                    let entry = self
                        .forbidden
                        .entry((directive.0.clone(), directive.1.clone(), graph))
                        .or_insert_with(|| (ConfigSet::default(), component.clone()));
                    entry.0.insert(configuration);
                    accounted = true;
                }
                for directive in matching_pairs(&policy.allowed_cycles, governed, other)
                    .chain(matching_pairs(&policy.reported_cycles, governed, other))
                {
                    self.pairs.entry(directive.clone()).or_default().insert(configuration);
                    accounted = true;
                }
                if !accounted {
                    let entry = self
                        .unapproved
                        .entry((pair.0, pair.1, graph))
                        .or_insert_with(|| (ConfigSet::default(), component.clone()));
                    entry.0.insert(configuration);
                }
            }
        }
        self.components.entry((graph, component)).or_default().insert(configuration);
    }

    fn report_errors(&self, policy: &Policy, space: &ConfigSpace, errors: &mut Vec<String>) {
        for ((left, right, graph), (configurations, component)) in &self.forbidden {
            errors.push(format!(
                "forbidden cycle {left} <-> {right} graph={graph} configurations={} in SCC {:?}",
                space.expression(*configurations),
                component
            ));
        }
        for ((left, right, graph), (configurations, component)) in &self.unapproved {
            errors.push(format!(
                "unapproved governed cycle {left} <-> {right} graph={graph} configurations={} in \
                 SCC {:?}; the pair needs a reviewed allow-cycle (governed/admitted) or \
                 report-cycle (reported) entry",
                space.expression(*configurations),
                component
            ));
        }
        for (label, pairs) in
            [("allow-cycle", &policy.allowed_cycles), ("report-cycle", &policy.reported_cycles)]
        {
            for (left, right) in pairs {
                if !self.pairs.contains_key(&(left.clone(), right.clone())) {
                    errors.push(format!(
                        "stale {label} {left} {right}: no evaluated configuration has a cycle \
                         joining this pair"
                    ));
                }
            }
        }
    }
}

/// The frozen module-level cycle inventory: every direct 2-cycle of the
/// module dependency graph with a governed member, and every module (root
/// items collapsed to `root`) inside a module-level SCC that contains a
/// governed module, each with the configurations it holds in.
type ModuleCycles = BTreeSet<(String, String, String)>;
type ModuleSccs = BTreeSet<(String, String)>;

fn module_cycle_inventory(
    policy: &Policy,
    module_graph: &MaskedGraph,
    space: &ConfigSpace,
) -> (ModuleCycles, ModuleSccs) {
    let governed = |node: &str| classification(policy, &node_module(node)) == Some("governed");
    let token = |node: &str| node.replacen('#', "::", 1);
    let mut cycles = BTreeMap::<(String, String), ConfigSet>::new();
    for ((from, to), configurations) in &module_graph.edges {
        let Some(back) = module_graph.edges.get(&(to.clone(), from.clone())) else { continue };
        let both = configurations.intersect(*back);
        if both.is_empty() || !(governed(from) || governed(to)) {
            continue;
        }
        // Governed member first; two governed members in name order.
        let (first, second) =
            if governed(from) && (!governed(to) || from < to) { (from, to) } else { (to, from) };
        cycles
            .entry((token(first), token(second)))
            .and_modify(|set| *set = set.union(both))
            .or_insert(both);
    }
    let mut members = BTreeMap::<String, ConfigSet>::new();
    for (index, component) in module_graph.components_by_configuration() {
        if !component.iter().any(|node| governed(node)) {
            continue;
        }
        for node in &component {
            members.entry(node_module(node)).or_default().insert(index);
        }
    }
    (
        cycles
            .into_iter()
            .map(|((first, second), set)| (first, second, space.expression(set)))
            .collect(),
        members.into_iter().map(|(module, set)| (module, space.expression(set))).collect(),
    )
}

fn graph_node(module: &str, item: &str) -> String {
    format!("{module}#{item}")
}

fn node_module(node: &str) -> String {
    node.split_once('#').map_or(node, |(module, _)| module).to_string()
}

fn reject_relevant_macros(
    physical_module: &str,
    info: &ModuleInfo,
    policy: &Policy,
    source_root: &Path,
    actual_local_macros: &mut BTreeSet<(String, String, String)>,
    errors: &mut Vec<String>,
) {
    for usage in &info.analysis.macros {
        let module = scoped_module(physical_module, &usage.source_scope);
        let governed = policy.classified.get(&module).map(String::as_str) == Some("governed");
        if let Some(name) = usage.target.strip_prefix("macro_rules::") {
            let fingerprint = usage.fingerprint.as_deref().expect("definition fingerprint");
            let actual = (module.clone(), name.to_string(), fingerprint.to_string());
            actual_local_macros.insert(actual.clone());
            if policy.allowed_local_macros.contains(&actual) {
                continue;
            }
            if policy
                .allowed_local_macros
                .iter()
                .any(|(owner, allowed_name, _)| owner == &module && allowed_name == name)
            {
                errors.push(format!(
                    "local macro definition fingerprint mismatch source={module} macro={name} actual={fingerprint} at {}:{}:{}",
                    relative(source_root, &info.file),
                    usage.location.line,
                    usage.location.column
                ));
            } else {
                errors.push(format!(
                    "unreviewed local macro definition source={module} item={} macro={name} at {}:{}:{}",
                    usage.source_item,
                    relative(source_root, &info.file),
                    usage.location.line,
                    usage.location.column
                ));
            }
            continue;
        }
        let internal = usage.target == "include"
            || usage.target.starts_with("crate::")
            || usage.target.starts_with("super::")
            || usage.target.starts_with("self::");
        if governed && internal {
            errors.push(format!(
                "unreviewed governed macro source={module} item={} macro={} at {}:{}:{}",
                usage.source_item,
                usage.target,
                relative(source_root, &info.file),
                usage.location.line,
                usage.location.column
            ));
        }
    }
}

type UnparsedMacro = (String, String, String, String);

fn reject_unparsed_macros(
    physical_module: &str,
    info: &ModuleInfo,
    policy: &Policy,
    source_root: &Path,
    actual_unparsed_macros: &mut BTreeSet<UnparsedMacro>,
    errors: &mut Vec<String>,
) {
    for usage in &info.analysis.unparsed_macros {
        let module = scoped_module(physical_module, &usage.source_scope);
        let fingerprint = usage.fingerprint.clone().expect("unparsed macro fingerprint");
        actual_unparsed_macros.insert((
            module.clone(),
            usage.source_item.clone(),
            usage.target.clone(),
            fingerprint.clone(),
        ));
        let key = (module.clone(), usage.source_item.clone(), usage.target.clone(), fingerprint);
        if policy.allowed_unparsed_macros.contains(&key) {
            continue;
        }
        errors.push(format!(
            "unparsed macro body source={module} item={} macro={} at {}:{}:{} fingerprint={}; \
             its dependencies are invisible, so rewrite it as parseable Rust or record a \
             reviewed unparsed-macro exception",
            usage.source_item,
            usage.target,
            relative(source_root, &info.file),
            usage.location.line,
            usage.location.column,
            key.3
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn edge_targets(
    module: &str,
    edge: &fathomdb_module_boundary_gate::Edge,
    modules: &BTreeMap<String, ModuleInfo>,
    root_aliases: Option<&BTreeMap<String, String>>,
    engine_fields: &BTreeSet<String>,
    field_owners: &BTreeMap<String, String>,
    method_owners: &MethodOwners,
) -> BTreeSet<ResolvedTarget> {
    let targets = direct_edge_targets(
        module,
        edge,
        modules,
        root_aliases,
        engine_fields,
        field_owners,
        method_owners,
    );
    if matches!(edge.kind, EdgeKind::FieldAccess | EdgeKind::EngineMethod) {
        return targets;
    }
    targets.into_iter().flat_map(|target| chase_reexports(target, modules)).collect()
}

/// Follows `use` aliases (at any visibility, since descendants may name a
/// parent's private import), `use` aliases inside inline modules, `type`
/// aliases and single-candidate glob re-exports from the module a path
/// resolved to, until the path reaches its defining module. A `use` alias is
/// replaced by what it names; a `type` alias is kept and every in-crate path
/// its definition names is added. Chains that leave the crate stop at the
/// last in-crate hop.
fn chase_reexports(
    start: ResolvedTarget,
    modules: &BTreeMap<String, ModuleInfo>,
) -> BTreeSet<ResolvedTarget> {
    let mut results = BTreeSet::new();
    let mut pending = vec![start.clone()];
    let mut seen = BTreeSet::new();
    while let Some(current) = pending.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        let (keep, next) = chase_step(&current, modules);
        if keep || next.is_empty() {
            results.insert(current);
        }
        pending.extend(next);
    }
    if results.is_empty() {
        results.insert(start);
    }
    results
}

/// One resolution hop: whether `current` itself stays a target, and the
/// targets it leads to.
fn chase_step(
    current: &ResolvedTarget,
    modules: &BTreeMap<String, ModuleInfo>,
) -> (bool, Vec<ResolvedTarget>) {
    if current.item == "<module>" {
        return (true, Vec::new());
    }
    let Some(info) = modules.get(&current.module) else { return (true, Vec::new()) };
    let segments = current.item.split("::").collect::<Vec<_>>();
    let inline_scopes = info
        .analysis
        .modules
        .iter()
        .filter(|declaration| declaration.inline)
        .map(|declaration| {
            if declaration.scope.is_empty() {
                declaration.name.clone()
            } else {
                format!("{}::{}", declaration.scope, declaration.name)
            }
        })
        .collect::<BTreeSet<_>>();
    let depth = (0..segments.len())
        .rev()
        .find(|end| *end == 0 || inline_scopes.contains(&segments[..*end].join("::")))
        .unwrap_or(0);
    let scope = segments[..depth].join("::");
    let head = segments[depth];
    let rest = (depth + 1 < segments.len()).then(|| segments[depth + 1..].join("::"));
    let scoped_module = scoped_module(&current.module, &scope);
    let aliases = if scope.is_empty() {
        Some(&info.analysis.import_aliases)
    } else {
        info.analysis.scoped_aliases.get(&scope)
    };
    let with_rest =
        |path: String| rest.as_ref().map_or_else(|| path.clone(), |rest| format!("{path}::{rest}"));
    if let Some(alias) = aliases.and_then(|aliases| aliases.get(head)) {
        let next = qualify_alias(&scoped_module, alias, modules)
            .and_then(|path| resolve_path(&with_rest(path), modules));
        return (next.is_none(), next.into_iter().collect());
    }
    let alias_key = if scope.is_empty() { head.to_string() } else { format!("{scope}::{head}") };
    if let Some(alias) = info.analysis.type_aliases.get(&alias_key) {
        let qualify = |path: &str| qualify_in_scope(&current.module, &scope, info, path, modules);
        let mut next = Vec::new();
        if let Some(primary) = &alias.primary {
            next.extend(qualify(primary).and_then(|path| resolve_path(&with_rest(path), modules)));
        }
        for path in alias.mentioned.iter().filter(|path| Some(*path) != alias.primary.as_ref()) {
            next.extend(qualify(path).and_then(|path| resolve_path(&path, modules)));
        }
        return (true, next);
    }
    if info.analysis.declared_items.contains(head) {
        return (true, Vec::new());
    }
    let candidates = info
        .analysis
        .edges
        .iter()
        .filter(|edge| {
            matches!(edge.kind, EdgeKind::Import | EdgeKind::Reexport)
                && edge.source_scope == scope
                && edge.target.ends_with("::*")
        })
        .filter_map(|glob| {
            let namespace =
                qualify_glob_namespace(&scoped_module, glob.target.trim_end_matches("::*"));
            modules
                .get(&namespace)
                .filter(|owner| {
                    owner.analysis.declared_items.contains(head)
                        || owner.analysis.import_aliases.contains_key(head)
                })
                .map(|_| format!("{namespace}::{head}"))
        })
        .collect::<BTreeSet<_>>();
    if candidates.len() != 1 {
        return (true, Vec::new());
    }
    let next =
        candidates.into_iter().next().and_then(|path| resolve_path(&with_rest(path), modules));
    (next.is_none(), next.into_iter().collect())
}

/// Qualifies a path written in a `type` definition inside `module` (and its
/// inline `scope`) to a crate-relative path, or `None` for another crate.
fn qualify_in_scope(
    module: &str,
    scope: &str,
    info: &ModuleInfo,
    path: &str,
    modules: &BTreeMap<String, ModuleInfo>,
) -> Option<String> {
    let scoped = scoped_module(module, scope);
    let (head, rest) =
        path.split_once("::").map_or((path, None), |(head, rest)| (head, Some(rest)));
    if matches!(head, "crate" | "self" | "super") {
        return qualify_alias(&scoped, path, modules);
    }
    let alias = if scope.is_empty() { None } else { info.analysis.scoped_aliases.get(scope) }
        .and_then(|aliases| aliases.get(head))
        .or_else(|| info.analysis.import_aliases.get(head));
    if let Some(alias) = alias {
        let full = rest.map_or_else(|| alias.clone(), |rest| format!("{alias}::{rest}"));
        return qualify_alias(&scoped, &full, modules);
    }
    if info.analysis.declared_items.contains(head) {
        return Some(if scoped == "root" { path.to_string() } else { format!("{scoped}::{path}") });
    }
    qualify_alias(&scoped, path, modules)
}

/// Qualifies a `use` path written inside `module` to a crate-relative path
/// (without `crate::`), or `None` when it names another crate.
fn qualify_alias(
    module: &str,
    alias: &str,
    modules: &BTreeMap<String, ModuleInfo>,
) -> Option<String> {
    if let Some(rest) = alias.strip_prefix("crate::") {
        return Some(rest.to_string());
    }
    let mut base = module.to_string();
    let mut rest = alias;
    let mut relative = false;
    loop {
        if let Some(tail) = rest.strip_prefix("super::") {
            base = base.rsplit_once("::").map_or("root", |(parent, _)| parent).to_string();
            rest = tail;
            relative = true;
        } else if let Some(tail) = rest.strip_prefix("self::") {
            rest = tail;
            relative = true;
        } else {
            break;
        }
    }
    let joined = if base == "root" { rest.to_string() } else { format!("{base}::{rest}") };
    if relative {
        return Some(joined);
    }
    let first = rest.split("::").next().unwrap_or(rest);
    let child = if base == "root" { first.to_string() } else { format!("{base}::{first}") };
    modules.contains_key(&child).then_some(joined)
}

#[allow(clippy::too_many_arguments)]
fn direct_edge_targets(
    module: &str,
    edge: &fathomdb_module_boundary_gate::Edge,
    modules: &BTreeMap<String, ModuleInfo>,
    root_aliases: Option<&BTreeMap<String, String>>,
    engine_fields: &BTreeSet<String>,
    field_owners: &BTreeMap<String, String>,
    method_owners: &MethodOwners,
) -> BTreeSet<ResolvedTarget> {
    if edge.kind == EdgeKind::FieldAccess {
        return if engine_fields.contains(&edge.target) {
            field_owners
                .get(&edge.target)
                .map(|owner| ResolvedTarget { module: owner.clone(), item: edge.target.clone() })
                .into_iter()
                .collect()
        } else {
            BTreeSet::new()
        };
    }
    if edge.kind == EdgeKind::EngineMethod {
        return method_owners
            .get(&edge.target)
            .into_iter()
            .flat_map(BTreeMap::keys)
            .map(|owner| ResolvedTarget { module: owner.clone(), item: edge.target.clone() })
            .collect();
    }
    if let Some(rest) = edge.target.strip_prefix(".<") {
        let Some((method, receiver_type)) = rest.split_once(">:") else {
            return BTreeSet::new();
        };
        let mut typed = edge.clone();
        typed.target = format!("{receiver_type}::{method}");
        return direct_edge_targets(
            module,
            &typed,
            modules,
            root_aliases,
            engine_fields,
            field_owners,
            method_owners,
        );
    }
    let relative = qualify_relative(module, &edge.target);
    let mut target = edge.target.as_str();
    let crate_qualified = target.starts_with("crate::") || relative.is_some();
    if let Some(rest) = target.strip_prefix("crate::") {
        target = rest;
    } else if let Some(qualified) = &relative {
        // A relative path that reaches the crate root resolves like
        // `crate::`: through root re-exports and root items.
        target = qualified;
    }
    if let Some(owner) = resolve_path(target, modules) {
        return BTreeSet::from([owner]);
    }
    let first = target.split("::").next().unwrap_or(target);
    if let Some(root_target) = root_aliases.and_then(|aliases| aliases.get(first)) {
        let root_target = root_target.strip_prefix("crate::").unwrap_or(root_target);
        if let Some(owner) = resolve_path(root_target, modules) {
            return BTreeSet::from([owner]);
        }
    }
    if edge.kind == EdgeKind::Callable && !target.contains("::") {
        let physical = modules
            .keys()
            .filter(|candidate| {
                module == candidate.as_str() || module.starts_with(&format!("{candidate}::"))
            })
            .max_by_key(|candidate| candidate.len())
            .map_or(module, String::as_str);
        if !crate_qualified
            && modules
                .get(physical)
                .is_some_and(|info| info.analysis.local_functions.contains_key(target))
        {
            return BTreeSet::from([ResolvedTarget {
                module: module.to_string(),
                item: target.to_string(),
            }]);
        }
        let mut candidates = BTreeSet::new();
        if let Some(info) = modules.get(physical) {
            for glob in info.analysis.edges.iter().filter(|candidate| {
                candidate.kind == EdgeKind::Import
                    && candidate.target.ends_with("::*")
                    && candidate.source_scope == edge.source_scope
            }) {
                let namespace = glob.target.trim_end_matches("::*");
                let namespace = qualify_glob_namespace(module, namespace);
                if namespace == "root" {
                    if let Some(root_target) = root_aliases.and_then(|aliases| aliases.get(target))
                    {
                        let root_target =
                            root_target.strip_prefix("crate::").unwrap_or(root_target);
                        if let Some(owner) = resolve_path(root_target, modules) {
                            candidates.insert(owner);
                        }
                    } else if modules
                        .get("root")
                        .is_some_and(|root| root.analysis.declared_items.contains(target))
                    {
                        candidates.insert(ResolvedTarget {
                            module: "root".to_string(),
                            item: target.to_string(),
                        });
                    }
                } else if let Some(owner) = modules.get(&namespace) {
                    if owner.analysis.declared_items.contains(target) {
                        candidates
                            .insert(ResolvedTarget { module: namespace, item: target.to_string() });
                    }
                }
            }
        }
        if !candidates.is_empty() {
            return candidates;
        }
    }
    if modules.get("root").is_some_and(|root| root.analysis.declared_items.contains(target)) {
        return BTreeSet::from([ResolvedTarget {
            module: "root".to_string(),
            item: target.to_string(),
        }]);
    }
    if !crate_qualified && target.contains("::") {
        let head = target.split("::").next().unwrap_or(target);
        let physical = modules
            .keys()
            .filter(|candidate| {
                module == candidate.as_str() || module.starts_with(&format!("{candidate}::"))
            })
            .max_by_key(|candidate| candidate.len());
        if physical
            .and_then(|physical| modules.get(physical))
            .is_some_and(|info| info.analysis.declared_items.contains(head))
        {
            return BTreeSet::from([ResolvedTarget {
                module: module.to_string(),
                item: target.to_string(),
            }]);
        }
    }
    BTreeSet::new()
}

/// Resolves any chain of leading `self::`/`super::` segments against
/// `module`; `None` when the path does not start with one.
fn qualify_relative(module: &str, target: &str) -> Option<String> {
    let mut base = module.to_string();
    let mut rest = target;
    let mut relative = false;
    loop {
        if let Some(tail) = rest.strip_prefix("super::") {
            base = base.rsplit_once("::").map_or("root", |(parent, _)| parent).to_string();
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("self::") {
            rest = tail;
        } else {
            break;
        }
        relative = true;
    }
    relative.then(|| if base == "root" { rest.to_string() } else { format!("{base}::{rest}") })
}

fn qualify_glob_namespace(module: &str, namespace: &str) -> String {
    if namespace == "crate" {
        return "root".to_string();
    }
    if let Some(rest) = namespace.strip_prefix("crate::") {
        return rest.to_string();
    }
    if namespace == "self" {
        return module.to_string();
    }
    if let Some(rest) = namespace.strip_prefix("self::") {
        return if module == "root" { rest.to_string() } else { format!("{module}::{rest}") };
    }
    if namespace == "super" {
        return module.rsplit_once("::").map_or("root", |(parent, _)| parent).to_string();
    }
    if let Some(rest) = namespace.strip_prefix("super::") {
        let parent = module.rsplit_once("::").map_or("root", |(parent, _)| parent);
        return if parent == "root" { rest.to_string() } else { format!("{parent}::{rest}") };
    }
    namespace.to_string()
}

/// A crate-relative type path (`module::Type`, or `Type` for a root item).
fn resolve_type(ty: &str, modules: &BTreeMap<String, ModuleInfo>) -> Option<ResolvedTarget> {
    resolve_path(ty, modules).filter(|target| target.item != "<module>").or_else(|| {
        (!ty.contains("::")
            && modules.get("root").is_some_and(|root| root.analysis.declared_items.contains(ty)))
        .then(|| ResolvedTarget { module: "root".to_string(), item: ty.to_string() })
    })
}

fn resolve_path(target: &str, modules: &BTreeMap<String, ModuleInfo>) -> Option<ResolvedTarget> {
    let module = longest_module_prefix(target, modules)?;
    let item = target
        .strip_prefix(&module)
        .and_then(|rest| rest.strip_prefix("::"))
        .unwrap_or("<module>")
        .to_string();
    Some(ResolvedTarget { module, item })
}

fn longest_module_prefix(target: &str, modules: &BTreeMap<String, ModuleInfo>) -> Option<String> {
    let segments = target.split("::").collect::<Vec<_>>();
    (1..=segments.len())
        .rev()
        .map(|end| segments[..end].join("::"))
        .find(|candidate| modules.contains_key(candidate))
}

/// A directed graph whose edges carry the configurations they are active in.
struct MaskedGraph {
    edges: BTreeMap<(String, String), ConfigSet>,
}

impl MaskedGraph {
    fn new() -> Self {
        Self { edges: BTreeMap::new() }
    }

    fn add(&mut self, from: &str, to: &str, configurations: ConfigSet) {
        let entry = self.edges.entry((from.to_string(), to.to_string())).or_default();
        *entry = entry.union(configurations);
    }

    /// Nontrivial SCCs of each configuration's graph. Any cycle of one
    /// configuration lies inside an SCC of the union graph, so only those
    /// SCCs are re-examined per configuration.
    fn components_by_configuration(&self) -> Vec<(usize, BTreeSet<String>)> {
        let mut union = BTreeMap::<String, BTreeSet<String>>::new();
        for (from, to) in self.edges.keys() {
            union.entry(from.clone()).or_default().insert(to.clone());
        }
        let nodes = union.keys().chain(union.values().flatten()).cloned().collect();
        let mut found = Vec::new();
        for component in strongly_connected(&union, &nodes) {
            if component.len() < 2 {
                continue;
            }
            let internal = self
                .edges
                .iter()
                .filter(|((from, to), _)| component.contains(from) && component.contains(to))
                .collect::<Vec<_>>();
            let active = internal
                .iter()
                .fold(ConfigSet::default(), |set, (_, configurations)| set.union(**configurations));
            for index in active.indices() {
                let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
                for ((from, to), configurations) in &internal {
                    if configurations.contains(index) {
                        adjacency.entry(from.clone()).or_default().insert(to.clone());
                    }
                }
                for sub in strongly_connected(&adjacency, &component) {
                    if sub.len() >= 2 {
                        found.push((index, sub));
                    }
                }
            }
        }
        found
    }
}

fn strongly_connected(
    adjacency: &BTreeMap<String, BTreeSet<String>>,
    nodes: &BTreeSet<String>,
) -> Vec<BTreeSet<String>> {
    fn finish_order(
        node: &str,
        adjacency: &BTreeMap<String, BTreeSet<String>>,
        visited: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) {
        if !visited.insert(node.to_string()) {
            return;
        }
        if let Some(targets) = adjacency.get(node) {
            for target in targets {
                finish_order(target, adjacency, visited, order);
            }
        }
        order.push(node.to_string());
    }

    fn collect_component(
        node: &str,
        reverse: &BTreeMap<String, BTreeSet<String>>,
        assigned: &mut BTreeSet<String>,
        component: &mut BTreeSet<String>,
    ) {
        if !assigned.insert(node.to_string()) {
            return;
        }
        component.insert(node.to_string());
        if let Some(sources) = reverse.get(node) {
            for source in sources {
                collect_component(source, reverse, assigned, component);
            }
        }
    }

    let mut order = Vec::with_capacity(nodes.len());
    let mut visited = BTreeSet::new();
    for node in nodes {
        finish_order(node, adjacency, &mut visited, &mut order);
    }
    let mut reverse = BTreeMap::<String, BTreeSet<String>>::new();
    for (source, targets) in adjacency {
        for target in targets {
            reverse.entry(target.clone()).or_default().insert(source.clone());
        }
    }
    let mut assigned = BTreeSet::new();
    let mut components = Vec::new();
    for node in order.into_iter().rev() {
        if assigned.contains(&node) {
            continue;
        }
        let mut component = BTreeSet::new();
        collect_component(&node, &reverse, &mut assigned, &mut component);
        components.push(component);
    }
    components
}

fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root).unwrap_or(file).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn policy_rejects_duplicate_exact_maps() {
        let nonce =
            SystemTime::now().duration_since(UNIX_EPOCH).expect("clock after epoch").as_nanos();
        let path = env::temp_dir()
            .join(format!("fathomdb-module-boundary-policy-{}-{nonce}.txt", std::process::id()));
        fs::write(
            &path,
            "version 1\ngoverned root\nfield path root\nfield path root\nowner Item root\nowner Item root\n",
        )
        .expect("write policy fixture");
        let errors = parse_policy(&path).expect_err("duplicates must fail");
        fs::remove_file(path).expect("remove policy fixture");
        assert!(errors.iter().any(|error| error.contains("duplicates Engine field path")));
        assert!(errors.iter().any(|error| error.contains("duplicates owner assertion Item")));
    }

    #[test]
    fn direct_reader_request_variants_are_distinct_from_factories() {
        assert!(direct_reader_request_variant("crate::reader_pool::ReaderRequest::Search"));
        assert!(!direct_reader_request_variant("crate::reader_pool::ReaderRequest::search"));
    }
}
