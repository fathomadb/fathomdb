//! Shared raw persistence: every16 records and phase boundaries, outside API timers.

use serde_json::{json, Value};
use std::path::Path;

const CHECKPOINT_RECORDS: u64 = 16;

struct RawStream {
    writer: std::io::BufWriter<std::fs::File>,
    records: u64,
    bytes: u64,
    pending: u64,
    checkpoints: u64,
}

pub(super) struct Recorder {
    streams: std::collections::BTreeMap<&'static str, RawStream>,
    elapsed_ns: u64,
    checkpoint_ns: u64,
}

impl Recorder {
    pub(super) fn new(out: &Path) -> Self {
        let streams = ["samples", "warmups", "batches", "marker-probes", "matches"]
            .into_iter()
            .map(|name| {
                (
                    name,
                    RawStream {
                        writer: std::io::BufWriter::new(
                            std::fs::File::create(out.join(format!("{name}.jsonl")))
                                .expect("create raw output"),
                        ),
                        records: 0,
                        bytes: 0,
                        pending: 0,
                        checkpoints: 0,
                    },
                )
            })
            .collect();
        Self { streams, elapsed_ns: 0, checkpoint_ns: 0 }
    }

    fn checkpoint(stream: &mut RawStream) -> u64 {
        use std::io::Write;
        if stream.pending == 0 {
            return 0;
        }
        let started = std::time::Instant::now();
        stream.writer.flush().expect("checkpoint raw records");
        stream.pending = 0;
        stream.checkpoints += 1;
        u64::try_from(started.elapsed().as_nanos()).unwrap()
    }

    pub(super) fn emit(&mut self, name: &str, row: &Value) {
        use std::io::Write;
        let started = std::time::Instant::now();
        let stream = self.streams.get_mut(name).expect("known raw stream");
        let mut encoded = serde_json::to_vec(row).expect("encode raw record");
        encoded.push(b'\n');
        stream.writer.write_all(&encoded).expect("append raw record");
        stream.records += 1;
        stream.bytes += u64::try_from(encoded.len()).unwrap();
        stream.pending += 1;
        if stream.pending == CHECKPOINT_RECORDS {
            self.checkpoint_ns += Self::checkpoint(stream);
        }
        self.elapsed_ns += u64::try_from(started.elapsed().as_nanos()).unwrap();
    }

    pub(super) fn flush_all(&mut self) {
        let started = std::time::Instant::now();
        for stream in self.streams.values_mut() {
            self.checkpoint_ns += Self::checkpoint(stream);
        }
        self.elapsed_ns += u64::try_from(started.elapsed().as_nanos()).unwrap();
    }

    pub(super) fn totals(&self) -> [u64; 5] {
        [
            self.streams.values().map(|s| s.records).sum(),
            self.streams.values().map(|s| s.bytes).sum(),
            self.streams.values().map(|s| s.checkpoints).sum(),
            self.elapsed_ns,
            self.checkpoint_ns,
        ]
    }

    pub(super) fn statistics_since(&self, before: [u64; 5]) -> Value {
        let after = self.totals();
        json!({"records":after[0]-before[0],"encoded_bytes":after[1]-before[1],
            "checkpoints":after[2]-before[2],"persistence_ns":after[3]-before[3],
            "checkpoint_ns":after[4]-before[4],"checkpoint_records":CHECKPOINT_RECORDS,
            "policy":"per stream every16 records and phase/pass boundaries; flush without fsync",
            "retention":"between completed checkpoints at most15 tail records per stream are uncheckpointed; an interrupted pending flush may be partial"})
    }
}
