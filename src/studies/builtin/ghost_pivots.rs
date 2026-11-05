//! Ziv Ghost Pivots.
//!
//! Traditional pivot levels the next period will use if the current one ended now. The levels
//! are drawn beyond the last bar and move with every update until the period closes, when they
//! become the confirmed pivots of the following period. They are called ghost pivots because the
//! target does not exist yet.
//!
//! # Periods
//!
//! | Setting        | Period whose high, low and close feed the projection |
//! |----------------|------------------------------------------------------|
//! | 4 hour         | Clock-aligned four-hour block                        |
//! | Next day (8 h) | Clock-aligned eight-hour block                       |
//! | Next week      | Exchange trading week (default)                      |
//! | Next month     | Calendar month                                       |
//!
//! With H, L and C the high, low and latest close of the period so far, P = (H + L + C) / 3,
//! R1 = 2P − L, S1 = 2P − H, R2/S2 = P ± (H − L). Overlay indicator; only the projected levels
//! are drawn, not confirmed historical pivots.
//!
//! # Example
//!
//! ```rust
//! use chrono::{TimeZone, Utc};
//! use egui_charts::model::Bar;
//! use egui_charts::studies::{GhostPeriod, GhostPivots, Indicator};
//!
//! // Two hourly bars of one ISO week: P = (12 + 8 + 11) / 3.
//! let bars: Vec<Bar> = [(10.0, 8.0, 9.0), (12.0, 9.0, 11.0)]
//!     .into_iter()
//!     .enumerate()
//!     .map(|(i, (high, low, close))| {
//!         let time = Utc.with_ymd_and_hms(2026, 3, 2, 9 + i as u32, 0, 0).unwrap();
//!         Bar::new(time, close, high, low, close, 1.0)
//!     })
//!     .collect();
//! let mut pivots = GhostPivots::new().with_period(GhostPeriod::Weekly);
//! pivots.calculate(&bars);
//! assert!((pivots.projections()[0].top - 31.0 / 3.0).abs() < 1e-12);
//! ```

use super::anchor_period::{AnchorPeriod, ExchangeDays};
use super::pivot_points::{PivotMethod, PivotPoints};
use crate::model::{Bar, BarClock, Instrument, Timeframe, TimezoneId};
use crate::studies::palette;
use crate::studies::{Indicator, IndicatorValues, PlotStyle, StudyProjection};
use egui::Color32;
use std::num::NonZeroU64;

/// The forming period whose high, low and close project the next period's pivots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GhostPeriod {
    /// Clock-aligned four-hour block.
    Hour4,
    /// Clock-aligned eight-hour block (next day / 8 hour).
    Hour8,
    /// Exchange trading week.
    #[default]
    Weekly,
    /// Calendar month.
    Monthly,
}

impl GhostPeriod {
    fn clock(self) -> Option<BarClock> {
        match self {
            Self::Hour4 => Some(BarClock::new(Timeframe::Hour4)),
            Self::Hour8 => Some(BarClock::new(Timeframe::Custom(
                NonZeroU64::new(8 * 3_600).expect("8h"),
            ))),
            Self::Weekly | Self::Monthly => None,
        }
    }

    fn anchor(self) -> Option<AnchorPeriod> {
        match self {
            Self::Weekly => Some(AnchorPeriod::Week),
            Self::Monthly => Some(AnchorPeriod::Month),
            Self::Hour4 | Self::Hour8 => None,
        }
    }
}

/// Ziv Ghost Pivots: next-period traditional pivots from the period so far.
#[derive(Clone)]
pub struct GhostPivots {
    period: GhostPeriod,
    days: ExchangeDays,
    zone: TimezoneId,
    values: IndicatorValues,
    projections: Vec<StudyProjection>,
    colors: Vec<Color32>,
    visible: bool,
}

impl GhostPivots {
    /// Weekly ghost pivots.
    pub fn new() -> Self {
        Self {
            period: GhostPeriod::Weekly,
            days: ExchangeDays::default(),
            zone: TimezoneId::utc(),
            values: IndicatorValues::new(),
            projections: Vec::new(),
            colors: vec![palette::DARK_ORANGE; 5],
            visible: true,
        }
    }

    /// The forming period that feeds the projection.
    pub fn with_period(mut self, period: GhostPeriod) -> Self {
        self.period = period;
        self
    }

    fn project(&mut self, data: &[Bar], start: usize, high: f64, low: f64, close: f64) {
        self.projections.clear();
        if data.is_empty() || start >= data.len() || !(high.is_finite() && low.is_finite()) {
            return;
        }
        let close = if close.is_finite() { close } else { return };
        let last = data.len() - 1;
        let width = (data.len() - start) as f64;
        let period = Bar::new(data[start].time, close, high, low, close, 0.0);
        // Traditional P, R1, S1, R2, S2 only: the levels Booker's ghost targets use.
        let levels = PivotPoints::levels(PivotMethod::Traditional, &period, close);
        for (plot, price) in levels.into_iter().enumerate().take(5) {
            if !price.is_finite() {
                continue;
            }
            self.projections.push(StudyProjection {
                bars: last as f64..last as f64 + width,
                top: price,
                bottom: price,
                plot,
            });
        }
    }

    fn update_clock(&mut self, data: &[Bar], clock: &BarClock) {
        let mut start = None;
        let mut open_key = None;
        let mut high = f64::NEG_INFINITY;
        let mut low = f64::INFINITY;
        let mut close = f64::NAN;
        for (i, bar) in data.iter().enumerate() {
            if !(bar.high.is_finite() && bar.low.is_finite() && bar.close.is_finite()) {
                continue;
            }
            let key = clock.bar_open(bar.time);
            if open_key != Some(key) {
                open_key = Some(key);
                start = Some(i);
                high = bar.high;
                low = bar.low;
            } else {
                high = high.max(bar.high);
                low = low.min(bar.low);
            }
            close = bar.close;
        }
        if let Some(start) = start {
            self.project(data, start, high, low, close);
        } else {
            self.projections.clear();
        }
    }

    fn update_anchor(&mut self, data: &[Bar], anchor: AnchorPeriod) {
        let days = self.days.of(data);
        let mut start = None;
        let mut key = None;
        let mut high = f64::NEG_INFINITY;
        let mut low = f64::INFINITY;
        let mut close = f64::NAN;
        for (i, bar) in data.iter().enumerate() {
            if !(bar.high.is_finite() && bar.low.is_finite() && bar.close.is_finite()) {
                continue;
            }
            let next = anchor.key(days[i]);
            if key != Some(next) {
                key = Some(next);
                start = Some(i);
                high = bar.high;
                low = bar.low;
            } else {
                high = high.max(bar.high);
                low = low.min(bar.low);
            }
            close = bar.close;
        }
        if let Some(start) = start {
            self.project(data, start, high, low, close);
        } else {
            self.projections.clear();
        }
    }
}

impl Default for GhostPivots {
    fn default() -> Self {
        Self::new()
    }
}

impl Indicator for GhostPivots {
    crate::studies::inputs::study_inputs!(self;
        period: choice("Pivot Period", [
            GhostPeriod::Hour4,
            GhostPeriod::Hour8,
            GhostPeriod::Weekly,
            GhostPeriod::Monthly
        ])
    ; {});

    fn name(&self) -> &str {
        "Ziv Ghost Pivots"
    }

    fn desc(&self) -> &str {
        "Ziv Ghost Pivots - Next period's traditional pivots from the period so far"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], _from: usize) {
        // Projection-only studies still keep one missing output row per source bar, like every
        // other indicator, so incremental updates and hostile bars preserve index alignment.
        self.values = IndicatorValues::missing(data.len());
        if let Some(mut clock) = self.period.clock() {
            clock = clock.with_timezone(self.zone.clone());
            self.update_clock(data, &clock);
        } else if let Some(anchor) = self.period.anchor() {
            self.update_anchor(data, anchor);
        }
    }

    fn is_incremental(&self) -> bool {
        true
    }

    fn set_instrument(&mut self, instrument: Option<&Instrument>) -> bool {
        let changed = self.days.set(instrument);
        let next = instrument
            .and_then(|i| i.time_zone.clone())
            .unwrap_or_else(TimezoneId::utc);
        let zone_changed = self.zone != next;
        self.zone = next;
        changed || zone_changed
    }

    fn projections(&self) -> &[StudyProjection] {
        &self.projections
    }

    fn values(&self) -> &IndicatorValues {
        &self.values
    }

    fn plot_style(&self, line: usize) -> PlotStyle {
        PlotStyle {
            color: self
                .colors
                .get(line)
                .copied()
                .unwrap_or(palette::DARK_ORANGE),
            width: 1.0,
            pattern: crate::model::LineStyle::Dashed,
            plot_type: Default::default(),
            visible: self.visible,
        }
    }

    fn colors(&self) -> Vec<Color32> {
        self.colors.clone()
    }

    fn set_colors(&mut self, colors: Vec<Color32>) {
        for (slot, color) in self.colors.iter_mut().zip(colors) {
            *slot = color;
        }
    }

    fn is_overlay(&self) -> bool {
        true
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn clone_box(&self) -> Box<dyn Indicator> {
        Box::new(self.clone())
    }

    fn line_names(&self) -> Vec<String> {
        ["P", "R1", "S1", "R2", "S2"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn weekly_ghost_projects_from_the_week_so_far() {
        let hour = |day, h| Utc.with_ymd_and_hms(2026, 3, day, h, 0, 0).unwrap();
        let data: Vec<Bar> = [
            (hour(2, 9), 10.0, 8.0, 9.0),
            (hour(2, 10), 12.0, 9.0, 11.0),
            (hour(3, 9), 11.0, 9.0, 10.0),
        ]
        .into_iter()
        .map(|(time, high, low, close)| Bar::new(time, close, high, low, close, 1.0))
        .collect();
        let mut pivots = GhostPivots::new().with_period(GhostPeriod::Weekly);
        pivots.calculate(&data);
        let p = (12.0 + 8.0 + 10.0) / 3.0;
        assert!((pivots.projections()[0].top - p).abs() < 1e-12);
        assert_eq!(pivots.projections()[0].bars, 2.0..5.0);
        assert!((pivots.projections()[1].top - (2.0 * p - 8.0)).abs() < 1e-12);
        assert!((pivots.projections()[2].top - (2.0 * p - 12.0)).abs() < 1e-12);
    }

    #[test]
    fn four_hour_ghost_resets_on_clock_blocks() {
        let at = |h| Utc.with_ymd_and_hms(2026, 3, 2, h, 0, 0).unwrap();
        // 08:00–12:00 block, then 12:00–16:00 block; ghost uses the second.
        let data: Vec<Bar> = [
            (at(8), 10.0, 9.0, 9.5),
            (at(9), 11.0, 9.0, 10.0),
            (at(12), 14.0, 12.0, 13.0),
            (at(13), 15.0, 12.5, 14.0),
        ]
        .into_iter()
        .map(|(time, high, low, close)| Bar::new(time, close, high, low, close, 1.0))
        .collect();
        let mut pivots = GhostPivots::new().with_period(GhostPeriod::Hour4);
        pivots.calculate(&data);
        let p = (15.0 + 12.0 + 14.0) / 3.0;
        assert!((pivots.projections()[0].top - p).abs() < 1e-12);
        assert_eq!(pivots.projections()[0].bars, 3.0..5.0);
    }
}
