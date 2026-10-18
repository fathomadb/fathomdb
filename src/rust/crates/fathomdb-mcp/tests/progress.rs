//! Progress notifications for one call reach the client in send order
//! (agent-access item 6). The raw client records server messages in wire
//! order; the rmcp client handler runs each notification on its own task,
//! so it cannot observe order.

mod common;

use std::time::{Duration, Instant};

use common::{Fixture, Harness, RawClient};
use serde_json::{json, Value};

fn progress_values(notifications: &[Value], token: &str) -> Vec<f64> {
    notifications
        .iter()
        .filter(|message| message["method"] == "notifications/progress")
        .filter(|message| message["params"]["progressToken"] == token)
        .map(|message| message["params"]["progress"].as_f64().expect("numeric progress"))
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn progress_notifications_arrive_in_order() {
    const BURST: u32 = 200;
    const CALLS: usize = 5;
    let fixture = Fixture::new();
    let harness = Harness::start(fixture.config(json!({}))).await;
    harness.adapter.seams().burst_progress("list_stores", BURST);
    let mut raw = RawClient::connect(&harness.adapter).await;
    for call in 0..CALLS {
        let token = format!("burst-{call}");
        let params =
            json!({"name": "list_stores", "arguments": {}, "_meta": {"progressToken": token}});
        let response = raw.request("tools/call", params).await;
        assert!(response.get("result").is_some(), "protocol error {response}");

        // Notifications may trail the response; keep reading until the burst
        // has arrived or the wait gives up.
        let deadline = Instant::now() + Duration::from_secs(10);
        while progress_values(&raw.notifications, &token).len() < BURST as usize
            && Instant::now() < deadline
        {
            raw.request("ping", json!({})).await;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let received = progress_values(&raw.notifications, &token);
        assert_eq!(received.len(), BURST as usize, "call {call}: every notification arrives");
        let inversion = received.windows(2).position(|pair| pair[0] >= pair[1]);
        assert!(
            inversion.is_none(),
            "call {call}: progress out of order at index {inversion:?}: {received:?}"
        );
    }
    harness.shutdown().await;
}
