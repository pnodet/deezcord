use crate::commands::{ClientCommand, Command, CommandMessage};
use crate::peer::create::create_peer_connection;
use crate::peer::handle_ice_candidate::apply_buffered_candidates;
use crate::socket::send::send_message;
use crate::socket::WsWriter;
use anyhow::Result;
use bytes::Bytes;
use std::collections::HashMap;
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::peer_connection::offer_answer_options::RTCAnswerOptions;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

pub async fn handle_offer(
    from_user: String,
    room_id: String,
    offer: RTCSessionDescription,
    username: &str,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ice_candidates: Arc<Mutex<HashMap<String, Vec<RTCIceCandidateInit>>>>,
    ws_stream: WsWriter,
    watch_tx: tokio::sync::watch::Sender<()>,
    audio_tx: broadcast::Sender<Bytes>,
) -> Result<()> {
    let mut stdout = stdout();

    write!(stdout, "\n\nhandling offer from\n{:?}", from_user).unwrap();
    stdout.flush().unwrap();

    let peer_connection = create_peer_connection(&audio_tx).await?;

    let _ = peer_connection
        .create_data_channel("data_2", None)
        .await
        .unwrap();

    let mut peer_connections = peer_connections.lock().await;

    peer_connections.insert(from_user.clone(), Arc::clone(&peer_connection));

    peer_connection.set_remote_description(offer).await?;

    apply_buffered_candidates(&from_user, &peer_connection, &ice_candidates).await?;

    peer_connection.on_peer_connection_state_change(Box::new(move |state| {
        println!("\n\rPeer connection state changed to {:?}", state);
        let _ = watch_tx.send(());
        Box::pin(async {})
    }));

    peer_connection.on_ice_connection_state_change(Box::new(
        move |state: RTCIceConnectionState| {
            println!("\n\rICE connection state changed to {:?}", state);
            Box::pin(async {})
        },
    ));

    peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_label = d.label().to_owned();
        let d_id = d.id();
        println!("DataChannel {d_label} {d_id}");
        Box::pin(async {})
    }));

    peer_connection.on_negotiation_needed(Box::new(move || {
        println!("\n\rNegotiation needed");
        Box::pin(async {})
    }));

    // Trickle our candidates to the offerer, like `connect_peer` does on its side.
    // Registered before `set_local_description`, which starts ICE gathering.
    let local_user_id = username.to_string();
    let offerer_id = from_user.clone();
    let candidate_room_id = room_id.clone();
    let candidate_ws_stream = Arc::clone(&ws_stream);

    peer_connection.on_ice_candidate(Box::new(move |candidate| {
        let local_user_id = local_user_id.clone();
        let offerer_id = offerer_id.clone();
        let room_id = candidate_room_id.clone();
        let ws_stream = Arc::clone(&candidate_ws_stream);

        Box::pin(async move {
            // `None` marks the end of gathering; the answerer has nothing to do then.
            let Some(candidate) = candidate else {
                return;
            };

            let candidate_str = serde_json::to_string(&candidate.to_json().unwrap()).unwrap();

            let ice_candidate_message = CommandMessage {
                user_id: local_user_id,
                command: Command::Client(ClientCommand::SendIceCandidate(
                    offerer_id,
                    room_id,
                    candidate_str,
                )),
            };

            if let Err(e) = send_message(ws_stream, &ice_candidate_message).await {
                eprintln!("\n\rFailed to send ICE candidate: {e}");
            }
        })
    }));

    let answer = peer_connection
        .create_answer(Some(RTCAnswerOptions {
            voice_activity_detection: true,
            ..Default::default()
        }))
        .await
        .unwrap();

    peer_connection
        .set_local_description(answer.clone())
        .await
        .unwrap();

    send_message(
        ws_stream,
        &CommandMessage {
            user_id: username.to_string(),
            command: Command::Client(ClientCommand::SendAnswer(
                from_user.clone(),
                room_id.clone(),
                answer.sdp,
            )),
        },
    )
    .await
    .unwrap();

    Ok(())
}
