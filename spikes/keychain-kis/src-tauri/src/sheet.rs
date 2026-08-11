use std::cell::RefCell;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, Message};
use objc2_app_kit::{NSAlert, NSAlertStyle, NSModalResponse, NSSecureTextField, NSTextField, NSView, NSWindow};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::secret::Secret;

const SAVE_BUTTON: usize = 0;
const SAVE_RETURN: NSModalResponse = 1000;

// The sheet currently on screen, if any.
//
// The rule spike 3 paid for applies here unchanged: take what you need out of
// this cell and let the borrow go before calling AppKit. Ending a sheet runs the
// completion handler, the handler comes back here, and a borrow still open
// across that call is a panic on the main thread — which ends the process.
thread_local!
{
    static ON_SCREEN: RefCell<Option<Showing>> = const { RefCell::new(None) };
}

struct Showing
{
    alert: Retained<NSAlert>,
    key_field: Retained<NSTextField>,
    secret_field: Retained<NSSecureTextField>,
    parent: Retained<NSWindow>,
    sheet: Retained<NSWindow>
}

/// Raises the credential sheet. Must be called on the main thread.
///
/// `saved` receives the two values and is expected to put them straight into the
/// Keychain. Note what does not happen in between: the values are not returned
/// to the caller of this function, not sent as an event, and not held anywhere
/// after the handler runs. Section 10 requires the sheet to reach the Keychain
/// directly and hand the WebView a state.
pub fn present(mtm: MainThreadMarker, parent: &NSWindow, saved: Box<dyn Fn(Secret, Secret)>)
{
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.setMessageText(&NSString::from_str("KIS app key and app secret"));
    alert.setInformativeText(&NSString::from_str(
        "These are stored in the macOS Keychain. The app window never receives them, and neither does any log, export or backup."
    ));

    let (container, key_field, secret_field) = fields(mtm);
    alert.setAccessoryView(Some(&container));
    alert.addButtonWithTitle(&NSString::from_str("Save"));
    // NSAlert gives Escape to a button with this title, which is what makes
    // dismissing the sheet without saving the easy action.
    alert.addButtonWithTitle(&NSString::from_str("Cancel"));

    let sheet = alert.window();
    let reading_key = key_field.clone();
    let reading_secret = secret_field.clone();
    let handler = RcBlock::new(move |response: NSModalResponse|
    {
        ON_SCREEN.with(|slot| slot.borrow_mut().take());
        if response != SAVE_RETURN
        {
            return;
        }
        let app_key = Secret::new(reading_key.stringValue().to_string());
        let app_secret = Secret::new(reading_secret.stringValue().to_string());
        // Cleared before the handler returns, so the value does not sit in a
        // view that outlives the sheet.
        reading_key.setStringValue(&NSString::from_str(""));
        reading_secret.setStringValue(&NSString::from_str(""));
        saved(app_key, app_secret);
    });

    ON_SCREEN.with(|slot| *slot.borrow_mut() = Some(Showing
    {
        alert: alert.clone(),
        key_field,
        secret_field,
        parent: parent.retain(),
        sheet: sheet.clone()
    }));

    alert.beginSheetModalForWindow_completionHandler(parent, Some(&handler));
}

/// Spike-only. Types into the sheet and presses a button, so the whole path can
/// be driven without an accessibility grant. `performClick:` runs the button's
/// own action, so everything after the keystroke is the real path.
pub fn fill_and_click(app_key: &str, app_secret: &str, save: bool) -> bool
{
    let Some((alert, key_field, secret_field)) = ON_SCREEN.with(|slot|
    {
        slot.borrow().as_ref().map(|showing|
        {
            (showing.alert.clone(), showing.key_field.clone(), showing.secret_field.clone())
        })
    })
    else
    {
        return false;
    };

    key_field.setStringValue(&NSString::from_str(app_key));
    secret_field.setStringValue(&NSString::from_str(app_secret));

    let buttons = alert.buttons();
    let wanted = if save { SAVE_BUTTON } else { SAVE_BUTTON + 1 };
    if wanted >= buttons.count()
    {
        return false;
    }
    unsafe { buttons.objectAtIndex(wanted).performClick(None) };
    true
}

/// Takes the sheet down without saving. Must be called on the main thread.
pub fn dismiss()
{
    let showing = ON_SCREEN.with(|slot| slot.borrow_mut().take());
    if let Some(showing) = showing
    {
        showing.parent.endSheet_returnCode(&showing.sheet, SAVE_RETURN + 1);
    }
}

fn fields(mtm: MainThreadMarker) -> (Retained<NSView>, Retained<NSTextField>, Retained<NSSecureTextField>)
{
    let container = NSView::initWithFrame(NSView::alloc(mtm), rect(0.0, 0.0, 420.0, 76.0));

    let key_label = NSTextField::labelWithString(&NSString::from_str("App key"), mtm);
    key_label.setFrame(rect(0.0, 46.0, 100.0, 20.0));
    let key_field = NSTextField::initWithFrame(NSTextField::alloc(mtm), rect(104.0, 44.0, 316.0, 24.0));

    let secret_label = NSTextField::labelWithString(&NSString::from_str("App secret"), mtm);
    secret_label.setFrame(rect(0.0, 8.0, 100.0, 20.0));
    // The one that matters. A plain field would put the value on screen, in a
    // screenshot and in anything recording the display.
    let secret_field = NSSecureTextField::initWithFrame(NSSecureTextField::alloc(mtm), rect(104.0, 6.0, 316.0, 24.0));

    for view in [&*key_label, &*key_field, &*secret_label, &*secret_field]
    {
        container.addSubview(view);
    }
    (container, key_field, secret_field)
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect
{
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

#[cfg(test)]
mod tests
{
    use super::*;

    // The sheet needs a running NSApplication, so what is checked without one is
    // the constant a wrong value would turn into a saved credential the user
    // meant to cancel.
    #[test]
    fn the_first_button_is_the_saving_one()
    {
        assert_eq!(SAVE_BUTTON, 0);
        assert_eq!(SAVE_RETURN, 1000, "the first button an NSAlert is given returns 1000");
    }
}
