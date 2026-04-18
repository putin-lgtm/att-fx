use att_fx::market::{format_price, normalize_price, split_symbol};
use chrono::Local;
use flate2::read::GzDecoder;
use futures_util::{Sink, SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::env;
use std::io::Read;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const WS_URL: &str = "wss://api.huobi.br.com/ws";
const DEFAULT_SYMBOL: &str = "solusdt";

fn to_huobi_topic(symbol: &str) -> String {
    format!("market.{symbol}.ticker")
}

fn extract_price(payload: &str) -> Option<Decimal> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return None;
    };

    let price = value
        .get("tick")
        .and_then(|tick| tick.get("close"))
        .and_then(Value::as_f64)
        .map(|value| value.to_string())
        .or_else(|| {
            value
                .get("tick")
                .and_then(|tick| tick.get("close"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })?;

    normalize_price(&price)
}

fn print_price_update(payload: &str, symbol: &str, last_price: &mut Option<Decimal>) {
    let Some(price) = extract_price(payload) else {
        return;
    };

    if last_price.as_ref() == Some(&price) {
        return;
    }

    *last_price = Some(price);

    let (base, quote) = split_symbol(symbol);
    let base = base.to_uppercase();
    let quote = quote.to_uppercase();
    let formatted_price = format_price(price);
    let formatted = if quote.is_empty() {
        formatted_price
    } else {
        format!("{formatted_price} {quote}")
    };

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S.%3f ms");
    println!("[{}] {}", timestamp, json!({ base: formatted }));
}

fn log_event(payload: &str) {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return;
    };

    if let Some(status) = value.get("status") {
        eprintln!("huobi status: {}", status);
    }

    if let Some(subbed) = value.get("subbed").and_then(Value::as_str) {
        eprintln!("huobi subscribed: {}", subbed);
    }
}

fn decode_gzip_payload(bytes: &[u8]) -> Option<String> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decoded = String::new();
    decoder.read_to_string(&mut decoded).ok()?;
    Some(decoded)
}

async fn handle_huobi_payload<W>(
    payload: &str,
    symbol: &str,
    last_price: &mut Option<Decimal>,
    write: &mut W,
) -> Result<(), Box<dyn std::error::Error>>
where
    W: Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return Ok(());
    };

    if let Some(ping) = value.get("ping") {
        let pong = json!({ "pong": ping });
        write.send(Message::Text(pong.to_string().into())).await?;
        return Ok(());
    }

    print_price_update(payload, symbol, last_price);
    log_event(payload);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let symbol = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SYMBOL.to_string())
        .to_lowercase();
    let topic = to_huobi_topic(&symbol);

    let (ws_stream, response) = connect_async(WS_URL).await?;
    eprintln!("connected: {} {}", response.status(), WS_URL);
    eprintln!("subscribing symbol: {}", symbol.to_uppercase());

    let (mut write, mut read) = ws_stream.split();

    let subscribe_message = json!({
        "sub": topic,
        "id": "id1"
    });

    write
        .send(Message::Text(subscribe_message.to_string().into()))
        .await?;

    let mut last_price: Option<Decimal> = None;

    while let Some(frame) = read.next().await {
        match frame? {
            Message::Text(text) => {
                handle_huobi_payload(&text, &symbol, &mut last_price, &mut write).await?;
            }
            Message::Binary(bytes) => {
                if let Some(payload) = decode_gzip_payload(&bytes) {
                    handle_huobi_payload(&payload, &symbol, &mut last_price, &mut write).await?;
                }
            }
            Message::Ping(payload) => write.send(Message::Pong(payload)).await?,
            Message::Pong(_) => {}
            Message::Close(close_frame) => {
                if let Some(frame) = close_frame {
                    eprintln!("connection closed: {} {}", frame.code, frame.reason);
                } else {
                    eprintln!("connection closed by server");
                }
                break;
            }
            Message::Frame(_) => {}
        }
    }

    Ok(())
}