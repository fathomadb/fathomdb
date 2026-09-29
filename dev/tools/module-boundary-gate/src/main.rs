use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fathomdb_module_boundary_gate::{
    analyze_source, Analysis, ConfigSpace, EdgeKind, InherentMethod,
};

#[derive(Debug, Default)]
struct Policy {
    classified: BTreeMap<String, String>,
    field_owners: BTreeMap<String, String>,
    owners: BTreeMap<String, String>,
    forbidden_dependencies: BTreeSet<(String, String)>,
    forbidden_cycles: BTreeSet<(String, String)>,
    allowed_cycles: BTreeSet<(String, String)>,
    allowed_local_macros: BTreeSet<(String, String, String)>,
    allowed_unparsed_macros: BTreeSet<(String, String, String, String)>,
    inherent_methods: BTreeSet<(String, String)>,
    expected_edges: BTreeSet<(String, String, String, String, String, String)>,
    configuration_features: Vec<(String, String)>,
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
    let space = ConfigSpace::from_manifest(&manifest, &policy.configuration_features)?;
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
         debug_assertions x {{{} combinations of {}; {} single-feature closures; all-features}})",
        module_inventory(&modules).len(),
        policy.classified.values().filter(|kind| kind.as_str() == "governed").count(),
        space.len(),
        1usize << space.axes.len(),
        space.axes.iter().map(|(feature, _)| feature.as_str()).collect::<Vec<_>>().join(", "),
        space.profiles.len().saturating_sub(2)
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

    let mut method_owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (module, info) in modules {
        for method in &info.analysis.engine_methods {
            method_owners.entry(method.clone()).or_default().insert(module.clone());
        }
    }
    if method_owners.is_empty() {
        errors.push("source-derived Engine method map is empty".to_string());
    }
    for (method, owners) in &method_owners {
        if owners.len() > 1 {
            errors
                .push(format!("Engine method {method} has multiple defining modules: {owners:?}"));
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
    let mut actual_edges = BTreeSet::new();
    let mut actual_local_macros = BTreeSet::new();
    let mut actual_unparsed_macros = BTreeSet::new();
    let mut edge_origins: BTreeMap<
        (String, String, String, String, String, String),
        (String, usize, usize, String),
    > = BTreeMap::new();
    let mut adjacency_by_configuration =
        vec![BTreeMap::<String, BTreeSet<String>>::new(); space.len()];
    let mut dependencies_by_configuration =
        vec![BTreeMap::<String, BTreeSet<String>>::new(); space.len()];
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
                && root_reexport_indirection(&edge.target, root_aliases)
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
            if governed && edge.kind == EdgeKind::Callable {
                if let Some(method) =
                    edge.target.strip_prefix(".<").and_then(|value| value.strip_suffix('>'))
                {
                    if governed_callable_names.contains(method) {
                        errors.push(format!(
                            "unresolved governed method receiver at {}:{}:{} source={} source_item={} method={method}",
                            relative(source_root, &info.file),
                            edge.location.line,
                            edge.location.column,
                            source_module,
                            edge.source_item
                        ));
                    }
                } else if !edge.target.contains("::")
                    && info.analysis.local_functions.contains_key(&edge.target)
                    && declared_owners.get(&edge.target).is_some_and(|owners| {
                        owners.iter().any(|owner| {
                            owner != module
                                && policy.classified.get(owner).map(String::as_str)
                                    == Some("governed")
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
            }
            let targets = edge_targets(
                &source_module,
                edge,
                modules,
                root_aliases,
                &engine_fields,
                &policy.field_owners,
                &method_owners,
            );
            for target in targets {
                let nonroot_local = target.module == source_module && source_module != "root";
                for configuration in edge.configurations.indices() {
                    dependencies_by_configuration[configuration]
                        .entry(source_module.clone())
                        .or_default()
                        .insert(target.module.clone());
                    if matches!(
                        edge.kind,
                        EdgeKind::Callable | EdgeKind::FieldAccess | EdgeKind::EngineMethod
                    ) {
                        adjacency_by_configuration[configuration]
                            .entry(graph_node(&source_module, &edge.source_item))
                            .or_default()
                            .insert(graph_node(&target.module, &target.item));
                    }
                }
                if nonroot_local {
                    continue;
                }
                if frozen_scope(policy, &source_module) || frozen_scope(policy, &target.module) {
                    let record = (
                        source_module.clone(),
                        edge.source_item.clone(),
                        target.module,
                        target.item,
                        edge.kind.as_str().to_string(),
                        space.expression(edge.configurations),
                    );
                    actual_edges.insert(record.clone());
                    edge_origins.entry(record).or_insert_with(|| {
                        (
                            relative(source_root, &info.file),
                            edge.location.line,
                            edge.location.column,
                            edge.target.clone(),
                        )
                    });
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

    for (module, name, _) in policy.allowed_local_macros.difference(&actual_local_macros) {
        errors.push(format!("stale local macro policy source={module} macro={name}"));
    }
    let governed_unparsed = actual_unparsed_macros
        .iter()
        .filter(|(governed, ..)| *governed)
        .map(|(_, module, item, name, fingerprint)| {
            (module.clone(), item.clone(), name.clone(), fingerprint.clone())
        })
        .collect::<BTreeSet<_>>();
    for (module, item, name, fingerprint) in
        policy.allowed_unparsed_macros.difference(&governed_unparsed)
    {
        errors.push(format!(
            "stale unparsed macro policy source={module} item={item} macro={name} fingerprint={fingerprint}"
        ));
    }

    for (index, adjacency) in adjacency_by_configuration.iter().enumerate() {
        let configuration = &space.configurations[index].label;
        let dependencies = &dependencies_by_configuration[index];
        for (source, target) in &policy.forbidden_dependencies {
            if dependencies.get(source).is_some_and(|targets| targets.contains(target)) {
                errors.push(format!(
                    "forbidden dependency {source} -> {target} configuration={configuration}"
                ));
            }
        }
        let graph_nodes =
            adjacency.keys().chain(adjacency.values().flatten()).cloned().collect::<BTreeSet<_>>();
        for component in strongly_connected(adjacency, &graph_nodes) {
            if component.len() < 2 {
                continue;
            }
            let component_modules =
                component.iter().map(|node| node_module(node)).collect::<BTreeSet<_>>();
            for (left, right) in &policy.forbidden_cycles {
                if component_modules.contains(left) && component_modules.contains(right) {
                    errors.push(format!(
                        "forbidden cycle {left} <-> {right} configuration={configuration} in SCC {:?}",
                        component
                    ));
                }
            }
            let governed = component_modules
                .iter()
                .filter(|module| {
                    policy.classified.get(*module).map(String::as_str) == Some("governed")
                })
                .cloned()
                .collect::<Vec<_>>();
            for left_index in 0..governed.len() {
                for right in governed.iter().skip(left_index + 1) {
                    let pair = sorted_pair(&governed[left_index], right);
                    if !policy.allowed_cycles.contains(&pair) {
                        errors.push(format!(
                            "unapproved governed cycle {} <-> {} configuration={} in SCC {:?}",
                            pair.0, pair.1, configuration, component
                        ));
                    }
                }
            }
        }
    }

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
        for (governed, module, item, name, fingerprint) in &actual_unparsed_macros {
            let label = if *governed { "unparsed-macro" } else { "reported-unparsed-macro" };
            println!("{label}\t{module}\t{item}\t{name}\t{fingerprint}");
        }
        for (index, adjacency) in adjacency_by_configuration.iter().enumerate() {
            let label = &space.configurations[index].label;
            let edge_count = adjacency.values().map(BTreeSet::len).sum::<usize>();
            println!("configuration\t{label}\texecutable_edges={edge_count}");
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
    target: &str,
    root_aliases: Option<&BTreeMap<String, String>>,
) -> bool {
    let Some(rest) = target.strip_prefix("crate::") else {
        return false;
    };
    let first = rest.split("::").next().unwrap_or(rest);
    root_aliases.is_some_and(|aliases| aliases.contains_key(first))
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ResolvedTarget {
    module: String,
    item: String,
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

type UnparsedMacro = (bool, String, String, String, String);

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
        let governed = policy.classified.get(&module).map(String::as_str) == Some("governed");
        let fingerprint = usage.fingerprint.clone().expect("unparsed macro fingerprint");
        actual_unparsed_macros.insert((
            governed,
            module.clone(),
            usage.source_item.clone(),
            usage.target.clone(),
            fingerprint.clone(),
        ));
        if !governed {
            continue;
        }
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
    method_owners: &BTreeMap<String, BTreeSet<String>>,
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
    targets.into_iter().map(|target| chase_reexports(target, modules)).collect()
}

/// Follows `use` aliases (at any visibility, since descendants may name a
/// parent's private import) and single-candidate glob re-exports from the
/// module a path resolved to, until the path reaches its defining module.
/// Chains that leave the crate stop at the last in-crate hop.
fn chase_reexports(
    start: ResolvedTarget,
    modules: &BTreeMap<String, ModuleInfo>,
) -> ResolvedTarget {
    let mut current = start;
    let mut seen = BTreeSet::new();
    while seen.insert(current.clone()) {
        if current.item == "<module>" {
            break;
        }
        let (head, rest) = current
            .item
            .split_once("::")
            .map_or((current.item.as_str(), None), |(head, rest)| (head, Some(rest)));
        let Some(info) = modules.get(&current.module) else { break };
        let next = if let Some(alias) = info.analysis.import_aliases.get(head) {
            qualify_alias(&current.module, alias, modules)
        } else if info.analysis.declared_items.contains(head) {
            None
        } else {
            let candidates = info
                .analysis
                .edges
                .iter()
                .filter(|edge| {
                    matches!(edge.kind, EdgeKind::Import | EdgeKind::Reexport)
                        && edge.source_scope.is_empty()
                        && edge.target.ends_with("::*")
                })
                .filter_map(|glob| {
                    let namespace = qualify_glob_namespace(
                        &current.module,
                        glob.target.trim_end_matches("::*"),
                    );
                    modules
                        .get(&namespace)
                        .filter(|owner| {
                            owner.analysis.declared_items.contains(head)
                                || owner.analysis.import_aliases.contains_key(head)
                        })
                        .map(|_| format!("{namespace}::{head}"))
                })
                .collect::<BTreeSet<_>>();
            if candidates.len() == 1 {
                candidates.into_iter().next()
            } else {
                None
            }
        };
        let Some(path) = next else { break };
        let full = rest.map_or_else(|| path.clone(), |rest| format!("{path}::{rest}"));
        let Some(resolved) = resolve_path(&full, modules) else { break };
        current = resolved;
    }
    current
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
    method_owners: &BTreeMap<String, BTreeSet<String>>,
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
            .flatten()
            .map(|owner| ResolvedTarget { module: owner.clone(), item: edge.target.clone() })
            .collect();
    }
    let mut target = edge.target.as_str();
    let crate_qualified = target.starts_with("crate::");
    if let Some(rest) = target.strip_prefix("crate::") {
        target = rest;
    } else if let Some(rest) = target.strip_prefix("super::") {
        let parent = module.rsplit_once("::").map_or("root", |(parent, _)| parent);
        let qualified =
            if parent == "root" { rest.to_string() } else { format!("{parent}::{rest}") };
        return resolve_path(&qualified, modules).into_iter().collect();
    } else if let Some(rest) = target.strip_prefix("self::") {
        let qualified =
            if module == "root" { rest.to_string() } else { format!("{module}::{rest}") };
        return resolve_path(&qualified, modules).into_iter().collect();
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
    BTreeSet::new()
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
