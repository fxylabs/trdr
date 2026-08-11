use std::fmt;

/// A value that must not reach a log, the database, an export or the WebView.
///
/// The type is the enforcement, not a convention. It implements `Debug` as a
/// fixed placeholder and implements nothing else — no `Display`, no `Serialize`,
/// no `Deref`, no `AsRef<str>`. That is what closes the paths a secret usually
/// escapes through:
///
/// - `format!("{secret}")` and `println!` do not compile.
/// - `#[derive(Serialize)]` on a struct holding one does not compile, so a
///   config or state object cannot pick up a secret by accident and no
///   `skip_serializing` attribute has to be remembered.
/// - `{:?}` and a panic message print the placeholder, so the error paths the
///   plan names — the ones that format a request for a log — have nothing to
///   print even when someone reaches for the lazy thing.
///
/// [`Secret::expose`] is the single door, and its name is what a reviewer greps
/// for. Every call site of it in this spike is a place a leak could start.
///
/// The two closed paths are checked rather than described. Adding `Display` or
/// `Serialize` to this type later would reopen both, and these stop compiling
/// the moment it happens:
///
/// ```compile_fail
/// let secret = trdr_spike_keychain_kis_lib::secret::Secret::new("x");
/// println!("{secret}");
/// ```
///
/// ```compile_fail
/// #[derive(serde::Serialize)]
/// struct Config { key: trdr_spike_keychain_kis_lib::secret::Secret }
/// ```
pub struct Secret(String);

pub const REDACTED: &str = "«redacted»";

impl Secret
{
    pub fn new(value: impl Into<String>) -> Self
    {
        Secret(value.into())
    }

    /// Hands out the value. Call it as late as possible and never bind the
    /// result to something that outlives the call.
    pub fn expose(&self) -> &str
    {
        &self.0
    }

    pub fn is_empty(&self) -> bool
    {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        // Not the length either. A length narrows a guess, and it is the kind of
        // detail that gets added "just for debugging" and then stays.
        formatter.write_str(REDACTED)
    }
}

impl Drop for Secret
{
    fn drop(&mut self)
    {
        // Best effort, and worth saying why it is only that: a `String` that grew
        // has already copied its bytes to a new allocation, and the old one is
        // not reachable from here. What this does close is the common case where
        // the value sat in one buffer from creation to drop, so a core file
        // written afterwards does not still hold it.
        let bytes = unsafe { self.0.as_bytes_mut() };
        bytes.fill(0);
    }
}

impl From<String> for Secret
{
    fn from(value: String) -> Self
    {
        Secret(value)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    const CANARY: &str = "TRDR-CANARY-6f2a9c";

    #[test]
    fn debug_prints_a_placeholder_rather_than_the_value()
    {
        let secret = Secret::new(CANARY);
        assert_eq!(format!("{secret:?}"), REDACTED);
        assert!(!format!("{secret:?}").contains(CANARY));
    }

    // The path the plan calls out: an error that formats the request it failed
    // on. A struct that holds a secret still cannot print one.
    #[test]
    fn a_struct_holding_a_secret_prints_no_part_of_it()
    {
        #[derive(Debug)]
        #[allow(dead_code, reason = "every field is read through the derived Debug, which dead-code analysis ignores")]
        struct Request
        {
            url: String,
            app_key: Secret,
            app_secret: Secret
        }

        let request = Request
        {
            url: "http://127.0.0.1:9/oauth2/tokenP".to_string(),
            app_key: Secret::new("key-abc"),
            app_secret: Secret::new(CANARY)
        };
        let printed = format!("{request:?}");
        assert!(printed.contains("127.0.0.1"), "the safe part should still be printable: {printed}");
        assert!(!printed.contains(CANARY));
        assert!(!printed.contains("key-abc"));
    }

    // A panic message is a crash artifact, and `unwrap` on a Result whose error
    // carries a secret is how one usually gets there.
    #[test]
    fn a_panic_carrying_a_secret_prints_no_part_of_it()
    {
        // Built from something the compiler cannot fold away, so this stays a
        // real `expect` on a real `Err` rather than a statically known panic.
        let reached: Result<(), Secret> = std::env::var("PATH").map(|_| ()).map_err(|_| Secret::new(CANARY));
        let failed: Result<(), Secret> = reached.and_then(|()| Err(Secret::new(CANARY)));
        let message = std::panic::catch_unwind(|| failed.expect("the spike expects this to fail"))
            .expect_err("this should have panicked");
        let printed = message
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(!printed.contains(CANARY), "a panic printed the secret: {printed}");
    }

    #[test]
    fn the_value_is_reachable_only_through_expose()
    {
        assert_eq!(Secret::new(CANARY).expose(), CANARY);
    }
}
