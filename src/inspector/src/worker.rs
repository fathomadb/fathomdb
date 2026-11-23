use crate::{
    policy::Policy,
    process::{inherited_socket, receive, send, BUILD},
    session::Session,
    storage::{Class, Filter, Reader},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Initial {
    path: String,
    model_cache: Option<String>,
    model_view: Option<String>,
    policy: Policy,
    build: String,
    protocol: u32,
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Catalog {},
    Check {
        handle: Option<String>,
        #[serde(default)]
        handles: Vec<String>,
        #[serde(default)]
        metadata_handles: Vec<String>,
    },
    Page {
        class: Class,
        after: String,
        limit: usize,
        filter: Filter,
    },
    Sample {
        class: Class,
        after: String,
        filter: Filter,
    },
    Detail {
        handle: String,
        field: Option<String>,
    },
    Chunk {
        handle: String,
        field: String,
        offset: String,
        limit: usize,
    },
    Tree {
        handle: String,
        #[serde(default)]
        field: Option<String>,
        start: String,
        after: Option<String>,
    },
    GraphServing {
        handle: String,
        direction: crate::storage::graph::Direction,
        kind: Option<String>,
        depth: usize,
        #[serde(default)]
        as_of_write_cursor: Option<String>,
        #[serde(default)]
        as_of_epoch_ms: Option<String>,
    },
    Investigate {
        handle: String,
    },
    Graph {
        handle: String,
        direction: crate::storage::graph::Direction,
        kind: Option<String>,
        depth: usize,
        expanded: Vec<String>,
        #[serde(default)]
        as_of_write_cursor: Option<String>,
        #[serde(default)]
        as_of_epoch_ms: Option<String>,
    },
    Associations {
        handle: String,
    },
    Lexical {
        route: crate::storage::lexical::Route,
        query: String,
        property: Option<String>,
        #[serde(default)]
        filter: crate::storage::lexical::LexicalFilter,
        #[serde(default)]
        view: crate::storage::lexical::LexicalView,
        limit: usize,
    },
    Neighborhood {
        handle: String,
        vector_rowid: String,
        after: String,
        limit: usize,
    },
    Serving {
        query: String,
        limit: usize,
    },
    Vectors {
        handle: String,
    },
    Indexed {
        handle: String,
        key: Option<String>,
        offset: String,
        limit: usize,
    },
    Export {
        handle: String,
        destination: String,
    },
    Close {},
}
#[derive(Deserialize)]
#[serde(tag = "origin", rename_all = "snake_case", deny_unknown_fields)]
enum Message {
    PrepareExport {
        handle: String,
        destination: String,
    },
    ExportSink {
        nonce: String,
    },
    Read {
        request: Value,
    },
    #[cfg(feature = "fixture-control")]
    Fixture {
        control: FixtureControl,
    },
}
#[cfg(feature = "fixture-control")]
#[derive(Deserialize)]
#[serde(tag = "control", rename_all = "snake_case", deny_unknown_fields)]
enum FixtureControl {
    Transition { identity: String, deleted: bool },
    PauseNext { milliseconds: u64 },
}
#[cfg(feature = "fixture-control")]
fn fixture(
    reader: &mut Reader,
    pause: &mut Option<std::time::Duration>,
    control: FixtureControl,
) -> Result<Value, &'static str> {
    match control {
        FixtureControl::PauseNext { milliseconds } => {
            if milliseconds == 0 || milliseconds > 30000 {
                return Err("invalid_range");
            }
            *pause = Some(std::time::Duration::from_millis(milliseconds));
            return Ok(json!({"status":"complete","effects":"fixture_one_shot_owned_pause_armed"}));
        }
        FixtureControl::Transition { identity, deleted } => {
            if identity.is_empty() || identity.len() > 65536 {
                return Err("invalid_range");
            }
            reader
                .session
                .engine
                .transition(
                    &identity,
                    if deleted {
                        fathomdb_engine::LifecycleState::Deleted
                    } else {
                        fathomdb_engine::LifecycleState::Active
                    },
                    None,
                )
                .map_err(|_| "fixture_failed")?;
        }
    }
    Ok(json!({"status":"complete","effects":"fixture_owned_engine_transition"}))
}
fn dispatch(reader: &mut Reader, value: Value) -> Result<Value, &'static str> {
    match serde_json::from_value(value).map_err(|_| "unsupported_operation")? {
        Request::Catalog {} => reader.catalog(),
        Request::Check { handle, handles, metadata_handles } => {
            reader.check_many(handle.as_deref(), &handles, &metadata_handles)
        }
        Request::Page { class, after, limit, filter } => {
            reader.page(class, after.parse().map_err(|_| "invalid_range")?, limit, filter)
        }
        Request::Sample { class, after, filter } => {
            reader.sample(class, after.parse().map_err(|_| "invalid_range")?, filter)
        }
        Request::Detail { handle, field } => match field {
            Some(field) => reader.detail_field(&handle, &field),
            None => reader.detail(&handle),
        },
        Request::Chunk { handle, field, offset, limit } => {
            reader.chunk(&handle, &field, offset.parse().map_err(|_| "invalid_range")?, limit)
        }
        Request::Tree { handle, field, start, after } => reader.tree_field(
            &handle,
            field.as_deref(),
            start.parse().map_err(|_| "invalid_range")?,
            after.map(|v| v.parse().map_err(|_| "invalid_range")).transpose()?,
        ),
        Request::GraphServing {
            handle,
            direction,
            kind,
            depth,
            as_of_write_cursor,
            as_of_epoch_ms,
        } => {
            let as_of = crate::storage::graph::AsOf::parse(
                as_of_write_cursor.as_deref(),
                as_of_epoch_ms.as_deref(),
            )?;
            reader.graph_serving_as_of(&handle, direction, kind, depth, as_of)
        }
        Request::Investigate { handle } => reader.investigate(&handle),
        Request::Graph {
            handle,
            direction,
            kind,
            depth,
            expanded,
            as_of_write_cursor,
            as_of_epoch_ms,
        } => {
            let as_of = crate::storage::graph::AsOf::parse(
                as_of_write_cursor.as_deref(),
                as_of_epoch_ms.as_deref(),
            )?;
            reader.graph_as_of(&handle, direction, kind, depth, expanded, as_of)
        }
        Request::Associations { handle } => reader.associations(&handle),
        Request::Lexical { route, query, property, filter, view, limit } => {
            reader.lexical(crate::storage::lexical::Query {
                route,
                query,
                property,
                filter,
                view,
                limit,
            })
        }
        Request::Neighborhood { handle, vector_rowid, after, limit } => reader.neighborhood(
            &handle,
            vector_rowid.parse().map_err(|_| "invalid_range")?,
            after.parse().map_err(|_| "invalid_range")?,
            limit,
        ),
        Request::Serving { query, limit } => reader.serving(&query, limit),
        Request::Vectors { handle } => reader.vectors(&handle),
        Request::Indexed { handle, key, offset, limit } => reader.indexed(
            &handle,
            key.as_deref(),
            offset.parse().map_err(|_| "invalid_range")?,
            limit,
        ),
        Request::Export { handle, destination } => {
            reader.export_to(&handle, Path::new(&destination))
        }
        Request::Close {} => Err("close_required"),
    }
}
pub(crate) fn run() -> Result<(), &'static str> {
    let mut stream = inherited_socket()?;
    let initial: Initial =
        serde_json::from_value(receive(&mut stream)?).map_err(|_| "invalid_pair")?;
    if initial.build != BUILD || initial.protocol != 1 {
        return Err("invalid_pair");
    }
    let scratch = std::env::var_os("TMPDIR").ok_or("invalid_environment")?;
    let view = Path::new(initial.model_view.as_deref().ok_or("invalid_environment")?);
    let work = Path::new(&scratch);
    if work.canonicalize().map_err(|_| "invalid_environment")? != work
        || view.canonicalize().map_err(|_| "invalid_environment")? != view
        || work.parent() != view.parent()
        || work.file_name() != Some(std::ffi::OsStr::new("work"))
        || view.file_name() != Some(std::ffi::OsStr::new("view"))
    {
        return Err("invalid_environment");
    }
    let expected = std::collections::BTreeMap::from([
        ("TMPDIR".to_string(), scratch.to_string_lossy().to_string()),
        ("LANG".into(), "C.UTF-8".into()),
        ("LC_ALL".into(), "C.UTF-8".into()),
        ("FATHOMDB_EMBED_DEVICE".into(), "cpu".into()),
        ("RAYON_NUM_THREADS".into(), "1".into()),
        ("XDG_CACHE_HOME".into(), view.to_string_lossy().to_string()),
    ]);
    if std::env::vars().collect::<std::collections::BTreeMap<_, _>>() != expected {
        return Err("invalid_environment");
    }
    let admitted = match crate::admission::admit(Path::new(&initial.path)) {
        Ok(admitted) => admitted,
        Err(failure) => {
            send(&mut stream, &failure.json())?;
            return Err(failure.error);
        }
    };
    let mut source_cache = None;
    let snapshot = match (initial.model_cache.as_deref(), initial.model_view.as_deref()) {
        (Some(cache), Some(view))
            if cfg!(feature = "cached-cpu") && model::ready(&admitted.path) =>
        {
            let work = Path::new(&scratch);
            let view = Path::new(view);
            if work.parent() != view.parent()
                || work.file_name() != Some(std::ffi::OsStr::new("work"))
                || view.file_name() != Some(std::ffi::OsStr::new("view"))
            {
                return Err("invalid_environment");
            }
            source_cache = Path::new(cache).canonicalize().ok();
            match source_cache
                .as_ref()
                .ok_or("model_unavailable")
                .and_then(|cache| model::Snapshot::new(cache, view))
            {
                Ok(snapshot) => {
                    if model::confine(&[admitted.path.parent().ok_or("invalid_path")?, work], view)
                        .is_ok()
                    {
                        model::deny_dense_writes()?;
                        Some(snapshot)
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        }
        _ => None,
    };
    match Session::attach_mode(&admitted.path, initial.policy, snapshot.is_some()) {
        Err(failure) => {
            send(&mut stream, &failure.json())?;
            Err(failure.error)
        }
        Ok(mut session) => {
            session.private_root = work.parent().map(std::path::Path::to_path_buf);
            session.source_cache = source_cache;
            session.source_cache_hint =
                initial.model_cache.as_deref().map(std::path::PathBuf::from);
            let mut reader = Reader::new(session);
            send(
                &mut stream,
                &json!({"status":"complete","attachment":reader.attachment,"composition":if reader.session.cached_cpu{"sealed-cache-cpu-v1"}else{"cleared-env-v2"}}),
            )?;
            #[cfg(feature = "fixture-control")]
            let mut pause = None;
            let mut pending_export: Option<(String, String, String)> = None;
            loop {
                let message: Message =
                    serde_json::from_value(receive(&mut stream)?).map_err(|_| "invalid_frame")?;
                let request = match message {
                    Message::Read { request } => {
                        pending_export = None;
                        request
                    }
                    Message::PrepareExport { handle, destination } => {
                        pending_export = None;
                        let response =
                            match reader.authorize_export(&handle, Path::new(&destination)) {
                                Ok(mut response) => {
                                    let nonce = crate::storage::nonce();
                                    response["nonce"] = json!(nonce);
                                    pending_export = Some((
                                        nonce,
                                        handle,
                                        response["destination"]
                                            .as_str()
                                            .ok_or("invalid_destination")?
                                            .to_string(),
                                    ));
                                    response
                                }
                                Err(code) => crate::failure::Failure::read(code).json(),
                            };
                        send(&mut stream, &response)?;
                        continue;
                    }
                    Message::ExportSink { nonce } => {
                        let file = crate::process::receive_fd(&stream)?;
                        let response = match pending_export.take() {
                            Some((expected, handle, destination)) if nonce == expected => reader
                                .export_to_sink(&handle, Path::new(&destination), Some(file))
                                .unwrap_or_else(|code| crate::failure::Failure::read(code).json()),
                            _ => crate::failure::Failure::read("invalid_sink").json(),
                        };
                        send(&mut stream, &response)?;
                        continue;
                    }
                    #[cfg(feature = "fixture-control")]
                    Message::Fixture { control } => {
                        let response = fixture(&mut reader, &mut pause, control)
                            .unwrap_or_else(|code| crate::failure::Failure::read(code).json());
                        send(&mut stream, &response)?;
                        continue;
                    }
                };
                #[cfg(feature = "fixture-control")]
                if request != json!({"op":"close"}) {
                    if let Some(duration) = pause.take() {
                        std::thread::sleep(duration);
                    }
                }
                if request == json!({"op":"close"}) {
                    let result = reader.session.close();
                    send(
                        &mut stream,
                        &match result {
                            Ok(()) => json!({"status":"complete"}),
                            Err(code) => crate::failure::Failure::read(code).json(),
                        },
                    )?;
                    break;
                }
                let response = match dispatch(&mut reader, request) {
                    Ok(value) => value,
                    Err(code) => crate::failure::Failure::read(code).json(),
                };
                send(&mut stream, &response)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "../tests/integration/lexical.rs"]
mod lexical_tests;

#[cfg(test)]
#[path = "../tests/integration/vector.rs"]
mod vector_tests;

#[path = "model.rs"]
mod model;
