//! Binance Spot market data for the WASM demo (BTCUSDT klines).
//!
//! History comes from the public REST API; live updates from the public kline
//! WebSocket. Parsing and candle-replacement helpers are covered by unit tests
//! that do not contact the network.

mod feed;
mod rest;
mod types;
mod websocket;

pub use feed::{DemoDatafeed, binance_instrument, uses_binance};
pub use types::DEFAULT_SYMBOL;
