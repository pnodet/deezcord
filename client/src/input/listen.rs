use std::io::{stdin, stdout, Write};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use termion::async_stdin;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tokio_tungstenite::WebSocketStream;

use crate::config::UserConfig;
use crate::rooms::create_room;

pub async fn listen_for_input(
    tx: Sender<()>,
    user: Arc<UserConfig>,
    ws_stream: Arc<
        tokio::sync::Mutex<
            WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        >,
    >,
) {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(
        stdout,
        "{}{}{}",
        termion::cursor::Goto(1, 1),
        termion::clear::All,
        termion::cursor::Hide
    )
    .unwrap();

    stdout.flush().unwrap();

    let mut stdin = async_stdin().keys();

    loop {
        let key = stdin.next();

        if let Some(Ok(key)) = key {
            match key {
                Key::Ctrl('r') => println!("Refresh!"),
                Key::Ctrl('n') => {
                    let _ = create_room(user.clone(), ws_stream.clone()).await;
                }
                Key::Char('q') | Key::Ctrl('q') | Key::Ctrl('c') => {
                    let _ = tx.send(());
                    return;
                }
                _ => (),
            }
        }

        stdout.flush().unwrap();
    }
}
