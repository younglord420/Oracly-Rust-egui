//! Saved options of the price action tools (fair value gap, order block, liquidity) and the
//! candle analysis they draw from: where a formation sits on the bars, and the bar on which
//! price later tests, half-fills, fills, breaks or sweeps it. Every event is the index of the
//! bar it happened on, or `None` while it has not happened, so a tool follows new bars.

use crate::config::DrawingToolType;
use crate::model::Bar;
use serde::{Deserialize, Serialize};

/// How many bars either side of the clicked one are searched for a fair value gap.
const GAP_REACH: usize = 3;

/// The event that stops a price action zone or level from running right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ZoneEnd {
    /// Price first trades back into the zone.
    Tested,
    /// Price reaches the zone's halfway line.
    Half,
    /// Price trades through the whole gap.
    #[default]
    Filled,
    /// A bar closes through the block's far side, or beyond the liquidity level.
    Broken,
    /// A bar trades through the liquidity level and closes back inside it.
    Swept,
    /// The zone runs on to the latest bar.
    Always,
}

impl ZoneEnd {
    /// The events a tool can stop at, in the order the Inputs list offers them.
    pub fn options(tool: DrawingToolType) -> &'static [Self] {
        match tool {
            DrawingToolType::OrderBlock => &[Self::Tested, Self::Half, Self::Broken, Self::Always],
            DrawingToolType::Liquidity => &[Self::Swept, Self::Broken, Self::Always],
            _ => &[Self::Tested, Self::Half, Self::Filled, Self::Always],
        }
    }

    /// The option's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Tested => "Until tested",
            Self::Half => "Until 50%",
            Self::Filled => "Until filled",
            Self::Broken => "Until broken",
            Self::Swept => "Until swept",
            Self::Always => "Always",
        }
    }
}

/// Options of a fair value gap, order block or liquidity level.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PriceActionConfig {
    /// How far right the zone or level runs.
    pub extend: ZoneEnd,
    /// Fill of a bullish zone as RGBA; its alpha is the zone's opacity.
    pub up_color: [u8; 4],
    /// Fill of a bearish zone as RGBA.
    pub down_color: [u8; 4],
    /// The dashed line at a zone's halfway price.
    pub midline: bool,
    /// The outline of a zone.
    pub border: bool,
    /// A zone that has run its course reads quieter.
    pub fade: bool,
    /// A filled gap that a bar closes beyond carries on the other way (an inverse gap); a broken
    /// block carries on as a breaker until price returns to it.
    pub flip: bool,
    /// An order block spans the candle's whole range rather than its body.
    pub full_range: bool,
    /// A cross marks the bar that swept a liquidity level.
    pub mark_sweep: bool,
}

impl Default for PriceActionConfig {
    fn default() -> Self {
        Self::for_tool(DrawingToolType::FairValueGap)
    }
}

impl PriceActionConfig {
    /// The options a new drawing of `tool` starts with.
    pub fn for_tool(tool: DrawingToolType) -> Self {
        let tokens = &crate::tokens::DESIGN_TOKENS.semantic.extended;
        let tint = |color: egui::Color32, alpha: u8| {
            let [r, g, b, _] = color.to_srgba_unmultiplied();
            [r, g, b, alpha]
        };
        let block = tool == DrawingToolType::OrderBlock;
        let alpha = if block { 41 } else { 51 };
        Self {
            extend: match tool {
                DrawingToolType::OrderBlock => ZoneEnd::Broken,
                DrawingToolType::Liquidity => ZoneEnd::Swept,
                _ => ZoneEnd::Filled,
            },
            up_color: tint(tokens.bullish, alpha),
            down_color: tint(tokens.bearish, alpha),
            midline: true,
            border: block,
            fade: true,
            flip: block,
            full_range: false,
            mark_sweep: true,
        }
    }
}

/// A zone as a tool reads it off the bars: its span, and the bars it starts, stops and flips at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneFormation {
    /// The bar the zone is placed on.
    pub index: usize,
    /// A bullish zone (demand, or a gap up).
    pub bull: bool,
    /// Upper price.
    pub top: f64,
    /// Lower price.
    pub bottom: f64,
    /// Halfway price.
    pub mid: f64,
    /// The bar the zone's box starts at.
    pub start: usize,
    /// The bar it stops at; `None` runs it on to the latest bar.
    pub end: Option<usize>,
    /// The bars it carries on between, the other way, once price flips it.
    pub flip: Option<(usize, Option<usize>)>,
}

/// What price has done to a fair value gap since it formed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GapState {
    /// The middle bar of the three.
    pub index: usize,
    /// A gap up.
    pub bull: bool,
    /// Upper price of the gap.
    pub top: f64,
    /// Lower price of the gap.
    pub bottom: f64,
    /// Price first traded back into the gap.
    pub tested: Option<usize>,
    /// Price reached the gap's halfway line.
    pub half: Option<usize>,
    /// Price traded through the whole gap.
    pub filled: Option<usize>,
    /// After filling, a bar closed beyond the gap's far side.
    pub inverted: Option<usize>,
}

/// The gap the bar at `i` leaves as the middle of three: bullish when the next bar's low stays
/// above the previous one's high, bearish when its high stays below the previous low.
fn gap_at(bars: &[Bar], i: usize) -> Option<(bool, f64, f64)> {
    let prev = bars.get(i.checked_sub(1)?)?;
    let next = bars.get(i + 1)?;
    if next.low > prev.high {
        Some((true, next.low, prev.high))
    } else if next.high < prev.low {
        Some((false, prev.low, next.high))
    } else {
        None
    }
}

/// A fair value gap at the bar at `i`, or at the nearest bar a few bars away that left one, and
/// what price has done to it since.
pub fn gap_state(bars: &[Bar], i: usize) -> Option<GapState> {
    let (index, (bull, top, bottom)) = (0..=GAP_REACH).find_map(|k| {
        let near = if k == 0 {
            [Some(i), None]
        } else {
            [i.checked_sub(k), Some(i + k)]
        };
        near.into_iter()
            .flatten()
            .find_map(|j| gap_at(bars, j).map(|gap| (j, gap)))
    })?;
    let mid = (top + bottom) / 2.0;
    let mut state = GapState {
        index,
        bull,
        top,
        bottom,
        tested: None,
        half: None,
        filled: None,
        inverted: None,
    };
    for (j, bar) in bars.iter().enumerate().skip(index + 2) {
        let reach = if bull { bar.low } else { bar.high };
        let into = |price: f64| if bull { reach <= price } else { reach >= price };
        if state.tested.is_none() && into(if bull { top } else { bottom }) {
            state.tested = Some(j);
        }
        if state.half.is_none() && into(mid) {
            state.half = Some(j);
        }
        if state.filled.is_none() && into(if bull { bottom } else { top }) {
            state.filled = Some(j);
        }
        if state.filled.is_some()
            && (if bull {
                bar.close < bottom
            } else {
                bar.close > top
            })
        {
            state.inverted = Some(j);
            break;
        }
    }
    Some(state)
}

/// What price has done to an order block since it formed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockState {
    /// The block's bar.
    pub index: usize,
    /// A down-close bar marks demand (a bullish block); an up-close one supply.
    pub bull: bool,
    /// Upper price of the block.
    pub top: f64,
    /// Lower price of the block.
    pub bottom: f64,
    /// Price came back into the block after leaving it.
    pub tested: Option<usize>,
    /// Price reached the block's halfway line after leaving it.
    pub half: Option<usize>,
    /// A bar closed through the block's far side.
    pub broken: Option<usize>,
    /// After the break, price returned to the block from the other side.
    pub retest: Option<usize>,
}

/// An order block on the bar at `i`, its body or its whole range, and what price has done to it
/// since. Tests count only once price has first left the block; a close through it breaks it.
pub fn block_state(bars: &[Bar], i: usize, full_range: bool) -> Option<BlockState> {
    let c = bars.get(i)?;
    let bull = c.close < c.open;
    let (top, bottom) = if full_range {
        (c.high, c.low)
    } else {
        (c.open.max(c.close), c.open.min(c.close))
    };
    let mid = (top + bottom) / 2.0;
    let mut state = BlockState {
        index: i,
        bull,
        top,
        bottom,
        tested: None,
        half: None,
        broken: None,
        retest: None,
    };
    let (mut left, mut left_after_break) = (false, false);
    for (j, b) in bars.iter().enumerate().skip(i + 1) {
        if state.broken.is_none() {
            if !left {
                left = if bull { b.low > top } else { b.high < bottom };
            } else {
                if state.tested.is_none() && (if bull { b.low <= top } else { b.high >= bottom }) {
                    state.tested = Some(j);
                }
                if state.half.is_none() && (if bull { b.low <= mid } else { b.high >= mid }) {
                    state.half = Some(j);
                }
            }
            if if bull {
                b.close < bottom
            } else {
                b.close > top
            } {
                state.broken = Some(j);
            }
        } else if !left_after_break {
            left_after_break = if bull { b.high < bottom } else { b.low > top };
        } else if if bull { b.high >= bottom } else { b.low <= top } {
            state.retest = Some(j);
            break;
        }
    }
    Some(state)
}

/// What price has done to a liquidity level since it formed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquidityState {
    /// The bar whose high or low the level rests beyond.
    pub index: usize,
    /// Resting above a high (buy-side) rather than below a low.
    pub high: bool,
    /// The level's price.
    pub price: f64,
    /// A bar traded through the level and closed back inside it.
    pub swept: Option<usize>,
    /// A bar closed beyond the level.
    pub broken: Option<usize>,
}

/// Liquidity resting beyond the high or low of the bar at `i`, and when price took it.
pub fn liquidity_state(bars: &[Bar], i: usize, high: bool) -> Option<LiquidityState> {
    let c = bars.get(i)?;
    let price = if high { c.high } else { c.low };
    let mut state = LiquidityState {
        index: i,
        high,
        price,
        swept: None,
        broken: None,
    };
    for (j, b) in bars.iter().enumerate().skip(i + 1) {
        if if high {
            b.close > price
        } else {
            b.close < price
        } {
            state.broken = Some(j);
            break;
        }
        if state.swept.is_none() && (if high { b.high > price } else { b.low < price }) {
            state.swept = Some(j);
        }
    }
    Some(state)
}

impl PriceActionConfig {
    /// The zone a fair value gap or order block on the bar at `at` forms, with the bars it
    /// stops and flips at under these options.
    pub fn zone(&self, tool: DrawingToolType, bars: &[Bar], at: usize) -> Option<ZoneFormation> {
        let always = self.extend == ZoneEnd::Always;
        if tool == DrawingToolType::OrderBlock {
            let s = block_state(bars, at, self.full_range)?;
            let end = match self.extend {
                ZoneEnd::Tested => s.tested,
                ZoneEnd::Half => s.half,
                _ => s.broken,
            };
            let flip = (self.flip && self.extend == ZoneEnd::Broken)
                .then_some(s.broken)
                .flatten()
                .map(|from| (from, s.retest));
            return Some(ZoneFormation {
                index: s.index,
                bull: s.bull,
                top: s.top,
                bottom: s.bottom,
                mid: (s.top + s.bottom) / 2.0,
                start: s.index,
                end: if always { None } else { end },
                flip,
            });
        }
        let s = gap_state(bars, at)?;
        let end = match self.extend {
            ZoneEnd::Tested => s.tested,
            ZoneEnd::Half => s.half,
            _ => s.filled,
        };
        let flip = (self.flip && self.extend == ZoneEnd::Filled)
            .then_some(s.inverted)
            .flatten()
            .map(|from| (from, None));
        Some(ZoneFormation {
            index: s.index,
            bull: s.bull,
            top: s.top,
            bottom: s.bottom,
            mid: (s.top + s.bottom) / 2.0,
            start: s.index - 1,
            end: if always { None } else { end },
            flip,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn bars(ohlc: &[[f64; 4]]) -> Vec<Bar> {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        ohlc.iter()
            .enumerate()
            .map(|(i, &[o, h, l, c])| Bar::new(start + Duration::hours(i as i64), o, h, l, c, 1.0))
            .collect()
    }

    /// A gap up is found from a click a bar away, then tested, half-filled, filled and inverted
    /// on the bars that reach each of its prices.
    #[test]
    fn gap_events_follow_the_bars() {
        let bars = bars(&[
            [10.0, 11.0, 9.0, 10.5],
            [10.5, 14.0, 10.4, 13.8],
            [13.8, 16.0, 13.0, 15.0],
            [15.0, 15.5, 12.5, 13.5],
            [13.5, 13.6, 11.9, 12.0],
            [12.0, 12.2, 10.8, 11.5],
            [11.5, 11.6, 10.0, 10.2],
        ]);
        let s = gap_state(&bars, 2).unwrap();
        assert_eq!((s.index, s.bull, s.top, s.bottom), (1, true, 13.0, 11.0));
        assert_eq!(
            (s.tested, s.half, s.filled, s.inverted),
            (Some(3), Some(4), Some(5), Some(6))
        );
        let config = PriceActionConfig {
            flip: true,
            ..PriceActionConfig::default()
        };
        let zone = config
            .zone(DrawingToolType::FairValueGap, &bars, 1)
            .unwrap();
        assert_eq!(
            (zone.start, zone.end, zone.flip),
            (0, Some(5), Some((6, None)))
        );
    }

    /// A down-close block is tested only after price leaves it, broken by a close below it, and
    /// retested from below once price has left it again.
    #[test]
    fn block_events_follow_the_bars() {
        let bars = bars(&[
            [12.0, 12.5, 9.5, 10.0],
            [12.2, 14.0, 12.1, 13.5],
            [13.5, 14.0, 11.5, 12.0],
            [12.0, 12.2, 9.0, 9.2],
            [9.2, 9.5, 8.0, 8.5],
            [8.5, 10.5, 8.4, 10.2],
        ]);
        let s = block_state(&bars, 0, false).unwrap();
        assert!(s.bull);
        assert_eq!(
            (s.tested, s.half, s.broken, s.retest),
            (Some(2), Some(3), Some(3), Some(5))
        );
    }

    /// A buy-side level is swept by a wick through it and broken by a close above it.
    #[test]
    fn liquidity_is_swept_then_broken() {
        let bars = bars(&[
            [10.0, 12.0, 9.0, 11.0],
            [11.0, 12.5, 10.5, 11.5],
            [11.5, 13.0, 11.0, 12.8],
        ]);
        let s = liquidity_state(&bars, 0, true).unwrap();
        assert_eq!((s.price, s.swept, s.broken), (12.0, Some(1), Some(2)));
    }
}
