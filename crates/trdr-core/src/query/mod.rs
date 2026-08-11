//! The per-screen query models (section 11), and the values they are built from.
//!
//! Section 11 puts one rule above the others: a screen never assembles an
//! upstream or database model itself. A query service in the runtime builds a
//! versioned model per screen, and the screen renders that and nothing else.
//! This module is where those models are written down.
//!
//! Three consequences run through everything below.
//!
//! # A model names its own version
//!
//! Section 11 spells the models `TodayModel/v1`, not `TodayModel`. The version
//! is part of the contract because the screen and the host ship separately —
//! a packaged app can meet a newer CLI, and the wrong assumption to make at that
//! moment is that a field's absence means it was empty. So every model carries
//! [`ModelId`] as its first field, the constant is on the model's own type, and
//! [`tests::a_model_is_named_the_way_section_11_names_it`] holds the spellings
//! against the design document.
//!
//! # A model says where its data came from
//!
//! Milestone M2 requires the app to state at all times that what is on screen is
//! synthetic rather than a real account or real market data. Making that a
//! screen's responsibility would make it a screen's option, so [`DataOrigin`]
//! is a field of every model instead. A screen cannot render one of these
//! without having been told, and a screen that ignores it is failing a visible
//! contract rather than forgetting an invisible one.
//!
//! # Money is an integer and a ratio is a string
//!
//! Section 7.2 fixes both: amounts are KRW integers, ratios are decimal strings
//! at a fixed scale, and no binary float is ever hashed. [`Krw`] and [`Ratio`]
//! are those two types. They are newtypes rather than aliases so that a field
//! holding one cannot silently be handed the other, and so that the boundary to
//! TypeScript is decided once here rather than per screen.

mod lab;
mod strategies;
mod today;

pub use lab::{
    BacktestAssumptions, BacktestMetrics, BacktestWarning, CoverageGap, CurvePoint, DataCoverage,
    LabDraftModel, LabResultModel, Rule, RuleSection, StrategyRules, SupportState, Verdict
};
pub use strategies::{
    ObservationProgress, PaperPosition, PaperValidationState, RuleDeviation, Signal,
    StrategiesModel, StrategyDetailModel, StrategyLineage, StrategySummary
};
pub use today::{
    AccountSummary, Disclosure, Holding, TodayEvent, TodayEventKind, TodayModel, ValidationEvent
};

use crate::time::Timestamp;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

/// The name and version of a query model, as section 11 spells it.
///
/// A plain string on the wire — `"TodayModel/v1"` — so that a reader of a
/// captured response or a log line sees the contract's own spelling rather than
/// a number that has to be looked up.
///
/// The inside is a [`Cow`] rather than a `&'static str` so the type can be both
/// a constant on a model and the result of parsing one back. A borrowed variant
/// is constructible in a `const`, which is what lets `TodayModel::MODEL` exist;
/// an owned one is what arrives when a response is deserialised.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelId(Cow<'static, str>);

impl ModelId
{
    /// Names a model at compile time.
    pub const fn new(name: &'static str) -> Self
    {
        Self(Cow::Borrowed(name))
    }

    /// The spelling, for a caller that needs the text rather than the value.
    pub fn as_str(&self) -> &str
    {
        &self.0
    }
}

impl fmt::Display for ModelId
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        formatter.write_str(&self.0)
    }
}

/// `ModelId` crosses to TypeScript as the string it is.
impl specta::Type for ModelId
{
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType
    {
        <str as specta::Type>::definition(types)
    }
}

/// Whether what a model carries is real or made up.
///
/// Milestone M2 runs the whole app on a synthetic fixture, and requires that the
/// person looking at it is never left to assume otherwise. This field is how the
/// app keeps that promise: it travels with the data rather than beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DataOrigin
{
    /// A checked-in fixture. Not an account, not a market, not a broker.
    Synthetic,
    /// Collected from the sources the workspace is configured for.
    Collected
}

/// How much of one section of a screen is actually there.
///
/// The visual contract's `asyncData` state model, unchanged. It is per section
/// rather than per screen because a screen is rarely in one state: Today's
/// holdings can be stale while its disclosures are ready, and a screen that had
/// to pick one state for all of itself would have to lie about one of them.
///
/// `idle` and `loading` are not here. Those are states of a request that has not
/// answered yet, and a model only exists once one has — the screen holds them
/// before this value arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SectionState
{
    /// Present, current, and complete.
    Ready,
    /// Nothing to show, and that is the truth rather than a failure.
    Empty,
    /// Present, but older than it should be. The screen shows it and says so.
    Stale,
    /// Could not be built. The screen shows the error, not an empty table.
    Error
}

/// Which way a number moved, in the sense the market means it.
///
/// The colour mapping belongs to the visual contract and not here, and it is the
/// Korean convention: up is red, down is blue. This type carries the direction
/// so that no screen has to derive it from a sign and get the mapping wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum MarketTone
{
    /// Gained.
    Up,
    /// Lost.
    Down,
    /// Unchanged.
    Flat
}

/// The state of the connection to the broker.
///
/// The visual contract's `brokerConnection` state model. Its rule is worth
/// repeating where the type is defined: green means the system is connected, and
/// never that an investment is doing well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum BrokerConnectionState
{
    /// Connected, and allowed to read only.
    ConnectedReadOnly,
    /// Talking to the broker right now.
    Syncing,
    /// Connected, but what it last returned is old.
    Stale,
    /// Not connected.
    Disconnected,
    /// The last attempt failed.
    Error
}

/// An amount of money, in won.
///
/// An integer, because section 7.2 says so and because the alternative is a
/// binary float that is one rounding away from a total that does not match its
/// own rows. Crosses to TypeScript as a number: won amounts in a personal
/// account stay far below the 2^53 that a JavaScript number represents exactly.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(transparent)]
pub struct Krw(pub i64);

/// A ratio, as a decimal string at a fixed scale.
///
/// A string, for the reason section 7.2 gives: a ratio takes part in the hashes
/// that make a backtest reproducible, and a binary float is not reproducible
/// across the places that would have to agree. `"0.0412"` is four decimal
/// places, which is the scale trdr writes ratios at.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(transparent)]
pub struct Ratio(pub String);

/// What every model carries, whatever screen it is for.
///
/// Split out so that adding a field the screens all need is one edit rather than
/// five, and so a reader can see at a glance what is common and what is Today's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ModelHeader
{
    /// The model's name and version, per section 11.
    pub model: ModelId,
    /// Whether this is real data.
    pub origin: DataOrigin,
    /// When the model was built.
    pub built_at: Timestamp
}

impl ModelHeader
{
    /// A header for a model built now, from data of the given origin.
    pub fn new(model: ModelId, origin: DataOrigin, built_at: Timestamp) -> Self
    {
        Self {
            model,
            origin,
            built_at
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// The one thing about these names that must not drift. Section 11's table
    /// spells all five, and a model that renames itself silently breaks a screen
    /// that was checking which contract it received.
    #[test]
    fn a_model_is_named_the_way_section_11_names_it()
    {
        assert_eq!(TodayModel::MODEL.as_str(), "TodayModel/v1");
        assert_eq!(LabDraftModel::MODEL.as_str(), "LabDraftModel/v1");
        assert_eq!(LabResultModel::MODEL.as_str(), "LabResultModel/v1");
        assert_eq!(StrategiesModel::MODEL.as_str(), "StrategiesModel/v1");
        assert_eq!(
            StrategyDetailModel::MODEL.as_str(),
            "StrategyDetailModel/v1"
        );
    }

    /// A model id is a string on the wire, not an object wrapping one. A screen
    /// reads `model === "TodayModel/v1"`, and `#[serde(transparent)]` is what
    /// makes that true.
    #[test]
    fn a_model_id_crosses_as_a_plain_string()
    {
        assert_eq!(
            serde_json::to_value(TodayModel::MODEL).unwrap(),
            serde_json::json!("TodayModel/v1")
        );
    }

    /// Money is an integer and a ratio is a string, and the wire is where that
    /// stops being a convention and starts being checked. A float here would
    /// pass every Rust test and break reproducibility somewhere else entirely.
    #[test]
    fn money_and_ratios_cross_as_section_7_2_fixes_them()
    {
        assert_eq!(
            serde_json::to_value(Krw(1_284_500)).unwrap(),
            serde_json::json!(1_284_500)
        );
        assert_eq!(
            serde_json::to_value(Ratio("0.0412".to_owned())).unwrap(),
            serde_json::json!("0.0412")
        );
    }

    /// The states are the visual contract's, spelled as the contract spells
    /// them. `connected-read-only` is kebab-case where the rest are snake, and
    /// that is the contract's spelling rather than an oversight — the UI kit
    /// carries it in `data-*` attributes exactly this way.
    #[test]
    fn the_states_are_spelled_the_way_the_visual_contract_spells_them()
    {
        assert_eq!(
            serde_json::to_value(BrokerConnectionState::ConnectedReadOnly).unwrap(),
            serde_json::json!("connected-read-only")
        );
        assert_eq!(
            serde_json::to_value(SectionState::Stale).unwrap(),
            serde_json::json!("stale")
        );
        assert_eq!(
            serde_json::to_value(MarketTone::Up).unwrap(),
            serde_json::json!("up")
        );
        assert_eq!(
            serde_json::to_value(DataOrigin::Synthetic).unwrap(),
            serde_json::json!("synthetic")
        );
    }
}
