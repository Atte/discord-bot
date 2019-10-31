use serde::{Deserialize, Serialize};
use std::fmt;

use super::{Error, ErrorKind, Result};

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
    pub(super) fn new(typ: SocketIOMessageType) -> Self {
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
