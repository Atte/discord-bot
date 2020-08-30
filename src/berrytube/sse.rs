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

        if let Some(index) = buffer.iter().rposition(|c| c == &b'\n') {
            let remainder = buffer.split_off(index);
            let lines: Vec<Result<String>> = buffer
                .lines()
                .map(|line| line.wrap_err("Line decode error"))
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
                Ok(Ok(line)) if line.is_empty() => {
                    let mut event = SseEvent::new();
                    for line in &line_buffer {
                        let (name, value) =
                            if let Some(index) = line.bytes().position(|c| c == b':') {
                                (&line[..index], line[index + 1..].trim().to_owned())
                            } else {
                                (line.as_ref(), String::new())
                            };
                        match name {
                            "event" => event.event = Some(value),
                            "data" => {
                                if let Some(existing) = event.data {
                                    event.data = Some(format!("{}\n{}", existing, value));
                                } else {
                                    event.data = Some(value);
                                }
                            }
                            "id" => event.id = Some(value),
                            "retry" => event.retry = value.parse().ok(),
                            _ => {} // spec says to ignore unknown fields
                        }
                    }
                    line_buffer.clear();
                    futures::stream::iter(vec![Ok(event)])
                }
                Ok(Ok(line)) => {
                    if !line.starts_with(':') {
                        line_buffer.push(line);
                    }
                    futures::stream::iter(Vec::new())
                }
            }
        });

    Ok(events)
}
