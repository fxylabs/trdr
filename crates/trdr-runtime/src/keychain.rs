//! Credentials, by handle only.
//!
//! What lands here (sections 5.1, 8.3, and 10): Keychain items under service
//! `com.fxylabs.trdr`, keyed by `<local-instance-id>:<collector>:<field>`, and
//! the handle a collector uses to make a call without ever seeing the secret.
//!
//! The local instance id is per machine, which is what stops a restored backup
//! or another Mac from inheriting an existing machine's stored secrets by name.
//! A secret value never leaves this module: the WebView and the CLI learn only
//! `missing`, `untested`, `valid`, `invalid`, or `rate-limited`.
//!
//! Nothing is implemented yet.
