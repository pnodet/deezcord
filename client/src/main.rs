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

use config::{create_config, get_user_or_create};
use std::io::{stdout, Write};
use std::sync::{mpsc, Arc};
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use uuid::Uuid;

const SERVER_URL: &str = "ws://localhost:3030/ws";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let debug = true;
    let mut user = get_user_or_create()?;

    if debug {
        user = create_config(Uuid::new_v4().to_string().as_str())?;
    }

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

    // {
    //     let user = user.clone();
    //     let ws_stream = ws_stream.clone();
    //     tokio::spawn(async move { listen_for_input(tx, user, ws_stream).await });
    // }

    {
        let tx = tx.clone();
        let user = user.clone();
        let ws_stream = ws_stream.clone();

        tokio::spawn(async move {
            let _ = listen_for_ws(tx, user, ws_stream).await;
        });
    }

    rx.recv().unwrap();

    let mut stdout = stdout();
    write!(stdout, "Shutting down...").unwrap();

    stdout.flush().unwrap();
    Ok(())
}
