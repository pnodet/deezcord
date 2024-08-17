mod commands;
mod config;
mod connect;
mod input;
mod rooms;
mod socket;

use crate::commands::wait_for_ack::wait_for_ack;
use crate::commands::{ClientCommand, Command, CommandMessage};
use crate::socket::listen::listen_for_ws;
use crate::socket::send::send_message;

use config::get_user_or_create;
use input::listen::listen_for_input;
use std::io::{stdout, Write};
use std::sync::{mpsc, Arc};
use termion::raw::IntoRawMode;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;

const SERVER_URL: &str = "ws://localhost:3030/ws";

#[derive(Clone, Debug)]
struct User {
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
    let user = Arc::new(user);

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

    send_message(
        ws_stream.clone(),
        &CommandMessage {
            user_id: user.id.clone(),
            command: Command::Client(ClientCommand::ListRooms),
        },
    )
    .await?;

    let (tx, rx): (mpsc::Sender<()>, mpsc::Receiver<()>) = mpsc::channel();

    {
        let user = user.clone();
        let ws_stream = ws_stream.clone();
        tokio::spawn(async move { listen_for_input(tx, user, ws_stream).await });
    }

    {
        let user = user.clone();
        let ws_stream = ws_stream.clone();
        tokio::spawn(async move {
            let _ = listen_for_ws(user, ws_stream).await;
        });
    }

    rx.recv().unwrap();

    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(stdout, "{}{}", termion::cursor::Show, termion::clear::All).unwrap();

    stdout.flush().unwrap();
    Ok(())
}
