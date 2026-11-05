use current_history_followups::current::CurrentStore;
use fathomdb_engine::{InitialState, PreparedWrite, SourceId};
use std::{sync::atomic::Ordering, time::Instant};
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1))
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn index(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }
    fn real(&mut self) -> f64 {
        (self.next() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = std::env::args().nth(1).expect("result dir");
    std::fs::create_dir_all(&root)?;
    let vocab = (0..1024)
        .map(|i| {
            format!(
                "{}{}{}{i:04}",
                (b'a' + ((i / 26 / 26) % 26) as u8) as char,
                (b'a' + ((i / 26) % 26) as u8) as char,
                (b'a' + (i % 26) as u8) as char
            )
        })
        .collect::<Vec<_>>();
    let mut cumulative = Vec::new();
    let mut sum = 0.;
    for k in 1..=1024 {
        sum += 1. / k as f64;
        cumulative.push(sum);
    }
    let mut rng = Rng::new(0x0AC0_12C0_12C0);
    let bodies = (0..10_000)
        .map(|_| {
            let n = 55 + rng.index(20);
            (0..n)
                .map(|_| {
                    let target = rng.real() * sum;
                    let at = cumulative
                        .binary_search_by(|v| v.partial_cmp(&target).unwrap())
                        .unwrap_or_else(|at| at.min(1023));
                    vocab[at].clone()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>();
    let band = &vocab[102..512];
    let mut rng = Rng::new(0x0AC0_120D_EC0D_E000);
    let queries = (0..1000).map(|_| band[rng.index(band.len())].clone()).collect::<Vec<_>>();
    for h in [0, 4, 16] {
        let path = std::path::Path::new(&root).join(format!("current-H{h}.sqlite"));
        let store = CurrentStore::open(&path)?;
        let seed = Instant::now();
        for version in 0..=h {
            for start in (0..10_000).step_by(4096) {
                let writes = (start..(start + 4096).min(10_000))
                    .map(|i| PreparedWrite::Node {
                        kind: "doc".into(),
                        body: if version == h {
                            bodies[i].clone()
                        } else {
                            format!("excludedhistory {version} {}", bodies[i])
                        },
                        logical_id: Some(format!("owner-{i}")),
                        source_id: SourceId::new("test:fixture").unwrap(),
                        state: InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                    })
                    .collect::<Vec<_>>();
                store.write(&writes)?;
            }
        }
        let publish = Instant::now();
        store.bootstrap(50)?;
        let publish_ms = publish.elapsed().as_secs_f64() * 1000.;
        for query in &queries {
            store.search(query, 50, 10)?;
        }
        let contacts = store.writer_contacts.load(Ordering::Relaxed);
        let mut times = Vec::new();
        let mut steps = Vec::new();
        let mut metadata_steps = Vec::new();
        let mut errors = Vec::new();
        for query in &queries {
            let t = Instant::now();
            match store.search(query, 50, 10) {
                Ok(result) => {
                    times.push(t.elapsed().as_secs_f64() * 1000.);
                    steps.push(result.vm_steps);
                    metadata_steps.push(result.metadata_vm_steps);
                }
                Err(e) => errors.push(
                    serde_json::json!({"error":e.to_string(),"ms":t.elapsed().as_secs_f64()*1000.}),
                ),
            }
        }
        let quantiles =
            current_history_followups::metrics::qualified_percentiles(&times, errors.len());
        println!(
            "{}",
            serde_json::json!({"history":h,"current":10000,"scheduled":1000,"success":times.len(),"errors":errors,"p50_ms":quantiles.map(|q|q.0),"p99_ms":quantiles.map(|q|q.1),"samples_ms":times,"vm_steps":steps,"metadata_vm_steps":metadata_steps,"healthy_writer_contacts":store.writer_contacts.load(Ordering::Relaxed)-contacts,"bootstrap_ms":publish_ms,"seed_and_queries_s":seed.elapsed().as_secs_f64(),"qualification":"SCOUT governed identities; not integrated AC076/081"})
        );
        let burst = Instant::now();
        store.with_writer(|e|e.execute_for_test("UPDATE canonical_nodes SET valid_until=100 WHERE superseded_at IS NULL; UPDATE _fathomdb_read_visibility_state SET generation=generation+1 WHERE singleton=1;")).unwrap();
        store.search(&queries[0], 50, 10)?;
        let boundary = Instant::now();
        let result = store.search(&queries[0], 100, 10)?;
        eprintln!(
            "{}",
            serde_json::json!({"history":h,"boundary_ms":boundary.elapsed().as_secs_f64()*1000.,"burst_setup_and_refresh_ms":burst.elapsed().as_secs_f64()*1000.,"winners_after":result.rows.len()})
        );
        store.close()?;
    }
    Ok(())
}
