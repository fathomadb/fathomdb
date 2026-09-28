//! Syntax-bounded module dependency analysis for `fathomdb-engine`.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Span;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprField, ExprMethodCall, ExprPath, ImplItem, ImplItemFn, ItemEnum, ItemFn,
    ItemImpl, ItemMod, ItemStruct, ItemTrait, ItemType, ItemUse, Local, Member, Meta, Pat, Type,
    Visibility,
};

pub const CONFIGURATIONS: [&str; 4] = ["default", "test-hooks", "tc5-benchmark", "cfg(test)"];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EdgeKind {
    Import,
    Callable,
    FieldAccess,
    EngineMethod,
    TypeOrComposition,
}

impl EdgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Callable => "callable",
            Self::FieldAccess => "field-access",
            Self::EngineMethod => "engine-method",
            Self::TypeOrComposition => "type-or-composition",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl From<Span> for Location {
    fn from(span: Span) -> Self {
        let start = span.start();
        Self { line: start.line, column: start.column }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Edge {
    pub kind: EdgeKind,
    pub target: String,
    pub location: Location,
    pub configurations: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleDecl {
    pub name: String,
    pub inline: bool,
    pub depth: usize,
    pub location: Location,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct InherentMethod {
    pub owner: String,
    pub method: String,
    pub location: Location,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Analysis {
    pub modules: BTreeSet<ModuleDecl>,
    pub edges: BTreeSet<Edge>,
    pub globs: BTreeSet<Location>,
    pub engine_fields: BTreeSet<String>,
    pub engine_methods: BTreeSet<String>,
    pub inherent_methods: BTreeSet<InherentMethod>,
    pub declared_items: BTreeSet<String>,
    pub local_functions: BTreeMap<String, Location>,
    pub import_aliases: BTreeMap<String, String>,
    pub unsupported_cfg: BTreeSet<String>,
}

pub fn analyze_source(source: &str) -> Result<Analysis, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut visitor = Analyzer::default();
    visitor.visit_file(&file);
    visitor.resolve_aliases();
    Ok(visitor.analysis)
}

struct Analyzer {
    analysis: Analysis,
    engine_aliases: BTreeSet<String>,
    impl_owners: Vec<String>,
    configurations: BTreeSet<String>,
    module_depth: usize,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self {
            analysis: Analysis::default(),
            engine_aliases: BTreeSet::from([
                "self".to_string(),
                "engine".to_string(),
                "opened".to_string(),
            ]),
            impl_owners: Vec::new(),
            configurations: CONFIGURATIONS.into_iter().map(str::to_string).collect(),
            module_depth: 0,
        }
    }
}

impl Analyzer {
    fn edge(&mut self, kind: EdgeKind, target: impl Into<String>, span: Span) {
        self.analysis.edges.insert(Edge {
            kind,
            target: target.into(),
            location: span.into(),
            configurations: self.configurations.clone(),
        });
    }

    fn declared(&mut self, name: impl Into<String>) {
        self.analysis.declared_items.insert(name.into());
    }

    fn enter_attrs(&mut self, attrs: &[Attribute]) -> BTreeSet<String> {
        let previous = self.configurations.clone();
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("cfg")) {
            let Meta::List(list) = &attr.meta else {
                self.analysis.unsupported_cfg.insert("malformed cfg attribute".to_string());
                self.configurations.clear();
                continue;
            };
            let predicate = list.tokens.to_string();
            let mut unsupported = false;
            self.configurations = self
                .configurations
                .iter()
                .filter(|configuration| match evaluate_cfg(&predicate, configuration) {
                    Some(active) => active,
                    None => {
                        unsupported = true;
                        false
                    }
                })
                .cloned()
                .collect();
            if unsupported {
                self.analysis.unsupported_cfg.insert(predicate);
            }
        }
        previous
    }

    fn resolve_aliases(&mut self) {
        let aliases = self.analysis.import_aliases.clone();
        let mut resolved = BTreeSet::new();
        for edge in &self.analysis.edges {
            let (first, tail) = edge
                .target
                .split_once("::")
                .map_or((edge.target.as_str(), None), |(head, rest)| (head, Some(rest)));
            if let Some(owner) = aliases.get(first) {
                let target = tail.map_or_else(|| owner.clone(), |rest| format!("{owner}::{rest}"));
                resolved.insert(Edge {
                    kind: edge.kind.clone(),
                    target,
                    location: edge.location,
                    configurations: edge.configurations.clone(),
                });
            } else {
                resolved.insert(edge.clone());
            }
        }
        self.analysis.edges = resolved;
    }
}

fn evaluate_cfg(predicate: &str, configuration: &str) -> Option<bool> {
    let predicate = predicate.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();
    evaluate_cfg_compact(&predicate, configuration)
}

fn evaluate_cfg_compact(predicate: &str, configuration: &str) -> Option<bool> {
    if let Some(inner) = predicate.strip_prefix("any(").and_then(|value| value.strip_suffix(')')) {
        return split_cfg_arguments(inner)
            .into_iter()
            .map(|argument| evaluate_cfg_compact(argument, configuration))
            .try_fold(false, |active, value| value.map(|value| active || value));
    }
    if let Some(inner) = predicate.strip_prefix("all(").and_then(|value| value.strip_suffix(')')) {
        return split_cfg_arguments(inner)
            .into_iter()
            .map(|argument| evaluate_cfg_compact(argument, configuration))
            .try_fold(true, |active, value| value.map(|value| active && value));
    }
    if let Some(inner) = predicate.strip_prefix("not(").and_then(|value| value.strip_suffix(')')) {
        return evaluate_cfg_compact(inner, configuration).map(|active| !active);
    }
    match predicate {
        "test" => Some(configuration == "cfg(test)"),
        "debug_assertions" | "unix" | "target_os=\"linux\"" => Some(true),
        _ => predicate.strip_prefix("feature=\"").and_then(|value| value.strip_suffix('"')).map(
            |feature| match feature {
                "test-hooks" => configuration == "test-hooks",
                "tc5-benchmark" => configuration == "tc5-benchmark",
                _ => false,
            },
        ),
    }
}

fn split_cfg_arguments(source: &str) -> Vec<&str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut arguments = Vec::new();
    for (index, ch) in source.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                arguments.push(&source[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    arguments.push(&source[start..]);
    arguments
}

impl<'ast> Visit<'ast> for Analyzer {
    fn visit_local(&mut self, local: &'ast Local) {
        if let (Some(binding), Some(init)) = (pattern_ident(&local.pat), &local.init) {
            if expression_base_ident(&init.expr)
                .is_some_and(|ident| self.engine_aliases.contains(&ident))
                || pattern_type(&local.pat).is_some_and(type_is_engine)
            {
                self.engine_aliases.insert(binding.ident.to_string());
            }
        }
        visit::visit_local(self, local);
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        let previous = self.enter_attrs(&item.attrs);
        self.analysis.modules.insert(ModuleDecl {
            name: item.ident.to_string(),
            inline: item.content.is_some(),
            depth: self.module_depth,
            location: item.ident.span().into(),
        });
        self.declared(item.ident.to_string());
        if item.content.is_some() {
            self.module_depth += 1;
        }
        visit::visit_item_mod(self, item);
        if item.content.is_some() {
            self.module_depth -= 1;
        }
        self.configurations = previous;
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let previous = self.enter_attrs(&item.attrs);
        let kind = if visibility_exceeds_module(&item.vis) {
            EdgeKind::TypeOrComposition
        } else {
            EdgeKind::Import
        };
        let mut nested_aliases = BTreeMap::new();
        let aliases = if self.module_depth == 0 {
            &mut self.analysis.import_aliases
        } else {
            &mut nested_aliases
        };
        collect_use_tree(
            &item.tree,
            &mut Vec::new(),
            kind,
            &mut self.analysis.edges,
            &mut self.analysis.globs,
            aliases,
            &self.configurations,
        );
        visit::visit_item_use(self, item);
        self.configurations = previous;
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        self.declared(item.ident.to_string());
        if item.ident == "Engine" {
            for field in &item.fields {
                if let Some(ident) = &field.ident {
                    self.analysis.engine_fields.insert(ident.to_string());
                }
            }
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        self.declared(item.ident.to_string());
        visit::visit_item_enum(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast ItemTrait) {
        self.declared(item.ident.to_string());
        visit::visit_item_trait(self, item);
    }

    fn visit_item_type(&mut self, item: &'ast ItemType) {
        self.declared(item.ident.to_string());
        visit::visit_item_type(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let previous = self.enter_attrs(&item.attrs);
        let name = item.sig.ident.to_string();
        self.declared(name.clone());
        self.analysis.local_functions.insert(name, item.sig.ident.span().into());
        visit::visit_item_fn(self, item);
        self.configurations = previous;
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        let previous = self.enter_attrs(&item.attrs);
        let owner = type_last_ident(&item.self_ty).map(ToString::to_string);
        if owner.as_deref() == Some("Engine") {
            for member in &item.items {
                if let ImplItem::Fn(method) = member {
                    self.analysis.engine_methods.insert(method.sig.ident.to_string());
                }
            }
        } else if item.trait_.is_none() {
            if let Some(owner) = &owner {
                for member in &item.items {
                    if let ImplItem::Fn(method) = member {
                        if visibility_exceeds_module(&method.vis) {
                            self.analysis.inherent_methods.insert(InherentMethod {
                                owner: owner.clone(),
                                method: method.sig.ident.to_string(),
                                location: method.sig.ident.span().into(),
                            });
                        }
                    }
                }
            }
        }
        if let Some(owner) = &owner {
            self.impl_owners.push(owner.clone());
        }
        visit::visit_item_impl(self, item);
        if owner.is_some() {
            self.impl_owners.pop();
        }
        self.configurations = previous;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        let previous = self.enter_attrs(&item.attrs);
        visit::visit_impl_item_fn(self, item);
        self.configurations = previous;
    }

    fn visit_expr_field(&mut self, expression: &'ast ExprField) {
        let Member::Named(field) = &expression.member else {
            visit::visit_expr_field(self, expression);
            return;
        };
        // Resolution against the source-derived Engine field set happens after
        // all modules are parsed. Recording every named access makes an
        // unknown receiver conservative without manufacturing an edge for
        // fields that do not belong to Engine.
        self.edge(EdgeKind::FieldAccess, field.to_string(), field.span());
        visit::visit_expr_field(self, expression);
    }

    fn visit_expr_method_call(&mut self, expression: &'ast ExprMethodCall) {
        if expression_base_ident(&expression.receiver)
            .is_some_and(|ident| self.engine_aliases.contains(&ident))
        {
            self.edge(
                EdgeKind::EngineMethod,
                expression.method.to_string(),
                expression.method.span(),
            );
        } else {
            self.edge(
                EdgeKind::Callable,
                format!(".<{}>", expression.method),
                expression.method.span(),
            );
        }
        visit::visit_expr_method_call(self, expression);
    }

    fn visit_expr_path(&mut self, expression: &'ast ExprPath) {
        if expression.qself.is_none() {
            let segments = expression
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            if segments.len() >= 2 {
                if segments.first().is_some_and(|segment| segment == "Engine")
                    || (segments.first().is_some_and(|segment| segment == "Self")
                        && self.impl_owners.last().is_some_and(|owner| owner == "Engine"))
                {
                    if let Some(method) = segments.last() {
                        self.edge(
                            EdgeKind::EngineMethod,
                            method.clone(),
                            expression.path.segments[0].ident.span(),
                        );
                    }
                } else {
                    self.edge(
                        EdgeKind::Callable,
                        segments.join("::"),
                        expression.path.segments[0].ident.span(),
                    );
                }
            } else if let Some(name) = segments.first() {
                self.edge(
                    EdgeKind::Callable,
                    name.clone(),
                    expression.path.segments[0].ident.span(),
                );
            }
        }
        visit::visit_expr_path(self, expression);
    }
}

fn pattern_ident(pattern: &Pat) -> Option<&syn::PatIdent> {
    match pattern {
        Pat::Ident(binding) => Some(binding),
        Pat::Type(typed) => pattern_ident(&typed.pat),
        _ => None,
    }
}

fn pattern_type(pattern: &Pat) -> Option<&Type> {
    match pattern {
        Pat::Type(typed) => Some(&typed.ty),
        _ => None,
    }
}

fn type_is_engine(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            path.path.segments.last().is_some_and(|segment| segment.ident == "Engine")
        }
        Type::Reference(reference) => type_is_engine(&reference.elem),
        Type::Paren(paren) => type_is_engine(&paren.elem),
        _ => false,
    }
}

fn visibility_exceeds_module(visibility: &Visibility) -> bool {
    !matches!(visibility, Visibility::Inherited)
}

fn expression_base_ident(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            path.path.segments.first().map(|segment| segment.ident.to_string())
        }
        Expr::Paren(paren) => expression_base_ident(&paren.expr),
        Expr::Reference(reference) => expression_base_ident(&reference.expr),
        Expr::Unary(unary) => expression_base_ident(&unary.expr),
        _ => None,
    }
}

fn type_last_ident(ty: &Type) -> Option<&syn::Ident> {
    let Type::Path(path) = ty else { return None };
    path.path.segments.last().map(|segment| &segment.ident)
}

fn collect_use_tree(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    kind: EdgeKind,
    edges: &mut BTreeSet<Edge>,
    globs: &mut BTreeSet<Location>,
    aliases: &mut BTreeMap<String, String>,
    configurations: &BTreeSet<String>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree(&path.tree, prefix, kind, edges, globs, aliases, configurations);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let mut target = prefix.clone();
            target.push(name.ident.to_string());
            let target = target.join("::");
            aliases.insert(name.ident.to_string(), target.clone());
            edges.insert(Edge {
                kind,
                target,
                location: name.ident.span().into(),
                configurations: configurations.clone(),
            });
        }
        syn::UseTree::Rename(rename) => {
            let mut target = prefix.clone();
            target.push(rename.ident.to_string());
            let target = target.join("::");
            aliases.insert(rename.rename.to_string(), target.clone());
            edges.insert(Edge {
                kind,
                target,
                location: rename.ident.span().into(),
                configurations: configurations.clone(),
            });
        }
        syn::UseTree::Glob(glob) => {
            let location = Location::from(glob.star_token.span);
            globs.insert(location);
            let mut target = prefix.clone();
            target.push("*".to_string());
            edges.insert(Edge {
                kind,
                target: target.join("::"),
                location,
                configurations: configurations.clone(),
            });
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, kind.clone(), edges, globs, aliases, configurations);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_modules_imports_fields_methods_and_callable_references() {
        let source = r#"
            mod child;
            use crate::{filter::validate, reader_pool as pool};
            use crate::search::*;
            struct Engine { reader_pool: usize, ignored: bool }
            impl Engine {
                fn search(&self) {
                    let worker = pool::dispatch;
                    worker(self.reader_pool);
                    Self::helper(self);
                }
                fn helper(&self) {}
            }
        "#;
        let analysis = analyze_source(source).expect("fixture parses");
        assert!(analysis.modules.iter().any(|module| module.name == "child"));
        assert_eq!(analysis.globs.len(), 1);
        assert_eq!(
            analysis.engine_fields,
            BTreeSet::from(["ignored".to_string(), "reader_pool".to_string()])
        );
        assert_eq!(
            analysis.engine_methods,
            BTreeSet::from(["helper".to_string(), "search".to_string()])
        );
        assert!(analysis
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::FieldAccess && edge.target == "reader_pool"));
        assert!(analysis
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::EngineMethod && edge.target == "helper"));
        assert!(analysis.edges.iter().any(|edge| {
            edge.kind == EdgeKind::Callable && edge.target == "crate::reader_pool::dispatch"
        }));
    }

    #[test]
    fn grouped_aliases_and_callable_references_are_stable_across_formatting() {
        let one_line =
            "use crate::{filter::validate as check, read::load}; fn f(){ let x = check; load(); }";
        let multi_line = r#"
            use crate::{
                filter::validate as check,
                read::load,
            };
            fn f() {
                let x = check;
                load();
            }
        "#;
        let one = analyze_source(one_line).expect("one-line fixture");
        let multi = analyze_source(multi_line).expect("multi-line fixture");
        let stable = |analysis: &Analysis| {
            analysis
                .edges
                .iter()
                .map(|edge| (edge.kind.clone(), edge.target.clone()))
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(stable(&one), stable(&multi));
        assert_eq!(one.import_aliases, multi.import_aliases);
    }

    #[test]
    fn records_non_call_field_accesses_and_alias_receivers() {
        let analysis = analyze_source(
            "fn f(engine: &Engine) { let e = &engine; let _ = e.reader_pool; engine.search(); }",
        )
        .expect("fixture parses");
        assert!(analysis.edges.iter().any(|edge| edge.target == "reader_pool"));
        assert!(analysis.edges.iter().any(|edge| edge.target == "search"));
    }

    #[test]
    fn externally_visible_inherent_methods_are_owner_qualified() {
        let analysis = analyze_source(
            "struct Work; impl Work { pub(crate) fn new() -> Self { Self } fn hidden() {} }",
        )
        .expect("fixture parses");
        assert!(analysis
            .inherent_methods
            .iter()
            .any(|method| method.owner == "Work" && method.method == "new"));
        assert!(!analysis.inherent_methods.iter().any(|method| method.method == "hidden"));
    }

    #[test]
    fn typed_and_dereferenced_engine_aliases_are_capabilities() {
        let analysis = analyze_source(
            "fn f(engine: &Engine) { let typed: &Engine = engine; let alias = &typed; \
             let _ = (*alias).reader_pool; alias.search(); }",
        )
        .expect("fixture parses");
        assert!(analysis
            .edges
            .iter()
            .any(|edge| { edge.kind == EdgeKind::FieldAccess && edge.target == "reader_pool" }));
        assert!(analysis
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::EngineMethod && edge.target == "search"));
    }

    #[test]
    fn unknown_field_receivers_remain_conservative_candidates() {
        let analysis = analyze_source("fn f(unknown: &Unknown) { let _ = unknown.reader_pool; }")
            .expect("fixture parses");
        assert!(analysis
            .edges
            .iter()
            .any(|edge| { edge.kind == EdgeKind::FieldAccess && edge.target == "reader_pool" }));
    }

    #[test]
    fn self_calls_are_engine_methods_only_inside_engine_impls() {
        let analysis = analyze_source(
            "struct Engine; struct Other; \
             impl Engine { fn a(&self) { Self::b(self); } fn b(&self) {} } \
             impl Other { fn a(&self) { Self::b(self); } fn b(&self) {} }",
        )
        .expect("fixture parses");
        let engine_b = analysis
            .edges
            .iter()
            .filter(|edge| edge.kind == EdgeKind::EngineMethod && edge.target == "b")
            .count();
        assert_eq!(engine_b, 1);
    }

    #[test]
    fn nested_modules_and_single_segment_callable_references_are_recorded() {
        let analysis = analyze_source(
            "mod outer { mod nested; } fn root_helper() {} \
             fn caller() { let deferred = root_helper; deferred(); }",
        )
        .expect("fixture parses");
        assert!(analysis.modules.iter().any(|module| module.name == "outer" && module.inline));
        assert!(analysis.modules.iter().any(|module| module.name == "nested" && !module.inline));
        assert!(analysis
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::Callable && edge.target == "root_helper"));
    }

    #[test]
    fn cfg_edges_are_kept_in_their_compiler_configuration() {
        let analysis = analyze_source(
            "#[cfg(feature = \"test-hooks\")] use crate::test_hooks::seam; \
             #[cfg(any(test, feature = \"test-hooks\"))] fn f() { seam(); } \
             #[cfg(feature = \"tc5-benchmark\")] fn b() { crate::tc5_benchmark::run(); }",
        )
        .expect("fixture parses");
        let seam = analysis
            .edges
            .iter()
            .find(|edge| edge.target.ends_with("test_hooks::seam"))
            .expect("test hook edge");
        assert_eq!(seam.configurations, BTreeSet::from(["test-hooks".to_string()]));
        let tc5 = analysis
            .edges
            .iter()
            .find(|edge| edge.target == "crate::tc5_benchmark::run")
            .expect("tc5 edge");
        assert_eq!(tc5.configurations, BTreeSet::from(["tc5-benchmark".to_string()]));
    }
}
