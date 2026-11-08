//! Live alert sources retain their interval and inputs independently of chart navigation and replay.

use super::{DemoApp, DemoSymbol, LiveFeed};
use egui_charts::model::{Alert, AlertCondition, AlertSource, AlertStatus, Timeframe};
use egui_charts::widget::{Chart, SeriesId};
use std::collections::HashMap;
use std::sync::Arc;

pub(super) struct AlertMonitor {
    chart: Chart,
    live: Arc<LiveFeed>,
    studies: HashMap<String, SeriesId>,
    sources: HashMap<String, Arc<LiveFeed>>,
    alert_sources: Vec<AlertSource>,
    /// Length and last bar of each requested history at the last refresh, which tells a new
    /// request history from an unchanged one.
    requested_shape: Vec<(usize, Option<egui_charts::model::Bar>)>,
    /// Feed generations already snapshotted. `None` forces the first snapshot.
    seen_live: Option<u64>,
    seen_sources: Vec<u64>,
}

/// The studies of `source` an alert reads: those it names, and the studies they take as source.
fn needed_studies(source: &Chart, alert: &Alert) -> std::collections::HashSet<usize> {
    let mut needed: std::collections::HashSet<usize> = alert_sources(alert)
        .filter_map(|source| match source {
            AlertSource::Study { id, .. } => id.parse().ok(),
            _ => None,
        })
        .collect();
    loop {
        let parents: Vec<usize> = source
            .indicators()
            .filter(|(id, _)| needed.contains(&id.0))
            .filter_map(|(_, study)| match study.source() {
                Some(egui_charts::studies::StudySource::Study { id, .. }) => Some(id),
                _ => None,
            })
            .filter(|parent| !needed.contains(parent))
            .collect();
        if parents.is_empty() {
            return needed;
        }
        needed.extend(parents);
    }
}

/// Whether the alert tests a drawing, so its monitor needs the chart's drawings.
fn reads_drawings(alert: &Alert) -> bool {
    alert_sources(alert).any(|source| matches!(source, AlertSource::Drawing(_)))
}

impl AlertMonitor {
    fn new(source: &Chart, feed: &LiveFeed, symbol: &str, alert: &Alert) -> Self {
        Self::with_studies(source, feed, symbol, alert, needed_studies(source, alert))
    }

    /// A monitor that copies the studies of `source` whose ids are in `needed`.
    fn with_studies(
        source: &Chart,
        feed: &LiveFeed,
        symbol: &str,
        alert: &Alert,
        needed: std::collections::HashSet<usize>,
    ) -> Self {
        let live = Arc::new(feed.fork());
        live.set_symbol(symbol);
        let timeframe = alert
            .timeframe
            .or(source.timeframe())
            .unwrap_or(Timeframe::Min1);
        let mut chart = egui_charts::ChartBuilder::new()
            .with_symbol(symbol)
            .with_timeframe(timeframe)
            .with_chart_options(source.chart_options().clone())
            .build();
        chart.set_extended_hours(source.extended_hours());
        chart.set_instrument(Some(live.instrument()));
        let ids: Vec<_> = chart.indicators().map(|(id, _)| id).collect();
        for id in ids {
            chart.remove_series(id);
        }
        let mut studies: HashMap<String, SeriesId> = HashMap::new();
        // Only the studies this alert reads are recomputed on every refresh.
        let mut pending: Vec<_> = source
            .indicators()
            .filter(|(id, _)| needed.contains(&id.0))
            .map(|(id, study)| (id, study.clone_box()))
            .collect();
        while !pending.is_empty() {
            let Some(at) = pending.iter().position(|(_, study)| match study.source() {
                Some(egui_charts::studies::StudySource::Study { id, .. }) => {
                    !pending.iter().any(|(key, _)| key.0 == id)
                }
                _ => true,
            }) else {
                break;
            };
            let (id, mut study) = pending.remove(at);
            if let Some(egui_charts::studies::StudySource::Study { id: parent, plot }) =
                study.source()
            {
                let parent = studies
                    .get(&parent.to_string())
                    .map_or(usize::MAX, |id| id.0);
                study
                    .set_input(
                        "source",
                        egui_charts::studies::InputValue::Source(
                            egui_charts::studies::StudySource::Study { id: parent, plot },
                        ),
                    )
                    .expect("copied study source");
            }
            studies.insert(id.0.to_string(), chart.add_indicator(study));
        }
        if source.symbol() == symbol && reads_drawings(alert) {
            chart.drawings_mut().drawings = source.drawings().drawings.clone();
        }
        let sources = chart
            .indicators()
            .flat_map(|(_, study)| study.data_requests())
            .filter_map(|request| {
                if request.symbol.is_empty()
                    || request.symbol == symbol
                    || !DemoSymbol::catalog()
                        .iter()
                        .any(|spec| spec.symbol == request.symbol)
                {
                    return None;
                }
                let feed = Arc::new(live.fork());
                feed.set_symbol(&request.symbol);
                Some((request.symbol, feed))
            })
            .collect();
        Self {
            chart,
            live,
            studies,
            sources,
            alert_sources: alert_sources(alert).cloned().collect(),
            requested_shape: Vec::new(),
            seen_live: None,
            seen_sources: Vec::new(),
        }
    }

    /// The value of `source` at `bar`; study sources name the studies of the chart the alert was
    /// made on, which `studies` maps to this copy's.
    fn value(&self, source: &AlertSource, bar: usize) -> Option<f64> {
        match source {
            AlertSource::Study { id, output } => {
                let id = self.studies.get(id)?.0.to_string();
                self.chart.alert_value(
                    &AlertSource::Study {
                        id,
                        output: *output,
                    },
                    bar,
                )
            }
            _ => self.chart.alert_value(source, bar),
        }
    }

    fn for_symbol(source: &Self, live: &LiveFeed, symbol: &str, alert: &Alert) -> Self {
        let live = live.fork();
        let (session, adjusted) = source.live.view(source.chart.extended_hours());
        live.set_view(session, adjusted);
        // The source monitor already holds only the studies this alert reads, under copied ids.
        let needed = source.chart.indicators().map(|(id, _)| id.0).collect();
        let mut next = Self::with_studies(&source.chart, &live, symbol, alert, needed);
        next.studies = source
            .studies
            .iter()
            .filter_map(|(original, copied)| {
                next.studies
                    .get(&copied.0.to_string())
                    .map(|id| (original.clone(), *id))
            })
            .collect();
        next
    }

    fn refresh(
        &mut self,
        now: chrono::DateTime<chrono::Utc>,
        paused: bool,
    ) -> (std::time::Duration, bool) {
        self.live.set_paused(paused);
        let mut delay = self.live.advance(now);
        for feed in self.sources.values() {
            feed.set_paused(paused);
            delay = delay.min(feed.advance(now));
        }
        let live_generation = self.live.generation();
        let source_generations: Vec<_> = self
            .sources
            .values()
            .map(|feed| feed.generation())
            .collect();
        if self.seen_live == Some(live_generation) && self.seen_sources == source_generations {
            return (delay, false);
        }
        self.seen_live = Some(live_generation);
        self.seen_sources = source_generations;
        let timeframe = self.chart.timeframe().unwrap_or(Timeframe::Min1);
        let bars = self.live.snapshot(
            timeframe,
            self.chart.bar_clock(),
            self.chart.extended_hours(),
        );
        let requested: Vec<_> = self
            .chart
            .indicators()
            .flat_map(|(_, study)| study.data_requests())
            .map(|request| {
                let available = request.symbol.is_empty()
                    || request.symbol == self.chart.symbol()
                    || self.sources.contains_key(&request.symbol);
                let feed = self.sources.get(&request.symbol).unwrap_or(&self.live);
                let timeframe = request.timeframe.unwrap_or(timeframe);
                let instrument = feed.instrument();
                let clock = egui_charts::model::BarClock::new(timeframe);
                let clock = match &self.chart.chart_options().time_scale.timezone {
                    egui_charts::config::TimezoneMode::Local => clock.with_local_timezone(),
                    zone => clock.with_timezone(
                        zone.to_tz()
                            .unwrap_or(egui_charts::model::TimezoneId::utc()),
                    ),
                };
                let clock = if let Some(session) = &instrument.session {
                    clock.with_session(
                        instrument
                            .time_zone
                            .clone()
                            .unwrap_or(egui_charts::model::TimezoneId::utc()),
                        session.clone(),
                        self.chart.extended_hours(),
                    )
                } else {
                    clock
                };
                let values = if !available {
                    Vec::new()
                } else if request.metric.as_deref() == Some("open_interest") {
                    feed.metric_snapshot()
                        .into_iter()
                        .map(egui_charts::model::Bar::from)
                        .collect()
                } else if request.metric.is_some() {
                    Vec::new()
                } else {
                    feed.snapshot(timeframe, clock.clone(), self.chart.extended_hours())
                };
                (request, values, clock, instrument, timeframe)
            })
            .collect();
        let sources: Vec<_> = requested
            .iter()
            .map(
                |(request, bars, clock, instrument, timeframe)| egui_charts::studies::StudyData {
                    request: request.clone(),
                    bars,
                    clock: clock.clone(),
                    instrument: Some(instrument),
                    timeframe: *timeframe,
                },
            )
            .collect();
        let shape: Vec<_> = requested
            .iter()
            .map(|(_, values, ..)| (values.len(), values.last().cloned()))
            .collect();
        let previous = self.chart.bars(SeriesId::MAIN);
        // Studies keep the unchanged prefix, so a live tick only recomputes the last bars.
        let first_changed = previous
            .iter()
            .zip(&bars)
            .position(|(old, new)| old != new)
            .unwrap_or(previous.len().min(bars.len()));
        let bars_changed = first_changed < bars.len() || previous.len() != bars.len();
        let requests_changed = shape != self.requested_shape;
        if bars_changed || requests_changed {
            let from = if requests_changed || previous.len() > bars.len() {
                0
            } else {
                first_changed
            };
            self.requested_shape = shape;
            self.chart.set_data(SeriesId::MAIN, bars.clone());
            let mut pending: Vec<_> = self.chart.indicators().map(|(id, _)| id).collect();
            while !pending.is_empty() {
                let Some(at) = pending.iter().position(|id| {
                    let study = self
                        .chart
                        .indicators()
                        .find(|(key, _)| key == id)
                        .unwrap()
                        .1;
                    match study.source() {
                        Some(egui_charts::studies::StudySource::Study { id, .. }) => {
                            !pending.contains(&SeriesId(id))
                        }
                        _ => true,
                    }
                }) else {
                    break;
                };
                let id = pending.remove(at);
                let source = self
                    .chart
                    .indicators()
                    .find(|(key, _)| *key == id)
                    .unwrap()
                    .1
                    .source();
                let values = match source {
                    Some(egui_charts::studies::StudySource::Study { id, plot }) => self
                        .chart
                        .indicators()
                        .find(|(key, _)| key.0 == id)
                        .and_then(|(_, study)| study.values().columns().get(plot))
                        .cloned(),
                    _ => None,
                };
                let study = self.chart.indicator_mut(id).unwrap();
                if matches!(
                    source,
                    Some(egui_charts::studies::StudySource::Study { .. })
                ) {
                    study.update_source(&bars, from, values.as_deref().unwrap_or(&[]));
                } else {
                    study.update_data(&bars, from, &sources);
                }
            }
        }
        (delay, bars_changed || requests_changed)
    }
}

fn alert_sources(alert: &Alert) -> impl Iterator<Item = &AlertSource> {
    std::iter::once((&alert.source, &alert.condition))
        .chain(
            alert
                .additional_conditions
                .iter()
                .map(|rule| (&rule.source, &rule.condition)),
        )
        .flat_map(|(source, condition)| {
            std::iter::once(source).chain(match condition {
                AlertCondition::Compare { other, .. } => Some(other),
                _ => None,
            })
        })
}

impl DemoApp {
    /// An active alert read from the chart's own bars: its symbol and interval are on screen and
    /// replay is not replacing them.
    fn alert_uses_chart(&self, alert: &Alert) -> bool {
        self.replay.is_none()
            && alert.watchlist.is_none()
            && alert.symbol == self.chart.symbol()
            && alert
                .timeframe
                .is_none_or(|timeframe| Some(timeframe) == self.chart.timeframe())
    }

    pub(super) fn monitor_alerts(&mut self, ctx: &egui::Context) {
        let now = chrono::Utc::now();
        let paused = if self.replay.is_some() {
            self.live_was_paused
        } else {
            self.live.paused()
        };
        let engine = self.alert_host.engine();
        engine.tick(now);
        for list in self.watchlist.lists() {
            engine.set_watchlist(&list.name, list.symbols().map(str::to_owned).collect());
        }
        let alerts = self.alerts.alerts();
        if alerts
            .iter()
            .any(|alert| alert.status == AlertStatus::Active && alert.expires.is_some())
        {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        let chart_symbol = self.chart.symbol().to_owned();
        let on_chart_ids: Vec<_> = alerts
            .iter()
            .filter(|alert| self.alert_uses_chart(alert))
            .filter_map(|alert| alert.id.clone())
            .collect::<Vec<_>>();
        self.alert_monitors.retain(|(id, symbol), _| {
            alerts.iter().any(|alert| alert.id.as_ref() == Some(id))
                && !super::binance::uses_binance(symbol)
                && !(on_chart_ids.contains(id) && symbol == &chart_symbol)
        });
        let on_chart_ids: Vec<_> = alerts
            .iter()
            .filter(|alert| alert.status == AlertStatus::Active && self.alert_uses_chart(alert))
            .filter_map(|alert| alert.id.clone())
            .collect();
        let bars = self.chart.bars(SeriesId::MAIN).to_vec();
        let chart_key = (bars.len(), bars.last().cloned());
        if !on_chart_ids.is_empty()
            && (self.alert_chart_key.as_ref() != Some(&chart_key)
                || self.alert_on_chart_ids != on_chart_ids)
        {
            let symbol = self.chart.symbol().to_owned();
            let timeframe = self.chart.timeframe().unwrap_or(Timeframe::Min1);
            self.alert_evaluations += 1;
            engine.update(&symbol, timeframe, &bars, now, |source, bar| {
                self.chart.alert_value(source, bar)
            });
            self.alert_chart_key = Some(chart_key);
            self.alert_on_chart_ids = on_chart_ids;
        }
        let mut refreshed = Vec::new();
        for alert in alerts
            .iter()
            .filter(|alert| alert.status == AlertStatus::Active)
        {
            let Some(id) = &alert.id else { continue };
            for ((current, symbol), monitor) in &mut self.alert_monitors {
                if current == id && monitor.alert_sources.iter().ne(alert_sources(alert)) {
                    let mut next = AlertMonitor::new(&self.chart, &monitor.live, symbol, alert);
                    next.chart
                        .set_timeframe(monitor.chart.timeframe().unwrap_or(Timeframe::Min1));
                    next.chart
                        .set_chart_options(monitor.chart.chart_options().clone());
                    next.chart
                        .set_extended_hours(monitor.chart.extended_hours());
                    if self.chart.symbol() != symbol && reads_drawings(alert) {
                        next.chart.drawings_mut().drawings =
                            monitor.chart.drawings().drawings.clone();
                    }
                    *monitor = next;
                }
            }
            let symbols: Vec<_> = alert.watchlist.as_ref().map_or_else(
                || vec![alert.symbol.clone()],
                |name| {
                    self.watchlist
                        .lists()
                        .iter()
                        .filter(|list| list.name == *name)
                        .flat_map(|list| list.symbols().map(str::to_owned))
                        .collect()
                },
            );
            for symbol in symbols {
                if !DemoSymbol::catalog()
                    .iter()
                    .any(|spec| spec.symbol == symbol)
                {
                    continue;
                }
                if super::binance::uses_binance(&symbol) {
                    let covered = self.alert_uses_chart(alert) && symbol == self.chart.symbol();
                    let waits = !covered
                        && (symbol != self.chart.symbol()
                            || alert.timeframe.is_some_and(|timeframe| {
                                Some(timeframe) != self.chart.timeframe()
                            }));
                    if waits
                        && self
                            .exchange_alert_notices
                            .insert((id.clone(), symbol.clone()))
                    {
                        self.toasts.info(egui_charts::localization::text(
                            ctx,
                            "This alert is checked while the chart shows its symbol and interval.",
                        ));
                    }
                    continue;
                }
                if self.alert_uses_chart(alert) && symbol == self.chart.symbol() {
                    continue;
                }
                let key = (id.clone(), symbol.clone());
                if !self.alert_monitors.contains_key(&key) {
                    let monitor = self
                        .alert_monitors
                        .iter()
                        .find(|((current, _), _)| current == id)
                        .map_or_else(
                            || AlertMonitor::new(&self.chart, &self.live, &symbol, alert),
                            |(_, source)| {
                                AlertMonitor::for_symbol(source, &self.live, &symbol, alert)
                            },
                        );
                    self.alert_monitors.insert(key.clone(), monitor);
                }
                let monitor = self.alert_monitors.get_mut(&key).unwrap();
                if self.replay.is_none()
                    && self.chart.symbol() == symbol
                    && self.chart.timeframe() == monitor.chart.timeframe()
                    && reads_drawings(alert)
                {
                    monitor.chart.drawings_mut().drawings = self.chart.drawings().drawings.clone();
                }
                let (delay, changed) = monitor.refresh(now, paused);
                if !monitor.live.paused() {
                    ctx.request_repaint_after(delay);
                }
                if changed {
                    refreshed.push(key);
                }
            }
        }
        // The engine evaluates every alert on a symbol and timeframe at once, so each is fed one
        // series, with a source resolved by the first of its monitors that knows the study or
        // drawing.
        let mut fed = Vec::new();
        for (index, key @ (_, symbol)) in refreshed.iter().enumerate() {
            let monitor = &self.alert_monitors[key];
            let timeframe = monitor.chart.timeframe().unwrap_or(Timeframe::Min1);
            let bars = monitor.chart.bars(SeriesId::MAIN);
            if fed.contains(&(symbol, timeframe)) {
                continue;
            }
            fed.push((symbol, timeframe));
            let group: Vec<_> = refreshed[index..]
                .iter()
                .filter(|(_, other)| other == symbol)
                .map(|key| &self.alert_monitors[key])
                .filter(|other| other.chart.bars(SeriesId::MAIN) == bars)
                .collect();
            self.alert_evaluations += 1;
            engine.update(symbol, timeframe, bars, now, |source, bar| {
                group.iter().find_map(|monitor| monitor.value(source, bar))
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_preserves_alert_pause_state() {
        for paused in [false, true] {
            let ctx = egui::Context::default();
            let mut app = DemoApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
            app.live.set_paused(paused);
            app.alerts
                .save(Alert::new(app.chart.symbol(), AlertCondition::Above(0.0)));
            app.alerts.poll(&ctx, &mut app.toasts);
            app.start_replay();
            assert!(app.replay.is_some());
            app.monitor_alerts(&ctx);
            assert_eq!(app.alert_monitors.len(), 1);
            let monitor = app.alert_monitors.values().next().unwrap();
            assert_eq!(monitor.live.paused(), paused);
        }
    }

    /// Two alerts on one symbol fire once each on unchanged bars, although only the later
    /// alert's monitor knows the study the later alert tests.
    #[test]
    fn alerts_on_one_symbol_see_one_series() {
        let ctx = egui::Context::default();
        let mut app = DemoApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.live.set_paused(true);
        let timeframe = app.chart.timeframe().unwrap_or(Timeframe::Min1);
        let bars = app
            .live
            .snapshot(timeframe, app.chart.bar_clock(), app.chart.extended_hours());
        app.chart.set_data(SeriesId::MAIN, bars);
        let mut price = Alert::new(app.chart.symbol(), AlertCondition::Above(0.0));
        price.frequency = egui_charts::model::AlertFrequency::EveryTime;
        app.alerts.save(price.clone());
        app.alerts.poll(&ctx, &mut app.toasts);
        app.monitor_alerts(&ctx);
        let study = app
            .chart
            .add_indicator(Box::new(egui_charts::studies::SMA::new(2)));
        let mut average = price;
        average.source = AlertSource::Study {
            id: study.0.to_string(),
            output: 0,
        };
        app.alerts.save(average);
        for _ in 0..3 {
            app.alerts.poll(&ctx, &mut app.toasts);
            app.monitor_alerts(&ctx);
        }
        app.alerts.poll(&ctx, &mut app.toasts);
        assert_eq!(app.alerts.log().len(), 2);
    }

    /// A study alert copies only the study it reads, so the chart's other studies are not
    /// recomputed on every refresh, and a price alert copies none.
    #[test]
    fn monitors_copy_only_the_studies_their_alert_reads() {
        let ctx = egui::Context::default();
        let mut app = DemoApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.live.set_paused(true);
        let average = app
            .chart
            .add_indicator(Box::new(egui_charts::studies::SMA::new(2)));
        app.chart
            .add_indicator(Box::new(egui_charts::studies::RSI::new(14)));
        app.chart
            .add_indicator(Box::new(egui_charts::studies::SMA::new(9)));
        let price = Alert::new("AAPL", AlertCondition::Above(0.0));
        let mut study = price.clone();
        study.source = AlertSource::Study {
            id: average.0.to_string(),
            output: 0,
        };
        app.alerts.save(price);
        app.alerts.save(study);
        app.alerts.poll(&ctx, &mut app.toasts);
        app.monitor_alerts(&ctx);
        let mut copied: Vec<_> = app
            .alert_monitors
            .values()
            .map(|monitor| monitor.chart.indicators().count())
            .collect();
        copied.sort_unstable();
        assert_eq!(copied, [0, 1]);
    }

    #[test]
    fn on_chart_alerts_use_the_chart_and_no_monitor() {
        let ctx = egui::Context::default();
        let mut app = DemoApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.live.set_paused(true);
        let open = "2026-06-01T14:00:00Z".parse().unwrap();
        app.chart.set_data(
            SeriesId::MAIN,
            vec![egui_charts::model::Bar::new(
                open, 10.0, 10.0, 10.0, 10.0, 1.0,
            )],
        );
        let study = app
            .chart
            .add_indicator(Box::new(egui_charts::studies::SMA::new(2)));
        let mut price = Alert::new(app.chart.symbol(), AlertCondition::Above(15.0));
        price.timeframe = app.chart.timeframe();
        let mut study_alert = price.clone();
        study_alert.condition = AlertCondition::Above(0.0);
        study_alert.source = AlertSource::Study {
            id: study.0.to_string(),
            output: 0,
        };
        app.alerts.save(price);
        app.alerts.save(study_alert);
        app.alerts.poll(&ctx, &mut app.toasts);
        for _ in 0..3 {
            app.monitor_alerts(&ctx);
        }
        assert!(app.alert_monitors.is_empty());
        let next = open + chrono::TimeDelta::minutes(1);
        app.chart.update(
            SeriesId::MAIN,
            egui_charts::model::Bar::new(next, 16.0, 20.0, 16.0, 18.0, 1.0),
        );
        app.monitor_alerts(&ctx);
        app.alerts.poll(&ctx, &mut app.toasts);
        let crosses = app
            .alerts
            .log()
            .iter()
            .filter(|trigger| trigger.price == Some(18.0))
            .count();
        assert_eq!(crosses, 1);
        app.monitor_alerts(&ctx);
        app.alerts.poll(&ctx, &mut app.toasts);
        assert_eq!(
            app.alerts
                .log()
                .iter()
                .filter(|trigger| trigger.price == Some(18.0))
                .count(),
            1
        );
    }

    #[test]
    fn pointer_only_frames_do_no_alert_work() {
        let ctx = egui::Context::default();
        let mut app = DemoApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.live.set_paused(true);
        let open = "2026-06-01T14:00:00Z".parse().unwrap();
        app.chart.set_data(
            SeriesId::MAIN,
            vec![egui_charts::model::Bar::new(
                open, 10.0, 10.0, 10.0, 10.0, 1.0,
            )],
        );
        let mut alert = Alert::new(app.chart.symbol(), AlertCondition::Above(100.0));
        alert.timeframe = app.chart.timeframe();
        app.alerts.save(alert);
        app.alerts.poll(&ctx, &mut app.toasts);
        app.monitor_alerts(&ctx);
        let evaluations = app.alert_evaluations;
        let logged = app.alerts.log().len();
        assert!(evaluations > 0);
        assert!(app.alert_monitors.is_empty());
        for _ in 0..5 {
            app.monitor_alerts(&ctx);
        }
        assert_eq!(app.alert_evaluations, evaluations);
        assert_eq!(app.alerts.log().len(), logged);
    }

    #[test]
    fn exchange_symbols_get_no_synthetic_monitor() {
        let ctx = egui::Context::default();
        let mut app = DemoApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.live.set_paused(true);
        let mut alert = Alert::new("BTCUSDT", AlertCondition::Above(0.0));
        alert.timeframe = Some(Timeframe::Hour1);
        app.alerts.save(alert);
        app.alerts.poll(&ctx, &mut app.toasts);
        app.monitor_alerts(&ctx);
        app.monitor_alerts(&ctx);
        assert!(app.alert_monitors.is_empty());
        assert_eq!(
            app.alerts.alerts().last().unwrap().status,
            AlertStatus::Active
        );
        let notice = "This alert is checked while the chart shows its symbol and interval.";
        assert_eq!(
            app.toasts
                .toasts()
                .iter()
                .filter(|toast| toast.message == notice)
                .count(),
            1
        );
    }

    #[test]
    fn off_chart_monitors_skip_unchanged_feeds() {
        let ctx = egui::Context::default();
        let app = DemoApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.live.set_paused(true);
        let alert = Alert::new("AAPL", AlertCondition::Above(1.0e12));
        let mut monitor = AlertMonitor::new(&app.chart, &app.live, "AAPL", &alert);
        let now = app.live.now();
        let (_, first) = monitor.refresh(now, true);
        let bars = monitor.chart.bars(SeriesId::MAIN).to_vec();
        let (_, second) = monitor.refresh(now, true);
        assert!(first);
        assert!(!second);
        assert_eq!(monitor.chart.bars(SeriesId::MAIN), bars.as_slice());
    }

    /// Steady pointer frames with the chart's own study alerts. Run with
    /// `cargo test --release -- --ignored fifteen_study_alerts`.
    #[test]
    #[ignore = "release frame sample"]
    fn fifteen_study_alerts_add_under_a_millisecond_per_frame() {
        use eframe::App;
        use egui::{Event, Pos2, RawInput};
        use std::time::{Duration, Instant};

        let ctx = egui::Context::default();
        let mut app = DemoApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.live.set_paused(true);
        app.switch_symbol("ETHUSD");
        app.switch_timeframe(Timeframe::Hour1);
        let study = app
            .chart
            .add_indicator(Box::new(egui_charts::studies::GuppyMA::new()));
        app.chart
            .add_indicator(Box::new(egui_charts::studies::HMA::new(20)));
        app.chart
            .add_indicator(Box::new(egui_charts::studies::KAMA::new(10)));
        app.chart
            .add_indicator(Box::new(egui_charts::studies::LSMA::new(25)));
        app.chart
            .add_indicator(Box::new(egui_charts::studies::RSI::new(14)));
        app.chart
            .add_indicator(Box::new(egui_charts::studies::MACD::new(12, 26, 9)));
        app.chart
            .add_indicator(Box::new(egui_charts::studies::ElderRay::new(13)));
        let screen = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 720.0));
        let mut clock = 0.0f64;
        let mut frame = |app: &mut DemoApp| {
            clock += 0.016;
            let at = Pos2::new(240.0 + ((clock * 40.0) % 600.0) as f32, 360.0);
            let started = Instant::now();
            ctx.run_ui(
                RawInput {
                    screen_rect: Some(screen),
                    events: vec![Event::PointerMoved(at)],
                    time: Some(clock),
                    ..Default::default()
                },
                |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()),
            )
            .drop_without_applying_deltas();
            started.elapsed()
        };
        for _ in 0..8 {
            frame(&mut app);
        }
        let average = |app: &mut DemoApp, frame: &mut dyn FnMut(&mut DemoApp) -> Duration| {
            let mut total = Duration::ZERO;
            for _ in 0..20 {
                total += frame(app);
            }
            total / 20
        };
        let none = average(&mut app, &mut frame);
        for index in 0..15 {
            let mut alert = Alert::new("ETHUSD", AlertCondition::Above(1.0e18));
            alert.timeframe = Some(Timeframe::Hour1);
            alert.source = AlertSource::Study {
                id: study.0.to_string(),
                output: 0,
            };
            alert.message = format!("study {index}");
            app.alerts.save(alert);
            app.alerts.poll(&ctx, &mut app.toasts);
            if index == 4 || index == 14 {
                for _ in 0..4 {
                    frame(&mut app);
                }
                let mean = average(&mut app, &mut frame);
                let count = index + 1;
                eprintln!("alert frame {count}: {mean:?} (0 alerts: {none:?})");
                if count == 15 {
                    assert!(
                        mean.saturating_sub(none) < Duration::from_millis(1),
                        "15 alerts added {mean:?} against {none:?}"
                    );
                }
            }
        }
        assert!(app.alert_monitors.is_empty());
    }
}
