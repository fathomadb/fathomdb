//! Private G drain adapter; source defaults remain the existing five seconds.

use serde_json::{json, Value};

pub(super) fn drain(
    _config: &Value,
    work: impl FnOnce(u64) -> Result<(), String>,
) -> Result<Value, String> {
    let start = super::budget_clock::monotonic_ns()?;
    let timeout_ms = 5000;
    work(timeout_ms)?;
    Ok(json!({"start_ns":start,"timeout_ms":timeout_ms,"drain_calls":1}))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use fathomdb_engine::{EmbedderChoice, Engine, InitialState, PreparedWrite, SourceId};
    use proptest::prelude::*;

    fn actual_drain(milliseconds: u64) {
        let directory = tempfile::Builder::new().prefix("s30-g-drain-").tempdir().unwrap().keep();
        let opened =
            Engine::open_with_choice(directory.join("store.sqlite"), EmbedderChoice::None).unwrap();
        let engine = &opened.engine;
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "actual drain control".into(),
                logical_id: Some("drain-control".into()),
                source_id: SourceId::new("test:s30-g-drain").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        let deadline =
            super::super::budget_clock::monotonic_ns().unwrap() + milliseconds * 1_000_000;
        let config = json!({"work_deadline_ns":deadline,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        let mut actual_timeout = None;
        let result = drain(&config, |timeout| {
            actual_timeout = Some(timeout);
            engine.drain(timeout).map_err(|error| format!("{error:?}"))
        });
        assert!(
            engine
                .search_text_only_filtered_with_snippets(
                    "actual",
                    None,
                    &fathomdb_engine::ReadView::default(),
                    10,
                    None
                )
                .unwrap()
                .results
                .len()
                == 1
        );
        engine.close().unwrap();
        println!(
            "real G drain {} actual_timeout={actual_timeout:?} result={result:?}",
            directory.display()
        );
        if milliseconds == 0 {
            assert!(result.is_err(), "expired G drain invoked actual engine instead of refusing");
            assert_eq!(actual_timeout, None);
        } else {
            let report = result.unwrap();
            let expected = (deadline - report["start_ns"].as_u64().unwrap()) / 1_000_000;
            assert_eq!(
                actual_timeout,
                Some(expected),
                "actual G drain received fresh five seconds instead of original remaining budget"
            );
            assert_eq!(report["timeout_ms"], expected);
        }
    }

    #[test]
    fn real_g_drain_uses_original_remaining_milliseconds() {
        actual_drain(1500);
    }
    #[test]
    fn expired_g_drain_never_invokes_engine() {
        actual_drain(0);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn remaining_g_drain_budget_is_never_renewed(milliseconds in 100u64..2000) {actual_drain(milliseconds);}
    }
}
