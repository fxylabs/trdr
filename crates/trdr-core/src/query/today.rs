//! `TodayModel/v1` — what happened since the person last looked.
//!
//! Section 11 gives this model three inputs: the latest account snapshot, the
//! disclosures belonging to what is held, and validation events. It gives it
//! four outputs: the total, the holdings, today's events, and the stale state.
//! The type below is that table and nothing beyond it.
//!
//! The screen it feeds composes `TopBar`, `Metric`, `DataTable` with
//! `StockCell`, `EventList`, and `Button` — the visual contract's `TodayView`
//! recipe. Every field here exists because one of those needs it.
//!
//! # Why each section carries its own state
//!
//! An account snapshot, a disclosure feed, and a validation log come from three
//! places and fail independently. Folding them into one screen-wide state means
//! either showing a stale banner over data that is current, or showing current
//! data over a feed that stopped updating an hour ago. Both are lies, and the
//! second one is the expensive kind, so each section says for itself what it is.

use super::{
    BrokerConnectionState, DataOrigin, Krw, MarketTone, ModelHeader, ModelId, Ratio, SectionState
};
use crate::id::ResourceId;
use crate::time::Timestamp;
use serde::{Deserialize, Serialize};

/// The Today screen's model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TodayModel
{
    /// Name, version, origin, and build time.
    #[serde(flatten)]
    pub header: ModelHeader,
    /// The account totals, and how fresh they are.
    pub account: AccountSummary,
    /// What is held, most valuable first.
    pub holdings: Vec<Holding>,
    /// The state of the holdings table on its own.
    pub holdings_state: SectionState,
    /// Disclosures, strategy signals, and validation progress for today.
    pub events: Vec<TodayEvent>,
    /// The state of the event list on its own.
    pub events_state: SectionState
}

impl TodayModel
{
    /// This model's name and version, per section 11.
    pub const MODEL: ModelId = ModelId::new("TodayModel/v1");

    /// An empty model, for the case where a workspace has nothing in it yet.
    ///
    /// Not a convenience: a screen with no account connected still has to render
    /// something, and the honest something is an account of zero with both
    /// sections reporting [`SectionState::Empty`] rather than a model that
    /// failed to build.
    pub fn empty(origin: DataOrigin, built_at: Timestamp) -> Self
    {
        Self {
            header: ModelHeader::new(Self::MODEL, origin, built_at.clone()),
            account: AccountSummary::empty(built_at),
            holdings: Vec::new(),
            holdings_state: SectionState::Empty,
            events: Vec::new(),
            events_state: SectionState::Empty
        }
    }
}

/// The account totals, as of the last snapshot.
///
/// `AccountSnapshot` in section 7 keeps the latest normalised account and no
/// raw response, no token, and no account number. This is the part of it a
/// screen is allowed to see, and the omissions are deliberate: there is no
/// account number field here, and there is no place to add one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AccountSummary
{
    /// Everything the account is worth, cash included.
    pub total_value: Krw,
    /// The part of it that is cash.
    pub cash: Krw,
    /// How much the total moved today.
    pub day_change: Krw,
    /// The same move as a ratio of yesterday's total.
    pub day_change_ratio: Ratio,
    /// Which way it moved, so no screen has to derive it from a sign.
    pub day_tone: MarketTone,
    /// When the broker last answered.
    pub as_of: Timestamp,
    /// Whether this section is current, old, or unavailable.
    pub state: SectionState,
    /// The state of the connection the number came over.
    pub connection: BrokerConnectionState
}

impl AccountSummary
{
    /// The summary for an account that holds nothing and is not connected.
    fn empty(as_of: Timestamp) -> Self
    {
        Self {
            total_value: Krw(0),
            cash: Krw(0),
            day_change: Krw(0),
            day_change_ratio: Ratio("0.0000".to_owned()),
            day_tone: MarketTone::Flat,
            as_of,
            state: SectionState::Empty,
            connection: BrokerConnectionState::Disconnected
        }
    }
}

/// One position.
///
/// The `DataTable` rule says the first column is the identity column, and
/// `StockCell` is what renders it: symbol, name, and a one-line summary of the
/// position. The rest are numeric cells, right-aligned and tabular.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Holding
{
    /// The ticker, as the market writes it.
    pub symbol: ResourceId,
    /// The company's name, in the language the market lists it under.
    pub name: String,
    /// How many shares.
    pub quantity: i64,
    /// What they cost on average.
    pub average_price: Krw,
    /// What they are worth each now.
    pub last_price: Krw,
    /// What the position is worth altogether.
    pub market_value: Krw,
    /// Gain or loss against what it cost.
    pub unrealized: Krw,
    /// The same, as a ratio of what it cost.
    pub unrealized_ratio: Ratio,
    /// Which way that went.
    pub tone: MarketTone
}

/// One thing that happened today and concerns this account.
///
/// `EventList`'s rule is that Today carries account-relevant events only. A
/// disclosure about a company that is not held does not belong here, and neither
/// does a market-wide notice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TodayEvent
{
    /// What kind of event this is, in text rather than by icon alone.
    pub kind: TodayEventKind,
    /// A short line naming the event.
    pub title: String,
    /// One more line saying what it means for this account.
    pub summary: String,
    /// When it happened.
    pub at: Timestamp,
    /// What it is about, when the screen can navigate to it.
    pub subject: Option<ResourceId>
}

/// The kinds of event Today shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TodayEventKind
{
    /// A regulatory filing about something held.
    Disclosure,
    /// A registered strategy matched one of its rules.
    StrategySignal,
    /// A paper validation moved a day forward.
    ValidationProgress
}

/// A disclosure, as Today's input rather than as its output.
///
/// Section 7's `Disclosure` entity keeps a receipt id and both the published and
/// available times. Today renders it as a [`TodayEvent`]; this type is what the
/// query service reads before it does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Disclosure
{
    /// The filing's receipt identifier at the disclosure service.
    pub receipt: ResourceId,
    /// Which instrument it is about.
    pub symbol: ResourceId,
    /// The filing's title, as filed.
    pub title: String,
    /// When it was published.
    pub published_at: Timestamp,
    /// When a strategy could legally have used it — section 7.1's `available_at`.
    pub available_at: Timestamp
}

/// One day's fact from a paper validation, as Today's input.
///
/// Section 7's `ValidationEvent` is append-only and tied to a registration and a
/// market date. Today shows the recent ones; the strategy detail screen shows
/// the whole log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ValidationEvent
{
    /// Which registration this belongs to.
    pub registration: ResourceId,
    /// The market date the fact is about, as `YYYY-MM-DD`.
    pub market_date: String,
    /// What happened, in the finite vocabulary the screens map to wording.
    pub summary: String,
    /// When trdr recorded it.
    pub recorded_at: Timestamp
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn instant() -> Timestamp
    {
        "2026-08-11T00:30:00Z".parse().unwrap()
    }

    /// The header is flattened, so a screen reads `model` and `origin` at the
    /// top level rather than under a `header` key. Nesting them would make every
    /// screen reach one level deeper for the two fields all of them need.
    #[test]
    fn the_header_fields_sit_at_the_top_level()
    {
        let json =
            serde_json::to_value(TodayModel::empty(DataOrigin::Synthetic, instant())).unwrap();

        assert_eq!(json["model"], "TodayModel/v1");
        assert_eq!(json["origin"], "synthetic");
        assert_eq!(json["built_at"], "2026-08-11T00:30:00Z");
        assert!(json.get("header").is_none());
    }

    /// An empty account is a real state, not a failed one. The distinction
    /// matters at the screen: `empty` gets an `EmptyState` explaining the next
    /// step, `error` gets an alert.
    #[test]
    fn an_empty_model_reports_empty_rather_than_error()
    {
        let model = TodayModel::empty(DataOrigin::Synthetic, instant());

        assert_eq!(model.holdings_state, SectionState::Empty);
        assert_eq!(model.events_state, SectionState::Empty);
        assert_eq!(model.account.state, SectionState::Empty);
        assert_eq!(model.account.total_value, Krw(0));
    }

    /// The account summary has no account number, and this test is what keeps
    /// it that way. Section 5.2 forbids the raw account number leaving the
    /// broker boundary, and a field added here is the easiest way for one to.
    #[test]
    fn the_account_summary_carries_no_account_identifier()
    {
        let json =
            serde_json::to_value(TodayModel::empty(DataOrigin::Synthetic, instant())).unwrap();
        let account = json["account"].as_object().unwrap();

        for field in account.keys()
        {
            assert!(
                !field.contains("account") && !field.contains("number"),
                "{field} looks like an account identifier"
            );
        }
    }
}
