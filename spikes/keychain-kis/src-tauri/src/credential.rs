use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;

use crate::secret::Secret;

/// Decision `01kznp0gq3x8ark9dq489z7wj8`, and section 5.1.
pub const SERVICE: &str = "com.fxylabs.trdr";

/// The collector this spike stores under. Production would call it `kis`; the
/// suffix keeps the spike's Keychain items separable from real ones, so deleting
/// M1 deletes them too.
pub const COLLECTOR: &str = "kis-spike";

/// Section 5.1: `<local-instance-id>:<collector>:<field>`. The instance id is
/// per Mac, so a workspace restored onto another machine does not inherit these
/// items by name.
pub fn account(instance_id: &str, collector: &str, field: &str) -> String
{
    format!("{instance_id}:{collector}:{field}")
}

/// Section 12: `missing → untested → valid | invalid | rate-limited`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialState
{
    Missing,
    Untested,
    Valid,
    Invalid,
    RateLimited
}

/// What the WebView is allowed to see.
///
/// It derives `Serialize` and holds no [`Secret`], which is the guarantee rather
/// than a promise: a field carrying the value could not be added without the
/// derive failing to compile.
#[derive(Clone, Debug, Serialize)]
pub struct CredentialHandle
{
    pub collector: String,
    pub state: CredentialState
}

impl CredentialHandle
{
    pub fn new(collector: &str, state: CredentialState) -> Self
    {
        CredentialHandle { collector: collector.to_string(), state }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CredentialError
{
    /// The OS refused. The code is kept because it is diagnostic and carries no
    /// part of the value; the OS message is not, because it can quote the item.
    Os(i32),
    Unavailable
}

impl CredentialError
{
    pub fn code(&self) -> &'static str
    {
        match self
        {
            CredentialError::Os(_) => "AUTH_STORE_REFUSED",
            CredentialError::Unavailable => "AUTH_STORE_UNAVAILABLE"
        }
    }
}

pub trait CredentialStore: Send + Sync
{
    fn set(&self, account: &str, value: &Secret) -> Result<(), CredentialError>;
    fn get(&self, account: &str) -> Result<Option<Secret>, CredentialError>;
    fn delete(&self, account: &str) -> Result<(), CredentialError>;

    fn state(&self, account: &str) -> CredentialState
    {
        match self.get(account)
        {
            Ok(Some(value)) if !value.is_empty() => CredentialState::Untested,
            _ => CredentialState::Missing
        }
    }
}

/// The real one, on the login Keychain.
pub struct KeychainStore;

#[cfg(target_os = "macos")]
mod macos
{
    use super::*;
    use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

    const ITEM_NOT_FOUND: i32 = -25300;

    impl CredentialStore for KeychainStore
    {
        fn set(&self, account: &str, value: &Secret) -> Result<(), CredentialError>
        {
            // The one place the value is handed to the OS. It goes straight from
            // here into the Keychain: it is not logged on the way, and the
            // caller gets back a unit, not an echo of what it stored.
            set_generic_password(SERVICE, account, value.expose().as_bytes())
                .map_err(|error| CredentialError::Os(error.code()))
        }

        fn get(&self, account: &str) -> Result<Option<Secret>, CredentialError>
        {
            match get_generic_password(SERVICE, account)
            {
                Ok(bytes) => Ok(Some(Secret::new(String::from_utf8_lossy(&bytes).to_string()))),
                Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
                Err(error) => Err(CredentialError::Os(error.code()))
            }
        }

        fn delete(&self, account: &str) -> Result<(), CredentialError>
        {
            match delete_generic_password(SERVICE, account)
            {
                Ok(()) => Ok(()),
                // Deleting what is not there is the state the caller wanted.
                Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
                Err(error) => Err(CredentialError::Os(error.code()))
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl CredentialStore for KeychainStore
{
    fn set(&self, _account: &str, _value: &Secret) -> Result<(), CredentialError>
    {
        Err(CredentialError::Unavailable)
    }

    fn get(&self, _account: &str) -> Result<Option<Secret>, CredentialError>
    {
        Err(CredentialError::Unavailable)
    }

    fn delete(&self, _account: &str) -> Result<(), CredentialError>
    {
        Err(CredentialError::Unavailable)
    }
}

/// The same interface without the OS, so the paths above it can be tested
/// without asking a developer's Keychain for anything. Section 13 names
/// `CredentialStore` as one of the seams that has to exist for this reason.
#[derive(Default)]
pub struct MemoryStore
{
    items: Mutex<HashMap<String, String>>
}

impl CredentialStore for MemoryStore
{
    fn set(&self, account: &str, value: &Secret) -> Result<(), CredentialError>
    {
        self.items.lock().unwrap().insert(account.to_string(), value.expose().to_string());
        Ok(())
    }

    fn get(&self, account: &str) -> Result<Option<Secret>, CredentialError>
    {
        Ok(self.items.lock().unwrap().get(account).map(Secret::new))
    }

    fn delete(&self, account: &str) -> Result<(), CredentialError>
    {
        self.items.lock().unwrap().remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    const CANARY: &str = "TRDR-CANARY-6f2a9c";

    #[test]
    fn an_account_name_follows_the_documented_shape()
    {
        assert_eq!(account("abc123", "kis", "app_secret"), "abc123:kis:app_secret");
    }

    #[test]
    fn a_handle_serialises_to_a_state_and_nothing_else()
    {
        let handle = CredentialHandle::new(COLLECTOR, CredentialState::RateLimited);
        let json = serde_json::to_string(&handle).expect("a handle should serialise");
        assert_eq!(json, r#"{"collector":"kis-spike","state":"rate-limited"}"#);
        assert!(!json.contains(CANARY));
    }

    #[test]
    fn a_store_holds_rotates_and_forgets_a_value()
    {
        let store = MemoryStore::default();
        let name = account("i1", COLLECTOR, "app_secret");

        assert_eq!(store.state(&name), CredentialState::Missing);
        store.set(&name, &Secret::new(CANARY)).expect("the value should store");
        assert_eq!(store.state(&name), CredentialState::Untested);
        assert_eq!(store.get(&name).expect("the value should read").expect("it should be there").expose(), CANARY);

        store.set(&name, &Secret::new("TRDR-CANARY-rotated")).expect("the value should rotate");
        assert_eq!(
            store.get(&name).expect("the value should read").expect("it should be there").expose(),
            "TRDR-CANARY-rotated"
        );

        store.delete(&name).expect("the value should delete");
        assert!(store.get(&name).expect("reading nothing is not an error").is_none());
        assert_eq!(store.state(&name), CredentialState::Missing);
    }

    #[test]
    fn deleting_what_is_not_there_is_not_an_error()
    {
        let store = MemoryStore::default();
        store.delete("i1:kis-spike:absent").expect("deleting nothing should be fine");
    }

    #[test]
    fn a_store_error_reports_a_code_and_no_message_from_the_os()
    {
        assert_eq!(CredentialError::Os(-25293).code(), "AUTH_STORE_REFUSED");
        assert!(!format!("{:?}", CredentialError::Os(-25293)).contains(CANARY));
    }

    /// The same round trip against the real login Keychain.
    ///
    /// Ignored by default and run by name, because it writes to the developer's
    /// own Keychain and because a test binary's path changes on every build,
    /// which is what makes macOS ask before letting a *new* binary read an item
    /// an older one created. Whether it asks is itself part of what this spike
    /// has to find out.
    #[test]
    #[ignore = "writes to the real login Keychain; run it by name"]
    fn the_real_keychain_holds_rotates_and_forgets_a_canary()
    {
        let store = KeychainStore;
        let name = account("spike-probe", COLLECTOR, "app_secret");

        store.set(&name, &Secret::new(CANARY)).expect("the canary should store");
        let read = store.get(&name).expect("the canary should read").expect("it should be there");
        assert_eq!(read.expose(), CANARY);

        store.set(&name, &Secret::new("TRDR-CANARY-rotated")).expect("the canary should rotate");
        let rotated = store.get(&name).expect("the canary should read").expect("it should be there");
        assert_eq!(rotated.expose(), "TRDR-CANARY-rotated");

        store.delete(&name).expect("the canary should delete");
        assert!(store.get(&name).expect("reading nothing is not an error").is_none());
    }

    /// The two halves of the question a single-process round trip cannot answer:
    /// does macOS let a *rebuilt* binary read an item the previous build stored,
    /// or does it ask the user first? A shipped app hits this on every update.
    /// Run `keychain_probe_store`, rebuild, then run `keychain_probe_read`.
    #[test]
    #[ignore = "half of a cross-build probe; run it by name"]
    fn keychain_probe_store()
    {
        KeychainStore
            .set(&account("spike-probe", COLLECTOR, "cross_build"), &Secret::new(CANARY))
            .expect("the canary should store");
    }

    #[test]
    #[ignore = "half of a cross-build probe; run it by name"]
    fn keychain_probe_read()
    {
        let name = account("spike-probe", COLLECTOR, "cross_build");
        let read = KeychainStore.get(&name).expect("the canary should read").expect("it should be there");
        assert_eq!(read.expose(), CANARY);
        KeychainStore.delete(&name).expect("the canary should delete");
    }
}
