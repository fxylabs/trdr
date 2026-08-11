//! The check that stops the committed TypeScript from drifting off the commands.
//!
//! `apps/desktop/src/bindings.ts` is generated and committed, which is the
//! arrangement that lets the React build stay a plain `vite build` with no Rust
//! toolchain in it. The cost of that arrangement is a file that can silently
//! describe a command signature the host no longer has, and this test is what
//! pays it: `cargo test --workspace` regenerates the bindings and fails on any
//! difference.

use std::path::Path;
use trdr_desktop_lib::bindings;

#[test]
fn the_committed_bindings_are_what_the_commands_generate()
{
    let path = bindings::bindings_path();
    let generated = bindings::generate().expect("could not generate the bindings");

    let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{} could not be read ({error}). Run `cargo run -p trdr-desktop --bin \
             export-bindings`.",
            path.display()
        )
    });

    assert_eq!(
        committed,
        generated,
        "\n{} is out of date with the commands in src/commands.rs.\nRun `cargo run -p \
         trdr-desktop --bin export-bindings` and commit the result.\n",
        path.display()
    );
}

/// The generated file is only useful to the React app if the React app can find
/// it, and a generator pointed at the wrong directory fails quietly — it writes
/// a perfectly good file nobody imports.
#[test]
fn the_bindings_land_inside_the_react_source_tree()
{
    let path = bindings::bindings_path();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri always has a parent")
        .join("src");

    assert!(
        path.starts_with(&source),
        "{} is not under {}",
        path.display(),
        source.display()
    );
}

/// The whole point of a typed bridge is that a validated Rust newtype arrives in
/// TypeScript as something TypeScript can actually hold. `RequestId` is a
/// 26-character ULID in Rust; if it crossed as a reference to a generated type
/// called `String` the bindings would not typecheck, which is the failure this
/// asserts against directly rather than waiting for `tsc` to find it.
#[test]
fn a_validated_id_crosses_as_a_plain_string()
{
    let generated = bindings::generate().expect("could not generate the bindings");

    assert!(
        generated.contains("id: string"),
        "the request id did not cross as a string:\n{generated}"
    );
    assert!(
        !generated.contains("String"),
        "a Rust type name leaked into the bindings:\n{generated}"
    );
}

/// A generic return type comes out of tauri-specta 2.0.0-rc.25 with its type
/// arguments dropped — `UiResponseEnvelope_Serialize<T>`, with `T` bound to
/// nothing. `tsc` catches it, but only once someone runs the frontend build, and
/// the message it gives points at generated code rather than at the Rust that
/// caused it. This says so directly. See `commands.rs` for the workaround.
#[test]
fn no_command_returns_an_unbound_generic()
{
    let generated = bindings::generate().expect("could not generate the bindings");

    // Only the command signatures. A `<T>` in a type declaration is a generic
    // parameter being introduced, which is correct and common; a `<T>` in the
    // return type of an `invoke` call is one being used without ever having been.
    let calls = generated
        .lines()
        .filter(|line| line.contains("__TAURI_INVOKE<"))
        .collect::<Vec<_>>();

    assert_eq!(calls.len(), 2, "expected one line per registered command");

    for line in calls
    {
        assert!(
            !line.contains("<T>"),
            "a command returns an unbound type parameter:\n{line}"
        );
    }
}

/// The exported type list, pinned.
///
/// Two things ride on this. The obvious one is that the WebView's whole type
/// surface stays something a person can read in one screen. The other is the
/// `dangerously_cast_bigints_to_number` setting in `src/bindings.rs`: it applies
/// to every type that crosses, so the moment a new one does, someone has to come
/// here, update the list, and — in doing so — see the note explaining what they
/// have just accepted for any 64-bit field it carries.
#[test]
fn the_exported_type_surface_is_the_one_that_was_reviewed()
{
    let generated = bindings::generate().expect("could not generate the bindings");

    let mut exported: Vec<&str> = generated
        .lines()
        .filter_map(|line| line.strip_prefix("export type "))
        .filter_map(|rest| rest.split([' ', '<', '=']).next())
        .collect();
    exported.sort_unstable();

    assert_eq!(
        exported,
        [
            "BootstrapModel",
            "BootstrapResponse",
            "BootstrapResponse_Deserialize",
            "BootstrapResponse_Serialize",
            "EnvelopeVersion",
            "ErrorCode",
            "ErrorEnvelope",
            "ErrorEnvelope_Deserialize",
            "ErrorEnvelope_Serialize",
            "ErrorParam",
            "PingResponse",
            "PingResponse_Deserialize",
            "PingResponse_Serialize",
            "Pong",
            "Retryability",
            "UiOutcome",
            "UiOutcome_Deserialize",
            "UiOutcome_Serialize",
            "UiResponseEnvelope",
            "UiResponseEnvelope_Deserialize",
            "UiResponseEnvelope_Serialize",
            "UpstreamStatus",
            "UpstreamStatus_Deserialize",
            "UpstreamStatus_Serialize"
        ]
    );
}
