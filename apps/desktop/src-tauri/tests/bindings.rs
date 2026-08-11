//! The check that stops the committed TypeScript from drifting off the commands.
//!
//! `apps/desktop/src/bindings.ts` is generated and committed, which is the
//! arrangement that lets the React build stay a plain `vite build` with no Rust
//! toolchain in it. The cost of that arrangement is a file that can silently
//! describe a command signature the host no longer has, and this test is what
//! pays it: `cargo test --workspace` regenerates the bindings and fails on any
//! difference.

use std::path::Path;
use trdr_desktop_lib::{bindings, commands};

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

    assert_eq!(
        calls.len(),
        commands::COMMANDS.len(),
        "expected one line per registered command"
    );

    for line in calls
    {
        assert!(
            !line.contains("<T>"),
            "a command returns an unbound type parameter:\n{line}"
        );
    }
}

/// The registered handlers are exactly the ones `commands::COMMANDS` describes.
///
/// This is the last of the four places a command has to appear, and the only one
/// left without a check. `build.rs` and the capability file are held together by
/// tauri-build, which refuses a permission identifier it did not generate;
/// `tests/config.rs` holds the capability file against `COMMANDS`. What was
/// missing is the list `collect_commands!` actually registers — a handler added
/// to `COMMANDS` and forgotten in `builder.rs` would leave a permission granted
/// for a command that does not exist, and nothing would have said so.
///
/// The generated bindings are the readable form of that registration, so they
/// are what gets compared.
#[test]
fn the_registered_handlers_are_the_ones_the_command_list_names()
{
    let generated = bindings::generate().expect("could not generate the bindings");

    let mut registered: Vec<&str> = generated
        .lines()
        .filter_map(|line| line.split("__TAURI_INVOKE<").nth(1))
        .filter_map(|rest| rest.split('"').nth(1))
        .collect();
    registered.sort_unstable();

    let mut expected: Vec<&str> = commands::COMMANDS
        .iter()
        .map(|command| command.handler)
        .collect();
    expected.sort_unstable();

    assert_eq!(registered, expected);
}

/// The exported type list, pinned.
///
/// Two things ride on this. The obvious one is that the WebView's whole type
/// surface stays something a person can read in one screen. The other is the
/// `dangerously_cast_bigints_to_number` setting in `src/bindings.rs`: it applies
/// to every type that crosses, so the moment a new one does, someone has to come
/// here, update the list, and — in doing so — see the note explaining what they
/// have just accepted for any 64-bit field it carries.
///
/// What has been accepted so far, and by whom: the query models added for
/// milestone M2 carry two kinds of 64-bit integer, and both were checked against
/// the 2^53 a JavaScript number represents exactly.
///
/// - `Krw`, an amount in won. A personal account, a paper position, and an
///   equity curve point are all far below 9,007,199,254,740,992 won, and a
///   holding that were not would have other problems first.
/// - `quantity` on a holding and a paper position, a share count.
///
/// Nothing here carries a hash, an id, or a nanosecond timestamp as a 64-bit
/// number — those cross as strings, which is why `input_hash`, `output_hash` and
/// `Timestamp` are `String` and not integers. A future field that does carry one
/// of those must not be added as an `i64`.
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
            "AccountSummary",
            "BacktestAssumptions",
            "BacktestMetrics",
            "BacktestResponse",
            "BacktestResponse_Deserialize",
            "BacktestResponse_Serialize",
            "BacktestWarning",
            "BootstrapModel",
            "BootstrapResponse",
            "BootstrapResponse_Deserialize",
            "BootstrapResponse_Serialize",
            "BrokerConnectionState",
            "CoverageGap",
            "CurvePoint",
            "DataCoverage",
            "DataOrigin",
            "EnvelopeVersion",
            "ErrorCode",
            "ErrorEnvelope",
            "ErrorEnvelope_Deserialize",
            "ErrorEnvelope_Serialize",
            "ErrorParam",
            "Holding",
            "Krw",
            "LabDraftModel",
            "LabDraftResponse",
            "LabDraftResponse_Deserialize",
            "LabDraftResponse_Serialize",
            "LabResultModel",
            "MarketTone",
            "ModelHeader",
            "ObservationProgress",
            "PaperPosition",
            "PaperValidationState",
            "PingResponse",
            "PingResponse_Deserialize",
            "PingResponse_Serialize",
            "Pong",
            "Ratio",
            "Retryability",
            "Rule",
            "RuleDeviation",
            "RuleSection",
            "SectionState",
            "Signal",
            "StrategiesModel",
            "StrategiesResponse",
            "StrategiesResponse_Deserialize",
            "StrategiesResponse_Serialize",
            "StrategyDetailModel",
            "StrategyLineage",
            "StrategyResponse",
            "StrategyResponse_Deserialize",
            "StrategyResponse_Serialize",
            "StrategyRules",
            "StrategySummary",
            "SupportState",
            "TerminalAcknowledged",
            "TerminalInputParams",
            "TerminalInputResponse",
            "TerminalInputResponse_Deserialize",
            "TerminalInputResponse_Serialize",
            "TerminalOutput",
            "TerminalProcess",
            "TerminalProcessEvent",
            "TerminalResizeParams",
            "TerminalResizeResponse",
            "TerminalResizeResponse_Deserialize",
            "TerminalResizeResponse_Serialize",
            "TerminalRestartResponse",
            "TerminalRestartResponse_Deserialize",
            "TerminalRestartResponse_Serialize",
            "TerminalSessionModel",
            "TerminalStartResponse",
            "TerminalStartResponse_Deserialize",
            "TerminalStartResponse_Serialize",
            "TodayEvent",
            "TodayEventKind",
            "TodayModel",
            "TodayResponse",
            "TodayResponse_Deserialize",
            "TodayResponse_Serialize",
            "UiOutcome",
            "UiOutcome_Deserialize",
            "UiOutcome_Serialize",
            "UiResponseEnvelope",
            "UiResponseEnvelope_Deserialize",
            "UiResponseEnvelope_Serialize",
            "UpstreamStatus",
            "UpstreamStatus_Deserialize",
            "UpstreamStatus_Serialize",
            "Verdict"
        ]
    );
}
