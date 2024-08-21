use crate::commands::CommandMessage;
use futures_util::SinkExt;
use std::error::Error;
use std::io::{stdout, Write};
use std::sync::Arc;
use termion::raw::IntoRawMode;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

pub async fn send_message(
    ws_stream: Arc<
        Mutex<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    >,
    message: &CommandMessage,
) -> Result<(), Box<dyn Error>> {
    let msg_text = serde_json::to_string(&message)?;
    let mut stdout = stdout().into_raw_mode().unwrap();
    // write!(stdout, "\n\r*** Sending ***\n\r{:?}\n\r", message.command).unwrap();
    stdout.flush().unwrap();
    drop(stdout);

    let mut ws_stream = ws_stream.lock().await;
    ws_stream.send(Message::text(msg_text)).await?;

    Ok(())
}
