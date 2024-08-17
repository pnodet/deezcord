use serde::{Deserialize, Serialize};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

pub mod wait_for_ack;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientCommand {
    Join(String),
    Leave,
    ListRooms,
    Connect(String),
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
    pub user_id: String,
    pub command: Command,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Client(ClientCommand),
    Server(ServerCommand),
}
