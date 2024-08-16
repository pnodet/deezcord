use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalMessage {
    pub username: String,
    pub room: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientCommand {
    Join,
    Leave,
    ListRooms,
    Connect,
    SendOffer(String, RTCSessionDescription),
    SendAnswer(String, RTCSessionDescription),
    SendIceCandidate(String, String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerCommand {
    Ack,
    ConnectedAs(String),
    Refresh(Vec<String>),
    RoomList(Vec<String>),
    SendOffer(String, RTCSessionDescription),
    SendAnswer(String, RTCSessionDescription),
    SendIceCandidate(String, String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandMessage {
    pub username: String,
    pub room: String,
    pub command: Command,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Client(ClientCommand),
    Server(ServerCommand),
}

pub async fn wait_for_ack(
    ws_stream: Arc<
        Mutex<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    >,
) -> Result<(), Box<dyn Error>> {
    match timeout(Duration::from_secs(5), ws_stream.lock().await.next()).await {
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
