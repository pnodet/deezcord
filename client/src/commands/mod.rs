use crate::rooms::Room;
use serde::{Deserialize, Serialize};

pub mod wait_for_ack;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientCommand {
    Connect(String),
    CreateRoom(String),
    ListRooms,
    Join(String),
    Leave,
    SendOffer(
        String, // user_id
        String, // room_id
        String, // sdp
    ),
    SendAnswer(
        String, // user_id
        String, // room_id
        String, // sdp
    ),
    SendIceCandidate(
        String, // user_id
        String, // room_id
        String, // candidate
    ),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerCommand {
    Ack,
    ConnectedAs(String),
    Refresh(Vec<String>),
    RoomList(Vec<Room>),
    IncomingOffer(
        String, // user_id
        String, // room_id
        String, // sdp
    ),
    IncomingAnswer(
        String, // user_id
        String, // room_id
        String, // sdp
    ),
    IncomingIceCandidate(
        String, // user_id
        String, // room_id
        String, // candidate
    ),
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
