//! Normal Default-BGE closed preparation; bounded source fixtures precede campaigns.

use fathomdb_embedder::EffectiveEmbedDevice;
use fathomdb_engine::{
    EmbedderChoice, Engine, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, SourceId,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Read};
use std::path::Path;

#[path = "../../../fathomdb-engine/examples/slice30/template_verify.rs"]
mod template_verify;

fn hash(path: &Path) -> String {
    let mut file = std::fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    digest.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn run(config: &Value, out: &Path) {
    let templates = config["templates"].as_array().unwrap();
    assert!(!templates.is_empty());
    std::fs::create_dir_all(out).unwrap();
    for (index, input) in templates.iter().enumerate() {
        let directory = out.join(format!("case-{index}"));
        std::fs::create_dir(&directory).unwrap();
        let store = directory.join("template.sqlite");
        let opening = std::time::Instant::now();
        let opened = Engine::open_with_choice(&store, EmbedderChoice::Default).unwrap();
        let open_ns = u64::try_from(opening.elapsed().as_nanos()).unwrap();
        let engine = &opened.engine;
        let resolution = opened.report.embedder_device_resolution.as_ref().unwrap();
        let device = match &resolution.effective_device {
            EffectiveEmbedDevice::Cuda(device) => device,
            EffectiveEmbedDevice::Cpu => panic!("normal G template requires approved CUDA"),
        };
        assert_eq!(resolution.selected_cuda_uuid.as_deref(), config["gpu_uuid"].as_str());
        assert_eq!(device.ordinal, 0);
        assert_eq!(opened.report.default_embedder.dimension, 384);
        let configuring = std::time::Instant::now();
        engine
            .configure_projections(
                &[ProjectionSpec {
                    name: "s30_dense".into(),
                    roles: [ProjectionRole::Searchable].into_iter().collect(),
                    fts: None,
                    vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                    source: None,
                }],
                &[],
            )
            .unwrap();
        let configure_ns = u64::try_from(configuring.elapsed().as_nanos()).unwrap();
        let seeding = std::time::Instant::now();
        let mut cohort = Vec::new();
        let mut cohorts = Vec::new();
        let mut ids = Vec::new();
        let mut written = 0;
        let mut commit = |cohort: &mut Vec<PreparedWrite>| {
            let started = std::time::Instant::now();
            let receipt = engine.write(cohort).unwrap();
            cohorts.push(
                json!({"first_input_index":written,"rows":cohort.len(),"cursor":receipt.cursor,
                "write_ns":u64::try_from(started.elapsed().as_nanos()).unwrap()}),
            );
            written += cohort.len();
            cohort.clear();
        };
        for line in std::io::BufReader::new(
            std::fs::File::open(input["records"].as_str().unwrap()).unwrap(),
        )
        .lines()
        {
            let record: Value = serde_json::from_str(&line.unwrap()).unwrap();
            let id = record["id"].as_str().unwrap();
            ids.push(id.to_owned());
            cohort.push(PreparedWrite::Node {
                kind: record["kind"].as_str().unwrap().into(),
                body: record["body"].as_str().unwrap().into(),
                logical_id: Some(id.into()),
                source_id: SourceId::new("test:slice30-g-template").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            });
            if cohort.len() == 64 {
                commit(&mut cohort);
            }
        }
        if !cohort.is_empty() {
            commit(&mut cohort);
        }
        assert!(!ids.is_empty());
        let seed_ns = u64::try_from(seeding.elapsed().as_nanos()).unwrap();
        let draining = std::time::Instant::now();
        engine.drain(5000).unwrap();
        let drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
        let pending = engine.read_embedding_readiness().unwrap().pending_count;
        assert_eq!(pending, 0);
        let closing = std::time::Instant::now();
        engine.close().unwrap();
        let close_ns = u64::try_from(closing.elapsed().as_nanos()).unwrap();
        let verifying = std::time::Instant::now();
        let coverage = template_verify::coverage(&store, &opened.report.default_embedder);
        assert!(
            coverage["rows"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["byte_end"].as_u64().is_some()),
            "pinned BGE successful spans must have known ends"
        );
        let verification_ns = u64::try_from(verifying.elapsed().as_nanos()).unwrap();
        let wal = std::path::PathBuf::from(format!("{}-wal", store.display()));
        assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
        let proof = json!({"api_complete":true,"input_ids":ids,"write_rows":written,"write_calls":cohorts.len(),"cohorts":cohorts,
            "open_ns":open_ns,"configure_ns":configure_ns,"seed_ns":seed_ns,"drain_ns":drain_ns,"close_ns":close_ns,"verification_ns":verification_ns,
            "pending":pending,"coverage":coverage,"closed_main_sha256":hash(&store),
            "selected_cuda_uuid":resolution.selected_cuda_uuid,"cuda_ordinal":device.ordinal,"cuda_compiled":resolution.cuda_compiled,
            "cuda_libraries":format!("{:?}",device.cuda_libraries),"pooling":"engine Default BGE Mean; governed projection preparation"});
        std::fs::write(
            directory.join("api-proof.json"),
            serde_json::to_vec_pretty(&proof).unwrap(),
        )
        .unwrap();
    }
}
