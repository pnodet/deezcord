mod audio;
mod commands;
mod config;
mod input;
mod peer;
mod rooms;
mod socket;

use crate::commands::wait_for_ack::wait_for_ack;
use crate::commands::{ClientCommand, Command, CommandMessage};
use crate::socket::listen::listen_for_ws;
use crate::socket::send::send_message;

use config::load_user;
use futures_util::StreamExt;
use std::io::{stdout, Write};
use std::sync::{mpsc, Arc};
use tokio::sync::Mutex;
use tokio::task::JoinError;
use tokio_tungstenite::connect_async;

const SERVER_URL: &str = "ws://localhost:3030/ws";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = Arc::new(load_user()?);

    let (ws_stream, _) = connect_async(SERVER_URL).await.expect("Failed to connect");
    let (ws_write, mut ws_read) = ws_stream.split();
    let ws_stream = Arc::new(Mutex::new(ws_write));

    send_message(
        ws_stream.clone(),
        &CommandMessage {
            user_id: user.id.clone(),
            command: Command::Client(ClientCommand::Connect(user.name.clone())),
        },
    )
    .await?;
    wait_for_ack(&mut ws_read).await?;

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

    // `tx` moves into the listener: once the listener ends (returns or panics) no
    // sender is left, so `rx.recv()` returns `Err` instead of blocking forever.
    let listener = tokio::spawn(listen_for_ws(tx, user, ws_read, ws_stream));

    let quit_requested = rx.recv().is_ok();

    let mut stdout = stdout();
    if quit_requested {
        write!(stdout, "Shutting down...").unwrap();
        stdout.flush().unwrap();
        return Ok(());
    }

    match listener_exit(listener.await) {
        Ok(()) => {
            write!(
                stdout,
                "\n\rServer closed the connection. Shutting down.\n\r"
            )
            .unwrap();
            stdout.flush().unwrap();
            Ok(())
        }
        Err(reason) => {
            eprintln!("\n\r{reason}");
            std::process::exit(1);
        }
    }
}

/// Maps how the listener task ended to main's exit: `Ok` for a clean close by the
/// server, `Err(reason)` for an exit with a non-zero status.
fn listener_exit(joined: Result<anyhow::Result<()>, JoinError>) -> Result<(), String> {
    match joined {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(format!("Lost the connection to the server: {e:#}")),
        Err(e) if e.is_panic() => Err("The client crashed (panic message above).".to_string()),
        Err(e) => Err(format!("The server listener stopped: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::listener_exit;

    #[test]
    fn clean_close_exits_ok() {
        assert_eq!(listener_exit(Ok(Ok(()))), Ok(()));
    }

    #[test]
    fn listener_error_exits_with_reason() {
        let reason = listener_exit(Ok(Err(anyhow::anyhow!("connection reset")))).unwrap_err();
        assert!(reason.contains("connection reset"), "{reason}");
    }

    #[tokio::test]
    async fn listener_panic_exits_with_reason() {
        let joined = tokio::spawn(async { panic!("boom") }).await;
        assert!(listener_exit(joined).unwrap_err().contains("crashed"));
    }
}
