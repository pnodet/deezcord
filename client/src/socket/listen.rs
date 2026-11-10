use crate::audio::capture::capture_audio;
use crate::commands::{Command, CommandMessage, ServerCommand};
use crate::config::UserConfig;
use crate::peer::{
    handle_answer::handle_answer, handle_ice_candidate::handle_ice_candidate,
    handle_offer::handle_offer,
};
use crate::rooms::{display_empty_room, display_room, display_rooms};

use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::io::{stdout, Write};
use std::sync::mpsc::{self, Sender};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{watch, Mutex};
use tokio_tungstenite::WebSocketStream;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::{sdp::session_description::RTCSessionDescription, RTCPeerConnection};

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

    let (tx_audio, rx_audio): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel();

    let rx_audio = Arc::new(Mutex::new(rx_audio));
    let tx_audio = Arc::new(Mutex::new(tx_audio));

    let audio_capture_started = Arc::new(AtomicBool::new(false));

    let (watch_tx, mut watch_rx) = watch::channel(());

    tokio::spawn(async move {
        while watch_rx.changed().await.is_ok() {
            println!("\n\rAudio capture changed");
        }
    });

    if !audio_capture_started.load(Ordering::SeqCst) {
        println!("\n\rStarting audio capture");
        audio_capture_started.store(true, Ordering::SeqCst);
        tokio::spawn(capture_audio(tx_audio));
    }

    loop {
        let mut stdout = stdout();
        let msg = {
            let mut ws_stream = ws_stream.lock().await;
            ws_stream.next().await
        };

        if msg.is_none() {
            break Ok(());
        }

        let msg = msg.unwrap().context("WebSocket connection failed")?;

        if msg.is_close() {
            break Ok(());
        }

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

        let command_message: CommandMessage = match serde_json::from_str(text) {
            Ok(command_message) => command_message,
            Err(e) => {
                write!(
                    stdout,
                    "\n\rIgnoring unreadable server message ({e}): {text}\n\r"
                )
                .unwrap();
                stdout.flush().unwrap();
                continue;
            }
        };

        if command_message.user_id != user.id {
            continue;
        }

        // write!(stdout, "\n\n*** Received ***\n{:?}", command_message).unwrap();
        // stdout.flush().unwrap();

        let handled: Result<()> = match command_message.command {
            Command::Server(ServerCommand::RoomList(rooms)) => {
                let current_room = rooms.iter().find(|room| room.users.contains(&user.id));

                if let Some(current_room) = current_room {
                    display_room(current_room.clone(), 0);
                    Ok(())
                } else if rooms.is_empty() {
                    display_empty_room(tx.clone(), user.clone(), ws_stream.clone()).await
                } else {
                    display_rooms(
                        rooms,
                        user.clone(),
                        peer_connections.clone(),
                        ws_stream.clone(),
                        watch_tx.clone(),
                        rx_audio.clone(),
                    )
                    .await
                }
            }

            Command::Server(ServerCommand::IncomingIceCandidate(
                from_user,
                _room_id,
                candidate,
            )) => handle_ice_candidate(from_user, candidate, ice_candidates.clone()).await,

            Command::Server(ServerCommand::IncomingAnswer(from_user, _room_id, sdp)) => {
                handle_answer(from_user, sdp, peer_connections.clone()).await
            }

            Command::Server(ServerCommand::IncomingOffer(from_user, room_id, sdp)) => {
                match RTCSessionDescription::offer(sdp) {
                    Ok(offer) => {
                        handle_offer(
                            from_user,
                            room_id,
                            offer,
                            &user.id,
                            peer_connections.clone(),
                            ice_candidates.clone(),
                            ws_stream.clone(),
                            watch_tx.clone(),
                            rx_audio.clone(),
                        )
                        .await
                    }
                    Err(e) => Err(e.into()),
                }
            }

            _ => {
                write!(stdout, "\n\rUnexpected message: {:?}", command_message).unwrap();
                stdout.flush().unwrap();
                Ok(())
            }
        };

        // A failed handler only affects that one message (one room action or one peer
        // handshake), so report it and keep listening.
        if let Err(e) = handled {
            write!(stdout, "\n\rFailed to handle server message: {e:#}\n\r").unwrap();
            stdout.flush().unwrap();
        }

        watch_tx.send(()).unwrap();
    }
}
