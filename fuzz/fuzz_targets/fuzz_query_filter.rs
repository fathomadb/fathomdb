#![no_main]

use std::collections::BTreeSet;
use std::sync::{Mutex, OnceLock};

use arbitrary::Unstructured;
use fathomdb_engine::{
    ComparisonOp, EmbedderChoice, Engine, Filter, FilterTerm, InitialState, Predicate,
    PreparedWrite, ProjectionRole, ProjectionSpec, PropertyComparisonOp, PropertyEntityClass,
    PropertyMap, PropertyPredicate, PropertyValue, ReadContextV1, ReadView, ScalarValue,
    SearchFilter, SourceId,
};
use fathomdb_query::{compile_text_query, compile_text_query_with, QueryStopWords, StopWordBase};
use libfuzzer_sys::fuzz_target;
use tempfile::TempDir;

struct QueryFixture {
    _dir: TempDir,
    engine: Engine,
}

fn fixture() -> &'static Mutex<QueryFixture> {
    static FIXTURE: OnceLock<Mutex<QueryFixture>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = TempDir::new().expect("tempdir for fuzz_query_filter");
        let path = dir.path().join("query_fuzz.sqlite");
        let engine = Engine::open_with_choice(&path, EmbedderChoice::None)
            .expect("open fixture engine")
            .engine;
        let _ = engine.configure_projections(
            &[
                ProjectionSpec {
                    name: "priority".to_string(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "score".to_string(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
            ],
            &[],
        );
        let mut props = PropertyMap::new();
        props.insert("priority".to_string(), PropertyValue::Int(5));
        props.insert("score".to_string(), PropertyValue::Float(0.75));
        let src = SourceId::new("fuzz-src-1").expect("valid source_id");
        let _ = engine.write(&[PreparedWrite::Node {
            kind: "note".to_string(),
            body: r#"{"status":"open","priority":5,"tags":"alpha beta","created_at":1700000000}"#
                .to_string(),
            source_id: src,
            logical_id: Some("n1".to_string()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(props),
        }]);
        Mutex::new(QueryFixture { _dir: dir, engine })
    })
}

fn arb_short_str(u: &mut Unstructured<'_>, max_len: usize) -> String {
    let raw: String = u.arbitrary().unwrap_or_default();
    raw.chars().take(max_len).collect()
}

fn arb_path(u: &mut Unstructured<'_>) -> String {
    match u.int_in_range(0..=9).unwrap_or(0) {
        0 => "$.status".to_string(),
        1 => "$.priority".to_string(),
        2 => "$.tags".to_string(),
        3 => "$.kind".to_string(),
        4 => "$.created_at".to_string(),
        5 => "$.action_kind".to_string(),
        6 => "$.status') OR 1=1--".to_string(),
        7 => "".to_string(),
        8 => "$.\0invalid".to_string(),
        _ => arb_short_str(u, 64),
    }
}

fn arb_scalar(u: &mut Unstructured<'_>) -> ScalarValue {
    match u.int_in_range(0..=2).unwrap_or(0) {
        0 => ScalarValue::Text(arb_short_str(u, 64)),
        1 => ScalarValue::Integer(u.arbitrary().unwrap_or(0)),
        _ => ScalarValue::Bool(u.arbitrary().unwrap_or(false)),
    }
}

fn arb_cmp(u: &mut Unstructured<'_>) -> ComparisonOp {
    match u.int_in_range(0..=3).unwrap_or(0) {
        0 => ComparisonOp::Gt,
        1 => ComparisonOp::Gte,
        2 => ComparisonOp::Lt,
        _ => ComparisonOp::Lte,
    }
}

fn arb_prop_pred(u: &mut Unstructured<'_>) -> PropertyPredicate {
    let key = match u.int_in_range(0..=3).unwrap_or(0) {
        0 => "priority".to_string(),
        1 => "score".to_string(),
        2 => "missing".to_string(),
        _ => arb_short_str(u, 32),
    };
    let op = match u.int_in_range(0..=5).unwrap_or(0) {
        0 => PropertyComparisonOp::Eq,
        1 => PropertyComparisonOp::Ne,
        2 => PropertyComparisonOp::Lt,
        3 => PropertyComparisonOp::Lte,
        4 => PropertyComparisonOp::Gt,
        _ => PropertyComparisonOp::Gte,
    };
    let value = match u.int_in_range(0..=3).unwrap_or(0) {
        0 => PropertyValue::String(arb_short_str(u, 32)),
        1 => PropertyValue::Int(u.arbitrary().unwrap_or(0)),
        2 => PropertyValue::Float(u.arbitrary().unwrap_or(0.0)),
        _ => PropertyValue::Bool(u.arbitrary().unwrap_or(false)),
    };
    PropertyPredicate { key, op, value }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // 1. Always fuzz the FTS5 text query compiler directly on raw UTF-8/lossy strings.
    let raw_query = String::from_utf8_lossy(data);
    let compiled = compile_text_query(raw_query.as_ref());
    for term in &compiled.terms {
        assert!(term.starts_with('"') && term.ends_with('"'));
    }

    let mut u = Unstructured::new(data);
    let mode = u.int_in_range(0..=2).unwrap_or(0);

    // 2. Fuzz QueryStopWords validation + compile_text_query_with.
    let base =
        if u.arbitrary().unwrap_or(false) { StopWordBase::Fathomdb } else { StopWordBase::None };
    let extra_len = u.int_in_range(0..=8).unwrap_or(0);
    let keep_len = u.int_in_range(0..=8).unwrap_or(0);
    let mut extra = Vec::with_capacity(extra_len);
    for _ in 0..extra_len {
        extra.push(arb_short_str(&mut u, 72));
    }
    let mut keep = Vec::with_capacity(keep_len);
    for _ in 0..keep_len {
        keep.push(arb_short_str(&mut u, 72));
    }
    let stop_cfg = QueryStopWords { base, extra, keep };
    if let Ok(stop_set) = stop_cfg.validate() {
        let q = arb_short_str(&mut u, 128);
        let compiled_custom = compile_text_query_with(q, &stop_set);
        for term in &compiled_custom.terms {
            assert!(term.starts_with('"') && term.ends_with('"'));
        }
    }

    // 3. Construct Filter / SearchFilter / Predicate combinations.
    let term_count = u.int_in_range(0..=6).unwrap_or(0);
    let mut filter = Filter::default();
    let mut predicates = Vec::new();
    for _ in 0..term_count {
        match u.int_in_range(0..=6).unwrap_or(0) {
            0 => filter.terms.push(FilterTerm::SourceType(arb_short_str(&mut u, 32))),
            1 => filter.terms.push(FilterTerm::Kind(arb_short_str(&mut u, 32))),
            2 => filter.terms.push(FilterTerm::CreatedAfter(u.arbitrary().unwrap_or(0))),
            3 => filter.terms.push(FilterTerm::Status(arb_short_str(&mut u, 32))),
            4 => {
                if let Ok(pred) = Predicate::json_path_eq(arb_path(&mut u), arb_scalar(&mut u)) {
                    predicates.push(pred.clone());
                    filter.terms.push(FilterTerm::Json(pred));
                }
            }
            5 => {
                if let Ok(pred) = Predicate::json_path_compare(
                    arb_path(&mut u),
                    arb_cmp(&mut u),
                    arb_scalar(&mut u),
                ) {
                    predicates.push(pred.clone());
                    filter.terms.push(FilterTerm::Json(pred));
                }
            }
            _ => filter.terms.push(FilterTerm::Property(arb_prop_pred(&mut u))),
        }
    }

    let mut search_filter = SearchFilter::default();
    if u.arbitrary().unwrap_or(false) {
        search_filter.source_type = Some(arb_short_str(&mut u, 32));
    }
    if u.arbitrary().unwrap_or(false) {
        search_filter.kind = Some(arb_short_str(&mut u, 32));
    }
    if u.arbitrary().unwrap_or(false) {
        search_filter.created_after = Some(u.arbitrary().unwrap_or(0));
    }
    if u.arbitrary().unwrap_or(false) {
        search_filter.status = Some(arb_short_str(&mut u, 32));
    }
    let attr_count = u.int_in_range(0..=4).unwrap_or(0);
    for _ in 0..attr_count {
        search_filter.attributes.push((arb_short_str(&mut u, 32), arb_short_str(&mut u, 32)));
    }
    let prop_count = u.int_in_range(0..=4).unwrap_or(0);
    for _ in 0..prop_count {
        search_filter.property_predicates.push(arb_prop_pred(&mut u));
    }

    let mut view = ReadView::default();
    view.include_superseded = u.arbitrary().unwrap_or(false);
    view.include_inactive = u.arbitrary().unwrap_or(false);
    view.include_out_of_window = u.arbitrary().unwrap_or(false);
    view.valid_as_of = if u.arbitrary().unwrap_or(false) {
        Some(u.arbitrary().unwrap_or(1_700_000_000))
    } else {
        None
    };

    let _ = ReadContextV1::new(view.clone(), search_filter.clone());

    // 4. Lower and execute against the live cached Schema 39 SQLite engine.
    let guard = fixture().lock().unwrap_or_else(|p| p.into_inner());
    let kind =
        if u.arbitrary().unwrap_or(true) { "note".to_string() } else { arb_short_str(&mut u, 32) };
    let limit = u.int_in_range(0..=120).unwrap_or(10);

    match mode {
        0 => {
            let _ = guard.engine.read_list(&kind, &predicates, limit, &view);
            let _ = guard.engine.read_list_filter(&kind, &filter, limit, &view);
        }
        1 => {
            let query = arb_short_str(&mut u, 160);
            let _ = guard.engine.search_text_only(&query);
            let _ = guard.engine.search_filtered(&query, Some(search_filter));
        }
        _ => {
            if let Ok(ctx) = ReadContextV1::new(view, search_filter) {
                let _ = guard.engine.freeze_read_context(&ctx);
            }
        }
    }
});
