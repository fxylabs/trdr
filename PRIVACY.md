# Privacy

trdr runs entirely on your Mac. There is no trdr server, no account to create, and no
data of yours that we can see.

> This describes the product as designed and contracted. The application has not been
> built yet; this document is the standard it will be verified against before any
> release, and it will be dated and versioned when the first build ships.

## What leaves your Mac

Only requests you cause, sent directly from your Mac to the providers whose keys you
entered:

| Destination | When | What it carries |
|---|---|---|
| KIS | you connect the account, refresh Today, or collect prices | your KIS credentials and the read-only request |
| OpenDART | you run a disclosure or financial collection | your OpenDART key and the request |
| ECOS | you run a macro series collection | your ECOS key and the request |

Nothing else. There is no trdr endpoint to send anything to.

## What never leaves your Mac

Your API keys and tokens, your account number and balances, your holdings, your
watchlists, your strategy files, your backtest results, your registrations, and
everything your agent types or prints in the terminal.

## Telemetry

Off by default. If analytics are ever added, they will be opt-in, described here before
they ship, and will never carry API keys, account values, holdings, strategies, file
paths from your workspace, or terminal content.

## Where your data is stored

```text
~/.trdr/config.json              which workspace is selected, and a local instance id
~/.trdr/workspaces/default/      the workspace: database, raw objects, strategies, logs
macOS Keychain                   API keys and tokens, service com.fxylabs.trdr
```

- Credentials live only in the Keychain. They are never written to the database, a
  config file, an environment variable, an export, or a log.
- The app returns a credential's *state* — missing, untested, valid, invalid,
  rate-limited — to the UI. It never returns the value.
- Your KIS account is stored as one normalized latest snapshot. The raw broker response
  and the access token are not stored at all.
- Logs are written locally with an allowlist of fields. Account numbers, tokens, secrets,
  HTTP headers and bodies, and terminal bytes are not log fields.

## Backups and exports

- A **local recovery point** is taken automatically before a schema migration and stays
  inside your workspace.
- A **portable backup** is the file you move to another disk or Mac. It contains your
  normalized account snapshot, so it is a sensitive file. It is encrypted with a
  passphrase you choose, and it never contains Keychain keys, tokens, logs, terminal
  content or raw broker responses.
- An **export** is for reading your data outside trdr. It never contains credentials or
  raw account responses.
- Restoring a backup on another Mac leaves every credential in the `missing` state. You
  enter the keys again there.

## Your agent

Claude Code or Codex runs as a normal child process on your Mac, under your account,
with whatever access you have already granted it. Whatever you type into that terminal
goes to that agent's provider under that provider's own privacy terms, not ours. trdr
neither reads nor stores what passes through it beyond the scrollback shown on screen.

## Deleting your data

Quit the app, then delete `~/.trdr` and remove the `com.fxylabs.trdr` Keychain items.
Nothing of yours remains anywhere else, because it was never anywhere else.
