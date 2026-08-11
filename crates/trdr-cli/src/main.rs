//! The `trdr` command line interface.
//!
//! The CLI's job (`docs/FOUNDATION_DESIGN.md` sections 3.1 and 9.2) is to ask a
//! running app for things over a Unix socket, to read and compute locally when
//! the app is closed, and to render the same structured result either as text
//! for a person or as JSON for whatever is reading it. It never looks up a
//! credential value, never registers a strategy by itself, and never writes to
//! the database while the app is running.
//!
//! At stage 0 there is no socket yet. `trdr app status` therefore answers with
//! the same error envelope any surface would get — the app cannot be reached —
//! which is the honest answer and exercises the contract end to end.

use clap::{Parser, Subcommand, ValueEnum};
use std::io::Write;
use trdr_core::error::{ErrorCode, ErrorEnvelope, ErrorParam, Retryability};

/// Read a structured result, write it the way the caller asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format
{
    /// Sentences, for a person.
    Text,
    /// One JSON object, for a program.
    Json
}

/// Local-first trading research terminal for macOS.
#[derive(Debug, Parser)]
#[command(name = "trdr", version, about, long_about = None)]
struct Cli
{
    /// How to write the result.
    #[arg(long, value_enum, default_value_t = Format::Text, global = true)]
    format: Format,

    #[command(subcommand)]
    command: Command
}

/// The top-level command groups.
#[derive(Debug, Subcommand)]
enum Command
{
    /// The running trdr app.
    App
    {
        #[command(subcommand)]
        command: AppCommand
    }
}

/// Commands about the app itself.
#[derive(Debug, Subcommand)]
enum AppCommand
{
    /// Whether an app is running, and what it is holding.
    Status
}

fn main()
{
    let cli = Cli::parse();

    match run(&cli.command)
    {
        Ok(()) => (),
        Err(envelope) =>
        {
            report(&envelope, cli.format);
            std::process::exit(1);
        }
    }
}

/// Runs the command, producing either a result or the one error shape.
fn run(command: &Command) -> Result<(), ErrorEnvelope>
{
    match command
    {
        Command::App {
            command: AppCommand::Status
        } => Err(app_status())
    }
}

/// What `trdr app status` can honestly answer at stage 0.
///
/// `APP_NOT_RUNNING` is not a placeholder standing in for "unimplemented": with
/// no socket built, no app can be reached, which is exactly what this code
/// means. The `stage` parameter is what lets the surface say why without the
/// envelope carrying a sentence of its own.
fn app_status() -> ErrorEnvelope
{
    ErrorEnvelope::new(ErrorCode::AppNotRunning)
        .with_param("stage", ErrorParam::literal("scaffold"))
        .with_retryability(Retryability::No)
}

/// Writes the envelope in the requested form: JSON to standard output for a
/// program to read, sentences to standard error for a person.
fn report(envelope: &ErrorEnvelope, format: Format)
{
    match format
    {
        Format::Json => match serde_json::to_string(envelope)
        {
            Ok(json) => println!("{json}"),
            Err(error) => eprintln!("the result could not be written as JSON: {error}")
        },
        Format::Text =>
        {
            let mut stderr = std::io::stderr();
            let _ = writeln!(stderr, "{}", render(envelope));
        }
    }
}

/// Turns an envelope into the words a person reads.
///
/// The wording lives on this side, never in the envelope, so that the CLI and
/// the app can say the same thing differently and neither can leak a payload
/// into a sentence (section 12).
fn render(envelope: &ErrorEnvelope) -> String
{
    let mut text = format!("error: {}", wire_code(envelope.code));

    if let Some(detail) = detail(envelope.code)
    {
        text.push_str(&format!("\n  {detail}"));
    }

    if envelope.params.contains_key("stage")
    {
        text.push_str("\n  the app socket is not built yet, so nothing can be running to ask.");
    }

    text
}

/// The sentence for a code this build can produce.
///
/// The table is short because stage 0 produces one code. Filling it in is the
/// CLI track's work; until then an unmapped code still prints its own name
/// rather than nothing.
fn detail(code: ErrorCode) -> Option<&'static str>
{
    match code
    {
        ErrorCode::AppNotRunning => Some("the trdr app is not running."),
        _ => None
    }
}

/// The code as it is spelled on the wire.
fn wire_code(code: ErrorCode) -> String
{
    match serde_json::to_string(&code)
    {
        Ok(json) => json.trim_matches('"').to_owned(),
        Err(_) => format!("{:?}", code.family())
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use trdr_core::error::ErrorFamily;

    #[test]
    fn the_argument_parser_is_well_formed()
    {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn app_status_parses_with_either_format()
    {
        for arguments in [
            vec!["trdr", "app", "status"],
            vec!["trdr", "app", "status", "--format", "json"],
            vec!["trdr", "--format", "json", "app", "status"]
        ]
        {
            assert!(Cli::try_parse_from(arguments).is_ok());
        }

        assert!(Cli::try_parse_from(["trdr", "app", "status", "--format", "yaml"]).is_err());
        assert!(Cli::try_parse_from(["trdr", "app"]).is_err());
    }

    #[test]
    fn app_status_answers_that_no_app_can_be_reached()
    {
        let envelope = app_status();

        assert_eq!(envelope.code, ErrorCode::AppNotRunning);
        assert_eq!(envelope.code.family(), ErrorFamily::App);
        assert_eq!(envelope.retryability, Retryability::No);
        assert_eq!(
            envelope.params.get("stage"),
            Some(&ErrorParam::literal("scaffold"))
        );
    }

    #[test]
    fn the_json_form_is_the_envelope_itself()
    {
        let envelope = app_status();
        let json = serde_json::to_string(&envelope).unwrap();

        assert_eq!(
            serde_json::from_str::<ErrorEnvelope>(&json).unwrap(),
            envelope
        );
        assert!(json.contains("\"code\":\"APP_NOT_RUNNING\""));
    }

    #[test]
    fn the_text_form_names_the_code_and_explains_it()
    {
        let text = render(&app_status());

        assert!(text.contains("APP_NOT_RUNNING"));
        assert!(text.contains("not running"));
        assert!(text.contains("socket"));
    }
}
