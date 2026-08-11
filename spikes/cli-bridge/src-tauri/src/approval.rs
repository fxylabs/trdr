use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome
{
    Approved,
    Rejected,
    Expired,
    /// The caller went away while its own approval was on screen.
    Cancelled,
    /// The spec or data hash moved between the request and the approval, so what
    /// the user read on the sheet is no longer what would be written.
    Stale
}

impl Outcome
{
    pub fn code(self) -> &'static str
    {
        match self
        {
            Outcome::Approved => "APPROVED",
            Outcome::Rejected => "REJECTED",
            Outcome::Expired => "EXPIRED",
            Outcome::Cancelled => "CANCELLED",
            Outcome::Stale => "STALE_APPROVAL"
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hashes
{
    pub spec: String,
    pub data: String
}

/// Everything the sheet displays, and nothing the caller wrote by hand. Section
/// 10 requires the sheet to present the host's own validation result; a caller
/// that could pass in its own "external effects" line would be writing the
/// sentence the user approves.
#[derive(Clone, Debug, Serialize)]
pub struct ApprovalSummary
{
    pub request_id: String,
    pub strategy_id: String,
    pub command: String,
    pub external_effects: String,
    pub local_effect: String,
    pub spec_hash: String,
    pub data_hash: String,
    pub expires_in_seconds: u64,
    pub expires_at_unix: u64
}

impl ApprovalSummary
{
    pub fn build(request_id: &str, strategy_id: &str, command: &str, frozen: &Hashes, expires_in_seconds: u64) -> Self
    {
        ApprovalSummary
        {
            request_id: request_id.to_string(),
            strategy_id: strategy_id.to_string(),
            command: command.to_string(),
            external_effects: "none — registration places no order and sends nothing to a broker".to_string(),
            local_effect: format!("registers `{strategy_id}` for paper validation, in one local transaction"),
            spec_hash: frozen.spec.clone(),
            data_hash: frozen.data.clone(),
            expires_in_seconds,
            expires_at_unix: now_unix() + expires_in_seconds
        }
    }
}

fn now_unix() -> u64
{
    SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or_default()
}

pub enum Started
{
    /// The approval is on its way to the sheet. The caller blocks on this.
    Waiting(Receiver<Outcome>),
    /// This request id already reached a terminal result, so it gets that one
    /// back rather than a second sheet.
    AlreadySettled(Outcome),
    /// Another approval is on screen. Section 9.3 allows exactly one at a time.
    Busy
}

struct Live
{
    request_id: String,
    strategy_id: String,
    frozen: Hashes,
    reply: Sender<Outcome>
}

#[derive(Default)]
struct Inner
{
    live: Option<Live>,
    settled: HashMap<String, Outcome>,
    current: HashMap<String, Hashes>,
    committed: Vec<String>
}

/// Holds every rule that decides an approval's terminal result under one lock.
///
/// The reason they are together rather than composed out of smaller pieces: the
/// dedupe memo, the revalidation and the write have to be one atomic step. Split
/// across two locks, an approve arriving as an expiry fires can pass the "not yet
/// settled" check and then commit after the caller has already been told the
/// request expired.
#[derive(Default)]
pub struct Broker
{
    inner: Mutex<Inner>
}

impl Broker
{
    pub fn start(&self, request_id: &str, strategy_id: &str, frozen: Hashes) -> Started
    {
        let mut inner = self.lock();

        if let Some(outcome) = inner.settled.get(request_id).copied()
        {
            return Started::AlreadySettled(outcome);
        }
        if inner.live.is_some()
        {
            return Started::Busy;
        }

        inner.current.entry(strategy_id.to_string()).or_insert_with(|| frozen.clone());
        let (reply, receiver) = mpsc::channel();
        inner.live = Some(Live
        {
            request_id: request_id.to_string(),
            strategy_id: strategy_id.to_string(),
            frozen,
            reply
        });
        Started::Waiting(receiver)
    }

    /// Settles the live approval, if this request id is still the live one.
    ///
    /// Returns the outcome that was recorded, which is not always the one asked
    /// for: an approve whose hashes moved is recorded as stale. `None` means this
    /// call did not settle anything, because something else already had.
    pub fn settle(&self, request_id: &str, wanted: Outcome) -> Option<Outcome>
    {
        let mut inner = self.lock();
        match inner.live.as_ref()
        {
            Some(live) if live.request_id == request_id => {}
            _ => return None
        }

        let live = inner.live.take().expect("the live approval was checked one line above");
        let recorded = match wanted
        {
            Outcome::Approved if inner.current.get(&live.strategy_id) != Some(&live.frozen) => Outcome::Stale,
            Outcome::Approved =>
            {
                inner.committed.push(live.strategy_id.clone());
                Outcome::Approved
            }
            other => other
        };

        inner.settled.insert(live.request_id.clone(), recorded);
        // The caller may have already gone; a send with nobody listening is the
        // disconnect case settling itself, not an error.
        let _ = live.reply.send(recorded);
        Some(recorded)
    }

    /// Spike-only. Moves a strategy's current hashes so the revalidation step of
    /// section 9.3 has something to catch.
    pub fn set_hashes(&self, strategy_id: &str, hashes: Hashes)
    {
        self.lock().current.insert(strategy_id.to_string(), hashes);
    }

    pub fn live_request_id(&self) -> Option<String>
    {
        self.lock().live.as_ref().map(|live| live.request_id.clone())
    }

    /// The strategies actually written. This is what a "ran twice" bug shows up
    /// in — the response alone cannot tell one write from two.
    pub fn committed(&self) -> Vec<String>
    {
        self.lock().committed.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner>
    {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub fn expiry_duration(seconds: u64) -> Duration
{
    Duration::from_secs(seconds)
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::sync::Arc;
    use std::thread;

    fn hashes(spec: &str) -> Hashes
    {
        Hashes { spec: spec.to_string(), data: "data-1".to_string() }
    }

    fn waiting(started: Started) -> Receiver<Outcome>
    {
        match started
        {
            Started::Waiting(receiver) => receiver,
            Started::AlreadySettled(outcome) => panic!("expected a new approval, got a settled {outcome:?}"),
            Started::Busy => panic!("expected a new approval, got busy")
        }
    }

    #[test]
    fn an_approve_returns_to_the_caller_and_writes_once()
    {
        let broker = Broker::default();
        let receiver = waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        assert_eq!(broker.settle("r1", Outcome::Approved), Some(Outcome::Approved));
        assert_eq!(receiver.recv().expect("the caller should be answered"), Outcome::Approved);
        assert_eq!(broker.committed(), vec!["low-vol-v1".to_string()]);
    }

    #[test]
    fn a_reject_returns_to_the_caller_and_writes_nothing()
    {
        let broker = Broker::default();
        let receiver = waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        broker.settle("r1", Outcome::Rejected);
        assert_eq!(receiver.recv().expect("the caller should be answered"), Outcome::Rejected);
        assert!(broker.committed().is_empty());
    }

    #[test]
    fn an_expiry_returns_to_the_caller_and_writes_nothing()
    {
        let broker = Broker::default();
        let receiver = waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        broker.settle("r1", Outcome::Expired);
        assert_eq!(receiver.recv().expect("the caller should be answered"), Outcome::Expired);
        assert!(broker.committed().is_empty());
    }

    // The disconnect case. Nobody is listening on the reply channel any more,
    // and settling must still record the terminal result rather than fail.
    #[test]
    fn a_cancel_after_the_caller_has_gone_still_settles()
    {
        let broker = Broker::default();
        let receiver = waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        drop(receiver);
        assert_eq!(broker.settle("r1", Outcome::Cancelled), Some(Outcome::Cancelled));
        assert!(broker.live_request_id().is_none());
    }

    #[test]
    fn a_repeated_request_id_gets_the_same_terminal_result_without_a_second_sheet()
    {
        let broker = Broker::default();
        waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        broker.settle("r1", Outcome::Approved);

        match broker.start("r1", "low-vol-v1", hashes("spec-1"))
        {
            Started::AlreadySettled(outcome) => assert_eq!(outcome, Outcome::Approved),
            _ => panic!("a settled request id started a second approval")
        }
        assert_eq!(broker.committed().len(), 1, "the registration was written twice");
    }

    #[test]
    fn a_repeated_request_id_replays_a_reject_rather_than_asking_again()
    {
        let broker = Broker::default();
        waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        broker.settle("r1", Outcome::Rejected);

        match broker.start("r1", "low-vol-v1", hashes("spec-1"))
        {
            Started::AlreadySettled(outcome) => assert_eq!(outcome, Outcome::Rejected),
            _ => panic!("a rejected request id was asked again")
        }
    }

    #[test]
    fn a_second_approval_is_refused_while_one_is_on_screen()
    {
        let broker = Broker::default();
        let first = waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        assert!(matches!(broker.start("r2", "momentum-v2", hashes("spec-9")), Started::Busy));

        broker.settle("r1", Outcome::Rejected);
        drop(first);
        waiting(broker.start("r2", "momentum-v2", hashes("spec-9")));
    }

    #[test]
    fn an_approve_whose_hashes_moved_is_refused_as_stale_and_writes_nothing()
    {
        let broker = Broker::default();
        let receiver = waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        broker.set_hashes("low-vol-v1", hashes("spec-2"));

        assert_eq!(broker.settle("r1", Outcome::Approved), Some(Outcome::Stale));
        assert_eq!(receiver.recv().expect("the caller should be answered"), Outcome::Stale);
        assert!(broker.committed().is_empty());
    }

    #[test]
    fn settling_a_request_id_that_is_not_live_changes_nothing()
    {
        let broker = Broker::default();
        assert_eq!(broker.settle("r1", Outcome::Approved), None);

        waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));
        assert_eq!(broker.settle("r2", Outcome::Approved), None);
        assert!(broker.live_request_id().is_some());
    }

    // The race the single lock exists for: an approve and an expiry arriving at
    // once must produce one terminal result and at most one write, whichever of
    // them wins.
    #[test]
    fn an_approve_racing_an_expiry_settles_exactly_once()
    {
        for _ in 0..200
        {
            let broker = Arc::new(Broker::default());
            let receiver = waiting(broker.start("r1", "low-vol-v1", hashes("spec-1")));

            let approving = Arc::clone(&broker);
            let expiring = Arc::clone(&broker);
            let one = thread::spawn(move || approving.settle("r1", Outcome::Approved));
            let two = thread::spawn(move || expiring.settle("r1", Outcome::Expired));

            let settled: Vec<_> = [one, two]
                .into_iter()
                .filter_map(|handle| handle.join().expect("a settling thread panicked"))
                .collect();

            assert_eq!(settled.len(), 1, "two callers both settled the same approval");
            assert_eq!(receiver.recv().expect("the caller should be answered"), settled[0]);
            assert!(broker.committed().len() <= 1);
        }
    }
}
