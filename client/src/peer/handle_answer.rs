use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

pub async fn handle_answer(
    from_user: String,
    sdp: String,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
) -> Result<()> {
    let peer_connections = peer_connections.lock().await;

    let answer = RTCSessionDescription::answer(sdp).unwrap();

    let peer_connection = peer_connections.get(&from_user);

    if peer_connection.is_none() {
        println!("Peer connection not found for user {:?}", from_user);
        return Ok(());
    }

    let peer_connection = peer_connection.unwrap();

    peer_connection
        .set_remote_description(answer)
        .await
        .unwrap();

    Ok(())
}
