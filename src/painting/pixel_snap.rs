//! Bars and hairlines placed on whole physical pixels.
//!
//! Anti-aliased edges at fractional positions make a bar or a one-pixel line change its apparent
//! width and sharpness every frame while the chart zooms or scrolls. Snapping their edges and
//! centres to the physical pixel grid keeps every bar the same width and every hairline crisp.

use egui::Painter;

/// Rounds chart geometry to the physical pixel grid of a painter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PixelSnap {
    pixels_per_point: f32,
}

impl PixelSnap {
    /// Snaps to the pixels of `pixels_per_point`; a non-finite or non-positive scale is taken as 1.
    pub(crate) fn new(pixels_per_point: f32) -> Self {
        Self {
            pixels_per_point: if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
                pixels_per_point
            } else {
                1.0
            },
        }
    }

    /// Snaps to the pixels `painter` paints on.
    pub(crate) fn of(painter: &Painter) -> Self {
        Self::new(painter.pixels_per_point())
    }

    /// `v` on the nearest pixel edge.
    pub(crate) fn edge(self, v: f32) -> f32 {
        if !v.is_finite() {
            return v;
        }
        (v * self.pixels_per_point).round() / self.pixels_per_point
    }

    /// The centre of the pixel holding `v`.
    pub(crate) fn center(self, v: f32) -> f32 {
        if !v.is_finite() {
            return v;
        }
        ((v * self.pixels_per_point).floor() + 0.5) / self.pixels_per_point
    }

    /// `width` points in whole pixels, at least one.
    pub(crate) fn line_width(self, width: f32) -> f32 {
        self.pixels(width) / self.pixels_per_point
    }

    /// Where a line of `width` points through `v` is centred so that both of its sides fall on
    /// pixel edges: a pixel centre for an odd number of pixels, a pixel edge for an even one.
    pub(crate) fn line_center(self, v: f32, width: f32) -> f32 {
        if self.pixels(width) % 2.0 == 1.0 {
            self.center(v)
        } else {
            self.edge(v)
        }
    }

    /// The left and right edges of a column about `width` points wide centred on `x`, a whole
    /// number of pixels wide and at least one.
    pub(crate) fn span(self, x: f32, width: f32) -> (f32, f32) {
        self.span_of(x, self.pixels(width))
    }

    /// The left and right edges of a bar about `width` points wide centred on `x`.
    ///
    /// The width is a whole number of pixels, at least one, and odd from three pixels on, so a
    /// one-pixel wick at [`center`](Self::center)`(x)` splits it evenly.
    pub(crate) fn bar_span(self, x: f32, width: f32) -> (f32, f32) {
        let mut pixels = self.pixels(width);
        if pixels >= 3.0 && pixels % 2.0 == 0.0 {
            pixels -= 1.0;
        }
        self.span_of(x, pixels)
    }

    fn span_of(self, x: f32, pixels: f32) -> (f32, f32) {
        let center = x * self.pixels_per_point;
        let left = if pixels % 2.0 == 1.0 {
            center.floor() - (pixels - 1.0) / 2.0
        } else {
            center.round() - pixels / 2.0
        };
        (
            left / self.pixels_per_point,
            (left + pixels) / self.pixels_per_point,
        )
    }

    /// The top and bottom of a span from `top` to `bottom`, on pixel edges and at least
    /// `min_height` points (and one pixel) tall.
    pub(crate) fn vertical_span(self, top: f32, bottom: f32, min_height: f32) -> (f32, f32) {
        let top = self.edge(top);
        let bottom = self
            .edge(bottom)
            .max(top + self.line_width(min_height.max(0.0)));
        (top, bottom)
    }

    fn pixels(self, width: f32) -> f32 {
        if width.is_finite() {
            (width * self.pixels_per_point).round().max(1.0)
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_keep_one_odd_width_wherever_they_sit() {
        let snap = PixelSnap::new(1.0);
        for x in [10.0, 10.2, 10.5, 10.8, 11.49] {
            for (width, pixels) in [(0.3, 1.0), (2.4, 2.0), (6.0, 5.0), (6.6, 7.0), (7.4, 7.0)] {
                let (left, right) = snap.bar_span(x, width);
                assert_eq!(right - left, pixels, "x={x}, width={width}");
                assert_eq!(left.fract(), 0.0, "x={x}, width={width}");
                if pixels % 2.0 == 1.0 {
                    assert_eq!((left + right) / 2.0, snap.center(x), "x={x}, width={width}");
                }
            }
        }
    }

    #[test]
    fn columns_keep_their_rounded_width_even_or_odd() {
        let snap = PixelSnap::new(1.0);
        assert_eq!(snap.span(10.0, 4.0), (8.0, 12.0));
        assert_eq!(snap.span(10.3, 4.4), (8.0, 12.0));
        assert_eq!(snap.span(10.3, 5.0), (8.0, 13.0));
        assert_eq!(PixelSnap::new(2.0).span(10.3, 2.0), (9.5, 11.5));
    }

    #[test]
    fn hairlines_sit_on_pixel_centres_at_any_scale() {
        for (ppp, v, center, edge) in [(1.0, 10.3, 10.5, 10.0), (2.0, 10.3, 10.25, 10.5)] {
            let snap = PixelSnap::new(ppp);
            assert_eq!(snap.center(v), center);
            assert_eq!(snap.edge(v), edge);
            assert_eq!(snap.line_center(v, 1.0 / ppp), center);
            assert_eq!(snap.line_center(v, 2.0 / ppp), edge);
        }
        let snap = PixelSnap::new(f32::NAN);
        assert_eq!(snap.line_width(1.0), 1.0);
        assert!(snap.center(f32::INFINITY).is_infinite());
    }

    #[test]
    fn vertical_spans_round_to_edges_and_keep_their_minimum() {
        let snap = PixelSnap::new(1.0);
        assert_eq!(snap.vertical_span(10.4, 20.6, 1.0), (10.0, 21.0));
        assert_eq!(snap.vertical_span(10.4, 10.45, 1.0), (10.0, 11.0));
        assert_eq!(snap.vertical_span(10.4, 10.45, 0.0), (10.0, 11.0));
        assert_eq!(snap.vertical_span(10.4, 10.45, 3.0), (10.0, 13.0));
    }
}
