use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fs::OpenOptions, io::Write, time::Duration};
use tokio::time::sleep;

const API_KEY: &str = "51e69000-a903-027E5-E3C0-e35501E2fbe";
const BASE_URL: &str = "https://api.zoomeye.org/host/search";

#[derive(Serialize, Deserialize, Debug)]
struct NormalizedRecord {
    ip: String,
    port: u16,
    service: String,
    banner: String,
    source: String,
    timestamp: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let query = "port:6379";

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("data.txt")?;

    println!("🚀 Passive Intel Collector started...");

    for page in 1..=3 {
        println!("📡 Fetching page {}", page);

        let mut retry = 0;

        loop {
            let res = client
                .get(BASE_URL)
                .header("API-KEY", API_KEY)
                .query(&[
                    ("query", query),
                    ("page", &page.to_string()),
                ])
                .send()
                .await;

            match res {
                Ok(resp) => {
                    let json: Value = resp.json().await?;

                    if let Some(matches) = json["matches"].as_array() {
                        for m in matches {
                            if let Some(ip) = m["ip"].as_str() {
                                let port = m["portinfo"]["port"]
                                    .as_u64()
                                    .unwrap_or(0) as u16;

                                let service = m["portinfo"]["service"]
                                    .as_str()
                                    .unwrap_or("unknown")
                                    .to_string();

                                let banner = m["portinfo"]["banner"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string();

                                let record = NormalizedRecord {
                                    ip: ip.to_string(),
                                    port,
                                    service,
                                    banner,
                                    source: "zoomeye".to_string(),
                                    timestamp: chrono::Utc::now().timestamp() as u64,
                                };

                                let line = serde_json::to_string(&record)?;
                                writeln!(file, "{}", line)?;
                            }
                        }
                    }

                    break;
                }

                Err(e) => {
                    retry += 1;
                    if retry > 3 {
                        println!("❌ Failed page {} after retries: {}", page, e);
                        break;
                    }

                    println!("⚠️ Retry {} for page {}", retry, page);
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        // rate limit (ZoomEye free plan)
        sleep(Duration::from_millis(800)).await;
    }

    println!("✅ Done. Data saved to data.txt");

    Ok(())
}