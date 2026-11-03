//! Saved options and plane geometry of the geometry tools: the Dedekind tessellation of the
//! upper half-plane, and the Mach wavefront figures (Sonic, Supersonic and their golden
//! variants) built from Huygens circles along a flight path.

use super::GannLevel;
use crate::config::DrawingToolType;
use egui::{Pos2, Vec2};
use serde::{Deserialize, Serialize};

/// Max curvature presets of a Dedekind tessellation.
pub const DEDEKIND_CURVATURES: [u32; 8] = [4, 8, 12, 16, 24, 32, 48, 64];

/// Circles a new Sonic or Supersonic figure shows of its twelve.
const DEFAULT_WAVES: usize = 6;

/// Radius steps of the golden Mach figures, as multiples of the first circle.
const GOLDEN_RATIOS: [f32; 11] = [
    0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.618, 2.618, 4.236, 6.854, 11.09,
];

/// Colours of the linear figures' circles, in order.
const WAVE_COLORS: [[u8; 4]; 12] = [
    [0, 188, 212, 255],
    [41, 98, 255, 255],
    [8, 153, 129, 255],
    [76, 175, 80, 255],
    [255, 152, 0, 255],
    [242, 54, 69, 255],
    [233, 30, 99, 255],
    [156, 39, 176, 255],
    [120, 123, 134, 255],
    [0, 151, 167, 255],
    [103, 58, 183, 255],
    [255, 112, 67, 255],
];

/// Options of a geometry tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeometryConfig {
    /// Circles of a Mach figure, by radius over the drawn first circle's.
    pub levels: Vec<GannLevel>,
    /// Mach number of a supersonic figure; above 1, the circles form a cone.
    pub mach: f32,
    /// Whether each circle's ratio is written beside it.
    pub show_ratios: bool,
    /// Largest circle curvature of a Dedekind tessellation; higher is denser.
    pub max_curvature: u32,
}

impl Default for GeometryConfig {
    fn default() -> Self {
        Self::for_tool(DrawingToolType::Sonic)
    }
}

impl GeometryConfig {
    /// The options a new drawing of `tool` starts with.
    pub fn for_tool(tool: DrawingToolType) -> Self {
        let levels = if matches!(
            tool,
            DrawingToolType::GoldenSonic | DrawingToolType::GoldenSupersonic
        ) {
            GOLDEN_RATIOS
                .iter()
                .enumerate()
                .map(|(i, &ratio)| GannLevel::new(ratio, super::fib_ratio_color(ratio, i)))
                .collect()
        } else {
            WAVE_COLORS
                .iter()
                .enumerate()
                .map(|(i, &color)| GannLevel {
                    enabled: i < DEFAULT_WAVES,
                    ..GannLevel::new((i + 1) as f32, color)
                })
                .collect()
        };
        Self {
            levels,
            mach: 2.0,
            show_ratios: true,
            max_curvature: 24,
        }
    }

    /// The Mach number `tool` draws with: 1 for the sonic figures.
    pub fn mach_number(&self, tool: DrawingToolType) -> f32 {
        match tool {
            DrawingToolType::Supersonic | DrawingToolType::GoldenSupersonic => {
                self.mach.clamp(1.01, 20.0)
            }
            _ => 1.0,
        }
    }

    /// How many of the linear figure's circles are on, counted from the first.
    pub fn waves(&self) -> usize {
        self.levels.iter().take_while(|l| l.enabled).count()
    }

    /// Turns on the first `count` circles and off the rest.
    pub fn set_waves(&mut self, count: usize) {
        for (i, level) in self.levels.iter_mut().enumerate() {
            level.enabled = i < count;
        }
    }
}

/// One circle of a Mach figure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MachCircle {
    /// Centre.
    pub center: Pos2,
    /// Radius.
    pub radius: f32,
    /// Stroke colour as RGBA.
    pub color: [u8; 4],
    /// Radius over the first circle's.
    pub ratio: f32,
}

/// A Mach figure laid out on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct MachFigure {
    /// Unit direction the circles grow in: from the first anchor to the second.
    pub direction: Vec2,
    /// The shock origin, on the first anchor's side of the first circle.
    pub nose: Pos2,
    /// Circles, smallest first.
    pub circles: Vec<MachCircle>,
    /// The envelope: one shock wall across the path when sonic, the two cone edges otherwise.
    pub rays: Vec<[Pos2; 2]>,
}

/// The Mach figure whose first circle has `a` and `b` as a diameter, with the circles of the
/// enabled `levels` spaced for Mach number `mach`. `None` when the diameter is under a pixel or
/// no circle is on.
pub fn mach_figure(a: Pos2, b: Pos2, levels: &[GannLevel], mach: f32) -> Option<MachFigure> {
    let center = a.lerp(b, 0.5);
    let radius = a.distance(b) / 2.0;
    if radius < 1.0 {
        return None;
    }
    let direction = (b - a).normalized();
    let mut on: Vec<_> = levels
        .iter()
        .filter(|l| l.enabled && l.ratio > 0.0 && l.ratio.is_finite())
        .collect();
    on.sort_by(|p, q| p.ratio.total_cmp(&q.ratio));
    let largest = on.last()?.ratio * radius;
    let mach = mach.max(1.0);
    let circles = on
        .iter()
        .map(|level| {
            let r = level.ratio * radius;
            MachCircle {
                center: center + direction * (mach * (r - radius)),
                radius: r,
                color: level.color,
                ratio: level.ratio,
            }
        })
        .collect();
    let nose = center - direction * (mach * radius);
    let reach = mach * largest + 2.0 * radius;
    let rays = if mach <= 1.0 + 1e-6 {
        let across = direction.rot90();
        vec![[nose - across * reach, nose + across * reach]]
    } else {
        let mu = (1.0 / mach).asin();
        [mu, -mu]
            .map(|angle| {
                [
                    nose,
                    nose + egui::emath::Rot2::from_angle(angle) * direction * reach,
                ]
            })
            .to_vec()
    };
    Some(MachFigure {
        direction,
        nose,
        circles,
        rays,
    })
}

/// Whether `k / n` is the centre of a Dedekind circle of curvature `n` (radius `1 / n`), by
/// Kocik's algebraic characterization (arXiv:1912.05768).
pub fn is_dedekind_center(k: i64, n: i64) -> bool {
    if n <= 0 {
        return false;
    }
    let k2m1 = k * k - 1;
    if n % 2 == 1 {
        k2m1 % n == 0
    } else if n % 8 == 0 {
        k2m1 % n == 0 && (k2m1 / n) % 2 != 0
    } else {
        false
    }
}

/// A geodesic of the Dedekind tessellation in a box, in screen points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DedekindGeodesic {
    /// A semicircle standing on the box's bottom edge.
    Arc {
        /// Centre, on the bottom edge.
        center: Pos2,
        /// Radius.
        radius: f32,
    },
    /// A vertical line from the bottom edge to the top.
    Line {
        /// Horizontal position.
        x: f32,
    },
}

/// The Dedekind tessellation in `rect`: its bottom edge is the real axis and the unit semicircle
/// reaches its top. Arcs use curvatures 1 to `max_curvature`; arcs under `min_radius` are culled.
pub fn dedekind_geodesics(
    rect: egui::Rect,
    max_curvature: u32,
    min_radius: f32,
) -> Vec<DedekindGeodesic> {
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return Vec::new();
    }
    let unit = rect.height();
    let span = f64::from(rect.width() / unit);
    let max_n = i64::from(max_curvature.clamp(1, 64));
    let pad = 1.0 / max_n as f64;
    let (lo, hi) = (-pad, span + pad);
    let x_of = |real: f64| rect.left() + (real as f32) * unit;
    let mut out = Vec::new();
    for k in ((lo * 2.0).floor() as i64)..=((hi * 2.0).ceil() as i64) {
        if k % 2 != 0 {
            let x = x_of(k as f64 / 2.0);
            if (rect.left() - 0.5..=rect.right() + 0.5).contains(&x) {
                out.push(DedekindGeodesic::Line { x });
            }
        }
    }
    for n in 1..=max_n {
        let r = 1.0 / n as f64;
        let radius = (r as f32) * unit;
        if radius < min_radius {
            continue;
        }
        for k in (0..n).filter(|&k| is_dedekind_center(k, n)) {
            for t in (lo.floor() as i64 - 1)..=(hi.ceil() as i64 + 1) {
                let c = k as f64 / n as f64 + t as f64;
                if c + r < lo || c - r > hi {
                    continue;
                }
                out.push(DedekindGeodesic::Arc {
                    center: Pos2::new(x_of(c), rect.bottom()),
                    radius,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kocik's centres for small curvatures: 0/1, then 1/3 and 2/3, then 3/8 and 5/8, whose
    /// (k² - 1) / 8 is odd.
    #[test]
    fn dedekind_centres_follow_kocik() {
        let centres = |n| {
            (0..n)
                .filter(|&k| is_dedekind_center(k, n))
                .collect::<Vec<_>>()
        };
        assert_eq!(centres(1), [0]);
        assert!(centres(2).is_empty() && centres(4).is_empty());
        assert_eq!(centres(3), [1, 2]);
        assert_eq!(centres(8), [3, 5]);
    }

    /// The first circle has the drawn diameter; a sonic figure's circles share its nose and
    /// meet one wall, a supersonic one's sit inside a cone of half-angle asin(1/M).
    #[test]
    fn mach_figures_grow_from_the_drawn_circle() {
        let levels: Vec<_> = (1..=3)
            .map(|r| GannLevel::new(r as f32, [1, 2, 3, 255]))
            .collect();
        let (a, b) = (Pos2::new(0.0, 0.0), Pos2::new(20.0, 0.0));
        let sonic = mach_figure(a, b, &levels, 1.0).unwrap();
        assert_eq!(sonic.circles[0].center, Pos2::new(10.0, 0.0));
        assert_eq!(sonic.circles[0].radius, 10.0);
        assert_eq!(sonic.nose, Pos2::new(0.0, 0.0));
        for c in &sonic.circles {
            assert!((c.center.x - c.radius - sonic.nose.x).abs() < 1e-4);
        }
        assert_eq!(sonic.rays.len(), 1);
        let cone = mach_figure(a, b, &levels, 2.0).unwrap();
        assert_eq!(cone.rays.len(), 2);
        let edge = cone.rays[0][1] - cone.rays[0][0];
        assert!((edge.angle().abs() - 0.5_f32.asin()).abs() < 1e-4);
        assert!(mach_figure(a, a, &levels, 1.0).is_none());
    }

    /// Waves turn on the first circles of a linear figure; golden figures start with all theirs.
    #[test]
    fn waves_count_the_first_circles() {
        let mut config = GeometryConfig::for_tool(DrawingToolType::Sonic);
        assert_eq!((config.levels.len(), config.waves()), (12, 6));
        config.set_waves(9);
        assert_eq!(config.waves(), 9);
        let golden = GeometryConfig::for_tool(DrawingToolType::GoldenSonic);
        assert!(golden.levels.iter().all(|l| l.enabled));
        assert_eq!(golden.mach_number(DrawingToolType::GoldenSonic), 1.0);
        assert_eq!(golden.mach_number(DrawingToolType::GoldenSupersonic), 2.0);
    }
}
