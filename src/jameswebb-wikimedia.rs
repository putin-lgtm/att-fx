use chrono::Local;
use eventsource_client::{Client as _, ClientBuilder, ReconnectOptions, SSE};
use futures_util::TryStreamExt;
use launchdarkly_sdk_transport::HyperTransport;
use serde_json::Value;
use std::env;
use std::time::{Duration, Instant};

const SSE_URL: &str = "https://stream.wikimedia.org/v2/stream/recentchange";

fn normalized_keywords() -> Vec<String> {
    env::args().skip(1).collect()
}

fn text_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_lowercase())
        .collect()
}

fn contains_keyword(payload: &str, keywords: &[String]) -> bool {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let searchable_text = [
            value.get("title").and_then(Value::as_str).unwrap_or(""),
            value.get("comment").and_then(Value::as_str).unwrap_or(""),
            value.get("parsedcomment").and_then(Value::as_str).unwrap_or(""),
            value.get("wiki").and_then(Value::as_str).unwrap_or(""),
        ]
        .join(" ");

        let tokens = text_tokens(&searchable_text);
        return keywords
            .iter()
            .map(|keyword| keyword.to_lowercase())
            .any(|keyword| tokens.iter().any(|token| token == &keyword));
    }

    let tokens = text_tokens(payload);
    keywords
        .iter()
        .map(|keyword| keyword.to_lowercase())
        .any(|keyword| tokens.iter().any(|token| token == &keyword))
}

fn print_match(payload: &str, keywords: &[String]) -> bool {
    if !keywords.is_empty() && !contains_keyword(payload, keywords) {
        return false;
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S.%3f ms");

    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        if let Ok(pretty_json) = serde_json::to_string_pretty(&value) {
            println!("[{}]\n{}", timestamp, pretty_json);
            return true;
        }
    }

    println!("[{}] {}", timestamp, payload);
    true
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let keywords = normalized_keywords();
    eprintln!("connected: SSE {}", SSE_URL);
    if keywords.is_empty() {
        eprintln!("filtering keywords: <disabled>");
    } else {
        eprintln!("filtering keywords: {}", keywords.join(", "));
    }

    let transport = HyperTransport::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .build_https()?;

    let client = ClientBuilder::for_url(SSE_URL)?
        .header("Accept", "text/event-stream")?
        .header("Cache-Control", "no-cache")?
        .header("User-Agent", "att-fx/0.1 SSE radar")?
        .reconnect(
            ReconnectOptions::reconnect(true)
                .retry_initial(false)
                .delay(Duration::from_secs(1))
                .backoff_factor(2)
                .delay_max(Duration::from_secs(30))
                .build(),
        )
        .build_with_transport(transport);

    let mut stream = client.stream();
    let mut scanned_events: u64 = 0;
    let mut matched_events: u64 = 0;
    let mut last_status_at = Instant::now();

    while let Some(event) = stream.try_next().await? {
        match event {
            SSE::Connected(connection) => {
                eprintln!("http status: {}", connection.response().status());
            }
            SSE::Event(event) => {
                scanned_events += 1;
                if print_match(&event.data, &keywords) {
                    matched_events += 1;
                }

                if last_status_at.elapsed() >= Duration::from_secs(5) {
                    eprintln!(
                        "wikimedia stream alive: scanned={} matched={} keywords={}",
                        scanned_events,
                        matched_events,
                        if keywords.is_empty() {
                            "<disabled>".to_string()
                        } else {
                            keywords.join(", ")
                        }
                    );
                    last_status_at = Instant::now();
                }
            }
            SSE::Comment(_) => {}
        }
    }

    Ok(())
}