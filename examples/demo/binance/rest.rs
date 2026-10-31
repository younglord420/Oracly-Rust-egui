//! Binance Spot REST kline fetch via `ehttp` (WASM + native).

use super::types::{HISTORY_LIMIT, klines_url, parse_klines_json};
use chrono::{DateTime, Utc};
use egui_charts::data::{FeedError, HistoryPage, Reply};
use egui_charts::model::Bar;

/// Fetch historical klines and deliver them through `reply`.
///
/// `before` maps to Binance `endTime` (open times strictly before that instant).
pub fn fetch_klines(
    symbol: &str,
    interval: &str,
    count: usize,
    before: Option<DateTime<Utc>>,
    reply: Reply<HistoryPage>,
) {
    let limit = count.clamp(1, HISTORY_LIMIT);
    let end_time_ms = before.map(|t| t.timestamp_millis().saturating_sub(1));
    let url = klines_url(symbol, interval, limit, end_time_ms);
    ehttp::fetch(ehttp::Request::get(&url), move |result| {
        let page = match result {
            Ok(response) if response.ok => match response.text() {
                Some(text) => match parse_klines_json(text) {
                    Ok(bars) => Ok(HistoryPage::new(filter_before(bars, before), false)),
                    Err(err) => Err(FeedError::InvalidData(err)),
                },
                None => Err(FeedError::InvalidData(
                    "binance response was not UTF-8 text".into(),
                )),
            },
            Ok(response) => Err(FeedError::Network(format!(
                "binance HTTP {}",
                response.status
            ))),
            Err(err) => Err(FeedError::Network(err)),
        };
        reply.send(page);
    });
}

fn filter_before(mut bars: Vec<Bar>, before: Option<DateTime<Utc>>) -> Vec<Bar> {
    if let Some(before) = before {
        bars.retain(|bar| bar.time < before);
    }
    bars
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn filter_before_drops_boundary_and_newer() {
        let t0 = Utc.timestamp_millis_opt(1_700_000_000_000).unwrap();
        let t1 = Utc.timestamp_millis_opt(1_700_000_060_000).unwrap();
        let bars = vec![
            Bar::new(t0, 1.0, 1.0, 1.0, 1.0, 1.0),
            Bar::new(t1, 2.0, 2.0, 2.0, 2.0, 1.0),
        ];
        let filtered = filter_before(bars, Some(t1));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].time, t0);
    }
}
