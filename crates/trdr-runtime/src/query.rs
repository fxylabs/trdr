//! The query service (section 11): one versioned model per screen.
//!
//! Section 11 puts the assembly here rather than in a screen. A screen asks for
//! `TodayModel/v1` and renders what it gets; it never joins an account snapshot
//! to a disclosure feed itself, and it never decides what counts as stale. Those
//! are decisions with one right answer per product, not per screen, and this is
//! where the one answer lives.
//!
//! # Why the milestone's data is a checked-in file
//!
//! Milestone M2 is the walking skeleton: every screen, the terminal, and the CLI
//! running end to end on synthetic data, before a single real broker call. The
//! source for that is [`SYNTHETIC_DOMAIN`], a fixture compiled into the binary.
//!
//! Compiled in, not read from disk, and that is the honest choice rather than
//! the convenient one. A packaged `.app` has no repository beside it, so a
//! fixture loaded by path would work in a checkout and fail in the build the
//! visual gate actually inspects — and the failure would arrive at the moment
//! the evidence was being collected. Section 13's seam is preserved a level up:
//! [`QueryService`] is a trait, [`SyntheticQueries`] is one implementation of
//! it, and the database-backed one that milestone M3 brings is another.
//!
//! # Every model says it is synthetic
//!
//! M2 requires the app to state at all times that what is shown is not an
//! account and not a market. This module never constructs a model with any
//! origin other than [`DataOrigin::Synthetic`], and
//! [`tests::every_model_declares_itself_synthetic`] is what holds that. The
//! screen cannot opt out, because the field arrives with the data.

use crate::clock::Clock;
use serde::Deserialize;
use trdr_core::error::{ErrorCode, ErrorEnvelope};
use trdr_core::id::ResourceId;
use trdr_core::query::{
    AccountSummary, BacktestAssumptions, BacktestMetrics, BacktestWarning, BrokerConnectionState,
    CoverageGap, CurvePoint, DataCoverage, DataOrigin, Holding, Krw, LabDraftModel, LabResultModel,
    MarketTone, ModelHeader, ObservationProgress, PaperPosition, PaperValidationState, Ratio, Rule,
    RuleDeviation, RuleSection, SectionState, Signal, StrategiesModel, StrategyDetailModel,
    StrategyLineage, StrategyRules, StrategySummary, SupportState, TodayEvent, TodayEventKind,
    TodayModel, Verdict
};
use trdr_core::time::Timestamp;

/// The synthetic domain objects milestone M2 runs on.
///
/// One file, compiled in. It is also the file the CLI reads, which is what makes
/// M2's requirement that the UI and the CLI see the same domain object true by
/// construction rather than by two loaders agreeing.
pub const SYNTHETIC_DOMAIN: &str =
    include_str!("../../../fixtures/synthetic/domain-v1/domain.json");

/// The schema version [`SYNTHETIC_DOMAIN`] must declare.
const SYNTHETIC_SCHEMA: &str = "trdr.synthetic-domain/v1";

/// What a screen can ask for.
///
/// Five methods, one per row of section 11's table. A sixth would mean a sixth
/// screen, and that is a change to the design document before it is a change
/// here.
pub trait QueryService: Send + Sync
{
    /// What happened since the person last looked.
    fn today(&self) -> Result<TodayModel, ErrorEnvelope>;

    /// The strategy currently being drafted, and whether its period is covered.
    fn lab_draft(&self) -> Result<LabDraftModel, ErrorEnvelope>;

    /// The most recent backtest result.
    fn lab_result(&self) -> Result<LabResultModel, ErrorEnvelope>;

    /// Every registered strategy.
    fn strategies(&self) -> Result<StrategiesModel, ErrorEnvelope>;

    /// One registered strategy in full.
    fn strategy(&self, strategy: &ResourceId) -> Result<StrategyDetailModel, ErrorEnvelope>;
}

/// The query service milestone M2 ships: models built from the fixture.
pub struct SyntheticQueries<C: Clock>
{
    domain: Domain,
    clock: C
}

impl<C: Clock> SyntheticQueries<C>
{
    /// Parses the compiled-in fixture, or says why it could not be read.
    ///
    /// Fallible rather than panicking, even though the input is a file this
    /// repository controls: the same parser reads it in a test and in a shipped
    /// binary, and a binary that aborts at start-up because a fixture drifted is
    /// worse than one that shows an error a person can report.
    pub fn load(clock: C) -> Result<Self, ErrorEnvelope>
    {
        let domain: Domain = serde_json::from_str(SYNTHETIC_DOMAIN)
            .map_err(|_| ErrorEnvelope::new(ErrorCode::DataIntegrity))?;

        if domain.schema_version != SYNTHETIC_SCHEMA
        {
            return Err(ErrorEnvelope::new(ErrorCode::DataUnsupported));
        }

        Ok(Self { domain, clock })
    }

    /// A header naming the model, marked synthetic, stamped now.
    fn header(&self, model: trdr_core::query::ModelId) -> ModelHeader
    {
        ModelHeader::new(model, DataOrigin::Synthetic, self.clock.now())
    }

    /// The registration with this id, or a `DATA_INCOMPLETE` failure naming it.
    fn registration(&self, strategy: &ResourceId) -> Result<&Registration, ErrorEnvelope>
    {
        self.domain
            .registrations
            .iter()
            .find(|entry| &entry.strategy == strategy)
            .ok_or_else(|| ErrorEnvelope::new(ErrorCode::DataIncomplete))
    }
}

impl<C: Clock> QueryService for SyntheticQueries<C>
{
    fn today(&self) -> Result<TodayModel, ErrorEnvelope>
    {
        Ok(TodayModel {
            header: self.header(TodayModel::MODEL),
            account: self.domain.account.summary(),
            holdings: self.domain.holdings.iter().map(Position::holding).collect(),
            holdings_state: state_of(self.domain.holdings.len()),
            events: self.domain.events(),
            events_state: state_of(
                self.domain.disclosures.len() + self.domain.validation_events.len()
            )
        })
    }

    fn lab_draft(&self) -> Result<LabDraftModel, ErrorEnvelope>
    {
        let draft = &self.domain.draft;

        Ok(LabDraftModel {
            header: self.header(LabDraftModel::MODEL),
            strategy: draft.strategy.clone(),
            name: draft.name.clone(),
            source_path: draft.source_path.clone(),
            rules: draft.rules.rules(),
            period_start: draft.period_start.clone(),
            period_end: draft.period_end.clone(),
            support: draft.support,
            coverage: draft.coverage.iter().map(Coverage::coverage).collect(),
            coverage_state: coverage_state(&draft.coverage)
        })
    }

    fn lab_result(&self) -> Result<LabResultModel, ErrorEnvelope>
    {
        let run = &self.domain.run;

        Ok(LabResultModel {
            header: self.header(LabResultModel::MODEL),
            run: run.run.clone(),
            strategy: run.strategy.clone(),
            rules: self.domain.draft.rules.frozen(),
            metrics: run.metrics.metrics(),
            curve: run.curve.iter().map(Point::point).collect(),
            assumptions: run.assumptions.assumptions(),
            input_hash: run.input_hash.clone(),
            output_hash: run.output_hash.clone(),
            warnings: run.warnings.iter().map(Warning::warning).collect(),
            verdict: run.verdict
        })
    }

    fn strategies(&self) -> Result<StrategiesModel, ErrorEnvelope>
    {
        Ok(StrategiesModel {
            header: self.header(StrategiesModel::MODEL),
            strategies: self
                .domain
                .registrations
                .iter()
                .map(Registration::summary)
                .collect(),
            state: state_of(self.domain.registrations.len())
        })
    }

    fn strategy(&self, strategy: &ResourceId) -> Result<StrategyDetailModel, ErrorEnvelope>
    {
        let registration = self.registration(strategy)?;

        Ok(StrategyDetailModel {
            header: self.header(StrategyDetailModel::MODEL),
            strategy: registration.strategy.clone(),
            name: registration.name.clone(),
            rules: registration.rules.frozen(),
            validation: registration.validation,
            observation: registration.observation.progress(),
            positions: registration
                .positions
                .iter()
                .map(PaperEntry::position)
                .collect(),
            positions_state: state_of(registration.positions.len()),
            signals: registration.signals.iter().map(SignalRow::signal).collect(),
            signals_state: state_of(registration.signals.len()),
            lineage: registration.lineage.lineage(),
            deviations: registration
                .deviations
                .iter()
                .map(DeviationRow::deviation)
                .collect()
        })
    }
}

/// Nothing to show is [`SectionState::Empty`], not an error and not a blank.
///
/// The distinction reaches the screen: `empty` renders an `EmptyState` that
/// explains the next step, `error` renders an alert. Collapsing them turns "you
/// have not registered a strategy yet" into "something went wrong".
fn state_of(count: usize) -> SectionState
{
    if count == 0
    {
        SectionState::Empty
    }
    else
    {
        SectionState::Ready
    }
}

/// Coverage with a gap in it is stale rather than ready.
///
/// The visual contract's `CoverageCard` has an `incomplete` state and the
/// section states have `stale`; both mean the same thing to a person about to
/// trust a backtest — some of the period is not there. Reporting `ready` over a
/// gap is how a result gets read as covering a period it did not.
fn coverage_state(coverage: &[Coverage]) -> SectionState
{
    if coverage.is_empty()
    {
        SectionState::Empty
    }
    else if coverage.iter().any(|entry| !entry.gaps.is_empty())
    {
        SectionState::Stale
    }
    else
    {
        SectionState::Ready
    }
}

/// The fixture's own shape.
///
/// Separate types from the query models on purpose. A fixture that deserialised
/// straight into the models would make every model change a fixture change, and
/// would let a model field exist only because the fixture happened to have one.
/// These are section 7's domain entities; the models are section 11's views of
/// them, and the mapping between is what this module does.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Domain
{
    schema_version: String,
    #[allow(dead_code)]
    notice: String,
    #[allow(dead_code)]
    built_at: Timestamp,
    account: Account,
    holdings: Vec<Position>,
    disclosures: Vec<DisclosureRow>,
    validation_events: Vec<ValidationRow>,
    draft: Draft,
    run: Run,
    registrations: Vec<Registration>
}

impl Domain
{
    /// Today's events, newest first.
    ///
    /// Disclosures and validation progress are two entities in section 7 and one
    /// list on screen, because a person reading Today is asking what happened,
    /// not which table it was stored in. The order is chronological because that
    /// is the only ordering that does not imply a judgement about importance.
    fn events(&self) -> Vec<TodayEvent>
    {
        let mut events: Vec<TodayEvent> = self
            .disclosures
            .iter()
            .map(DisclosureRow::event)
            .chain(self.validation_events.iter().map(ValidationRow::event))
            .collect();

        events.sort_by(|left, right| right.at.as_str().cmp(left.at.as_str()));
        events
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Account
{
    as_of: Timestamp,
    connection: BrokerConnectionState,
    total_value: i64,
    cash: i64,
    day_change: i64,
    day_change_ratio: String,
    day_tone: MarketTone
}

impl Account
{
    fn summary(&self) -> AccountSummary
    {
        AccountSummary {
            total_value: Krw(self.total_value),
            cash: Krw(self.cash),
            day_change: Krw(self.day_change),
            day_change_ratio: Ratio(self.day_change_ratio.clone()),
            day_tone: self.day_tone,
            as_of: self.as_of.clone(),
            state: SectionState::Ready,
            connection: self.connection
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Position
{
    symbol: ResourceId,
    name: String,
    quantity: i64,
    average_price: i64,
    last_price: i64,
    market_value: i64,
    unrealized: i64,
    unrealized_ratio: String,
    tone: MarketTone
}

impl Position
{
    fn holding(&self) -> Holding
    {
        Holding {
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            quantity: self.quantity,
            average_price: Krw(self.average_price),
            last_price: Krw(self.last_price),
            market_value: Krw(self.market_value),
            unrealized: Krw(self.unrealized),
            unrealized_ratio: Ratio(self.unrealized_ratio.clone()),
            tone: self.tone
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisclosureRow
{
    #[allow(dead_code)]
    receipt: ResourceId,
    symbol: ResourceId,
    title: String,
    #[allow(dead_code)]
    published_at: Timestamp,
    available_at: Timestamp
}

impl DisclosureRow
{
    /// A disclosure as Today shows it.
    ///
    /// The summary is a code, not a sentence: section 11 forbids this layer from
    /// writing user-facing wording, and section 3.1 repeats the rule for the
    /// whole runtime. The screen's localisation table turns
    /// `disclosure-for-holding` into Korean.
    fn event(&self) -> TodayEvent
    {
        TodayEvent {
            kind: TodayEventKind::Disclosure,
            title: self.title.clone(),
            summary: "disclosure-for-holding".to_owned(),
            at: self.available_at.clone(),
            subject: Some(self.symbol.clone())
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidationRow
{
    registration: ResourceId,
    market_date: String,
    summary: String,
    recorded_at: Timestamp
}

impl ValidationRow
{
    fn event(&self) -> TodayEvent
    {
        TodayEvent {
            kind: TodayEventKind::ValidationProgress,
            title: self.market_date.clone(),
            summary: self.summary.clone(),
            at: self.recorded_at.clone(),
            subject: Some(self.registration.clone())
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft
{
    strategy: ResourceId,
    name: String,
    source_path: String,
    period_start: String,
    period_end: String,
    support: SupportState,
    rules: RuleSet,
    coverage: Vec<Coverage>
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleSet
{
    universe: Vec<RuleRow>,
    entry: Vec<RuleRow>,
    exit_and_cost: Vec<RuleRow>,
    validation_window: Vec<RuleRow>,
    section: RuleSection
}

impl RuleSet
{
    /// The rules as the fixture declares them.
    fn rules(&self) -> StrategyRules
    {
        self.with_section(self.section)
    }

    /// The same rules, frozen.
    ///
    /// A result and a registration both show rules that cannot be edited, and
    /// `RuleGrid`'s rule is that frozen rules show no edit affordance. Rather
    /// than trusting a fixture to have spelled `frozen` in every place it
    /// matters, the two callers that require it say so here.
    fn frozen(&self) -> StrategyRules
    {
        self.with_section(RuleSection::Frozen)
    }

    fn with_section(&self, section: RuleSection) -> StrategyRules
    {
        StrategyRules {
            universe: self.universe.iter().map(RuleRow::rule).collect(),
            entry: self.entry.iter().map(RuleRow::rule).collect(),
            exit_and_cost: self.exit_and_cost.iter().map(RuleRow::rule).collect(),
            validation_window: self.validation_window.iter().map(RuleRow::rule).collect(),
            section
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleRow
{
    label: String,
    value: String
}

impl RuleRow
{
    fn rule(&self) -> Rule
    {
        Rule {
            label: self.label.clone(),
            value: self.value.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coverage
{
    source: ResourceId,
    total_days: u32,
    covered_days: u32,
    gaps: Vec<Gap>
}

impl Coverage
{
    fn coverage(&self) -> DataCoverage
    {
        DataCoverage {
            source: self.source.clone(),
            total_days: self.total_days,
            covered_days: self.covered_days,
            gaps: self.gaps.iter().map(Gap::gap).collect()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gap
{
    from: String,
    to: String
}

impl Gap
{
    fn gap(&self) -> CoverageGap
    {
        CoverageGap {
            from: self.from.clone(),
            to: self.to.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Run
{
    run: ResourceId,
    strategy: ResourceId,
    input_hash: String,
    output_hash: String,
    verdict: Verdict,
    metrics: Metrics,
    assumptions: Assumptions,
    warnings: Vec<Warning>,
    curve: Vec<Point>
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metrics
{
    total_return: String,
    annualised_return: String,
    max_drawdown: String,
    sharpe: String,
    trades: u32,
    win_rate: String
}

impl Metrics
{
    fn metrics(&self) -> BacktestMetrics
    {
        BacktestMetrics {
            total_return: Ratio(self.total_return.clone()),
            annualised_return: Ratio(self.annualised_return.clone()),
            max_drawdown: Ratio(self.max_drawdown.clone()),
            sharpe: Ratio(self.sharpe.clone()),
            trades: self.trades,
            win_rate: Ratio(self.win_rate.clone())
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assumptions
{
    fill: String,
    commission: String,
    slippage: String,
    tax: String,
    engine_version: String
}

impl Assumptions
{
    fn assumptions(&self) -> BacktestAssumptions
    {
        BacktestAssumptions {
            fill: self.fill.clone(),
            commission: self.commission.clone(),
            slippage: self.slippage.clone(),
            tax: self.tax.clone(),
            engine_version: self.engine_version.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Warning
{
    code: String,
    params: Vec<String>
}

impl Warning
{
    fn warning(&self) -> BacktestWarning
    {
        BacktestWarning {
            code: self.code.clone(),
            params: self.params.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Point
{
    date: String,
    equity: i64
}

impl Point
{
    fn point(&self) -> CurvePoint
    {
        CurvePoint {
            date: self.date.clone(),
            equity: Krw(self.equity)
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration
{
    strategy: ResourceId,
    name: String,
    description: String,
    validation: PaperValidationState,
    paper_return: String,
    observation: Observation,
    lineage: Lineage,
    rules: RuleSet,
    positions: Vec<PaperEntry>,
    signals: Vec<SignalRow>,
    deviations: Vec<DeviationRow>
}

impl Registration
{
    fn summary(&self) -> StrategySummary
    {
        StrategySummary {
            strategy: self.strategy.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            validation: self.validation,
            observation: self.observation.progress(),
            paper_return: Ratio(self.paper_return.clone()),
            deviations: self.deviations.len() as u32
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation
{
    elapsed_days: u32,
    total_days: u32,
    today_index: Option<u32>
}

impl Observation
{
    fn progress(&self) -> ObservationProgress
    {
        ObservationProgress {
            elapsed_days: self.elapsed_days,
            total_days: self.total_days,
            today_index: self.today_index
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lineage
{
    registered_at: Timestamp,
    supersedes: Option<ResourceId>,
    approved_run: Option<ResourceId>,
    spec_hash: String
}

impl Lineage
{
    fn lineage(&self) -> StrategyLineage
    {
        StrategyLineage {
            registered_at: self.registered_at.clone(),
            supersedes: self.supersedes.clone(),
            approved_run: self.approved_run.clone(),
            spec_hash: self.spec_hash.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaperEntry
{
    symbol: ResourceId,
    name: String,
    quantity: i64,
    entry_price: i64,
    last_price: i64,
    entered_on: String
}

impl PaperEntry
{
    fn position(&self) -> PaperPosition
    {
        PaperPosition {
            symbol: self.symbol.clone(),
            name: self.name.clone(),
            quantity: self.quantity,
            entry_price: Krw(self.entry_price),
            last_price: Krw(self.last_price),
            entered_on: self.entered_on.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignalRow
{
    action: String,
    symbol: ResourceId,
    rule: String,
    at: Timestamp,
    market_date: String
}

impl SignalRow
{
    fn signal(&self) -> Signal
    {
        Signal {
            action: self.action.clone(),
            symbol: self.symbol.clone(),
            rule: self.rule.clone(),
            at: self.at.clone(),
            market_date: self.market_date.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviationRow
{
    market_date: String,
    rule: String,
    code: String
}

impl DeviationRow
{
    fn deviation(&self) -> RuleDeviation
    {
        RuleDeviation {
            market_date: self.market_date.clone(),
            rule: self.rule.clone(),
            code: self.code.clone()
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::test_support::FixedClock;

    fn service() -> SyntheticQueries<FixedClock>
    {
        SyntheticQueries::load(FixedClock::at("2026-08-11T00:30:00Z")).unwrap()
    }

    /// The fixture is compiled in, so a change that makes it unparseable is a
    /// change that fails here rather than at the first screen a person opens.
    #[test]
    fn the_compiled_in_fixture_parses()
    {
        let service = service();

        assert_eq!(service.domain.schema_version, SYNTHETIC_SCHEMA);
        assert!(!service.domain.holdings.is_empty());
        assert!(!service.domain.registrations.is_empty());
    }

    /// The promise milestone M2 makes to the person looking at the screen. Every
    /// model, without exception, and there is no code path in this module that
    /// produces any other origin.
    #[test]
    fn every_model_declares_itself_synthetic()
    {
        let service = service();
        let strategy = ResourceId::parse("syn-meanrev").unwrap();

        assert_eq!(
            service.today().unwrap().header.origin,
            DataOrigin::Synthetic
        );
        assert_eq!(
            service.lab_draft().unwrap().header.origin,
            DataOrigin::Synthetic
        );
        assert_eq!(
            service.lab_result().unwrap().header.origin,
            DataOrigin::Synthetic
        );
        assert_eq!(
            service.strategies().unwrap().header.origin,
            DataOrigin::Synthetic
        );
        assert_eq!(
            service.strategy(&strategy).unwrap().header.origin,
            DataOrigin::Synthetic
        );
    }

    /// Each model names itself, so a screen that received the wrong one can say
    /// so instead of rendering fields that happen to line up.
    #[test]
    fn each_model_names_the_contract_it_answers()
    {
        let service = service();

        assert_eq!(service.today().unwrap().header.model, TodayModel::MODEL);
        assert_eq!(
            service.lab_draft().unwrap().header.model,
            LabDraftModel::MODEL
        );
        assert_eq!(
            service.lab_result().unwrap().header.model,
            LabResultModel::MODEL
        );
        assert_eq!(
            service.strategies().unwrap().header.model,
            StrategiesModel::MODEL
        );
    }

    /// Today's list is one chronological sequence rather than two tables shown
    /// in the order they were stored.
    #[test]
    fn todays_events_arrive_newest_first()
    {
        let events = service().today().unwrap().events;

        assert!(events.len() >= 2);

        for pair in events.windows(2)
        {
            assert!(
                pair[0].at.as_str() >= pair[1].at.as_str(),
                "{} came before {}",
                pair[0].at,
                pair[1].at
            );
        }
    }

    /// A gap in the coverage has to reach the screen as `stale`. The fixture
    /// carries gaps precisely so this path is exercised rather than assumed.
    #[test]
    fn a_period_with_a_gap_is_reported_stale_rather_than_ready()
    {
        let draft = service().lab_draft().unwrap();

        assert_eq!(draft.coverage_state, SectionState::Stale);
        assert!(draft.coverage.iter().any(|entry| !entry.gaps.is_empty()));
        assert_eq!(draft.support, SupportState::MissingData);
    }

    /// A draft is editable and a result's rules are not, and the second must not
    /// depend on the fixture having said so.
    #[test]
    fn a_result_shows_its_rules_frozen_whatever_the_fixture_said()
    {
        assert_eq!(
            service().lab_draft().unwrap().rules.section,
            RuleSection::Draft
        );
        assert_eq!(
            service().lab_result().unwrap().rules.section,
            RuleSection::Frozen
        );
        assert_eq!(
            service()
                .strategy(&ResourceId::parse("syn-meanrev").unwrap())
                .unwrap()
                .rules
                .section,
            RuleSection::Frozen
        );
    }

    /// The count on the list and the rows on the detail are the same fact, and a
    /// list that under-reports deviations is exactly the failure `StrategyCard`'s
    /// rule exists to prevent.
    #[test]
    fn the_list_and_the_detail_agree_about_deviations()
    {
        let service = service();
        let summaries = service.strategies().unwrap().strategies;

        for summary in summaries
        {
            let detail = service.strategy(&summary.strategy).unwrap();

            assert_eq!(summary.deviations as usize, detail.deviations.len());
            assert_eq!(summary.observation, detail.observation);
            assert_eq!(summary.validation, detail.validation);
        }
    }

    /// Asking for a strategy that is not there is a stated failure, not an empty
    /// detail screen that looks like a strategy with nothing in it.
    #[test]
    fn an_unknown_strategy_is_refused_rather_than_answered_empty()
    {
        let missing = ResourceId::parse("not-a-strategy").unwrap();
        let error = service().strategy(&missing).unwrap_err();

        assert_eq!(error.code, ErrorCode::DataIncomplete);
    }

    /// A model built with nothing in it says `empty`, and the distinction from
    /// `error` is what the screen branches on.
    #[test]
    fn nothing_to_show_is_empty_rather_than_error()
    {
        assert_eq!(state_of(0), SectionState::Empty);
        assert_eq!(state_of(1), SectionState::Ready);
        assert_eq!(coverage_state(&[]), SectionState::Empty);
    }

    /// The fixture's own account block carries no account number, and this is
    /// the check that keeps it from acquiring one. Section 5.2 forbids the raw
    /// account number leaving the broker boundary at all.
    #[test]
    fn the_fixture_carries_no_account_identifier()
    {
        let raw: serde_json::Value = serde_json::from_str(SYNTHETIC_DOMAIN).unwrap();
        let account = raw["account"].as_object().unwrap();

        for field in account.keys()
        {
            assert!(
                !field.contains("account") && !field.contains("number"),
                "{field} looks like an account identifier"
            );
        }
    }
}
