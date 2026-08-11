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
spikes/                      throwaway proofs of three risk paths, deleted when M1 closes
```

No `apps/`, `crates/`, `packages/` or `schemas/` directory exists yet. They arrive with
the first scaffold, described in `docs/FOUNDATION_DESIGN.md` section 14.

`spikes/` is not that scaffold and is not imported by anything. It exists to answer three
questions that were expensive to get wrong — whether a raw terminal, a CLI socket bridge with
a native approval, and a Keychain-backed credential hold up on Tauri — and it is deleted once
its answers are carried into the scaffold. See `spikes/README.md`.

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
