use color_eyre::eyre::{Result, WrapErr};
use futures::{Stream, StreamExt};
use reqwest::{Client, IntoUrl};
use std::{io::BufRead, time::Duration};
use tokio_stream::StreamExt as TokioStreamExt;

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: Option<String>,
    pub id: Option<String>,
    pub retry: Option<usize>,
}

impl SseEvent {
    #[inline]
    const fn new() -> Self {
        Self {
            event: None,
            data: None,
            id: None,
            retry: None,
        }
    }
}

// https://html.spec.whatwg.org/multipage/server-sent-events.html
pub async fn stream_sse_events(url: impl IntoUrl) -> Result<impl Stream<Item = Result<SseEvent>>> {
    let client = Client::builder()
        .user_agent("discord-bot")
        .connect_timeout(Duration::from_secs(10))
        .build()?;

    let bytes = client.get(url).send().await?.bytes_stream();

    let mut buffer: Vec<u8> = Vec::new();
    let lines = bytes.flat_map(move |chunk| {
        match chunk {
            Err(err) => return futures::stream::iter(vec![Err(err.into())]),
            Ok(chunk) => buffer.extend(chunk),
        }

        // split incomplete line off manually in case it ends with incomplete unicode
        buffer.iter().rposition(|c| c == &b'\n').map_or_else(
            || futures::stream::iter(Vec::new()),
            |index| {
                let remainder = buffer.split_off(index);
                let lines: Vec<Result<String>> = buffer
                    .lines()
                    .map(|line| line.wrap_err("line decode error"))
                    .collect();
                buffer = remainder;
                futures::stream::iter(lines)
            },
        )
    });

    let mut line_buffer: Vec<String> = Vec::new();
    let events = lines
        .timeout(Duration::from_secs(30))
        .flat_map(move |line| {
            match line {
                // timeout
                Err(err) => futures::stream::iter(vec![Err(err.into())]),
                // some other error
                Ok(Err(err)) => futures::stream::iter(vec![Err(err)]),
                // empty line (event delimiter)
                Ok(Ok(line)) if line.is_empty() => {
                    let mut event = SseEvent::new();
                    for line in &line_buffer {
                        let (name, value) = line.bytes().position(|c| c == b':').map_or_else(
                            || (line.as_ref(), ""),
                            |index| {
                                let value = &line[index + 1..];
                                (&line[..index], value.strip_prefix(' ').unwrap_or(value))
                            },
                        );
                        match name {
                            "id" => {
                                if value != "\0" {
                                    event.id = Some(String::from(value));
                                }
                            }
                            "event" => event.event = Some(String::from(value)),
                            "retry" => event.retry = value.parse().ok().or(event.retry),
                            "data" => {
                                if let Some(ref mut existing) = event.data {
                                    existing.push('\n');
                                    existing.push_str(value);
                                } else {
                                    event.data = Some(String::from(value));
                                }
                            }
                            _ => {} // spec says to ignore unknown fields
                        }
                    }
                    line_buffer.clear();
                    futures::stream::iter(vec![Ok(event)])
                }
                // event content
                Ok(Ok(line)) => {
                    // don't bother storing comments
                    if !line.starts_with(':') {
                        line_buffer.push(line);
                    }
                    futures::stream::iter(Vec::new())
                }
            }
        });

    Ok(events)
}
