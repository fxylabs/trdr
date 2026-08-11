use std::cell::RefCell;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, Message};
use objc2_app_kit::{NSAlert, NSAlertStyle, NSFont, NSModalResponse, NSTextField, NSWindow};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::approval::ApprovalSummary;

// The first button an NSAlert is given is its default: rightmost, focused, and
// bound to Return. Section 9.3 puts initial focus on the safe action, so reject
// is added first and a stray Return refuses the registration.
const REJECT_BUTTON: usize = 0;
const APPROVE_BUTTON: usize = 1;
const REJECT_RETURN: NSModalResponse = 1000;

// The sheet currently on screen, if any.
//
// A thread-local is the honest container here. Every value in it is an AppKit
// object that may only be touched on the main thread, and every function in
// this file already runs there. A `Mutex` would compile and would be a lie:
// `Retained<NSWindow>` is not `Send`, and the lock would suggest a second
// thread is allowed to reach the sheet.
//
// One rule holds for every reader below: take what you need out of this cell and
// let the borrow go before calling AppKit. Ending a sheet runs the completion
// handler, the handler comes back here for a mutable borrow, and a borrow still
// open across that call is a panic on the main thread — which ends the process,
// not just the request.
thread_local!
{
    static ON_SCREEN: RefCell<Option<Showing>> = const { RefCell::new(None) };
}

struct Showing
{
    request_id: String,
    alert: Retained<NSAlert>,
    parent: Retained<NSWindow>,
    sheet: Retained<NSWindow>
}

/// Raises the approval sheet on `parent`. Must be called on the main thread.
///
/// `answered` is called with the user's decision. It is not called when the
/// sheet is taken down by [`dismiss`], because by then the request has already
/// been settled by whatever took it down.
pub fn present(mtm: MainThreadMarker, parent: &NSWindow, summary: &ApprovalSummary, answered: Box<dyn Fn(bool)>)
{
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.setMessageText(&NSString::from_str(&format!("Register `{}` for paper validation?", summary.strategy_id)));
    alert.setInformativeText(&NSString::from_str(
        "trdr is asking for this because a registration is a durable write. Approving freezes the rules below."
    ));
    let detail = detail_view(mtm, summary);
    alert.setAccessoryView(Some(&detail));
    alert.addButtonWithTitle(&NSString::from_str("Reject"));
    alert.addButtonWithTitle(&NSString::from_str("Approve"));

    let sheet = alert.window();
    let request_id = summary.request_id.clone();
    let handler = RcBlock::new(move |response: NSModalResponse|
    {
        ON_SCREEN.with(|slot| slot.borrow_mut().take());
        answered(response != REJECT_RETURN);
    });

    ON_SCREEN.with(|slot| *slot.borrow_mut() = Some(Showing
    {
        request_id,
        alert: alert.clone(),
        parent: parent.retain(),
        sheet: sheet.clone()
    }));

    alert.beginSheetModalForWindow_completionHandler(parent, Some(&handler));
}

/// Presses a button on the sheet that is on screen. Must be called on the main
/// thread. Returns whether there was a sheet to press.
///
/// Spike-only, and the reason it exists is evidence rather than convenience. The
/// approve and reject paths are the two this spike has to demonstrate, and
/// driving a real mouse from a test needs an accessibility grant that a checkout
/// on another Mac will not have. `performClick:` is the nearest thing that is
/// still the real path: the button's own action runs, the sheet ends itself, and
/// the completion handler receives the return code AppKit chose. Only the
/// hardware event is missing.
pub fn click(approve: bool) -> bool
{
    // The alert is copied out and the borrow released before AppKit is called.
    // Holding it across `performClick:` is what the first version did, and it
    // took the app down: the button's action ends the sheet, AppKit runs the
    // completion handler from inside that same call, and the handler asks this
    // same cell for a mutable borrow. See the note on `ON_SCREEN` above.
    let Some(alert) = ON_SCREEN.with(|slot| slot.borrow().as_ref().map(|showing| showing.alert.clone())) else
    {
        return false;
    };

    let buttons = alert.buttons();
    let wanted = if approve { APPROVE_BUTTON } else { REJECT_BUTTON };
    if wanted >= buttons.count()
    {
        return false;
    }
    unsafe { buttons.objectAtIndex(wanted).performClick(None) };
    true
}

/// Takes the sheet down when something other than the user settled the request —
/// an expiry, or the caller disconnecting. Must be called on the main thread.
///
/// The request id is checked rather than assumed: by the time a dismissal is
/// dispatched to the main thread the user may already have answered, and the
/// next approval may already be on screen.
pub fn dismiss(request_id: &str)
{
    let showing = ON_SCREEN.with(|slot|
    {
        let matches = slot.borrow().as_ref().is_some_and(|showing| showing.request_id == request_id);
        if matches { slot.borrow_mut().take() } else { None }
    });

    if let Some(showing) = showing
    {
        showing.parent.endSheet_returnCode(&showing.sheet, REJECT_RETURN);
    }
}

fn detail_view(mtm: MainThreadMarker, summary: &ApprovalSummary) -> Retained<NSTextField>
{
    let label = NSTextField::labelWithString(&NSString::from_str(&detail_text(summary)), mtm);
    label.setFont(Some(&NSFont::monospacedSystemFontOfSize_weight(11.0, 0.0)));
    label.setUsesSingleLineMode(false);
    label.setMaximumNumberOfLines(0);
    label.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(460.0, 132.0)));
    label
}

/// Section 10 fixes what the sheet shows: the exact command, the external
/// effects, the local effect, a fixed hash and an expiry.
pub fn detail_text(summary: &ApprovalSummary) -> String
{
    format!(
        "command        {}\n\
         external       {}\n\
         local          {}\n\
         spec hash      {}\n\
         data hash      {}\n\
         expires        in {}s\n\
         request        {}",
        summary.command,
        summary.external_effects,
        summary.local_effect,
        summary.spec_hash,
        summary.data_hash,
        summary.expires_in_seconds,
        summary.request_id
    )
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::approval::Hashes;

    fn summary() -> ApprovalSummary
    {
        let frozen = Hashes { spec: "spec-1".to_string(), data: "data-1".to_string() };
        ApprovalSummary::build("r1", "low-vol-v1", "trdr strategy register low-vol-v1", &frozen, 60)
    }

    // The sheet itself needs a running NSApplication, so what is checked here is
    // the part that can be wrong without one: the sentence the user reads.
    #[test]
    fn the_sheet_states_every_field_section_ten_requires()
    {
        let text = detail_text(&summary());
        for required in ["trdr strategy register low-vol-v1", "spec-1", "data-1", "in 60s", "r1"]
        {
            assert!(text.contains(required), "the sheet does not show {required}:\n{text}");
        }
        assert!(text.contains("none — registration places no order"), "the sheet does not state the external effects");
        assert!(text.contains("paper validation"), "the sheet does not state the local effect");
    }

    // A response that is not the reject button is an approval, and the reject
    // button is the one Return reaches. Getting this backwards would turn a
    // stray keystroke into a durable write.
    #[test]
    fn the_default_button_is_the_refusing_one()
    {
        assert_eq!(REJECT_BUTTON, 0, "the button added first is the default one");
        assert_eq!(REJECT_RETURN, 1000, "the first button an NSAlert is given returns 1000");
        assert_eq!(APPROVE_BUTTON, REJECT_BUTTON + 1);
    }
}
