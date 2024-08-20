use crate::commands::{ClientCommand, Command, CommandMessage};
use crate::socket::send::send_message;
use anyhow::Result;
use std::collections::HashMap;
use std::io::stdout;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::offer_answer_options::{RTCAnswerOptions, RTCOfferOptions};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::{math_rand_alpha, RTCPeerConnection};

pub async fn create_peer_connection() -> Result<Arc<RTCPeerConnection>, webrtc::Error> {
    let mut m = MediaEngine::default();
    m.register_default_codecs().unwrap();

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)?;

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec![
                "stun:localhost:3479".to_owned(),
                "stun:localhost:3478".to_owned(),
            ],
            ..Default::default()
        }],
        ..Default::default()
    };

    let peer_connection = Arc::new(api.new_peer_connection(config).await?);

    Ok(peer_connection)
}

pub async fn connect_to_room_user(
    user_id: String,
    other_id: String,
    room_id: String,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ws_stream: Arc<
        Mutex<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    >,
) {
    let peer_connection = create_peer_connection().await.unwrap();

    let data_channel = peer_connection
        .create_data_channel("data", None)
        .await
        .unwrap();

    let d1 = Arc::clone(&data_channel);

    data_channel.on_open(Box::new(move || {
					println!("Data channel '{}'-'{}' open. Random messages will now be sent to any connected DataChannels every 5 seconds", d1.label(), d1.id());

					let d2 = Arc::clone(&d1);
					Box::pin(async move {
							let mut result = Result::<usize>::Ok(0);
							while result.is_ok() {
									let timeout = tokio::time::sleep(Duration::from_secs(5));
									tokio::pin!(timeout);

									tokio::select! {
											_ = timeout.as_mut() =>{
													let message = math_rand_alpha(15);
													println!("Sending '{message}'");
													result = d2.send_text(message).await.map_err(Into::into);
											}
									};
							}
					})
			}));

    let d_label = data_channel.label().to_owned();
    data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
        let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
        println!("Message from DataChannel '{d_label}': '{msg_str}'");
        Box::pin(async {})
    }));

    let mut peer_connections = peer_connections.lock().await;

    peer_connections.insert(other_id.clone(), Arc::clone(&peer_connection));

    let (ice_complete_tx, ice_complete_rx) = tokio::sync::oneshot::channel();
    let ice_complete_tx = Arc::new(Mutex::new(Some(ice_complete_tx)));

    peer_connection.on_ice_connection_state_change(Box::new(
        |connection_state: RTCIceConnectionState| {
            println!("ICE Connection State has changed: {connection_state}");
            Box::pin(async {})
        },
    ));

    let user_id_cloned = user_id.clone();
    let other_id_cloned = other_id.clone();
    let room_id_cloned = room_id.clone();
    let ws_stream_cloned = Arc::clone(&ws_stream);

    peer_connection.on_ice_candidate(Box::new(move |candidate| {
        let ice_complete_tx = Arc::clone(&ice_complete_tx);

        let user_id_cloned = user_id_cloned.clone();
        let other_id_cloned = other_id_cloned.clone();
        let room_id_cloned = room_id_cloned.clone();
        let ws_stream_cloned = Arc::clone(&ws_stream_cloned);

        Box::pin(async move {
            match candidate {
                Some(candidate) => {
                    let candidate_str =
                        serde_json::to_string(&candidate.to_json().unwrap()).unwrap();
                    println!("ICE Candidate: {:?}", candidate_str);

                    let ice_candidate_message = &CommandMessage {
                        user_id: user_id_cloned.to_string(),
                        command: Command::Client(ClientCommand::SendIceCandidate(
                            other_id_cloned.clone(),
                            room_id_cloned.clone(),
                            candidate_str,
                        )),
                    };

                    send_message(ws_stream_cloned.clone(), ice_candidate_message)
                        .await
                        .unwrap();
                }

                None => {
                    println!("ICE Candidate: None");
                    if let Some(tx) = ice_complete_tx.lock().await.take() {
                        let _ = tx.send(());
                    }
                }
            }
        })
    }));

    let offer = peer_connection
        .create_offer(Some(RTCOfferOptions {
            voice_activity_detection: true,
            ..Default::default()
        }))
        .await
        .unwrap();

    peer_connection
        .set_local_description(offer.clone())
        .await
        .unwrap();
    let mut gather_complete = peer_connection.gathering_complete_promise().await;

    let _ = gather_complete.recv().await;

    ice_complete_rx.await.unwrap();

    let offer_message = &CommandMessage {
        user_id: user_id.to_string(),
        command: Command::Client(ClientCommand::SendOffer(
            other_id.clone(),
            room_id.clone(),
            offer.sdp,
        )),
    };

    send_message(ws_stream.clone(), offer_message)
        .await
        .unwrap();

    let mut stdout = stdout();

    write!(stdout, "\n\r*** Sent offer message ***\n\r").unwrap();
    stdout.flush().unwrap();
}

pub async fn handle_offer(
    from_user: String,
    room_id: String,
    offer: RTCSessionDescription,
    username: &str,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ice_candidates: Arc<Mutex<HashMap<String, Vec<RTCIceCandidateInit>>>>,
    ws_stream: Arc<
        Mutex<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    >,
) {
    let mut stdout = stdout();

    write!(stdout, "\n\nhandling offer from\n{:?}", from_user).unwrap();
    stdout.flush().unwrap();

    let peer_connection = create_peer_connection().await.unwrap();

    let mut peer_connections = peer_connections.lock().await;

    peer_connections.insert(from_user.clone(), Arc::clone(&peer_connection));

    peer_connection.set_remote_description(offer).await.unwrap();

    let mut ice_candidates = ice_candidates.lock().await;

    let user_ice_candidates = ice_candidates.get_mut(&from_user);

    if let Some(user_ice_candidates) = user_ice_candidates {
        for candidate in user_ice_candidates {
            peer_connection
                .add_ice_candidate(candidate.clone())
                .await
                .unwrap();
        }
    }

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
}

pub async fn handle_answer(
    from_user: String,
    sdp: String,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
) {
    let peer_connections = peer_connections.lock().await;

    let answer = RTCSessionDescription::answer(sdp).unwrap();

    let peer_connection = peer_connections.get(&from_user);

    if peer_connection.is_none() {
        println!("Peer connection not found for user {:?}", from_user);
        return;
    }

    let peer_connection = peer_connection.unwrap();

    peer_connection
        .set_remote_description(answer)
        .await
        .unwrap();

    // Register data channel creation handling
    peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
			let d_label = d.label().to_owned();
			let d_id = d.id();
			println!("New DataChannel {d_label} {d_id}");

			Box::pin(async move{
					// Register channel opening handling
					let d2 =  Arc::clone(&d);
					let d_label2 = d_label.clone();
					let d_id2 = d_id;
					d.on_open(Box::new(move || {
							println!("Data channel '{d_label2}'-'{d_id2}' open. Random messages will now be sent to any connected DataChannels every 5 seconds");
							Box::pin(async move {
									let mut result = Result::<usize>::Ok(0);
									while result.is_ok() {
											let timeout = tokio::time::sleep(Duration::from_secs(5));
											tokio::pin!(timeout);

											tokio::select! {
													_ = timeout.as_mut() =>{
															let message = math_rand_alpha(15);
															println!("Sending '{message}'");
															result = d2.send_text(message).await.map_err(Into::into);
													}
											};
									}
							})
					}));

					// Register text message handling
					d.on_message(Box::new(move |msg: DataChannelMessage| {
						 let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
						 println!("Message from DataChannel '{d_label}': '{msg_str}'");
						 Box::pin(async{})
				 }));
			})
	}));
}

pub async fn handle_ice_candidate(
    from_user: String,
    candidate_str: String,
    ice_candidates: Arc<Mutex<HashMap<String, Vec<RTCIceCandidateInit>>>>,
) {
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
}
