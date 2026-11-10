use super::{pick_config, spawn_stream, FRAME_SAMPLES, SAMPLE_RATE};
use anyhow::{bail, Context, Result};
use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, SupportedStreamConfig};
use std::sync::mpsc;
use tokio::sync::broadcast;

/// Turns interleaved device samples into mono frames of exactly `FRAME_SAMPLES` samples.
pub struct FrameAccumulator {
    channels: usize,
    pending: Vec<f32>,
}

impl FrameAccumulator {
    pub fn new(channels: usize) -> Self {
        Self {
            channels,
            pending: Vec::with_capacity(FRAME_SAMPLES),
        }
    }

    /// Downmixes `interleaved` to mono and calls `on_frame` for every completed frame.
    /// Leftover samples are kept for the next call.
    pub fn push<T>(&mut self, interleaved: &[T], mut on_frame: impl FnMut(&[f32]))
    where
        T: Sample,
        f32: FromSample<T>,
    {
        for frame in interleaved.chunks_exact(self.channels) {
            let sum: f32 = frame.iter().map(|s| s.to_sample::<f32>()).sum();
            self.pending.push(sum / self.channels as f32);

            if self.pending.len() == FRAME_SAMPLES {
                on_frame(&self.pending);
                self.pending.clear();
            }
        }
    }
}

/// Starts microphone capture on a dedicated thread and returns the channel every encoded
/// 20 ms Opus frame is published on. Capture stops when the returned guard is dropped.
pub fn start_capture() -> Result<(broadcast::Sender<Bytes>, mpsc::Sender<()>)> {
    // 50 frames = 1 s; a receiver further behind than that skips ahead (RecvError::Lagged).
    let (frames_tx, _) = broadcast::channel(50);
    let frames = frames_tx.clone();

    let guard = spawn_stream(move || {
        let device = cpal::default_host()
            .default_input_device()
            .context("no default input device")?;
        let config = pick_config(device.supported_input_configs()?, "input")?;

        match config.sample_format() {
            SampleFormat::F32 => build_input::<f32>(&device, config, frames),
            SampleFormat::I16 => build_input::<i16>(&device, config, frames),
            other => bail!("unsupported input sample format {other}"),
        }
    })?;

    Ok((frames_tx, guard))
}

fn build_input<T>(
    device: &cpal::Device,
    config: SupportedStreamConfig,
    frames: broadcast::Sender<Bytes>,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let mut accumulator = FrameAccumulator::new(config.channels() as usize);
    let mut encoder =
        opus::Encoder::new(SAMPLE_RATE, opus::Channels::Mono, opus::Application::Voip)?;
    let mut packet = [0u8; 4000];

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            accumulator.push(data, |frame| {
                if let Ok(len) = encoder.encode_float(frame, &mut packet) {
                    // Err only means nobody is subscribed yet (no peers).
                    let _ = frames.send(Bytes::copy_from_slice(&packet[..len]));
                }
            });
        },
        |err| eprintln!("\n\rMicrophone error: {err}"),
        None,
    )?;

    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(acc: &mut FrameAccumulator, data: &[f32]) -> Vec<Vec<f32>> {
        let mut frames = vec![];
        acc.push(data, |f| frames.push(f.to_vec()));
        frames
    }

    #[test]
    fn partial_buffers_are_kept_until_a_frame_is_full() {
        let mut acc = FrameAccumulator::new(1);
        assert!(collect(&mut acc, &[0.5; 500]).is_empty());
        let frames = collect(&mut acc, &[0.5; 500]);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].len(), FRAME_SAMPLES);
        // 40 samples are left over; 920 more complete the next frame.
        assert_eq!(collect(&mut acc, &[0.5; 920]).len(), 1);
    }

    #[test]
    fn one_callback_can_yield_several_frames() {
        let mut acc = FrameAccumulator::new(1);
        let frames = collect(&mut acc, &[0.1; FRAME_SAMPLES * 3 + 10]);
        assert_eq!(frames.len(), 3);
        assert!(frames.iter().all(|f| f.len() == FRAME_SAMPLES));
    }

    #[test]
    fn stereo_is_downmixed_to_mono() {
        let mut acc = FrameAccumulator::new(2);
        let stereo: Vec<f32> = (0..FRAME_SAMPLES).flat_map(|_| [1.0, 0.0]).collect();
        let frames = collect(&mut acc, &stereo);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].iter().all(|&s| s == 0.5));
    }

    #[test]
    fn i16_samples_are_converted() {
        let mut acc = FrameAccumulator::new(1);
        let mut frames = vec![];
        acc.push(&[i16::MIN; FRAME_SAMPLES], |f| frames.push(f.to_vec()));
        assert_eq!(frames[0][0], -1.0);
    }
}
