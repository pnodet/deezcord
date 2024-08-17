use crate::commands::wait_for_ack::wait_for_ack;
use crate::commands::{ClientCommand, Command, CommandMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_OPUS};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};

use crate::socket::send_message;
use crate::{ConnectionState, User};

pub async fn handle_refresh(
    user_list: Vec<String>,
    curr_id: &str,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ws_stream: Arc<
        Mutex<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    >,
) {
    let mut peer_connections = peer_connections.lock().await;

    let user_list = user_list
        .iter()
        .filter(|other| other.as_str() != curr_id && !peer_connections.contains_key(other.as_str()))
        .collect::<Vec<_>>();

    if user_list.is_empty() {
        println!("No new users to connect to");
        return;
    }

    for other in user_list {
        if other == curr_id {
            continue;
        }

        if peer_connections.contains_key(other) {
            println!("Peer connection already exists for user {:?}", other);
            continue;
        }

        println!("Connecting to user {:?}", other);

        let peer_connection = create_peer_connection().await.unwrap();
        peer_connections.insert(other.clone(), Arc::clone(&peer_connection));

        println!("Peer connection created {:?}", peer_connection);

        let (ice_complete_tx, ice_complete_rx) = tokio::sync::oneshot::channel();
        let ice_complete_tx = Arc::new(Mutex::new(Some(ice_complete_tx)));

        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let ice_complete_tx = Arc::clone(&ice_complete_tx);
            Box::pin(async move {
                if candidate.is_none() {
                    if let Some(tx) = ice_complete_tx.lock().await.take() {
                        let _ = tx.send(());
                    }
                }
            })
        }));

        let offer = peer_connection.create_offer(None).await.unwrap();
        let mut gather_complete = peer_connection.gathering_complete_promise().await;
        peer_connection.set_local_description(offer).await.unwrap();
        let _ = gather_complete.recv().await;

        if let Some(local_desc) = peer_connection.local_description().await {
            let json_str = serde_json::to_string(&local_desc).unwrap();
            println!("{}", json_str);
        } else {
            println!("generate local_description failed!");
        }

        let _ = ice_complete_rx.await;

        let local_desc = peer_connection.local_description().await.unwrap();

        println!("");
        println!("Local desc {:?}", local_desc);
        println!("");

        let offer_message = &CommandMessage {
            user_id: curr_id.to_string(),
            command: Command::Client(ClientCommand::SendOffer(other.clone(), local_desc)),
        };

        send_message(ws_stream.clone(), offer_message)
            .await
            .unwrap();

        println!("");
        println!("Offer sent to user {:?}", other);

        let ws_stream_clone = ws_stream.clone();
        let user_id = curr_id.to_string();
        let user = other.clone();
        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let ws_stream_clone = ws_stream_clone.clone();
            let user_id = user_id.clone();
            let user = user.clone();
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    let candidate_str =
                        serde_json::to_string(&candidate.to_json().unwrap()).unwrap();
                    send_message(
                        ws_stream_clone.clone(),
                        &CommandMessage {
                            user_id,
                            command: Command::Client(ClientCommand::SendIceCandidate(
                                user,
                                candidate_str,
                            )),
                        },
                    )
                    .await
                    .unwrap();
                    wait_for_ack(ws_stream_clone.clone()).await.unwrap();
                }
            })
        }));
    }
}

pub async fn handle_offer(
    from_user: String,
    sdp: RTCSessionDescription,
    username: &str,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ws_stream: Arc<
        Mutex<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    >,
    users: Arc<Mutex<HashMap<String, User>>>,
) {
    let mut peer_connections = peer_connections.lock().await;
    let mut users = users.lock().await;

    let peer_connection = create_peer_connection().await.unwrap();
    peer_connections.insert(from_user.clone(), Arc::clone(&peer_connection));

    peer_connection.set_remote_description(sdp).await.unwrap();
    let answer = peer_connection.create_answer(None).await.unwrap();
    peer_connection.set_local_description(answer).await.unwrap();

    let (ice_complete_tx, ice_complete_rx) = tokio::sync::oneshot::channel();
    let ice_complete_tx = Arc::new(Mutex::new(Some(ice_complete_tx)));

    peer_connection.on_ice_candidate(Box::new(move |candidate| {
        let ice_complete_tx = Arc::clone(&ice_complete_tx);
        Box::pin(async move {
            if candidate.is_none() {
                if let Some(tx) = ice_complete_tx.lock().await.take() {
                    let _ = tx.send(());
                }
            }
        })
    }));

    let _ = ice_complete_rx.await;

    let local_desc = peer_connection.local_description().await.unwrap();
    send_message(
        ws_stream,
        &CommandMessage {
            user_id: username.to_string(),
            command: Command::Client(ClientCommand::SendAnswer(from_user.clone(), local_desc)),
        },
    )
    .await
    .unwrap();

    if let Some(user) = users.get_mut(&from_user) {
        user.state = ConnectionState::Connected;
    }
}

pub async fn handle_answer(
    from_user: String,
    sdp: RTCSessionDescription,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    users: Arc<Mutex<HashMap<String, User>>>,
) {
    let peer_connections = peer_connections.lock().await;
    let mut users = users.lock().await;

    if let Some(peer_connection) = peer_connections.get(&from_user) {
        peer_connection.set_remote_description(sdp).await.unwrap();
        if let Some(user) = users.get_mut(&from_user) {
            user.state = ConnectionState::Connected;
        }
    }
}

pub async fn handle_ice_candidate(
    from_user: String,
    candidate_str: String,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
) {
    let candidate: RTCIceCandidateInit = serde_json::from_str(&candidate_str).unwrap();
    let peer_connections = peer_connections.lock().await;

    if let Some(peer_connection) = peer_connections.get(&from_user) {
        peer_connection.add_ice_candidate(candidate).await.unwrap();
    }
}

pub async fn create_peer_connection() -> Result<Arc<RTCPeerConnection>, webrtc::Error> {
    let mut m = MediaEngine::default();
    m.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                ..Default::default()
            },
            payload_type: 120,
            ..Default::default()
        },
        RTPCodecType::Audio,
    )?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)?;

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec![
                "stun:stun.l.google.com:19302".to_owned(),
                "stun:stun1.l.google.com:19302".to_owned(),
                "stun:stun2.l.google.com:19302".to_owned(),
            ],
            ..Default::default()
        }],
        ..Default::default()
    };

    let peer_connection = Arc::new(api.new_peer_connection(config).await?);

    Ok(peer_connection)
}
