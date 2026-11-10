use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::RTCPeerConnection;

// Lock order: `peer_connections` before `ice_candidates`. Neither lock is held
// while this file awaits a webrtc call.

/// Adds a remote candidate to the peer connection for `from_user` if that connection
/// already has its remote description; otherwise buffers it until
/// `apply_buffered_candidates` runs after `set_remote_description`.
pub async fn handle_ice_candidate(
    from_user: String,
    candidate_str: String,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ice_candidates: Arc<Mutex<HashMap<String, Vec<RTCIceCandidateInit>>>>,
) -> Result<()> {
    let candidate: RTCIceCandidateInit = serde_json::from_str(&candidate_str)?;

    let peer_connection = peer_connections.lock().await.get(&from_user).cloned();

    if let Some(peer_connection) = peer_connection {
        if peer_connection.remote_description().await.is_some() {
            peer_connection.add_ice_candidate(candidate).await?;
            return Ok(());
        }
    }

    ice_candidates
        .lock()
        .await
        .entry(from_user)
        .or_default()
        .push(candidate);

    Ok(())
}

/// Adds every candidate buffered for `from_user` to `peer_connection` and clears the
/// buffer. Call right after `set_remote_description`.
pub async fn apply_buffered_candidates(
    from_user: &str,
    peer_connection: &RTCPeerConnection,
    ice_candidates: &Mutex<HashMap<String, Vec<RTCIceCandidateInit>>>,
) -> Result<()> {
    let buffered = ice_candidates.lock().await.remove(from_user);

    for candidate in buffered.unwrap_or_default() {
        peer_connection.add_ice_candidate(candidate).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use webrtc::api::APIBuilder;
    use webrtc::peer_connection::configuration::RTCConfiguration;

    const CANDIDATE: &str = r#"{"candidate":"candidate:1 1 udp 2130706431 127.0.0.1 50000 typ host","sdpMid":"0","sdpMLineIndex":0,"usernameFragment":null}"#;

    #[tokio::test]
    async fn candidates_wait_for_the_remote_description_then_apply_directly() {
        let api = APIBuilder::new().build();
        let offerer = api
            .new_peer_connection(RTCConfiguration::default())
            .await
            .unwrap();
        let answerer = Arc::new(
            api.new_peer_connection(RTCConfiguration::default())
                .await
                .unwrap(),
        );
        offerer.create_data_channel("data", None).await.unwrap();
        let offer = offerer.create_offer(None).await.unwrap();

        let peer_connections = Arc::new(Mutex::new(HashMap::new()));
        let ice_candidates = Arc::new(Mutex::new(HashMap::new()));
        let buffered = |map: &HashMap<String, Vec<RTCIceCandidateInit>>| {
            map.get("offerer").map_or(0, Vec::len)
        };

        // No peer connection for the sender yet: buffered.
        handle_ice_candidate(
            "offerer".into(),
            CANDIDATE.into(),
            peer_connections.clone(),
            ice_candidates.clone(),
        )
        .await
        .unwrap();
        assert_eq!(buffered(&*ice_candidates.lock().await), 1);

        // Peer connection exists but has no remote description: still buffered.
        peer_connections
            .lock()
            .await
            .insert("offerer".to_string(), answerer.clone());
        handle_ice_candidate(
            "offerer".into(),
            CANDIDATE.into(),
            peer_connections.clone(),
            ice_candidates.clone(),
        )
        .await
        .unwrap();
        assert_eq!(buffered(&*ice_candidates.lock().await), 2);

        // Remote description set: the buffer is applied and cleared.
        answerer.set_remote_description(offer).await.unwrap();
        apply_buffered_candidates("offerer", &answerer, &ice_candidates)
            .await
            .unwrap();
        assert!(ice_candidates.lock().await.is_empty());

        // Later candidates are added directly, not buffered.
        handle_ice_candidate(
            "offerer".into(),
            CANDIDATE.into(),
            peer_connections.clone(),
            ice_candidates.clone(),
        )
        .await
        .unwrap();
        assert!(ice_candidates.lock().await.is_empty());
    }
}
