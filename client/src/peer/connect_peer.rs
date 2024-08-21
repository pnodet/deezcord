use crate::audio::receive::receive_audio;
use crate::audio::send::send_audio;
use crate::commands::{ClientCommand, Command, CommandMessage};
use crate::peer::create::create_peer_connection;
use crate::socket::send::send_message;
use anyhow::Result;
use std::collections::HashMap;
use std::io::stdout;
use std::io::Write;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::api::media_engine::MIME_TYPE_OPUS;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::peer_connection::offer_answer_options::RTCOfferOptions;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocal;
use webrtc::track::track_remote::TrackRemote;

pub async fn connect_peer(
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
    watch_tx: tokio::sync::watch::Sender<()>,
    rx_audio: Arc<Mutex<Receiver<Vec<u8>>>>,
) -> Result<()> {
    let peer_connection = create_peer_connection(rx_audio.clone()).await.unwrap();

    let _ = peer_connection
        .create_data_channel("data", None)
        .await
        .unwrap();

    peer_connection
        .add_transceiver_from_kind(
            webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio,
            None,
        )
        .await?;

    println!("Setting up audio for {:?}", peer_connection.get_stats_id());

    let audio_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_owned(),
            channels: 2,
            clock_rate: 48000,
            ..Default::default()
        },
        "audio_track".to_owned(),
        "webrtc-rs".to_owned(),
    ));

    if let Err(e) = peer_connection
        .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await
    {
        eprintln!("Failed to add track: {:?}", e);
        return Err(e.into());
    }

    let rx_audio = Arc::clone(&rx_audio);
    tokio::spawn(send_audio(
        peer_connection.clone(),
        rx_audio.clone(),
        audio_track.clone(),
    ));

    receive_audio(&peer_connection).await;

    let mut peer_connections = peer_connections.lock().await;

    peer_connections.insert(other_id.clone(), Arc::clone(&peer_connection));

    let (ice_complete_tx, ice_complete_rx) = tokio::sync::oneshot::channel();
    let ice_complete_tx = Arc::new(Mutex::new(Some(ice_complete_tx)));

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
        println!("\n\rDataChannel {d_label} {d_id}");
        Box::pin(async {})
    }));

    peer_connection.on_negotiation_needed(Box::new(move || {
        println!("\n\rNegotiation needed");
        Box::pin(async {})
    }));

    peer_connection.on_track(Box::new(
        move |track: Arc<TrackRemote>,
              _receiver: Arc<RTCRtpReceiver>,
              _transceiver: Arc<RTCRtpTransceiver>| {
            println!("\n\rReceived remote track: {:?}", track.ssrc());

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

    Ok(())
}
