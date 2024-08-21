use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Write;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn capture_audio(tx_audio: Arc<Mutex<Sender<Vec<u8>>>>) {
    println!("capture_audio function started");
    std::io::stdout().flush().unwrap();

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("Failed to get default input device");

    let config = device.default_input_config().unwrap();

    let tx_audio = tx_audio.lock().await.clone();

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                std::io::stdout().flush().unwrap();

                let mut buffer = vec![];
                for &sample in data.iter() {
                    buffer.extend_from_slice(&sample.to_ne_bytes());
                }

                if let Err(e) = tx_audio.send(buffer) {
                    eprintln!("Failed to send audio buffer: {}", e);
                    std::io::stdout().flush().unwrap();
                }
            },
            move |err| {
                eprintln!("Stream error: {}", err);
                std::io::stdout().flush().unwrap();
            },
            None,
        )
        .expect("Failed to build input stream");

    stream.play().expect("Failed to start audio stream");
    println!("Audio stream started");
    std::io::stdout().flush().unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5)); // Adjust as needed
}
