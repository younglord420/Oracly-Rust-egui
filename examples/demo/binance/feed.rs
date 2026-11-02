//! [`Datafeed`] that serves Binance Spot for BTCUSDT and delegates everything else.

use super::super::live::LiveFeed;
use super::rest::fetch_klines;
use super::types::{DEFAULT_SYMBOL, interval_for_timeframe, is_binance_symbol};
use super::websocket::WsHandle;
use egui_charts::data::{
    CalendarEvent, CalendarQuery, Datafeed, FeedCapabilities, FeedError, FeedService, HistoryPage,
    HistoryRequest, HotlistEntry, HotlistQuery, MetricPage, MetricRequest, MetricSeries,
    MetricSubscription, NewsQuery, NewsStory, OptionsVolumeQuery, OptionsVolumeSink, Reply,
    ScreenerQuery, ScreenerRow, ScreenerTextAnswer, Subscription, SubscriptionId, SymbolDetails,
};
use egui_charts::model::{Instrument, SymbolType, Timeframe};
use std::sync::Arc;

/// Demo chart feed: Binance live data for BTCUSDT, synthetic [`LiveFeed`] otherwise.
pub struct DemoDatafeed {
    live: Arc<LiveFeed>,
    ws: WsHandle,
}

impl DemoDatafeed {
    pub fn new(live: Arc<LiveFeed>) -> Self {
        Self {
            live,
            ws: WsHandle::new(),
        }
    }
}

impl Datafeed for DemoDatafeed {
    fn capabilities(&self, reply: Reply<FeedCapabilities>) {
        reply.send(Ok(FeedCapabilities::new()
            .with_timeframes([
                Timeframe::Min1,
                Timeframe::Min5,
                Timeframe::Min15,
                Timeframe::Min30,
                Timeframe::Hour1,
                Timeframe::Hour4,
                Timeframe::Day1,
                Timeframe::Week1,
                Timeframe::Month1,
            ])
            .with_exchanges(["Binance".into(), "Synthetic".into()])
            .with_symbol_types([SymbolType::Crypto, SymbolType::Stock, SymbolType::Index])
            .with_services([
                FeedService::News,
                FeedService::Calendar,
                FeedService::Screener,
                FeedService::Hotlist,
                FeedService::SymbolDetails,
            ])));
    }

    fn resolve(&self, symbol: &str, reply: Reply<Instrument>) {
        if is_binance_symbol(symbol) {
            reply.send(Ok(binance_instrument()));
            return;
        }
        self.live.resolve(symbol, reply);
    }

    fn history(&self, request: HistoryRequest, reply: Reply<HistoryPage>) {
        if !is_binance_symbol(&request.symbol) {
            self.live.history(request, reply);
            return;
        }
        let Some(interval) = interval_for_timeframe(request.timeframe) else {
            reply.send(Ok(HistoryPage::new(Vec::new(), true)));
            return;
        };
        fetch_klines(
            DEFAULT_SYMBOL,
            interval,
            request.count.max(1),
            request.before,
            reply,
        );
    }

    fn subscribe(&self, subscription: Subscription) {
        if !is_binance_symbol(&subscription.symbol) {
            self.live.subscribe(subscription);
            return;
        }
        let Some(interval) = interval_for_timeframe(subscription.timeframe) else {
            return;
        };
        self.ws.subscribe(
            subscription.id,
            subscription.sink,
            DEFAULT_SYMBOL.into(),
            interval.into(),
        );
    }

    fn unsubscribe(&self, id: SubscriptionId) {
        self.ws.unsubscribe(id);
        self.live.unsubscribe(id);
    }

    fn options_volume(&self, query: OptionsVolumeQuery, sink: OptionsVolumeSink) {
        self.live.options_volume(query, sink);
    }

    fn metrics(&self, symbol: &str, reply: Reply<Vec<MetricSeries>>) {
        if is_binance_symbol(symbol) {
            reply.send(Ok(Vec::new()));
            return;
        }
        self.live.metrics(symbol, reply);
    }

    fn metric_history(&self, request: MetricRequest, reply: Reply<MetricPage>) {
        if is_binance_symbol(&request.symbol) {
            reply.send(Err(FeedError::NotSupported));
            return;
        }
        self.live.metric_history(request, reply);
    }

    fn subscribe_metric(&self, subscription: MetricSubscription) {
        if is_binance_symbol(&subscription.symbol) {
            return;
        }
        self.live.subscribe_metric(subscription);
    }

    fn price_targets(&self, symbol: &str, reply: Reply<egui_charts::data::PriceTargets>) {
        if is_binance_symbol(symbol) {
            reply.send(Err(FeedError::NotSupported));
            return;
        }
        self.live.price_targets(symbol, reply);
    }

    fn hotlist(&self, query: HotlistQuery, reply: Reply<Vec<HotlistEntry>>) {
        self.live.hotlist(query, reply);
    }

    fn screen(&self, query: ScreenerQuery, reply: Reply<Vec<ScreenerRow>>) {
        self.live.screen(query, reply);
    }

    fn screen_text(&self, query: ScreenerQuery, reply: Reply<ScreenerTextAnswer>) {
        self.live.screen_text(query, reply);
    }

    fn calendar(&self, query: CalendarQuery, reply: Reply<Vec<CalendarEvent>>) {
        self.live.calendar(query, reply);
    }

    fn news(&self, query: NewsQuery, reply: Reply<Vec<NewsStory>>) {
        self.live.news(query, reply);
    }

    fn symbol_details(&self, symbol: &str, reply: Reply<SymbolDetails>) {
        if is_binance_symbol(symbol) {
            reply.send(Ok(SymbolDetails::new(
                vec![
                    ("Exchange".into(), "Binance Spot".into()),
                    ("Pair".into(), DEFAULT_SYMBOL.into()),
                ],
                vec![("Source".into(), "Public Binance market data".into())],
            )
            .with_url("https://www.binance.com/en/trade/BTC_USDT")
            .with_category("Cryptocurrency")));
            return;
        }
        self.live.symbol_details(symbol, reply);
    }

    fn request_depth_snapshot(&self, id: SubscriptionId) {
        self.live.request_depth_snapshot(id);
    }
}

/// Instrument metadata for the Binance BTCUSDT chart.
pub fn binance_instrument() -> Instrument {
    Instrument::new(DEFAULT_SYMBOL)
        .with_description("Bitcoin / Tether")
        .with_exchange("Binance")
        .with_symbol_type(SymbolType::Crypto)
        .with_price_decimals(2)
        .with_tick_size(0.01)
        .with_native_timeframes([Timeframe::Min1])
}

/// Whether the demo should drive the main series from Binance rather than synthetic bars.
pub fn uses_binance(symbol: &str) -> bool {
    is_binance_symbol(symbol)
}
