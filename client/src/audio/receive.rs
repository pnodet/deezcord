use std::sync::Arc;
use webrtc::{
    peer_connection::RTCPeerConnection,
    rtp_transceiver::{rtp_receiver::RTCRtpReceiver, RTCRtpTransceiver},
    track::track_remote::TrackRemote,
};

use super::decode::decode_opus;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn play_audio(pcm_data: Vec<i16>) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Failed to get default output device");
    let config = device
        .default_output_config()
        .expect("Failed to get default output format");

    let stream = device
        .build_output_stream(
            &config.into(),
            move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for (i, sample) in pcm_data.iter().enumerate() {
                    if i < output.len() {
                        output[i] = *sample as f32 / i16::MAX as f32;
                    }
                }
            },
            move |err| {
                eprintln!("Stream error: {}", err);
            },
            None,
        )
        .unwrap();

    stream.play().unwrap();
    // Keep the stream alive to play the audio
    std::thread::sleep(std::time::Duration::from_secs(5));
}

pub async fn receive_audio(peer_connection: &RTCPeerConnection) {
    println!("Receiving audio from {:?}", peer_connection.get_stats_id());

    peer_connection.on_track(Box::new(
        move |track: Arc<TrackRemote>,
              _receiver: Arc<RTCRtpReceiver>,
              _transceiver: Arc<RTCRtpTransceiver>| {
            println!("Received remote track: {:?}", track);

            Box::pin(async move {
                let mut buffer = vec![0u8; 2048];
                loop {
                    match track.read(&mut buffer).await {
                        Ok(n) => {
                            let (packet, _info) = n;
                            let encoded_data = packet.payload;
                            println!("Received RTP packet with size: {}", encoded_data.len());

                            match decode_opus(&encoded_data) {
                                Ok(decoded_pcm) => {
                                    println!("Decoded PCM data with length: {}", decoded_pcm.len());
                                    play_audio(decoded_pcm);
                                }
                                Err(e) => {
                                    eprintln!("Failed to decode OPUS data: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading from track: {:?}", e);
                            break;
                        }
                    }
                }
            })
        },
    ));
}
