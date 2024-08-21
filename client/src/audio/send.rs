use anyhow::Result;
use bytes::Bytes;
use rtp::packet::Packet;
use std::{
    io::Write,
    sync::{mpsc::Receiver, Arc},
};
use tokio::sync::Mutex;
use webrtc::track::track_local::TrackLocalWriter;
use webrtc::{
    peer_connection::RTCPeerConnection,
    track::track_local::track_local_static_rtp::TrackLocalStaticRTP,
};

pub async fn send_audio(
    peer_connection: Arc<RTCPeerConnection>,
    rx: Arc<Mutex<Receiver<Vec<u8>>>>,
    audio_track: Arc<TrackLocalStaticRTP>,
) -> Result<()> {
    println!("Sending audio to {:?}", peer_connection.get_stats_id());
    std::io::stdout().flush().unwrap();

    let rx = rx.lock().await;
    let mut sequence_number: u16 = 0;
    let mut timestamp: u32 = 0;

    while let Ok(audio_data) = rx.recv() {
        let packet = Packet {
            header: rtp::header::Header {
                version: 2,
                padding: false,
                extension: false,
                marker: false,
                payload_type: 111,
                sequence_number,
                timestamp,
                ssrc: 200566587,
                ..Default::default()
            },
            payload: Bytes::from(audio_data),
        };

        if let Err(e) = audio_track.write_rtp(&packet).await {
            eprintln!("Failed to write sample: {:?}", e);
        }

        sequence_number = sequence_number.wrapping_add(1);
        timestamp = timestamp.wrapping_add(960); // 960 =r 20ms of Opus at 48kHz
    }

    Ok(())
}
