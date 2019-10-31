use log::trace;
use reqwest;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use websocket::{
    client::Url,
    sync::{
        stream::{AsTcpStream, NetworkStream},
        Client, Stream,
    },
    ClientBuilder, Message, OwnedMessage, WebSocketError,
};

use super::{models::*, ErrorKind, Result};

pub struct SocketIOClient {
    origin: Url,
    namespace: String,
    heartbeat_interval: Option<Duration>,
    last_heartbeat: Instant,
}

impl SocketIOClient {
    pub fn new(origin: Url) -> Self {
        Self::new_with_namespace(origin, "socket.io")
    }

    pub fn new_with_namespace(origin: Url, namespace: impl Into<String>) -> Self {
        Self {
            origin,
            namespace: namespace.into(),
            heartbeat_interval: None,
            last_heartbeat: Instant::now(),
        }
    }

    fn handshake(&self) -> Result<(String, Option<u64>, Option<u64>)> {
        let handshake_url = {
            let mut url = self.origin.clone();
            url.path_segments_mut()
                .map_err(|_| ErrorKind::InvalidUrl(self.origin.to_string(), "cannot be base"))?
                .push(&self.namespace)
                .push("1");
            url
        };
        let response = reqwest::get(handshake_url)?.error_for_status()?.text()?;

        match response.split(':').collect::<Vec<&str>>().as_slice() {
            [sid, hbeat_timeout, conn_timeout, transports_string] => {
                let transports: HashSet<&str> = transports_string.split(',').collect();
                if transports.contains("websocket") {
                    trace!("socket.io sid: {}", sid);
                    Ok((
                        sid.to_string(),
                        hbeat_timeout.parse().ok(),
                        conn_timeout.parse().ok(),
                    ))
                } else {
                    Err(ErrorKind::UnsupportedTransports(transports_string.to_string()).into())
                }
            }
            _ => Err(ErrorKind::InvalidHandshake(response).into()),
        }
    }

    fn connect(&mut self) -> Result<Client<Box<dyn NetworkStream + Send>>> {
        let (sid, hbeat_timeout, _close_timeout) = self.handshake()?;
        self.heartbeat_interval = hbeat_timeout.and_then(|timeout| {
            if timeout < 1 {
                None
            } else {
                Some(Duration::from_secs(timeout / 2))
            }
        });

        let socket_url = {
            let mut url = self.origin.clone();
            url.set_scheme(if url.scheme() == "http" { "ws" } else { "wss" })
                .map_err(|_| ErrorKind::InvalidUrl(self.origin.to_string(), "cannot be base"))?;
            url.path_segments_mut()
                .map_err(|_| ErrorKind::InvalidUrl(self.origin.to_string(), "cannot be base"))?
                .push(&self.namespace)
                .push("1")
                .push("websocket")
                .push(&sid);
            url
        };

        Ok(ClientBuilder::from_url(&socket_url).connect(None)?)
    }

    fn message_loop<S, F>(&mut self, mut client: Client<S>, mut event_callback: F) -> Result<()>
    where
        S: Stream + AsTcpStream,
        F: FnMut(SocketIOEvent) -> Option<SocketIOMessage>,
    {
        client.set_nonblocking(true)?;
        loop {
            match client.recv_message() {
                Err(WebSocketError::IoError(ref err))
                    if err.kind() == ::std::io::ErrorKind::WouldBlock =>
                {
                    ::std::thread::sleep(Duration::from_secs(1));
                }
                Err(err) => return Err(err.into()),
                Ok(OwnedMessage::Ping(data)) => {
                    client.send_message(&Message::pong(data))?;
                }
                Ok(OwnedMessage::Pong(_)) | Ok(OwnedMessage::Binary(_)) => { /* noop */ }
                Ok(OwnedMessage::Text(text)) => {
                    if let Ok(msg) = text.parse::<SocketIOMessage>() {
                        // trace!("{:?}", msg);
                        if let (SocketIOMessageType::Event, Some(ref data)) = (msg.typ, msg.data) {
                            let event = serde_json::from_str(data)?;
                            if let Some(response) = event_callback(event) {
                                client.send_message(&Message::text(response.to_string()))?;
                            }
                        }
                    }
                }
                Ok(OwnedMessage::Close(Some(data))) => {
                    return Err(ErrorKind::WebsocketClose(data.reason).into());
                }
                Ok(OwnedMessage::Close(None)) => {
                    return Err(ErrorKind::WebsocketClose("no reason".to_owned()).into());
                }
            }

            if let Some(interval) = self.heartbeat_interval {
                let now = Instant::now();
                if now.duration_since(self.last_heartbeat) >= interval {
                    let msg = SocketIOMessage::new(SocketIOMessageType::Heartbeat).to_string();
                    client.send_message(&Message::text(msg))?;
                    self.last_heartbeat = now;
                }
            }
        }
    }

    pub fn run<F>(&mut self, event_callback: F) -> Result<()>
    where
        F: FnMut(SocketIOEvent) -> Option<SocketIOMessage>,
    {
        let client = self.connect()?;
        self.message_loop(client, event_callback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CONFIG;

    #[test]
    fn handshake() {
        let sock = SocketIOClient::new(CONFIG.berrytube.origin.to_string().parse().unwrap());
        let (sid, _hbeat, _conn) = sock.handshake().unwrap();
        assert!(!sid.is_empty());
    }

    #[test]
    fn connect() {
        let mut sock = SocketIOClient::new(CONFIG.berrytube.origin.to_string().parse().unwrap());
        sock.connect().unwrap();
    }
}
