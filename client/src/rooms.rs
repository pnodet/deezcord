use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    io::{stdout, Write},
    sync::Arc,
};
use termion::{async_stdin, raw::IntoRawMode};
use termion::{event::Key, input::TermRead};
use tokio_tungstenite::WebSocketStream;

use crate::{
    commands::{ClientCommand, Command, CommandMessage},
    config::UserConfig,
    socket::send::send_message,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub users: Vec<String>,
    pub host_id: String,
}

pub fn display_room(room: Room) {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(stdout, "\n\rRoom {} :", room.name).unwrap();
    for user in room.users {
        write!(stdout, "\r- {}", user).unwrap();
    }
    stdout.flush().unwrap();
}

pub fn display_rooms(rooms: Vec<Room>) {
    let mut stdout = stdout();
    write!(
        stdout,
        "{}{}",
        termion::cursor::Goto(1, 1),
        termion::clear::All
    )
    .unwrap();

    for room in rooms {
        display_room(room);
    }

    stdout.flush().unwrap();
}

pub fn display_empty_room() {
    let mut stdout = stdout();
    write!(
        stdout,
        "\rNo rooms found
				\n\r - Press ctrl+n to create a new room
				\r - Press ctrl+r to refresh rooms
				\r - Press ctrl+c or q to quit",
    )
    .unwrap();

    stdout.flush().unwrap();
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
    write!(
        stdout,
        "{}{}{}Enter the room name:\n\r",
        termion::cursor::Goto(1, 1),
        termion::clear::All,
        termion::cursor::Show
    )
    .unwrap();

    stdout.flush().unwrap();

    let mut room_name = String::new();

    let mut stdin = async_stdin().keys();

    loop {
        let key = stdin.next();

        if let Some(Ok(key)) = key {
            match key {
                Key::Ctrl('c') | Key::Ctrl('q') | Key::Esc | Key::Char('\r') | Key::Char('\n') => {
                    break
                }

                Key::Char(k) => {
                    // Save the key to the room name
                    room_name.push(k);
                    write!(
                        stdout,
                        "{}{}{:?}",
                        termion::clear::All,
                        termion::cursor::Goto(1, 1),
                        room_name
                    )
                    .unwrap();

                    stdout.lock().flush().unwrap();
                }

                _ => (),
            }
        }

        stdout.flush().unwrap();
    }

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
