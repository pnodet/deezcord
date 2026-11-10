use futures_util::StreamExt;
use std::error::Error;
use tokio::time::{timeout, Duration};

use super::{Command, CommandMessage, ServerCommand};
use crate::socket::WsReader;

pub async fn wait_for_ack(ws_read: &mut WsReader) -> Result<(), Box<dyn Error>> {
    match timeout(Duration::from_secs(5), ws_read.next()).await {
        Ok(Some(Ok(msg))) => {
            if let Ok(text) = msg.to_text() {
                let ack_msg: CommandMessage = serde_json::from_str(text)?;
                if matches!(ack_msg.command, Command::Server(ServerCommand::Ack)) {
                    return Ok(());
                }
            }
        }
        _ => eprintln!("Did not receive ack in time"),
    }
    Err("Ack not received".into())
}
