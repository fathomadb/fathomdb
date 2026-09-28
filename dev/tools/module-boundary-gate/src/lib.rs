//! Syntax-bounded module dependency analysis for `fathomdb-engine`.

use std::collections::BTreeSet;

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
pub fn analyze_source(_source: &str) -> Result<Analysis, syn::Error> {
    Ok(Analysis::default())
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
        assert!(analysis.edges.contains(&Edge {
            kind: EdgeKind::EngineField,
            target: "reader_pool".to_string(),
        }));
        assert!(analysis.edges.contains(&Edge {
            kind: EdgeKind::EngineMethod,
            target: "helper".to_string(),
        }));
        assert!(analysis.edges.contains(&Edge {
            kind: EdgeKind::Callable,
            target: "pool::dispatch".to_string(),
        }));
    }

    #[test]
    fn grouped_aliases_and_callable_references_are_stable_across_formatting() {
        let one_line = "use crate::{filter::validate as check, read::load}; fn f(){ let x = check; load(); }";
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
