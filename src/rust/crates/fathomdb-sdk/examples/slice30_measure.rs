//! Provider-free Slice30 direct-text measurement adapter. See the committed
//! dev/experiments/0.8.29-slice30-measurements/README.md for commands and scope.

#[cfg(test)]
use fathomdb_engine::{EmbedderChoice, Engine as CoreEngine};
#[cfg(test)]
use fathomdb_sdk::{Engine as SdkEngine, InitialState, OpenOptions, PreparedWrite, SourceId};
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
fn observe(_engine: &Adapter, _query: &str, _kind: Option<&str>) -> Value {
    json!({"ok": false, "ids": [], "error": "measurement adapter unimplemented"})
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
