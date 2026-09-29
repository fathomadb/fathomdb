//! Syntax-bounded module dependency analysis for `fathomdb-engine`.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use syn::parse::{ParseStream, Parser};
use syn::visit::{self, Visit};
use syn::{
    Attribute, Block, Expr, ExprCall, ExprField, ExprMethodCall, ExprPath, ExprStruct, FnArg,
    ImplItem, ImplItemFn, ItemEnum, ItemFn, ItemImpl, ItemMacro, ItemMod, ItemStruct, ItemTrait,
    ItemType, ItemUse, Local, Macro, Member, Meta, Pat, PatStruct, PatTupleStruct, Type, TypePath,
    Visibility,
};

pub const CONFIGURATIONS: [&str; 16] = [
    "default-linux",
    "default-nonlinux",
    "hooks-linux",
    "hooks-nonlinux",
    "tc5-linux",
    "tc5-nonlinux",
    "hooks-tc5-linux",
    "hooks-tc5-nonlinux",
    "test-linux",
    "test-nonlinux",
    "test-hooks-linux",
    "test-hooks-nonlinux",
    "test-tc5-linux",
    "test-tc5-nonlinux",
    "test-hooks-tc5-linux",
    "test-hooks-tc5-nonlinux",
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EdgeKind {
    Import,
    Callable,
    FieldAccess,
    EngineMethod,
    /// A `pub`/`pub(crate)`/`pub(super)` `use`: composition metadata.
    Reexport,
    /// A type, constant, variant, struct literal or pattern reference: a
    /// contract/owner dependency.
    Type,
}

impl EdgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Callable => "callable",
            Self::FieldAccess => "field-access",
            Self::EngineMethod => "engine-method",
            Self::Reexport => "reexport",
            Self::Type => "type",
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
    pub source_scope: String,
    pub source_item: String,
    pub location: Location,
    pub configurations: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MacroUse {
    pub target: String,
    pub fingerprint: Option<String>,
    pub source_scope: String,
    pub source_item: String,
    pub location: Location,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleDecl {
    pub name: String,
    pub scope: String,
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
    pub macros: BTreeSet<MacroUse>,
    /// Macro invocations whose token body is not an expression list, a
    /// `matches!`-style `expr, pattern` body, or a statement list. Their
    /// dependencies are invisible, so governed modules fail closed on them.
    pub unparsed_macros: BTreeSet<MacroUse>,
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
    module_stack: Vec<String>,
    item_stack: Vec<String>,
    binding_stack: Vec<BTreeSet<String>>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self {
            analysis: Analysis::default(),
            engine_aliases: BTreeSet::from(["engine".to_string()]),
            impl_owners: Vec::new(),
            configurations: CONFIGURATIONS.into_iter().map(str::to_string).collect(),
            module_depth: 0,
            module_stack: Vec::new(),
            item_stack: Vec::new(),
            binding_stack: Vec::new(),
        }
    }
}

impl Analyzer {
    fn edge(&mut self, kind: EdgeKind, target: impl Into<String>, span: Span) {
        self.analysis.edges.insert(Edge {
            kind,
            target: target.into(),
            source_scope: self.module_stack.join("::"),
            source_item: self.item_stack.last().cloned().unwrap_or_else(|| "<module>".to_string()),
            location: span.into(),
            configurations: self.configurations.clone(),
        });
    }

    fn declared(&mut self, name: impl Into<String>) {
        self.analysis.declared_items.insert(name.into());
    }

    fn record_macro(&mut self, path: &syn::Path, span: Span) {
        self.analysis.macros.insert(MacroUse {
            target: path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
            fingerprint: None,
            source_scope: self.module_stack.join("::"),
            source_item: self.item_stack.last().cloned().unwrap_or_else(|| "<module>".to_string()),
            location: span.into(),
        });
    }

    fn record_local_macro(&mut self, item: &ItemMacro) {
        let name = item.ident.as_ref().expect("macro_rules item has a name");
        self.analysis.macros.insert(MacroUse {
            target: format!("macro_rules::{name}"),
            fingerprint: Some(stable_token_fingerprint(&item.mac.tokens.to_string())),
            source_scope: self.module_stack.join("::"),
            source_item: self.item_stack.last().cloned().unwrap_or_else(|| "<module>".to_string()),
            location: name.span().into(),
        });
    }

    fn record_callable_path(&mut self, path: &syn::Path) {
        self.record_path(path, true);
    }

    /// Records an expression-position path. Capitalised final segments
    /// (constants, statics, enum variants, unit/tuple-struct constructors,
    /// struct literals and patterns) are dependencies too: in call position
    /// they are callable edges, otherwise type edges.
    fn record_path(&mut self, path: &syn::Path, call_position: bool) {
        let segments =
            path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>();
        let Some(last) = segments.last() else { return };
        let reader_request_variant = segments.windows(2).any(|pair| pair[0] == "ReaderRequest");
        let capitalised = last.chars().next().is_some_and(char::is_uppercase);
        if capitalised && !reader_request_variant {
            if segments.len() < 2 || segments.first().is_some_and(|segment| segment == "Self") {
                return;
            }
            let kind = if call_position { EdgeKind::Callable } else { EdgeKind::Type };
            self.edge(kind, segments.join("::"), path.segments[0].ident.span());
            return;
        }
        if segments.first().is_some_and(|segment| segment == "Engine")
            || (segments.first().is_some_and(|segment| segment == "Self")
                && self.impl_owners.last().is_some_and(|owner| owner == "Engine"))
        {
            self.edge(EdgeKind::EngineMethod, last.clone(), path.segments[0].ident.span());
        } else {
            self.edge(EdgeKind::Callable, segments.join("::"), path.segments[0].ident.span());
        }
    }

    fn record_unparsed_macro(&mut self, mac: &Macro, span: Span) {
        self.analysis.unparsed_macros.insert(MacroUse {
            target: mac
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
            fingerprint: Some(stable_token_fingerprint(&mac.tokens.to_string())),
            source_scope: self.module_stack.join("::"),
            source_item: self.item_stack.last().cloned().unwrap_or_else(|| "<module>".to_string()),
            location: span.into(),
        });
    }

    /// Visits a macro body with the ordinary extractors. Returns `false` when
    /// the body is not one of the supported token grammars.
    fn visit_macro_body(&mut self, mac: &Macro) -> bool {
        if mac.tokens.is_empty() {
            return true;
        }
        let name = mac.path.segments.last().map(|segment| segment.ident.to_string());
        if matches!(name.as_deref(), Some("matches" | "assert_matches" | "debug_assert_matches")) {
            if let Ok((expression, pattern, guard, trailing)) =
                parse_matches_body.parse2(mac.tokens.clone())
            {
                self.visit_expr(&expression);
                self.visit_pat(&pattern);
                for expression in guard.iter().chain(&trailing) {
                    self.visit_expr(expression);
                }
                return true;
            }
        }
        if name.as_deref() == Some("json") {
            let mut values = Vec::new();
            if (|input: ParseStream<'_>| parse_json_value(input, &mut values))
                .parse2(mac.tokens.clone())
                .is_ok()
            {
                for value in &values {
                    self.visit_expr(value);
                }
                return true;
            }
        }
        let bracketed =
            TokenStream::from(TokenTree::Group(Group::new(Delimiter::Bracket, mac.tokens.clone())));
        if let Ok(expression) = syn::parse2::<Expr>(bracketed) {
            match expression {
                Expr::Array(array) => {
                    for element in &array.elems {
                        self.visit_expr(element);
                    }
                    return true;
                }
                Expr::Repeat(repeat) => {
                    self.visit_expr(&repeat.expr);
                    self.visit_expr(&repeat.len);
                    return true;
                }
                _ => {}
            }
        }
        if let Ok(statements) = Block::parse_within.parse2(mac.tokens.clone()) {
            let inherited = self.binding_stack.last().cloned().unwrap_or_default();
            self.binding_stack.push(inherited);
            for statement in &statements {
                self.visit_stmt(statement);
            }
            self.binding_stack.pop();
            return true;
        }
        false
    }

    fn enter_attrs(&mut self, attrs: &[Attribute]) -> BTreeSet<String> {
        let previous = self.configurations.clone();
        for attr in attrs {
            if attr.path().is_ident("cfg_attr") {
                let Meta::List(list) = &attr.meta else {
                    self.analysis
                        .unsupported_cfg
                        .insert("malformed cfg_attr attribute".to_string());
                    self.configurations.clear();
                    continue;
                };
                let compact = list
                    .tokens
                    .to_string()
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>();
                let Some((condition, nested)) = split_cfg_attr(&compact) else {
                    self.analysis.unsupported_cfg.insert(compact);
                    self.configurations.clear();
                    continue;
                };
                if let Some(predicate) =
                    nested.strip_prefix("cfg(").and_then(|value| value.strip_suffix(')'))
                {
                    let mut unsupported = false;
                    self.configurations = self
                        .configurations
                        .iter()
                        .filter(|configuration| {
                            let active = evaluate_cfg(condition, configuration);
                            let nested_active = evaluate_cfg(predicate, configuration);
                            match (active, nested_active) {
                                (Some(false), _) => true,
                                (Some(true), Some(value)) => value,
                                _ => {
                                    unsupported = true;
                                    false
                                }
                            }
                        })
                        .cloned()
                        .collect();
                    if unsupported {
                        self.analysis.unsupported_cfg.insert(compact);
                    }
                }
                continue;
            }
            if !attr.path().is_ident("cfg") {
                continue;
            }
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
            if self.analysis.local_functions.contains_key(first) {
                resolved.insert(edge.clone());
            } else if let Some(owner) = aliases.get(first) {
                let target = tail.map_or_else(|| owner.clone(), |rest| format!("{owner}::{rest}"));
                resolved.insert(Edge {
                    kind: edge.kind.clone(),
                    target,
                    source_scope: edge.source_scope.clone(),
                    source_item: edge.source_item.clone(),
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
        "test" => Some(configuration.starts_with("test-")),
        "debug_assertions" => Some(true),
        "unix" | "target_os=\"linux\"" => Some(configuration.ends_with("-linux")),
        "windows" | "target_os=\"windows\"" => Some(configuration.ends_with("-nonlinux")),
        _ => predicate
            .strip_prefix("feature=\"")
            .and_then(|value| value.strip_suffix('"'))
            .and_then(|feature| match feature {
                "test-hooks" => Some(configuration.contains("hooks")),
                "tc5-benchmark" => Some(configuration.contains("tc5")),
                "default-embedder"
                | "default-reranker"
                | "embed-cuda"
                | "embed-metal"
                | "migration-test-hooks"
                | "operator"
                | "rerank-cuda"
                | "rerank-metal"
                | "slice72-gpu-tests"
                | "slice72-test-hooks" => Some(false),
                _ => return None,
            }),
    }
}

fn split_cfg_attr(source: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (index, ch) in source.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            ',' if depth == 0 => return Some((&source[..index], &source[index + 1..])),
            _ => {}
        }
    }
    None
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
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let inherited = self.binding_stack.last().cloned().unwrap_or_default();
        self.binding_stack.push(inherited);
        visit::visit_block(self, block);
        self.binding_stack.pop();
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if let (Some(binding), Some(init)) = (pattern_ident(&local.pat), &local.init) {
            if expression_base_ident(&init.expr)
                .is_some_and(|ident| self.engine_aliases.contains(&ident))
                || pattern_type(&local.pat).is_some_and(type_is_engine)
            {
                self.engine_aliases.insert(binding.ident.to_string());
            }
        }
        for attribute in &local.attrs {
            self.visit_attribute(attribute);
        }
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.visit_expr(diverge);
            }
        }
        self.visit_pat(&local.pat);
        if let Some(bindings) = self.binding_stack.last_mut() {
            collect_pattern_bindings(&local.pat, bindings);
        }
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        let previous = self.enter_attrs(&item.attrs);
        self.analysis.modules.insert(ModuleDecl {
            name: item.ident.to_string(),
            scope: self.module_stack.join("::"),
            inline: item.content.is_some(),
            depth: self.module_depth,
            location: item.ident.span().into(),
        });
        self.declared(item.ident.to_string());
        if item.content.is_some() {
            self.module_depth += 1;
            self.module_stack.push(item.ident.to_string());
        }
        visit::visit_item_mod(self, item);
        if item.content.is_some() {
            self.module_stack.pop();
            self.module_depth -= 1;
        }
        self.configurations = previous;
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let previous = self.enter_attrs(&item.attrs);
        let kind = if visibility_exceeds_module(&item.vis) {
            EdgeKind::Reexport
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
            &self.module_stack.join("::"),
            self.item_stack.last().map_or("<module>", String::as_str),
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
        self.analysis.local_functions.insert(name.clone(), item.sig.ident.span().into());
        self.item_stack.push(name);
        self.binding_stack.push(function_bindings(&item.sig.inputs));
        visit::visit_item_fn(self, item);
        self.binding_stack.pop();
        self.item_stack.pop();
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
        let owner = self.impl_owners.last().map_or("<impl>", String::as_str);
        self.item_stack.push(format!("{owner}::{}", item.sig.ident));
        self.binding_stack.push(function_bindings(&item.sig.inputs));
        visit::visit_impl_item_fn(self, item);
        self.binding_stack.pop();
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        if let Some(segment) = mac.path.segments.first() {
            self.record_macro(&mac.path, segment.ident.span());
            if !self.visit_macro_body(mac) {
                self.record_unparsed_macro(mac, segment.ident.span());
            }
        }
        visit::visit_macro(self, mac);
    }

    fn visit_expr_struct(&mut self, expression: &'ast ExprStruct) {
        if expression.qself.is_none() {
            self.record_path(&expression.path, false);
        }
        visit::visit_expr_struct(self, expression);
    }

    fn visit_pat_struct(&mut self, pattern: &'ast PatStruct) {
        if pattern.qself.is_none() {
            self.record_path(&pattern.path, false);
        }
        visit::visit_pat_struct(self, pattern);
    }

    fn visit_pat_tuple_struct(&mut self, pattern: &'ast PatTupleStruct) {
        if pattern.qself.is_none() {
            self.record_path(&pattern.path, false);
        }
        visit::visit_pat_tuple_struct(self, pattern);
    }

    fn visit_item_macro(&mut self, item: &'ast ItemMacro) {
        if item.mac.path.is_ident("macro_rules") {
            if item.ident.is_some() {
                self.record_local_macro(item);
            }
            return;
        }
        visit::visit_item_macro(self, item);
    }

    fn visit_type_path(&mut self, ty: &'ast TypePath) {
        if ty.qself.is_none() {
            let target = ty
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            if let Some(first) = ty.path.segments.first() {
                self.edge(EdgeKind::Type, target, first.ident.span());
            }
        }
        visit::visit_type_path(self, ty);
    }

    fn visit_expr_field(&mut self, expression: &'ast ExprField) {
        let Member::Named(field) = &expression.member else {
            visit::visit_expr_field(self, expression);
            return;
        };
        if expression_base_ident(&expression.base).as_deref() == Some("self")
            && !self.impl_owners.last().is_some_and(|owner| owner == "Engine")
        {
            visit::visit_expr_field(self, expression);
            return;
        }
        // Resolution against the source-derived Engine field set happens after
        // all modules are parsed. Recording every named access makes an
        // unknown receiver conservative without manufacturing an edge for
        // fields that do not belong to Engine.
        self.edge(EdgeKind::FieldAccess, field.to_string(), field.span());
        visit::visit_expr_field(self, expression);
    }

    fn visit_expr_method_call(&mut self, expression: &'ast ExprMethodCall) {
        if expression_base_ident(&expression.receiver).is_some_and(|ident| {
            self.engine_aliases.contains(&ident)
                || (ident == "self"
                    && self.impl_owners.last().is_some_and(|owner| owner == "Engine"))
        }) {
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

    fn visit_expr_call(&mut self, expression: &'ast ExprCall) {
        if let Expr::Path(path) = expression.func.as_ref() {
            if path.qself.is_none() {
                self.record_callable_path(&path.path);
            }
        } else {
            self.visit_expr(&expression.func);
        }
        for argument in &expression.args {
            self.visit_expr(argument);
        }
    }

    fn visit_expr_path(&mut self, expression: &'ast ExprPath) {
        if expression.qself.is_none() {
            let segments = expression
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            let callable_reference = segments.len() >= 2
                || segments.first().is_some_and(|name| {
                    self.analysis.local_functions.contains_key(name)
                        || self.analysis.import_aliases.contains_key(name)
                        || (self.analysis.edges.iter().any(|edge| {
                            edge.kind == EdgeKind::Import
                                && (edge.target == "super::*" || edge.target.starts_with("super::"))
                                && edge.source_scope == self.module_stack.join("::")
                        }) && !self
                            .binding_stack
                            .last()
                            .is_some_and(|bindings| bindings.contains(name)))
                });
            if callable_reference {
                self.record_path(&expression.path, false);
            }
        }
        visit::visit_expr_path(self, expression);
    }
}

/// `serde_json::json!` grammar: objects and arrays of Rust expressions.
fn parse_json_value(input: ParseStream<'_>, values: &mut Vec<Expr>) -> syn::Result<()> {
    if input.peek(syn::token::Brace) {
        let content;
        syn::braced!(content in input);
        while !content.is_empty() {
            values.push(content.parse::<Expr>()?);
            content.parse::<syn::Token![:]>()?;
            parse_json_value(&content, values)?;
            if content.is_empty() {
                break;
            }
            content.parse::<syn::Token![,]>()?;
        }
    } else if input.peek(syn::token::Bracket) {
        let content;
        syn::bracketed!(content in input);
        while !content.is_empty() {
            parse_json_value(&content, values)?;
            if content.is_empty() {
                break;
            }
            content.parse::<syn::Token![,]>()?;
        }
    } else {
        values.push(input.parse::<Expr>()?);
    }
    Ok(())
}

type MatchesBody = (Expr, Pat, Option<Expr>, Vec<Expr>);

fn parse_matches_body(input: ParseStream<'_>) -> syn::Result<MatchesBody> {
    let expression: Expr = input.parse()?;
    input.parse::<syn::Token![,]>()?;
    let pattern = Pat::parse_multi_with_leading_vert(input)?;
    let guard = if input.peek(syn::Token![if]) {
        input.parse::<syn::Token![if]>()?;
        Some(input.parse::<Expr>()?)
    } else {
        None
    };
    let mut trailing = Vec::new();
    if input.peek(syn::Token![,]) {
        input.parse::<syn::Token![,]>()?;
        // assert_matches! accepts trailing format arguments.
        while !input.is_empty() {
            trailing.push(input.parse::<Expr>()?);
            if input.is_empty() {
                break;
            }
            input.parse::<syn::Token![,]>()?;
        }
    }
    Ok((expression, pattern, guard, trailing))
}

fn stable_token_fingerprint(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn function_bindings(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> BTreeSet<String> {
    let mut bindings = BTreeSet::new();
    for input in inputs {
        match input {
            FnArg::Receiver(_) => {
                bindings.insert("self".to_string());
            }
            FnArg::Typed(typed) => collect_pattern_bindings(&typed.pat, &mut bindings),
        }
    }
    bindings
}

fn collect_pattern_bindings(pattern: &Pat, bindings: &mut BTreeSet<String>) {
    match pattern {
        Pat::Ident(binding) => {
            bindings.insert(binding.ident.to_string());
            if let Some((_, subpattern)) = &binding.subpat {
                collect_pattern_bindings(subpattern, bindings);
            }
        }
        Pat::Reference(reference) => collect_pattern_bindings(&reference.pat, bindings),
        Pat::Type(typed) => collect_pattern_bindings(&typed.pat, bindings),
        Pat::Paren(paren) => collect_pattern_bindings(&paren.pat, bindings),
        Pat::Slice(slice) => {
            for element in &slice.elems {
                collect_pattern_bindings(element, bindings);
            }
        }
        Pat::Struct(structure) => {
            for field in &structure.fields {
                collect_pattern_bindings(&field.pat, bindings);
            }
        }
        Pat::Tuple(tuple) => {
            for element in &tuple.elems {
                collect_pattern_bindings(element, bindings);
            }
        }
        Pat::TupleStruct(tuple) => {
            for element in &tuple.elems {
                collect_pattern_bindings(element, bindings);
            }
        }
        Pat::Or(or) => {
            for case in &or.cases {
                collect_pattern_bindings(case, bindings);
            }
        }
        _ => {}
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
        Expr::Field(field) => match &field.member {
            Member::Named(member) if member == "engine" => Some("engine".to_string()),
            _ => expression_base_ident(&field.base),
        },
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
    source_scope: &str,
    source_item: &str,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree(
                &path.tree,
                prefix,
                kind.clone(),
                edges,
                globs,
                aliases,
                configurations,
                source_scope,
                source_item,
            );
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let mut target = prefix.clone();
            let binding = if name.ident == "self" {
                prefix.last().cloned().unwrap_or_else(|| name.ident.to_string())
            } else {
                target.push(name.ident.to_string());
                name.ident.to_string()
            };
            let target = target.join("::");
            aliases.insert(binding.clone(), target.clone());
            edges.insert(Edge {
                kind: kind.clone(),
                target,
                source_scope: source_scope.to_string(),
                source_item: use_source_item(source_item, &kind, &binding),
                location: name.ident.span().into(),
                configurations: configurations.clone(),
            });
        }
        syn::UseTree::Rename(rename) => {
            let mut target = prefix.clone();
            if rename.ident != "self" {
                target.push(rename.ident.to_string());
            }
            let target = target.join("::");
            aliases.insert(rename.rename.to_string(), target.clone());
            edges.insert(Edge {
                kind: kind.clone(),
                target,
                source_scope: source_scope.to_string(),
                source_item: use_source_item(source_item, &kind, &rename.rename.to_string()),
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
                kind: kind.clone(),
                target: target.join("::"),
                source_scope: source_scope.to_string(),
                source_item: use_source_item(source_item, &kind, "*"),
                location,
                configurations: configurations.clone(),
            });
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(
                    item,
                    prefix,
                    kind.clone(),
                    edges,
                    globs,
                    aliases,
                    configurations,
                    source_scope,
                    source_item,
                );
            }
        }
    }
}

fn use_source_item(source_item: &str, kind: &EdgeKind, binding: &str) -> String {
    if source_item == "<module>" && *kind == EdgeKind::Reexport {
        format!("<module>::{binding}")
    } else {
        source_item.to_string()
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
        assert_eq!(
            seam.configurations,
            CONFIGURATIONS
                .into_iter()
                .filter(|configuration| configuration.contains("hooks"))
                .map(str::to_string)
                .collect()
        );
        let tc5 = analysis
            .edges
            .iter()
            .find(|edge| edge.target == "crate::tc5_benchmark::run")
            .expect("tc5 edge");
        assert_eq!(
            tc5.configurations,
            CONFIGURATIONS
                .into_iter()
                .filter(|configuration| configuration.contains("tc5"))
                .map(str::to_string)
                .collect()
        );
    }

    #[test]
    fn unknown_features_are_rejected_instead_of_erasing_edges() {
        let analysis = analyze_source(
            "#[cfg(feature = \"slice85-unknown\")] use crate::reader_pool::ReaderRequest;",
        )
        .expect("fixture parses");
        assert_eq!(
            analysis.unsupported_cfg,
            BTreeSet::from(["feature = \"slice85-unknown\"".to_string()])
        );
    }

    #[test]
    fn root_glob_bare_callable_references_are_recorded() {
        let analysis = analyze_source(
            "use super::*; fn hidden_exec() { let _ = encode_graph_expand_result_v1; }",
        )
        .expect("fixture parses");
        assert!(analysis.edges.iter().any(|edge| {
            edge.kind == EdgeKind::Callable
                && edge.source_item == "hidden_exec"
                && edge.target == "encode_graph_expand_result_v1"
        }));
    }

    #[test]
    fn root_glob_candidates_respect_parameter_and_local_shadowing() {
        let analysis = analyze_source(
            "use super::*; \
             fn parameter(encode_graph_expand_result_v1: usize) { \
                 let _ = encode_graph_expand_result_v1; \
             } \
             fn local() { \
                 let encode_graph_expand_result_v1 = 1usize; \
                 let _ = encode_graph_expand_result_v1; \
             }",
        )
        .expect("fixture parses");
        assert!(!analysis.edges.iter().any(|edge| {
            edge.kind == EdgeKind::Callable && edge.target == "encode_graph_expand_result_v1"
        }));
    }

    #[test]
    fn fully_qualified_internal_type_paths_are_recorded() {
        let analysis = analyze_source(
            "fn hidden_type(_: crate::reader_pool::ReaderRequest) -> \
             crate::search::SearchReaderWork { todo!() }",
        )
        .expect("fixture parses");
        assert!(analysis.edges.iter().any(|edge| {
            edge.kind == EdgeKind::Type
                && edge.source_item == "hidden_type"
                && edge.target == "crate::reader_pool::ReaderRequest"
        }));
    }

    #[test]
    fn local_macro_definitions_keep_their_declared_name() {
        let analysis = analyze_source(
            "macro_rules! hidden_boundary { () => {{ \
                 let _ = crate::reader_pool::ReaderRequest::Shutdown; \
             }}; }",
        )
        .expect("fixture parses");
        assert!(analysis.macros.iter().any(|usage| usage.target == "macro_rules::hidden_boundary"));
    }

    #[test]
    fn local_macro_definition_identity_changes_with_its_body() {
        let clean =
            analyze_source("macro_rules! m { () => {{ 1usize }}; }").expect("clean fixture parses");
        let changed = analyze_source("macro_rules! m { () => {{ 2usize }}; }")
            .expect("changed fixture parses");
        assert_ne!(clean.macros, changed.macros);
    }

    #[test]
    fn root_glob_shadowing_expires_at_the_end_of_its_lexical_block() {
        let analysis = analyze_source(
            "use super::*; fn scope_escape() { \
                 { \
                     let encode_graph_expand_result_v1 = 1usize; \
                     let _ = encode_graph_expand_result_v1; \
                 } \
                 let _ = encode_graph_expand_result_v1; \
             }",
        )
        .expect("fixture parses");
        assert_eq!(
            analysis
                .edges
                .iter()
                .filter(|edge| {
                    edge.kind == EdgeKind::Callable
                        && edge.source_item == "scope_escape"
                        && edge.target == "encode_graph_expand_result_v1"
                })
                .count(),
            1
        );
    }

    #[test]
    fn source_items_inline_scopes_cfg_attr_and_macros_are_preserved() {
        let analysis = analyze_source(
            r#"
            mod tests {
                #[cfg_attr(target_os = "linux", cfg(feature = "test-hooks"))]
                fn helper() { crate::internal_macro!(); crate::reader_pool::dispatch(); }
            }
            "#,
        )
        .expect("fixture parses");
        let call = analysis
            .edges
            .iter()
            .find(|edge| edge.target == "crate::reader_pool::dispatch")
            .expect("call edge");
        assert_eq!(call.source_scope, "tests");
        assert_eq!(call.source_item, "helper");
        assert!(call.configurations.iter().all(|configuration| {
            !configuration.ends_with("-linux") || configuration.contains("hooks")
        }));
        assert!(
            analysis.macros.iter().any(|usage| {
                usage.target == "crate::internal_macro"
                    && usage.source_scope == "tests"
                    && usage.source_item == "helper"
            }),
            "{:?}",
            analysis.macros
        );
    }

    #[test]
    fn std_macro_bodies_are_extracted_like_expressions() {
        let analysis = analyze_source(
            "fn f(value: usize, out: &mut String) { \
                 let _ = vec![crate::a::one()]; \
                 let _ = vec![crate::a::two(); 2]; \
                 let _ = format!(\"{}\", crate::a::three()); \
                 let _ = write!(out, \"{}\", crate::a::four()); \
                 assert!(matches!(value, x if x == crate::a::five())); \
                 let _ = serde_json::json!({\"k\": crate::a::six(), \"l\": [crate::a::seven()]}); \
             }",
        )
        .expect("fixture parses");
        for name in ["one", "two", "three", "four", "five", "six", "seven"] {
            assert!(
                analysis.edges.iter().any(|edge| edge.target == format!("crate::a::{name}")),
                "missing {name}"
            );
        }
        assert!(analysis.unparsed_macros.is_empty(), "{:?}", analysis.unparsed_macros);
    }

    #[test]
    fn unparsable_macro_bodies_are_recorded_with_identity() {
        let analysis =
            analyze_source("fn f() { opaque!(=> crate::a::hidden); }").expect("fixture parses");
        let usage = analysis.unparsed_macros.iter().next().expect("unparsed macro recorded");
        assert_eq!(usage.target, "opaque");
        assert_eq!(usage.source_item, "f");
        assert!(usage.fingerprint.is_some());
    }

    #[test]
    fn capitalised_qualified_paths_are_edges() {
        let analysis = analyze_source(
            "fn f(v: usize) { let _ = crate::a::CONST; let _ = crate::a::E::V; \
             let _ = crate::a::Ctor(1); let _ = crate::a::S { x: 1 }; \
             let _: Option<crate::a::T> = None; if let crate::a::P(_) = v {} \
             let _ = Self::Variant; }",
        )
        .expect("fixture parses");
        for (target, kind) in [
            ("crate::a::CONST", EdgeKind::Type),
            ("crate::a::E::V", EdgeKind::Type),
            ("crate::a::Ctor", EdgeKind::Callable),
            ("crate::a::S", EdgeKind::Type),
            ("crate::a::T", EdgeKind::Type),
            ("crate::a::P", EdgeKind::Type),
        ] {
            assert!(
                analysis.edges.iter().any(|edge| edge.target == target && edge.kind == kind),
                "missing {target}"
            );
        }
        assert!(!analysis.edges.iter().any(|edge| edge.target.starts_with("Self::")));
    }
}
