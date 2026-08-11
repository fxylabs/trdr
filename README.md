# trdr

A local-first trading research terminal for macOS. Your API keys, your market data,
your strategies, and your results stay on your own Mac.

> **Status: pre-release.** The product contracts are settled and the application code
> has not been written yet. There is no downloadable build, and no part of this
> repository is ready to run.

## What it is

- **Strategy as a file.** A strategy is a declarative `.trdr.yaml` spec, not a script.
  The same spec, data snapshot, cost model and engine version always produce the same
  result hash.
- **Pre-registration before observation.** A strategy is frozen by a human approval in
  the app before paper validation starts. Registrations are append-only: nothing edits
  or deletes one afterwards.
- **Bring your own agent.** Claude Code or Codex runs in a persistent terminal rail
  inside the app. The agent writes strategy files and calls the `trdr` CLI. The app
  never interprets terminal output as product state, and never runs an LLM itself.
- **You own the data.** Built-in collectors (KIS, OpenDART, ECOS) run on your Mac with
  your own API keys. Any other source enters through a versioned ingest bundle that
  passes the same validation as the built-in ones.

## What it does not do

- No orders, no order previews, no broker mutation, no automated trading. There is no
  code path for it.
- No cloud backend, no account server, no hosted runner, no data redistribution.
- No telemetry by default, and never for keys, account values, holdings, strategies or
  terminal content.
- No official KRX collector, credentials or support. Data you lawfully obtained
  yourself can still be loaded as a `user.*` ingest bundle.

## Scope of v0

| Item | v0 |
|---|---|
| Platform | Apple Silicon, macOS 14 or newer |
| Distribution | Developer ID signed, Apple notarized DMG. Not the App Store |
| Shell | Tauri 2 host, React/TypeScript/Vite UI, Rust core and CLI |
| Surfaces | Fixed Today / Lab / Strategies, plus a persistent raw terminal rail |
| Broker | KIS only, read-only |
| Built-in collectors | KIS, OpenDART, ECOS |
| Store | Bundled SQLite in WAL mode with a single-writer lease |
| Strategies | Long-only daily bars, MA20/60 crossover, equal weight, capped holdings |

## Repository map

```text
docs/FOUNDATION_DESIGN.md    process, storage, ingest, IPC and authority contract
docs/IMPLEMENTATION_PLAN.md  approved scope, milestones and evidence gates
docs/README.md               which document answers which question
design/ui-kit/               approved v2 visual contract: tokens, components, contracts
crates/trdr-core             the domain: identity, errors, the two IPC contracts, query models
crates/trdr-runtime          everything that touches the outside world: SQLite, the socket, the PTY
crates/trdr-cli              the `trdr` command line
apps/desktop                 the Tauri host, its capability file, and the React screens
packages/ui                  the visual contract as production components
fixtures/synthetic           the invented data the app runs on until real collectors land
spikes/                      throwaway proofs of two remaining risk paths, each deleted
                             by the change that reproduces its answer
```

`spikes/` is not part of that and is not imported by anything. It exists to answer questions
that were expensive to get wrong — whether a raw terminal, a CLI socket bridge with a native
approval, and a Keychain-backed credential hold up on Tauri. The terminal's answer is now
production code and its spike is gone; the other two are deleted by the changes that
reproduce them. See `spikes/README.md`.

## Running it

Requires macOS 13 or later, a Rust toolchain, and Node 22 with pnpm.

```sh
pnpm install
pnpm --filter @trdr/desktop build      # the screens
cargo build -p trdr-desktop --release  # the app around them
./target/release/trdr-desktop
```

A release build serves the screens from what `pnpm build` produced, so this is
the whole of it.

A **debug** build does not. Tauri points a debug WebView at the dev server named
by `devUrl` in `apps/desktop/src-tauri/tauri.conf.json`, so `cargo build` without
`--release` gives a window that stays blank until that server is running:

```sh
pnpm --filter @trdr/desktop dev        # leave this running, then, elsewhere:
cargo run -p trdr-desktop
```

The blank window is the failure this catches, and it is silent — no line on
standard error, no exit code, nothing in the log. `TRDR_DEVTOOLS=1` opens the
WebView inspector on start-up, where it reads *Could not connect to the server*.

`TRDR_PRODUCT_ROOT` moves the state directory somewhere other than `~/.trdr`,
which is how to run a build without touching a real workspace. Keep the path
short: a Unix socket path cannot exceed 104 bytes and the socket lives inside it.

```sh
TRDR_PRODUCT_ROOT=/private/tmp/trdr-qa ./target/release/trdr-desktop
```

The CLI is the same workspace's other half, and it talks to a running app:

```sh
cargo run -p trdr-cli -- app status
cargo run -p trdr-cli -- account inspect
```

Everything a build shows is synthetic until the collectors land, and it says so
on every screen and above every number the CLI prints.

## License

trdr is free software under the **GNU Affero General Public License v3.0** — see
[`LICENSE`](LICENSE). The whole desktop product, including the UI, is covered.

The `trdr` name, the logo, the official Apple Developer ID signature, the notarized DMG
and the update channel are project identity, not part of the code grant. See
[`TRADEMARKS.md`](TRADEMARKS.md) before distributing a fork.

## Contributing

External code contributions and pull requests are **not accepted yet**. Bug reports and
proposals are welcome as issues, and project work is tracked against issues. See
[`CONTRIBUTING.md`](CONTRIBUTING.md).

Security reports: [`SECURITY.md`](SECURITY.md). Data handling: [`PRIVACY.md`](PRIVACY.md).
