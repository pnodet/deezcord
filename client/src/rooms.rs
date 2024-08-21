use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::stdin;
use std::sync::mpsc::{Receiver, Sender};
use std::{
    io::{stdout, Write},
    sync::Arc,
};
use termion::cursor::DetectCursorPos;
use termion::raw::IntoRawMode;
use termion::{event::Key, input::TermRead};
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use webrtc::peer_connection::RTCPeerConnection;

use crate::peer::connect_peer::connect_peer;
use crate::{
    commands::{ClientCommand, Command, CommandMessage},
    config::UserConfig,
    socket::send::send_message,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub users: Vec<String>,
}

pub fn display_room(room: Room, index: usize) {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(stdout, "\n\r{}) Room {} :", index, room.name).unwrap();
    stdout.flush().unwrap();

    for user in room.users {
        write!(stdout, "\n\r- {}", user).unwrap();
        stdout.flush().unwrap();
    }

    drop(stdout);
}

pub async fn join_room(
    room: Room,
    user: Arc<UserConfig>,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ws_stream: Arc<
        tokio::sync::Mutex<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    >,
    watch_tx: tokio::sync::watch::Sender<()>,
    rx_audio: Arc<Mutex<Receiver<Vec<u8>>>>,
) -> Result<()> {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(stdout, "\n\rJoining room {}\n\r", room.name).unwrap();
    stdout.flush().unwrap();
    drop(stdout);

    connect_to_room_users(
        room.clone(),
        user.clone(),
        peer_connections.clone(),
        ws_stream.clone(),
        watch_tx.clone(),
        rx_audio.clone(),
    )
    .await?;

    let _ = send_message(
        ws_stream.clone(),
        &CommandMessage {
            user_id: user.id.clone(),
            command: Command::Client(ClientCommand::Join(room.id)),
        },
    )
    .await;

    Ok(())
}

pub async fn display_rooms(
    rooms: Vec<Room>,
    user: Arc<UserConfig>,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ws_stream: Arc<
        tokio::sync::Mutex<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    >,
    watch_tx: tokio::sync::watch::Sender<()>,
    rx_audio: Arc<Mutex<Receiver<Vec<u8>>>>,
) -> Result<()> {
    let mut stdout = std::io::stdout().into_raw_mode().unwrap();

    write!(stdout, "\n\rAvailable rooms:\n\r").unwrap();
    stdout.flush().unwrap();
    drop(stdout);

    for index in 0..rooms.len() {
        let room: Room = rooms[index].clone();
        display_room(room, index);
    }

    let mut stdout = std::io::stdout().into_raw_mode().unwrap();
    write!(stdout, "\n\r").unwrap();
    stdout.flush().unwrap();
    drop(stdout);

    let stdin = stdin();

    for key in stdin.keys() {
        match key.unwrap() {
            Key::Ctrl('c') => break,
            Key::Char(key) => {
                // Check if the key is a digit
                if key.is_ascii_digit() {
                    let index = key.to_digit(10).unwrap() as usize;
                    if index < rooms.len() {
                        let room: Room = rooms[index].clone();
                        join_room(
                            room,
                            user.clone(),
                            peer_connections.clone(),
                            ws_stream.clone(),
                            watch_tx.clone(),
                            rx_audio.clone(),
                        )
                        .await?;
                        break;
                    }
                }
            }
            _ => (),
        }
    }

    Ok(())
}

pub async fn display_empty_room(
    tx: Sender<()>,
    user: Arc<UserConfig>,
    ws_stream: Arc<
        tokio::sync::Mutex<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    >,
) -> Result<()> {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(
        stdout,
        "\rNo rooms found\n
				\r - Press ctrl+n to create a new room
				\r - Press ctrl+c to quit\n\r",
    )
    .unwrap();

    stdout.flush().unwrap();

    let stdin = stdin();

    for key in stdin.keys() {
        match key.unwrap() {
            Key::Ctrl('n') => {
                let _ = create_room(user.clone(), ws_stream.clone()).await;
                break;
            }
            Key::Ctrl('c') => {
                let _ = tx.send(());
                break;
            }
            _ => (),
        }
    }

    stdout.flush().unwrap();
    drop(stdout);

    Ok(())
}

pub async fn create_room(
    user: Arc<UserConfig>,
    ws_stream: Arc<
        tokio::sync::Mutex<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    >,
) -> Result<()> {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(stdout, "\n\rEnter the room name:\n\r",).unwrap();

    stdout.flush().unwrap();

    let mut room_name = String::new();

    let stdin = stdin();
    for key in stdin.keys() {
        match key.unwrap() {
            Key::Ctrl('c') => {
                return Ok(());
            }
            Key::Char('\n') => break,
            Key::Char(k) => {
                // Save the key to the room name
                room_name.push(k);
                let (_, y) = stdout.cursor_pos().unwrap();
                write!(
                    stdout,
                    "{}{}{}",
                    termion::clear::CurrentLine,
                    termion::cursor::Goto(0, y),
                    room_name
                )
                .unwrap();

                stdout.lock().flush().unwrap();
            }

            _ => (),
        }
    }

    stdout.flush().unwrap();
    drop(stdout);

    let room_name = room_name.trim().to_owned();

    let _ = send_message(
        ws_stream.clone(),
        &CommandMessage {
            user_id: user.id.clone(),
            command: Command::Client(ClientCommand::CreateRoom(room_name)),
        },
    )
    .await;

    Ok(())
}

pub async fn connect_to_room_users(
    room: Room,
    user: Arc<UserConfig>,
    peer_connections: Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    ws_stream: Arc<
        tokio::sync::Mutex<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    >,
    watch_tx: tokio::sync::watch::Sender<()>,
    rx_audio: Arc<Mutex<Receiver<Vec<u8>>>>,
) -> Result<()> {
    if room.users.len() > 0 {
        for index in 0..room.users.len() {
            let other_id = room.users[index].clone();

            if other_id == user.id {
                continue;
            }

            connect_peer(
                user.id.clone(),
                other_id,
                room.id.clone(),
                peer_connections.clone(),
                ws_stream.clone(),
                watch_tx.clone(),
                rx_audio.clone(),
            )
            .await?;
        }
    }

    Ok(())
}
