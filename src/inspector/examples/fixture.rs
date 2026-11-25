#[path = "../tests/fixtures/mod.rs"]
#[cfg(target_os = "linux")]
mod fixtures;
#[cfg(target_os = "linux")]
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("fixture directory"));
    if std::env::args().nth(2).as_deref() == Some("comparison-policy") {
        println!("{}", fixtures::comparison_policy(&dir).display());
        return;
    }
    let path = fixtures::database(&dir);
    let opened =
        fathomdb_engine::Engine::open_with_choice(&path, fathomdb_engine::EmbedderChoice::None)
            .unwrap();
    opened.engine.write(&[fixtures::node("structured",r#"{"integer":9007199254740993123456789,"nested":{"null":null,"empty":"","html":"<img src=https://evil.invalid/x onerror=alert(1)>"},"array":[true,false]}"#)]).unwrap();
    opened.engine.close().unwrap();
    match std::env::args().nth(2).as_deref() {
        Some("catalog-caps") => fixtures::add_catalog_cap_cases(&path),
        Some("metadata") => fixtures::add_metadata(&path),
        Some("chunked") => fixtures::add_chunked(&path),
        Some("nullable") => fixtures::add_nullable(&path),
        Some("scale") => fixtures::add_scale(&path),
        Some("many-linked") => fixtures::add_many_related(&path),
        Some("lexical-native") => fixtures::add_lexical_native(&path),
        Some("lexical") => fixtures::add_lexical(&path),
        Some("comparison-envelope") => {
            fixtures::add_lexical_envelope(&path);
        }
        Some("comparison-expiry") => fixtures::add_comparison_expiry(&path),
        Some("graph-highdegree") => fixtures::add_graph_highdegree(&path),
        Some("graph-missing-prefix") => fixtures::add_graph_missing_prefix(&path),
        Some("graph-visual") => fixtures::add_graph_visual(&path),
        Some("graph-history") => fixtures::add_graph_history(&path),
        Some("properties") => fixtures::add_properties(&path),
        Some("properties-raw") => fixtures::add_properties_raw(&path),
        Some("ux-hub") => fixtures::add_ux_hub(&path),
        Some("graph") => fixtures::add_graph(&path),
        Some("linked") => fixtures::add_related(&path),
        Some("erasure") => fixtures::add_erasure_cases(&path),
        Some("scoped") => fixtures::add_scoped(&path),
        None => {}
        Some(_) => panic!("unknown fixture"),
    }

    println!("{}", path.display());
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("unsupported_platform: this Inspector build is qualified for Linux only");
    std::process::exit(1);
}
