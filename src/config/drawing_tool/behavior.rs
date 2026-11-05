//! Behavioral queries for [`DrawingToolType`].
//!
//! Provides methods that describe *how* each tool type behaves: whether it is
//! a cursor, what interaction mode it uses, and how many points it requires.

use super::{DrawingInteractionMode, DrawingToolType};

impl DrawingToolType {
    /// Returns `true` if this tool is a cursor/utility tool rather than a
    /// drawing tool.
    ///
    /// Cursor tools (`CrossCursor`, `DotCursor`, `ArrowCursor`, `Eraser`)
    /// do not create `Drawing` objects. They modify
    /// the cursor appearance or provide utility functions (e.g., erasing).
    #[inline]
    pub fn is_cursor(&self) -> bool {
        matches!(
            self,
            Self::CrossCursor | Self::DotCursor | Self::ArrowCursor | Self::Eraser
        )
    }

    /// Whether the renderer paints a filled area that takes the drawing's fill colour.
    pub fn supports_fill(&self) -> bool {
        !matches!(
            self,
            Self::CrossCursor
                | Self::DotCursor
                | Self::ArrowCursor
                | Self::Eraser
                | Self::TrendLine
                | Self::TrendAngle
                | Self::InfoLine
                | Self::ElliottImpulse
                | Self::ElliottDoubleCombo
                | Self::ElliottTripleCombo
                | Self::GannFan
                | Self::FairValueGap
                | Self::OrderBlock
                | Self::Liquidity
                | Self::DedekindTessellation
                | Self::Sonic
                | Self::Supersonic
                | Self::GoldenSonic
                | Self::GoldenSupersonic
                | Self::Ray
                | Self::ExtendedLine
                | Self::HorizontalLine
                | Self::HorizontalRay
                | Self::VerticalLine
                | Self::CrossLine
                | Self::FibonacciRetracement
                | Self::FibonacciExtension
                | Self::FibonacciChannel
                | Self::FibonacciSpeedFan
                | Self::FibonacciTimeZones
                | Self::TrendBasedFibTime
                | Self::FibonacciCircles
                | Self::FibonacciSpiral
                | Self::FibonacciSpeedResistanceArcs
                | Self::FibonacciWedge
                | Self::Pitchfan
                | Self::Pitchfork
                | Self::SchiffPitchfork
                | Self::ModifiedSchiffPitchfork
                | Self::InsidePitchfork
                | Self::RegressionTrend
                | Self::GhostFeed
                | Self::BarsPattern
                | Self::Forecast
                | Self::LongPos
                | Self::ShortPos
                | Self::AnchoredVWAP
                | Self::FixedRangeVolumeProfile
                | Self::AnchoredVolumeProfile
                | Self::Brush
                | Self::Highlighter
                | Self::ArrowMarker
                | Self::Arrow
                | Self::ArrowMarkUp
                | Self::ArrowMarkDown
                | Self::ArrowMarkLeft
                | Self::ArrowMarkRight
                | Self::Path
                | Self::Polyline
                | Self::DoubleCurve
                | Self::CyclicLines
                | Self::SineLine
                | Self::AnchoredNote
                | Self::PriceLabel
                | Self::TextLabel
                | Self::AnchoredText
                | Self::FlagNote
                | Self::FontIcon
                | Self::Sticker
        )
    }

    /// Whether the drawing's left and right extension flags change its geometry.
    /// A ray always runs from its leftmost anchor through both points to the
    /// right pane edge, so those flags do not apply.
    pub fn supports_extend(&self) -> bool {
        matches!(
            self,
            Self::TrendLine
                | Self::ExtendedLine
                | Self::Rect
                | Self::InfoLine
                | Self::TrendAngle
                | Self::Arrow
                | Self::ArrowMarker
                | Self::ParallelChannel
                | Self::FlatTopBottom
                | Self::DisjointChannel
                | Self::RegressionTrend
                | Self::Pitchfork
                | Self::SchiffPitchfork
                | Self::ModifiedSchiffPitchfork
                | Self::InsidePitchfork
        )
    }

    /// Whether the renderer displays the drawing's editable text.
    ///
    /// Primary text tools always show their content. Line and shape tools also accept an optional
    /// annotation label painted near the geometry when `Drawing::text` is set.
    pub fn supports_text(&self) -> bool {
        self.is_text_annotation() || self.supports_line_label()
    }

    /// Dedicated text / note tools whose body is the drawing itself.
    pub fn is_text_annotation(&self) -> bool {
        matches!(
            self,
            Self::TextLabel
                | Self::AnchoredText
                | Self::Note
                | Self::AnchoredNote
                | Self::Callout
                | Self::Comment
                | Self::FlagNote
                | Self::Signpost
                | Self::Table
                | Self::Tweet
                | Self::Idea
        )
    }

    /// Line and shape tools that can carry an optional user label.
    pub fn supports_line_label(&self) -> bool {
        matches!(
            self,
            Self::TrendLine
                | Self::Ray
                | Self::ExtendedLine
                | Self::HorizontalLine
                | Self::HorizontalRay
                | Self::VerticalLine
                | Self::CrossLine
                | Self::InfoLine
                | Self::Arrow
                | Self::ArrowMarkUp
                | Self::ArrowMarkDown
                | Self::ArrowMarkLeft
                | Self::ArrowMarkRight
                | Self::Path
                | Self::Polyline
                | Self::Rect
                | Self::RotatedRect
                | Self::Circle
                | Self::Ellipse
                | Self::Triangle
                | Self::Arc
                | Self::Curve
                | Self::DoubleCurve
                | Self::Sector
                | Self::Brush
                | Self::ParallelChannel
                | Self::FlatTopBottom
                | Self::DisjointChannel
        )
    }

    /// Whether the drawing properties dialog should show text styling controls
    /// (text color, font size, bold, italic).
    ///
    /// Returns `true` for the non-cursor drawing tools that paint text, so users can configure
    /// text formatting even on tools whose labels are generated automatically (Fibonacci,
    /// Elliott waves, patterns, measurements, positions). Time cycles, sine lines, the price
    /// action tools and the Dedekind tessellation paint none.
    pub fn supports_text_style(&self) -> bool {
        !self.is_cursor()
            && !matches!(
                self,
                Self::TimeCycles
                    | Self::SineLine
                    | Self::Highlighter
                    | Self::GhostFeed
                    | Self::FairValueGap
                    | Self::OrderBlock
                    | Self::Liquidity
                    | Self::DedekindTessellation
            )
    }

    /// Returns the [`DrawingInteractionMode`] that governs how mouse input is
    /// captured when creating a drawing of this type.
    ///
    /// See [`DrawingInteractionMode`] for details on each mode.
    pub fn interaction_mode(&self) -> DrawingInteractionMode {
        match self {
            // Cursors - not drawing tools
            Self::CrossCursor | Self::DotCursor | Self::ArrowCursor | Self::Eraser => {
                DrawingInteractionMode::NotDrawing
            }

            // Single-click tools
            Self::HorizontalLine
            | Self::VerticalLine
            | Self::CrossLine
            | Self::HorizontalRay
            | Self::TextLabel
            | Self::AnchoredText
            | Self::Note
            | Self::AnchoredNote
            | Self::PriceLabel
            | Self::FlagNote
            | Self::ArrowMarkUp
            | Self::ArrowMarkDown
            | Self::ArrowMarkLeft
            | Self::ArrowMarkRight
            | Self::Sticker
            | Self::AnchoredVWAP
            | Self::AnchoredVolumeProfile
            | Self::Table
            | Self::FontIcon
            | Self::Tweet
            | Self::Idea
            | Self::FairValueGap
            | Self::OrderBlock
            | Self::Liquidity => DrawingInteractionMode::SingleClick,

            // Drag-to-draw tools also accept two successive clicks.
            Self::Rect
            | Self::Circle
            | Self::Ellipse
            | Self::Measure
            | Self::PriceRange
            | Self::DateRange
            | Self::DateAndPriceRange
            | Self::GannBox
            | Self::GannSquare
            | Self::GannFixed
            | Self::Forecast
            | Self::FixedRangeVolumeProfile
            | Self::FibonacciRetracement
            | Self::FibonacciSpeedResistanceArcs
            | Self::FibonacciTimeZones
            | Self::FibonacciCircles
            | Self::FibonacciSpeedFan
            | Self::FibonacciSpiral
            | Self::GannFan
            | Self::RegressionTrend
            | Self::CyclicLines
            | Self::TimeCycles
            | Self::SineLine
            | Self::Callout
            | Self::Comment
            | Self::Signpost
            | Self::Image
            | Self::Arrow
            | Self::LongPos
            | Self::ShortPos
            | Self::DedekindTessellation
            | Self::Sonic
            | Self::Supersonic
            | Self::GoldenSonic
            | Self::GoldenSupersonic => DrawingInteractionMode::DragToDraw,

            // Continuous drawing tools
            Self::Brush | Self::Highlighter => DrawingInteractionMode::ContinuousDraw,

            // Multi-point tools
            Self::RotatedRect
            | Self::Arc
            | Self::Sector
            | Self::FibonacciExtension
            | Self::FibonacciWedge
            | Self::TrendBasedFibTime
            | Self::Pitchfan
            | Self::Curve
            | Self::DoubleCurve
            | Self::ProjectionTool
            | Self::Polyline
            | Self::Path
            | Self::Triangle
            | Self::ParallelChannel
            | Self::FlatTopBottom
            | Self::DisjointChannel
            | Self::FibonacciChannel
            | Self::Pitchfork
            | Self::SchiffPitchfork
            | Self::ModifiedSchiffPitchfork
            | Self::InsidePitchfork
            | Self::XABCDPattern
            | Self::CypherPattern
            | Self::GartleyPattern
            | Self::BatPattern
            | Self::ButterflyPattern
            | Self::CrabPattern
            | Self::SharkPattern
            | Self::HeadAndShoulders
            | Self::ABCDPattern
            | Self::TrianglePattern
            | Self::ThreeDrivesPattern
            | Self::ElliottImpulse
            | Self::ElliottCorrection
            | Self::ElliottTriangle
            | Self::ElliottDoubleCombo
            | Self::ElliottTripleCombo
            | Self::GhostFeed => DrawingInteractionMode::MultiPoint,

            // Click-click tools
            Self::ArrowMarker
            | Self::PriceNote
            | Self::TrendLine
            | Self::Ray
            | Self::InfoLine
            | Self::ExtendedLine
            | Self::TrendAngle
            | Self::BarsPattern => DrawingInteractionMode::ClickClick,
        }
    }

    /// Anchors a drawing of this tool needs to be drawn and selected: what its renderer reads, not
    /// what the toolbar collects, so a drawing finished early or saved by an earlier release
    /// still counts. Tools without a meaningful partial construction require all their
    /// geometry anchors; the other multi-point tools can draw partial paths.
    #[cfg(feature = "drawings")]
    pub(crate) fn anchors_needed(&self) -> usize {
        match self {
            Self::ParallelChannel
            | Self::RotatedRect
            | Self::FibonacciWedge
            | Self::Pitchfan
            | Self::Sector => 3,
            Self::DoubleCurve => 4,
            Self::Triangle
            | Self::FlatTopBottom
            | Self::DisjointChannel
            | Self::Pitchfork
            | Self::SchiffPitchfork
            | Self::ModifiedSchiffPitchfork
            | Self::InsidePitchfork
            | Self::Polyline
            | Self::Path
            | Self::GhostFeed
            | Self::XABCDPattern
            | Self::CypherPattern
            | Self::GartleyPattern
            | Self::BatPattern
            | Self::ButterflyPattern
            | Self::CrabPattern
            | Self::SharkPattern
            | Self::HeadAndShoulders
            | Self::ABCDPattern
            | Self::TrianglePattern
            | Self::ThreeDrivesPattern
            | Self::ElliottImpulse
            | Self::ElliottCorrection
            | Self::ElliottDoubleCombo
            | Self::ElliottTripleCombo => 1,
            Self::ElliottTriangle => 6,
            _ if self.interaction_mode() == DrawingInteractionMode::SingleClick => 1,
            _ => 2,
        }
    }

    /// Returns the number of `ChartPoint`s
    /// required to complete a drawing of this type, or `None` for tools with
    /// no fixed point count (`Polyline`, `Brush`, cursors).
    ///
    /// | Mode | Typical count |
    /// |---|---|
    /// | `SingleClick` | 1 |
    /// | `ClickClick` / `DragToDraw` | 2 |
    /// | `MultiPoint` | 3--7 depending on tool |
    /// | `ContinuousDraw` | `None` (unlimited) |
    /// | `NotDrawing` | `None` |
    pub fn required_points(&self) -> Option<usize> {
        match self.interaction_mode() {
            DrawingInteractionMode::SingleClick => Some(1),
            DrawingInteractionMode::ClickClick | DrawingInteractionMode::DragToDraw => Some(2),
            DrawingInteractionMode::MultiPoint => {
                // Multi-point tools have varying requirements
                match self {
                    Self::FibonacciExtension
                    | Self::FibonacciWedge
                    | Self::TrendBasedFibTime
                    | Self::Pitchfan
                    | Self::Arc
                    | Self::Sector
                    | Self::RotatedRect
                    | Self::Curve
                    | Self::ProjectionTool
                    | Self::FlatTopBottom
                    | Self::DisjointChannel
                    | Self::FibonacciChannel
                    | Self::Triangle
                    | Self::ParallelChannel
                    | Self::Pitchfork
                    | Self::SchiffPitchfork
                    | Self::ModifiedSchiffPitchfork
                    | Self::InsidePitchfork => Some(3),
                    Self::ABCDPattern
                    | Self::ElliottCorrection
                    | Self::ElliottDoubleCombo
                    | Self::TrianglePattern
                    | Self::DoubleCurve => Some(4),
                    Self::XABCDPattern
                    | Self::CypherPattern
                    | Self::GartleyPattern
                    | Self::BatPattern
                    | Self::ButterflyPattern
                    | Self::CrabPattern
                    | Self::SharkPattern => Some(5),
                    Self::ElliottImpulse | Self::ElliottTriangle | Self::ElliottTripleCombo => {
                        Some(6)
                    }
                    Self::HeadAndShoulders | Self::ThreeDrivesPattern => Some(7),
                    // Polyline, Path and Ghost Feed have no fixed limit
                    _ => None,
                }
            }
            DrawingInteractionMode::ContinuousDraw => None,
            DrawingInteractionMode::NotDrawing => None,
        }
    }
}

#[cfg(all(test, feature = "drawings"))]
mod tests {
    use super::*;

    #[test]
    fn placement_collects_every_construction_anchor() {
        use DrawingToolType::*;
        for (tool, mode, count) in [
            (FibonacciExtension, DrawingInteractionMode::MultiPoint, 3),
            (TrendBasedFibTime, DrawingInteractionMode::MultiPoint, 3),
            (FibonacciWedge, DrawingInteractionMode::MultiPoint, 3),
            (Pitchfan, DrawingInteractionMode::MultiPoint, 3),
            (Arc, DrawingInteractionMode::MultiPoint, 3),
            (Sector, DrawingInteractionMode::MultiPoint, 3),
            (ArrowMarkLeft, DrawingInteractionMode::SingleClick, 1),
            (ArrowMarkRight, DrawingInteractionMode::SingleClick, 1),
            (Sticker, DrawingInteractionMode::SingleClick, 1),
            (RotatedRect, DrawingInteractionMode::MultiPoint, 3),
            (ProjectionTool, DrawingInteractionMode::MultiPoint, 3),
            (FlatTopBottom, DrawingInteractionMode::MultiPoint, 3),
            (DisjointChannel, DrawingInteractionMode::MultiPoint, 3),
            (FibonacciChannel, DrawingInteractionMode::MultiPoint, 3),
            (Curve, DrawingInteractionMode::MultiPoint, 3),
            (DoubleCurve, DrawingInteractionMode::MultiPoint, 4),
            (TrianglePattern, DrawingInteractionMode::MultiPoint, 4),
            (ThreeDrivesPattern, DrawingInteractionMode::MultiPoint, 7),
            (ElliottDoubleCombo, DrawingInteractionMode::MultiPoint, 4),
            (ElliottTriangle, DrawingInteractionMode::MultiPoint, 6),
            (ElliottTripleCombo, DrawingInteractionMode::MultiPoint, 6),
            (GartleyPattern, DrawingInteractionMode::MultiPoint, 5),
            (SharkPattern, DrawingInteractionMode::MultiPoint, 5),
            (PriceNote, DrawingInteractionMode::ClickClick, 2),
            (Image, DrawingInteractionMode::DragToDraw, 2),
            (ArrowMarker, DrawingInteractionMode::ClickClick, 2),
            (LongPos, DrawingInteractionMode::DragToDraw, 2),
            (ShortPos, DrawingInteractionMode::DragToDraw, 2),
            (GhostFeed, DrawingInteractionMode::MultiPoint, 5),
        ] {
            assert_eq!(tool.interaction_mode(), mode, "{tool:?}");
            let required = if tool == GhostFeed { None } else { Some(count) };
            assert_eq!(tool.required_points(), required, "{tool:?}");
            let mut manager = crate::drawings::DrawingManager::new();
            manager.start_drawing_with_coords(
                tool,
                egui::pos2(10.0, 500.0),
                |x| x,
                |y| 1000.0 - f64::from(y),
            );
            for i in 1..count {
                assert!(manager.drawings.is_empty(), "{tool:?} completed too soon");
                manager.add_point_with_coords(
                    egui::pos2(10.0 + i as f32 * 100.0, 400.0),
                    |x| x,
                    |y| 1000.0 - f64::from(y),
                );
            }
            if required.is_none() {
                assert!(manager.drawings.is_empty(), "{tool:?} completed too soon");
                manager.complete_curr_drawing();
            }
            assert_eq!(manager.drawings.len(), 1, "{tool:?}");
            if required.is_none() {
                assert_eq!(manager.drawings[0].chart_points.len(), count, "{tool:?}");
            }
            // Position and channel handles can sit away from their stored anchors, and a price
            // note's box moves by its body; `HandleService`'s `get_handles_by_tool` covers them.
            if matches!(
                tool,
                LongPos | ShortPos | ParallelChannel | FlatTopBottom | DisjointChannel | PriceNote
            ) {
                continue;
            }
            let drawing = &manager.drawings[0];
            let handles = crate::drawings::services::DrawingInteraction::new().get_handles(drawing);
            for point in &drawing.points {
                assert!(
                    handles.iter().any(|(_, p)| p == point),
                    "{tool:?}: no handle for {point:?}"
                );
            }
        }
    }
}
