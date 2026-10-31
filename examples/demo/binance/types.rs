//! Binance REST / WebSocket payload parsing into egui-charts [`Bar`]s.

use chrono::{DateTime, TimeZone, Utc};
use egui_charts::model::Bar;
use serde::Deserialize;
use serde_json::Value;

/// Spot REST base URL (public market data; no API key).
pub const REST_BASE: &str = "https://api.binance.com";

/// Spot combined stream host for kline WebSockets.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const WS_BASE: &str = "wss://stream.binance.com:9443";

/// Default live symbol for this phase.
pub const DEFAULT_SYMBOL: &str = "BTCUSDT";

/// Default live interval for this phase.
#[allow(dead_code)]
pub const DEFAULT_INTERVAL: &str = "1m";

/// How many historical candles to request on the initial load.
pub const HISTORY_LIMIT: usize = 1000;

/// Whether `symbol` should be served by the Binance feed (this phase: BTCUSDT only).
pub fn is_binance_symbol(symbol: &str) -> bool {
    symbol.eq_ignore_ascii_case(DEFAULT_SYMBOL)
}

/// Maps an egui-charts timeframe to a Binance kline interval string.
pub fn interval_for_timeframe(tf: egui_charts::model::Timeframe) -> Option<&'static str> {
    use egui_charts::model::Timeframe::*;
    match tf {
        Min1 => Some("1m"),
        Min5 => Some("5m"),
        Min15 => Some("15m"),
        Min30 => Some("30m"),
        Hour1 => Some("1h"),
        Hour4 => Some("4h"),
        Day1 => Some("1d"),
        Week1 => Some("1w"),
        Month1 => Some("1M"),
        _ => None,
    }
}

/// WebSocket URL for a symbol/interval kline stream.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn kline_stream_url(symbol: &str, interval: &str) -> String {
    format!(
        "{WS_BASE}/ws/{}@kline_{interval}",
        symbol.to_ascii_lowercase()
    )
}

/// REST klines URL. `end_time_ms` is exclusive upper bound on open time when set.
pub fn klines_url(
    symbol: &str,
    interval: &str,
    limit: usize,
    end_time_ms: Option<i64>,
) -> String {
    let mut url = format!(
        "{REST_BASE}/api/v3/klines?symbol={}&interval={interval}&limit={limit}",
        symbol.to_ascii_uppercase()
    );
    if let Some(end) = end_time_ms {
        url.push_str(&format!("&endTime={end}"));
    }
    url
}

/// Result of applying one candle onto an ordered bar series (oldest first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CandleApply {
    /// Replaced the open candle that shares this open time.
    Replaced,
    /// Appended a newer candle.
    Appended,
    /// Ignored a stale / older open time.
    IgnoredStale,
}

/// Replace the active candle, append a newer one, or ignore a stale event.
///
/// Timestamps are compared on bar open time equality (Binance `k.t`).
/// Live updates use [`Sink::push_bar`](egui_charts::data::Sink::push_bar); this helper
/// documents and unit-tests the same replace/append/ignore rules.
#[allow(dead_code)]
pub fn apply_candle_update(bars: &mut Vec<Bar>, candle: Bar) -> CandleApply {
    match bars.last_mut() {
        Some(last) if last.time == candle.time => {
            *last = candle;
            CandleApply::Replaced
        }
        Some(last) if candle.time > last.time => {
            bars.push(candle);
            CandleApply::Appended
        }
        Some(_) => CandleApply::IgnoredStale,
        None => {
            bars.push(candle);
            CandleApply::Appended
        }
    }
}

/// Parse a Binance REST klines JSON array into bars (oldest first).
///
/// Each row is:
/// `[ open_time, open, high, low, close, volume, close_time, ... ]`
pub fn parse_klines_json(body: &str) -> Result<Vec<Bar>, String> {
    let rows: Vec<Value> = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let mut bars = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(arr) = row.as_array() else {
            continue;
        };
        match bar_from_kline_row(arr) {
            Ok(bar) => bars.push(bar),
            Err(err) => log::warn!("binance: skip kline row: {err}"),
        }
    }
    Ok(bars)
}

fn bar_from_kline_row(row: &[Value]) -> Result<Bar, String> {
    if row.len() < 6 {
        return Err("kline row too short".into());
    }
    let open_ms = json_i64(&row[0]).ok_or("missing open time")?;
    let open = json_f64(&row[1]).ok_or("bad open")?;
    let high = json_f64(&row[2]).ok_or("bad high")?;
    let low = json_f64(&row[3]).ok_or("bad low")?;
    let close = json_f64(&row[4]).ok_or("bad close")?;
    let volume = json_f64(&row[5]).ok_or("bad volume")?;
    let time = millis_to_utc(open_ms).ok_or("invalid open time")?;
    Ok(Bar::new(time, open, high, low, close, volume))
}

/// WebSocket kline event (`e` == `"kline"`).
#[derive(Debug, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub struct KlineEvent {
    #[serde(rename = "E")]
    pub event_time: Option<i64>,
    #[serde(rename = "k")]
    pub kline: KlinePayload,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub struct KlinePayload {
    /// Candle open time (ms).
    #[serde(rename = "t")]
    pub open_time: i64,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "v")]
    pub volume: String,
    /// Whether this kline is closed.
    #[serde(rename = "x")]
    pub is_closed: bool,
}

/// Parse a Binance kline WebSocket text frame into a bar + closed flag.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn parse_kline_message(text: &str) -> Result<(Bar, bool, Option<DateTime<Utc>>), String> {
    let event: KlineEvent = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let k = &event.kline;
    let time = millis_to_utc(k.open_time).ok_or("invalid kline open time")?;
    let open: f64 = k.open.parse().map_err(|_| "bad open".to_string())?;
    let high: f64 = k.high.parse().map_err(|_| "bad high".to_string())?;
    let low: f64 = k.low.parse().map_err(|_| "bad low".to_string())?;
    let close: f64 = k.close.parse().map_err(|_| "bad close".to_string())?;
    let volume: f64 = k.volume.parse().map_err(|_| "bad volume".to_string())?;
    if ![open, high, low, close, volume]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("non-finite ohlcv".into());
    }
    let event_time = event.event_time.and_then(millis_to_utc);
    Ok((
        Bar::new(time, open, high, low, close, volume),
        k.is_closed,
        event_time,
    ))
}

fn millis_to_utc(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().map(|v| v as i64))
        .or_else(|| value.as_str()?.parse().ok())
}

fn json_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.parse().ok())
        .filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rest_klines() {
        let body = r#"[
          [1700000000000,"100.0","110.0","90.0","105.0","12.5",1700000059999,"0",0,"0","0","0"],
          [1700000060000,"105.0","120.0","104.0","118.0","8.0",1700000119999,"0",0,"0","0","0"]
        ]"#;
        let bars = parse_klines_json(body).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].open, 100.0);
        assert_eq!(bars[0].high, 110.0);
        assert_eq!(bars[0].low, 90.0);
        assert_eq!(bars[0].close, 105.0);
        assert_eq!(bars[0].volume, 12.5);
        assert_eq!(bars[1].close, 118.0);
        assert_eq!(
            bars[0].time,
            Utc.timestamp_millis_opt(1_700_000_000_000).unwrap()
        );
    }

    #[test]
    fn skips_malformed_rest_rows() {
        let body = r#"[[1,"bad"],[1700000000000,"1","2","0.5","1.5","3",0,"0",0,"0","0","0"]]"#;
        let bars = parse_klines_json(body).unwrap();
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].close, 1.5);
    }

    #[test]
    fn parses_ws_kline_and_closed_flag() {
        let text = r#"{
          "e":"kline","E":1700000001000,"s":"BTCUSDT",
          "k":{"t":1700000000000,"T":1700000059999,"s":"BTCUSDT","i":"1m",
               "o":"100.0","c":"101.0","h":"102.0","l":"99.0","v":"123.45","x":false}
        }"#;
        let (bar, closed, event_time) = parse_kline_message(text).unwrap();
        assert!(!closed);
        assert_eq!(bar.open, 100.0);
        assert_eq!(bar.close, 101.0);
        assert_eq!(bar.high, 102.0);
        assert_eq!(bar.low, 99.0);
        assert_eq!(bar.volume, 123.45);
        assert_eq!(
            event_time,
            Some(Utc.timestamp_millis_opt(1_700_000_001_000).unwrap())
        );
    }

    #[test]
    fn rejects_bad_ws_payload() {
        assert!(parse_kline_message(r#"{"e":"kline","k":{"t":1}}"#).is_err());
        assert!(parse_kline_message("not-json").is_err());
    }

    #[test]
    fn replace_append_ignore_semantics() {
        let t0 = Utc.timestamp_millis_opt(1_700_000_000_000).unwrap();
        let t1 = Utc.timestamp_millis_opt(1_700_000_060_000).unwrap();
        let mut bars = vec![Bar::new(t0, 1.0, 2.0, 0.5, 1.5, 10.0)];

        assert_eq!(
            apply_candle_update(&mut bars, Bar::new(t0, 1.0, 3.0, 0.4, 2.0, 11.0)),
            CandleApply::Replaced
        );
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].high, 3.0);
        assert_eq!(bars[0].close, 2.0);

        assert_eq!(
            apply_candle_update(&mut bars, Bar::new(t1, 2.0, 2.5, 1.8, 2.2, 1.0)),
            CandleApply::Appended
        );
        assert_eq!(bars.len(), 2);

        let older = Utc.timestamp_millis_opt(1_699_999_940_000).unwrap();
        assert_eq!(
            apply_candle_update(&mut bars, Bar::new(older, 9.0, 9.0, 9.0, 9.0, 1.0)),
            CandleApply::IgnoredStale
        );
        assert_eq!(bars.len(), 2);
    }

    #[test]
    fn duplicate_open_time_does_not_append() {
        let t0 = Utc.timestamp_millis_opt(1_700_000_000_000).unwrap();
        let mut bars = Vec::new();
        for close in [100.0, 101.0, 102.0] {
            assert_ne!(
                apply_candle_update(&mut bars, Bar::new(t0, 100.0, close, 99.0, close, 1.0)),
                CandleApply::IgnoredStale
            );
        }
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].close, 102.0);
    }
}
