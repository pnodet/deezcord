use futures_util::stream::{SplitSink, SplitStream};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub mod listen;
pub mod send;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Write half of the server WebSocket, shared by every task that sends.
pub type WsWriter = Arc<Mutex<SplitSink<WsStream, Message>>>;

/// Read half of the server WebSocket. Only one reader exists: `wait_for_ack`
/// borrows it at startup, then `listen_for_ws` owns it.
pub type WsReader = SplitStream<WsStream>;
