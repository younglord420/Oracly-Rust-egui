//! Price-axis tick motion: freeze nice prices while the range moves,
//! remap their Y each frame (slide), and crossfade when the nice step changes.

use super::{PriceFormatter, PriceMap, PriceMark, PriceScaleMode};

#[inline]
fn ease_toward(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    if !dt.is_finite() || dt <= 0.0 || !tau.is_finite() || tau <= 0.0 {
        return target;
    }
    let alpha = 1.0 - (-dt / tau).exp();
    current + (target - current) * alpha.clamp(0.0, 1.0)
}

/// One frozen tick: axis value + label, Y remapped from the live [`PriceMap`].
#[derive(Debug, Clone)]
struct FrozenTick {
    /// Same units as [`PriceMark::price`] for the scale mode that produced it.
    value: f64,
    label: String,
    weight: u8,
}

/// Price-axis tick camera: slides labels with the range, fades on step change.
///
/// Ticks of an unchanged step are nice multiples of it, so the set is refreshed every frame:
/// ticks keep their prices while the range moves and new ones appear as soon as their price
/// comes into view. A change of step crossfades the two sets, and reverses when the step returns.
#[derive(Debug, Clone, Default)]
pub(crate) struct PriceTickCamera {
    frozen: Vec<FrozenTick>,
    incoming: Option<Vec<FrozenTick>>,
    /// 0 = only frozen, 1 = only incoming.
    fade: f32,
    /// Nice step of `frozen` (median adjacent gap), for step-change detection.
    step_key: Option<f64>,
}

/// Whether two tick steps are the same nice step.
fn same_step(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) if a > 0.0 && b > 0.0 => (a / b).max(b / a) < 1.001,
        (None, None) => true,
        _ => false,
    }
}

impl PriceTickCamera {
    /// True while a step crossfade is still in flight (needs another frame).
    pub(crate) fn settling(&self) -> bool {
        self.incoming.is_some()
    }

    /// Resolve marks for this frame: remapped frozen (+ optional fade-in set).
    ///
    /// `fade_tau` is the exponential time-constant for step crossfade (seconds).
    /// Pass `0` to snap the new step instantly.
    pub(crate) fn resolve(
        &mut self,
        map: &PriceMap,
        formatter: Option<&dyn PriceFormatter>,
        dt: f32,
        fade_tau: f32,
    ) -> Vec<(PriceMark, f32)> {
        let fresh = labeled_marks(map, formatter);
        let (top, bottom) = map.extent();
        let height = bottom - top;

        if fresh.is_empty() {
            self.frozen.clear();
            self.incoming = None;
            self.fade = 0.0;
            self.step_key = None;
            return Vec::new();
        }

        if self.frozen.is_empty() {
            self.frozen = freeze(&fresh);
            self.step_key = step_key(&self.frozen);
            self.incoming = None;
            self.fade = 0.0;
            return project(&self.frozen, map, 1.0);
        }

        let fresh_step = step_key_from_marks(&fresh);
        let target = if same_step(self.step_key, fresh_step) {
            self.frozen = freeze(&fresh);
            0.0
        } else if self
            .incoming
            .as_ref()
            .is_some_and(|ticks| same_step(step_key(ticks), fresh_step))
        {
            self.incoming = Some(freeze(&fresh));
            1.0
        } else {
            // A third step: the set showing more stays as the one fading out.
            if self.fade > 0.5
                && let Some(incoming) = self.incoming.take()
            {
                self.frozen = incoming;
                self.step_key = step_key(&self.frozen);
                self.fade = 1.0 - self.fade;
            }
            self.incoming = Some(freeze(&fresh));
            1.0
        };

        if self.incoming.is_some() {
            self.fade = if fade_tau > 0.0 {
                ease_toward(self.fade, target, dt, fade_tau)
            } else {
                target
            };
        }

        if self.fade >= 0.99 {
            if let Some(incoming) = self.incoming.take() {
                self.frozen = incoming;
                self.step_key = step_key(&self.frozen);
            }
            self.fade = 0.0;
        } else if self.fade <= 0.01 && target == 0.0 {
            self.incoming = None;
            self.fade = 0.0;
        }

        let mut out = project(&self.frozen, map, 1.0 - self.fade.max(0.0));
        if let Some(incoming) = &self.incoming {
            out.extend(project(incoming, map, self.fade.clamp(0.0, 1.0)));
        }
        // Drop ticks whose opacity is negligible or that sit far outside the pane.
        let pad = height.abs().max(1.0) * 0.15;
        out.retain(|(mark, alpha)| {
            *alpha > 0.02 && mark.y_coord >= top - pad && mark.y_coord <= bottom + pad
        });
        out
    }
}

fn labeled_marks(map: &PriceMap, formatter: Option<&dyn PriceFormatter>) -> Vec<PriceMark> {
    let mut marks = map.marks();
    if let Some(formatter) = formatter {
        for mark in &mut marks {
            mark.label = formatter.format_tick(mark.price);
        }
    }
    marks
}

fn freeze(marks: &[PriceMark]) -> Vec<FrozenTick> {
    marks
        .iter()
        .map(|mark| FrozenTick {
            value: mark.price,
            label: mark.label.clone(),
            weight: mark.weight,
        })
        .collect()
}

fn project(ticks: &[FrozenTick], map: &PriceMap, opacity: f32) -> Vec<(PriceMark, f32)> {
    if opacity <= 0.0 {
        return Vec::new();
    }
    ticks
        .iter()
        .map(|tick| {
            (
                PriceMark {
                    price: tick.value,
                    label: tick.label.clone(),
                    y_coord: mark_y(map, tick.value),
                    weight: tick.weight,
                },
                opacity,
            )
        })
        .collect()
}

fn mark_y(map: &PriceMap, value: f64) -> f32 {
    match map.transform().mode {
        PriceScaleMode::Logarithmic => map.price_to_y(value),
        _ => map.scale_to_y(value),
    }
}

fn step_key(ticks: &[FrozenTick]) -> Option<f64> {
    median_step(ticks.iter().map(|t| t.value))
}

fn step_key_from_marks(marks: &[PriceMark]) -> Option<f64> {
    median_step(marks.iter().map(|m| m.price))
}

fn median_step(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut vals: Vec<f64> = values.filter(|v| v.is_finite()).collect();
    if vals.len() < 2 {
        return None;
    }
    vals.sort_by(|a, b| a.total_cmp(b));
    let mut gaps: Vec<f64> = vals
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .filter(|g| *g > 0.0 && g.is_finite())
        .collect();
    if gaps.is_empty() {
        return None;
    }
    gaps.sort_by(|a, b| a.total_cmp(b));
    Some(gaps[gaps.len() / 2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scales::{PriceMap, PriceScaleMode, PriceTransform};

    fn map(min: f64, max: f64, height: f32) -> PriceMap {
        PriceMap::new(
            PriceTransform {
                mode: PriceScaleMode::Normal,
                base: 0.0,
            },
            crate::scales::PriceRange::new(min, max),
            0.0,
            height,
            false,
        )
    }

    #[test]
    fn sliding_keeps_tick_prices_and_fills_the_range_while_it_moves() {
        let mut cam = PriceTickCamera::default();
        let first = cam.resolve(&map(100.0, 200.0, 400.0), None, 1.0 / 60.0, 0.08);
        assert!(!first.is_empty());

        let mut range = (100.0, 200.0);
        let mut last = first.clone();
        for _ in 0..30 {
            range = (range.0 + 5.0, range.1 + 5.0);
            let next = cam.resolve(&map(range.0, range.1, 400.0), None, 1.0 / 60.0, 0.08);
            // A step that does not change never fades: every tick is fully shown.
            assert!(next.iter().all(|(_, alpha)| *alpha == 1.0), "{next:?}");
            // Ticks still in view keep their price and slide with the range.
            for (mark, _) in &next {
                if let Some((before, _)) = last.iter().find(|(m, _)| m.price == mark.price) {
                    assert!(
                        mark.y_coord > before.y_coord,
                        "{} must slide down",
                        mark.price
                    );
                }
            }
            last = next;
        }
        // The prices scrolled into view carry ticks without waiting for the motion to stop.
        assert!(last.iter().any(|(mark, _)| mark.price > 300.0), "{last:?}");
        assert!(last.iter().all(|(mark, _)| mark.price >= range.0 - 50.0));
    }

    #[test]
    fn a_step_change_that_returns_fades_back_out() {
        let mut cam = PriceTickCamera::default();
        let _ = cam.resolve(&map(100.0, 200.0, 400.0), None, 1.0 / 60.0, 0.05);
        let _ = cam.resolve(&map(100.0, 200.0, 1200.0), None, 1.0 / 60.0, 0.05);
        assert!(cam.incoming.is_some());
        for _ in 0..40 {
            let _ = cam.resolve(&map(100.0, 200.0, 400.0), None, 0.05, 0.05);
        }
        assert!(cam.incoming.is_none());
        let settled = cam.resolve(&map(100.0, 200.0, 400.0), None, 1.0 / 60.0, 0.05);
        assert_eq!(
            settled.iter().map(|(m, _)| m.price).collect::<Vec<_>>(),
            map(100.0, 200.0, 400.0)
                .marks()
                .iter()
                .map(|m| m.price)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn step_change_crossfades_then_settles() {
        let mut cam = PriceTickCamera::default();
        let _ = cam.resolve(&map(100.0, 200.0, 400.0), None, 1.0 / 60.0, 0.05);
        // Much taller pane → denser marks → different nice step.
        let mid = cam.resolve(&map(100.0, 200.0, 1200.0), None, 1.0 / 60.0, 0.05);
        let fading = mid.iter().any(|(_, a)| *a < 0.99) || cam.incoming.is_some();
        assert!(fading || cam.fade > 0.0 || cam.incoming.is_some());

        for _ in 0..40 {
            let _ = cam.resolve(&map(100.0, 200.0, 1200.0), None, 0.05, 0.05);
        }
        assert!(cam.incoming.is_none());
        let settled = cam.resolve(&map(100.0, 200.0, 1200.0), None, 1.0 / 60.0, 0.05);
        assert!(settled.iter().all(|(_, a)| *a > 0.99));
    }
}
