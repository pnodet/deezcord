use crate::audio::{receive::play_track, send::send_audio};
use anyhow::Result;
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::broadcast;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_OPUS},
        APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{configuration::RTCConfiguration, RTCPeerConnection},
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{track_local_static_sample::TrackLocalStaticSample, TrackLocal},
};

/// Creates a peer connection with the audio pipeline attached: one sendrecv Opus
/// transceiver, a task sending our microphone frames, and playback of the remote track.
/// Used by both the offerer (`connect_peer`) and the answerer (`handle_offer`).
pub async fn create_peer_connection(
    audio_tx: &broadcast::Sender<Bytes>,
) -> Result<Arc<RTCPeerConnection>> {
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;

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

    // add_track creates the connection's only audio transceiver (sendrecv). On the answerer
    // this runs before set_remote_description, which pairs this transceiver with the offer's
    // audio m-line instead of creating a second one.
    let track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_owned(),
            ..Default::default()
        },
        "audio".to_owned(),
        "deezcord".to_owned(),
    ));
    peer_connection
        .add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;
    tokio::spawn(send_audio(
        Arc::clone(&peer_connection),
        track,
        audio_tx.subscribe(),
    ));

    peer_connection.on_track(Box::new(|track, _receiver, _transceiver| {
        tokio::spawn(play_track(track));
        Box::pin(async {})
    }));

    Ok(peer_connection)
}
