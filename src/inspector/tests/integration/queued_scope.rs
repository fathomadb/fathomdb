use super::Session;
use crate::policy::Policy;

#[test]
fn interrupted_queued_scope_read_is_budget_exhaustion_not_unknown_disclosure() {
    let dir = tempfile::tempdir().unwrap();
    let path = crate::fixtures::database(dir.path());
    let session = Session::attach(&path, Policy::default()).unwrap();
    let mut read = session.connection().unwrap();
    read.connection.progress_handler(1, Some(|| true)).unwrap();
    assert_eq!(read.queued_erasure_contains(1), Err("budget_exhausted"));
    drop(read);
    session.close().unwrap();
}
