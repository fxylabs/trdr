//! `StrategiesModel/v1` and `StrategyDetailModel/v1` — registered strategies.
//!
//! Section 11 builds the list from registrations plus the latest events derived
//! from them, and asks for progress, observed days, reference performance, and
//! rule deviation. It builds the detail from one registration and its events,
//! and asks for the frozen rules, the paper positions, the signal log, and the
//! lineage.
//!
//! # Why performance is not the headline
//!
//! `StrategyCard`'s rule says performance never outranks observation progress
//! and rule deviations, and the model is shaped so a screen has to work to break
//! it. [`StrategySummary`] carries the observation progress and the deviation
//! count as first-class values, and the return is named `paper_return` rather
//! than `return` so that nothing on screen reads as a realised result. A number
//! from ten observed days is not evidence, and the surrounding fields are what
//! say so.
//!
//! # Why a deviation is counted here rather than computed on screen
//!
//! A rule deviation is the gap between what the registered rules said and what
//! the paper run actually did. Deriving it needs both sides, and a screen has
//! only one. Section 11's rule that a screen never assembles a model itself is
//! exactly this case.

use super::lab::StrategyRules;
use super::{Krw, ModelHeader, ModelId, Ratio, SectionState};
use crate::id::ResourceId;
use crate::time::Timestamp;
use serde::{Deserialize, Serialize};

/// The Strategies list screen's model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StrategiesModel
{
    /// Name, version, origin, and build time.
    #[serde(flatten)]
    pub header: ModelHeader,
    /// Every registered strategy, most recently registered first.
    pub strategies: Vec<StrategySummary>,
    /// The state of the list on its own.
    pub state: SectionState
}

impl StrategiesModel
{
    /// This model's name and version, per section 11.
    pub const MODEL: ModelId = ModelId::new("StrategiesModel/v1");
}

/// One row of the strategies list, as `StrategyCard` renders it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StrategySummary
{
    /// Which strategy.
    pub strategy: ResourceId,
    /// What the person called it.
    pub name: String,
    /// One line the person wrote about what it is for.
    pub description: String,
    /// Where the paper validation has got to.
    pub validation: PaperValidationState,
    /// How far through its observation window it is.
    pub observation: ObservationProgress,
    /// What it would have returned so far, on paper.
    pub paper_return: Ratio,
    /// How many times the run did something the registered rules did not say.
    pub deviations: u32
}

/// Where a paper validation has got to.
///
/// The visual contract's `paperValidation` state model, unchanged. `extended`
/// is a real state rather than a variant of running: a window that was extended
/// after the fact is a different claim from one that ran its declared length,
/// and collapsing them is how an out-of-sample result quietly becomes an
/// in-sample one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PaperValidationState
{
    /// Written, not yet registered.
    Draft,
    /// Registered, not yet started.
    Ready,
    /// Observing.
    Running,
    /// Observed for its whole declared window.
    Completed,
    /// Observed past the window it declared, and marked as such.
    Extended,
    /// Abandoned.
    Discarded
}

/// How far through its observation window a validation is.
///
/// `DayProgress`'s anatomy is elapsed, total, and today, and its accessibility
/// line requires elapsed and total to be readable as text rather than only as a
/// bar. All three are here so no screen has to compute one from the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ObservationProgress
{
    /// Market days observed so far.
    pub elapsed_days: u32,
    /// Market days the registration declared.
    pub total_days: u32,
    /// Which day today is, or `None` outside market days.
    pub today_index: Option<u32>
}

/// The strategy detail screen's model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StrategyDetailModel
{
    /// Name, version, origin, and build time.
    #[serde(flatten)]
    pub header: ModelHeader,
    /// Which strategy.
    pub strategy: ResourceId,
    /// What the person called it.
    pub name: String,
    /// The rules a person approved, which cannot be edited.
    pub rules: StrategyRules,
    /// Where the validation has got to.
    pub validation: PaperValidationState,
    /// How far through the window.
    pub observation: ObservationProgress,
    /// What the paper run is holding.
    pub positions: Vec<PaperPosition>,
    /// The state of the positions table on its own.
    pub positions_state: SectionState,
    /// Every signal the run produced, oldest first.
    pub signals: Vec<Signal>,
    /// The state of the signal log on its own.
    pub signals_state: SectionState,
    /// Where this registration came from, and what it replaced.
    pub lineage: StrategyLineage,
    /// What the run did that the rules did not say.
    pub deviations: Vec<RuleDeviation>
}

impl StrategyDetailModel
{
    /// This model's name and version, per section 11.
    pub const MODEL: ModelId = ModelId::new("StrategyDetailModel/v1");
}

/// One position a paper validation is holding.
///
/// Paper, and the type says so in its name. These are not shares anyone owns,
/// and the screen must never render them in the same table as Today's holdings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PaperPosition
{
    /// The ticker.
    pub symbol: ResourceId,
    /// The company's name.
    pub name: String,
    /// How many shares the run would hold.
    pub quantity: i64,
    /// What the rules would have paid.
    pub entry_price: Krw,
    /// What the market says now.
    pub last_price: Krw,
    /// The market date the run entered on.
    pub entered_on: String
}

/// One signal a registered strategy produced.
///
/// `SignalLog`'s rule: every signal explains the matching registered rule. That
/// is what [`Signal::rule`] is for, and it is not optional — a signal that
/// cannot name the rule it came from is a signal nobody can check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Signal
{
    /// What the strategy did, in the finite vocabulary the screen maps to words.
    pub action: String,
    /// Which instrument.
    pub symbol: ResourceId,
    /// The registered rule that matched, as it was written down.
    pub rule: String,
    /// When it happened.
    pub at: Timestamp,
    /// The market date it belongs to.
    pub market_date: String
}

/// Something the paper run did that the registered rules did not say.
///
/// The reason this product exists is that the gap between a strategy as written
/// and a strategy as run is where results stop being trustworthy. A deviation is
/// therefore an object with its own row, not a footnote on a chart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RuleDeviation
{
    /// The market date it happened on.
    pub market_date: String,
    /// Which registered rule was not followed.
    pub rule: String,
    /// The stable code naming the kind of deviation, from section 12's families.
    pub code: String
}

/// Where a registration came from.
///
/// Registrations are append-only (section 7), so a strategy that was changed is
/// a new registration pointing back at the old one rather than an edit. The
/// chain is what lets a person see that a result they are reading came from the
/// third attempt at a rule set, which is exactly the context a single number
/// hides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StrategyLineage
{
    /// When a person approved this registration.
    pub registered_at: Timestamp,
    /// The registration this one replaced, if any.
    pub supersedes: Option<ResourceId>,
    /// The backtest run the registration was approved against.
    pub approved_run: Option<ResourceId>,
    /// Section 7.2's spec hash, frozen at registration.
    pub spec_hash: String
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::query::DataOrigin;

    fn summary(deviations: u32) -> StrategySummary
    {
        StrategySummary {
            strategy: ResourceId::parse("syn-momentum").unwrap(),
            name: "합성 모멘텀".to_owned(),
            description: "20일 이동평균 돌파".to_owned(),
            validation: PaperValidationState::Running,
            observation: ObservationProgress {
                elapsed_days: 9,
                total_days: 60,
                today_index: Some(9)
            },
            paper_return: Ratio("0.0180".to_owned()),
            deviations
        }
    }

    /// The list model must be renderable as an empty list rather than as a
    /// failure. A person with no registered strategies is the normal first case,
    /// not an error condition.
    #[test]
    fn an_empty_list_is_a_state_rather_than_a_failure()
    {
        let model = StrategiesModel {
            header: ModelHeader::new(
                StrategiesModel::MODEL,
                DataOrigin::Synthetic,
                "2026-08-11T00:30:00Z".parse().unwrap()
            ),
            strategies: Vec::new(),
            state: SectionState::Empty
        };

        assert_eq!(model.state, SectionState::Empty);
        assert_eq!(
            serde_json::to_value(&model).unwrap()["model"],
            "StrategiesModel/v1"
        );
    }

    /// The card's rule is a layout constraint, and this is the field that makes
    /// obeying it possible: the deviation count is on the summary, so a card can
    /// show it without a second request.
    #[test]
    fn a_summary_carries_its_deviation_count()
    {
        assert_eq!(summary(2).deviations, 2);
        assert_eq!(
            serde_json::to_value(summary(0)).unwrap()["deviations"],
            serde_json::json!(0)
        );
    }

    /// The return field is named for what it is. A screen that renders a field
    /// called `paper_return` beside an observation count is much harder to
    /// misread than one rendering `return`.
    #[test]
    fn the_return_is_named_paper_return_on_the_wire()
    {
        let json = serde_json::to_value(summary(0)).unwrap();

        assert!(json.get("paper_return").is_some());
        assert!(json.get("return").is_none());
    }

    /// `extended` has to survive as its own value. Folding it into `completed`
    /// is how a window that was stretched after seeing the result stops being
    /// visible as such.
    #[test]
    fn an_extended_window_is_distinguishable_from_a_completed_one()
    {
        assert_ne!(
            serde_json::to_value(PaperValidationState::Extended).unwrap(),
            serde_json::to_value(PaperValidationState::Completed).unwrap()
        );
        assert_eq!(
            serde_json::to_value(PaperValidationState::Extended).unwrap(),
            serde_json::json!("extended")
        );
    }
}
