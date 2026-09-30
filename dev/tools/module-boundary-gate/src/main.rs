use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fathomdb_module_boundary_gate::{
    analyze_source, analyze_source_in, Analysis, ConfigSet, ConfigSpace, EdgeKind,
};

#[derive(Debug, Default)]
struct Policy {
    classified: BTreeMap<String, String>,
    field_owners: BTreeMap<String, String>,
    owners: BTreeMap<String, String>,
    forbidden_dependencies: BTreeSet<(String, String)>,
    forbidden_cycles: BTreeSet<(String, String)>,
    /// Every forbid line as written: its line number, directive and
    /// operands.
    forbid_lines: Vec<(usize, String, String, String)>,
    allowed_cycles: BTreeSet<(String, String)>,
    reported_cycles: BTreeSet<(String, String)>,
    admissions: BTreeSet<(String, String, String, String)>,
    allowed_local_macros: BTreeSet<(String, String, String)>,
    allowed_unparsed_macros: BTreeSet<(String, String, String, String)>,
    configuration_features: Vec<(String, String)>,
    configuration_cases: Vec<(String, Vec<String>)>,
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
        &policy.configuration_cases,
    )?;
    let mut policy_errors = Vec::new();
    let modules = discover_modules(&source_root, &space)?;
    policy_errors.extend(enforce_required_forbids(&mut policy, &module_inventory(&modules)));
    let external_crates = manifest_crate_names(&manifest);
    let mut result =
        evaluate(&source_root, &modules, &policy, &space, &external_crates, report_only);
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
        "ok    module-boundary: {} modules, {} governed, {} reviewed configurations",
        module_inventory(&modules).len(),
        policy.classified.values().filter(|kind| kind.as_str() == "governed").count(),
        space.len()
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
    for info in modules.values() {
        for (name, location) in &info.analysis.path_attributes {
            errors.push(format!(
                "unsupported #[path] on mod {name} at {}:{}:{}; the compiler would build a file \
                 other than the one the gate analyses for this module",
                relative(source_root, &info.file),
                location.line,
                location.column
            ));
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
            ["configuration-case", name, features] => {
                policy.configuration_cases.push((
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
            [directive @ "forbid-dependency", source, target] => {
                policy.forbid_lines.push((
                    index + 1,
                    (*directive).to_string(),
                    (*source).to_string(),
                    (*target).to_string(),
                ));
                policy
                    .forbidden_dependencies
                    .insert(((*source).to_string(), (*target).to_string()));
            }
            [directive @ "forbid-cycle", left, right] => {
                policy.forbid_lines.push((
                    index + 1,
                    (*directive).to_string(),
                    (*left).to_string(),
                    (*right).to_string(),
                ));
                policy.forbidden_cycles.insert(sorted_pair(left, right));
            }
            ["allow-cycle", left, right] => {
                policy.allowed_cycles.insert(sorted_pair(left, right));
            }
            ["report-cycle", left, right] => {
                policy.reported_cycles.insert(sorted_pair(left, right));
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

/// Cycles the release plan's boundary acceptance (AC27-85B/C) forbids
/// outright: none may be retained, excepted or allowlisted, so the gate
/// holds them itself rather than trusting the policy file to keep them.
const REQUIRED_FORBIDDEN_CYCLES: [(&str, &str); 5] = [
    ("graph_expand", "reader_pool"),
    ("graph_expand", "search"),
    ("graph_expand", "search_api"),
    ("read", "reader_pool"),
    ("reader_pool", "search"),
];

/// Dependency directions the same acceptance forbids outright.
const REQUIRED_FORBIDDEN_DEPENDENCIES: [(&str, &str); 8] = [
    ("filter", "search"),
    ("frozen_read", "search"),
    ("graph_expand", "reader_pool"),
    ("graph_expand", "search"),
    ("graph_expand", "search_api"),
    ("read", "reader_pool"),
    ("read", "search"),
    ("search", "reader_pool"),
];

/// Fails a policy that drops a required forbid, downgrades one to an
/// allow-cycle or report-cycle, or names a module that does not exist in a
/// forbid line (a typo would silently disable it). The required forbids
/// are then enforced whatever the policy says.
fn enforce_required_forbids(policy: &mut Policy, modules: &BTreeSet<String>) -> Vec<String> {
    let mut errors = Vec::new();
    for (line, directive, left, right) in &policy.forbid_lines {
        for operand in [left, right] {
            if !modules.contains(operand) {
                errors.push(format!(
                    "policy:{line} {directive} {left} {right} names no module {operand}"
                ));
            }
        }
    }
    for (left, right) in REQUIRED_FORBIDDEN_CYCLES {
        let pair = sorted_pair(left, right);
        if !policy.forbidden_cycles.contains(&pair) {
            errors.push(format!(
                "required forbid-cycle {left} {right} is missing from the policy; the gate holds \
                 this cycle forbidden, so the policy cannot drop or downgrade it"
            ));
        }
        for (label, pairs) in
            [("allow-cycle", &policy.allowed_cycles), ("report-cycle", &policy.reported_cycles)]
        {
            // A pair of descendants names the same cycle.
            let overlaps = |directive: &str, module: &str| {
                covers(directive, module) || covers(module, directive)
            };
            for (first, second) in pairs.iter().filter(|(first, second)| {
                (overlaps(first, left) && overlaps(second, right))
                    || (overlaps(first, right) && overlaps(second, left))
            }) {
                errors.push(format!("{label} {first} {second} names a required forbidden cycle"));
            }
        }
        policy.forbidden_cycles.insert(pair);
    }
    for (source, target) in REQUIRED_FORBIDDEN_DEPENDENCIES {
        let pair = (source.to_string(), target.to_string());
        if !policy.forbidden_dependencies.contains(&pair) {
            errors.push(format!(
                "required forbid-dependency {source} {target} is missing from the policy; the \
                 gate holds this direction forbidden, so the policy cannot drop it"
            ));
        }
        policy.forbidden_dependencies.insert(pair);
    }
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
    external_crates: &BTreeSet<String>,
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
    for (item, owner) in &policy.owners {
        match modules.get(owner) {
            Some(module) if module.analysis.declared_items.contains(item) => {}
            Some(_) => {
                errors.push(format!("owner assertion stale: {owner} does not declare {item}"))
            }
            None => errors.push(format!("owner assertion names missing module {owner} for {item}")),
        }
    }

    let namespaces = Namespaces::new(modules);
    // A crate-root alias that resolves to no in-crate item and names no
    // declared dependency would end every path through it on the alias
    // itself, severing the item and module graphs.
    for (name, target) in namespaces.bindings_of("root") {
        let first = target.split("::").next().unwrap_or(target);
        if external_crates.contains(first)
            || matches!(namespaces.resolve("root", target, Want::Module), Resolution::InCrate(_))
            || root.is_some_and(|root| root.analysis.declared_items.contains(first))
        {
            continue;
        }
        errors.push(format!(
            "unresolved crate-root alias {name} -> {target}; a crate-root `use` must name an \
             in-crate item or a declared dependency"
        ));
    }
    let mut admission_edges = BTreeSet::<EdgeKey>::new();
    let mut actual_local_macros = BTreeSet::new();
    let mut actual_unparsed_macros = BTreeSet::new();
    let mut item_graph = MaskedGraph::new();
    let mut module_dependencies = MaskedGraph::new();
    let mut governed_graph = MaskedGraph::new();
    for (module, info) in modules {
        for predicate in &info.analysis.unsupported_cfg {
            errors.push(format!(
                "unsupported cfg predicate in {}: {predicate}",
                relative(source_root, &info.file)
            ));
        }
        for (name, location) in &info.analysis.extern_crates {
            errors.push(format!(
                "unsupported extern crate declaration {name} at {}:{}:{}; it can rename this \
                 crate or another out of the gate's path resolution",
                relative(source_root, &info.file),
                location.line,
                location.column
            ));
        }
        for edge in &info.analysis.edges {
            let source_module = scoped_module(module, &edge.source_scope);
            let (targets, unresolved) = edge_targets(
                module,
                edge,
                modules,
                &namespaces,
                &engine_fields,
                &policy.field_owners,
                &method_owners,
            );
            for reason in unresolved {
                errors.push(format!(
                    "unresolved in-crate path {} at {}:{}:{} source={source_module} \
                         source_item={} kind={}: {reason}; every in-crate path must resolve to a \
                         declared item or module, so the gate fails closed rather than read it as \
                         another crate's",
                    edge.target,
                    relative(source_root, &info.file),
                    edge.location.line,
                    edge.location.column,
                    edge.source_item,
                    edge.kind.as_str()
                ));
            }
            // A `use` that names nothing in the crate must name a declared
            // dependency; otherwise it hides a path from the resolver.
            if matches!(edge.kind, EdgeKind::Import | EdgeKind::Reexport) && targets.is_empty() {
                let head = edge.target.split("::").next().unwrap_or(&edge.target);
                let written = scoped_module(module, &edge.target_scope);
                if !matches!(head, "crate" | "self" | "super")
                    && !external_crates.contains(head)
                    && !namespaces.binds(&written, head)
                {
                    errors.push(format!(
                        "unresolved in-crate path {} at {}:{}:{} source={source_module}: a `use` \
                         must name an in-crate item or module or a declared dependency",
                        edge.target,
                        relative(source_root, &info.file),
                        edge.location.line,
                        edge.location.column
                    ));
                }
            }
            if edge.configurations.is_empty() {
                if relevant_scope(policy, &source_module)
                    || targets.keys().any(|target| relevant_scope(policy, &target.module))
                {
                    errors.push(format!(
                        "edge has no evaluated configuration source={source_module} source_item={} \
                         syntax={} kind={} at {}:{}:{}; every edge in the boundary scope must be \
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
            if governed
                && edge.kind == EdgeKind::Import
                && ["crate::*", "super::*", "self::*"].contains(&edge.target.as_str())
            {
                errors.push(format!(
                    "governed internal glob import {}:{}:{}",
                    relative(source_root, &info.file),
                    edge.location.line,
                    edge.location.column
                ));
            }
            if governed
                && matches!(edge.kind, EdgeKind::Import | EdgeKind::Reexport | EdgeKind::Type)
                && root_reexport_indirection(&source_module, &edge.target, &namespaces)
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
                && (direct_reader_request_variant(&edge.target)
                    || targets.keys().any(|target| direct_reader_request_variant(&target.item)))
            {
                errors.push(format!(
                    "direct ReaderRequest variant construction outside reader_pool at {}:{}:{} item={}",
                    relative(source_root, &info.file),
                    edge.location.line,
                    edge.location.column,
                    edge.target
                ));
            }
            if governed
                && edge.kind == EdgeKind::Callable
                && !edge.target.starts_with(".<")
                && !edge.target.contains("::")
                && info.analysis.local_functions.contains_key(&edge.target)
                // An inline module's own `use` of the name wins over the
                // file's function of that name.
                && (edge.source_scope.is_empty()
                    || !namespaces.has_binding(&source_module, &edge.target))
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
            for (target, target_configurations) in targets {
                let nonroot_local = target.module == source_module && source_module != "root";
                let configurations = if edge.kind == EdgeKind::EngineMethod {
                    method_owners
                        .get(&edge.target)
                        .and_then(|owners| owners.get(&target.module))
                        .map_or(target_configurations, |owner| {
                            target_configurations.intersect(*owner)
                        })
                } else {
                    target_configurations
                };
                let admitted = edge.kind == EdgeKind::Type
                    && policy.admissions.contains(&(
                        source_module.clone(),
                        edge.source_item.clone(),
                        target.module.clone(),
                        target.item.clone(),
                    ));
                let engine_layout = source_module == "root" && edge.source_item == "Engine";
                let composition = edge.kind == EdgeKind::Reexport || admitted || engine_layout;
                let item_edge = !composition && edge.kind != EdgeKind::Import;
                let governed_edge = !composition
                    && governed
                    && target.module != source_module
                    && classification(policy, &target.module) == Some("governed");
                module_dependencies.add(&source_module, &target.module, configurations);
                if governed_edge {
                    governed_graph.add(&source_module, &target.module, configurations);
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
                if edge.kind == EdgeKind::Type && !configurations.is_empty() {
                    admission_edges.insert((
                        source_module.clone(),
                        edge.source_item.clone(),
                        target.module,
                        target.item,
                        "type".to_string(),
                    ));
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
        reject_include_imports(module, info, source_root, &mut errors);
        reject_unparsed_serde_paths(module, info, source_root, &mut errors);
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

    validate_cycle_policy(policy, &admission_edges, &mut errors);

    // A forbidden dependency covers descendant modules on both sides.
    for ((source, target), configurations) in &module_dependencies.edges {
        if source == target || !forbidden(policy, source, target) {
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

    if report_only {
        for module in &discovered {
            println!("module\t{module}");
        }
        for field in &engine_fields {
            println!("field\t{field}");
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
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Relevant zero-configuration edges fail independently of policy inventories.
fn relevant_scope(policy: &Policy, module: &str) -> bool {
    module == "root" || policy.classified.get(module).map(String::as_str) == Some("governed")
}

fn direct_reader_request_variant(target: &str) -> bool {
    let segments = target.split("::").collect::<Vec<_>>();
    segments.windows(2).any(|pair| {
        pair[0] == "ReaderRequest" && pair[1].chars().next().is_some_and(char::is_uppercase)
    })
}

fn root_reexport_indirection(module: &str, target: &str, namespaces: &Namespaces) -> bool {
    // `super::X` from a top-level module names the root just as `crate::X`.
    let relative = qualify_relative(module, target);
    let Some(rest) = target.strip_prefix("crate::").or(relative.as_deref()) else {
        return false;
    };
    let first = rest.split("::").next().unwrap_or(rest);
    namespaces.imported_at_root(first)
}

/// Engine method name to each defining module and its active configurations.
type MethodOwners = BTreeMap<String, BTreeMap<String, ConfigSet>>;

type EdgeKey = (String, String, String, String, String);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ResolvedTarget {
    module: String,
    item: String,
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

/// Whether a forbid-dependency line covers `source -> target`.
fn forbidden(policy: &Policy, source: &str, target: &str) -> bool {
    policy.forbidden_dependencies.iter().any(|(forbidden_source, forbidden_target)| {
        covers(forbidden_source, source) && covers(forbidden_target, target)
    })
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
        // `include!` splices source the gate never parses, in any module and
        // under any path (`std::prelude::v1::include!`);
        // `include_str!`/`include_bytes!` are data.
        if usage.target.rsplit("::").next() == Some("include") {
            errors.push(format!(
                "unreviewed include source={module} item={} macro={} at {}:{}:{}; the included \
                 source is not parsed, so declare it as a module instead",
                usage.source_item,
                usage.target,
                relative(source_root, &info.file),
                usage.location.line,
                usage.location.column
            ));
            continue;
        }
        let internal = usage.target.starts_with("crate::")
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

/// A `use` that imports `include`, renamed or not, would let `include!`
/// be invoked under a name the macro check cannot recognise.
fn reject_include_imports(
    physical_module: &str,
    info: &ModuleInfo,
    source_root: &Path,
    errors: &mut Vec<String>,
) {
    for edge in &info.analysis.edges {
        if !matches!(edge.kind, EdgeKind::Import | EdgeKind::Reexport)
            || edge.target.rsplit("::").next() != Some("include")
        {
            continue;
        }
        errors.push(format!(
            "unreviewed include import source={} item={} path={} at {}:{}:{}; `include!` \
             splices source the gate never parses, so declare it as a module instead",
            scoped_module(physical_module, &edge.source_scope),
            edge.source_item,
            edge.target,
            relative(source_root, &info.file),
            edge.location.line,
            edge.location.column
        ));
    }
}

fn reject_unparsed_serde_paths(
    physical_module: &str,
    info: &ModuleInfo,
    source_root: &Path,
    errors: &mut Vec<String>,
) {
    for usage in &info.analysis.unparsed_serde_paths {
        errors.push(format!(
            "unsupported owner-bearing attribute source={} item={} key={} value={} at {}:{}:{}; write an explicit owner path instead",
            scoped_module(physical_module, &usage.source_scope),
            usage.source_item,
            usage.key,
            usage.value,
            relative(source_root, &info.file),
            usage.location.line,
            usage.location.column
        ));
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

type Targets = BTreeMap<ResolvedTarget, ConfigSet>;
fn edge_targets(
    file: &str,
    edge: &fathomdb_module_boundary_gate::Edge,
    _modules: &BTreeMap<String, ModuleInfo>,
    resolver: &Namespaces,
    fields: &BTreeSet<String>,
    owners: &BTreeMap<String, String>,
    methods: &MethodOwners,
) -> (Targets, Vec<String>) {
    if edge.kind == EdgeKind::FieldAccess {
        return (
            owners
                .get(&edge.target)
                .filter(|_| fields.contains(&edge.target))
                .map(|owner| {
                    (
                        ResolvedTarget { module: owner.clone(), item: edge.target.clone() },
                        edge.configurations,
                    )
                })
                .into_iter()
                .collect(),
            Vec::new(),
        );
    }
    if edge.kind == EdgeKind::EngineMethod {
        return (
            methods
                .get(&edge.target)
                .into_iter()
                .flat_map(|owners| owners.iter())
                .map(|(owner, mask)| {
                    (
                        ResolvedTarget { module: owner.clone(), item: edge.target.clone() },
                        edge.configurations.intersect(*mask),
                    )
                })
                .collect(),
            Vec::new(),
        );
    }
    let node = scoped_module(file, &edge.target_scope);
    let import = matches!(edge.kind, EdgeKind::Import | EdgeKind::Reexport);
    let path = edge.target.strip_suffix("::*").unwrap_or(&edge.target);
    let mut targets = Targets::new();
    let mut errors = BTreeSet::new();
    // Resolve one concrete reviewed configuration; combine identical endpoints.
    let mask = if edge.configurations.is_empty() { resolver.all } else { edge.configurations };
    let mut remaining = mask;
    while let Some(index) = remaining.indices().next() {
        let mut stable = remaining;
        let resolved =
            resolver.lookup(&node, path, import, index, &mut BTreeSet::new(), &mut stable);
        remaining = remaining.difference(stable);
        match resolved {
            Resolution::InCrate(target) => {
                let active = targets.entry(target).or_default();
                if !edge.configurations.is_empty() {
                    *active = active.union(stable);
                }
            }
            Resolution::External => {}
            Resolution::Unresolved(reason, _) => {
                errors.insert(reason);
            }
        }
    }
    (targets, errors.into_iter().collect())
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

#[derive(Clone, Copy)]
enum Want {
    Module,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Resolution {
    InCrate(ResolvedTarget),
    External,
    Unresolved(String, Vec<ResolvedTarget>),
}

/// Only declared modules, items, named uses, and the existing outside globs.
/// No receiver typing, type projection, consumer profiles or namespace forks.
struct Scope {
    module: String,
    parent: Option<String>,
    items: BTreeSet<String>,
    children: BTreeMap<String, String>,
    bindings: BTreeMap<String, Vec<(String, ConfigSet)>>,
    globs: Vec<(String, ConfigSet)>,
}
struct Namespaces {
    scopes: BTreeMap<String, Scope>,
    all: ConfigSet,
}
impl Namespaces {
    fn new(modules: &BTreeMap<String, ModuleInfo>) -> Self {
        let mut scopes = BTreeMap::new();
        for (file, info) in modules {
            let mut names = BTreeSet::from([String::new()]);
            names.extend(info.analysis.scoped_items.iter().map(|(scope, _)| scope.clone()));
            names.extend(info.analysis.modules.iter().filter(|item| item.inline).map(|item| {
                if item.scope.is_empty() {
                    item.name.clone()
                } else {
                    format!("{}::{}", item.scope, item.name)
                }
            }));
            names.extend(info.analysis.block_scopes.keys().cloned());
            for name in names {
                let node = scoped_module(file, &name);
                let block = info.analysis.block_scopes.get(&name);
                let mut bindings = BTreeMap::<String, Vec<(String, ConfigSet)>>::new();
                for binding in info.analysis.bindings.iter().filter(|binding| binding.scope == name)
                {
                    bindings
                        .entry(binding.name.clone())
                        .or_default()
                        .push((binding.target.clone(), binding.configurations));
                }
                scopes.insert(
                    node.clone(),
                    Scope {
                        module: block.map_or_else(
                            || node.clone(),
                            |block| scoped_module(file, &block.module),
                        ),
                        parent: block.map(|block| scoped_module(file, &block.parent)),
                        items: info
                            .analysis
                            .scoped_items
                            .iter()
                            .filter(|(scope, _)| *scope == name)
                            .map(|(_, item)| item.clone())
                            .collect(),
                        children: BTreeMap::new(),
                        bindings,
                        globs: info
                            .analysis
                            .edges
                            .iter()
                            .filter(|edge| {
                                edge.target_scope == name
                                    && matches!(edge.kind, EdgeKind::Import | EdgeKind::Reexport)
                            })
                            .filter_map(|edge| {
                                edge.target
                                    .strip_suffix("::*")
                                    .map(|path| (path.to_string(), edge.configurations))
                            })
                            .collect(),
                    },
                );
            }
        }
        for (file, info) in modules {
            for item in &info.analysis.modules {
                let parent = scoped_module(file, &item.scope);
                let child = if item.inline {
                    format!("{parent}::{}", item.name)
                } else if file == "root" {
                    item.name.clone()
                } else {
                    format!("{file}::{}", item.name)
                };
                if let Some(scope) = scopes.get_mut(&parent) {
                    scope.children.insert(item.name.clone(), child);
                }
            }
        }
        let all = modules
            .values()
            .fold(ConfigSet::default(), |all, info| all.union(info.analysis.configurations));
        Self { scopes, all }
    }
    fn lookup(
        &self,
        node: &str,
        path: &str,
        import: bool,
        index: usize,
        seen: &mut BTreeSet<(String, String)>,
        stable: &mut ConfigSet,
    ) -> Resolution {
        if !seen.insert((node.to_string(), path.to_string())) {
            return Resolution::Unresolved(format!("recursive import {node}::{path}"), Vec::new());
        }
        let scope = match self.scopes.get(node) {
            Some(scope) => scope,
            None => return Resolution::External,
        };
        if let Some(rest) = path.strip_prefix("crate::") {
            let found = self.lookup("root", rest, import, index, seen, stable);
            let head = rest.split("::").next().unwrap_or(rest);
            let declared = self.scopes.get("root").is_some_and(|root| {
                root.bindings.contains_key(head)
                    || root.items.contains(head)
                    || root.children.contains_key(head)
            });
            return if matches!(found, Resolution::External) && !declared {
                Resolution::Unresolved(format!("root declares no {rest}"), Vec::new())
            } else {
                found
            };
        }
        if matches!(path, "crate" | "self" | "super") && import {
            let module = match path {
                "crate" => "root",
                "self" => scope.module.as_str(),
                _ => scope.module.rsplit_once("::").map_or("root", |(parent, _)| parent),
            };
            return Resolution::InCrate(ResolvedTarget {
                module: module.to_string(),
                item: "<module>".to_string(),
            });
        }
        if let Some(rest) = qualify_relative(&scope.module, path) {
            return self.lookup("root", &rest, import, index, seen, stable);
        }
        let (head, rest) = path.split_once("::").unwrap_or((path, ""));
        if let Some(bindings) = scope.bindings.get(head) {
            for (_, mask) in bindings {
                *stable = if mask.contains(index) {
                    stable.intersect(*mask)
                } else {
                    stable.difference(*mask)
                };
            }
            let active =
                bindings.iter().filter(|(_, mask)| mask.contains(index)).collect::<Vec<_>>();
            if active.len() > 1 {
                return Resolution::Unresolved(format!("ambiguous use {node}::{head}"), Vec::new());
            }
            if let Some((target, _)) = active.first() {
                let target =
                    if rest.is_empty() { (*target).clone() } else { format!("{target}::{rest}") };
                return self.lookup(node, &target, import, index, seen, stable);
            }
        }
        if let Some(child) = scope.children.get(head) {
            if rest.is_empty() {
                return if import {
                    Resolution::InCrate(ResolvedTarget {
                        module: child.clone(),
                        item: "<module>".to_string(),
                    })
                } else {
                    Resolution::External
                };
            }
            let found = self.lookup(child, rest, import, index, seen, stable);
            return if matches!(found, Resolution::External) {
                Resolution::Unresolved(format!("{child} declares no {rest}"), Vec::new())
            } else {
                found
            };
        }
        if scope.items.contains(head) {
            return Resolution::InCrate(ResolvedTarget {
                module: scope.module.clone(),
                item: path.to_string(),
            });
        }
        if let Some(parent) = &scope.parent {
            let found = self.lookup(parent, path, import, index, &mut seen.clone(), stable);
            if !matches!(found, Resolution::External) {
                return found;
            }
        }
        // Existing outside globs are conservative: a bare name resolves to every
        // matching imported item; ambiguity fails instead of hiding an edge.
        let mut found = BTreeSet::new();
        for (glob, mask) in &scope.globs {
            *stable = if mask.contains(index) {
                stable.intersect(*mask)
            } else {
                stable.difference(*mask)
            };
            if !mask.contains(index) {
                continue;
            }
            let mut branch = seen.clone();
            if let Resolution::InCrate(target) =
                self.lookup(node, glob, true, index, &mut branch, stable)
            {
                if target.item == "<module>" {
                    if let Resolution::InCrate(target) =
                        self.lookup(&target.module, path, import, index, &mut branch, stable)
                    {
                        found.insert(target);
                    }
                }
            }
        }
        if found.len() == 1 {
            return Resolution::InCrate(found.into_iter().next().unwrap());
        }
        if found.len() > 1 {
            return Resolution::Unresolved(
                format!("ambiguous outside glob {node}::{path}"),
                Vec::new(),
            );
        }
        Resolution::External
    }
    fn resolve(&self, node: &str, path: &str, want: Want) -> Resolution {
        self.all
            .indices()
            .find_map(|index| {
                let mut stable = self.all;
                let found = self.lookup(
                    node,
                    path,
                    matches!(want, Want::Module),
                    index,
                    &mut BTreeSet::new(),
                    &mut stable,
                );
                (!matches!(found, Resolution::External)).then_some(found)
            })
            .unwrap_or(Resolution::External)
    }
    fn bindings_of(&self, node: &str) -> impl Iterator<Item = (&String, &String)> {
        self.scopes
            .get(node)
            .into_iter()
            .flat_map(|scope| scope.bindings.iter())
            .flat_map(|(name, targets)| targets.iter().map(move |(target, _)| (name, target)))
    }
    fn has_binding(&self, node: &str, name: &str) -> bool {
        self.scopes.get(node).is_some_and(|scope| scope.bindings.contains_key(name))
    }
    fn imported_at_root(&self, name: &str) -> bool {
        self.has_binding("root", name)
    }
    fn binds(&self, node: &str, name: &str) -> bool {
        self.has_binding(node, name)
    }
}

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

/// The crate names a path in the engine may start with to leave the crate:
/// the standard library crates and every dependency the manifest declares,
/// in any dependency table.
fn manifest_crate_names(manifest: &str) -> BTreeSet<String> {
    let mut names = ["std", "core", "alloc", "proc_macro", "test"]
        .map(String::from)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut in_dependencies = false;
    // Open `{`/`[` of a multi-line value, whose lines carry no crate key.
    let mut depth = 0_i32;
    for line in manifest.lines().map(str::trim) {
        if depth == 0 {
            if let Some(header) = line.strip_prefix('[') {
                let header = header.trim_start_matches('[').trim_end_matches(']');
                in_dependencies = header.ends_with("dependencies");
                if let Some((_, name)) =
                    header.rsplit_once('.').filter(|(parent, _)| parent.ends_with("dependencies"))
                {
                    names.insert(name.trim_matches('"').replace('-', "_"));
                }
                continue;
            }
            if in_dependencies && !line.is_empty() && !line.starts_with('#') {
                let key = line.split(['=', '.']).next().unwrap_or(line).trim().trim_matches('"');
                if !key.is_empty() {
                    names.insert(key.replace('-', "_"));
                }
            }
        }
        let value = line.split_once('=').map_or(line, |(_, value)| value);
        let value = if depth == 0 { value } else { line };
        for character in value.chars() {
            match character {
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => {}
            }
        }
        depth = depth.max(0);
    }
    names
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
    fn manifest_crate_names_cover_every_dependency_table() {
        let names = manifest_crate_names(
            "[package]\nname = \"fathomdb-engine\"\n[dependencies]\nfathomdb-schema.workspace = true\n\
             rusqlite = { version = \"0.40\",\n    features = [\"bundled\"] }\n[features]\nslice = []\n\
             [target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n[dev-dependencies]\ntempfile = \"3\"\n\
             [dependencies.sha2]\nversion = \"0.11\"\n",
        );
        for name in
            ["std", "core", "alloc", "fathomdb_schema", "rusqlite", "libc", "tempfile", "sha2"]
        {
            assert!(names.contains(name), "{name} missing from {names:?}");
        }
        for name in ["fathomdb_engine", "slice", "version", "name", "features"] {
            assert!(!names.contains(name), "{name} is not a dependency: {names:?}");
        }
    }

    fn crate_of(files: &[(&str, &str)]) -> BTreeMap<String, ModuleInfo> {
        let space = ConfigSpace::from_manifest("[features]\ndefault = []\n", &[])
            .expect("empty configuration space");
        files
            .iter()
            .map(|(module, source)| {
                let analysis = analyze_source(source, &space).expect("fixture parses");
                ((*module).to_string(), ModuleInfo { file: PathBuf::from(module), analysis })
            })
            .collect()
    }

    #[test]
    fn anchored_missing_root_items_fail_closed() {
        let modules = crate_of(&[("root", "fn helper() {}")]);
        let resolver = Namespaces::new(&modules);
        assert!(matches!(
            resolver.resolve("root", "crate::missing", Want::Module),
            Resolution::Unresolved(..)
        ));
    }

    #[test]
    fn compact_resolver_keeps_named_imports_helpers_and_lexical_aliases() {
        let modules = crate_of(&[
            ("root", "mod search; mod reader_pool; use reader_pool::dispatch as start; fn helper() { start(); }"),
            ("reader_pool", "pub(crate) fn dispatch() {}"),
            ("search", "fn run() { use crate::helper as local; local(); }"),
        ]);
        let resolver = Namespaces::new(&modules);
        assert!(
            matches!(resolver.resolve("root", "start", Want::Module), Resolution::InCrate(target) if target.module == "reader_pool" && target.item == "dispatch")
        );
        assert!(
            matches!(resolver.resolve("search", "crate::helper", Want::Module), Resolution::InCrate(target) if target.module == "root" && target.item == "helper")
        );
    }

    #[test]
    fn direct_reader_request_variants_are_distinct_from_factories() {
        assert!(direct_reader_request_variant("crate::reader_pool::ReaderRequest::Search"));
        assert!(!direct_reader_request_variant("crate::reader_pool::ReaderRequest::search"));
    }
}
