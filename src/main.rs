use chrono::Local;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::env;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const WS_URL: &str = "wss://stream.binance.com:9443/ws";
const DEFAULT_SYMBOL: &str = "solusdt";

fn split_symbol(symbol: &str) -> (&str, &str) {
    for quote in ["usdt", "fdusd", "usdc", "btc", "eth", "bnb", "try", "eur"] {
        if let Some(base) = symbol.strip_suffix(quote) {
            return (base, quote);
        }
    }

    (symbol, "")
}

fn print_price_update(payload: &str, symbol: &str, last_price: &mut Option<String>) {
    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return;
    };

    let Some(price) = value.get("c").and_then(Value::as_str) else {
        return;
    };

    if last_price.as_deref() == Some(price) {
        return;
    }

    *last_price = Some(price.to_string());

    let (base, quote) = split_symbol(symbol);
    let base = base.to_uppercase();
    let quote = quote.to_uppercase();
    let formatted = if quote.is_empty() {
        price.to_string()
    } else {
        format!("{price} {quote}")
    };

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S.%3f ms");
    println!("[{}] {}", timestamp, json!({ base: formatted }));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let symbol = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SYMBOL.to_string())
        .to_lowercase();

    let stream = format!("{symbol}@ticker");

    let (ws_stream, response) = connect_async(WS_URL).await?;
    eprintln!("connected: {} {}", response.status(), WS_URL);
    eprintln!("subscribing symbol: {}", symbol.to_uppercase());

    let (mut write, mut read) = ws_stream.split();

    let subscribe_message = json!({
        "method": "SUBSCRIBE",
        "params": [stream],
        "id": 1
    });

    write
        .send(Message::Text(subscribe_message.to_string().into()))
        .await?;

    let mut last_price: Option<String> = None;

    while let Some(frame) = read.next().await {
        match frame? {
            Message::Text(text) => print_price_update(&text, &symbol, &mut last_price),
            Message::Binary(bytes) => {
                let payload = String::from_utf8_lossy(&bytes);
                print_price_update(&payload, &symbol, &mut last_price);
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