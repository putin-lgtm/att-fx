use rust_decimal::Decimal;
use std::str::FromStr;

const KNOWN_QUOTES: [&str; 8] = ["usdt", "fdusd", "usdc", "btc", "eth", "bnb", "try", "eur"];
const PRICE_SCALE: u32 = 8;

pub fn split_symbol(symbol: &str) -> (&str, &str) {
    for quote in KNOWN_QUOTES {
        if let Some(base) = symbol.strip_suffix(quote) {
            return (base, quote);
        }
    }

    (symbol, "")
}

pub fn to_okx_inst_id(symbol: &str) -> String {
    for quote in KNOWN_QUOTES {
        if let Some(base) = symbol.strip_suffix(quote) {
            return format!("{}-{}", base.to_uppercase(), quote.to_uppercase());
        }
    }

    symbol.to_uppercase()
}

pub fn normalize_price(price: &str) -> Option<Decimal> {
    let mut normalized = Decimal::from_str(price).ok()?.round_dp(PRICE_SCALE);
    normalized.rescale(PRICE_SCALE);
    Some(normalized)
}

pub fn format_price(price: Decimal) -> String {
    let mut normalized = price.round_dp(PRICE_SCALE);
    normalized.rescale(PRICE_SCALE);
    normalized.to_string()
}