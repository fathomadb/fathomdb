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
}
