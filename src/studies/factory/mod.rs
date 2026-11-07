//! Indicator factory for dynamic indicator creation by name.
//!
//! The [`IndicatorFactory`] provides a registry of constructor functions
//! keyed by indicator name strings. The built-in catalogue is
//! registered under its canonical [`Indicator::name`] key and a category,
//! and users can add their own at runtime. It is the catalogue of studies: a
//! picker lists [`list`](IndicatorFactory::list), describes each entry with
//! [`Indicator::desc`] and [`category`](IndicatorFactory::category), and
//! creates it with [`create`](IndicatorFactory::create).
//!
//! # Example
//!
//! ```rust
//! use egui_charts::studies::IndicatorFactory;
//!
//! let factory = IndicatorFactory::new(); // includes all built-ins
//! let rsi = factory.create("RSI").unwrap();
//! assert_eq!(rsi.name(), "RSI");
//!
//! // Register a custom entry
//! let mut factory = factory;
//! factory.register("MySMA", || Box::new(egui_charts::studies::SMA::new(42)));
//! ```

use crate::data::OrderFlowKind;
use crate::studies::*;
use std::collections::HashMap;

/// Type alias for indicator constructor closures stored in the factory.
type IndicatorConstructor = Box<dyn Fn() -> Box<dyn Indicator> + Send + Sync>;

/// Register every listed built-in indicator type with the factory, under its category.
///
/// Each indicator is constructed through its [`Default`] implementation --
/// the single canonical source of its conventional parameters -- and keyed by
/// the [`Indicator::name`] it reports. Keying off `name()` keeps the registry
/// in lock-step with the indicators themselves: a name can never drift from
/// the value the rendering layer and legends display, and a new indicator is
/// reachable through the factory the moment it is added to this list.
macro_rules! register_builtins {
    ($factory:expr, $($category:expr => [$($ty:ty),+ $(,)?]),+ $(,)?) => {
        $($(
            // Read the canonical name from a throwaway default instance so the
            // key is always exactly what the indicator reports at runtime.
            let key = <$ty as Default>::default().name().to_string();
            $factory.constructors.insert(
                key,
                ($category, Box::new(|| Box::new(<$ty as Default>::default()))),
            );
        )+)+
    };
}

/// A factory that creates [`Indicator`] instances by name.
///
/// On construction, catalogue studies are registered under their canonical
/// [`Indicator::name`] (e.g. `"SMA"`, `"RSI"`, `"MACD"`, `"SuperTrend"`) using
/// its conventional default parameters. Each name selects a distinct calculation
/// or source range. Additional indicators can be registered at runtime with
/// [`register`](Self::register).
pub struct IndicatorFactory {
    constructors: HashMap<String, (&'static str, IndicatorConstructor)>,
}

impl IndicatorFactory {
    /// The category of every indicator added with [`register`](Self::register).
    pub const CUSTOM: &'static str = "Custom";
    /// The category of the user's own studies, such as scripts the host compiles or imports;
    /// a study browser lists them as the user's scripts.
    pub const PERSONAL: &'static str = "Personal";
    /// The category of studies the host obtains from other users.
    pub const COMMUNITY: &'static str = "Community";
    /// The category of the built-in volume and time price profiles.
    pub const PROFILES: &'static str = "Profiles";
    /// The category of the built-in pattern detectors.
    pub const PATTERNS: &'static str = "Patterns";
    /// Moving-average studies.
    pub const MOVING_AVERAGES: &'static str = "Moving averages";
    /// Bands and channels drawn around price.
    pub const BANDS: &'static str = "Bands and channels";
    /// Trend strength and direction.
    pub const TREND: &'static str = "Trend";
    /// Trailing stops.
    pub const STOPS: &'static str = "Stops and trailing";
    /// Unbounded momentum studies.
    pub const MOMENTUM: &'static str = "Momentum";
    /// Bounded oscillators.
    pub const OSCILLATORS: &'static str = "Oscillators";
    /// Volatility studies.
    pub const VOLATILITY: &'static str = "Volatility";
    /// Volume studies.
    pub const VOLUME: &'static str = "Volume";
    /// VWAP and other average-price studies.
    pub const AVERAGE_PRICE: &'static str = "VWAP and average price";
    /// Pivots, levels and automatic drawings.
    pub const LEVELS: &'static str = "Pivots and levels";
    /// Regression and statistical studies.
    pub const STATISTICS: &'static str = "Statistics and regression";
    /// Price replicas and session studies.
    pub const PRICE: &'static str = "Price and sessions";
    /// Market-breadth studies.
    pub const BREADTH: &'static str = "Market breadth";
    /// Order-flow studies.
    pub const ORDER_FLOW: &'static str = "Order flow";

    /// The built-in categories of the Indicators page, in picker order.
    pub const INDICATOR_CATEGORIES: [&'static str; 14] = [
        Self::MOVING_AVERAGES,
        Self::BANDS,
        Self::TREND,
        Self::STOPS,
        Self::MOMENTUM,
        Self::OSCILLATORS,
        Self::VOLATILITY,
        Self::VOLUME,
        Self::AVERAGE_PRICE,
        Self::LEVELS,
        Self::STATISTICS,
        Self::PRICE,
        Self::BREADTH,
        Self::ORDER_FLOW,
    ];

    /// Create a factory with the built-in catalogue enabled by the current features.
    pub fn new() -> Self {
        let mut factory = Self {
            constructors: HashMap::new(),
        };

        factory.register_builtin();
        factory
    }

    /// Register every built-in indicator under its canonical name.
    ///
    /// Types use their defaults; automatic drawings select a construction kind and
    /// stream studies select aggregation parameters. Names come from each study.
    fn register_builtin(&mut self) {
        register_builtins!(
            self,
            Self::MOVING_AVERAGES => [
                SMA, EMA, WMA, DEMA, TEMA, HMA, ALMA, VWMA, LSMA, SWMA, KAMA, SmoothedMA,
                GuppyMA, McGinleyDynamic, VIDYA, ZLEMA, HammingMA, MultipleMA, MARibbon,
                EMACross, MACross, MixedMACross, TripleMA,
            ],
            Self::BANDS => [
                BollingerBands, KeltnerChannels, DonchianChannels, Envelopes, MAChannel,
                AccelerationBands, StandardErrorBands, LinearRegressionChannel, Median,
            ],
            Self::TREND => [
                ADX, DirectionalMovement, AdxBreakout, Aroon, AroonOscillator, VortexIndicator,
                IchimokuCloud, WilliamsAlligator, SqueezeMomentum, TrendIntensityIndex,
                TrendDetectionIndex, Qstick, ChoppinessIndex, ChopZone, ElderImpulseSystem,
            ],
            Self::STOPS => [
                SuperTrend, ParabolicSAR, ChandelierExit, ChandeKrollStop, VolatilityStop,
            ],
            Self::MOMENTUM => [
                MACD, VolumeWeightedMACD, Momentum, RateOfChange, AwesomeOscillator,
                AcceleratorOscillator, TRIX, CoppockCurve, KnowSureThing, PriceOscillator,
                PriceMomentumOscillator, PringsSpecialK, ElderRay, BullBearPower,
                DetrendedPriceOscillator, BalanceOfPower, TechnicalRatings,
            ],
            Self::OSCILLATORS => [
                RSI, Stochastic, StochasticRSI, WilliamsR, CCI, WoodiesCCI, ConnorsRSI, KDJ,
                DoubleStochastic, StochasticMomentumIndex, SMIErgodicIndicator,
                SMIErgodicOscillator, TrueStrengthIndex, UltimateOscillator, FisherTransform,
                RelativeVigorIndex, ChandeMomentumOscillator, RainbowOscillator,
                UltimateMomentum, SchaffTrendCycle, CenterOfGravity, PrettyGoodOscillator,
                RsiDivergence, KnoxvilleDivergence,
            ],
            Self::VOLATILITY => [
                ATR, TrueRange, StandardDeviation, HistoricalVolatility, ChaikinVolatility,
                BollingerBandsWidth, PercentB, BBTrend, OhlcVolatility, ParkinsonVol,
                ZeroTrendVol, AverageDailyRange, UlcerIndex, RelativeVolatilityIndex,
                CommoditySelectionIndex, MassIndex,
            ],
            Self::VOLUME => [
                Volume, UpDownVolume, NetVolume, Volume24h, RelativeVolumeAtTime,
                VolumeRateOfChange, VolumeOscillator, PercentageVolumeOscillator,
                OnBalanceVolume, AccumulationDistribution, WilliamsAD, ChaikinMoneyFlow,
                ChaikinOscillator, MoneyFlowIndex, KlingerOscillator, ForceIndex,
                EaseOfMovement, VolumePriceTrend, PriceVolumeTrendStudy, NegativeVolumeIndex,
                PositiveVolumeIndex, MarketFacilitationIndex,
            ],
            Self::AVERAGE_PRICE => [
                VolumeWeightedAvgPrice, AnchoredVWAP, VwapAutoAnchored, TimeWeightedAvgPrice,
                VisibleAveragePrice,
            ],
            Self::LEVELS => [
                PivotPoints, PivotPointsHighLow, IntradayPivotPoints, ProjectedPivotPoints,
                GhostPivots, MissedPivots, KeyLevels, WilliamsFractals, ZigZag,
                Week52HighLowLevels, Gaps,
            ],
            Self::STATISTICS => [
                LinearRegression, LinearRegressionSlope, StandardError, TrendStrengthIndex,
                RankCorrelationIndex, RciRibbon, Correlation, Ratio, Spread, MajorityRule,
                AccumulationSwingIndex,
            ],
            Self::PRICE => [
                HigherIntervalCandles, BollingerBars, TypicalPrice, MedianPrice, WeightedClose,
                OHLC4, TradingSessions, Week52HighLow, Performance, Seasonality, LunarPhases,
            ],
            Self::BREADTH => [
                AdvanceDeclineRatio, AdvanceDeclineLine, CumulativeVolumeIndex,
                AdvanceDeclineRatioBars,
            ],
            Self::PROFILES => [
                VolProfileVisible, VolProfileFixed, VolProfileSession, VolProfilePeriodic,
                VolProfileAnchored, VolProfileAutoAnchored, TimePriceOpportunity,
            ],
            Self::PATTERNS => [
                CandlestickPatterns, RisingWedge, FallingWedge, ElliottWave, Reversal,
            ],
        );
        for kind in [
            OrderFlowKind::Spread,
            OrderFlowKind::BookSize {
                levels: 10,
                distance: 0.0,
            },
            OrderFlowKind::TradeRate {
                volume: false,
                seconds: 60,
                instant: 3,
            },
            OrderFlowKind::AggressorRate {
                seconds: 60,
                instant: 3,
            },
            OrderFlowKind::LevelsExecuted {
                seconds: 0,
                reversal: false,
                min: 0.0,
                max: f64::MAX,
            },
            OrderFlowKind::SizeDelta {
                ranges: [[0.0, 9.0], [10.0, 99.0], [100.0, f64::MAX]],
            },
            OrderFlowKind::FlowSum { bars: 10 },
            OrderFlowKind::FilteredCvd {
                min: 100.0,
                max: f64::MAX,
            },
            OrderFlowKind::LevelsTradedThrough { seconds: 0 },
        ] {
            let name = OrderFlowStudy::new(kind).name().to_owned();
            self.constructors.insert(
                name,
                (
                    Self::ORDER_FLOW,
                    Box::new(move || Box::new(OrderFlowStudy::new(kind))),
                ),
            );
        }
        for family in PatternFamily::ALL {
            let name = ChartPatterns::new(family).name().to_owned();
            self.constructors.insert(
                name,
                (
                    Self::PATTERNS,
                    Box::new(move || Box::new(ChartPatterns::new(family))),
                ),
            );
        }
        #[cfg(feature = "drawings")]
        for kind in AutoDrawingKind::ALL {
            let name = AutoDrawing::new(kind, 5.0, 10).name().to_owned();
            self.constructors.insert(
                name,
                (
                    Self::LEVELS,
                    Box::new(move || Box::new(AutoDrawing::new(kind, 5.0, 10))),
                ),
            );
        }
    }

    /// Register a named indicator constructor in the [`CUSTOM`](Self::CUSTOM) category.
    ///
    /// If a constructor with the same name already exists it is replaced.
    ///
    /// # Arguments
    /// * `name` -- The lookup key (e.g. `"MySMA"`).
    /// * `constructor` -- A closure that returns a new boxed indicator.
    pub fn register<F>(&mut self, name: impl Into<String>, constructor: F)
    where
        F: Fn() -> Box<dyn Indicator> + Send + Sync + 'static,
    {
        self.register_in(Self::CUSTOM, name, constructor);
    }

    /// Register a named indicator constructor in `category`, such as [`PERSONAL`](Self::PERSONAL)
    /// for a script the user imported, [`COMMUNITY`](Self::COMMUNITY) or
    /// [`PROFILES`](Self::PROFILES).
    ///
    /// If a constructor with the same name already exists it is replaced.
    pub fn register_in<F>(
        &mut self,
        category: &'static str,
        name: impl Into<String>,
        constructor: F,
    ) where
        F: Fn() -> Box<dyn Indicator> + Send + Sync + 'static,
    {
        self.constructors
            .insert(name.into(), (category, Box::new(constructor)));
    }

    /// Create an indicator by its registered name, or `None` if not found.
    pub fn create(&self, name: &str) -> Option<Box<dyn Indicator>> {
        self.constructors.get(name).map(|(_, f)| f())
    }

    /// The category of the indicator registered under `name`, such as `"Momentum"` or
    /// `"Volume"` for a built-in and [`CUSTOM`](Self::CUSTOM) for one added with
    /// [`register`](Self::register), or `None` if not found.
    pub fn category(&self, name: &str) -> Option<&'static str> {
        self.constructors.get(name).map(|(category, _)| *category)
    }

    /// Return a list of all registered indicator names (unordered).
    pub fn list(&self) -> Vec<String> {
        self.constructors.keys().cloned().collect()
    }

    /// Check whether an indicator with `name` is registered.
    pub fn has(&self, name: &str) -> bool {
        self.constructors.contains_key(name)
    }

    /// Remove the indicator registered under `name`.
    ///
    /// Returns `true` if it was present and removed, `false` otherwise.
    pub fn unregister(&mut self, name: &str) -> bool {
        self.constructors.remove(name).is_some()
    }

    /// Remove all registered indicators, including built-ins.
    pub fn clear(&mut self) {
        self.constructors.clear();
    }

    /// Return the number of currently registered indicators.
    pub fn count(&self) -> usize {
        self.constructors.len()
    }
}

impl Default for IndicatorFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod builtin_properties;
#[cfg(test)]
mod tests;
