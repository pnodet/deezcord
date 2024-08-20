use crate::commands::{Command, CommandMessage, ServerCommand};
use crate::config::UserConfig;
use crate::connect::{handle_answer, handle_ice_candidate, handle_offer};
use crate::rooms::{display_empty_room, display_room, display_rooms};

use anyhow::Result;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::io::{stdout, Write};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

pub async fn listen_for_ws(
    tx: Sender<()>,
    user: Arc<UserConfig>,
    ws_stream: Arc<
        tokio::sync::Mutex<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    >,
) -> Result<()> {
    let peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let ice_candidates: Arc<Mutex<HashMap<String, Vec<RTCIceCandidateInit>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        let mut stdout = stdout();
        let msg = {
            let mut ws_stream = ws_stream.lock().await;
            ws_stream.next().await
        };

        if msg.is_none() {
            break Ok(());
        }

        let msg = msg.unwrap();
        if msg.is_err() {
            write!(stdout, "\n\rError message: {:?}\n\r", msg).unwrap();
            stdout.flush().unwrap();
            tx.send(()).unwrap();
            continue;
        }

        let msg = msg.unwrap();

        if msg.is_ping() || msg.is_pong() {
            continue;
        }

        if !msg.is_text() {
            write!(stdout, "\n\rError handling message: {:?}\n\r", msg).unwrap();
            stdout.flush().unwrap();
            continue;
        }

        let text = msg.to_text();
        if text.is_err() {
            write!(stdout, "\n\rReceived non-text message: {:?}\n\r", msg).unwrap();
            stdout.flush().unwrap();
            continue;
        }

        let text = text.unwrap();

        let command_message: CommandMessage = serde_json::from_str(text).unwrap();

        if command_message.user_id != user.id {
            continue;
        }

        write!(stdout, "\n\n*** Received ***\n{:?}", command_message).unwrap();
        stdout.flush().unwrap();

        match command_message.command {
            Command::Server(ServerCommand::IncomingOffer(from_user, room_id, sdp)) => {
                let offer = RTCSessionDescription::offer(sdp)?;

                handle_offer(
                    from_user,
                    room_id,
                    offer,
                    &user.id,
                    peer_connections.clone(),
                    ice_candidates.clone(),
                    ws_stream.clone(),
                )
                .await;
            }
            Command::Server(ServerCommand::RoomList(rooms)) => {
                // Check if current user is in any of the rooms

                let current_room = rooms.iter().find(|room| room.users.contains(&user.id));

                if let Some(current_room) = current_room {
                    display_room(current_room.clone(), 0);
                } else if rooms.is_empty() {
                    display_empty_room(tx.clone(), user.clone(), ws_stream.clone()).await;
                } else {
                    display_rooms(
                        rooms,
                        user.clone(),
                        peer_connections.clone(),
                        ws_stream.clone(),
                    )
                    .await;
                }
            }

            Command::Server(ServerCommand::IncomingIceCandidate(
                from_user,
                _room_id,
                candidate,
            )) => {
                handle_ice_candidate(from_user, candidate, ice_candidates.clone()).await;
            }

            Command::Server(ServerCommand::IncomingAnswer(from_user, _room_id, sdp)) => {
                handle_answer(from_user, sdp, peer_connections.clone()).await;
            }

            _ => {
                write!(stdout, "Unexpected message: {:?}", command_message).unwrap();
                stdout.flush().unwrap();
            }
        }
    }
}
