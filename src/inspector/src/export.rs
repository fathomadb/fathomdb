use crate::{
    policy::{Classification, Policy},
    storage::Class,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExportScope {
    pub(crate) database: String,
    pub(crate) attachment: String,
    pub(crate) class: Class,
    pub(crate) cursor: String,
    pub(crate) revision: Option<String>,
    pub(crate) identity: Option<String>,
    pub(crate) collection: Option<String>,
    pub(crate) key: Option<String>,
    pub(crate) observation: serde_json::Value,
}
pub(crate) struct CheckedChunk {
    pub(crate) bytes: Vec<u8>,
    pub(crate) classification: Classification,
    pub(crate) available: bool,
}
#[derive(Debug)]
pub(crate) struct Outcome {
    pub(crate) status: &'static str,
    pub(crate) bytes_written: usize,
    pub(crate) payload_bytes: usize,
    pub(crate) error: Option<&'static str>,
}
struct Header(Vec<u8>);
impl Write for Header {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.len() + bytes.len() > crate::storage::RESPONSE {
            return Err(std::io::Error::other("header_byte_cap"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[derive(Serialize)]
struct Envelope<'a> {
    format: &'static str,
    scope: &'a ExportScope,
}
fn stop(outcome: &mut Outcome, code: &'static str) {
    outcome.status = if code == "cancelled" {
        "cancelled"
    } else if outcome.bytes_written > 0 {
        "partial"
    } else {
        match code {
            "refused" | "unknown_classification" => "refused",
            "unavailable" | "not_erasure_pending" => "unavailable",
            "budget_exhausted" => "budget_exhausted",
            _ => "error",
        }
    };
    outcome.error = Some(code);
}
fn write<W: Write>(
    writer: &mut W,
    mut bytes: &[u8],
    payload: bool,
    outcome: &mut Outcome,
    started: std::time::Instant,
    cancel: &mut impl FnMut() -> bool,
) -> Result<(), &'static str> {
    while !bytes.is_empty() {
        if cancel() {
            return Err("cancelled");
        }
        if started.elapsed() >= std::time::Duration::from_secs(30) {
            return Err("budget_exhausted");
        }
        let n = writer.write(bytes).map_err(|_| "write_failed")?;
        if n == 0 || n > bytes.len() {
            return Err("write_failed");
        }
        outcome.bytes_written += n;
        if payload {
            outcome.payload_bytes += n;
        }
        bytes = &bytes[n..];
    }
    Ok(())
}
pub(crate) fn stream<W: Write, I: IntoIterator<Item = CheckedChunk>>(
    writer: &mut W,
    scope: &ExportScope,
    policy: Policy,
    chunks: I,
    mut cancel: impl FnMut() -> bool,
) -> Outcome {
    let mut outcome = Outcome {
        status: "unavailable",
        bytes_written: 0,
        payload_bytes: 0,
        error: Some("unavailable"),
    };
    let started = std::time::Instant::now();
    let mut initialized = false;
    if cancel() {
        stop(&mut outcome, "cancelled");
        return outcome;
    }
    for chunk in chunks {
        let refusal = if !chunk.available {
            Some("unavailable")
        } else if !chunk.classification.known {
            Some("unknown_classification")
        } else if !chunk.classification.erased {
            Some("not_erasure_pending")
        } else if !policy.permits(chunk.classification) {
            Some("refused")
        } else if chunk.bytes.len() > crate::storage::CHUNK {
            Some("budget_exhausted")
        } else {
            None
        };
        if let Some(code) = refusal {
            stop(&mut outcome, code);
            return outcome;
        }
        if !initialized {
            let mut header = Header(Vec::new());
            if serde_json::to_writer(
                &mut header,
                &Envelope { format: "fathomdb-inspector-retained-value-v1", scope },
            )
            .is_err()
            {
                stop(&mut outcome, "budget_exhausted");
                return outcome;
            }
            header.0.push(b'\n');
            if let Err(code) = write(writer, &header.0, false, &mut outcome, started, &mut cancel) {
                stop(&mut outcome, code);
                return outcome;
            }
            initialized = true;
        }
        if let Err(code) = write(writer, &chunk.bytes, true, &mut outcome, started, &mut cancel) {
            stop(&mut outcome, code);
            return outcome;
        }
    }
    if initialized {
        if writer.flush().is_err() {
            stop(&mut outcome, "write_failed");
        } else {
            outcome.status = "complete";
            outcome.error = None;
        }
    }
    outcome
}
