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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;

    macro_rules! fixture {
        ($name:literal) => {
            include_str!(concat!("../../../protocol-fixtures/", $name))
        };
    }

    /// A message the client sends: it must deserialize and re-serialize to the same JSON.
    fn client_fixture(json: &str) -> ClientCommand {
        let msg: CommandMessage = serde_json::from_str(json).expect("fixture deserializes");
        let expected: Value = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_value(&msg).unwrap(), expected);
        match msg.command {
            Command::Client(cmd) => cmd,
            other => panic!("expected a Client command, got {:?}", other),
        }
    }

    /// A message the client receives: it must deserialize. No re-serialization check,
    /// because the server sends `{"Ack":null}` (serde writes `"Ack"`) and extra
    /// room fields such as `users_ids` (serde drops them).
    fn server_fixture(json: &str) -> ServerCommand {
        let msg: CommandMessage = serde_json::from_str(json).expect("fixture deserializes");
        match msg.command {
            Command::Server(cmd) => cmd,
            other => panic!("expected a Server command, got {:?}", other),
        }
    }

    #[test]
    fn client_fixtures_round_trip() {
        assert!(matches!(
            client_fixture(fixture!("client-connect.json")),
            ClientCommand::Connect(_)
        ));
        assert!(matches!(
            client_fixture(fixture!("client-list-rooms.json")),
            ClientCommand::ListRooms
        ));
        assert!(matches!(
            client_fixture(fixture!("client-create-room.json")),
            ClientCommand::CreateRoom(_)
        ));
        assert!(matches!(
            client_fixture(fixture!("client-join.json")),
            ClientCommand::Join(_)
        ));
        assert!(matches!(
            client_fixture(fixture!("client-send-offer.json")),
            ClientCommand::SendOffer(..)
        ));
        assert!(matches!(
            client_fixture(fixture!("client-send-answer.json")),
            ClientCommand::SendAnswer(..)
        ));
        match client_fixture(fixture!("client-send-ice-candidate.json")) {
            ClientCommand::SendIceCandidate(_, _, candidate) => {
                serde_json::from_str::<RTCIceCandidateInit>(&candidate)
                    .expect("candidate is an RTCIceCandidateInit JSON string");
            }
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn server_fixtures_deserialize() {
        assert!(matches!(
            server_fixture(fixture!("server-ack.json")),
            ServerCommand::Ack
        ));
        match server_fixture(fixture!("server-room-list.json")) {
            ServerCommand::RoomList(rooms) => {
                assert_eq!(rooms.len(), 1);
                assert_eq!(rooms[0].users, vec!["user-a", "user-b"]);
            }
            other => panic!("unexpected {:?}", other),
        }
        assert!(matches!(
            server_fixture(fixture!("server-incoming-offer.json")),
            ServerCommand::IncomingOffer(..)
        ));
        assert!(matches!(
            server_fixture(fixture!("server-incoming-answer.json")),
            ServerCommand::IncomingAnswer(..)
        ));
        match server_fixture(fixture!("server-incoming-ice-candidate.json")) {
            ServerCommand::IncomingIceCandidate(_, _, candidate) => {
                serde_json::from_str::<RTCIceCandidateInit>(&candidate)
                    .expect("candidate is an RTCIceCandidateInit JSON string");
            }
            other => panic!("unexpected {:?}", other),
        }
    }
}
