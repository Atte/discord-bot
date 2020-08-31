use crate::eyre::{Report, Result, WrapErr};
use futures::{Stream, StreamExt};
use reqwest::{Client, IntoUrl};
use std::{io::BufRead, time::Duration};
use tokio::stream::StreamExt as TokioStreamExt;

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: Option<String>,
    pub id: Option<String>,
    pub retry: Option<usize>,
}

impl SseEvent {
    fn new() -> Self {
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
            Err(err) => return futures::stream::iter(vec![Err(Report::new(err))]),
            Ok(chunk) => buffer.extend(chunk),
        }

        // split incomplete line off manually in case it ends with incomplete unicode
        if let Some(index) = buffer.iter().rposition(|c| c == &b'\n') {
            let remainder = buffer.split_off(index);
            let lines: Vec<Result<String>> = buffer
                .lines()
                .map(|line| line.wrap_err("line decode error"))
                .collect();
            buffer = remainder;
            futures::stream::iter(lines)
        } else {
            futures::stream::iter(Vec::new())
        }
    });

    let mut line_buffer: Vec<String> = Vec::new();
    let events = lines
        .timeout(Duration::from_secs(30))
        .flat_map(move |line| {
            match line {
                Err(err) => futures::stream::iter(vec![Err(Report::new(err))]),
                Ok(Err(err)) => futures::stream::iter(vec![Err(err)]),
                // events are delimited by empty lines
                Ok(Ok(line)) if line.is_empty() => {
                    let mut event = SseEvent::new();
                    for line in &line_buffer {
                        let (name, value) =
                            if let Some(index) = line.bytes().position(|c| c == b':') {
                                let value = &line[index + 1..];
                                (
                                    &line[..index],
                                    if value.starts_with(' ') {
                                        &value[1..]
                                    } else {
                                        value
                                    },
                                )
                            } else {
                                (line.as_ref(), "")
                            };
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
