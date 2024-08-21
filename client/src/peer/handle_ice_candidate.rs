use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;

pub async fn handle_ice_candidate(
    from_user: String,
    candidate_str: String,
    ice_candidates: Arc<Mutex<HashMap<String, Vec<RTCIceCandidateInit>>>>,
) -> Result<()> {
    let candidate: RTCIceCandidateInit = serde_json::from_str(&candidate_str).unwrap();
    let mut ice_candidates = ice_candidates.lock().await;

    match ice_candidates.get_mut(&from_user) {
        Some(user_candidates) => {
            user_candidates.push(candidate);
        }

        None => {
            ice_candidates.insert(from_user, vec![candidate]);
        }
    }

    Ok(())
}
