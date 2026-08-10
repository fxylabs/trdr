# Security policy

## Reporting a vulnerability

**Do not open a public issue for a security problem.**

Report it privately through GitHub's private vulnerability reporting on this repository:
*Security → Report a vulnerability*. That channel is monitored by the maintainers and
keeps the report unpublished while it is being fixed.

Please include:

- what an attacker gets, in one sentence
- the steps to reproduce it, and the macOS version and hardware you saw it on
- the app or commit version
- whether you have disclosed it anywhere else

Please do not include real API keys, real account numbers, real balances or unredacted
terminal output. A redacted reproduction is enough. If a key was exposed while you were
testing, rotate it at the provider first.

We aim to acknowledge a report within five business days and to tell you our assessment
and a fix plan within fifteen. There is no bug bounty. Reporters are credited in the
release notes unless they ask not to be.

## Supported versions

There is no released build yet. Once releases begin, only the latest published version
receives fixes.

## What we treat as a vulnerability

The v0 trust boundary is: the officially signed app, the same macOS user account, and
the official `trdr` CLI installed alongside it. Everything crossing into that boundary
is untrusted input — WebView input, agent terminal output, strategy files, ingest
bundles, and upstream API responses.

In scope, and treated as blocking:

- an API key, token or account number reaching the WebView, the CLI, a log, the database,
  an export, a backup or a crash report
- the WebView or an agent reaching Keychain, SQLite, arbitrary files, the network or a
  spawned process directly, instead of through the narrow commands the host exposes
- a strategy registration completing without a human approval in the running app, or an
  approval bypass through the CLI, the socket or the UI
- a crafted ingest bundle or backup package escaping its directory, corrupting canonical
  data, or committing a partial write
- an order or any broker mutation being reachable at all
- a deterministic backtest producing a different result hash for the same input, or
  reading data ahead of its `available_at` time

Out of scope for v0, and stated in the plan rather than defended against:

- another process running as the same macOS user, or an attacker with root or admin
- a compromised operating system, or a Mac where the attacker has physical access
- multi-user or multi-device isolation; trdr is a single local user product
- theoretical future surfaces that do not exist yet: plugins, cloud sync, a second
  broker, a hosted runner

trdr does not claim application-level encryption of the local database. Files are stored
with user-only permissions. Portable backups are encrypted with a user passphrase because
they leave the machine; the live workspace is not.

## Third-party services

trdr talks to KIS, OpenDART and ECOS with credentials you supply. A vulnerability in one
of those services belongs to that provider. Tell us anyway if trdr's handling of their
response is what makes it exploitable.
