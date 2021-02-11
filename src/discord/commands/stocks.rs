use crate::{discord::get_data_or_insert_with, serialization::first_entry};
use reqwest::{Client, Url};
use serde::Deserialize;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::TypeMapKey,
};
use std::time::Duration;

struct ClientKey;

impl TypeMapKey for ClientKey {
    type Value = Result<Client, String>;
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    quotes: Vec<SearchResponseQuote>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponseQuote {
    exchange: String,
    #[serde(rename = "isYahooFinance")]
    is_yahoo_finance: bool,
    longname: Option<String>,
    #[serde(rename = "quoteType")]
    quote_type: String,
    shortname: Option<String>,
    symbol: String,
}

async fn search(
    client: &Client,
    query: impl AsRef<str>,
) -> CommandResult<Option<SearchResponseQuote>> {
    let response: SearchResponse = client
        .get(Url::parse_with_params(
            "https://query1.finance.yahoo.com/v1/finance/search",
            &[("q", query)],
        )?)
        .send()
        .await?
        .json()
        .await?;
    Ok(response
        .quotes
        .into_iter()
        .find(|quote| quote.is_yahoo_finance))
}

#[derive(Debug, Clone, Deserialize)]
struct HistoryResponse {
    chart: HistoryResponseChart,
}

#[derive(Debug, Clone, Deserialize)]
struct HistoryResponseChart {
    #[serde(deserialize_with = "first_entry")]
    result: Option<HistoryResponseChartResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct HistoryResponseChartResult {
    timestamp: Vec<i64>,
    indicators: HistoryResponseIndicators,
}

#[derive(Debug, Clone, Deserialize)]
struct HistoryResponseIndicators {
    #[serde(deserialize_with = "first_entry")]
    quote: Option<HistoryResponseQuote>,
}

#[derive(Debug, Clone, Deserialize)]
struct HistoryResponseQuote {
    volume: Vec<usize>,
    open: Vec<f64>,
    close: Vec<f64>,
    low: Vec<f64>,
    high: Vec<f64>,
}

async fn get_history(
    client: &Client,
    symbol: impl AsRef<str>,
) -> CommandResult<Option<HistoryResponseChartResult>> {
    let mut url = Url::parse_with_params(
        "https://query1.finance.yahoo.com/v8/finance/chart/",
        &[
            ("range", "1mo"),
            ("interval", "15m"),
            ("includePrePost", "false"),
        ],
    )?;
    // the URL is hardcoded and thus can always be a base: the unwrap is safe
    url.path_segments_mut().unwrap().push(symbol.as_ref());

    let response: HistoryResponse = client.get(url).send().await?.json().await?;

    Ok(response.chart.result)
}

// separate function because lifetime shenanigans
fn chart_time_formatter(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    timestamp.format("%Y-%m-%d").to_string()
}

fn draw_history_chart(
    timestamps: impl IntoIterator<Item = i64>,
    history: &HistoryResponseQuote,
) -> CommandResult<Vec<u8>> {
    use chrono::{Duration, TimeZone, Utc};
    use image::{DynamicImage, ImageOutputFormat, RgbImage};
    use plotters::prelude::*;

    const IMAGE_WIDTH: u32 = 1024;
    const IMAGE_HEIGHT: u32 = 768;

    let timestamps: Vec<_> = timestamps
        .into_iter()
        .map(|t| Utc.timestamp(t, 0))
        .collect();
    let first_timestamp = *timestamps
        .first()
        .ok_or("incomplete history timestamp data")?;
    let last_timestamp = *timestamps
        .last()
        .ok_or("incomplete history timestamp data")?;

    // this check ensures the unwraps and vector indexings later are safe
    if history.open.len() != timestamps.len()
        || history.close.len() != timestamps.len()
        || history.low.len() != timestamps.len()
        || history.high.len() != timestamps.len()
        || history.volume.len() != timestamps.len()
    {
        return Err("history data is missing values".into());
    }

    #[allow(clippy::cast_possible_truncation)]
    let min_price = history
        .low
        .iter()
        .min_by_key(|val| val.floor() as i64)
        .unwrap();
    #[allow(clippy::cast_possible_truncation)]
    let max_price = history
        .high
        .iter()
        .max_by_key(|val| val.ceil() as i64)
        .unwrap();

    /*
    let min_volume = *history.volume.iter().min().unwrap();
    let max_volume = *history.volume.iter().max().unwrap();
    */

    let mut buffer = vec![0_u8; (IMAGE_WIDTH * IMAGE_HEIGHT) as usize * 3];
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (IMAGE_WIDTH, IMAGE_HEIGHT))
            .into_drawing_area();
        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(32)
            .y_label_area_size(64)
            .build_cartesian_2d(
                (first_timestamp - Duration::days(1))..(last_timestamp + Duration::days(1)),
                (min_price * 0.9)..(max_price * 1.1),
            )?
            /*.set_secondary_coord(
                (first_timestamp - Duration::days(1))..(last_timestamp + Duration::days(1)),
                (min_volume)..((max_volume - min_volume) * 10),
            )*/;
        chart
            .configure_mesh()
            .x_label_formatter(&chart_time_formatter)
            .draw()?;
        /*
        chart
            .configure_secondary_axes()
            .x_labels(0)
            .y_labels(0)
            .draw()?;
        */

        #[allow(clippy::cast_possible_truncation)]
        chart.draw_series(timestamps.iter().enumerate().map(|(i, timestamp)| {
            CandleStick::new(
                *timestamp,
                history.open[i],
                history.high[i],
                history.low[i],
                history.close[i],
                GREEN.filled(),
                RED.filled(),
                4,
            )
        }))?;
        /*
        chart.draw_secondary_series(timestamps.iter().enumerate().map(|(i, timestamp)| {
            let mut bar = Rectangle::new(
                [
                    (*timestamp, 0),
                    (*timestamp + Duration::days(1), history.volume[i]),
                ],
                &RGBColor(125, 125, 125),
            );
            bar.set_margin(0, 0, 5, 5);
            bar
        }))?;
        */
    }

    let image = DynamicImage::ImageRgb8(
        RgbImage::from_vec(IMAGE_WIDTH, IMAGE_HEIGHT, buffer).ok_or("invalid image buffer")?,
    );
    let mut file = Vec::new();
    image.write_to(&mut file, ImageOutputFormat::Png)?;
    Ok(file)
}

#[command]
#[aliases(stocks, stonk, stonks)]
#[description("Look up the price of a stock")]
#[usage("GOOGL")]
#[num_args(1)]
async fn stock(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let client = get_data_or_insert_with::<ClientKey, _>(&ctx, || {
        Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            // simplify error to a `String` to make it `impl Clone`
            .map_err(|err| format!("Unable to create reqwest::Client: {}", err))
    })
    .await?;

    if let Some(info) = search(&client, args.rest().trim()).await? {
        let name = info
            .longname
            .as_ref()
            .or_else(|| info.shortname.as_ref())
            .unwrap_or(&info.symbol);
        let symbol = info.symbol.split('.').next().unwrap();
        if let Some(history) = get_history(&client, &info.symbol).await? {
            if let Some(quotes) = history.indicators.quote {
                let chart = draw_history_chart(history.timestamp, &quotes)?;
                msg.channel_id
                    .send_files(&ctx, vec![(chart.as_slice(), "chart.png")], |message| {
                        message.embed(|embed| {
                            embed.title(name);
                            embed.url(format!("https://finance.yahoo.com/quote/{}/", &info.symbol));
                            embed.field("Symbol", &symbol, true);
                            embed.field("Exchange", &info.exchange, true);
                            embed.attachment("chart.png")
                        })
                    })
                    .await?;
            } else {
                msg.reply(
                    ctx,
                    format!("Unable to get quotes for {} ({})", name, info.symbol),
                )
                .await?;
            }
        } else {
            msg.reply(
                ctx,
                format!("Unable to get history for {} ({})", name, info.symbol),
            )
            .await?;
        }
    } else {
        msg.reply(ctx, "No such stock!").await?;
    }

    Ok(())
}
