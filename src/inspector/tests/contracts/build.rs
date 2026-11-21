#[test]
fn artifact_description_identifies_target_profile_features_and_compiler() {
    let d: serde_json::Value =
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/build-description.json")))
            .unwrap();
    let target = d["target"].as_str().expect("artifact target is required");
    assert!(target.starts_with(std::env::consts::ARCH));
    assert!(target.contains(std::env::consts::OS));
    assert_eq!(d["profile"], if cfg!(debug_assertions) { "debug" } else { "release" });
    assert!(d["rustc"].as_str().unwrap().starts_with("rustc "));
    assert_eq!(d["fixture_control"], cfg!(feature = "fixture-control"));
    assert_eq!(d["build"], crate::process::BUILD);
    assert_eq!(d["protocol"], 1);
}
