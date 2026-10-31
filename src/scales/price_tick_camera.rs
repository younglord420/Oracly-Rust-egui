//! Vela-like price-axis tick motion: freeze nice prices while the range moves,
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
#[derive(Debug, Clone, Default)]
pub(crate) struct PriceTickCamera {
    frozen: Vec<FrozenTick>,
    incoming: Option<Vec<FrozenTick>>,
    /// 0 = only frozen, 1 = only incoming.
    fade: f32,
    /// Nice step of `frozen` (median adjacent gap), for step-change detection.
    step_key: Option<f64>,
    last_range: Option<(f64, f64)>,
    last_height: f32,
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
        let range = map.range();
        let (top, bottom) = map.extent();
        let height = bottom - top;

        let range_moved = match self.last_range {
            Some((min, max)) => {
                (min - range.min).abs() + (max - range.max).abs() > range.length() * 1e-6 + 1e-12
            }
            None => false,
        };
        let height_moved = self.last_height > 0.0 && (self.last_height - height).abs() > 0.5;
        let in_motion = range_moved || height_moved || self.incoming.is_some();

        if fresh.is_empty() {
            self.frozen.clear();
            self.incoming = None;
            self.fade = 0.0;
            self.step_key = None;
            self.last_range = Some((range.min, range.max));
            self.last_height = height;
            return Vec::new();
        }

        if self.frozen.is_empty() {
            self.frozen = freeze(&fresh);
            self.step_key = step_key(&self.frozen);
            self.incoming = None;
            self.fade = 0.0;
            self.last_range = Some((range.min, range.max));
            self.last_height = height;
            return project(&self.frozen, map, 1.0);
        }

        let fresh_step = step_key_from_marks(&fresh);
        let step_changed = match (self.step_key, fresh_step) {
            (Some(old), Some(new)) if old > 0.0 && new > 0.0 => {
                let ratio = (new / old).max(old / new);
                ratio > 1.35
            }
            (None, Some(_)) => true,
            _ => false,
        };

        if step_changed {
            let next = freeze(&fresh);
            let same_incoming = self
                .incoming
                .as_ref()
                .and_then(|ticks| step_key(ticks))
                .zip(fresh_step)
                .is_some_and(|(a, b)| a > 0.0 && b > 0.0 && (a / b).max(b / a) < 1.05);
            if !same_incoming {
                self.incoming = Some(next);
                self.fade = 0.0;
            }
        } else if !in_motion && self.incoming.is_none() {
            // Quiet refresh: same nice family, update labels/weights from fresh.
            self.frozen = freeze(&fresh);
            self.step_key = step_key(&self.frozen);
        }

        if let Some(tau) = (fade_tau > 0.0).then_some(fade_tau) {
            if self.incoming.is_some() {
                self.fade = ease_toward(self.fade, 1.0, dt, tau);
            }
        } else if self.incoming.is_some() {
            self.fade = 1.0;
        }

        if self.fade >= 0.99 {
            if let Some(incoming) = self.incoming.take() {
                self.frozen = incoming;
                self.step_key = step_key(&self.frozen);
            }
            self.fade = 0.0;
        }

        self.last_range = Some((range.min, range.max));
        self.last_height = height;

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
    fn sliding_keeps_tick_prices_while_range_moves() {
        let mut cam = PriceTickCamera::default();
        let first = cam.resolve(&map(100.0, 200.0, 400.0), None, 1.0 / 60.0, 0.08);
        assert!(!first.is_empty());
        let prices: Vec<f64> = first.iter().map(|(m, _)| m.price).collect();

        let second = cam.resolve(&map(110.0, 210.0, 400.0), None, 1.0 / 60.0, 0.08);
        let prices2: Vec<f64> = second.iter().map(|(m, _)| m.price).collect();
        assert_eq!(
            prices, prices2,
            "same nice family must slide, not regenerate"
        );
        // At least one tick should have moved in Y.
        let moved = first
            .iter()
            .zip(&second)
            .any(|((a, _), (b, _))| a.price == b.price && (a.y_coord - b.y_coord).abs() > 0.5);
        assert!(moved, "remapped Y should slide with the range");
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
