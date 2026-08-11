//! The `trdr` command line interface.
//!
//! The CLI's job (`docs/FOUNDATION_DESIGN.md` sections 3.1 and 9.2) is to ask a
//! running app for things over a Unix socket, to read and compute locally when
//! the app is closed, and to render the same structured result either as text
//! for a person or as JSON for whatever is reading it. It never looks up a
//! credential value, never registers a strategy by itself, and never writes to
//! the database while the app is running.
//!
//! `trdr app status` is the first command to make the round trip. It connects to
//! `<product root>/run/app.sock`, sends the `app.status` frame, and prints what
//! comes back. When nothing is listening there is no app to ask, and section 9.2
//! calls that `APP_NOT_RUNNING` rather than a socket error shown to a person.
//!
//! The words live on this side. An [`ErrorEnvelope`] carries a code and named
//! parameters and never a sentence, so that no payload can reach a screen by
//! being put into a message somewhere further down (section 12).

use clap::{Parser, Subcommand, ValueEnum};
use std::io::Write;
use std::path::PathBuf;
use trdr_core::error::{ErrorCode, ErrorEnvelope, ErrorParam};
use trdr_core::query::DataOrigin;
use trdr_core::socket::{AccountInspectResult, AppStatusResult, DataCoverageResult};
use trdr_runtime::root::ProductRoot;
use trdr_runtime::socket::AppClient;

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

    /// The directory trdr keeps its state in. Defaults to `~/.trdr`.
    #[arg(long, value_name = "PATH", global = true)]
    product_root: Option<PathBuf>,

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
    },
    /// The account the running app has open.
    Account
    {
        #[command(subcommand)]
        command: AccountCommand
    },
    /// The data behind a strategy's period.
    Data
    {
        #[command(subcommand)]
        command: DataCommand
    }
}

/// Commands about the app itself.
#[derive(Debug, Subcommand)]
enum AppCommand
{
    /// Whether an app is running, and what it is holding.
    Status
}

/// Commands about the account.
#[derive(Debug, Subcommand)]
enum AccountCommand
{
    /// The totals and the positions, as the Today screen shows them.
    Inspect
}

/// Commands about collected data.
#[derive(Debug, Subcommand)]
enum DataCommand
{
    /// How much of the drafted strategy's period is present.
    Coverage
}

/// Everything a command can produce when it worked.
#[derive(Debug)]
enum Report
{
    /// What a running app said about itself.
    AppStatus(Box<AppStatusResult>),
    /// The account, as the Today screen would show it.
    Account(Box<AccountInspectResult>),
    /// What the drafted strategy's period is missing.
    Coverage(Box<DataCoverageResult>)
}

fn main()
{
    let cli = Cli::parse();

    match run(&cli)
    {
        Ok(report) => print_report(&report, cli.format),
        Err(envelope) =>
        {
            print_error(&envelope, cli.format);
            std::process::exit(1);
        }
    }
}

/// Runs the command, producing either a result or the one error shape.
fn run(cli: &Cli) -> Result<Report, ErrorEnvelope>
{
    let root = match &cli.product_root
    {
        Some(path) => ProductRoot::at(path),
        None => ProductRoot::for_current_user().map_err(|_| {
            ErrorEnvelope::new(ErrorCode::AppNotRunning)
                .with_param("reason", ErrorParam::literal("home_unknown"))
        })?
    };

    match cli.command
    {
        Command::App {
            command: AppCommand::Status
        } => app_status(&root),
        Command::Account {
            command: AccountCommand::Inspect
        } => account_inspect(&root),
        Command::Data {
            command: DataCommand::Coverage
        } => data_coverage(&root)
    }
}

/// Asks the running app about itself.
fn app_status(root: &ProductRoot) -> Result<Report, ErrorEnvelope>
{
    let mut client = AppClient::connect(root).map_err(|error| error.to_envelope())?;
    let status = client.app_status().map_err(|error| error.to_envelope())?;

    Ok(Report::AppStatus(Box::new(status)))
}

/// Asks the running app what the account holds.
fn account_inspect(root: &ProductRoot) -> Result<Report, ErrorEnvelope>
{
    let mut client = AppClient::connect(root).map_err(|error| error.to_envelope())?;
    let account = client
        .account_inspect()
        .map_err(|error| error.to_envelope())?;

    Ok(Report::Account(Box::new(account)))
}

/// Asks the running app how much of the period is covered.
fn data_coverage(root: &ProductRoot) -> Result<Report, ErrorEnvelope>
{
    let mut client = AppClient::connect(root).map_err(|error| error.to_envelope())?;
    let coverage = client
        .data_coverage()
        .map_err(|error| error.to_envelope())?;

    Ok(Report::Coverage(Box::new(coverage)))
}

/// Writes a result: JSON to standard output, sentences to standard output.
fn print_report(report: &Report, format: Format)
{
    let written = match (report, format)
    {
        (Report::AppStatus(status), Format::Json) => serde_json::to_string(status.as_ref()),
        (Report::Account(account), Format::Json) => serde_json::to_string(account.as_ref()),
        (Report::Coverage(coverage), Format::Json) => serde_json::to_string(coverage.as_ref()),
        (Report::AppStatus(status), Format::Text) => Ok(render_status(status)),
        (Report::Account(account), Format::Text) => Ok(render_account(account)),
        (Report::Coverage(coverage), Format::Text) => Ok(render_coverage(coverage))
    };

    match written
    {
        Ok(text) => println!("{text}"),
        Err(_) => print_error(
            &ErrorEnvelope::new(ErrorCode::AppProtocolVersion)
                .with_param("reason", ErrorParam::literal("result_unserialisable")),
            Format::Text
        )
    }
}

/// Writes the envelope in the requested form: JSON to standard output for a
/// program to read, sentences to standard error for a person.
fn print_error(envelope: &ErrorEnvelope, format: Format)
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
            let _ = writeln!(stderr, "{}", render_error(envelope));
        }
    }
}

/// Turns a status into the lines a person reads.
fn render_status(status: &AppStatusResult) -> String
{
    let lease = match status.holds_writer_lease
    {
        true => "held",
        false => "not held"
    };

    format!(
        "the trdr app is running\n  \
         version         {}\n  \
         pid             {}\n  \
         writer lease    {lease}\n  \
         product root    {}\n  \
         workspace       {}\n  \
         schema version  {}",
        status.app_version,
        status.pid,
        status.product_root.display(),
        status.workspace_path.display(),
        status.schema_version
    )
}

/// Turns an account into the lines a person reads.
///
/// The synthetic banner comes first and is not optional. Milestone M2 requires
/// the app to say at all times that these are not real numbers, and a terminal
/// is where that is easiest to forget — output gets scrolled past, pasted, and
/// read back later with no window around it to give it context.
fn render_account(account: &AccountInspectResult) -> String
{
    let mut text = String::new();

    if account.origin == DataOrigin::Synthetic
    {
        text.push_str(SYNTHETIC_BANNER);
        text.push('\n');
    }

    text.push_str(&format!(
        "account\n  \
         total       {} KRW\n  \
         cash        {} KRW\n  \
         today       {} KRW ({})\n  \
         as of       {}\n  \
         connection  {}\n  \
         state       {}",
        account.account.total_value.0,
        account.account.cash.0,
        account.account.day_change.0,
        account.account.day_change_ratio.0,
        account.account.as_of,
        wire_value(serde_json::to_value(account.account.connection).unwrap_or_default()),
        wire_value(serde_json::to_value(account.account.state).unwrap_or_default())
    ));

    for holding in &account.holdings
    {
        text.push_str(&format!(
            "\n  {} {} {:>6} x {:>9} KRW  {:>10} KRW ({})",
            pad(&holding.symbol.to_string(), 10),
            pad(&holding.name, 14),
            holding.quantity,
            holding.last_price.0,
            holding.unrealized.0,
            holding.unrealized_ratio.0
        ));
    }

    text
}

/// Turns coverage into the lines a person reads.
fn render_coverage(coverage: &DataCoverageResult) -> String
{
    let mut text = String::new();

    if coverage.origin == DataOrigin::Synthetic
    {
        text.push_str(SYNTHETIC_BANNER);
        text.push('\n');
    }

    if coverage.coverage.is_empty()
    {
        text.push_str("no source covers this period");
        return text;
    }

    for source in &coverage.coverage
    {
        text.push_str(&format!(
            "{}\n  covered  {} of {} market days",
            source.source, source.covered_days, source.total_days
        ));

        for gap in &source.gaps
        {
            text.push_str(&format!("\n  missing  {} to {}", gap.from, gap.to));
        }
    }

    text
}

/// The one line that says these numbers are invented.
const SYNTHETIC_BANNER: &str = "synthetic data — not an account, not a market";

/// A finite state's own spelling, taken from its serialised form.
///
/// The states are the visual contract's, and the contract's spelling is what
/// both surfaces show. Going through serde rather than writing a match keeps the
/// CLI from acquiring a second vocabulary that has to be kept in step.
fn wire_value(value: serde_json::Value) -> String
{
    value
        .as_str()
        .map_or_else(|| "unknown".to_owned(), str::to_owned)
}

/// Pads text to a column width a terminal will agree with.
///
/// `{:<12}` counts characters, and a terminal counts columns. Korean, Chinese
/// and Japanese characters occupy two columns each, so a name of four Hangul
/// syllables is four to `format!` and eight on screen — which is why a table of
/// Korean company names padded with `{:<}` comes out ragged. This counts the
/// columns instead.
fn pad(text: &str, width: usize) -> String
{
    let mut padded = text.to_owned();

    for _ in display_width(text)..width
    {
        padded.push(' ');
    }

    padded
}

/// How many terminal columns a string occupies.
///
/// The wide ranges of Unicode's East Asian Width property, which is the part
/// that matters here: Hangul, the CJK ideographs, the kana, and the fullwidth
/// forms. Everything else is counted as one column, including the combining
/// marks that are really zero — trdr renders company names and stable codes, and
/// neither carries one.
fn display_width(text: &str) -> usize
{
    text.chars().map(char_width).sum()
}

/// One character's width in columns.
fn char_width(character: char) -> usize
{
    match character as u32
    {
        0x1100..=0x115F
        | 0x2E80..=0x303E
        | 0x3041..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x20000..=0x3FFFD => 2,
        _ => 1
    }
}

/// Turns an envelope into the words a person reads.
///
/// The wording lives on this side, never in the envelope, so that the CLI and
/// the app can say the same thing differently and neither can leak a payload
/// into a sentence (section 12).
fn render_error(envelope: &ErrorEnvelope) -> String
{
    let mut text = format!("error: {}", wire_code(envelope.code));

    if let Some(detail) = detail(envelope.code)
    {
        text.push_str(&format!("\n  {detail}"));
    }

    if let Some(ErrorParam::Text(reason)) = envelope.params.get("reason")
    {
        text.push_str(&format!("\n  ({reason})"));
    }

    text
}

/// The sentence for a code this build can produce.
///
/// The table is short because this build produces few codes. Filling it in
/// happens as each command lands; until then an unmapped code still prints its
/// own name rather than nothing.
fn detail(code: ErrorCode) -> Option<&'static str>
{
    match code
    {
        ErrorCode::AppNotRunning => Some("the trdr app is not running."),
        ErrorCode::AppProtocolVersion => Some("the app answered something this build cannot read."),
        ErrorCode::AppPermission => Some("the app refused this request."),
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
    use trdr_runtime::test_support::scratch_root;

    fn sample_status() -> AppStatusResult
    {
        AppStatusResult {
            app_version: "0.0.0".to_owned(),
            pid: 4321,
            holds_writer_lease: true,
            product_root: PathBuf::from("/private/tmp/t"),
            workspace_path: PathBuf::from("/private/tmp/t/workspaces/default"),
            schema_version: 1
        }
    }

    #[test]
    fn the_argument_parser_is_well_formed()
    {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    /// The bug this exists for: `{:<12}` pads by characters, a terminal aligns
    /// by columns, and the two disagree by exactly one column per Hangul
    /// syllable. Four names of different syllable counts is what made the
    /// holdings table ragged.
    #[test]
    fn a_korean_name_is_padded_by_the_columns_it_occupies()
    {
        assert_eq!(display_width("SYN0001"), 7);
        assert_eq!(display_width("합성전자"), 8);
        assert_eq!(display_width("합성바이오"), 10);

        for name in ["합성전자", "합성바이오", "SYN0001", ""]
        {
            assert_eq!(display_width(&pad(name, 14)), 14, "{name} padded wrong");
        }
    }

    /// Padding never truncates. A name wider than the column pushes the row out
    /// rather than losing a character, because a cut company name is a different
    /// company name.
    #[test]
    fn a_name_wider_than_its_column_is_left_alone()
    {
        let long = "합성중공업지주회사";

        assert!(display_width(long) > 14);
        assert_eq!(pad(long, 14), long);
    }

    #[test]
    fn app_status_parses_with_either_format()
    {
        for arguments in [
            vec!["trdr", "app", "status"],
            vec!["trdr", "app", "status", "--format", "json"],
            vec!["trdr", "--format", "json", "app", "status"],
            vec!["trdr", "app", "status", "--product-root", "/private/tmp/t"]
        ]
        {
            assert!(Cli::try_parse_from(arguments).is_ok());
        }

        assert!(Cli::try_parse_from(["trdr", "app", "status", "--format", "yaml"]).is_err());
        assert!(Cli::try_parse_from(["trdr", "app"]).is_err());
    }

    #[test]
    fn a_product_root_nothing_is_serving_answers_that_no_app_can_be_reached()
    {
        let root = scratch_root("cli-not-running");
        let envelope = app_status(&root).expect_err("nothing is listening there");

        assert_eq!(envelope.code, ErrorCode::AppNotRunning);
        assert_eq!(envelope.code.family(), ErrorFamily::App);
        assert_eq!(
            envelope.params.get("reason"),
            Some(&ErrorParam::literal("no_socket"))
        );
    }

    #[test]
    fn the_json_form_of_a_failure_is_the_envelope_itself()
    {
        let root = scratch_root("cli-json");
        let envelope = app_status(&root).expect_err("nothing is listening there");
        let json = serde_json::to_string(&envelope).unwrap();

        assert_eq!(
            serde_json::from_str::<ErrorEnvelope>(&json).unwrap(),
            envelope
        );
        assert!(json.contains("\"code\":\"APP_NOT_RUNNING\""));
    }

    #[test]
    fn the_text_form_of_a_failure_names_the_code_and_explains_it()
    {
        let text = render_error(
            &ErrorEnvelope::new(ErrorCode::AppNotRunning)
                .with_param("reason", ErrorParam::literal("no_socket"))
        );

        assert!(text.contains("APP_NOT_RUNNING"));
        assert!(text.contains("not running"));
        assert!(text.contains("no_socket"));
    }

    #[test]
    fn the_text_form_of_a_status_says_what_the_app_is_holding()
    {
        let text = render_status(&sample_status());

        assert!(text.contains("running"));
        assert!(text.contains("4321"));
        assert!(text.contains("held"));
        assert!(text.contains("/private/tmp/t/workspaces/default"));
    }

    #[test]
    fn the_json_form_of_a_status_is_the_result_itself()
    {
        let status = sample_status();
        let json = serde_json::to_string(&status).unwrap();

        assert_eq!(
            serde_json::from_str::<AppStatusResult>(&json).unwrap(),
            status
        );
    }
}
