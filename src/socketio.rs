use error_chain::error_chain;
use log::trace;
use reqwest;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt,
    time::{Duration, Instant},
};
use websocket::{
    client::{sync::Client, ClientBuilder, Url},
    message::{Message, OwnedMessage},
    stream::sync::NetworkStream,
    WebSocketError,
};

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Discord(::serenity::Error);
        Http(::reqwest::Error);
        Json(::serde_json::Error);
        IntParse(::std::num::ParseIntError);
        Websocket(::websocket::WebSocketError);
    }

    errors {
        InvalidUrl(url: String, reason: &'static str) {
            description("invalid URL")
            display("invalid URL {}: {}", url, reason)
        }

        InvalidHandshake(content: String) {
            description("invalid handshake")
            display("invalid handshake: {}", content)
        }

        UnsupportedTransports(transports: String) {
            description("unsupported transports")
            display("unsupported transports: {}", transports)
        }

        ParseMessage(reason: &'static str) {
            description("invalid message")
            display("invalid message: {}", reason)
        }

        WebsocketClose(reason: String) {
            description("got websocket close message")
            display("got websocket close message: {}", reason)
        }
    }
}

#[derive(Debug)]
pub enum SocketIOMessageType {
    Disconnect = 0,
    Connect = 1,
    Heartbeat = 2,
    Message = 3,
    Json = 4,
    Event = 5,
    Ack = 6,
    Error = 7,
    Noop = 8,
}

impl fmt::Display for SocketIOMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Disconnect => 0,
                Self::Connect => 1,
                Self::Heartbeat => 2,
                Self::Message => 3,
                Self::Json => 4,
                Self::Event => 5,
                Self::Ack => 6,
                Self::Error => 7,
                Self::Noop => 8,
            }
        )
    }
}

impl ::std::str::FromStr for SocketIOMessageType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "0" => Ok(Self::Disconnect),
            "1" => Ok(Self::Connect),
            "2" => Ok(Self::Heartbeat),
            "3" => Ok(Self::Message),
            "4" => Ok(Self::Json),
            "5" => Ok(Self::Event),
            "6" => Ok(Self::Ack),
            "7" => Ok(Self::Error),
            "8" => Ok(Self::Noop),
            _ => Err(ErrorKind::ParseMessage("unknown message type").into()),
        }
    }
}

#[derive(Debug)]
pub struct SocketIOMessage {
    pub typ: SocketIOMessageType,
    pub id: Option<usize>,
    pub endpoint: Option<String>,
    pub data: Option<String>,
}

impl SocketIOMessage {
    fn new(typ: SocketIOMessageType) -> Self {
        Self {
            typ,
            id: None,
            endpoint: None,
            data: None,
        }
    }
}

impl fmt::Display for SocketIOMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.typ,
            self.id.map_or_else(String::new, |id| id.to_string()),
            self.endpoint.as_ref().unwrap_or(&String::new())
        )?;
        if let Some(ref data) = self.data {
            write!(f, ":{}", data)?;
        }
        Ok(())
    }
}

impl ::std::str::FromStr for SocketIOMessage {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.splitn(4, ':').collect::<Vec<&str>>().as_slice() {
            [typ, id, endpoint, data] => Ok(Self {
                typ: typ.parse()?,
                id: if id.is_empty() { None } else { id.parse().ok() },
                endpoint: if endpoint.is_empty() {
                    None
                } else {
                    Some(endpoint.to_string())
                },
                data: Some(data.to_string()),
            }),
            [typ, id, endpoint] => Ok(Self {
                typ: typ.parse()?,
                id: if id.is_empty() { None } else { id.parse().ok() },
                endpoint: if endpoint.is_empty() {
                    None
                } else {
                    Some(endpoint.to_string())
                },
                data: None,
            }),
            _ => Err(ErrorKind::ParseMessage("wrong number of elements").into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SocketIOEvent {
    pub name: String,
    pub args: Vec<::serde_json::Value>,
}

pub struct SocketIO {
    origin: Url,
    namespace: String,
    client: Option<Client<Box<dyn NetworkStream + Send>>>,
    heartbeat_interval: Option<Duration>,
    last_heartbeat: Instant,
}

impl SocketIO {
    pub fn new(origin: Url) -> Self {
        Self::new_with_namespace(origin, "socket.io")
    }

    pub fn new_with_namespace(origin: Url, namespace: impl Into<String>) -> Self {
        Self {
            origin,
            namespace: namespace.into(),
            client: None,
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

    pub fn connect(&mut self) -> Result<()> {
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
        self.client = Some(ClientBuilder::from_url(&socket_url).connect(None)?);
        Ok(())
    }

    pub fn run<F>(&mut self, mut event_callback: F) -> Result<()>
    where
        F: FnMut(SocketIOEvent) -> Option<SocketIOMessage>,
    {
        if let Some(ref mut client) = self.client {
            client.set_nonblocking(true).unwrap();
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
                            if let (SocketIOMessageType::Event, Some(ref data)) =
                                (msg.typ, msg.data)
                            {
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
                    Ok(OwnedMessage::Close(_)) => {
                        return Err(ErrorKind::WebsocketClose("no reason".to_owned()).into());
                    }
                }

                if let Some(interval) = self.heartbeat_interval {
                    let now = Instant::now();
                    if now.duration_since(self.last_heartbeat) >= interval {
                        let msg = SocketIOMessage::new(SocketIOMessageType::Heartbeat).to_string();
                        client.send_message(&Message::text(msg))?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CONFIG;

    #[test]
    fn handshake() {
        let sock = SocketIO::new(CONFIG.berrytube.origin.parse().unwrap());
        let (sid, _hbeat, _conn) = sock.handshake().unwrap();
        assert!(!sid.is_empty());
    }

    #[test]
    fn connect() {
        let mut sock = SocketIO::new(CONFIG.berrytube.origin.parse().unwrap());
        sock.connect().unwrap();
        assert!(sock.client.is_some());
    }
}
