//! Syntax-bounded module dependency analysis for `fathomdb-engine`.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use syn::parse::{ParseStream, Parser};
use syn::visit::{self, Visit};
use syn::{
    Attribute, Block, Expr, ExprCall, ExprField, ExprMethodCall, ExprPath, ExprStruct, FnArg,
    ImplItem, ImplItemFn, ItemEnum, ItemExternCrate, ItemFn, ItemImpl, ItemMacro, ItemMod,
    ItemStruct, ItemTrait, ItemType, ItemUse, Local, Macro, Member, Meta, Pat, PatStruct,
    PatTupleStruct, Type, TypePath, Visibility,
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
    /// The lexical scope in which `target` is written, where the resolver
    /// starts looking its first segment up.
    pub target_scope: String,
    pub source_item: String,
    pub location: Location,
    pub configurations: ConfigSet,
}

/// A `use` binding as written: `name` bound to the path `target` in the
/// inline scope `scope` (empty at file level). The namespace resolver is the
/// only consumer; edge targets are never rewritten through bindings.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UseBinding {
    pub scope: String,
    pub name: String,
    pub target: String,
    pub location: Location,
    pub configurations: ConfigSet,
}

/// A block that declares a `use`: names it binds are visible only inside
/// it, and a name it does not bind is looked up in `parent`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BlockScope {
    /// The enclosing block or inline-module scope.
    pub parent: String,
    /// The inline module the block is in, which `self::`/`super::` name.
    pub module: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MacroUse {
    pub target: String,
    pub fingerprint: Option<String>,
    pub source_scope: String,
    pub source_item: String,
    pub location: Location,
}

/// Unsupported owner-bearing attribute indirection.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnparsedSerdePath {
    pub key: String,
    pub value: String,
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
    /// Configurations in which the declaration (and so the module) exists.
    pub configurations: ConfigSet,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Analysis {
    /// The configurations the file was analysed in: those in which its
    /// declaring `mod` item exists.
    pub configurations: ConfigSet,
    pub modules: BTreeSet<ModuleDecl>,
    pub edges: BTreeSet<Edge>,
    pub globs: BTreeSet<Location>,
    pub engine_fields: BTreeSet<String>,
    /// Engine method name to the configurations in which this file defines it.
    pub engine_methods: BTreeMap<String, ConfigSet>,
    pub declared_items: BTreeSet<String>,
    /// `(inline scope, name)` of every declared item, so a path can be
    /// checked against the scope that actually declares it.
    pub scoped_items: BTreeSet<(String, String)>,
    pub local_functions: BTreeMap<String, Location>,
    /// Every `use` binding of the file, in every scope.
    pub bindings: BTreeSet<UseBinding>,
    /// Block scopes that declare a `use`, keyed by scope name (the enclosing
    /// scope followed by `{line:column}` of the block's brace).
    pub block_scopes: BTreeMap<String, BlockScope>,
    pub unsupported_cfg: BTreeSet<String>,
    /// `mod` items carrying `#[path]` (bare or inside `cfg_attr`), by name.
    /// The compiler then builds a file other than the one the gate maps the
    /// module to, so the gate fails closed on every one.
    pub path_attributes: BTreeSet<(String, Location)>,
    /// `extern crate` declarations by the name they bind. One can rename
    /// this crate (`extern crate self as x`) or another out of path
    /// resolution, so the gate fails closed on every one.
    pub extern_crates: BTreeMap<String, Location>,
    pub macros: BTreeSet<MacroUse>,
    /// Macro invocations whose token body is not an expression list, a
    /// `matches!`-style `expr, pattern` body, or a statement list. Their
    /// dependencies are invisible, so governed modules fail closed on them.
    pub unparsed_macros: BTreeSet<MacroUse>,
    /// Owner-bearing attribute paths outside the supported grammar.
    pub unparsed_serde_paths: BTreeSet<UnparsedSerdePath>,
}

/// Parses one engine source file and extracts its edges, evaluating every
/// `cfg`/`cfg_attr` in each configuration of `space`.
pub fn analyze_source(source: &str, space: &ConfigSpace) -> Result<Analysis, syn::Error> {
    analyze_source_in(source, space, space.all())
}

/// Like [`analyze_source`] for a file whose declaring `mod` item exists only
/// in `configurations`; the file's inner `#![cfg]` narrows it further.
pub fn analyze_source_in(
    source: &str,
    space: &ConfigSpace,
    configurations: ConfigSet,
) -> Result<Analysis, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut visitor = Analyzer::new(space);
    visitor.configurations = configurations;
    visitor.analysis.configurations = configurations;
    visitor.enter_attrs(&file.attrs);
    visitor.visit_file(&file);
    visitor.resolve_generated_types();
    Ok(visitor.analysis)
}

struct Analyzer<'s> {
    space: &'s ConfigSpace,
    analysis: Analysis,
    engine_aliases: BTreeSet<String>,
    alias_scopes: Vec<BTreeSet<String>>,
    impl_owners: Vec<String>,
    configurations: ConfigSet,
    module_depth: usize,
    module_stack: Vec<String>,
    item_stack: Vec<String>,
    binding_stack: Vec<BTreeSet<String>>,
    /// Module scopes that have already imported through `super::`.
    super_import_scopes: BTreeSet<String>,
    local_macros: BTreeSet<String>,
    /// Item-position invocations of a macro: `(macro, first identifier)`.
    item_macro_invocations: Vec<(String, String, String)>,
    /// Enclosing inline-module and `use`-declaring block scopes, innermost
    /// last.
    lexical_scopes: Vec<String>,
    /// Names the `use` items of each enclosing block scope bind.
    block_names: Vec<BTreeSet<String>>,
}

impl<'s> Analyzer<'s> {
    fn new(space: &'s ConfigSpace) -> Self {
        Self {
            space,
            analysis: Analysis::default(),
            engine_aliases: BTreeSet::new(),
            alias_scopes: Vec::new(),
            impl_owners: Vec::new(),
            configurations: space.all(),
            module_depth: 0,
            module_stack: Vec::new(),
            item_stack: Vec::new(),
            binding_stack: Vec::new(),
            super_import_scopes: BTreeSet::new(),
            local_macros: BTreeSet::new(),
            item_macro_invocations: Vec::new(),
            lexical_scopes: Vec::new(),
            block_names: Vec::new(),
        }
    }

    fn lexical_scope(&self) -> String {
        self.lexical_scopes.last().cloned().unwrap_or_else(|| self.module_stack.join("::"))
    }

    /// A type generated by an item-position invocation of a local
    /// `macro_rules!` (its first identifier argument) is declared here and
    /// has the `fn`s the macro body defines.
    fn resolve_generated_types(&mut self) {
        for (name, ty, scope) in std::mem::take(&mut self.item_macro_invocations) {
            if !self.local_macros.contains(&name) {
                continue;
            }
            self.analysis.scoped_items.insert((scope, ty.clone()));
            self.analysis.declared_items.insert(ty.clone());
        }
    }

    fn edge(&mut self, kind: EdgeKind, target: impl Into<String>, span: Span) {
        self.analysis.edges.insert(Edge {
            kind,
            target: target.into(),
            source_scope: self.module_stack.join("::"),
            target_scope: self.lexical_scope(),
            source_item: self.item_stack.last().cloned().unwrap_or_else(|| "<module>".to_string()),
            location: span.into(),
            configurations: self.configurations,
        });
    }

    fn declared(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.analysis.scoped_items.insert((self.module_stack.join("::"), name.clone()));
        self.analysis.declared_items.insert(name);
    }

    fn push_inherited_scope(&mut self) {
        self.alias_scopes.push(self.engine_aliases.clone());
        self.binding_stack.push(self.binding_stack.last().cloned().unwrap_or_default());
    }
    fn pop_scope(&mut self) {
        self.engine_aliases = self.alias_scopes.pop().unwrap_or_default();
        self.binding_stack.pop();
    }
    fn enter_function(
        &mut self,
        inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
        _self_type: Option<&str>,
    ) {
        self.alias_scopes.push(std::mem::take(&mut self.engine_aliases));
        for input in inputs {
            if let FnArg::Typed(typed) = input {
                if type_is_engine(&typed.ty) {
                    if let Some(binding) = pattern_ident(&typed.pat) {
                        self.engine_aliases.insert(binding.ident.to_string());
                    }
                }
            }
        }
        self.binding_stack.push(function_bindings(inputs));
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

    fn visit_local_inner(&mut self, local: &Local) {
        if let (Some(binding), Some(init)) = (pattern_ident(&local.pat), &local.init) {
            if expression_base_ident(&init.expr)
                .is_some_and(|ident| self.engine_aliases.contains(&ident))
                || pattern_type(&local.pat).is_some_and(type_is_engine)
            {
                self.engine_aliases.insert(binding.ident.to_string());
            } else {
                self.engine_aliases.remove(&binding.ident.to_string());
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

    fn filter_configurations(&mut self, mut keep: impl FnMut(usize) -> Option<bool>) -> bool {
        let mut unsupported = false;
        let mut kept = ConfigSet::default();
        for index in self.space.all().indices() {
            match keep(index) {
                Some(true) if self.configurations.contains(index) => kept.insert(index),
                Some(true) => {}
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
}

fn split_top_level_commas(tokens: TokenStream) -> Vec<TokenStream> {
    let mut entries = vec![TokenStream::new()];
    for token in tokens {
        match &token {
            TokenTree::Punct(punct) if punct.as_char() == ',' => entries.push(TokenStream::new()),
            _ => entries.last_mut().expect("one entry").extend([token]),
        }
    }
    entries.retain(|entry| !entry.is_empty());
    entries
}

/// `path = ".."` itself, or a `cfg_attr` that applies one under any condition.
fn attribute_sets_path(meta: &Meta) -> bool {
    if meta.path().is_ident("path") {
        return true;
    }
    let Meta::List(list) = meta else { return false };
    if !list.path.is_ident("cfg_attr") {
        return false;
    }
    split_top_level_commas(list.tokens.clone())
        .into_iter()
        .skip(1)
        .any(|entry| syn::parse2::<Meta>(entry).is_ok_and(|nested| attribute_sets_path(&nested)))
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
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if attr.path().is_ident("serde") {
            let Meta::List(list) = &attr.meta else { return };
            let text = list.tokens.to_string();
            if [
                "serialize_with",
                "deserialize_with",
                "with =",
                "getter",
                "remote",
                "from =",
                "try_from",
                "into =",
                "bound =",
                "crate =",
                "skip_serializing_if",
            ]
            .iter()
            .any(|key| text.contains(key))
                && (text.contains("crate::")
                    || text.contains("super::")
                    || text.contains("self::")
                    || text.contains("<"))
            {
                self.analysis.unparsed_serde_paths.insert(UnparsedSerdePath {
                    key: "owner-bearing attribute".to_string(),
                    value: text,
                    source_scope: self.module_stack.join("::"),
                    source_item: self.item_stack.last().cloned().unwrap_or_default(),
                    location: attr.path().segments[0].ident.span().into(),
                });
            }
        }
        visit::visit_attribute(self, attr);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let mut names = BTreeSet::new();
        for statement in &block.stmts {
            if let syn::Stmt::Item(syn::Item::Use(item)) = statement {
                use_tree_names(&item.tree, &mut names);
            }
        }
        let block_scope = !names.is_empty();
        if block_scope {
            let location = Location::from(block.brace_token.span.open());
            let parent = self.lexical_scope();
            let name = format!("{{{}:{}}}", location.line, location.column);
            let scope = if parent.is_empty() { name } else { format!("{parent}::{name}") };
            self.analysis
                .block_scopes
                .insert(scope.clone(), BlockScope { parent, module: self.module_stack.join("::") });
            self.lexical_scopes.push(scope);
            self.block_names.push(names);
        }
        self.push_inherited_scope();
        visit::visit_block(self, block);
        self.pop_scope();
        if block_scope {
            self.lexical_scopes.pop();
            self.block_names.pop();
        }
    }

    fn visit_local(&mut self, local: &'ast Local) {
        let previous = self.enter_attrs(&local.attrs);
        self.visit_local_inner(local);
        self.configurations = previous;
    }

    fn visit_expr(&mut self, expression: &'ast Expr) {
        let previous = self.enter_attrs(expression_attrs(expression));
        visit::visit_expr(self, expression);
        self.configurations = previous;
    }

    fn visit_stmt_macro(&mut self, statement: &'ast syn::StmtMacro) {
        let previous = self.enter_attrs(&statement.attrs);
        visit::visit_stmt_macro(self, statement);
        self.configurations = previous;
    }

    fn visit_fn_arg(&mut self, argument: &'ast FnArg) {
        let attrs = match argument {
            FnArg::Receiver(receiver) => &receiver.attrs,
            FnArg::Typed(typed) => &typed.attrs,
        };
        let previous = self.enter_attrs(attrs);
        visit::visit_fn_arg(self, argument);
        self.configurations = previous;
    }

    fn visit_generic_param(&mut self, parameter: &'ast syn::GenericParam) {
        let attrs = match parameter {
            syn::GenericParam::Lifetime(parameter) => &parameter.attrs,
            syn::GenericParam::Type(parameter) => &parameter.attrs,
            syn::GenericParam::Const(parameter) => &parameter.attrs,
        };
        let previous = self.enter_attrs(attrs);
        visit::visit_generic_param(self, parameter);
        self.configurations = previous;
    }

    fn visit_field_pat(&mut self, field: &'ast syn::FieldPat) {
        let previous = self.enter_attrs(&field.attrs);
        visit::visit_field_pat(self, field);
        self.configurations = previous;
    }

    fn visit_field_value(&mut self, field: &'ast syn::FieldValue) {
        let previous = self.enter_attrs(&field.attrs);
        visit::visit_field_value(self, field);
        self.configurations = previous;
    }
    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        self.push_inherited_scope();
        visit::visit_expr_closure(self, closure);
        self.pop_scope();
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        let previous = self.enter_attrs(&arm.attrs);
        self.push_inherited_scope();
        visit::visit_arm(self, arm);
        self.pop_scope();
        self.configurations = previous;
    }

    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.push_inherited_scope();
        visit::visit_expr_for_loop(self, expression);
        self.pop_scope();
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if item.attrs.iter().any(|attr| attribute_sets_path(&attr.meta)) {
            self.analysis
                .path_attributes
                .insert((item.ident.to_string(), item.ident.span().into()));
        }
        let previous = self.enter_attrs(&item.attrs);
        self.analysis.modules.insert(ModuleDecl {
            name: item.ident.to_string(),
            scope: self.module_stack.join("::"),
            inline: item.content.is_some(),
            depth: self.module_depth,
            location: item.ident.span().into(),
            configurations: self.configurations,
        });
        // A module is a namespace, not an item: `scoped_items` excludes it.
        self.analysis.declared_items.insert(item.ident.to_string());
        if item.content.is_some() {
            self.module_depth += 1;
            self.module_stack.push(item.ident.to_string());
            self.lexical_scopes.push(self.module_stack.join("::"));
        }
        visit::visit_item_mod(self, item);
        if item.content.is_some() {
            self.lexical_scopes.pop();
            self.module_stack.pop();
            self.module_depth -= 1;
        }
        self.configurations = previous;
    }

    fn visit_item_extern_crate(&mut self, item: &'ast ItemExternCrate) {
        let name = item.rename.as_ref().map_or(&item.ident, |(_, rename)| rename);
        self.analysis.extern_crates.insert(name.to_string(), name.span().into());
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let previous = self.enter_attrs(&item.attrs);
        let kind = if visibility_exceeds_module(&item.vis) {
            EdgeKind::Reexport
        } else {
            EdgeKind::Import
        };
        let mut use_edges = BTreeSet::new();
        let source_scope = self.module_stack.join("::");
        let lexical_scope = self.lexical_scope();
        UseCollector {
            kind,
            edges: &mut use_edges,
            globs: &mut self.analysis.globs,
            bindings: &mut self.analysis.bindings,
            configurations: self.configurations,
            source_scope: &source_scope,
            lexical_scope: &lexical_scope,
            source_item: self.item_stack.last().map_or("<module>", String::as_str),
        }
        .collect(&item.tree, &mut Vec::new());
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
        visit::visit_item_trait(self, item);
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        let previous = self.enter_attrs(&item.attrs);
        let owner = self.item_stack.last().cloned().unwrap_or_default();
        self.item_stack.push(format!("{owner}::{}", item.sig.ident));
        self.enter_function(&item.sig.inputs, None);
        visit::visit_trait_item_fn(self, item);
        self.pop_scope();
        self.item_stack.pop();
        self.configurations = previous;
    }

    fn visit_item_type(&mut self, item: &'ast ItemType) {
        let previous = self.enter_attrs(&item.attrs);
        self.declared(item.ident.to_string());
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
        self.enter_function(&item.sig.inputs, None);
        visit::visit_item_fn(self, item);
        self.pop_scope();
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
        }
        if let Some((_, trait_path, _)) = &item.trait_ {
            self.record_trait_path(trait_path);
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
        let owner = self.impl_owners.last().cloned().unwrap_or_else(|| "<impl>".to_string());
        self.item_stack.push(format!("{owner}::{}", item.sig.ident));
        self.enter_function(&item.sig.inputs, Some(owner.as_str()));
        visit::visit_impl_item_fn(self, item);
        self.pop_scope();
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
            if let Some(name) = &item.ident {
                self.record_local_macro(item);
                self.local_macros.insert(name.to_string());
            }
            return;
        }
        if let (Some(name), Some(TokenTree::Ident(first))) =
            (item.mac.path.get_ident(), item.mac.tokens.clone().into_iter().next())
        {
            self.item_macro_invocations.push((
                name.to_string(),
                first.to_string(),
                self.module_stack.join("::"),
            ));
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
            && self.impl_owners.last().is_none_or(|owner| owner != "Engine")
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
                        || self.block_names.iter().any(|names| names.contains(name))
                        || self
                            .analysis
                            .bindings
                            .iter()
                            .any(|binding| binding.scope.is_empty() && binding.name == *name)
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

fn expression_attrs(expression: &Expr) -> &[Attribute] {
    match expression {
        Expr::Array(e) => &e.attrs,
        Expr::Assign(e) => &e.attrs,
        Expr::Async(e) => &e.attrs,
        Expr::Await(e) => &e.attrs,
        Expr::Binary(e) => &e.attrs,
        Expr::Block(e) => &e.attrs,
        Expr::Break(e) => &e.attrs,
        Expr::Call(e) => &e.attrs,
        Expr::Cast(e) => &e.attrs,
        Expr::Closure(e) => &e.attrs,
        Expr::Const(e) => &e.attrs,
        Expr::Continue(e) => &e.attrs,
        Expr::Field(e) => &e.attrs,
        Expr::ForLoop(e) => &e.attrs,
        Expr::Group(e) => &e.attrs,
        Expr::If(e) => &e.attrs,
        Expr::Index(e) => &e.attrs,
        Expr::Infer(e) => &e.attrs,
        Expr::Let(e) => &e.attrs,
        Expr::Lit(e) => &e.attrs,
        Expr::Loop(e) => &e.attrs,
        Expr::Macro(e) => &e.attrs,
        Expr::Match(e) => &e.attrs,
        Expr::MethodCall(e) => &e.attrs,
        Expr::Paren(e) => &e.attrs,
        Expr::Path(e) => &e.attrs,
        Expr::Range(e) => &e.attrs,
        Expr::RawAddr(e) => &e.attrs,
        Expr::Reference(e) => &e.attrs,
        Expr::Repeat(e) => &e.attrs,
        Expr::Return(e) => &e.attrs,
        Expr::Struct(e) => &e.attrs,
        Expr::Try(e) => &e.attrs,
        Expr::TryBlock(e) => &e.attrs,
        Expr::Tuple(e) => &e.attrs,
        Expr::Unary(e) => &e.attrs,
        Expr::Unsafe(e) => &e.attrs,
        Expr::While(e) => &e.attrs,
        Expr::Yield(e) => &e.attrs,
        _ => &[],
    }
}

/// Every token of a stream, groups flattened.
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

/// Walks one `use` tree, recording its edges, globs and bindings.
struct UseCollector<'a> {
    kind: EdgeKind,
    edges: &'a mut BTreeSet<Edge>,
    globs: &'a mut BTreeSet<Location>,
    bindings: &'a mut BTreeSet<UseBinding>,
    configurations: ConfigSet,
    /// The inline module the `use` belongs to.
    source_scope: &'a str,
    /// The scope the `use` binds in: its inline module, or the block it is
    /// written in.
    lexical_scope: &'a str,
    source_item: &'a str,
}

impl UseCollector<'_> {
    fn collect(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.collect(&path.tree, prefix);
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
                self.record(binding, target.join("::"), name.ident.span());
            }
            syn::UseTree::Rename(rename) => {
                let mut target = prefix.clone();
                if rename.ident != "self" {
                    target.push(rename.ident.to_string());
                }
                self.record(rename.rename.to_string(), target.join("::"), rename.ident.span());
            }
            syn::UseTree::Glob(glob) => {
                let location = Location::from(glob.star_token.span);
                self.globs.insert(location);
                let mut target = prefix.clone();
                target.push("*".to_string());
                self.edge(target.join("::"), "*", location);
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.collect(item, prefix);
                }
            }
        }
    }

    fn record(&mut self, binding: String, target: String, span: Span) {
        self.bindings.insert(UseBinding {
            scope: self.lexical_scope.to_string(),
            name: binding.clone(),
            target: target.clone(),
            location: span.into(),
            configurations: self.configurations,
        });
        self.edge(target, &binding, span.into());
    }

    fn edge(&mut self, target: String, binding: &str, location: Location) {
        self.edges.insert(Edge {
            kind: self.kind.clone(),
            target,
            source_scope: self.source_scope.to_string(),
            target_scope: self.lexical_scope.to_string(),
            source_item: use_source_item(self.source_item, &self.kind, binding),
            location,
            configurations: self.configurations,
        });
    }
}

/// The names a `use` tree binds (`*` for a glob).
fn use_tree_names(tree: &syn::UseTree, names: &mut BTreeSet<String>) {
    match tree {
        syn::UseTree::Path(path) => use_tree_names(&path.tree, names),
        syn::UseTree::Name(name) => {
            names.insert(name.ident.to_string());
        }
        syn::UseTree::Rename(rename) => {
            names.insert(rename.rename.to_string());
        }
        syn::UseTree::Glob(_) => {
            names.insert("*".to_string());
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                use_tree_names(item, names);
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
    fn engine_parameters_are_typed_and_aliases_expire_with_their_scope() {
        let analysis = analyze("fn f(db: &Engine) { { let alias = db; alias.search(); } } fn g(alias: String) { alias.search(); }").unwrap();
        assert!(analysis.edges.iter().any(|edge| edge.kind == EdgeKind::EngineMethod
            && edge.source_item == "f"
            && edge.target == "search"));
        assert!(!analysis
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::EngineMethod && edge.source_item == "g"));
    }

    #[test]
    fn external_receivers_do_not_manufacture_inferred_edges() {
        let analysis =
            analyze(r#"fn f(connection: rusqlite::Connection) { connection.execute("", []); }"#)
                .unwrap();
        assert!(!analysis.edges.iter().any(|edge| edge.target.starts_with(".<")));
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
        // Edge targets stay as written; the namespace resolver expands
        // `pool` through the recorded binding.
        assert!(analysis
            .edges
            .iter()
            .any(|edge| edge.kind == EdgeKind::Callable && edge.target == "pool::dispatch"));
        assert!(analysis.bindings.iter().any(|binding| binding.scope.is_empty()
            && binding.name == "pool"
            && binding.target == "crate::reader_pool"));
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
        let bindings = |analysis: &Analysis| {
            analysis
                .bindings
                .iter()
                .map(|binding| {
                    (binding.scope.clone(), binding.name.clone(), binding.target.clone())
                })
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(bindings(&one), bindings(&multi));
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
    fn inline_module_import_wins_over_a_same_file_function() {
        let analysis = analyze(
            "fn helper() {} mod inner { use crate::reader_pool::helper; fn call() { helper(); } }",
        )
        .expect("fixture parses");
        assert!(
            analysis.edges.iter().any(|edge| edge.kind == EdgeKind::Callable
                && edge.source_item == "call"
                && edge.target == "helper"
                && edge.target_scope == "inner"),
            "{:?}",
            analysis.edges
        );
        assert!(analysis.bindings.iter().any(|binding| binding.scope == "inner"
            && binding.name == "helper"
            && binding.target == "crate::reader_pool::helper"));
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
    fn items_are_scoped_and_file_level_alias_targets_keep_their_scope() {
        let analysis = analyze(
            "use crate::search::run;\nfn top() {}\nmod inner {\n    use super::*;\n    \
             pub(crate) use crate::fusion::fused as local;\n    struct Nested;\n    \
             fn call() { run(); local(); }\n}\n",
        )
        .expect("fixture parses");
        assert!(analysis.scoped_items.contains(&(String::new(), "top".to_string())));
        assert!(analysis.scoped_items.contains(&("inner".to_string(), "Nested".to_string())));
        assert!(!analysis.scoped_items.iter().any(|(_, name)| name == "inner"));
        let call = |target: &str| {
            analysis
                .edges
                .iter()
                .find(|edge| edge.source_item == "call" && edge.target == target)
                .unwrap_or_else(|| panic!("edge {target}: {:?}", analysis.edges))
                .clone()
        };
        // Both are written in `inner`; the resolver finds `run` through its
        // `use super::*` and `local` through its own binding.
        assert_eq!(call("run").target_scope, "inner");
        assert_eq!(call("local").target_scope, "inner");
    }

    #[test]
    fn extern_crate_declarations_are_recorded_by_bound_name() {
        let analysis = analyze("extern crate self as renamed; fn f() { extern crate alloc; }")
            .expect("fixture parses");
        assert_eq!(
            analysis.extern_crates.keys().cloned().collect::<BTreeSet<_>>(),
            BTreeSet::from(["alloc".to_string(), "renamed".to_string()])
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
