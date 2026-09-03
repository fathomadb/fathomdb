//! Bounded property checks for existing Engine durability and identity contracts.

use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use tempfile::TempDir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(12))]

    #[test]
    fn written_record_identity_survives_reopen(
        token in "[a-z]{4,12}",
        logical_suffix in "[a-z0-9]{4,12}",
    ) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join(format!("identity{SQLITE_SUFFIX}"));
        let body = format!("property durable {token}");
        let logical_id = format!("property-{logical_suffix}");
        let source_text = format!("property:{logical_suffix}");
        let source = SourceId::new(source_text.clone()).expect("source id");

        let opened = Engine::open(&path).expect("open for write");
        opened.engine.write(&[PreparedWrite::Node {
            kind: "doc".to_string(),
            body: body.clone(),
            source_id: source,
            logical_id: Some(logical_id.clone()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }]).expect("write");
        opened.engine.drain(5_000).expect("projection drain");
        let before = opened.engine.search(&token).expect("search before close");
        let before_hit = before.results.iter().find(|hit| hit.body == body).expect("written hit");
        prop_assert_eq!(before_hit.id.to_prefixed(), format!("l:{logical_id}"));
        let before_id = before_hit.id.clone();
        opened.engine.close().expect("close");

        let reopened = Engine::open(&path).expect("reopen");
        let after = reopened.engine.search(&token).expect("search after reopen");
        let after_hit = after.results.iter().find(|hit| hit.body == body).expect("reopened hit");
        prop_assert_eq!(&after_hit.id, &before_id);
        prop_assert_eq!(&after_hit.body, &body);
        prop_assert_eq!(after_hit.source_id.as_deref(), Some(source_text.as_str()));
        reopened.engine.close().expect("close reopened");
    }
}
