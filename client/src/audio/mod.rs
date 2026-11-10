pub mod capture;
pub mod receive;
pub mod send;

use anyhow::{anyhow, Result};
use cpal::traits::StreamTrait;
use cpal::{SampleFormat, SampleRate, SupportedStreamConfig, SupportedStreamConfigRange};
use std::sync::mpsc;

/// Opus in WebRTC runs at 48 kHz.
pub const SAMPLE_RATE: u32 = 48_000;
/// One 20 ms mono frame at 48 kHz.
pub const FRAME_SAMPLES: usize = 960;

/// Picks a 48 kHz config with a sample format we convert (f32 or i16).
// ponytail: 48 kHz devices only; resample (e.g. with `rubato`) to support other rates.
fn pick_config(
    configs: impl Iterator<Item = SupportedStreamConfigRange>,
    kind: &str,
) -> Result<SupportedStreamConfig> {
    configs
        .filter(|c| matches!(c.sample_format(), SampleFormat::F32 | SampleFormat::I16))
        .find_map(|c| c.try_with_sample_rate(SampleRate(SAMPLE_RATE)))
        .ok_or_else(|| {
            anyhow!("the default {kind} device does not support 48 kHz f32/i16 audio (resampling is not implemented)")
        })
}

/// Builds and plays a cpal stream on a dedicated thread (cpal streams are not `Send` on
/// macOS). The stream keeps running until the returned `Sender` is dropped.
fn spawn_stream<F>(build: F) -> Result<mpsc::Sender<()>>
where
    F: FnOnce() -> Result<cpal::Stream> + Send + 'static,
{
    let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    std::thread::spawn(move || {
        let stream = match build().and_then(|stream| {
            stream.play()?;
            Ok(stream)
        }) {
            Ok(stream) => stream,
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };
        let _ = ready_tx.send(Ok(()));
        // Returns once every `Sender` for `stop_rx` is dropped.
        let _ = stop_rx.recv();
        drop(stream);
    });

    ready_rx
        .recv()
        .map_err(|_| anyhow!("audio thread exited during setup"))??;
    Ok(stop_tx)
}
