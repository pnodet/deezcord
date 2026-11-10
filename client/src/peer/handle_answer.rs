use crate::peer::handle_ice_candidate::apply_buffered_candidates;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

pub async fn handle_answer(
    from_user: String,
    sdp: String,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ice_candidates: Arc<Mutex<HashMap<String, Vec<RTCIceCandidateInit>>>>,
) -> Result<()> {
    let answer = RTCSessionDescription::answer(sdp)?;

    let peer_connection = peer_connections.lock().await.get(&from_user).cloned();

    let Some(peer_connection) = peer_connection else {
        println!("Peer connection not found for user {:?}", from_user);
        return Ok(());
    };

    peer_connection.set_remote_description(answer).await?;

    apply_buffered_candidates(&from_user, &peer_connection, &ice_candidates).await
}
