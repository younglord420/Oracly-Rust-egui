//! Levels of the long and short position drawings: entry, target and stop, the direction the
//! target gives them, and how placement and handle drags keep the stop and target on opposite
//! sides of the entry.

use super::{ChartPoint, Drawing, DrawingToolType, PositionDisplay, PositionSize};
use egui::Pos2;

/// Reward over risk a placed position starts with: the stop sits at half the target's distance.
pub const POSITION_REWARD_RATIO: f64 = 2.0;
/// A second click or drag closer than this to the entry places a default box.
const MIN_DRAG: f32 = 6.0;
/// Risk, in points, of the default box.
const DEFAULT_RISK: f32 = 64.0;
/// Width, in points, of the default box.
const DEFAULT_WIDTH: f32 = 150.0;
/// Narrowest box a placement makes.
const MIN_WIDTH: f32 = 48.0;

impl Drawing {
    /// Whether this is a long or short position drawing.
    pub fn is_position(&self) -> bool {
        matches!(
            self.tool_type,
            DrawingToolType::LongPos | DrawingToolType::ShortPos
        )
    }

    /// Entry, target and stop prices. While only the entry and target are placed, the stop sits
    /// opposite the target at the starting reward ratio.
    pub fn position_prices(&self) -> Option<(f64, f64, f64)> {
        let [entry, target, ..] = self.chart_points.as_slice() else {
            return None;
        };
        let stop = self.chart_points.get(2).map_or_else(
            || entry.price - (target.price - entry.price) / POSITION_REWARD_RATIO,
            |stop| stop.price,
        );
        Some((entry.price, target.price, stop))
    }

    /// A position is long while its target is at or above its entry, whichever tool placed it.
    pub fn position_is_long(&self) -> bool {
        self.position_prices()
            .is_none_or(|(entry, target, _)| target >= entry)
    }

    /// Turns the trade around in place: the target and stop mirror across the entry, keeping
    /// their distances and so the reward ratio.
    pub fn set_position_long(&mut self, long: bool) {
        if long == self.position_is_long() || self.chart_points.len() < 3 {
            return;
        }
        for level in 1..3 {
            self.mirror_position_level(level);
        }
    }

    /// Mirrors anchor `level` across the entry, in price and on screen.
    fn mirror_position_level(&mut self, level: usize) {
        let entry = self.chart_points[0].price;
        let point = &mut self.chart_points[level];
        point.price = 2.0 * entry - point.price;
        if let Some(&entry) = self.points.first()
            && let Some(screen) = self.points.get_mut(level)
        {
            screen.y = 2.0 * entry.y - screen.y;
        }
    }

    /// After anchor `moved` (0 entry, 1 target, 2 stop) was dragged, puts the stop and target
    /// back on opposite sides of the entry by mirroring the one that was not dragged.
    pub(crate) fn keep_position_sides(&mut self, moved: usize) {
        let Some((entry, target, stop)) = self.position_prices() else {
            return;
        };
        if self.chart_points.len() < 3 {
            return;
        }
        let (t, s) = ((target - entry).signum(), (stop - entry).signum());
        if target == entry || stop == entry || t != s {
            return;
        }
        self.mirror_position_level(if moved == 1 { 2 } else { 1 });
    }

    /// Lays a position out from its entry and the target its second click or drag gave: the box
    /// runs as far right as the target, at least [`MIN_WIDTH`], and the stop sits opposite at
    /// the starting reward ratio. A second point next to the entry places a default long box.
    pub fn lay_out_position(
        &mut self,
        x_to_bar: impl Fn(f32) -> f32,
        y_to_price: impl Fn(f32) -> f64,
    ) {
        let [entry, target, ..] = self.points.as_slice() else {
            return;
        };
        let (entry, target) = (*entry, *target);
        let (target_y, right) = if (target.y - entry.y).abs() >= MIN_DRAG {
            (
                target.y,
                entry.x + (target.x - entry.x).abs().max(MIN_WIDTH),
            )
        } else {
            (
                entry.y - DEFAULT_RISK * POSITION_REWARD_RATIO as f32,
                entry.x + DEFAULT_WIDTH,
            )
        };
        let stop_y = entry.y - (target_y - entry.y) / POSITION_REWARD_RATIO as f32;
        let bar = x_to_bar(right);
        self.points.truncate(1);
        self.chart_points.truncate(1);
        for y in [target_y, stop_y] {
            self.points.push(Pos2::new(right, y));
            self.chart_points.push(ChartPoint::new(bar, y_to_price(y)));
        }
    }

    /// Adds the stop to a position placed with only its entry and target.
    pub(crate) fn add_position_stop(&mut self) {
        let (&[entry, target], &[entry_at, target_at]) =
            (self.chart_points.as_slice(), self.points.as_slice())
        else {
            return;
        };
        let ratio = POSITION_REWARD_RATIO;
        self.chart_points.push(ChartPoint::new(
            target.bar_idx,
            entry.price - (target.price - entry.price) / ratio,
        ));
        self.points.push(Pos2::new(
            target_at.x,
            entry_at.y - (target_at.y - entry_at.y) / ratio as f32,
        ));
    }

    /// Saved labels and colours, or the defaults.
    pub fn position_display(&self) -> PositionDisplay {
        self.position_display.unwrap_or_default()
    }

    /// The account balance and the percent of it risked at the stop.
    pub fn position_account(&self) -> (f64, f64) {
        let (entry, _, stop) = self.position_prices().unwrap_or((0.0, 0.0, 0.0));
        match self.position_size {
            Some(size) => size.account_risk(entry, stop),
            None => (super::DEFAULT_ACCOUNT_BALANCE, 1.0),
        }
    }

    /// The amount lost at the stop and the size that loses it.
    pub fn position_loss_and_size(&self) -> (f64, f64) {
        let (balance, percent) = self.position_account();
        let loss = balance.max(0.0) * percent.max(0.0) / 100.0;
        let distance = self
            .position_prices()
            .map_or(0.0, |(entry, _, stop)| (entry - stop).abs());
        let size = if distance > 1e-12 {
            loss / distance
        } else {
            0.0
        };
        (loss, size)
    }

    /// Sizes the position by risking `percent` of `balance` at the stop.
    pub fn set_position_account(&mut self, balance: f64, percent: f64) {
        self.position_size = Some(PositionSize::RiskPercent {
            account_size: balance,
            percent,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(target: f64, stop: f64) -> Drawing {
        let mut drawing = Drawing::new(1, DrawingToolType::LongPos);
        drawing.chart_points = vec![
            ChartPoint::new(0.0, 100.0),
            ChartPoint::new(10.0, target),
            ChartPoint::new(10.0, stop),
        ];
        drawing.points = vec![
            Pos2::new(0.0, 1000.0 - 100.0),
            Pos2::new(100.0, (1000.0 - target) as f32),
            Pos2::new(100.0, (1000.0 - stop) as f32),
        ];
        drawing
    }

    /// The direction follows the target, flips by mirroring both levels, and a level dragged
    /// across the entry pushes the other one over.
    #[test]
    fn direction_follows_the_levels() {
        let mut drawing = placed(110.0, 95.0);
        assert!(drawing.position_is_long());
        drawing.set_position_long(false);
        assert_eq!(drawing.position_prices(), Some((100.0, 90.0, 105.0)));
        assert_eq!(drawing.points[1].y, 910.0);
        drawing.chart_points[2].price = 85.0;
        drawing.keep_position_sides(2);
        assert_eq!(drawing.position_prices(), Some((100.0, 110.0, 85.0)));
        assert!(drawing.position_is_long());
    }

    /// The second point sets the target and the box width; the stop sits at half the distance,
    /// and a click on the entry places the default long box.
    #[test]
    fn placement_lays_out_the_box() {
        let price = |y: f32| 1000.0 - f64::from(y);
        let mut drawing = Drawing::new(1, DrawingToolType::LongPos);
        drawing.points = vec![Pos2::new(100.0, 500.0), Pos2::new(110.0, 560.0)];
        drawing.chart_points = vec![ChartPoint::new(10.0, 500.0), ChartPoint::new(11.0, 440.0)];
        drawing.lay_out_position(|x| x / 10.0, price);
        assert_eq!(drawing.points[1], Pos2::new(148.0, 560.0));
        assert_eq!(drawing.points[2], Pos2::new(148.0, 470.0));
        assert!(!drawing.position_is_long());
        drawing.points.truncate(1);
        drawing.points.push(Pos2::new(101.0, 502.0));
        drawing.chart_points.truncate(1);
        drawing.lay_out_position(|x| x / 10.0, price);
        assert_eq!(drawing.points[1], Pos2::new(250.0, 372.0));
        assert_eq!(drawing.points[2], Pos2::new(250.0, 564.0));
        assert!(drawing.position_is_long());
    }

    /// A saved risk amount or quantity reads as the same risk against the default account.
    #[test]
    fn saved_sizes_read_as_account_risk() {
        let mut drawing = placed(110.0, 95.0);
        assert_eq!(drawing.position_account(), (10_000.0, 1.0));
        drawing.position_size = Some(PositionSize::RiskAmount(250.0));
        assert_eq!(drawing.position_account(), (10_000.0, 2.5));
        drawing.position_size = Some(PositionSize::Quantity(20.0));
        assert_eq!(drawing.position_account(), (10_000.0, 1.0));
        let (loss, size) = drawing.position_loss_and_size();
        assert_eq!((loss, size), (100.0, 20.0));
    }
}
