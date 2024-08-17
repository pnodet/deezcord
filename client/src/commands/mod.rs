use serde::{Deserialize, Serialize};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::rooms::Room;

pub mod wait_for_ack;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientCommand {
    Connect(String),
    CreateRoom(String),
    ListRooms,
    Join(String),
    Leave,
    SendOffer(String, RTCSessionDescription),
    SendAnswer(String, RTCSessionDescription),
    SendIceCandidate(String, String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerCommand {
    Ack,
    ConnectedAs(String),
    Refresh(Vec<String>),
    RoomList(Vec<Room>),
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
