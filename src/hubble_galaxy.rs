use reqwest::Client;
use serde::{Serialize};
use serde_json::Value;
use std::{fs::OpenOptions, io::Write};
use tokio::time::{sleep, Duration};

#[derive(Serialize)]
struct Record {
    source: String,
    key: String,
    value: Value,
    timestamp: u64,
}

fn log_and_write_record(
    file: &mut std::fs::File,
    record: &Record,
) -> Result<(), Box<dyn std::error::Error>> {
    let pretty_json = serde_json::to_string_pretty(record)?;
    println!("{pretty_json}");

    writeln!(file, "{}", serde_json::to_string(record)?)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let output_path = "output/hubble_galaxy_output.txt";

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)?;

    println!("🚀 Multi-source passive intel collector started");

    // =========================
    // 1. CT LOGS (crt.sh)
    // =========================
    println!("📡 Fetching CT logs...");
    let ct_url = "https://crt.sh/?q=%25google.com&output=json";

    if let Ok(res) = client.get(ct_url).send().await {
        if let Ok(json) = res.json::<Value>().await {
            if let Some(arr) = json.as_array() {
                for item in arr.iter().take(20) {
                    let record = Record {
                        source: "crt.sh".to_string(),
                        key: item["name_value"].as_str().unwrap_or("").to_string(),
                        value: Value::String(
                            item["issuer_name"].as_str().unwrap_or("").to_string(),
                        ),
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };

                    log_and_write_record(&mut file, &record)?;
                }
            }
        }
    }

    sleep(Duration::from_millis(500)).await;

    // =========================
    // 2. RIPE DB
    // =========================
    println!("📡 Fetching RIPE data...");
    let ripe_url = "https://rest.db.ripe.net/search.json?query-string=8.8.8.8";

    if let Ok(res) = client.get(ripe_url).send().await {
        if let Ok(json) = res.json::<Value>().await {
            let record = Record {
                source: "ripe".to_string(),
                key: "8.8.8.8".to_string(),
                value: json,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            log_and_write_record(&mut file, &record)?;
        }
    }

    sleep(Duration::from_millis(500)).await;

    // =========================
    // 3. DNS (Google DoH)
    // =========================
    println!("📡 Fetching DNS data...");
    let dns_url = "https://dns.google/resolve?name=example.com&type=A";

    if let Ok(res) = client.get(dns_url).send().await {
        if let Ok(json) = res.json::<Value>().await {
            let record = Record {
                source: "dns_google".to_string(),
                key: "example.com".to_string(),
                value: json,
                timestamp: chrono::Utc::now().timestamp() as u64,
            };

            log_and_write_record(&mut file, &record)?;
        }
    }

    println!("✅ Done. Saved to {output_path}");

    Ok(())
}