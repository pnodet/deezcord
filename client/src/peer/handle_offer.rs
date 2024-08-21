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
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::peer_connection::offer_answer_options::RTCAnswerOptions;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocal;
use webrtc::track::track_remote::TrackRemote;

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
    watch_tx: tokio::sync::watch::Sender<()>,
    rx_audio: Arc<Mutex<Receiver<Vec<u8>>>>,
) -> Result<()> {
    let mut stdout = stdout();

    write!(stdout, "\n\nhandling offer from\n{:?}", from_user).unwrap();
    stdout.flush().unwrap();

    let peer_connection = create_peer_connection(rx_audio.clone()).await.unwrap();

    println!("Setting up audio for {:?}", peer_connection.get_stats_id());

    let audio_track = Arc::new(TrackLocalStaticRTP::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_owned(),
            channels: 2,
            clock_rate: 48000,
            ..Default::default()
        },
        "audio_track_2".to_owned(),
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

    let _ = peer_connection
        .create_data_channel("data_2", None)
        .await
        .unwrap();

    peer_connection
        .add_transceiver_from_kind(
            webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio,
            None,
        )
        .await?;

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

    peer_connection.on_track(Box::new(
        move |track: Arc<TrackRemote>,
              _receiver: Arc<RTCRtpReceiver>,
              _transceiver: Arc<RTCRtpTransceiver>| {
            println!("Received remote track: {:?}", track.ssrc());

            Box::pin(async {})
        },
    ));

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
