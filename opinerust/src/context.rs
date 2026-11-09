//! Chart metadata a script reads through `syminfo.*` and `timeframe.*`.
//!
//! Pinecone registers both namespaces, but with empty defaults unless the host
//! supplies real values. The bridge therefore carries the chart's instrument and
//! interval into the engine. Values the chart genuinely does not know stay
//! empty/zero rather than being invented: a script that needs an unavailable
//! field should see the same "not provided" state as the chart's own UI.

use pine_core::TimeframeUnit;
use Oracly_charts::model::{Instrument, Timeframe as ChartTimeframe};

/// Symbol and interval a run should report to the script.
#[derive(Clone, Debug, PartialEq)]
pub struct RunContext {
    /// Symbol without the exchange prefix, e.g. `"AAPL"`.
    pub ticker: String,
    /// Exchange or data-source prefix, e.g. `"NASDAQ"`.
    pub prefix: String,
    /// Fully qualified symbol, e.g. `"NASDAQ:AAPL"`.
    pub tickerid: String,
    /// Human readable instrument name.
    pub description: String,
    /// Instrument type, e.g. `"stock"`.
    pub type_: String,
    /// Smallest price increment.
    pub mintick: f64,
    /// Price currency, e.g. `"USD"`.
    pub currency: String,
    /// Interval multiplier, e.g. `5` in `5m`; `0` when the chart has none or the
    /// interval has no honest Pinecone representation.
    pub multiplier: u32,
    /// Interval unit.
    pub unit: TimeframeUnit,
    /// Why the interval could not be represented, when that is the case.
    ///
    /// A chart interval Pinecone cannot express is not silently rounded to a
    /// nearby clock interval: scripts get no `timeframe.*` value and the host can
    /// explain why instead of showing a plausible but wrong period.
    pub unsupported_interval: Option<String>,
}

impl Default for RunContext {
    fn default() -> Self {
        Self {
            ticker: String::new(),
            prefix: String::new(),
            tickerid: String::new(),
            description: String::new(),
            type_: String::new(),
            mintick: 0.0,
            currency: String::new(),
            multiplier: 0,
            unit: TimeframeUnit::Minutes,
            unsupported_interval: None,
        }
    }
}

impl RunContext {
    /// Build the context from the chart's instrument and interval.
    ///
    /// Fields the chart leaves unset stay empty so a script can tell "unknown"
    /// from a real value; nothing is filled in with a plausible default.
    pub fn from_chart(instrument: Option<&Instrument>, timeframe: Option<ChartTimeframe>) -> Self {
        let mut context = Self::default();
        if let Some(instrument) = instrument {
            let exchange = instrument.exchange.clone().unwrap_or_default();
            context.ticker = instrument.symbol.clone();
            context.prefix = exchange.clone();
            context.tickerid = if exchange.is_empty() {
                instrument.symbol.clone()
            } else {
                format!("{exchange}:{}", instrument.symbol)
            };
            context.description = instrument.description.clone().unwrap_or_default();
            context.type_ = instrument
                .symbol_type
                .map(|kind| kind.to_string())
                .unwrap_or_default();
            context.mintick = instrument.tick_size.unwrap_or(0.0);
            context.currency = instrument.currency.clone().unwrap_or_default();
        }
        if let Some(timeframe) = timeframe {
            match pine_timeframe(timeframe) {
                Ok((multiplier, unit)) => {
                    context.multiplier = multiplier;
                    context.unit = unit;
                }
                Err(reason) => context.unsupported_interval = Some(reason),
            }
        }
        context
    }

    /// The Pinecone `SymInfo` this context describes.
    pub fn syminfo(&self) -> pine_core::SymInfo {
        pine_core::SymInfo {
            ticker: self.ticker.clone(),
            tickerid: self.tickerid.clone(),
            description: self.description.clone(),
            prefix: self.prefix.clone(),
            type_: self.type_.clone(),
            mintick: self.mintick,
            currency: self.currency.clone(),
            ..Default::default()
        }
    }

    /// The Pinecone `Timeframe` this context describes, or `None` when the chart
    /// has no interval to report.
    pub fn timeframe(&self) -> Option<pine_core::Timeframe> {
        if self.multiplier == 0 {
            return None;
        }
        Some(pine_core::Timeframe {
            multiplier: self.multiplier,
            unit: self.unit,
        })
    }

    /// Whether the chart interval has no honest Pinecone representation.
    pub fn interval_is_unsupported(&self) -> bool {
        self.unsupported_interval.is_some()
    }
}

/// Map the chart's interval onto Pinecone's multiplier/unit pair.
///
/// Pinecone has no sub-second, tick, volume, range or delta unit, and its period
/// string is built from the multiplier and suffix. Reporting such an interval as
/// a clock period would hand scripts a wrong period (a 100 ms chart is not
/// `100S`), so those intervals are refused with a reason instead.
fn pine_timeframe(timeframe: ChartTimeframe) -> Result<(u32, TimeframeUnit), String> {
    use ChartTimeframe as T;
    let clock = |multiplier: u64, unit: TimeframeUnit| -> Result<(u32, TimeframeUnit), String> {
        u32::try_from(multiplier)
            .map(|multiplier| (multiplier, unit))
            .map_err(|_| format!("interval {multiplier} is too large to report"))
    };
    match timeframe {
        // Pinecone has no sub-second unit; these are genuinely not expressible.
        T::Ms100 | T::Ms250 | T::Ms500 => Err(format!(
            "{timeframe} has no Pine Script interval: the smallest unit is seconds"
        )),
        T::Ticks(_) | T::Volume(_) | T::Range(_) | T::Delta(_) => Err(format!(
            "{timeframe} is not a clock interval and has no Pine Script period"
        )),
        T::Sec1 => Ok((1, TimeframeUnit::Seconds)),
        T::Sec2 => Ok((2, TimeframeUnit::Seconds)),
        T::Sec5 => Ok((5, TimeframeUnit::Seconds)),
        T::Sec10 => Ok((10, TimeframeUnit::Seconds)),
        T::Sec30 => Ok((30, TimeframeUnit::Seconds)),
        T::Min1 => Ok((1, TimeframeUnit::Minutes)),
        T::Min5 => Ok((5, TimeframeUnit::Minutes)),
        T::Min15 => Ok((15, TimeframeUnit::Minutes)),
        T::Min30 => Ok((30, TimeframeUnit::Minutes)),
        // Keep the multiplier in the smallest exact unit Pinecone has so
        // `timeframe.period` reads like Pine's own (`"60"`, `"240"`).
        T::Hour1 => Ok((60, TimeframeUnit::Minutes)),
        T::Hour4 => Ok((240, TimeframeUnit::Minutes)),
        T::Day1 => Ok((1, TimeframeUnit::Daily)),
        T::Week1 => Ok((1, TimeframeUnit::Weekly)),
        T::Month1 => Ok((1, TimeframeUnit::Monthly)),
        T::Weeks(count) => clock(count.get() as u64, TimeframeUnit::Weekly),
        T::Months(count) => clock(count.get() as u64, TimeframeUnit::Monthly),
        T::Custom(seconds) => {
            // Custom intervals are in seconds; larger than that has no suffix.
            let seconds = seconds.get();
            if seconds >= 60 {
                return Err(format!(
                    "{timeframe} is longer than a minute and has no Pine Script suffix"
                ));
            }
            clock(seconds, TimeframeUnit::Seconds)
        }
        // `Timeframe` is `#[non_exhaustive]`: a variant added later has no
        // Pinecone unit we can prove. Leave `timeframe.*` unset rather than
        // inventing a period a script would act on.
        other => Err(format!("{other} has no Pinecone interval mapping")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context_for(timeframe: ChartTimeframe) -> RunContext {
        RunContext::from_chart(None, Some(timeframe))
    }

    #[test]
    fn clock_intervals_map_to_the_same_period() {
        for (timeframe, period) in [
            (ChartTimeframe::Sec1, "1S"),
            (ChartTimeframe::Sec30, "30S"),
            (ChartTimeframe::Min1, "1"),
            (ChartTimeframe::Min15, "15"),
            (ChartTimeframe::Hour1, "60"),
            (ChartTimeframe::Hour4, "240"),
            (ChartTimeframe::Day1, "1D"),
            (ChartTimeframe::Week1, "1W"),
            (ChartTimeframe::Month1, "1M"),
        ] {
            let context = context_for(timeframe);
            let mapped = context.timeframe().expect("mapped");
            assert_eq!(mapped.period(), period, "{timeframe}");
        }
    }

    /// A 100 ms chart is not a 100 second chart. Pinecone has no sub-second unit,
    /// so the honest answer is "no period", not a wrong one.
    #[test]
    fn sub_second_intervals_are_refused_instead_of_misreported() {
        for timeframe in [
            ChartTimeframe::Ms100,
            ChartTimeframe::Ms250,
            ChartTimeframe::Ms500,
        ] {
            let context = context_for(timeframe);
            assert!(context.timeframe().is_none(), "{timeframe}");
            assert!(context.interval_is_unsupported(), "{timeframe}");
        }
    }

    /// Tick, volume, range and delta bars are not clock bars; reporting them as
    /// minutes would make `timeframe.period` lie.
    #[test]
    fn trade_based_intervals_are_refused() {
        use std::num::NonZeroU64;
        let count = NonZeroU64::new(500).expect("nonzero");
        for timeframe in [
            ChartTimeframe::Ticks(count),
            ChartTimeframe::Volume(count),
            ChartTimeframe::Range(count),
            ChartTimeframe::Delta(count),
        ] {
            let context = context_for(timeframe);
            assert!(context.timeframe().is_none(), "{timeframe}");
            assert!(context.interval_is_unsupported(), "{timeframe}");
        }
    }

    #[test]
    fn a_custom_interval_keeps_its_seconds() {
        use std::num::NonZeroU64;
        let context = context_for(ChartTimeframe::Custom(
            NonZeroU64::new(45).expect("nonzero"),
        ));
        assert_eq!(context.timeframe().expect("mapped").period(), "45S");
    }

    #[test]
    fn a_chart_without_an_interval_reports_none() {
        let context = RunContext::from_chart(None, None);
        assert!(context.timeframe().is_none());
        assert!(
            !context.interval_is_unsupported(),
            "no interval is not an error"
        );
    }

    #[test]
    fn instrument_fields_come_from_the_chart() {
        use Oracly_charts::model::{Instrument, SymbolType};
        let instrument = Instrument::new("AAPL")
            .with_exchange("NASDAQ")
            .with_description("Apple Inc.")
            .with_symbol_type(SymbolType::Stock)
            .with_tick_size(0.01);
        let context = RunContext::from_chart(Some(&instrument), Some(ChartTimeframe::Min5));
        assert_eq!(context.ticker, "AAPL");
        assert_eq!(context.prefix, "NASDAQ");
        assert_eq!(context.tickerid, "NASDAQ:AAPL");
        assert_eq!(context.description, "Apple Inc.");
        assert_eq!(context.type_, "stock");
        assert_eq!(context.mintick, 0.01);
        assert_eq!(context.multiplier, 5);
    }

    /// An instrument without an exchange keeps a bare ticker rather than a
    /// dangling separator.
    #[test]
    fn an_exchange_less_instrument_keeps_a_bare_ticker_id() {
        use Oracly_charts::model::Instrument;
        let instrument = Instrument::new("BTCUSDT");
        let context = RunContext::from_chart(Some(&instrument), None);
        assert_eq!(context.tickerid, "BTCUSDT");
        assert_eq!(context.prefix, "");
    }
}
