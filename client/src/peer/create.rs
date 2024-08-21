use anyhow::Result;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{configuration::RTCConfiguration, RTCPeerConnection},
};

pub async fn create_peer_connection(
    _rx_audio: Arc<Mutex<Receiver<Vec<u8>>>>,
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

    let peer_connection = api.new_peer_connection(config).await?;

    peer_connection
        .add_transceiver_from_kind(
            webrtc::rtp_transceiver::rtp_codec::RTPCodecType::Audio,
            None,
        )
        .await?;

    let peer_connection = Arc::new(peer_connection);

    Ok(peer_connection)
}
