use futures_util::SinkExt;
use signal::CommandMessage;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::signal;

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
    println!("Sending message: {:?}", message.command); // Debug log
    let mut ws_stream = ws_stream.lock().await;
    ws_stream.send(Message::text(msg_text)).await?;
    Ok(())
}
