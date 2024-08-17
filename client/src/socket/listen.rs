use crate::commands::{Command, CommandMessage, ServerCommand};
use crate::config::UserConfig;
use crate::connect::{handle_answer, handle_ice_candidate, handle_offer, handle_refresh};
use crate::rooms::{display_empty_room, display_rooms};
use crate::{ConnectionState, User};

use anyhow::Result;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use webrtc::peer_connection::RTCPeerConnection;

pub async fn listen_for_ws(
    user: Arc<UserConfig>,
    ws_stream: Arc<
        tokio::sync::Mutex<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    >,
) -> Result<()> {
    let users: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(HashMap::new()));
    let peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        let msg = {
            let mut ws_stream = ws_stream.lock().await;
            ws_stream.next().await
        };

        if msg.is_none() {
            break Ok(());
        }

        let msg = msg.unwrap();
        if msg.is_err() {
            println!("Error receiving message: {:?}", msg);
            continue;
        }
        let msg = msg.unwrap();

        let text = msg.to_text();
        if text.is_err() {
            println!("Error parsing message: {:?}", msg);
            continue;
        }
        let text = text.unwrap();

        let command_message: CommandMessage = serde_json::from_str(text)?;

        println!("\n\n *** Received ***\n{:?}", command_message.command);

        match command_message.command {
            Command::Server(ServerCommand::RoomList(rooms)) => {
                println!("Received room list command with rooms: {:?}", rooms);

                if rooms.is_empty() {
                    display_empty_room();
                } else {
                    display_rooms(rooms);
                }
            }
            Command::Server(ServerCommand::Refresh(user_list)) => {
                println!("Received refresh command with users: {:?}", user_list);
                {
                    let mut users = users.lock().await;
                    for other in user_list.iter() {
                        if other == &user.id {
                            continue;
                        }

                        users.entry(other.clone()).or_insert(User {
                            username: other.clone(),
                            state: ConnectionState::Connecting,
                        });
                    }
                }

                handle_refresh(
                    user_list,
                    &user.id,
                    peer_connections.clone(),
                    ws_stream.clone(),
                )
                .await;

                println!("Refreshed users");
            }
            Command::Server(ServerCommand::SendOffer(from_user, sdp)) => {
                handle_offer(
                    from_user,
                    sdp,
                    &user.id,
                    peer_connections.clone(),
                    ws_stream.clone(),
                    users.clone(),
                )
                .await;
            }
            Command::Server(ServerCommand::SendIceCandidate(from_user, candidate)) => {
                handle_ice_candidate(from_user, candidate, peer_connections.clone()).await;
            }
            Command::Server(ServerCommand::SendAnswer(from_user, sdp)) => {
                handle_answer(from_user, sdp, peer_connections.clone(), users.clone()).await;
            }
            _ => println!("Unexpected message: {:?}", command_message),
        }
    }
}
