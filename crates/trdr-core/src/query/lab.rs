//! `LabDraftModel/v1` and `LabResultModel/v1` — the two halves of the Lab.
//!
//! Section 11 gives the draft model a valid strategy draft and its coverage, and
//! asks for rules, period, whether the rules are supported, and what data is
//! missing. It gives the result model a backtest run and its data snapshot, and
//! asks for metrics, the equity curve, the execution assumptions, the hashes,
//! and any warnings.
//!
//! The pair is one module because the second is the first one answered, and the
//! two share [`StrategyRules`]: the rules shown as a draft are the same rules
//! shown frozen beside a result. Splitting them would mean two rule types that
//! have to be kept identical by hand.
//!
//! # Why the hashes are in the model
//!
//! Section 7.2 builds the input hash from the normalised spec, the data snapshot
//! manifest, the execution assumptions, the cost model, and the engine version;
//! the output hash covers the metrics, the trades, the curve, and the input
//! hash. Putting both on screen is what makes the claim checkable rather than
//! asserted — a person can rerun the same backtest and compare, and the
//! registration round-trip in section 9.3 rejects an approval whose hashes moved
//! underneath it. A result rendered without its hashes is a number with no way
//! to tell whether it came from the rules being read.

use super::{Krw, ModelHeader, ModelId, Ratio, SectionState};
use crate::id::ResourceId;
use serde::{Deserialize, Serialize};

/// The Lab draft screen's model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LabDraftModel
{
    /// Name, version, origin, and build time.
    #[serde(flatten)]
    pub header: ModelHeader,
    /// The draft's own identifier.
    pub strategy: ResourceId,
    /// What the person called it.
    pub name: String,
    /// The file the draft was read from, relative to the workspace.
    pub source_path: String,
    /// The normalised rules, in the four sections `RuleGrid` renders.
    pub rules: StrategyRules,
    /// The first market date the draft asks to be validated over.
    pub period_start: String,
    /// The last one.
    pub period_end: String,
    /// Whether trdr can execute these rules at all.
    pub support: SupportState,
    /// What data the period needs and how much of it is present.
    pub coverage: Vec<DataCoverage>,
    /// The state of the coverage section on its own.
    pub coverage_state: SectionState
}

impl LabDraftModel
{
    /// This model's name and version, per section 11.
    pub const MODEL: ModelId = ModelId::new("LabDraftModel/v1");
}

/// Whether trdr will run a draft, and why not when it will not.
///
/// Not a boolean: the difference between "this rule is not supported yet" and
/// "the data for this period is not here" is the difference between a person
/// rewriting a rule and a person running a collector. Section 12's
/// `STRATEGY_*` and `DATA_*` families keep them separate for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum SupportState
{
    /// Every rule is supported and the period is covered.
    Supported,
    /// The rules are fine; some of the data the period needs is missing.
    MissingData,
    /// At least one rule is not something this engine executes.
    UnsupportedRule,
    /// The period itself is not usable — inverted, or in the future.
    InvalidPeriod
}

/// The normalised rules of one strategy, in the sections the grid renders.
///
/// `RuleGrid`'s anatomy fixes these four and their order. A fifth section would
/// be a change to the visual contract, not a field added here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StrategyRules
{
    /// Which instruments the strategy is allowed to consider.
    pub universe: Vec<Rule>,
    /// What makes it buy.
    pub entry: Vec<Rule>,
    /// What makes it sell, and what the trade is assumed to cost.
    pub exit_and_cost: Vec<Rule>,
    /// How long it is to be validated for, and against what.
    pub validation_window: Vec<Rule>,
    /// Whether these rules can still be edited.
    pub section: RuleSection
}

/// Whether a rule grid is still a draft or has been frozen by a registration.
///
/// `RuleGrid`'s rule: frozen rules cannot show an edit affordance. This value is
/// what the screen reads to obey it, and a screen deciding for itself from
/// context would eventually decide wrong on the screen where it matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum RuleSection
{
    /// Editable.
    Draft,
    /// Fixed by a registration a person approved.
    Frozen
}

/// One normalised rule, as a label and the value it was normalised to.
///
/// Both sides are text because the grid renders text, and because a rule's value
/// is not one type: a threshold is a number, a universe is a list, a window is a
/// duration. Normalisation happens before this type, not in the screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Rule
{
    /// What the rule is called, in the vocabulary the person wrote it in.
    pub label: String,
    /// What it normalised to.
    pub value: String
}

/// How much of one source's data the requested period actually has.
///
/// `CoverageCard`'s rule says the progress track is supplementary and never the
/// only value, which is why `covered` and `total` are both here as numbers a
/// screen can render as text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DataCoverage
{
    /// Which source, spelled as section 7's `Source` namespaces it.
    pub source: ResourceId,
    /// How many market days the period asks for.
    pub total_days: u32,
    /// How many of them are present.
    pub covered_days: u32,
    /// The ranges that are not, so the screen can say which rather than how many.
    pub gaps: Vec<CoverageGap>
}

/// One stretch of missing data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CoverageGap
{
    /// First missing market date, as `YYYY-MM-DD`.
    pub from: String,
    /// Last missing market date.
    pub to: String
}

/// The Lab result screen's model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LabResultModel
{
    /// Name, version, origin, and build time.
    #[serde(flatten)]
    pub header: ModelHeader,
    /// Which run this is.
    pub run: ResourceId,
    /// Which strategy it ran.
    pub strategy: ResourceId,
    /// The rules it ran, frozen at the moment it ran them.
    pub rules: StrategyRules,
    /// What the run produced.
    pub metrics: BacktestMetrics,
    /// The equity curve, in order.
    pub curve: Vec<CurvePoint>,
    /// What the engine assumed while executing.
    pub assumptions: BacktestAssumptions,
    /// Section 7.2's input hash, as lowercase hex.
    pub input_hash: String,
    /// Section 7.2's output hash.
    pub output_hash: String,
    /// Everything that should temper reading the metrics.
    pub warnings: Vec<BacktestWarning>,
    /// trdr's own reading of the result.
    pub verdict: Verdict
}

impl LabResultModel
{
    /// This model's name and version, per section 11.
    pub const MODEL: ModelId = ModelId::new("LabResultModel/v1");
}

/// What a backtest run produced.
///
/// Ratios rather than floats, for section 7.2's reason: these numbers go into
/// the output hash, and a hash over a binary float is a hash that disagrees with
/// itself across machines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BacktestMetrics
{
    /// Total return over the period.
    pub total_return: Ratio,
    /// The same, annualised.
    pub annualised_return: Ratio,
    /// The worst peak-to-trough fall.
    pub max_drawdown: Ratio,
    /// Return per unit of volatility.
    pub sharpe: Ratio,
    /// How many trades it took.
    pub trades: u32,
    /// What share of them made money.
    pub win_rate: Ratio
}

/// One point on the equity curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CurvePoint
{
    /// The market date, as `YYYY-MM-DD`.
    pub date: String,
    /// What the portfolio was worth at the close of it.
    pub equity: Krw
}

/// What the engine assumed while executing a run.
///
/// On screen because they are half of what makes a number mean anything. A
/// return computed with no slippage and same-close fills is a different claim
/// from the same number computed with next-open fills, and a result that hides
/// which one it was is the kind of honest-looking output this product exists to
/// refuse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BacktestAssumptions
{
    /// When an order is assumed to fill.
    pub fill: String,
    /// The commission model, as it was written down.
    pub commission: String,
    /// The slippage model.
    pub slippage: String,
    /// What was assumed about taxes.
    pub tax: String,
    /// Which engine version ran it — part of the input hash.
    pub engine_version: String
}

/// Something about a run that should be read alongside its metrics.
///
/// Warnings carry a code from section 12's stable families rather than a
/// sentence, because section 11 forbids a model from writing user-facing wording
/// and section 3.1 keeps that rule in the runtime too. The screen maps the code
/// to a localised string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BacktestWarning
{
    /// The stable code, for example `DATA_INCOMPLETE`.
    pub code: String,
    /// Safe parameters the wording may interpolate. No raw payloads.
    pub params: Vec<String>
}

/// trdr's reading of a result, as a finite value rather than a sentence.
///
/// Section 11 is explicit that no language model writes a headline, a verdict,
/// or an error text. This enum is the whole vocabulary, and the wording for each
/// value lives in the screen's localisation table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict
{
    /// The run is complete and its assumptions hold.
    Sound,
    /// Complete, but something in the warnings limits what it shows.
    Qualified,
    /// The data behind it does not support reading the metrics.
    Unsupported
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::query::DataOrigin;

    fn header(model: ModelId) -> ModelHeader
    {
        ModelHeader::new(
            model,
            DataOrigin::Synthetic,
            "2026-08-11T00:30:00Z".parse().unwrap()
        )
    }

    fn rules(section: RuleSection) -> StrategyRules
    {
        StrategyRules {
            universe: vec![Rule {
                label: "market".to_owned(),
                value: "SYN".to_owned()
            }],
            entry: Vec::new(),
            exit_and_cost: Vec::new(),
            validation_window: Vec::new(),
            section
        }
    }

    /// A frozen grid and a draft grid are the same type, and the difference has
    /// to survive the crossing as the contract's own spelling — the UI kit reads
    /// it straight into a `data-*` attribute.
    #[test]
    fn a_frozen_rule_set_says_so_on_the_wire()
    {
        assert_eq!(
            serde_json::to_value(rules(RuleSection::Frozen)).unwrap()["section"],
            "frozen"
        );
        assert_eq!(
            serde_json::to_value(rules(RuleSection::Draft)).unwrap()["section"],
            "draft"
        );
    }

    /// The reason both hashes are on the model at all. If a result could reach a
    /// screen without them, the screen would have nothing to show for the claim
    /// that the run is reproducible.
    #[test]
    fn a_result_carries_both_hashes_section_7_2_defines()
    {
        let result = LabResultModel {
            header: header(LabResultModel::MODEL),
            run: ResourceId::parse("run-1").unwrap(),
            strategy: ResourceId::parse("syn-momentum").unwrap(),
            rules: rules(RuleSection::Frozen),
            metrics: BacktestMetrics {
                total_return: Ratio("0.0412".to_owned()),
                annualised_return: Ratio("0.1030".to_owned()),
                max_drawdown: Ratio("0.0870".to_owned()),
                sharpe: Ratio("0.6200".to_owned()),
                trades: 14,
                win_rate: Ratio("0.5710".to_owned())
            },
            curve: Vec::new(),
            assumptions: BacktestAssumptions {
                fill: "next-open".to_owned(),
                commission: "0.015%".to_owned(),
                slippage: "1 tick".to_owned(),
                tax: "0.20% on sale".to_owned(),
                engine_version: "0.0.0".to_owned()
            },
            input_hash: "a".repeat(64),
            output_hash: "b".repeat(64),
            warnings: Vec::new(),
            verdict: Verdict::Sound
        };
        let json = serde_json::to_value(&result).unwrap();

        assert_eq!(json["input_hash"].as_str().unwrap().len(), 64);
        assert_eq!(json["output_hash"].as_str().unwrap().len(), 64);
        assert_eq!(json["model"], "LabResultModel/v1");
    }

    /// A warning is a code and safe parameters, never a sentence. Section 11
    /// keeps wording out of the model, and this is the field a sentence would
    /// most naturally sneak into.
    #[test]
    fn a_warning_carries_a_code_rather_than_wording()
    {
        let warning = BacktestWarning {
            code: "DATA_INCOMPLETE".to_owned(),
            params: vec!["2026-08-04".to_owned()]
        };
        let json = serde_json::to_value(&warning).unwrap();

        assert_eq!(json["code"], "DATA_INCOMPLETE");
        assert!(json.get("message").is_none());
    }

    /// Coverage reports which days are missing, not only how many. "3 days
    /// missing" cannot be acted on; a date range can.
    #[test]
    fn coverage_names_the_gaps_rather_than_counting_them()
    {
        let coverage = DataCoverage {
            source: ResourceId::parse("user.synthetic").unwrap(),
            total_days: 11,
            covered_days: 9,
            gaps: vec![CoverageGap {
                from: "2026-08-06".to_owned(),
                to: "2026-08-07".to_owned()
            }]
        };

        assert_eq!(coverage.total_days - coverage.covered_days, 2);
        assert_eq!(coverage.gaps.len(), 1);
    }
}
