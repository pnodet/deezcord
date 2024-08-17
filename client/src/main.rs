mod commands;
mod config;
mod connect;
mod socket;
mod user;

use crate::commands::wait_for_ack::wait_for_ack;
use crate::commands::{ClientCommand, Command, CommandMessage, ServerCommand};
use connect::{handle_answer, handle_ice_candidate, handle_offer, handle_refresh};
use futures_util::StreamExt;
use socket::send_message;
use std::collections::HashMap;
use std::io::{stdout, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio::time::{self, Duration};
use tokio_tungstenite::connect_async;
use user::{get_user_or_create, set_id};
use webrtc::peer_connection::RTCPeerConnection;

const SERVER_URL: &str = "ws://localhost:3030/ws";
const ROOM_NAME: &str = "nivalis";

#[derive(Clone, Debug)]
struct User {
    username: String,
    state: ConnectionState,
}

#[derive(Clone, Debug)]
enum ConnectionState {
    Connecting,
    Connected,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = get_user_or_create()?;

    let (ws_stream, _) = connect_async(SERVER_URL).await.expect("Failed to connect");
    let ws_stream = Arc::new(Mutex::new(ws_stream));

    send_message(
        ws_stream.clone(),
        &CommandMessage {
            user_id: user.id.clone(),
            command: Command::Client(ClientCommand::Connect(user.name.clone())),
        },
    )
    .await?;
    wait_for_ack(ws_stream.clone()).await?;

    match timeout(Duration::from_secs(5), ws_stream.lock().await.next()).await {
        Ok(Some(Ok(msg))) => {
            if let Ok(text) = msg.to_text() {
                let ack_msg: CommandMessage = serde_json::from_str(text)?;
                if let Command::Server(ServerCommand::ConnectedAs(id)) = ack_msg.command {
                    set_id(&id);
                }
            }
        }
        _ => eprintln!("Did not receive ack in time"),
    }

    send_message(
        ws_stream.clone(),
        &CommandMessage {
            user_id: user.id.clone(),
            command: Command::Client(ClientCommand::Join(ROOM_NAME.to_string())),
        },
    )
    .await?;
    wait_for_ack(ws_stream.clone()).await?;

    println!("Connected to server, joined room {}", ROOM_NAME);

    let users: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(HashMap::new()));
    let peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let users_clone = users.clone();
    tokio::spawn(async move {
        loop {
            {
                let users = users_clone.lock().await;
                let user_list: Vec<User> = users.values().cloned().collect();
                display_room(ROOM_NAME, user_list);
            }
            time::sleep(Duration::from_secs(1)).await;
        }
    });

    loop {
        let msg = {
            let mut ws_stream = ws_stream.lock().await;
            ws_stream.next().await
        };

        if msg.is_none() {
            break;
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

        println!("Received message {:?}", command_message.command);

        match command_message.command {
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

    Ok(())
}

fn display_room(room_name: &str, users: Vec<User>) {
    println!("Displaying room {:?} : {:?}", room_name, users);
    let mut stdout = stdout();
    // execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
    println!("Room {} :", room_name);
    for user in users {
        println!("- {} : {:?}", user.username, user.state);
    }
    stdout.flush().unwrap();
}
