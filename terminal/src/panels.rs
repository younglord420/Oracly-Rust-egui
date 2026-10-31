//! Panels: the widget bar icon strip and the side panels it toggles.
//!
//! Always available (chart feed):
//! - Highlights, News, Calendar, Screener, Hotlists, Market heatmap, Products
//!
//! Enabled by the host's services:
//! - `with_quotes`        → Watchlist (+ symbol details)
//! - `with_alerts`        → Alerts rail entry (floating panel)
//! - `with_notifications` → Notifications panel
//! - `with_chats`         → Chats panel
//!
//! The ObjectTree panel is owned by `drawings.rs` and uses the same `self.panel` field,
//! so this module skips `SidePanel::ObjectTree` and `SidePanel::DataWindow` when toggling.

use super::core::{Direction, SidePanel};
use super::layout::{Region, panel_frame};
use super::services::{ExportContents, ExportFile};
use super::{Action, Terminal};
use egui_charts::data::{ScreenerColumn, ScreenerColumnSet};
use egui_charts::icons::icons;
use egui_charts::localization as l10n;
use egui_charts::model::{Instrument, Watchlist};
use egui_charts::services::UserSettings;
use egui_charts::ui::widget_bar::{
    CalendarAction, ChatsPanel, HotlistsAction, HotlistsPanel, MarketDetail, MarketDetailRequest,
    MarketHeatmapPanel, NewsAction, NewsPanel, NotificationsAction, NotificationsPanel, Product,
    ProductsAction, ProductsPanel, RightBarIcon, ScreenerAction, ScreenerPanel, SymbolDetailsPanel,
    WatchlistAction, WatchlistPanel, WidgetBar, WidgetBarAction,
};

pub(super) struct State {
    pub(super) widget_bar: WidgetBar,
    watchlist: WatchlistPanel,
    symbol_details: SymbolDetailsPanel,
    highlights: NewsPanel,
    news: NewsPanel,
    calendar: egui_charts::ui::widget_bar::CalendarPanel,
    screener: ScreenerPanel,
    hotlists: HotlistsPanel,
    market_heatmap: MarketHeatmapPanel,
    notifications: NotificationsPanel,
    chats: ChatsPanel,
    products: ProductsPanel,
}

impl Default for State {
    fn default() -> Self {
        Self {
            widget_bar: WidgetBar::default(),
            watchlist: WatchlistPanel::new(vec![Watchlist::new("Watchlist")]),
            symbol_details: SymbolDetailsPanel::new(),
            highlights: NewsPanel::highlights(),
            news: NewsPanel::new(),
            calendar: egui_charts::ui::widget_bar::CalendarPanel::new(),
            screener: ScreenerPanel::new(default_screener_sets()),
            hotlists: HotlistsPanel::new(default_hotlists()),
            market_heatmap: MarketHeatmapPanel::default(),
            notifications: NotificationsPanel::new(),
            chats: ChatsPanel::new(),
            products: ProductsPanel::new(default_products()),
        }
    }
}

fn default_screener_sets() -> Vec<ScreenerColumnSet> {
    let column = ScreenerColumn::new;
    vec![ScreenerColumnSet::new(
        "Overview",
        vec![
            column("price", "Price"),
            column("change_percent", "Chg %"),
            column("volume", "Volume"),
        ],
    )]
}

fn default_hotlists() -> Vec<String> {
    ["Top gainers", "Top losers", "Most volatile"]
        .map(String::from)
        .to_vec()
}

fn default_products() -> Vec<Product> {
    vec![
        Product {
            id: "screener".into(),
            icon: icons::WIDGET_BAR_SCREENER,
            title: "Screener".into(),
            description: "Filter instruments by columns and rules".into(),
        },
        Product {
            id: "calendar".into(),
            icon: icons::WIDGET_BAR_CALENDAR,
            title: "Calendars".into(),
            description: "Economic and corporate events".into(),
        },
        Product {
            id: "news".into(),
            icon: icons::WIDGET_BAR_NEWS,
            title: "News".into(),
            description: "Headlines for the chart's symbol".into(),
        },
        Product {
            id: "shortcuts".into(),
            icon: icons::WIDGET_BAR_HELP,
            title: "Keyboard shortcuts".into(),
            description: "Every key the chart understands".into(),
        },
    ]
}

impl Terminal {
    /// Replaces the screener's column sets. Call before the first [`Self::show`], or after
    /// [`Self::with_storage`] when saved screens should keep using the new sets.
    pub fn with_screener_sets(mut self, sets: Vec<ScreenerColumnSet>) -> Self {
        self.panels.screener = ScreenerPanel::new(sets);
        self
    }

    /// Names the hotlists offered in the rail, such as `"Top gainers"`.
    pub fn with_hotlists(mut self, lists: Vec<String>) -> Self {
        self.panels.hotlists = HotlistsPanel::new(lists);
        self
    }

    /// The market heatmap panel, for the host to supply universes and metrics.
    pub fn market_heatmap_mut(&mut self) -> &mut MarketHeatmapPanel {
        &mut self.panels.market_heatmap
    }

    pub(super) fn panels_init(&mut self) {
        let feed = self.feed.clone();
        self.panels.highlights.set_feed(feed.clone());
        self.panels.news.set_feed(feed.clone());
        self.panels.calendar.set_feed(feed.clone());
        self.panels.screener.set_feed(feed.clone());
        self.panels.hotlists.set_feed(feed.clone());
        self.panels.symbol_details.set_feed(feed);

        if let Some(storage) = &self.services.storage {
            self.panels.screener.set_storage(storage.clone());
            self.panels.symbol_details.set_storage(storage.clone());
        }
        if let Some(quotes) = self.services.quotes.clone() {
            self.panels.watchlist.set_service(quotes);
        }
        if let Some(notifications) = self.services.notifications.clone() {
            self.panels.notifications.set_service(notifications);
        }
        if let Some(chats) = self.services.chats.clone() {
            self.panels.chats.set_service(chats);
        }

        self.panels.widget_bar.icons = self.panel_icons();
    }

    fn panel_icons(&self) -> Vec<RightBarIcon> {
        let mut icons = Vec::new();
        if self.services.quotes.is_some() {
            icons.push(RightBarIcon::Watchlist);
        }
        if self.services.alerts.is_some() {
            icons.push(RightBarIcon::Alerts);
        }
        icons.push(RightBarIcon::ObjectTree);
        icons.extend([
            RightBarIcon::Highlights,
            RightBarIcon::Screener,
            RightBarIcon::Hotlists,
            RightBarIcon::MarketHeatmap,
            RightBarIcon::Calendar,
            RightBarIcon::News,
        ]);
        if self.services.notifications.is_some() {
            icons.push(RightBarIcon::Notifications);
        }
        if self.services.chats.is_some() {
            icons.push(RightBarIcon::Chats);
        }
        icons.push(RightBarIcon::Products);
        icons.push(RightBarIcon::Help);
        icons
    }

    pub(super) fn panels_poll(&mut self, ctx: &egui::Context) {
        let symbol = self.chart().symbol().to_owned();
        let instrument = self
            .chart()
            .instrument()
            .cloned()
            .unwrap_or_else(|| Instrument::new(&symbol));

        self.panels.watchlist.set_selected(&symbol);
        self.panels.watchlist.poll(ctx);
        self.panels.symbol_details.set_instrument(&instrument);
        if let Some(quote) = self.panels.watchlist.quote(&symbol).cloned() {
            self.panels.symbol_details.set_quote(quote);
        }
        self.panels.symbol_details.poll(ctx);
        for request in self.panels.symbol_details.take_market_requests() {
            match request {
                MarketDetailRequest::PeriodReturns(symbol) => {
                    self.panels
                        .symbol_details
                        .set_period_returns(&symbol, MarketDetail::Unavailable);
                }
                MarketDetailRequest::Seasonality(symbol) => {
                    self.panels
                        .symbol_details
                        .set_seasonality(&symbol, MarketDetail::Unavailable);
                }
                _ => {}
            }
        }
        // Financial statements stay with the host; clear the request so the panel does not retry.
        let _ = self.panels.symbol_details.take_financials_request();

        self.panels.highlights.set_symbol(&symbol);
        self.panels.news.set_symbol(&symbol);
        self.panels.highlights.poll(ctx);
        self.panels.news.poll(ctx);
        self.panels.calendar.poll(ctx);
        self.panels.screener.poll(ctx);
        self.panels.hotlists.poll(ctx);
        self.panels.notifications.poll(ctx);
        self.panels.chats.poll(ctx);

        let unread = self.panels.notifications.unread();
        self.panels
            .widget_bar
            .set_notification_cnt(RightBarIcon::Notifications, unread as u32);
    }

    pub(super) fn panels_show(&mut self, ui: &mut egui::Ui, region: Region) {
        if region != Region::Right {
            return;
        }
        let bg = ui.style().visuals.panel_fill;

        self.panels.widget_bar.active_icon = self.panel.and_then(panel_icon);
        let action = egui::Panel::right("panels_widget_bar")
            .exact_size(self.panels.widget_bar.config.width)
            .show_separator_line(false)
            .frame(panel_frame(bg))
            .show(ui, |ui| self.panels.widget_bar.show(ui))
            .inner;

        if !matches!(action, WidgetBarAction::None) {
            self.emit(Action::WidgetBar(action));
        }

        let open = match self.panel {
            Some(SidePanel::Watchlist) if self.services.quotes.is_some() => {
                Some(SidePanel::Watchlist)
            }
            Some(SidePanel::Notifications) if self.services.notifications.is_some() => {
                Some(SidePanel::Notifications)
            }
            Some(SidePanel::Chats) if self.services.chats.is_some() => Some(SidePanel::Chats),
            Some(
                panel @ (SidePanel::Highlights
                | SidePanel::Screener
                | SidePanel::Hotlists
                | SidePanel::MarketHeatmap
                | SidePanel::Calendar
                | SidePanel::News
                | SidePanel::Products),
            ) => Some(panel),
            _ => None,
        };
        let Some(open) = open else { return };

        egui::Panel::right("panels_side_panel")
            .resizable(true)
            .default_size(280.0)
            .min_size(200.0)
            .show_separator_line(false)
            .frame(panel_frame(bg))
            .show(ui, |ui| match open {
                SidePanel::Watchlist => {
                    if ui.available_height() >= 400.0 {
                        egui::Panel::bottom("panels_symbol_details")
                            .resizable(true)
                            .default_size(ui.available_height() / 2.0)
                            .min_size(120.0)
                            .frame(egui::Frame::NONE)
                            .show(ui, |ui| {
                                self.panels.symbol_details.show(ui);
                            });
                    }
                    let action = self.panels.watchlist.show(ui);
                    handle_watchlist_action(action, &mut self.emitted);
                }
                SidePanel::Highlights => {
                    let action = self.panels.highlights.show(ui);
                    if action != NewsAction::None {
                        self.emitted.push(Action::News(action));
                    }
                }
                SidePanel::News => {
                    let action = self.panels.news.show(ui);
                    if action != NewsAction::None {
                        self.emitted.push(Action::News(action));
                    }
                }
                SidePanel::Calendar => {
                    let action = self.panels.calendar.show(ui);
                    if action != CalendarAction::None {
                        self.emitted.push(Action::Calendar(action));
                    }
                }
                SidePanel::Screener => {
                    let action = self.panels.screener.show(ui);
                    if action != ScreenerAction::None {
                        self.emitted.push(Action::Screener(action));
                    }
                }
                SidePanel::Hotlists => {
                    let action = self.panels.hotlists.show(ui);
                    if action != HotlistsAction::None {
                        self.emitted.push(Action::Hotlists(action));
                    }
                }
                SidePanel::MarketHeatmap => {
                    if let Some(symbol) = self.panels.market_heatmap.show(ui) {
                        self.emitted
                            .push(Action::Watchlist(WatchlistAction::SelectSymbol(symbol)));
                    }
                }
                SidePanel::Notifications => {
                    let action = self.panels.notifications.show(ui);
                    if action != NotificationsAction::None {
                        self.emitted.push(Action::Notifications(action));
                    }
                }
                SidePanel::Chats => {
                    self.panels.chats.show(ui);
                }
                SidePanel::Products => {
                    let action = self.panels.products.show(ui);
                    if action != ProductsAction::None {
                        self.emitted.push(Action::Products(action));
                    }
                }
                _ => {}
            });
    }

    pub(super) fn panels_perform(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::WidgetBar(WidgetBarAction::Clicked(RightBarIcon::Help)) => Some(action),
            Action::WidgetBar(WidgetBarAction::Clicked(RightBarIcon::ObjectTree)) => Some(action),
            Action::WidgetBar(WidgetBarAction::Clicked(RightBarIcon::Alerts)) => {
                if self.services.alerts.is_some() {
                    self.alerts.panel_open = !self.alerts.panel_open;
                    None
                } else {
                    Some(action)
                }
            }
            Action::WidgetBar(WidgetBarAction::Clicked(icon)) => {
                self.toggle_panel_by_icon(icon);
                None
            }
            Action::WidgetBar(WidgetBarAction::WidgetClicked(_)) => Some(action),
            Action::WidgetBar(WidgetBarAction::None) => None,
            Action::Watchlist(WatchlistAction::SelectSymbol(symbol)) => {
                self.switch_symbol(&symbol);
                None
            }
            Action::Watchlist(_) => None,
            Action::News(NewsAction::ShowOnChart(time)) => {
                self.chart_mut().scroll_to_time(time);
                None
            }
            Action::News(NewsAction::OpenUrl(url)) => {
                self.ctx().open_url(egui::OpenUrl::new_tab(url));
                None
            }
            Action::News(NewsAction::None) => None,
            Action::News(other) => Some(Action::News(other)),
            Action::Calendar(CalendarAction::SelectSymbol(symbol)) => {
                self.switch_symbol(&symbol);
                None
            }
            Action::Calendar(CalendarAction::None) => None,
            Action::Calendar(other) => Some(Action::Calendar(other)),
            Action::Screener(ScreenerAction::SelectSymbol(symbol)) => {
                self.switch_symbol(&symbol);
                None
            }
            Action::Screener(ScreenerAction::SendToWatchlist(symbols)) => {
                self.panels
                    .watchlist
                    .import("Screener", &symbols.join(","));
                if self.services.quotes.is_some() {
                    self.panel = Some(SidePanel::Watchlist);
                }
                None
            }
            Action::Screener(ScreenerAction::ExportCsv(text)) => {
                if self.services.save.is_some() {
                    if let Some(save) = &mut self.services.save {
                        save(ExportFile {
                            name: "screener.csv".into(),
                            path: None,
                            contents: ExportContents::Text(text),
                        });
                    }
                    self.toast(l10n::text(&self.ctx(), "Export successful: screener.csv"));
                    None
                } else {
                    Some(Action::Screener(ScreenerAction::ExportCsv(text)))
                }
            }
            Action::Screener(ScreenerAction::None) => None,
            Action::Screener(other) => Some(Action::Screener(other)),
            Action::Hotlists(HotlistsAction::SelectSymbol(symbol)) => {
                self.switch_symbol(&symbol);
                None
            }
            Action::Hotlists(HotlistsAction::None) => None,
            Action::Hotlists(other) => Some(Action::Hotlists(other)),
            Action::Notifications(NotificationsAction::SelectSymbol(symbol)) => {
                self.switch_symbol(&symbol);
                None
            }
            Action::Notifications(NotificationsAction::None) => None,
            Action::Notifications(other) => Some(Action::Notifications(other)),
            Action::Products(ProductsAction::Open(id)) => match id.as_str() {
                "screener" => {
                    self.panel = Some(SidePanel::Screener);
                    None
                }
                "calendar" => {
                    self.panel = Some(SidePanel::Calendar);
                    None
                }
                "news" => {
                    self.panel = Some(SidePanel::News);
                    None
                }
                "shortcuts" => Some(Action::WidgetBar(WidgetBarAction::Clicked(
                    RightBarIcon::Help,
                ))),
                _ => Some(Action::Products(ProductsAction::Open(id))),
            },
            Action::Products(ProductsAction::None) => None,
            Action::Products(other) => Some(Action::Products(other)),
            other => Some(other),
        }
    }

    pub(super) fn panels_settings(&mut self, settings: &mut UserSettings, direction: Direction) {
        match direction {
            Direction::Load => {
                if !settings.watchlists.is_empty() {
                    self.panels.watchlist = WatchlistPanel::new(settings.watchlists.clone());
                    if let Some(quotes) = self.services.quotes.clone() {
                        self.panels.watchlist.set_service(quotes);
                    }
                }
                self.panels.market_heatmap.selection = settings.market_heatmap.clone();
            }
            Direction::Save => {
                settings.watchlists = self.panels.watchlist.lists().to_vec();
                settings.market_heatmap = self.panels.market_heatmap.selection.clone();
            }
        }
    }

    pub(super) fn panels_modal(&self) -> bool {
        false
    }

    /// Toggles the side panel that corresponds to `icon`, closing it when already open.
    fn toggle_panel_by_icon(&mut self, icon: RightBarIcon) {
        let target = match icon {
            RightBarIcon::Watchlist => SidePanel::Watchlist,
            RightBarIcon::Highlights => SidePanel::Highlights,
            RightBarIcon::Screener => SidePanel::Screener,
            RightBarIcon::Hotlists => SidePanel::Hotlists,
            RightBarIcon::MarketHeatmap => SidePanel::MarketHeatmap,
            RightBarIcon::Calendar => SidePanel::Calendar,
            RightBarIcon::News => SidePanel::News,
            RightBarIcon::Notifications => SidePanel::Notifications,
            RightBarIcon::Chats => SidePanel::Chats,
            RightBarIcon::Products => SidePanel::Products,
            RightBarIcon::ObjectTree => SidePanel::ObjectTree,
            RightBarIcon::Alerts | RightBarIcon::Help => return,
            _ => return,
        };
        if matches!(icon, RightBarIcon::Screener) {
            self.panels.screener.refresh();
        }
        if matches!(icon, RightBarIcon::Hotlists) {
            self.panels.hotlists.refresh();
        }
        if self.panel == Some(target) {
            self.panel = None;
        } else {
            self.panel = Some(target);
        }
    }
}

/// The `RightBarIcon` that corresponds to a `SidePanel`, for widget bar highlighting.
fn panel_icon(panel: SidePanel) -> Option<RightBarIcon> {
    match panel {
        SidePanel::Watchlist => Some(RightBarIcon::Watchlist),
        SidePanel::Highlights => Some(RightBarIcon::Highlights),
        SidePanel::Screener => Some(RightBarIcon::Screener),
        SidePanel::Hotlists => Some(RightBarIcon::Hotlists),
        SidePanel::MarketHeatmap => Some(RightBarIcon::MarketHeatmap),
        SidePanel::Calendar => Some(RightBarIcon::Calendar),
        SidePanel::News => Some(RightBarIcon::News),
        SidePanel::Notifications => Some(RightBarIcon::Notifications),
        SidePanel::Chats => Some(RightBarIcon::Chats),
        SidePanel::Products => Some(RightBarIcon::Products),
        SidePanel::ObjectTree | SidePanel::DataWindow => Some(RightBarIcon::ObjectTree),
        SidePanel::Alerts => Some(RightBarIcon::Alerts),
    }
}

fn handle_watchlist_action(action: WatchlistAction, emitted: &mut Vec<Action>) {
    match action {
        WatchlistAction::None => {}
        other => emitted.push(Action::Watchlist(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{SIZE, TestFeed, frame};
    use super::*;
    use std::sync::Arc;

    #[test]
    fn feed_panels_open_from_the_rail_and_help_reaches_shortcuts() {
        let mut terminal = Terminal::new(Arc::new(TestFeed)).with_symbol("AAPL");
        frame(&mut terminal, SIZE, Vec::new());
        assert!(
            terminal
                .panels
                .widget_bar
                .icons
                .contains(&RightBarIcon::News)
        );
        terminal.run(Action::WidgetBar(WidgetBarAction::Clicked(RightBarIcon::News)));
        assert_eq!(terminal.panel, Some(SidePanel::News));
        terminal.run(Action::News(NewsAction::ShowOnChart(chrono::Utc::now())));
        // Panels leave Help for studies, which opens the shortcuts dialog.
        assert!(
            terminal
                .run(Action::WidgetBar(WidgetBarAction::Clicked(
                    RightBarIcon::Help
                )))
                .is_none()
        );
        assert!(terminal.studies_modal());
    }

    #[test]
    fn screener_symbol_switches_the_chart() {
        let mut terminal = Terminal::new(Arc::new(TestFeed)).with_symbol("AAPL");
        frame(&mut terminal, SIZE, Vec::new());
        terminal.run(Action::Screener(ScreenerAction::SelectSymbol(
            "MSFT".into(),
        )));
        assert_eq!(terminal.chart().symbol(), "MSFT");
    }
}
