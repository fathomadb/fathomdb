//! Syntax-bounded module dependency analysis for `fathomdb-engine`.

use std::collections::BTreeSet;

use syn::visit::{self, Visit};
use syn::{
    Expr, ExprField, ExprPath, ImplItem, ItemImpl, ItemMod, ItemStruct, ItemUse, Member, Type,
};

/// One dependency family extracted from Rust source.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EdgeKind {
    /// An explicit import or re-export.
    Import,
    /// A call or callable item reference.
    Callable,
    /// An access to an `Engine` field.
    EngineField,
    /// An `Engine` method call or callable reference.
    EngineMethod,
}

/// One source-level dependency edge.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Edge {
    /// Edge family.
    pub kind: EdgeKind,
    /// Semantic owner or referenced item path.
    pub target: String,
}

/// Analysis result for one module.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Analysis {
    /// Declared child modules.
    pub modules: BTreeSet<String>,
    /// Imported or referenced dependencies.
    pub edges: BTreeSet<Edge>,
    /// Whether a wildcard import was present.
    pub has_glob: bool,
    /// Fields declared on `Engine` in this source.
    pub engine_fields: BTreeSet<String>,
    /// Methods declared by `impl Engine` blocks.
    pub engine_methods: BTreeSet<String>,
}

/// Analyze one Rust source file under the gate's deliberately bounded grammar.
pub fn analyze_source(source: &str) -> Result<Analysis, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut visitor = Analyzer::default();
    visitor.visit_file(&file);
    Ok(visitor.analysis)
}

#[derive(Default)]
struct Analyzer {
    analysis: Analysis,
}

impl Analyzer {
    fn edge(&mut self, kind: EdgeKind, target: impl Into<String>) {
        self.analysis.edges.insert(Edge { kind, target: target.into() });
    }
}

impl<'ast> Visit<'ast> for Analyzer {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        self.analysis.modules.insert(item.ident.to_string());
        visit::visit_item_mod(self, item);
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        collect_use_tree(
            &item.tree,
            &mut Vec::new(),
            &mut self.analysis.edges,
            &mut self.analysis.has_glob,
        );
        visit::visit_item_use(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        if item.ident == "Engine" {
            for field in &item.fields {
                if let Some(ident) = &field.ident {
                    self.analysis.engine_fields.insert(ident.to_string());
                }
            }
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if type_last_ident(&item.self_ty).is_some_and(|ident| ident == "Engine") {
            for member in &item.items {
                if let ImplItem::Fn(method) = member {
                    self.analysis.engine_methods.insert(method.sig.ident.to_string());
                }
            }
        }
        visit::visit_item_impl(self, item);
    }

    fn visit_expr_field(&mut self, expression: &'ast ExprField) {
        if expression_base_is_self(&expression.base) {
            if let Member::Named(field) = &expression.member {
                self.edge(EdgeKind::EngineField, field.to_string());
            }
        }
        visit::visit_expr_field(self, expression);
    }

    fn visit_expr_path(&mut self, expression: &'ast ExprPath) {
        if expression.qself.is_none() && expression.path.segments.len() >= 2 {
            let segments = expression
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            if matches!(segments.first().map(String::as_str), Some("Self" | "Engine")) {
                if let Some(method) = segments.last() {
                    self.edge(EdgeKind::EngineMethod, method.clone());
                }
            } else {
                self.edge(EdgeKind::Callable, segments.join("::"));
            }
        }
        visit::visit_expr_path(self, expression);
    }
}

fn expression_base_is_self(expression: &Expr) -> bool {
    match expression {
        Expr::Path(path) => path.qself.is_none() && path.path.is_ident("self"),
        Expr::Paren(paren) => expression_base_is_self(&paren.expr),
        Expr::Reference(reference) => expression_base_is_self(&reference.expr),
        Expr::Unary(unary) => expression_base_is_self(&unary.expr),
        _ => false,
    }
}

fn type_last_ident(ty: &Type) -> Option<&syn::Ident> {
    let Type::Path(path) = ty else { return None };
    path.path.segments.last().map(|segment| &segment.ident)
}

fn collect_use_tree(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    edges: &mut BTreeSet<Edge>,
    has_glob: &mut bool,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree(&path.tree, prefix, edges, has_glob);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            let mut target = prefix.clone();
            target.push(name.ident.to_string());
            edges.insert(Edge { kind: EdgeKind::Import, target: target.join("::") });
        }
        syn::UseTree::Rename(rename) => {
            let mut target = prefix.clone();
            target.push(rename.ident.to_string());
            edges.insert(Edge { kind: EdgeKind::Import, target: target.join("::") });
        }
        syn::UseTree::Glob(_) => {
            *has_glob = true;
            let mut target = prefix.clone();
            target.push("*".to_string());
            edges.insert(Edge { kind: EdgeKind::Import, target: target.join("::") });
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree(item, prefix, edges, has_glob);
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
        assert_eq!(analysis.modules, BTreeSet::from(["child".to_string()]));
        assert!(analysis.has_glob);
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
            .contains(&Edge { kind: EdgeKind::EngineField, target: "reader_pool".to_string() }));
        assert!(analysis
            .edges
            .contains(&Edge { kind: EdgeKind::EngineMethod, target: "helper".to_string() }));
        assert!(analysis
            .edges
            .contains(&Edge { kind: EdgeKind::Callable, target: "pool::dispatch".to_string() }));
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
        assert_eq!(
            analyze_source(one_line).expect("one-line fixture"),
            analyze_source(multi_line).expect("multi-line fixture")
        );
    }
}
