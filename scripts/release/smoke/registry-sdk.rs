use fathomdb_sdk::{read, Engine, InitialState, OpenOptions, PreparedWrite, SourceId};

const RECORD_ID: &str = "registry-smoke:record";
const BODY: &str = "registry SDK persisted café";

fn main() -> fathomdb_sdk::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("database path");
    let phase = args.next().expect("write or reopen");
    assert!(phase == "write" || phase == "reopen", "invalid smoke phase");
    assert!(
        phase != "reopen" || std::path::Path::new(&path).is_file(),
        "persisted store is missing"
    );
    let engine = Engine::open(path, OpenOptions::default())?;
    if phase == "write" {
        engine.write(&[PreparedWrite::Node {
            kind: "doc".to_owned(),
            body: BODY.to_owned(),
            source_id: SourceId::new("smoke:crates-sdk")?,
            logical_id: Some(RECORD_ID.to_owned()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])?;
    }
    let found = read::get(&engine, RECORD_ID, None)?.expect("persisted record is missing");
    assert_eq!(found.body, BODY);
    assert!(read::get(&engine, "registry-smoke:missing", None)?.is_none());
    engine.close()?;
    println!("sdk {phase}: pid={} exact body and missing record verified", std::process::id());
    Ok(())
}
