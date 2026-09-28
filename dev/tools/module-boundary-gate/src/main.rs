use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fathomdb_module_boundary_gate::{
    analyze_source, Analysis, EdgeKind, InherentMethod, CONFIGURATIONS,
};

#[derive(Debug, Default)]
struct Policy {
    classified: BTreeMap<String, String>,
    field_owners: BTreeMap<String, String>,
    owners: BTreeMap<String, String>,
    forbidden_dependencies: BTreeSet<(String, String)>,
    forbidden_cycles: BTreeSet<(String, String)>,
    allowed_cycles: BTreeSet<(String, String)>,
    inherent_methods: BTreeSet<(String, String)>,
    expected_edges: BTreeSet<(String, String, String)>,
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
    let modules = discover_modules(&source_root)?;
    let policy = parse_policy(&policy_path)?;
    let result = evaluate(&source_root, &modules, &policy, report_only);
    if report_only {
        return result;
    }
    result?;
    println!(
        "ok    module-boundary: {} modules, {} governed, configurations=default,test-hooks,tc5-benchmark,cfg(test)",
        modules.len(),
        policy.classified.values().filter(|kind| kind.as_str() == "governed").count()
    );
    Ok(())
}

fn discover_modules(source_root: &Path) -> Result<BTreeMap<String, ModuleInfo>, Vec<String>> {
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
        match analyze_source(&source) {
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
            ["inherent", module, method] => {
                if !policy.inherent_methods.insert(((*module).to_string(), (*method).to_string())) {
                    errors.push(format!(
                        "policy:{} duplicates inherent method {module} {method}",
                        index + 1
                    ));
                }
            }
            ["edge", source, target, kind] => {
                if !policy.expected_edges.insert((
                    (*source).to_string(),
                    (*target).to_string(),
                    (*kind).to_string(),
                )) {
                    errors.push(format!(
                        "policy:{} duplicates edge {source} {target} {kind}",
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
    report_only: bool,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let discovered = modules.keys().cloned().collect::<BTreeSet<_>>();
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
    let mut edge_origins: BTreeMap<(String, String, String), (String, usize, usize, String)> =
        BTreeMap::new();
    let mut adjacency_by_configuration = CONFIGURATIONS
        .into_iter()
        .map(|configuration| (configuration.to_string(), BTreeMap::new()))
        .collect::<BTreeMap<String, BTreeMap<String, BTreeSet<String>>>>();
    for (module, info) in modules {
        let governed = policy.classified.get(module).map(String::as_str) == Some("governed");
        let participates = module != "root"
            && matches!(
                policy.classified.get(module).map(String::as_str),
                Some("governed" | "admitted")
            );
        for predicate in &info.analysis.unsupported_cfg {
            errors.push(format!(
                "unsupported cfg predicate in {}: {predicate}",
                relative(source_root, &info.file)
            ));
        }
        if governed {
            for edge in &info.analysis.edges {
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
        }
        for edge in &info.analysis.edges {
            if governed
                && matches!(edge.kind, EdgeKind::Import | EdgeKind::TypeOrComposition)
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
            let targets = edge_targets(
                module,
                edge,
                modules,
                root_aliases,
                &engine_fields,
                &policy.field_owners,
                &method_owners,
            );
            for target in targets {
                if target == *module || !modules.contains_key(&target) {
                    continue;
                }
                if participates && edge.kind != EdgeKind::TypeOrComposition {
                    for configuration in &edge.configurations {
                        adjacency_by_configuration
                            .get_mut(configuration)
                            .expect("known configuration")
                            .entry(module.clone())
                            .or_default()
                            .insert(target.clone());
                    }
                }
                if governed {
                    let record = (module.clone(), target, edge.kind.as_str().to_string());
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
    }

    for (configuration, adjacency) in &adjacency_by_configuration {
        if configuration == "cfg(test)" {
            continue;
        }
        for (source, target) in &policy.forbidden_dependencies {
            if adjacency.get(source).is_some_and(|targets| targets.contains(target)) {
                errors.push(format!(
                    "forbidden dependency {source} -> {target} configuration={configuration}"
                ));
            }
        }
        for (left, right) in &policy.forbidden_cycles {
            if reachable(adjacency, left, right) && reachable(adjacency, right, left) {
                errors.push(format!(
                    "forbidden cycle {left} <-> {right} configuration={configuration}"
                ));
            }
        }
        for component in strongly_connected(adjacency, &discovered) {
            if component.len() < 2 {
                continue;
            }
            let governed = component
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
        for module in modules.keys() {
            println!("module\t{module}");
        }
        for (parent, info) in modules {
            for declaration in info.analysis.modules.iter().filter(|item| item.inline) {
                println!(
                    "inline-module\t{}::{}\t{}:{}:{}",
                    parent,
                    declaration.name,
                    relative(source_root, &info.file),
                    declaration.location.line,
                    declaration.location.column
                );
            }
        }
        for field in &engine_fields {
            println!("field\t{field}");
        }
        for (module, method) in &actual_inherent {
            println!("inherent\t{module}\t{method}");
        }
        for (source, target, kind) in &actual_edges {
            println!("edge\t{source}\t{target}\t{kind}");
        }
        for (label, adjacency) in &adjacency_by_configuration {
            let edge_count = adjacency.values().map(BTreeSet::len).sum::<usize>();
            println!("configuration\t{label}\texecutable_edges={edge_count}");
        }
    } else {
        if policy.expected_edges.is_empty() {
            errors.push("policy expected-edge set is empty".to_string());
        }
        for missing in actual_edges.difference(&policy.expected_edges) {
            let origin = edge_origins.get(missing).expect("actual edge has an origin");
            errors.push(format!(
                "unexpected boundary edge source={} destination={} item={} kind={} configuration=union(default,test-hooks,tc5-benchmark,cfg(test)) at {}:{}:{}",
                missing.0, missing.1, origin.3, missing.2, origin.0, origin.1, origin.2
            ));
        }
        for stale in policy.expected_edges.difference(&actual_edges) {
            errors.push(format!(
                "stale boundary edge source={} destination={} kind={}",
                stale.0, stale.1, stale.2
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
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

#[allow(clippy::too_many_arguments)]
fn edge_targets(
    module: &str,
    edge: &fathomdb_module_boundary_gate::Edge,
    modules: &BTreeMap<String, ModuleInfo>,
    root_aliases: Option<&BTreeMap<String, String>>,
    engine_fields: &BTreeSet<String>,
    field_owners: &BTreeMap<String, String>,
    method_owners: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    if edge.kind == EdgeKind::FieldAccess {
        return if engine_fields.contains(&edge.target) {
            field_owners.get(&edge.target).cloned().into_iter().collect()
        } else {
            BTreeSet::new()
        };
    }
    if edge.kind == EdgeKind::EngineMethod {
        return method_owners.get(&edge.target).cloned().unwrap_or_default();
    }
    let mut target = edge.target.as_str();
    if let Some(rest) = target.strip_prefix("crate::") {
        target = rest;
    } else if let Some(rest) = target.strip_prefix("super::") {
        let parent = module.rsplit_once("::").map_or("root", |(parent, _)| parent);
        let qualified =
            if parent == "root" { rest.to_string() } else { format!("{parent}::{rest}") };
        return longest_module_prefix(&qualified, modules).into_iter().collect();
    } else if let Some(rest) = target.strip_prefix("self::") {
        let qualified =
            if module == "root" { rest.to_string() } else { format!("{module}::{rest}") };
        return longest_module_prefix(&qualified, modules).into_iter().collect();
    }
    if let Some(owner) = longest_module_prefix(target, modules) {
        return BTreeSet::from([owner]);
    }
    let first = target.split("::").next().unwrap_or(target);
    if let Some(root_target) = root_aliases.and_then(|aliases| aliases.get(first)) {
        let root_target = root_target.strip_prefix("crate::").unwrap_or(root_target);
        if let Some(owner) = longest_module_prefix(root_target, modules) {
            return BTreeSet::from([owner]);
        }
    }
    BTreeSet::new()
}

fn longest_module_prefix(target: &str, modules: &BTreeMap<String, ModuleInfo>) -> Option<String> {
    let segments = target.split("::").collect::<Vec<_>>();
    (1..=segments.len())
        .rev()
        .map(|end| segments[..end].join("::"))
        .find(|candidate| modules.contains_key(candidate))
}

fn reachable(adjacency: &BTreeMap<String, BTreeSet<String>>, source: &str, target: &str) -> bool {
    let mut pending = VecDeque::from([source.to_string()]);
    let mut visited = BTreeSet::new();
    while let Some(current) = pending.pop_front() {
        if current == target {
            return true;
        }
        if !visited.insert(current.clone()) {
            continue;
        }
        if let Some(next) = adjacency.get(&current) {
            pending.extend(next.iter().cloned());
        }
    }
    false
}

fn strongly_connected(
    adjacency: &BTreeMap<String, BTreeSet<String>>,
    nodes: &BTreeSet<String>,
) -> Vec<BTreeSet<String>> {
    let mut unassigned = nodes.clone();
    let mut components = Vec::new();
    while let Some(seed) = unassigned.iter().next().cloned() {
        let component = unassigned
            .iter()
            .filter(|candidate| {
                reachable(adjacency, &seed, candidate) && reachable(adjacency, candidate, &seed)
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        for node in &component {
            unassigned.remove(node);
        }
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
