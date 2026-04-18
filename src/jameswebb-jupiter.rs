use att_fx::market::{format_price, normalize_price, split_symbol};
use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::env;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const WS_URL: &str = "wss://quote-api.jup.ag/v6/ws";
const DEFAULT_SYMBOL: &str = "solusdt";

fn extract_level(side: &Value, index: usize) -> Option<(&str, &str)> {
    let level = side.get(index)?.as_array()?;
    let price = level.first()?.as_str()?;
    let size = level.get(1)?.as_str()?;
    Some((price, size))
}

fn extract_price(payload: &str) -> Option<Decimal> {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return None;
    };

    let Some(book) = value
        .get("data")
        .and_then(Value::as_array)
        .and_then(|data| data.first())
    else {
        return None;
    };

    let best_bid = book.get("bids").and_then(|bids| extract_level(bids, 0));
    let best_ask = book.get("asks").and_then(|asks| extract_level(asks, 0));

    let price = match (best_bid, best_ask) {
        (_, Some((ask_price, ask_size))) if ask_size != "0" => ask_price,
        (Some((bid_price, bid_size)), _) if bid_size != "0" => bid_price,
        (_, Some((ask_price, _))) => ask_price,
        (Some((bid_price, _)), _) => bid_price,
        _ => return None,
    };

    normalize_price(price)
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

    if let Some(event) = value.get("type") {
        eprintln!("jupiter event: {}", event);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let symbol = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SYMBOL.to_string())
        .to_lowercase();

    let (ws_stream, response) = connect_async(WS_URL).await?;
    eprintln!("connected: {} {}", response.status(), WS_URL);
    eprintln!("subscribing symbol: {}", symbol.to_uppercase());

    let (mut write, mut read) = ws_stream.split();

    let subscribe_message = json!({
        "type": "subscribe",
        "channel": "bestPrice"
    });

    write
        .send(Message::Text(subscribe_message.to_string().into()))
        .await?;

    let mut last_price: Option<Decimal> = None;

    while let Some(frame) = read.next().await {
        match frame? {
            Message::Text(text) => {
                print_price_update(&text, &symbol, &mut last_price);
                log_event(&text);
            }
            Message::Binary(bytes) => {
                let payload = String::from_utf8_lossy(&bytes);
                print_price_update(&payload, &symbol, &mut last_price);
                log_event(&payload);
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