use fathomdb_embedder::CudaPoolObservation;
use serde_json::json;

fn encode(observation: CudaPoolObservation) -> String {
    let error = observation.error.map(|error| {
        json!({
            "domain": error.domain,
            "code": error.code.map(|value| value.to_string()),
            "message": error.message,
            "observed_context_id": error.observed_context_id.map(|value| value.to_string()),
            "operation": error.operation,
        })
    });
    json!({
        "schema": "fathomdb.cuda-pool-observation/v1",
        "status": observation.status,
        "pid": observation.pid,
        "device_ordinal": observation.device_ordinal,
        "uuid": observation.uuid,
        "context_id": observation.context_id.map(|value| value.to_string()),
        "pool_id": observation.pool_id,
        "live_models_before": observation.live_models_before.map(|value| value.to_string()),
        "live_models_after": observation.live_models_after.map(|value| value.to_string()),
        "used_current_bytes": observation.used_current_bytes.map(|value| value.to_string()),
        "reserved_current_bytes": observation.reserved_current_bytes.map(|value| value.to_string()),
        "configured_max_size_bytes": observation.configured_max_size_bytes.map(|value| value.to_string()),
        "release_threshold": observation.release_threshold,
        "reason": observation.reason,
        "error": error,
    }).to_string()
}

impl crate::Engine {
    /// Return one passive process-pool observation as lossless JSON.
    ///
    /// No engine/model is borrowed, so this works before open and after close.
    /// Current untracked/native-unavailable states contain null current counters.
    #[doc(hidden)]
    pub fn cuda_pool_observation_for_qualification() -> Result<String, crate::EngineError> {
        Ok(encode(fathomdb_embedder::observe_cuda_pool_for_qualification()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn wire_keeps_every_u64_identity_and_counter_as_a_decimal_string(value in any::<u64>()) {
            let mut observation = fathomdb_embedder::observe_cuda_pool_for_qualification();
            observation.status = "private";
            observation.context_id = Some(value);
            observation.used_current_bytes = Some(value);
            observation.reserved_current_bytes = Some(value);
            observation.configured_max_size_bytes = Some(value);
            let wire: serde_json::Value = serde_json::from_str(&encode(observation)).unwrap();
            let expected = value.to_string();
            for field in ["context_id", "used_current_bytes", "reserved_current_bytes", "configured_max_size_bytes"] {
                prop_assert_eq!(wire[field].as_str(), Some(expected.as_str()));
                prop_assert_eq!(wire[field].as_str().unwrap().parse::<u64>().unwrap(), value);
            }
        }
    }

    #[test]
    fn unavailable_counters_are_null_while_known_zero_metadata_stays_zero() {
        let mut observation = fathomdb_embedder::observe_cuda_pool_for_qualification();
        observation.status = "untracked";
        observation.device_ordinal = Some(0);
        observation.context_id = Some(u64::MAX);
        observation.live_models_before = Some(0);
        observation.live_models_after = Some(0);
        observation.release_threshold = Some("0");
        let wire: serde_json::Value = serde_json::from_str(&encode(observation)).unwrap();
        assert_eq!(wire["context_id"], "18446744073709551615");
        assert_eq!(wire["device_ordinal"], 0);
        assert_eq!(wire["live_models_before"], "0");
        assert_eq!(wire["release_threshold"], "0");
        assert!(wire["used_current_bytes"].is_null());
        assert!(wire["reserved_current_bytes"].is_null());
    }
}
