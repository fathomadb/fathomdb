//! Provider-free Slice30 direct-text measurement adapter. See the committed
//! dev/experiments/0.8.29-slice30-measurements/README.md for commands and scope.

#[cfg(test)]
use fathomdb_engine::{EmbedderChoice, Engine as CoreEngine, ReadView, SearchFilter};
#[cfg(test)]
use fathomdb_sdk::{
    Engine as SdkEngine, InitialState, OpenOptions, PreparedWrite, SourceId, TextSearchOptions,
};
#[cfg(test)]
use serde_json::{json, Value};
#[cfg(test)]
use std::path::Path;

#[cfg(test)]
enum Adapter {
    Core(Box<CoreEngine>),
    Sdk(Box<SdkEngine>),
}

#[cfg(test)]
impl Adapter {
    fn open(adapter: &str, path: &Path) -> Self {
        match adapter {
            "engine" => Self::Core(Box::new(
                CoreEngine::open_with_choice(path, EmbedderChoice::None)
                    .expect("open core without provider")
                    .engine,
            )),
            "sdk" => Self::Sdk(Box::new(
                SdkEngine::open(path, OpenOptions::default()).expect("open SDK without provider"),
            )),
            _ => panic!("unknown adapter"),
        }
    }

    fn write(&self, batch: &[PreparedWrite]) {
        match self {
            Self::Core(engine) => {
                engine.write(batch).expect("core write");
            }
            Self::Sdk(engine) => {
                engine.write(batch).expect("SDK write");
            }
        }
    }

    fn close(&self) {
        match self {
            Self::Core(engine) => engine.close().expect("core close"),
            Self::Sdk(engine) => engine.close().expect("SDK close"),
        }
    }
}

#[cfg(test)]
fn node(id: &str, kind: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new("test:slice30").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[cfg(test)]
fn observe(engine: &Adapter, query: &str, kind: Option<&str>) -> Value {
    let filter = kind.map(|kind| {
        let mut filter = SearchFilter::default();
        filter.kind = Some(kind.into());
        filter
    });
    let started = std::time::Instant::now();
    let result = match engine {
        Adapter::Core(engine) => engine
            .search_text_only_filtered_with_snippets(query, filter, &ReadView::default(), 10, None)
            .map_err(|error| format!("{error:?}")),
        Adapter::Sdk(engine) => {
            let mut options = TextSearchOptions::default();
            options.filter = filter;
            engine.search_text_only(query, options).map_err(|error| format!("{error:?}"))
        }
    };
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).expect("bounded sample duration");
    match result {
        Ok(result) => json!({"ok": true, "elapsed_ns": elapsed_ns,
            "ids": result.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>(),
            "bodies": result.results.iter().map(|hit| &hit.body).collect::<Vec<_>>(),
            "returned_hits": result.results.len(), "projection_cursor": result.projection_cursor,
            "soft_fallback": format!("{:?}", result.soft_fallback), "degradations": format!("{:?}", result.degradations),
            "search_snapshot": null, "snapshot_observation": "unavailable"}),
        Err(error) => json!({"ok": false, "elapsed_ns": elapsed_ns, "ids": [], "error": error}),
    }
}

#[cfg(test)]
struct Schedule {
    searches: Vec<Value>,
    batches: Vec<Value>,
    probes: Vec<Value>,
}

#[cfg(test)]
fn run_schedule(
    _engine: &Adapter,
    _queries: &[String],
    _samples: usize,
    _batch_size: usize,
    _searches_per_batch: usize,
) -> Schedule {
    Schedule { searches: vec![], batches: vec![], probes: vec![] }
}

fn main() {
    eprintln!("Slice30 cell command is not implemented yet");
    std::process::exit(2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_records_real_text_identity_across_reopen() {
        for name in ["engine", "sdk"] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("literal.sqlite");
            let engine = Adapter::open(name, &path);
            engine.write(&[
                node("alpha", "doc", "alphaonly evidence"),
                node("beta", "doc", "betaonly evidence"),
            ]);
            let result = observe(&engine, "alphaonly", None);
            assert_eq!(result["ids"], json!(["l:alpha"]), "{name}: {result}");
            assert_eq!(result["ok"], true);
            assert!(result["elapsed_ns"].as_u64().unwrap() > 0);
            engine.close();
            drop(engine);
            let reopened = Adapter::open(name, &path);
            assert_eq!(observe(&reopened, "betaonly", None)["ids"], json!(["l:beta"]));
            reopened.close();
        }
    }

    #[test]
    fn schedule_observes_commits_search_counts_and_current_marker_bodies() {
        let cases = [
            (1, 1, 4, vec![0, 1, 2, 3], 4, 3, "s30markercurrent3 revision3"),
            (10, 10, 20, vec![0, 10], 20, 9, "s30markercurrent9 revision1"),
            (1, 100, 200, vec![0, 100], 2, 1, "s30markercurrent1 revision1"),
        ];
        for (batch_size, ratio, samples, starts, probe_count, last_slot, expected_body) in cases {
            let directory = tempfile::tempdir().unwrap();
            let engine = Adapter::open("sdk", &directory.path().join("schedule.sqlite"));
            engine.write(&[node("natural", "doc", "naturalalpha")]);
            for slot in 0..10 {
                engine.write(&[node(&format!("s30marker-{slot}"), "note", "initial marker")]);
            }
            let result =
                run_schedule(&engine, &["naturalalpha".into()], samples, batch_size, ratio);
            assert_eq!(result.searches.len(), samples);
            assert!(result.searches.iter().all(|row| row["ids"] == json!(["l:natural"])));
            assert_eq!(
                result
                    .batches
                    .iter()
                    .map(|row| row["before_sample"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                starts
            );
            assert_eq!(result.probes.len(), probe_count);
            assert!(result.probes.iter().all(|row| row["ok"] == true));
            assert_eq!(
                observe(&engine, &format!("s30markercurrent{last_slot}"), Some("note"))["bodies"],
                json!([expected_body])
            );
            engine.close();
        }
    }
}
