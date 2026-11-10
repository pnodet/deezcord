use bytes::Bytes;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::{self, error::RecvError};
use webrtc::media::Sample;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

/// Forwards encoded microphone frames to one peer until the capture channel closes or the
/// connection is closed/failed. `write_sample` builds the RTP packets (sequence numbers,
/// timestamps, payload type).
pub async fn send_audio(
    peer_connection: Arc<RTCPeerConnection>,
    track: Arc<TrackLocalStaticSample>,
    mut frames: broadcast::Receiver<Bytes>,
) {
    loop {
        let data = match frames.recv().await {
            Ok(data) => data,
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => return,
        };

        // write_sample returns Ok while the track is unbound (before connecting and after
        // close), so the connection state is what ends this task.
        if matches!(
            peer_connection.connection_state(),
            RTCPeerConnectionState::Closed | RTCPeerConnectionState::Failed
        ) {
            return;
        }

        let sample = Sample {
            data,
            duration: Duration::from_millis(20),
            ..Default::default()
        };
        if let Err(e) = track.write_sample(&sample).await {
            eprintln!("\n\rStopped sending audio: {e}");
            return;
        }
    }
}
