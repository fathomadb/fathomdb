use crate::{
    export::{stream, CheckedChunk, ExportScope},
    policy::{Classification, Policy},
    storage::Class,
};
use proptest::prelude::*;
use std::io::Write;
fn scope() -> ExportScope {
    ExportScope {
        database: "database-exact".into(),
        attachment: "session-exact".into(),
        class: Class::Node,
        cursor: "9223372036854775806".into(),
        revision: Some("revision\nexact".into()),
        identity: Some("logical:é".into()),
        collection: None,
        key: None,
        observation: serde_json::json!({"operation":"7","snapshot":"this_read_only"}),
    }
}
fn chunk(bytes: &[u8], inactive: bool, erased: bool, telemetry: bool) -> CheckedChunk {
    CheckedChunk {
        bytes: bytes.to_vec(),
        classification: Classification { inactive, erased, telemetry, known: true },
        available: true,
    }
}
#[test]
fn conditional_export_requires_erasure_intent_and_every_independent_gate() {
    for n in [false, true] {
        for e in [false, true] {
            for t in [false, true] {
                for inactive in [false, true] {
                    for erased in [false, true] {
                        for telemetry in [false, true] {
                            let policy =
                                Policy { no_show_inactive: n, show_erased: e, show_payloads: t };
                            let mut output = vec![];
                            let result = stream(
                                &mut output,
                                &scope(),
                                policy,
                                [chunk(b"exact\nvalue", inactive, erased, telemetry)],
                                || false,
                            );
                            let permitted = erased && e && (!inactive || !n) && (!telemetry || t);
                            assert_eq!(result.status == "complete", permitted);
                            if permitted {
                                let split = output.iter().position(|b| *b == b'\n').unwrap();
                                let header: serde_json::Value =
                                    serde_json::from_slice(&output[..split]).unwrap();
                                assert_eq!(header["scope"]["cursor"], "9223372036854775806");
                                assert_eq!(&output[split + 1..], b"exact\nvalue");
                                assert_eq!(result.payload_bytes, 11);
                            } else {
                                assert!(output.is_empty());
                            }
                        }
                    }
                }
            }
        }
    }
}
struct Failing {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for Failing {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.bytes.len() >= self.limit {
            return Err(std::io::Error::other("selected file write failure"));
        }
        let n = bytes.len().min(self.limit - self.bytes.len());
        self.bytes.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[test]
fn export_cancellation_disappearance_reclassification_and_write_failure_are_not_complete() {
    let policy = Policy { show_erased: true, no_show_inactive: true, show_payloads: false };
    let mut output = vec![];
    let r = stream(&mut output, &scope(), policy, [chunk(b"value", false, true, false)], || true);
    assert_eq!(r.status, "cancelled");
    assert!(output.is_empty());
    let mut absent = chunk(b"must not appear", false, true, false);
    absent.available = false;
    let mut output = vec![];
    let r = stream(
        &mut output,
        &scope(),
        policy,
        [chunk(b"first", false, true, false), absent],
        || false,
    );
    assert_eq!(r.status, "partial");
    assert_eq!(r.error, Some("unavailable"));
    assert!(output.ends_with(b"first"));
    assert_eq!(r.payload_bytes, 5);
    let mut output = vec![];
    let r = stream(
        &mut output,
        &scope(),
        policy,
        [chunk(b"first", false, true, false), chunk(b"withheld", true, true, false)],
        || false,
    );
    assert_eq!(r.status, "partial");
    assert_eq!(r.error, Some("refused"));
    assert!(output.ends_with(b"first"));
    let mut file = Failing { bytes: vec![], limit: 3 };
    let r = stream(&mut file, &scope(), policy, [chunk(b"value", false, true, false)], || false);
    assert_eq!(r.status, "partial");
    assert_eq!(r.bytes_written, 3);
}
proptest! {
 #![proptest_config(ProptestConfig::with_cases(40))]
 #[test]
 fn retained_stream_and_scope_codec_preserve_exact_unicode(content in ".{0,1000}",identity in ".{0,100}"){
  let mut scope=scope();scope.identity=Some(identity);let encoded=serde_json::to_vec(&scope).unwrap();let decoded:ExportScope=serde_json::from_slice(&encoded).unwrap();prop_assert_eq!(&decoded,&scope);
  let mut output=vec![];let result=stream(&mut output,&scope,Policy{show_erased:true,..Policy::default()},[chunk(content.as_bytes(),false,true,false)],||false);prop_assert_eq!(result.status,"complete");let split=output.iter().position(|b|*b==b'\n').unwrap();prop_assert_eq!(&output[split+1..],content.as_bytes());
 }
}
