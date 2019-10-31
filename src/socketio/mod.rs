use error_chain::error_chain;

mod client;
mod models;

pub use client::*;
pub use models::*;

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
