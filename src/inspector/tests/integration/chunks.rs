use crate::{
    fixtures::{database, node},
    policy::Policy,
    session::Session,
    storage::{Class, Filter, Reader, CHUNK},
};
use fathomdb_engine::{EmbedderChoice, Engine};
#[test]
fn bounded_chunks_copy_exact_unicode_large_numbers_and_large_identities() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let body = format!(
        "{}é🦀{}",
        "x".repeat(CHUNK - 1),
        r#"{"integer":9007199254740993123456789,"empty":"","null":null}"#.repeat(180000)
    );
    let identity = format!("{}é", "identity".repeat(1500));
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened.engine.write(&[node(&identity, &body)]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let page = r.page(Class::Node, 0, 200, Filter::default()).unwrap();
    let handle = page["items"].as_array().unwrap().last().unwrap()["handle"].as_str().unwrap();
    for (field, expected) in [("body", body.as_str()), ("logical_id", identity.as_str())] {
        let mut offset = 0;
        let mut actual = String::new();
        loop {
            let chunk = r.chunk(handle, field, offset, CHUNK).unwrap();
            let text = chunk["text"].as_str().unwrap();
            assert!(text.len() <= CHUNK);
            assert_eq!(chunk["offset"], offset.to_string());
            actual.push_str(text);
            match chunk["continuation"].as_str() {
                Some(n) => {
                    let next = n.parse().unwrap();
                    assert!(next > offset);
                    offset = next
                }
                None => break,
            }
        }
        assert_eq!(actual, expected);
    }
    assert!(r.chunk(handle, "body", CHUNK, CHUNK).is_err());
    assert!(r.chunk(handle, "body", 0, CHUNK + 1).is_err());
    assert!(r.chunk(handle, "sqlite_master", 0, 10).is_err());
    r.session.close().unwrap();
}

#[test]
fn identity_filter_matches_complete_stored_value_not_preview_or_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let prefix = "é".repeat(4096);
    let identity = format!("{prefix}exact-suffix");
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    opened.engine.write(&[node(&identity, "exact target")]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let exact = r
        .page(Class::Node, 0, 10, Filter { identity: Some(identity), ..Filter::default() })
        .unwrap();
    assert_eq!(
        exact["items"].as_array().unwrap().len(),
        1,
        "complete identity lookup must find its exact record"
    );
    let prefix_only =
        r.page(Class::Node, 0, 10, Filter { identity: Some(prefix), ..Filter::default() }).unwrap();
    assert!(
        prefix_only["items"].as_array().unwrap().is_empty(),
        "metadata preview is not an exact identity"
    );
    r.session.close().unwrap();
}

#[test]
fn full_metadata_fields_preserve_long_values_numeric_precision_and_null_empty_distinctions() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    crate::fixtures::add_metadata(&path);
    let mut r = Reader::new(Session::attach(&path, Policy::default()).unwrap());
    let nodes = r
        .page(Class::Node, 0, 10, Filter { identity: Some("metadata".into()), ..Filter::default() })
        .unwrap();
    let handle = nodes["items"][0]["handle"].as_str().unwrap();
    let mut text = String::new();
    let mut offset = 0;
    loop {
        let part = r
            .chunk(handle, "reason", offset, CHUNK)
            .expect("retained metadata uses bounded full-field access");
        text.push_str(part["text"].as_str().unwrap());
        if let Some(next) = part["continuation"].as_str() {
            offset = next.parse().unwrap()
        } else {
            break;
        }
    }
    assert_eq!(text, "retained reason é🦀".repeat(20000));
    assert_eq!(r.chunk(handle, "valid_from", 0, CHUNK).unwrap()["text"], "1");
    assert_eq!(r.chunk(handle, "valid_until", 0, CHUNK).unwrap()["state"], "null");
    assert!(r.chunk(handle, "confidence", 0, CHUNK).is_err());
    let edges = r
        .page(
            Class::Edge,
            0,
            10,
            Filter { identity: Some("metadata-edge".into()), ..Filter::default() },
        )
        .unwrap();
    let handle = edges["items"][0]["handle"].as_str().unwrap();
    let precise = r.chunk(handle, "confidence", 0, CHUNK).unwrap();
    assert_eq!(
        precise["text"].as_str().unwrap().parse::<f64>().unwrap().to_bits(),
        0.12345678901234566_f64.to_bits()
    );
    assert_eq!(r.chunk(handle, "extractor_model_id", 0, CHUNK).unwrap()["state"], "empty");
    assert_eq!(r.chunk(handle, "temporal_fallback", 0, CHUNK).unwrap()["text"], "1");
    r.session.close().unwrap();
}
