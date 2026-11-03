//! Saved options of the chart pattern, Elliott wave and cycle tools: which annotations they
//! show, the degree Elliott waves are labelled in, and the ratios harmonic patterns are judged by.

use crate::config::DrawingToolType;
use serde::{Deserialize, Serialize};

/// Degree of an Elliott wave, which fixes how its waves are labelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WaveDegree {
    /// `((I))` and `((a))`.
    GrandSupercycle,
    /// `(I)` and `(a)`.
    Supercycle,
    /// `I` and `a`.
    Cycle,
    /// Circled `1` and `A`.
    Primary,
    /// `(1)` and `(A)`.
    Intermediate,
    /// `1` and `A`.
    #[default]
    Minor,
    /// `i` and `a`.
    Minute,
    /// `(i)` and `(a)`.
    Minuette,
}

/// One label of an Elliott wave: its text, and whether a circle is drawn around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveLabel {
    /// Text of the label.
    pub text: String,
    /// Whether the label is drawn inside a circle.
    pub circled: bool,
}

/// A wave of an Elliott pattern: a numbered motive wave or a lettered corrective one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    /// Motive waves 0 (the origin) to 5.
    Number(u8),
    /// Corrective waves A to E and combination waves W, X, Y and Z.
    Letter(char),
}

impl WaveDegree {
    /// Every degree, from the largest to the smallest.
    pub const ALL: [Self; 8] = [
        Self::GrandSupercycle,
        Self::Supercycle,
        Self::Cycle,
        Self::Primary,
        Self::Intermediate,
        Self::Minor,
        Self::Minute,
        Self::Minuette,
    ];

    /// The degree's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::GrandSupercycle => "Grand Supercycle",
            Self::Supercycle => "Supercycle",
            Self::Cycle => "Cycle",
            Self::Primary => "Primary",
            Self::Intermediate => "Intermediate",
            Self::Minor => "Minor",
            Self::Minute => "Minute",
            Self::Minuette => "Minuette",
        }
    }

    /// The label of `wave` in this degree. Degrees that count in roman numerals leave the
    /// origin unlabelled.
    pub fn label(self, wave: Wave) -> WaveLabel {
        const UPPER_ROMAN: [&str; 6] = ["", "I", "II", "III", "IV", "V"];
        const LOWER_ROMAN: [&str; 6] = ["", "i", "ii", "iii", "iv", "v"];
        // Degrees counting in roman numerals letter their corrections in lower case.
        let roman = matches!(
            self,
            Self::GrandSupercycle | Self::Supercycle | Self::Cycle | Self::Minute | Self::Minuette
        );
        let core = match wave {
            Wave::Number(n) if roman => {
                let table = if matches!(self, Self::Minute | Self::Minuette) {
                    LOWER_ROMAN
                } else {
                    UPPER_ROMAN
                };
                table.get(usize::from(n)).copied().unwrap_or("").to_owned()
            }
            Wave::Number(n) => n.to_string(),
            Wave::Letter(c) if roman => c.to_ascii_lowercase().to_string(),
            Wave::Letter(c) => c.to_string(),
        };
        if core.is_empty() {
            return WaveLabel {
                text: core,
                circled: false,
            };
        }
        let text = match self {
            Self::GrandSupercycle => format!("(({core}))"),
            Self::Supercycle | Self::Intermediate | Self::Minuette => format!("({core})"),
            _ => core,
        };
        WaveLabel {
            text,
            circled: self == Self::Primary,
        }
    }
}

/// A closed range a harmonic ratio must lie in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatioBand {
    /// Least accepted ratio.
    pub min: f64,
    /// Greatest accepted ratio.
    pub max: f64,
}

impl RatioBand {
    const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    /// Whether `ratio` lies in the band.
    pub fn contains(self, ratio: f64) -> bool {
        (self.min..=self.max).contains(&ratio)
    }
}

/// A ratio a harmonic pattern is judged by: the leg measured, the leg it is measured
/// against (anchor indices into X, A, B, C, D), and the band it must lie in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmonicRule {
    /// Name of the ratio, such as `AB/XA`.
    pub name: &'static str,
    /// Anchors of the measured leg.
    pub leg: [usize; 2],
    /// Anchors of the leg it is measured against.
    pub base: [usize; 2],
    /// Accepted ratios.
    pub band: RatioBand,
}

const fn rule(
    name: &'static str,
    leg: [usize; 2],
    base: [usize; 2],
    band: RatioBand,
) -> HarmonicRule {
    HarmonicRule {
        name,
        leg,
        base,
        band,
    }
}

const AB_XA: ([usize; 2], [usize; 2]) = ([1, 2], [0, 1]);
const BC_AB: ([usize; 2], [usize; 2]) = ([2, 3], [1, 2]);
const CD_BC: ([usize; 2], [usize; 2]) = ([3, 4], [2, 3]);
const AD_XA: ([usize; 2], [usize; 2]) = ([1, 4], [0, 1]);

/// The ratios a named harmonic pattern is judged by, or `None` for other tools.
pub fn harmonic_rules(tool: DrawingToolType) -> Option<Vec<HarmonicRule>> {
    use DrawingToolType as T;
    let standard = |ab: RatioBand, bc: RatioBand, cd: RatioBand, ad: RatioBand| {
        vec![
            rule("AB/XA", AB_XA.0, AB_XA.1, ab),
            rule("BC/AB", BC_AB.0, BC_AB.1, bc),
            rule("CD/BC", CD_BC.0, CD_BC.1, cd),
            rule("AD/XA", AD_XA.0, AD_XA.1, ad),
        ]
    };
    let bc = RatioBand::new(0.382, 0.886);
    Some(match tool {
        T::GartleyPattern => standard(
            RatioBand::new(0.55, 0.68),
            bc,
            RatioBand::new(1.13, 1.618),
            RatioBand::new(0.74, 0.83),
        ),
        T::BatPattern => standard(
            RatioBand::new(0.382, 0.5),
            bc,
            RatioBand::new(1.618, 2.618),
            RatioBand::new(0.84, 0.92),
        ),
        T::ButterflyPattern => standard(
            RatioBand::new(0.74, 0.83),
            bc,
            RatioBand::new(1.618, 2.618),
            RatioBand::new(1.272, 1.618),
        ),
        T::CrabPattern => standard(
            RatioBand::new(0.382, 0.618),
            bc,
            RatioBand::new(2.618, 3.618),
            RatioBand::new(1.55, 1.69),
        ),
        T::SharkPattern => standard(
            RatioBand::new(0.382, 0.618),
            RatioBand::new(1.13, 1.618),
            RatioBand::new(1.618, 2.24),
            RatioBand::new(0.886, 1.13),
        ),
        // C extends XA and D retraces XC.
        T::CypherPattern => vec![
            rule("AB/XA", AB_XA.0, AB_XA.1, RatioBand::new(0.382, 0.618)),
            rule("XC/XA", [0, 3], [0, 1], RatioBand::new(1.272, 1.414)),
            rule("CD/XC", [3, 4], [0, 3], RatioBand::new(0.74, 0.83)),
        ],
        _ => return None,
    })
}

/// Which annotations a pattern, Elliott wave or cycle tool shows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PatternConfig {
    /// Letters, numbers or names at the anchors.
    pub show_labels: bool,
    /// Leg ratios of harmonic, ABCD and three drives patterns.
    pub show_ratios: bool,
    /// Whether a harmonic pattern colours its ratios by its rules and names itself with a
    /// verdict at D.
    pub show_validity: bool,
    /// Head and shoulders neckline.
    pub show_neckline: bool,
    /// Head and shoulders target, and the wave 3 target of an Elliott impulse.
    pub show_target: bool,
    /// Reversal zone of a three drives pattern.
    pub show_zone: bool,
    /// Cycle numbers along cyclic lines.
    pub show_cycle_numbers: bool,
    /// Degree Elliott waves are labelled in.
    pub degree: WaveDegree,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            show_labels: true,
            show_ratios: true,
            show_validity: true,
            show_neckline: true,
            show_target: true,
            show_zone: true,
            show_cycle_numbers: false,
            degree: WaveDegree::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees_label_waves_in_their_notation() {
        let labels = |degree: WaveDegree| {
            [Wave::Number(0), Wave::Number(3), Wave::Letter('B')].map(|w| degree.label(w).text)
        };
        assert_eq!(labels(WaveDegree::Minor), ["0", "3", "B"]);
        assert_eq!(labels(WaveDegree::Intermediate), ["(0)", "(3)", "(B)"]);
        assert_eq!(labels(WaveDegree::Cycle), ["", "III", "b"]);
        assert_eq!(labels(WaveDegree::Supercycle), ["", "(III)", "(b)"]);
        assert_eq!(
            labels(WaveDegree::GrandSupercycle),
            ["", "((III))", "((b))"]
        );
        assert_eq!(labels(WaveDegree::Minute), ["", "iii", "b"]);
        assert_eq!(labels(WaveDegree::Minuette), ["", "(iii)", "(b)"]);
        assert_eq!(labels(WaveDegree::Primary), ["0", "3", "B"]);
        assert!(WaveDegree::Primary.label(Wave::Number(3)).circled);
        assert!(!WaveDegree::Minor.label(Wave::Number(3)).circled);
    }

    #[test]
    fn named_harmonics_carry_their_ratio_bands() {
        let rules = harmonic_rules(DrawingToolType::GartleyPattern).unwrap();
        assert_eq!(
            rules.iter().map(|r| r.name).collect::<Vec<_>>(),
            ["AB/XA", "BC/AB", "CD/BC", "AD/XA"]
        );
        assert!(rules[0].band.contains(0.618) && !rules[0].band.contains(0.5));
        assert!(rules[3].band.contains(0.786));
        let cypher = harmonic_rules(DrawingToolType::CypherPattern).unwrap();
        assert_eq!(cypher.len(), 3);
        assert_eq!(cypher[1].leg, [0, 3]);
        assert!(cypher[2].band.contains(0.786));
        assert!(harmonic_rules(DrawingToolType::XABCDPattern).is_none());
        for tool in [
            DrawingToolType::BatPattern,
            DrawingToolType::ButterflyPattern,
            DrawingToolType::CrabPattern,
            DrawingToolType::SharkPattern,
        ] {
            assert!(harmonic_rules(tool).is_some(), "{tool:?}");
        }
    }

    #[test]
    fn options_read_partial_saves() {
        let config: PatternConfig =
            serde_json::from_value(serde_json::json!({"show_ratios": false})).unwrap();
        assert!(!config.show_ratios && config.show_labels);
        assert_eq!(config.degree, WaveDegree::Minor);
    }
}
