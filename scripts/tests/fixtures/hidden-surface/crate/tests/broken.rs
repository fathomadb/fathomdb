#[cfg(feature = "hooks")]
compile_error!("broken only with hooks");

#[test]
fn broken() {}
