//! Chart motion easing (eased zoom / pan / crosshair glide).
//!
//! Durations are exponential time-constants in milliseconds: after one constant the
//! motion has covered about 63% of the remaining distance, and looks settled after
//! roughly three. Cap is 1000 ms, matching common charting UIs.

/// Eased chart motions for zoom, scroll, auto price-scale and crosshair.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ChartAnimations {
    /// Wheel / pinch zoom ease time-constant in milliseconds.
    ///
    /// `None` or `Some(0)` applies zoom instantly. Default `Some(70)`.
    pub zoom_ms: Option<u32>,

    /// Auto price-scale ease time-constant in milliseconds.
    ///
    /// `None` or `Some(0)` snaps the range every frame. Default `Some(80)`
    /// (glides while zoom/fling is in flight).
    pub autoscale_ms: Option<u32>,

    /// Go-to-realtime / near-edge snap ease time-constant in milliseconds.
    ///
    /// `None` or `Some(0)` jumps instantly when animated scroll is requested.
    /// Default `Some(130)`.
    pub scroll_ms: Option<u32>,

    /// Price-axis wheel zoom ease time-constant in milliseconds.
    ///
    /// `None` or `Some(0)` applies instantly. Drag-zoom on the axis stays instant.
    /// Default `Some(70)` (aligned with wheel zoom).
    pub price_zoom_ms: Option<u32>,

    /// Crosshair follow ease time-constant in milliseconds.
    ///
    /// `None` or `Some(0)` tracks the pointer/snap target instantly. Default `Some(45)`.
    pub crosshair_ms: Option<u32>,
}

impl Default for ChartAnimations {
    fn default() -> Self {
        Self {
            zoom_ms: Some(70),
            autoscale_ms: Some(80),
            scroll_ms: Some(130),
            price_zoom_ms: Some(70),
            crosshair_ms: Some(45),
        }
    }
}

impl ChartAnimations {
    /// Zoom time-constant in seconds, or `None` for instant zoom.
    pub(crate) fn zoom_tau_secs(self) -> Option<f32> {
        tau_secs(self.zoom_ms)
    }

    /// Autoscale time-constant in seconds, or `None` for instant fit.
    pub(crate) fn autoscale_tau_secs(self) -> Option<f32> {
        tau_secs(self.autoscale_ms)
    }

    /// Go-live scroll time-constant in seconds, or `None` for an instant jump.
    pub(crate) fn scroll_tau_secs(self) -> Option<f32> {
        tau_secs(self.scroll_ms)
    }

    /// Price-axis wheel zoom time-constant in seconds, or `None` for instant zoom.
    pub(crate) fn price_zoom_tau_secs(self) -> Option<f32> {
        tau_secs(self.price_zoom_ms)
    }

    /// Crosshair follow time-constant in seconds, or `None` for instant tracking.
    pub(crate) fn crosshair_tau_secs(self) -> Option<f32> {
        tau_secs(self.crosshair_ms)
    }
}

fn tau_secs(ms: Option<u32>) -> Option<f32> {
    let ms = ms.filter(|v| *v > 0)?.min(1_000);
    Some(ms as f32 / 1_000.0)
}

/// Exponential step toward `target`: after one `tau`, ~63% of the remaining gap closes.
#[inline]
pub(crate) fn ease_step(current: f64, target: f64, dt: f32, tau: f32) -> f64 {
    if !dt.is_finite() || dt <= 0.0 || !tau.is_finite() || tau <= 0.0 {
        return target;
    }
    let alpha = 1.0 - (-dt / tau).exp();
    current + (target - current) * f64::from(alpha.clamp(0.0, 1.0))
}

/// Same as [`ease_step`] for `f32` positions (crosshair).
#[inline]
pub(crate) fn ease_step_f32(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    ease_step(f64::from(current), f64::from(target), dt, tau) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_documented_constants() {
        let a = ChartAnimations::default();
        assert_eq!(a.zoom_ms, Some(70));
        assert_eq!(a.autoscale_ms, Some(80));
        assert_eq!(a.scroll_ms, Some(130));
        assert_eq!(a.price_zoom_ms, Some(70));
        assert_eq!(a.crosshair_ms, Some(45));
        assert!((a.zoom_tau_secs().unwrap() - 0.07).abs() < 1e-6);
        assert!((a.scroll_tau_secs().unwrap() - 0.13).abs() < 1e-6);
    }

    #[test]
    fn zero_or_none_is_instant() {
        assert!(
            ChartAnimations {
                zoom_ms: None,
                ..Default::default()
            }
            .zoom_tau_secs()
            .is_none()
        );
        assert!(
            ChartAnimations {
                zoom_ms: Some(0),
                ..Default::default()
            }
            .zoom_tau_secs()
            .is_none()
        );
        assert!(
            ChartAnimations {
                scroll_ms: Some(0),
                ..Default::default()
            }
            .scroll_tau_secs()
            .is_none()
        );
    }

    #[test]
    fn ease_step_moves_most_of_the_way_in_one_tau() {
        let next = ease_step(0.0, 10.0, 0.07, 0.07);
        assert!((next - 6.3).abs() < 0.2, "got {next}");
    }
}
