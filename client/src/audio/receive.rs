use super::{pick_config, spawn_stream, SAMPLE_RATE};
use anyhow::{bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{FromSample, SampleFormat, SizedSample, SupportedStreamConfig};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use webrtc::track::track_remote::TrackRemote;

/// At most 1 s of decoded audio is queued per remote track.
const MAX_BUFFERED_SAMPLES: usize = SAMPLE_RATE as usize;
/// Largest Opus packet: 120 ms at 48 kHz.
const MAX_PACKET_SAMPLES: usize = 5760;

type PlaybackBuffer = Arc<Mutex<VecDeque<f32>>>;

/// Appends `samples`, dropping the oldest ones beyond `max`.
// ponytail: no jitter buffer, packets are played in arrival order; add one if audio stutters.
pub fn push_bounded(buffer: &mut VecDeque<f32>, samples: &[f32], max: usize) {
    buffer.extend(samples);
    if buffer.len() > max {
        buffer.drain(..buffer.len() - max);
    }
}

/// Decodes one remote Opus track and plays it on the default output device until the track ends.
pub async fn play_track(track: Arc<TrackRemote>) {
    if let Err(e) = decode_and_play(track).await {
        eprintln!("\n\rAudio playback stopped: {e:#}");
    }
}

async fn decode_and_play(track: Arc<TrackRemote>) -> Result<()> {
    let buffer: PlaybackBuffer = Arc::default();
    let playback = Arc::clone(&buffer);

    // Dropping `_playback_guard` when this function returns stops the output stream.
    let _playback_guard = spawn_stream(move || {
        let device = cpal::default_host()
            .default_output_device()
            .context("no default output device")?;
        let config = pick_config(device.supported_output_configs()?, "output")?;

        match config.sample_format() {
            SampleFormat::F32 => build_output::<f32>(&device, config, playback),
            SampleFormat::I16 => build_output::<i16>(&device, config, playback),
            other => bail!("unsupported output sample format {other}"),
        }
    })?;

    let mut decoder = opus::Decoder::new(SAMPLE_RATE, opus::Channels::Mono)?;
    let mut pcm = vec![0f32; MAX_PACKET_SAMPLES];

    while let Ok((packet, _)) = track.read_rtp().await {
        let Ok(len) = decoder.decode_float(&packet.payload, &mut pcm, false) else {
            continue; // skip a corrupt packet
        };
        push_bounded(&mut buffer.lock().unwrap(), &pcm[..len], MAX_BUFFERED_SAMPLES);
    }

    Ok(())
}

fn build_output<T>(
    device: &cpal::Device,
    config: SupportedStreamConfig,
    buffer: PlaybackBuffer,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels() as usize;

    let stream = device.build_output_stream(
        &config.into(),
        move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut buffer = buffer.lock().unwrap();
            for frame in out.chunks_mut(channels) {
                // Mono source: the same sample on every output channel; silence when empty.
                frame.fill(T::from_sample(buffer.pop_front().unwrap_or(0.0)));
            }
        },
        |err| eprintln!("\n\rSpeaker error: {err}"),
        None,
    )?;

    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::FRAME_SAMPLES;

    #[test]
    fn bounded_buffer_drops_oldest() {
        let mut buffer = VecDeque::new();
        push_bounded(&mut buffer, &[1.0, 2.0, 3.0], 4);
        push_bounded(&mut buffer, &[4.0, 5.0], 4);
        assert_eq!(buffer, [2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn opus_round_trip_yields_one_frame() {
        let mut encoder =
            opus::Encoder::new(SAMPLE_RATE, opus::Channels::Mono, opus::Application::Voip).unwrap();
        let input: Vec<f32> = (0..FRAME_SAMPLES)
            .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / SAMPLE_RATE as f32).sin() * 0.5)
            .collect();
        let mut packet = [0u8; 4000];
        let len = encoder.encode_float(&input, &mut packet).unwrap();
        assert!(len > 0 && len < 1200, "an encoded frame must fit one RTP packet");

        let mut decoder = opus::Decoder::new(SAMPLE_RATE, opus::Channels::Mono).unwrap();
        let mut pcm = vec![0f32; MAX_PACKET_SAMPLES];
        let decoded = decoder.decode_float(&packet[..len], &mut pcm, false).unwrap();
        assert_eq!(decoded, FRAME_SAMPLES);
    }
}
