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

pub mod config;

pub use config::{ConfigSet, ConfigSpace};

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
    pub configurations: ConfigSet,
    /// For a dot call, the receiver expression's tokens with whitespace
    /// removed; it identifies the call site of a reviewed exception.
    pub receiver: Option<String>,
    /// For a dot call whose receiver was typed by `let x = Type::f(..)`,
    /// the associated function `f`; the type holds only if `f` returns it.
    pub constructor: Option<String>,
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
    /// Inline module scope of the `impl` block (empty at file level).
    pub scope: String,
    pub location: Location,
}

/// The paths a `type` alias's definition names, as written.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeAlias {
    /// The aliased path itself when the definition is a plain path.
    pub primary: Option<String>,
    /// Every type and trait path in the definition, generic parameters of
    /// the alias excluded.
    pub mentioned: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Analysis {
    pub modules: BTreeSet<ModuleDecl>,
    pub edges: BTreeSet<Edge>,
    pub globs: BTreeSet<Location>,
    pub engine_fields: BTreeSet<String>,
    /// Engine method name to the configurations in which this file defines it.
    pub engine_methods: BTreeMap<String, ConfigSet>,
    pub inherent_methods: BTreeSet<InherentMethod>,
    /// Every method of every `impl` block (inherent or trait, any
    /// visibility), keyed by the implementing type's last identifier.
    pub impl_methods: BTreeSet<(String, String)>,
    /// Associated functions whose declared return type is `Self` or the
    /// implementing type itself: the only calls `let x = Type::f(..)` types.
    pub constructors: BTreeSet<(String, String)>,
    pub declared_items: BTreeSet<String>,
    pub local_functions: BTreeMap<String, Location>,
    pub import_aliases: BTreeMap<String, String>,
    /// `use` aliases declared inside inline modules, keyed by the inline
    /// scope (`tests`, `outer::inner`).
    pub scoped_aliases: BTreeMap<String, BTreeMap<String, String>>,
    /// `type` aliases keyed by scope-qualified name (`Alias`,
    /// `inner::Alias`).
    pub type_aliases: BTreeMap<String, TypeAlias>,
    pub unsupported_cfg: BTreeSet<String>,
    pub macros: BTreeSet<MacroUse>,
    /// Macro invocations whose token body is not an expression list, a
    /// `matches!`-style `expr, pattern` body, or a statement list. Their
    /// dependencies are invisible, so governed modules fail closed on them.
    pub unparsed_macros: BTreeSet<MacroUse>,
}

/// Parses one engine source file and extracts its edges, evaluating every
/// `cfg`/`cfg_attr` in each configuration of `space`.
pub fn analyze_source(source: &str, space: &ConfigSpace) -> Result<Analysis, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut visitor = Analyzer::new(space);
    collect_struct_fields(&file.items, &mut visitor.struct_fields);
    visitor.visit_file(&file);
    visitor.resolve_aliases();
    Ok(visitor.analysis)
}

struct Analyzer<'s> {
    space: &'s ConfigSpace,
    analysis: Analysis,
    engine_aliases: BTreeSet<String>,
    impl_owners: Vec<String>,
    configurations: ConfigSet,
    module_depth: usize,
    module_stack: Vec<String>,
    item_stack: Vec<String>,
    binding_stack: Vec<BTreeSet<String>>,
    /// Lexically scoped `binding -> type path` for syntactically typed
    /// receivers (typed parameters, `self` in a non-Engine impl, typed lets,
    /// and `let x = Type::ctor(..)` / `Type { .. }` initialisers).
    receiver_types: Vec<BTreeMap<String, String>>,
    /// Named-field types of structs declared in this file, for receivers of
    /// the form `binding.field` where the binding's type is such a struct.
    struct_fields: BTreeMap<(String, String), String>,
    /// Module scopes that have already imported through `super::`.
    super_import_scopes: BTreeSet<String>,
    /// Generic type parameters in scope; a binding typed by one has no
    /// syntactic owner.
    generic_scopes: Vec<BTreeSet<String>>,
}

impl<'s> Analyzer<'s> {
    fn new(space: &'s ConfigSpace) -> Self {
        Self {
            space,
            analysis: Analysis::default(),
            engine_aliases: BTreeSet::from(["engine".to_string()]),
            impl_owners: Vec::new(),
            configurations: space.all(),
            module_depth: 0,
            module_stack: Vec::new(),
            item_stack: Vec::new(),
            binding_stack: Vec::new(),
            receiver_types: Vec::new(),
            struct_fields: BTreeMap::new(),
            super_import_scopes: BTreeSet::new(),
            generic_scopes: Vec::new(),
        }
    }

    fn push_generics(&mut self, generics: &syn::Generics) {
        self.generic_scopes.push(
            generics
                .params
                .iter()
                .filter_map(|parameter| match parameter {
                    syn::GenericParam::Type(parameter) => Some(parameter.ident.to_string()),
                    _ => None,
                })
                .collect(),
        );
    }

    /// A receiver type is usable only when it names a concrete type, not a
    /// generic parameter in scope.
    fn concrete_type(&self, ty: String) -> Option<String> {
        let head = ty.split("::").next().unwrap_or(&ty).split("=>").next().unwrap_or(&ty);
        (!self.generic_scopes.iter().any(|scope| scope.contains(head))).then_some(ty)
    }

    fn edge(&mut self, kind: EdgeKind, target: impl Into<String>, span: Span) {
        self.analysis.edges.insert(Edge {
            kind,
            target: target.into(),
            source_scope: self.module_stack.join("::"),
            source_item: self.item_stack.last().cloned().unwrap_or_else(|| "<module>".to_string()),
            location: span.into(),
            configurations: self.configurations,
            receiver: None,
            constructor: None,
        });
    }

    fn declared(&mut self, name: impl Into<String>) {
        self.analysis.declared_items.insert(name.into());
    }

    fn push_scope(&mut self, bindings: BTreeSet<String>, types: BTreeMap<String, String>) {
        self.binding_stack.push(bindings);
        self.receiver_types.push(types);
    }

    fn push_inherited_scope(&mut self) {
        let bindings = self.binding_stack.last().cloned().unwrap_or_default();
        let types = self.receiver_types.last().cloned().unwrap_or_default();
        self.push_scope(bindings, types);
    }

    fn pop_scope(&mut self) {
        self.binding_stack.pop();
        self.receiver_types.pop();
    }

    fn enter_function(
        &mut self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
        self_type: Option<&str>,
    ) {
        // Typed parameters are recorded when their patterns are visited.
        let mut types = BTreeMap::new();
        if inputs.iter().any(|input| matches!(input, FnArg::Receiver(_))) {
            if let Some(owner) = self_type
                .filter(|owner| *owner != "Engine")
                .and_then(|owner| self.concrete_type(owner.to_string()))
            {
                types.insert("self".to_string(), owner);
            }
        }
        self.push_scope(function_bindings(inputs), types);
    }

    /// `binding.field` where `binding` has a known type declared in this file.
    fn field_receiver_type(&self, receiver: &Expr) -> Option<String> {
        let Expr::Field(field) = receiver else { return None };
        let Member::Named(member) = &field.member else { return None };
        let base = direct_receiver_ident(&field.base)?;
        let base_type = self.receiver_types.last()?.get(&base)?;
        let owner = base_type.rsplit("::").next()?;
        self.struct_fields.get(&(owner.to_string(), member.to_string())).cloned()
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

    /// A trait named in a bound, `impl` header, `dyn`/`impl Trait` or
    /// supertrait list is a contract (type) dependency on its owner.
    fn record_trait_path(&mut self, path: &syn::Path) {
        let Some(first) = path.segments.first() else { return };
        let target =
            path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>();
        self.edge(EdgeKind::Type, target.join("::"), first.ident.span());
    }

    /// `<T as Trait>::item`: the trait path and the item named through it.
    /// The self type `T` is visited separately as a type.
    fn record_qualified_self_path(&mut self, qself: &syn::QSelf, path: &syn::Path, kind: EdgeKind) {
        if qself.position == 0 {
            return;
        }
        let Some(first) = path.segments.first() else { return };
        let target =
            path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>();
        self.edge(kind, target.join("::"), first.ident.span());
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
            self.push_inherited_scope();
            for statement in &statements {
                self.visit_stmt(statement);
            }
            self.pop_scope();
            return true;
        }
        false
    }

    fn filter_configurations(&mut self, mut keep: impl FnMut(usize) -> Option<bool>) -> bool {
        let mut unsupported = false;
        let mut kept = ConfigSet::default();
        for index in self.configurations.indices() {
            match keep(index) {
                Some(true) => kept.insert(index),
                Some(false) => {}
                None => unsupported = true,
            }
        }
        self.configurations = kept;
        unsupported
    }

    fn enter_attrs(&mut self, attrs: &[Attribute]) -> ConfigSet {
        let previous = self.configurations;
        let space = self.space;
        for attr in attrs {
            if attr.path().is_ident("cfg_attr") {
                let Meta::List(list) = &attr.meta else {
                    self.analysis
                        .unsupported_cfg
                        .insert("malformed cfg_attr attribute".to_string());
                    self.configurations = ConfigSet::default();
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
                    self.configurations = ConfigSet::default();
                    continue;
                };
                if let Some(predicate) =
                    nested.strip_prefix("cfg(").and_then(|value| value.strip_suffix(')'))
                {
                    let unsupported = self.filter_configurations(|index| {
                        match (space.evaluate(condition, index), space.evaluate(predicate, index)) {
                            (Some(false), _) => Some(true),
                            (Some(true), Some(value)) => Some(value),
                            _ => None,
                        }
                    });
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
                self.configurations = ConfigSet::default();
                continue;
            };
            let predicate = list.tokens.to_string();
            let unsupported = self.filter_configurations(|index| space.evaluate(&predicate, index));
            if unsupported {
                self.analysis.unsupported_cfg.insert(predicate);
            }
        }
        previous
    }

    fn resolve_aliases(&mut self) {
        let top_level = &self.analysis.import_aliases;
        let mut resolved = BTreeSet::new();
        for edge in &self.analysis.edges {
            // An inline module sees its own imports first; the file's
            // top-level imports approximate its usual `use super::*`.
            let scoped = self.analysis.scoped_aliases.get(&edge.source_scope);
            let aliases = AliasScope { scoped, top_level };
            if let Some((method, ty)) = edge.target.split_once(">:") {
                let (head, rest) =
                    ty.split_once("::").map_or((ty, None), |(head, rest)| (head, Some(rest)));
                let mut resolved_edge = edge.clone();
                if let Some(owner) = aliases.get(head) {
                    let qualified =
                        rest.map_or_else(|| owner.clone(), |rest| format!("{owner}::{rest}"));
                    resolved_edge.target = format!("{method}>:{qualified}");
                }
                resolved.insert(resolved_edge);
                continue;
            }
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
                    configurations: edge.configurations,
                    receiver: edge.receiver.clone(),
                    constructor: edge.constructor.clone(),
                });
            } else {
                resolved.insert(edge.clone());
            }
        }
        self.analysis.edges = resolved;
    }
}

struct AliasScope<'a> {
    scoped: Option<&'a BTreeMap<String, String>>,
    top_level: &'a BTreeMap<String, String>,
}

impl AliasScope<'_> {
    fn get(&self, name: &str) -> Option<&String> {
        self.scoped.and_then(|aliases| aliases.get(name)).or_else(|| self.top_level.get(name))
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

impl<'ast> Visit<'ast> for Analyzer<'_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.push_inherited_scope();
        visit::visit_block(self, block);
        self.pop_scope();
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
        if let (Pat::Ident(binding), Some(init)) = (&local.pat, &local.init) {
            let constructed = constructed_type(&init.expr).and_then(|ty| self.concrete_type(ty));
            if let (Some(ty), Some(types)) = (constructed, self.receiver_types.last_mut()) {
                types.insert(binding.ident.to_string(), ty);
            }
        }
        if let Some(bindings) = self.binding_stack.last_mut() {
            collect_pattern_bindings(&local.pat, bindings);
        }
    }

    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        // Any rebinding forgets a receiver type; forgetting is conservative.
        if let Some(types) = self.receiver_types.last_mut() {
            types.remove(&pattern.ident.to_string());
        }
        visit::visit_pat_ident(self, pattern);
    }

    fn visit_pat_type(&mut self, pattern: &'ast syn::PatType) {
        visit::visit_pat_type(self, pattern);
        if let (Pat::Ident(binding), Some(ty)) = (
            pattern.pat.as_ref(),
            type_path_string(&pattern.ty).and_then(|ty| self.concrete_type(ty)),
        ) {
            if let Some(types) = self.receiver_types.last_mut() {
                types.insert(binding.ident.to_string(), ty);
            }
        }
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        self.push_inherited_scope();
        visit::visit_expr_closure(self, closure);
        self.pop_scope();
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.push_inherited_scope();
        visit::visit_arm(self, arm);
        self.pop_scope();
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.push_inherited_scope();
        visit::visit_expr_for_loop(self, expression);
        self.pop_scope();
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
        let aliases = if self.module_depth == 0 {
            &mut self.analysis.import_aliases
        } else {
            self.analysis.scoped_aliases.entry(self.module_stack.join("::")).or_default()
        };
        let mut use_edges = BTreeSet::new();
        collect_use_tree(
            &item.tree,
            &mut Vec::new(),
            kind,
            &mut use_edges,
            &mut self.analysis.globs,
            aliases,
            self.configurations,
            &self.module_stack.join("::"),
            self.item_stack.last().map_or("<module>", String::as_str),
        );
        if use_edges.iter().any(|edge| {
            edge.kind == EdgeKind::Import
                && (edge.target == "super::*" || edge.target.starts_with("super::"))
        }) {
            self.super_import_scopes.insert(self.module_stack.join("::"));
        }
        self.analysis.edges.extend(use_edges);
        visit::visit_item_use(self, item);
        self.configurations = previous;
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        let previous = self.enter_attrs(&item.attrs);
        self.declared(item.ident.to_string());
        self.item_stack.push(item.ident.to_string());
        if item.ident == "Engine" {
            for field in &item.fields {
                if let Some(ident) = &field.ident {
                    self.analysis.engine_fields.insert(ident.to_string());
                }
            }
        }
        visit::visit_item_struct(self, item);
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_field(&mut self, field: &'ast syn::Field) {
        let previous = self.enter_attrs(&field.attrs);
        visit::visit_field(self, field);
        self.configurations = previous;
    }

    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        self.declared(item.ident.to_string());
        let previous = self.enter_attrs(&item.attrs);
        self.item_stack.push(item.ident.to_string());
        visit::visit_item_enum(self, item);
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        let previous = self.enter_attrs(&variant.attrs);
        let owner = self.item_stack.last().cloned().unwrap_or_default();
        self.item_stack.push(format!("{owner}::{}", variant.ident));
        visit::visit_variant(self, variant);
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        let previous = self.enter_attrs(&item.attrs);
        self.declared(item.ident.to_string());
        self.item_stack.push(item.ident.to_string());
        visit::visit_item_const(self, item);
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        let previous = self.enter_attrs(&item.attrs);
        self.declared(item.ident.to_string());
        self.item_stack.push(item.ident.to_string());
        visit::visit_item_static(self, item);
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_item_trait(&mut self, item: &'ast ItemTrait) {
        let previous = self.enter_attrs(&item.attrs);
        self.declared(item.ident.to_string());
        self.item_stack.push(item.ident.to_string());
        self.push_generics(&item.generics);
        // `Self` in a trait body is the implementing type, never concrete.
        if let Some(scope) = self.generic_scopes.last_mut() {
            scope.insert("Self".to_string());
        }
        visit::visit_item_trait(self, item);
        self.generic_scopes.pop();
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        let previous = self.enter_attrs(&item.attrs);
        let owner = self.item_stack.last().cloned().unwrap_or_default();
        self.item_stack.push(format!("{owner}::{}", item.sig.ident));
        self.push_generics(&item.sig.generics);
        self.enter_function(&item.sig.inputs, None);
        visit::visit_trait_item_fn(self, item);
        self.pop_scope();
        self.generic_scopes.pop();
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_item_type(&mut self, item: &'ast ItemType) {
        let previous = self.enter_attrs(&item.attrs);
        self.declared(item.ident.to_string());
        let generics = item
            .generics
            .params
            .iter()
            .filter_map(|parameter| match parameter {
                syn::GenericParam::Type(parameter) => Some(parameter.ident.to_string()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let mut collector = PathCollector::default();
        collector.visit_type(&item.ty);
        let scope = self.module_stack.join("::");
        let key = if scope.is_empty() {
            item.ident.to_string()
        } else {
            format!("{scope}::{}", item.ident)
        };
        let named = |path: &String| {
            !generics.contains(path.split("::").next().unwrap_or(path)) && path != "Self"
        };
        self.analysis.type_aliases.insert(
            key,
            TypeAlias {
                primary: plain_type_path(&item.ty).filter(named),
                mentioned: collector.paths.into_iter().filter(named).collect(),
            },
        );
        self.item_stack.push(item.ident.to_string());
        visit::visit_item_type(self, item);
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let previous = self.enter_attrs(&item.attrs);
        let name = item.sig.ident.to_string();
        self.declared(name.clone());
        self.analysis.local_functions.insert(name.clone(), item.sig.ident.span().into());
        self.item_stack.push(name);
        self.push_generics(&item.sig.generics);
        self.enter_function(&item.sig.inputs, None);
        visit::visit_item_fn(self, item);
        self.pop_scope();
        self.generic_scopes.pop();
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        let previous = self.enter_attrs(&item.attrs);
        let owner = type_last_ident(&item.self_ty).map(ToString::to_string);
        if owner.as_deref() == Some("Engine") {
            for member in &item.items {
                if let ImplItem::Fn(method) = member {
                    let impl_configurations = self.enter_attrs(&method.attrs);
                    let active = self.configurations;
                    self.configurations = impl_configurations;
                    let entry = self
                        .analysis
                        .engine_methods
                        .entry(method.sig.ident.to_string())
                        .or_default();
                    *entry = entry.union(active);
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
                                scope: self.module_stack.join("::"),
                                location: method.sig.ident.span().into(),
                            });
                        }
                    }
                }
            }
        }
        if let Some((_, trait_path, _)) = &item.trait_ {
            self.record_trait_path(trait_path);
        }
        if let Some(owner) = &owner {
            for member in &item.items {
                if let ImplItem::Fn(method) = member {
                    let name = method.sig.ident.to_string();
                    self.analysis.impl_methods.insert((owner.clone(), name.clone()));
                    if returns_owner(&method.sig.output, owner) {
                        self.analysis.constructors.insert((owner.clone(), name));
                    }
                }
            }
            self.impl_owners.push(owner.clone());
        }
        self.push_generics(&item.generics);
        visit::visit_item_impl(self, item);
        self.generic_scopes.pop();
        if owner.is_some() {
            self.impl_owners.pop();
        }
        self.configurations = previous;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        let previous = self.enter_attrs(&item.attrs);
        let owner = self.impl_owners.last().cloned().unwrap_or_else(|| "<impl>".to_string());
        self.item_stack.push(format!("{owner}::{}", item.sig.ident));
        self.push_generics(&item.sig.generics);
        self.enter_function(&item.sig.inputs, Some(owner.as_str()));
        visit::visit_impl_item_fn(self, item);
        self.pop_scope();
        self.generic_scopes.pop();
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
        if let Some(qself) = &ty.qself {
            self.record_qualified_self_path(qself, &ty.path, EdgeKind::Type);
        } else {
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

    fn visit_trait_bound(&mut self, bound: &'ast syn::TraitBound) {
        self.record_trait_path(&bound.path);
        visit::visit_trait_bound(self, bound);
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
            // `.<method>` is an unresolved receiver; `.<method>:Type` carries
            // the receiver's syntactic type for owner resolution.
            let receiver_type = direct_receiver_ident(&expression.receiver)
                .and_then(|ident| {
                    self.receiver_types.last().and_then(|types| types.get(&ident)).cloned()
                })
                .or_else(|| self.field_receiver_type(&expression.receiver))
                .or_else(|| constructed_type(&expression.receiver))
                .and_then(|ty| self.concrete_type(ty));
            let (receiver_type, constructor) = match receiver_type {
                Some(ty) => match ty.split_once("=>") {
                    Some((ty, constructor)) => {
                        (Some(ty.to_string()), Some(constructor.to_string()))
                    }
                    None => (Some(ty), None),
                },
                None => (None, None),
            };
            let target = match receiver_type {
                Some(ty) => format!(".<{}>:{ty}", expression.method),
                None => format!(".<{}>", expression.method),
            };
            let receiver = receiver_text(&expression.receiver);
            self.analysis.edges.insert(Edge {
                kind: EdgeKind::Callable,
                target,
                source_scope: self.module_stack.join("::"),
                source_item: self
                    .item_stack
                    .last()
                    .cloned()
                    .unwrap_or_else(|| "<module>".to_string()),
                location: expression.method.span().into(),
                configurations: self.configurations,
                receiver: Some(receiver),
                constructor,
            });
        }
        visit::visit_expr_method_call(self, expression);
    }

    fn visit_expr_call(&mut self, expression: &'ast ExprCall) {
        if let Expr::Path(path) = expression.func.as_ref() {
            match &path.qself {
                Some(qself) => {
                    self.visit_type(&qself.ty);
                    self.record_qualified_self_path(qself, &path.path, EdgeKind::Callable);
                }
                None => self.record_callable_path(&path.path),
            }
            // Turbofish and const-generic arguments name owners too.
            for segment in &path.path.segments {
                self.visit_path_arguments(&segment.arguments);
            }
        } else {
            self.visit_expr(&expression.func);
        }
        for argument in &expression.args {
            self.visit_expr(argument);
        }
    }

    fn visit_expr_path(&mut self, expression: &'ast ExprPath) {
        if let Some(qself) = &expression.qself {
            self.record_qualified_self_path(qself, &expression.path, EdgeKind::Callable);
        } else {
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
                        || (self.super_import_scopes.contains(&self.module_stack.join("::"))
                            && !self
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

/// Collects the type and trait paths a type names, as written.
#[derive(Default)]
struct PathCollector {
    paths: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for PathCollector {
    fn visit_type_path(&mut self, ty: &'ast TypePath) {
        let segments =
            ty.path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>();
        match &ty.qself {
            Some(qself) if qself.position > 0 => {
                self.paths.insert(segments[..qself.position].join("::"));
            }
            Some(_) => {}
            None => {
                self.paths.insert(segments.join("::"));
            }
        }
        visit::visit_type_path(self, ty);
    }

    fn visit_trait_bound(&mut self, bound: &'ast syn::TraitBound) {
        self.paths.insert(
            bound
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        );
        visit::visit_trait_bound(self, bound);
    }
}

/// The path of a type written as a plain path (through references,
/// parentheses and groups), generic arguments dropped.
fn plain_type_path(ty: &Type) -> Option<String> {
    match ty {
        Type::Reference(reference) => plain_type_path(&reference.elem),
        Type::Paren(paren) => plain_type_path(&paren.elem),
        Type::Group(group) => plain_type_path(&group.elem),
        Type::Path(path) if path.qself.is_none() => Some(
            path.path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        ),
        _ => None,
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

fn collect_struct_fields(items: &[syn::Item], fields: &mut BTreeMap<(String, String), String>) {
    for item in items {
        match item {
            syn::Item::Struct(structure) => {
                let generics = structure
                    .generics
                    .type_params()
                    .map(|parameter| parameter.ident.to_string())
                    .collect::<BTreeSet<_>>();
                for field in &structure.fields {
                    let ty = type_path_string(&field.ty)
                        .filter(|ty| !generics.contains(ty.split("::").next().unwrap_or(ty)));
                    if let (Some(name), Some(ty)) = (&field.ident, ty) {
                        fields.insert((structure.ident.to_string(), name.to_string()), ty);
                    }
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect_struct_fields(items, fields);
                }
            }
            _ => {}
        }
    }
}

/// The path of a syntactically named type, looking through references,
/// parentheses and groups; generic arguments are not part of the identity.
fn type_path_string(ty: &Type) -> Option<String> {
    match ty {
        Type::Reference(reference) => type_path_string(&reference.elem),
        Type::Paren(paren) => type_path_string(&paren.elem),
        Type::Group(group) => type_path_string(&group.elem),
        Type::Path(path) if path.qself.is_none() => {
            // Smart pointers auto-deref to their pointee for method calls.
            let last = path.path.segments.last()?;
            if matches!(last.ident.to_string().as_str(), "Arc" | "Rc" | "Box") {
                if let syn::PathArguments::AngleBracketed(arguments) = &last.arguments {
                    if let [syn::GenericArgument::Type(inner)] =
                        arguments.args.iter().collect::<Vec<_>>().as_slice()
                    {
                        return type_path_string(inner);
                    }
                }
                return None;
            }
            Some(
                path.path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::"),
            )
        }
        _ => None,
    }
}

/// `Type::constructor(..)`, `Type { .. }` and `&`/paren forms of them.
fn constructed_type(expression: &Expr) -> Option<String> {
    let capitalised = |name: &str| name.chars().next().is_some_and(char::is_uppercase);
    match expression {
        Expr::Call(call) => {
            let Expr::Path(path) = call.func.as_ref() else { return None };
            let segments = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            let (last, owner) = segments.split_last()?;
            let owner_name = owner.last()?;
            // Smart-pointer constructors type the binding as their pointee
            // when it is itself constructed, and as unknown otherwise.
            if matches!(owner_name.as_str(), "Arc" | "Rc" | "Box") {
                return call.args.first().and_then(constructed_type);
            }
            // The call types the binding only if `last` is a constructor
            // of `owner`; that is checked against the owner's declared
            // return type once every module is parsed.
            (!capitalised(last) && capitalised(owner_name) && owner_name != "Self")
                .then(|| format!("{}=>{last}", owner.join("::")))
        }
        Expr::Struct(structure) if structure.qself.is_none() => {
            let segments = structure
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            segments.last().is_some_and(|name| name != "Self").then(|| segments.join("::"))
        }
        Expr::Reference(reference) => constructed_type(&reference.expr),
        Expr::Paren(paren) => constructed_type(&paren.expr),
        _ => None,
    }
}

/// `-> Self` or `-> Owner` (generic arguments ignored).
fn returns_owner(output: &syn::ReturnType, owner: &str) -> bool {
    let syn::ReturnType::Type(_, ty) = output else { return false };
    let Type::Path(path) = ty.as_ref() else { return false };
    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "Self" || segment.ident == owner)
}

/// A canonical spelling of a receiver expression: bindings, paths, field
/// and index chains, calls, method calls, references, dereferences, `?`
/// and macro names; any other expression form is `_`.
fn receiver_text(expression: &Expr) -> String {
    let path = |path: &syn::Path| {
        path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>().join("::")
    };
    match expression {
        Expr::Path(expression) => path(&expression.path),
        Expr::Field(field) => {
            let member = match &field.member {
                Member::Named(name) => name.to_string(),
                Member::Unnamed(index) => index.index.to_string(),
            };
            format!("{}.{member}", receiver_text(&field.base))
        }
        Expr::MethodCall(call) => format!("{}.{}()", receiver_text(&call.receiver), call.method),
        Expr::Call(call) => format!("{}()", receiver_text(&call.func)),
        Expr::Index(index) => format!("{}[]", receiver_text(&index.expr)),
        Expr::Reference(reference) => format!("&{}", receiver_text(&reference.expr)),
        Expr::Unary(unary) => {
            let operator = match unary.op {
                syn::UnOp::Deref(_) => "*",
                syn::UnOp::Not(_) => "!",
                _ => "-",
            };
            format!("{operator}{}", receiver_text(&unary.expr))
        }
        Expr::Paren(paren) => receiver_text(&paren.expr),
        Expr::Try(expression) => format!("{}?", receiver_text(&expression.expr)),
        Expr::Macro(expression) => format!("{}!", path(&expression.mac.path)),
        Expr::Lit(_) => "<literal>".to_string(),
        _ => "_".to_string(),
    }
}

/// A receiver that is a binding itself (not a field or call result).
fn direct_receiver_ident(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            path.path.segments.first().map(|segment| segment.ident.to_string())
        }
        Expr::Paren(paren) => direct_receiver_ident(&paren.expr),
        Expr::Reference(reference) => direct_receiver_ident(&reference.expr),
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => {
            direct_receiver_ident(&unary.expr)
        }
        _ => None,
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
    configurations: ConfigSet,
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
                configurations,
                receiver: None,
                constructor: None,
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
                configurations,
                receiver: None,
                constructor: None,
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
                configurations,
                receiver: None,
                constructor: None,
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

    const MANIFEST: &str = "[features]\ndefault = []\ntest-hooks = []\ntc5-benchmark = []\n\
                            operator = []\ndefault-reranker = []\n";

    fn space() -> ConfigSpace {
        ConfigSpace::from_manifest(
            MANIFEST,
            &[
                ("test-hooks".to_string(), "hooks".to_string()),
                ("tc5-benchmark".to_string(), "tc5".to_string()),
                ("operator".to_string(), "operator".to_string()),
            ],
        )
        .expect("fixture configuration space")
    }

    fn analyze(source: &str) -> Result<Analysis, syn::Error> {
        analyze_source(source, &space())
    }

    fn feature_set(space: &ConfigSpace, feature: &str) -> ConfigSet {
        let mut set = ConfigSet::default();
        for (index, configuration) in space.configurations.iter().enumerate() {
            if configuration.features.contains(feature) {
                set.insert(index);
            }
        }
        set
    }

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
        let analysis = analyze(source).expect("fixture parses");
        assert!(analysis.modules.iter().any(|module| module.name == "child"));
        assert_eq!(analysis.globs.len(), 1);
        assert_eq!(
            analysis.engine_fields,
            BTreeSet::from(["ignored".to_string(), "reader_pool".to_string()])
        );
        assert_eq!(
            analysis.engine_methods.keys().cloned().collect::<BTreeSet<_>>(),
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
        let one = analyze(one_line).expect("one-line fixture");
        let multi = analyze(multi_line).expect("multi-line fixture");
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
        let analysis = analyze(
            "fn f(engine: &Engine) { let e = &engine; let _ = e.reader_pool; engine.search(); }",
        )
        .expect("fixture parses");
        assert!(analysis.edges.iter().any(|edge| edge.target == "reader_pool"));
        assert!(analysis.edges.iter().any(|edge| edge.target == "search"));
    }

    #[test]
    fn externally_visible_inherent_methods_are_owner_qualified() {
        let analysis = analyze(
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
        let analysis = analyze(
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
        let analysis = analyze("fn f(unknown: &Unknown) { let _ = unknown.reader_pool; }")
            .expect("fixture parses");
        assert!(analysis
            .edges
            .iter()
            .any(|edge| { edge.kind == EdgeKind::FieldAccess && edge.target == "reader_pool" }));
    }

    #[test]
    fn self_calls_are_engine_methods_only_inside_engine_impls() {
        let analysis = analyze(
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
        let analysis = analyze(
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
        let analysis = analyze(
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
        let space = space();
        assert_eq!(seam.configurations, feature_set(&space, "test-hooks"));
        let tc5 = analysis
            .edges
            .iter()
            .find(|edge| edge.target == "crate::tc5_benchmark::run")
            .expect("tc5 edge");
        assert_eq!(tc5.configurations, feature_set(&space, "tc5-benchmark"));
    }

    #[test]
    fn unknown_features_are_rejected_instead_of_erasing_edges() {
        let analysis =
            analyze("#[cfg(feature = \"slice85-unknown\")] use crate::reader_pool::ReaderRequest;")
                .expect("fixture parses");
        assert_eq!(
            analysis.unsupported_cfg,
            BTreeSet::from(["feature = \"slice85-unknown\"".to_string()])
        );
    }

    #[test]
    fn root_glob_bare_callable_references_are_recorded() {
        let analysis =
            analyze("use super::*; fn hidden_exec() { let _ = encode_graph_expand_result_v1; }")
                .expect("fixture parses");
        assert!(analysis.edges.iter().any(|edge| {
            edge.kind == EdgeKind::Callable
                && edge.source_item == "hidden_exec"
                && edge.target == "encode_graph_expand_result_v1"
        }));
    }

    #[test]
    fn root_glob_candidates_respect_parameter_and_local_shadowing() {
        let analysis = analyze(
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
        let analysis = analyze(
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
        let analysis = analyze(
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
            analyze("macro_rules! m { () => {{ 1usize }}; }").expect("clean fixture parses");
        let changed =
            analyze("macro_rules! m { () => {{ 2usize }}; }").expect("changed fixture parses");
        assert_ne!(clean.macros, changed.macros);
    }

    #[test]
    fn root_glob_shadowing_expires_at_the_end_of_its_lexical_block() {
        let analysis = analyze(
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
        let analysis = analyze(
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
        let space = space();
        assert!(call.configurations.indices().all(|index| {
            let configuration = &space.configurations[index];
            !configuration.linux || configuration.features.contains("test-hooks")
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
        let analysis = analyze(
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
        let analysis = analyze("fn f() { opaque!(=> crate::a::hidden); }").expect("fixture parses");
        let usage = analysis.unparsed_macros.iter().next().expect("unparsed macro recorded");
        assert_eq!(usage.target, "opaque");
        assert_eq!(usage.source_item, "f");
        assert!(usage.fingerprint.is_some());
    }

    #[test]
    fn capitalised_qualified_paths_are_edges() {
        let analysis = analyze(
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

    #[test]
    fn receivers_carry_their_syntactic_type() {
        let analysis = analyze(
            "struct Holder { inner: crate::a::Inner } \
             fn f(sink: &crate::a::Sink, shared: std::sync::Arc<crate::a::Shared>, h: Holder) { \
                 sink.one(); shared.two(); h.inner.three(); \
                 let built = crate::a::Built::new(); built.four(); \
                 let sink = unknown(); sink.five(); \
                 let _ = |sink: crate::a::Other| sink.six(); \
             }",
        )
        .expect("fixture parses");
        let targets =
            analysis.edges.iter().map(|edge| edge.target.as_str()).collect::<BTreeSet<_>>();
        for expected in [
            ".<one>:crate::a::Sink",
            ".<two>:crate::a::Shared",
            ".<three>:crate::a::Inner",
            ".<four>:crate::a::Built",
            ".<five>",
            ".<six>:crate::a::Other",
        ] {
            assert!(targets.contains(expected), "missing {expected}: {targets:?}");
        }
    }

    #[test]
    fn statement_expression_and_inner_cfg_are_evaluated() {
        let space = space();
        let analysis = analyze("#![cfg(feature = \"operator\")]\nfn f() { crate::a::one(); }")
            .expect("parses");
        let one = analysis.edges.iter().find(|edge| edge.target == "crate::a::one").expect("edge");
        assert_eq!(one.configurations, feature_set(&space, "operator"));
        let analysis = analyze(
            "fn g(x: u8) { #[cfg(feature = \"operator\")] crate::a::two(); \
             #[cfg(feature = \"operator\")] let _ = crate::a::three; \
             match x { #[cfg(feature = \"operator\")] 1 => crate::a::four(), _ => {} } \
             #[cfg(feature = \"slice85-unknown\")] crate::a::five(); }",
        )
        .expect("parses");
        for name in ["two", "three", "four"] {
            let edge = analysis
                .edges
                .iter()
                .find(|edge| edge.target == format!("crate::a::{name}"))
                .expect("edge");
            assert_eq!(edge.configurations, feature_set(&space, "operator"), "{name}");
        }
        assert!(analysis.unsupported_cfg.contains("feature = \"slice85-unknown\""));
    }
}
