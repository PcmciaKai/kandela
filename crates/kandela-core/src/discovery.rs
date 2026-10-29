//! Finding bridges on the local network.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::bridge::{self, BridgeInfo};

/// Browses mDNS for `_hue._tcp` services. Blocking.
fn mdns_addresses(timeout: Duration) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let Ok(daemon) = mdns_sd::ServiceDaemon::new() else {
        return found;
    };
    let Ok(rx) = daemon.browse("_hue._tcp.local.") else {
        return found;
    };
    let deadline = Instant::now() + timeout;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) => {
                found.extend(info.get_addresses_v4().into_iter().map(|a| a.to_string()));
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = daemon.shutdown();
    found
}

/// Asks Philips' discovery service which bridges share our public IP.
async fn cloud_addresses() -> BTreeSet<String> {
    crate::pinning::install_crypto_provider();
    let reply = async {
        reqwest::Client::new()
            .get("https://discovery.meethue.com/")
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await
    };
    match reply.await {
        Ok(Value::Array(items)) => items
            .iter()
            .filter_map(|i| i.get("internalipaddress")?.as_str().map(str::to_owned))
            .collect(),
        Ok(_) => BTreeSet::new(),
        Err(e) => {
            log::info!("cloud discovery failed: {e}");
            BTreeSet::new()
        }
    }
}

/// Finds bridges using mDNS and, as a fallback, the cloud discovery service.
pub async fn discover() -> Vec<BridgeInfo> {
    let mdns = tokio::task::spawn_blocking(|| mdns_addresses(Duration::from_secs(4)));
    let mut addresses = cloud_addresses().await;
    addresses.extend(mdns.await.unwrap_or_default());

    let mut bridges = Vec::new();
    for address in addresses {
        match bridge::bridge_info(&address).await {
            Ok(info) => bridges.push(info),
            Err(e) => log::info!("{address} did not answer as a bridge: {e}"),
        }
    }
    bridges.dedup_by(|a, b| !a.id.is_empty() && a.id == b.id);
    bridges
}
